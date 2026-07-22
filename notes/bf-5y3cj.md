# bf-5y3cj — bf-checkpoint.sh throttle + opt-in push + deploy/scripts parity

**Slice #4 (FINAL)** of parent bf-3cu1k (ADR-1 bf-checkpoint.sh). Depends on
bf-61nxm (commit/identity). The script already existed and was committed in
`fef0340`; this child OWNS and VERIFIES the throttle, the opt-in push, and the
final deploy/scripts parity. `src/claim.rs` and the claim/close hot path were
NOT touched.

## What this slice changed

Only the variant-description comment in the two scripts (a parity tightening).
The runtime logic (throttle, push) was already correct from `fef0340` and is
verified below, not changed.

The acceptance allows the deploy/scripts diff to differ in exactly two spots:
the shebang and "the **one** variant-description comment." The committed script
had a 3-line variant comment block, which was ambiguous against "one comment."
Collapsed it to a single focused line in both variants so parity is
unambiguous and "exactly mirrors the bf-update.sh split" (whose diff is the
shebang alone).

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

## Verification (isolated /tmp harness — never touched a real repo)

`/tmp/cp-harness.sh` builds throwaway git workspaces under `/tmp` with an
isolated `XDG_STATE_HOME`, seeds each with one commit + upstream tracking (like
a real clone), and points a bare remote at each so `git push` can actually
succeed and the remote can be inspected. Ran against BOTH variants.

**Result: 13/13 PASS on `deploy/bf-checkpoint.sh` and 13/13 PASS on
`scripts/bf-checkpoint.sh`.**

| Scenario | Expectation | Result |
|---|---|---|
| Throttle (recent state file, interval 1m) | skip before flush/commit, exit 0, `< 1m ago ... skipping` | PASS (0 new commits) |
| Push OFF (no `--push`, `push=false`) | commit locally, no `git push`, remote unchanged | PASS (remote 1→1) |
| Push ON via `--push` (config `push=false`) | `Pushing` printed, remote gains checkpoint commit | PASS (remote 1→2) |
| Push ON via config `push=true` (no `--push`) | `Pushing` printed, remote gains checkpoint commit | PASS (remote 1→2) |

Mechanics confirmed during testing:
- Throttle state file: `${XDG_STATE_HOME:-$HOME/.local/state}/bf-checkpoint/<workspace-slug>.last`,
  written on the successful-commit path and on the no-op no-change path.
- `--push` is OFF by default; `git push` runs only with `--push` OR
  `checkpoint.push=true`. Default (neither) does NOT push.
- The push decision gate is `(( PUSH_FLAG == 1 )) || is_true "$CFG_PUSH"`
  (script §4).

## Syntax / lint

- `bash -n deploy/bf-checkpoint.sh` — OK
- `bash -n scripts/bf-checkpoint.sh` — OK
- `shellcheck` — **SKIPPED**: not installed on this box. Recorded, not blocking
  (acceptance explicitly allows the skip).

## Hot-path safety

- `git diff HEAD -- src/claim.rs` empty — claim.rs untouched.
- Only files changed by this slice: `deploy/bf-checkpoint.sh`,
  `scripts/bf-checkpoint.sh` (comment collapse), plus this notes file.
