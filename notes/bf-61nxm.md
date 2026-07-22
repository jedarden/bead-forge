# bf-61nxm — Stage-only-issues.jsonl commit path with fixed git identity (verification)

**Task:** Own and verify slice #3 of parent `bf-3cu1k` (the ADR-1 `bf-checkpoint.sh`) — the
**highest-risk slice**, the one that failed verification on the parent and is therefore isolated
here. Scope is **only** the stage/commit/identity tail: stage ONLY `.beads/issues.jsonl`, constrain
the commit to that pathspec so nothing else staged can ride along, fixed identity
`github@jedarden.com` / `jedarden`, message prefix EXACTLY `chore(beads): auto-checkpoint`, and
`beads.db` NEVER staged or committed. `src/claim.rs` and any claim/close hot path must stay untouched.

**Outcome:** No source change required. The script was committed whole in `fef0340` (parent
`bf-3cu1k`); this slice's stage/commit/identity logic is already present and correct in **both**
`deploy/bf-checkpoint.sh` and `scripts/bf-checkpoint.sh` (identical logic — they differ only by
shebang and a descriptive comment). This note records the slice-#3 verification against every
acceptance criterion, including end-to-end runtime tests against isolated `/tmp` workspaces. The
real `/home/coding/bead-forge` tree is shared across needle agents and was **not** mutated by any
test (all tests used throwaway `/tmp/cp-61nxm-*` dirs).

## Slice scope as committed (both variants, `:159-169`)

```bash
# --- 3. stage ONLY issues.jsonl and commit ---------------------------------
git -C "$WORKSPACE" add .beads/issues.jsonl
MSG="chore(beads): auto-checkpoint $(date -u +%Y-%m-%dT%H:%M:%SZ)"
git -C "$WORKSPACE" \
    -c user.email=github@jedarden.com \
    -c user.name=jedarden \
    commit -m "$MSG" -- .beads/issues.jsonl
```

- **stage only issues.jsonl** — the lone `git add` targets the single path `.beads/issues.jsonl`;
  `beads.db` is never passed to `add`.
- **pathspec-constrained commit** — `commit -m "$MSG" -- .beads/issues.jsonl` limits the commit to
  that pathspec, so files already staged in the index by anything else cannot ride along (proven by
  the decoy test below).
- **fixed identity** — `-c user.email` / `-c user.name` are scoped to this one `git` invocation and
  set **both** author and committer (see identity test below).
- **message prefix** — `chore(beads): auto-checkpoint ` + a UTC `date -u +%Y-%m-%dT%H:%M:%SZ` stamp.

`beads.db` is additionally git-ignored in every workspace: `bf init` writes `.beads/.gitignore`
containing `beads.db` / `beads.db-shm` / `beads.db-wal` (verified in the probe workspace), and the
live repo's `.beads/.gitignore` ignores `*.db*`. So `beads.db` is defended at two layers: never
`add`'d by the script, and ignored by git regardless.

## Acceptance criteria — all satisfied (empirical)

All tests run against throwaway workspaces (`/tmp/cp-61nxm-deploy`, `-scripts`, `-decoy`). The
adversarial harness set a **wrong** repo-local identity (`user.email=WRONG@example.invalid`,
`user.name=Wrong Person`) so that a correct result proves the script's `-c` overrides repo config.

### 1. `enabled=true` + uncommitted issues.jsonl → commits ONLY `.beads/issues.jsonl`

Two phases on `/tmp/cp-61nxm-deploy` (config: `enabled:true`, `interval_minutes:0`):

**RUN 1 — first-ever flush (issues.jsonl was untracked, db-only):**
```
Flushing SQLite -> JSONL for /tmp/cp-61nxm-deploy
Flushed 1 beads to JSONL
[master 06c16ac] chore(beads): auto-checkpoint 2026-07-22T13:48:48Z
 1 file changed, 1 insertion(+)
 create mode 100644 .beads/issues.jsonl
[exit=0]
git show --stat HEAD  ->  .beads/issues.jsonl | 1 +   (1 file, nothing else)
```

**RUN 2 — issues.jsonl already tracked, modified by a second bead:**
```
Flushed 2 beads to JSONL
[master a11f537] chore(beads): auto-checkpoint 2026-07-22T13:48:48Z
 1 file changed, 1 insertion(+)
[exit=0]
git show --stat HEAD  ->  .beads/issues.jsonl | 1 +   (1 file, nothing else)
```

In both runs `git diff --cached --name-only` was **empty** after the commit (nothing else left
staged) and the same result held for the `scripts/` (NixOS) variant on `/tmp/cp-61nxm-scripts`.

### 2. Dirty/untracked `beads.db` is NEVER staged or committed

`beads.db` was present in every test workspace (created by `bf init`, written by `bf sync`).
After each run:
```
git diff --cached --name-only -- .beads/beads.db   ->  (empty — not staged)
git show --name-only --format='' HEAD | grep -c beads.db  ->  0   (not in commit)
```

### 3. Commit author/committer identity = `github@jedarden.com` / `jedarden`

Repo-local config was the adversarial `WRONG@example.invalid` / `Wrong Person`, yet every checkpoint
commit recorded:
```
author    = jedarden <github@jedarden.com>
committer = jedarden <github@jedarden.com>
```
The `-c user.email` / `-c user.name` on the commit overrode the wrong repo-local config for
**both** author and committer.

### 4. Message begins with `chore(beads): auto-checkpoint`

`git log -1 --format='%s'` → `chore(beads): auto-checkpoint 2026-07-22T13:48:48Z` (exact prefix +
UTC stamp) on every commit, both variants.

### Decoy / pathspec-isolation test (the safety belt — `/tmp/cp-61nxm-decoy`)

The decisive test for "nothing else staged in the repo can ride along." Before the run, the index
was pre-poisoned: `decoy.txt` (tracked, modified + staged) and `decoy2.txt` (staged), plus an
untracked `stray.txt`. `git diff --cached --name-only` before the run listed `decoy.txt` /
`decoy2.txt`. After the run:
```
[master 6cc6934] chore(beads): auto-checkpoint 2026-07-22T13:49:29Z
 1 file changed, 1 insertion(+)   create mode 100644 .beads/issues.jsonl
git show --stat HEAD            -> ONLY .beads/issues.jsonl
git diff --cached --name-only   -> decoy.txt, decoy2.txt   (still staged, NOT committed)
beads.db in commit?             -> 0   (absent)
```
The two pre-staged decoys **stayed in the index but were not committed** — the `-- .beads/issues.jsonl`
pathspec on `git commit` isolated the commit exactly as required.

## Note: `GIT_*` env vars override `-c` (deployment precondition, not a script defect)

The acceptance specifies the identity mechanism as `-c user.email=… -c user.name=…`, which the
script implements. Verified git precedence (standalone repo):
- `-c` only (no env vars): author/committer = `jedarden <github@jedarden.com>` ✅
- `-c` **and** `GIT_AUTHOR_*` / `GIT_COMMITTER_*` env vars set: env vars win
  (`EnvAuth <env@x.invalid>` / `EnvComm <enccomm@x.invalid>`).

So the `-c` flags correctly override repo-local **and** global config (the realistic systemd-timer
environment, where those env vars are unset — which is why the test harness unsets them). Should the
deployed `bf-checkpoint.service` ever inherit `GIT_AUTHOR_*` / `GIT_COMMITTER_*`, those would
override the script. No checkpoint `.service`/`.timer` is deployed yet (only `bf-update`'s are, per
`deploy/`), so there is no current risk; the rollout precondition is simply that the unit not export
those env vars. This is a standard git-property caveat, not something this slice was asked to change.

## `src/claim.rs` / hot path — untouched

- `git show --name-only fef0340` → only `deploy/bf-checkpoint.sh` and `scripts/bf-checkpoint.sh`. No
  `src/` file touched by the commit that introduced the script.
- `git status --short -- src/claim.rs src/claim src/batch.rs src/storage/sqlite.rs` → clean (no
  changes from this slice). The only uncommitted `src/` edits in the working tree
  (`src/cli/mod.rs`, `src/format/*`) are pre-existing in-flight work from sibling beads, present at
  session start and unrelated to this slice. This commit adds only `notes/bf-61nxm.md` (single path).

## Syntax

```
bash -n deploy/bf-checkpoint.sh   -> OK
bash -n scripts/bf-checkpoint.sh  -> OK
```
(shellcheck not installed on this box; `bash -n` is the available gate.) The two variants are
byte-identical from the `set -euo pipefail` line onward — diff is confined to the shebang and the
self-describing comment block.
