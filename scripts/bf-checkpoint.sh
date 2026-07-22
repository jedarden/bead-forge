#!/usr/bin/env bash
# bf-checkpoint.sh — periodic git checkpoint of a .beads/ workspace (ADR-1)
#
# Runs out-of-band (systemd timer only — never from the bf claim/close hot path).
# For a given workspace this:
#   1. flushes SQLite → JSONL            : `bf sync --flush-only`
#   2. detects whether .beads/issues.jsonl changed in git
#   3. if it did, stages ONLY .beads/issues.jsonl (NEVER beads.db) and commits
#      it with a fixed git identity and the `chore(beads): auto-checkpoint`
#      message prefix.
#
# NixOS portable variant — use deploy/bf-checkpoint.sh on Debian/Ubuntu.
#
# Configuration (.beads/config.yaml, `checkpoint:` block):
#   enabled: bool          (default false) master switch; when false this script
#                           is a no-op even with the timer deployed. New
#                           rollouts stay disabled until a maintainer opts in.
#   interval_minutes: int  (default 60)    minimum gap between commits; the
#                           script self-throttles to this regardless of how
#                           often the timer fires.
#   push: bool             (default false) persistently opt into `git push`
#                           after each commit.
#
# Usage:
#   bf-checkpoint.sh [--push] [-w|--workspace <dir>]
#
#   --push           also run `git push` after committing (one-shot override;
#                    also enabled persistently by checkpoint.push=true)
#   -w, --workspace  workspace root containing .beads/ (default: $BF_WORKSPACE,
#                    or $PWD, or $HOME/bead-forge)
#
# Install alongside bf-update.sh:
#   cp deploy/bf-checkpoint.sh ~/.local/bin/bf-checkpoint.sh
#   chmod +x ~/.local/bin/bf-checkpoint.sh
# Driven by bf-checkpoint.service/.timer (see deploy/README.md).

set -euo pipefail

# --- locate bf and git ------------------------------------------------------
# bf lives in ~/.local/bin (not in the systemd unit's PATH). On NixOS git lives
# in /run/current-system/sw/bin. Prepend both so the script is portable across
# the deploy/ and systemd/ (NixOS) variants.
case ":$PATH:" in
    *":$HOME/.local/bin:"*) ;;
    *) PATH="$HOME/.local/bin:$PATH" ;;
esac
if [ -d /run/current-system/sw/bin ]; then
    case ":$PATH:" in
        *":/run/current-system/sw/bin:"*) ;;
        *) PATH="/run/current-system/sw/bin:$PATH" ;;
    esac
fi
export PATH

# --- usage ------------------------------------------------------------------
print_usage() {
    sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'
}

# --- argument parsing -------------------------------------------------------
PUSH_FLAG=0
WORKSPACE=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --push)        PUSH_FLAG=1; shift ;;
        -w|--workspace) WORKSPACE="${2:?--workspace requires a value}"; shift 2 ;;
        -h|--help)     print_usage; exit 0 ;;
        --)            shift; break ;;
        -*)            echo "ERROR: unknown option: $1" >&2; exit 2 ;;
        *)             WORKSPACE="${WORKSPACE:-$1}"; shift ;;
    esac
done
# Default workspace: -w flag, else $BF_WORKSPACE (set by the systemd unit), else
# the current directory, else $HOME/bead-forge. $BF_WORKSPACE intentionally takes
# precedence over $PWD so bf-checkpoint.service can target a specific workspace
# even though user services start with PWD=$HOME.
WORKSPACE="${WORKSPACE:-${BF_WORKSPACE:-${PWD:-$HOME/bead-forge}}}"
WORKSPACE_RAW="$WORKSPACE"
WORKSPACE="$(cd "$WORKSPACE" 2>/dev/null && pwd)" || {
    echo "ERROR: workspace directory not found: $WORKSPACE_RAW" >&2
    exit 1
}

# --- helpers ----------------------------------------------------------------
# Treat true/True/TRUE/1/yes as enabled (serde prints booleans as `true`/`false`).
is_true() {
    case "${1,,}" in true|1|yes|on) return 0 ;; *) return 1 ;; esac
}
cfg_get() {  # cfg_get <dotted-key> <fallback>
    bf config get "$1" -w "$WORKSPACE" 2>/dev/null || printf '%s' "$2"
}

# --- read checkpoint config -------------------------------------------------
ENABLED="$(cfg_get checkpoint.enabled false)"
# sanitize interval to digits only (defends against empty/garbage arithmetic)
INTERVAL="$(cfg_get checkpoint.interval_minutes 60)"
INTERVAL="${INTERVAL//[^0-9]/}"
INTERVAL="${INTERVAL:-60}"
CFG_PUSH="$(cfg_get checkpoint.push false)"

if ! is_true "$ENABLED"; then
    echo "checkpoint disabled for $WORKSPACE (checkpoint.enabled != true) — nothing to do"
    exit 0
fi

# --- self-throttle on interval_minutes --------------------------------------
# Per-workspace state file so cadence is controlled by config, not just by how
# often the timer fires. Slug the workspace path into a safe filename.
STATE_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/bf-checkpoint"
mkdir -p "$STATE_DIR"
SLUG="${WORKSPACE#/}"            # drop leading /
SLUG="${SLUG//\//_}"            # /  -> _
STATE_FILE="$STATE_DIR/${SLUG}.last"

now=$(date +%s)
if [[ -f "$STATE_FILE" ]]; then
    last="$(cat "$STATE_FILE" 2>/dev/null || echo 0)"
    last="${last//[^0-9]/}"; last="${last:-0}"
    if (( now - last < INTERVAL * 60 )); then
        echo "last checkpoint < ${INTERVAL}m ago for $WORKSPACE — skipping"
        exit 0
    fi
fi

# --- preflight: must be a git repo ------------------------------------------
if ! git -C "$WORKSPACE" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "not a git repository: $WORKSPACE — nothing to checkpoint"
    exit 0
fi
JSONL="$WORKSPACE/.beads/issues.jsonl"

# --- 1. flush SQLite -> JSONL -----------------------------------------------
# Flush FIRST. The on-disk issues.jsonl is produced by this flush — it is not
# maintained incrementally yet (Phase 7.1's auto-flush is spec'd but not built),
# so checking for the file before flushing would bail out of any workspace that
# has never been flushed. Flush, then let the git diff below decide whether a
# commit is warranted.
echo "Flushing SQLite -> JSONL for $WORKSPACE"
bf sync --flush-only -w "$WORKSPACE"

# --- 2. detect change -------------------------------------------------------
# After the flush the file must exist (guarded defensively). `git diff --quiet`
# covers tracked changes; `status --porcelain` also catches a file the flush
# just created (still untracked) so the very first checkpoint lands in git.
if [[ ! -f "$JSONL" ]]; then
    echo "flush produced no .beads/issues.jsonl in $WORKSPACE — nothing to checkpoint"
    echo "$now" > "$STATE_FILE"
    exit 0
fi
if git -C "$WORKSPACE" diff --quiet -- .beads/issues.jsonl \
   && [[ -z "$(git -C "$WORKSPACE" status --porcelain -- .beads/issues.jsonl)" ]]; then
    echo "no changes to .beads/issues.jsonl — nothing to commit"
    echo "$now" > "$STATE_FILE"
    exit 0
fi

# --- 3. stage ONLY issues.jsonl and commit ---------------------------------
# Safety: (a) add the single path, (b) constrain the commit to that pathspec so
# nothing else staged in the repo can ride along, and (c) never touch beads.db
# (it is not git-tracked and is explicitly never staged here).
git -C "$WORKSPACE" add .beads/issues.jsonl
MSG="chore(beads): auto-checkpoint $(date -u +%Y-%m-%dT%H:%M:%SZ)"
git -C "$WORKSPACE" \
    -c user.email=github@jedarden.com \
    -c user.name=jedarden \
    commit -m "$MSG" -- .beads/issues.jsonl
echo "Committed: $MSG"

# record this checkpoint so the throttle window applies
echo "$now" > "$STATE_FILE"

# --- 4. push (opt-in) -------------------------------------------------------
if (( PUSH_FLAG == 1 )) || is_true "$CFG_PUSH"; then
    echo "Pushing (checkpoint.push=$CFG_PUSH, --push=$PUSH_FLAG)"
    git -C "$WORKSPACE" push
fi

echo "Checkpoint complete for $WORKSPACE"
