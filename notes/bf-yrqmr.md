# bf-yrqmr — Add flush + git change detection to bf-checkpoint.sh (verification)

**Task:** Own and verify slice #2 of parent `bf-3cu1k` (the ADR-1 `bf-checkpoint.sh`).
This slice covers ONLY the git-repo preflight, the `bf sync --flush-only` flush, and the
change-detection that decides whether to commit — i.e. everything *between* the config
gate (owned by sibling `bf-4gkg5`) and the stage/commit/push tail (owned by later
siblings). Sibling children own gating, commit, and throttle/push; `src/claim.rs` and any
claim/close hot path must stay untouched.

**Outcome:** No source change required. The script was committed whole in `fef0340`
(parent `bf-3cu1k`); this slice's flush + change-detection logic is already present and
correct in both `deploy/bf-checkpoint.sh` and `scripts/bf-checkpoint.sh` (identical
logic, differing only by shebang). This note records the slice-#2 verification against
every acceptance criterion, including end-to-end runtime tests against isolated temp
workspaces (the real `/home/coding/bead-forge` tree is shared across needle agents and
was not mutated).

## Slice scope as committed (scripts/bf-checkpoint.sh)

- **git-repo preflight** — `scripts/bf-checkpoint.sh:127-131`:
  `git -C "$WORKSPACE" rev-parse --is-inside-work-tree`; on failure prints
  `not a git repository: <ws> — nothing to checkpoint` and exits 0 (no commit).
- **Step 1 flush** — `scripts/bf-checkpoint.sh:134-141`:
  `bf sync --flush-only -w "$WORKSPACE"` runs *before* any file check (the JSONL is
  produced by the flush; checking pre-flush would bail on any never-flushed workspace).
- **Step 2 detect change** — `scripts/bf-checkpoint.sh:143-157`:
  - `147-151`: if the flush left no `.beads/issues.jsonl`, print a message and exit 0
    (no commit);
  - `152-157`: `git diff --quiet -- .beads/issues.jsonl` (tracked changes) **AND**
    `git status --porcelain -- .beads/issues.jsonl` empty (catches a first-time
    untracked file). If neither fired, print `no changes … — nothing to commit` and
    exit 0 without staging/committing.

## Acceptance criteria — all satisfied by committed code

1. **Clean workspace (jsonl unchanged after flush) flushes and exits 0 without
   staging/committing anything** — ✅
   Set up an isolated git repo with `checkpoint.enabled=true`, `interval_minutes=0`
   (so the immediate re-run is not throttled), and a single db-only bead. First run
   flushed + committed (scenario B below). The immediate *second* run reproduced
   byte-identical JSONL, so:
   ```
   Flushing SQLite -> JSONL for /tmp/cp-git
   Flushed 1 beads to JSONL
   no changes to .beads/issues.jsonl — nothing to commit
   [exit=0]
   ```
   HEAD unchanged after the run; `git diff --cached --name-only` empty (nothing staged).
   Only `git add`/`commit` the script performs is scoped to the single `.beads/issues.jsonl`
   pathspec, so `beads.db`/`config.yaml` are never staged.

2. **A first-ever flush that creates a still-untracked issues.jsonl is detected as a
   change (not silently skipped)** — ✅
   Same repo, state before first run was db-only (`issues.jsonl present before run?
   NO-it-is-db-only`). The `status --porcelain` arm of the detection caught the freshly
   created (untracked) file and the script committed it:
   ```
   Flushed 1 beads to JSONL
   [master (root-commit) 8f6450c] chore(beads): auto-checkpoint 2026-07-22T13:40:49Z
    1 file changed, 1 insertion(+)
    create mode 100644 .beads/issues.jsonl
   [exit=0]
   ```
   `git show --stat HEAD` touched exactly one file: `.beads/issues.jsonl`.

3. **A non-git directory exits 0 with a "not a git repository" message (no error, no
   commit)** — ✅
   Ran the script against a temp dir with a `checkpoint:` block but no `.git`:
   ```
   not a git repository: /tmp/cp-nongit — nothing to checkpoint
   [exit=0]
   ```
   No `git add`, no commit (control flow exits at the preflight, line 128-131, before the
   flush/diff/commit tail).

4. **`bash -n` passes on both variants** — ✅
   ```
   bash -n scripts/bf-checkpoint.sh -> OK
   bash -n deploy/bf-checkpoint.sh  -> OK
   ```
   (shellcheck is not installed on this box; `bash -n` is the available syntax gate. The
   `deploy/` `#!/bin/bash` variant cannot be exec'd directly here — no `/bin/bash` on
   NixOS — but its logic is byte-identical to `scripts/`, which is why the split exists.)

5. **`src/claim.rs` and any hot path untouched** — ✅
   `git diff HEAD -- src/claim.rs` empty. The only `src/` diffs vs HEAD are
   `src/format/json.rs` and `src/format/mod.rs` — those belong to other beads'
   list/ready JSON-formatting work (commits `12f5d64` bf-doiq, `b21abf1` bf-64zt), not
   the claim/close path. `grep` for the claim hot path (`BEGIN IMMEDIATE`/`fn claim`/
   `fn close`) lands in `src/{bead_store,close,claim,batch}.rs`, `src/storage/{schema,sqlite}.rs`,
   and `src/cli/mod.rs` — none modified by this slice.

## Working-tree hygiene

No drift between the working-tree scripts and `fef0340`
(`git diff HEAD -- scripts/bf-checkpoint.sh deploy/bf-checkpoint.sh` empty), so the code
verified above is exactly what is committed. All runtime tests ran in `/tmp` scratch
workspaces; the shared `/home/coding/bead-forge` repo was not mutated.
