# bf-37bzi — systemd bf-checkpoint.service/.timer units (deploy/ + systemd/)

## Status: VERIFIED & CLOSING

The four units were committed in `d6f5d8a` and are already on
`origin/needle/bf-5wku` (branch up to date). The previous run committed + pushed
but failed before `br close` (bead carries `failure-count:1`). This run
independently re-verified every acceptance criterion before closing.

## Deliverable

| File                          | Variant      | ExecStart                                                       |
|-------------------------------|--------------|-----------------------------------------------------------------|
| `deploy/bf-checkpoint.service`  | Debian/Ubuntu | `%h/.local/bin/bf-checkpoint.sh` (direct, relies on `#!/bin/bash`) |
| `deploy/bf-checkpoint.timer`    | Debian/Ubuntu | `OnCalendar=hourly` …                                           |
| `systemd/bf-checkpoint.service` | NixOS         | `/run/current-system/sw/bin/bash %h/.local/bin/bf-checkpoint.sh`  |
| `systemd/bf-checkpoint.timer`   | NixOS         | `OnCalendar=hourly` …                                           |

Both services are `Type=oneshot`, `Nice=19`, `IOSchedulingClass=idle`,
`IOSchedulingPriority=7`, with `NoNewPrivileges=true` + `PrivateTmp=true`
hardening — byte-for-byte the same hardening/scheduling block as the `bf-update`
pair. Both timers are identical to `bf-update.timer` except `Description` and
`Requires=` (verified by direct diff). Each service pins the target workspace
via `Environment=BF_WORKSPACE=%h/bead-forge` (the script honors `$BF_WORKSPACE`
over `$PWD`, since user services start in `$HOME`).

## Variant mapping (verified)

The two installable script variants differ only in shebang + header comment
(mirrors the `bf-update.sh` split exactly):

- `deploy/bf-checkpoint.sh` → `#!/bin/bash`, Debian/Ubuntu portable.
- `scripts/bf-checkpoint.sh` → `#!/usr/bin/env bash`, NixOS portable.

The `deploy/*.service` comment references `deploy/bf-checkpoint.sh`; the
`systemd/*.service` comment references `scripts/bf-checkpoint.sh` — correct
mapping for each host type.

## Acceptance criteria — all pass

1. **Both pairs present & syntactically valid.** `systemd-analyze verify`
   parses all four units with no syntax/parse errors. The `systemd/` pair
   verifies clean (no output). The `deploy/` pair emits one warning —
   `Command ~/.local/bin/bf-checkpoint.sh is not executable: No such file or
   directory` — which is purely because the script is not yet installed on this
   dev box (the units are new and undeployed). `bf-update.service` produces no
   such warning only because `~/.local/bin/bf-update.sh` is already installed
   here; pre-deploy behavior is identical. Not a unit defect.
2. **ExecStart points at the correct variant per host type.** Deploy invokes
   the script directly (Debian `#!/bin/bash`); systemd wraps it with the NixOS
   system bash (`/run/current-system/sw/bin/bash`). Matches the `bf-update`
   ExecStart pattern exactly.
3. **Inert by default.** Documented in both service comments *and* the READMEs:
   `bf-checkpoint.sh` exits 0 immediately unless `checkpoint.enabled: true` is
   set in `.beads/config.yaml` for the workspace. A freshly enabled timer does
   nothing until a maintainer opts in.
4. **Push off by default.** Documented in both service comments *and* READMEs:
   commits stay local unless `checkpoint.push: true` is set — the units never
   add `--push` to ExecStart (that would force-push on every fire).
5. **README cross-references alongside bf-update.** `deploy/README.md`,
   `systemd/README.md`, and `deploy/ROLLOUT.md` each describe `bf-checkpoint` as
   the out-of-band "companion to" / "ships alongside" `bf-update`, with
   side-by-side install instructions and shared cadence.

## Deploy notes

Per `deploy/ROLLOUT.md`, the checkpoint units deploy identically to `bf-update`
(same host-variant split, same `~/.config/systemd/user/` target, same
`OnCalendar=hourly`). Safe to roll out everywhere immediately because the timer
is inert by default — deploying it commits/pushes nothing until
`checkpoint.enabled` is set per workspace.
