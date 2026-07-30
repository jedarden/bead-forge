# bf-5y3cj — bf-checkpoint.sh throttle + opt-in push + deploy/scripts parity

**Slice #4 (FINAL)** of parent bf-3cu1k (ADR-1 bf-checkpoint.sh). Depends on
bf-61nxm (commit/identity). This bead was re-dispatched on `failure-count:1`;
this note records the **independent re-verification** done on the retry. The
runtime logic (throttle, push) was already correct from `fef0340`; the only
source change in this slice is the parity tightening of the variant comment
(commit `d8712fb`). `src/claim.rs` and the claim/close hot path were NOT touched.

## What this slice changed

Only the variant-description comment in the two scripts (a parity tightening).
The acceptance allows the deploy/scripts diff to differ in exactly two spots:
the shebang and "the **one** variant-description comment." The committed script
had a 3-line variant comment block, which was ambiguous against "one comment."
`d8712fb` collapsed it to a single focused line in both variants so parity is
unambiguous and "exactly mirrors the bf-update.sh split".

```
diff deploy/bf-checkpoint.sh scripts/bf-checkpoint.sh
1c1
< #!/bin/bash
---
> #!/usr/bin/env bash
12c12
< # Debian/Ubuntu portable variant — use scripts/bf-checkpoint.sh on NixOS.
---
> # NixOS portable variant — use deploy/bf-checkpoint.sh on Debian/Ubuntu.
```

`diff | grep -c '^[0-9]'` = **2** (exactly the two allowed hunks). This matches
the bf-update.sh split, whose diff is the shebang alone.

## Independent re-verification (retry, `/tmp/cp-verify.sh`)

**Correction to the prior note's environment assumption:** this box **is
NixOS** — `git` is at `/run/current-system/sw/bin/git` (that dir exists), and
there is **no `/bin/bash`**. So the `scripts/` variant (`#!/usr/bin/env bash`)
is the *native* one here; the `deploy/` variant's `#!/bin/bash` shebang cannot
be *executed* on this box. Per the acceptance, that is fine — it only requires
`bash -n` to pass (done) and the diff parity (done); both variants' **logic**
was exercised identically via explicit `bash <script>` invocation.

The harness builds isolated `/tmp` git workspaces (fresh per scenario → fresh
state-file slug), an isolated `XDG_STATE_HOME`, a clean git env
(`GIT_CONFIG_GLOBAL=/dev/null`), and a **bare git remote** per workspace so a
real `git push` can land and the remote ref can be inspected. A db-only bead is
seeded so the script's `bf sync --flush-only` produces a tracked change and the
commit path actually fires.

**Decisive push detection** = the script's own `Pushing (...)` output line AND
the bare-remote `main` ref moving (`git rev-list --count`). If the OFF path ever
called `git push`, `set -euo pipefail` would make it exit non-zero and/or the
remote would move — neither happens.

**Result: 20/20 PASS on `deploy/bf-checkpoint.sh` and 20/20 PASS on
`scripts/bf-checkpoint.sh`. Harness exit code 0.**

| Scenario | Expectation | Result |
|---|---|---|
| Sanity | `bf config get checkpoint.enabled` reads `true`, `push` reads `false` from workspace config | PASS×2 |
| Throttle T1 (state file pre-seeded `now`, interval 1m) | skip **before** flush/commit, exit 0, `… skipping`, no `Flushing` line | PASS×2 |
| Throttle T2 (run, then re-run within interval) | run1 commits; run2 exit 0, `… skipping`, **no new commit** | PASS×2 |
| Push OFF (no `--push`, config `push=false`) | commits locally, **no** `Pushing` line, remote unchanged | PASS×2 |
| Push ON via `--push` (config `push=false`) | `Pushing` printed, remote gains the checkpoint commit | PASS×2 |
| Push ON via config `push=true` (no `--push`) | `Pushing` printed, remote gains the checkpoint commit | PASS×2 |

Mechanics confirmed during testing:
- Throttle state file lives at
  `${XDG_STATE_HOME:-$HOME/.local/state}/bf-checkpoint/<workspace-slug>.last`
  (slug = workspace path with leading `/` dropped, `/` → `_`), written on the
  successful-commit path and on the no-op no-change path; checked **before**
  the flush (T1 proves it skips before `Flushing`).
- `--push` is OFF by default; `git push` runs only with `--push` OR
  `checkpoint.push=true`. Default (neither) does NOT push (POFF remote
  unchanged).
- Push decision gate is `(( PUSH_FLAG == 1 )) || is_true "$CFG_PUSH"` (script §4).

## Syntax / lint

- `bash -n deploy/bf-checkpoint.sh` — OK
- `bash -n scripts/bf-checkpoint.sh` — OK
- `shellcheck` — **SKIPPED**: not installed on this box (`which shellcheck`
  finds nothing). Recorded, not blocking (acceptance explicitly allows the skip).

## Hot-path safety

- `d8712fb` (this slice's commit) touched only `deploy/bf-checkpoint.sh`,
  `scripts/bf-checkpoint.sh`, and this notes file — **no `src/`**.
- `src/claim.rs` clean in the working tree (`git status --short src/claim.rs`
  empty); the claim/close hot path is untouched.
