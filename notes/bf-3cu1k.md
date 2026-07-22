# bf-3cu1k — bf-checkpoint.sh flush/diff/commit script (ADR-1)

## Status: VERIFIED & CLOSED

The implementation (`deploy/bf-checkpoint.sh` + `scripts/bf-checkpoint.sh`) was
committed in `fef0340` and is already on `origin/main`. This run independently
verified every acceptance criterion end-to-end before closing.

## Deliverable

Both variants (identical logic, mirroring the `bf-update.sh` split):

| Variant        | Shebang             | Target           |
|----------------|---------------------|------------------|
| `deploy/`      | `#!/bin/bash`       | Debian/Ubuntu    |
| `scripts/`     | `#!/usr/bin/env bash` | NixOS          |

The only difference between the two is the explanatory header comment (which
variant it is) — the body logic is byte-identical.

## Flow

1. `bf config get checkpoint.enabled` → if not true, print `checkpoint disabled`
   and exit 0 (master switch, default off).
2. Self-throttle: per-workspace state file under
   `$XDG_STATE_HOME/bf-checkpoint/<slug>.last`; skip if last checkpoint was <
   `checkpoint.interval_minutes` ago.
3. `bf sync --flush-only` → SQLite → JSONL.
4. Detect change via `git diff --quiet -- .beads/issues.jsonl` **and**
   `git status --porcelain` (the latter catches a first-time untracked file the
   flush just created).
5. `git add .beads/issues.jsonl` + `git commit -- .beads/issues.jsonl` (pathspec
   constrained so nothing else can ride along), fixed identity
   `github@jedarden.com` / `jedarden`, message prefix
   `chore(beads): auto-checkpoint`.
6. `--push` (default off) or persistent `checkpoint.push=true` → `git push`.

`beads.db` is NEVER staged: the script adds only the explicit `issues.jsonl`
path, and `bf init` gitignores `beads.db` regardless.

## Verification (this run)

Static:
- `bash -n` clean on both variants.
- `shellcheck` not installed on this box → "clean if available" clause N/A.
- Shebangs: `deploy` = `#!/bin/bash`, `scripts` = `#!/usr/bin/env bash`.
- `diff` of the two variants after line 1: only the header comment differs.
- `git show --stat fef0340` touched ONLY the two `.sh` files — `src/claim.rs` and
  the claim/close hot path untouched (ADR-1 out-of-band only).

Functional (throwaway repo in `~/scratch/cp-test`, `checkpoint.enabled=true`):
1. New commit created after a db-only bead.           PASS
2. Message prefix `chore(beads): auto-checkpoint`.      PASS
3. Author identity `jedarden <github@jedarden.com>`.    PASS
4. ONLY `.beads/issues.jsonl` in the commit (1 file).   PASS
5. `beads.db` on disk (348 KB) but absent from git tree / untracked. PASS
6. `--push` omitted + `push=false` → no push attempt.   PASS
7. Immediate second run → `last checkpoint < 60m ago ... skipping` (throttle). PASS
8. `scripts/` (NixOS) variant behaves identically.      PASS
9. `--push` flag attempts `git push` (exit 128 only because the probe repo has no remote). PASS
10. `push=false` & no `--push` → no `Pushing` line.      PASS

Enabled check on the real workspace (no `checkpoint:` block):
`bf config get checkpoint.enabled` → `false`, `.interval_minutes` → `60`,
`.push` → `false`; script prints `checkpoint disabled ...` and exits 0.  PASS

## Scope discipline (left untouched)

- `src/claim.rs` and the claim/close hot path — out-of-band only per ADR-1.
- `deploy/bf-checkpoint.service`, `*.timer`, `systemd/*` — those are a sibling
  deliverable (parent bf-48pw0 scope #3), NOT bf-3cu1k. Left as untracked.
