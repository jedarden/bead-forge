# bf-4gkg5 — Scaffold bf-checkpoint.sh config gating + workspace resolution (verification)

**Task:** Own and verify slice #1 of parent `bf-3cu1k` (the ADR-1 `bf-checkpoint.sh`).
This slice covers ONLY the startup / config-gating / workspace-resolution portion:
shebang split, PATH bootstrap, `set -euo pipefail`, `--help`/`print_usage`, arg parsing,
the `-w` → `$BF_WORKSPACE` → `$PWD` → `$HOME/bead-forge` workspace chain, the
`is_true()` / `cfg_get()` helpers, reading `checkpoint.enabled` / `.interval_minutes` /
`.push` via `bf config get`, and the `enabled != true` NOOP-exit-0 path. The
flush/diff/commit/push tail is owned by later siblings; `src/claim.rs` and the hot path
must stay untouched.

**Outcome:** No source change required. The script was committed whole in `fef0340`
(parent `bf-3cu1k`); this slice's startup + config-gating logic is already present and
correct in both `deploy/bf-checkpoint.sh` and `scripts/bf-checkpoint.sh`. This note
records the slice-#1 verification against every acceptance criterion.

## Acceptance criteria — all satisfied by committed code

1. **`bash -n` passes on both variants** — ✅
   ```
   bash -n deploy/bf-checkpoint.sh   -> PASS
   bash -n scripts/bf-checkpoint.sh  -> PASS
   ```

2. **`checkpoint.enabled=false` (or no block) prints a "checkpoint disabled" message and
   exits 0 WITHOUT flushing, staging, or committing** — ✅
   `scripts/bf-checkpoint.sh:103-106` checks `is_true "$ENABLED"` immediately after reading
   config and before the throttle, git-preflight, or `bf sync --flush-only` (line 141). On
   this workspace there is no `checkpoint:` block, so `cfg_get checkpoint.enabled false`
   yields `false` and the script exits 0 at the gate:
   ```
   checkpoint disabled for /home/coding/bead-forge (checkpoint.enabled != true) — nothing to do
   [exit=0]
   ```
   No flush (control flow never reaches line 141), no `git add`, no commit. The only `git`
   call before the gate is absent — `git -C` first appears at the post-gate preflight
   (line 128). The `deploy/` variant was exercised the same way via `bash deploy/...`
   (its `#!/bin/bash` shebang cannot be exec'd directly on this NixOS box, which has no
   `/bin/bash` — that is the entire reason the split exists; logic is identical).

3. **`-w <dir>` / `--workspace <dir>` resolves correctly; a bogus dir errors with exit 1** — ✅
   `scripts/bf-checkpoint.sh:79-84` resolves the chosen workspace to an absolute `pwd` and
   falls through to the `ERROR: workspace directory not found` + `exit 1` arm on failure:
   ```
   scripts/bf-checkpoint.sh -w /no/such/dir/xyz
   -> ERROR: workspace directory not found: /no/such/dir/xyz   [exit=1]
   bash deploy/bf-checkpoint.sh --workspace /no/such/dir/xyz
   -> ERROR: workspace directory not found: /no/such/dir/xyz   [exit=1]
   ```
   A valid `-w "$PWD"` resolves to the absolute path (`/home/coding/bead-forge`) shown in
   the disabled message. Default chain (`-w` unset, no `$BF_WORKSPACE`) lands on `$PWD`.

4. **`deploy/` vs `scripts/` differ ONLY in the shebang line and the one variant-descriptive comment** — ✅
   `diff deploy/bf-checkpoint.sh scripts/bf-checkpoint.sh` reports exactly two hunks:
   - line 1: `#!/bin/bash` vs `#!/usr/bin/env bash` (the shebang split, mirroring
     `deploy/bf-update.sh` / `scripts/bf-update.sh`, which differ by the shebang alone), and
   - the single variant-description comment block (lines 12-14) — "Debian/Ubuntu portable
     variant … see systemd/README.md" vs "NixOS portable variant … see deploy/README.md".
   All executable logic (PATH bootstrap, arg parsing, workspace chain, `is_true`/`cfg_get`,
   config read, disabled gate, and the later-sibling throttle/flush/commit tail) is
   byte-identical between the two files.

5. **`src/claim.rs` and the hot path untouched** — ✅
   `git status --porcelain -- src/` is empty; `git diff --quiet HEAD -- src/claim.rs`
   confirms `claim.rs` is unchanged. No file under `src/` was modified for this bead.

## Scope-respect check

Nothing in the later-sibling-owned tail (throttle `STATE_FILE` logic, git preflight,
`bf sync --flush-only`, change detection, stage/commit, push) was altered — it is the
parent's committed logic and is left as-is for the owning siblings to verify. The arg
parser also rejects unknown options with `exit 2` (`-- end-of-options` and `-h/--help`
both honored), confirming the startup slice's parser contract.
