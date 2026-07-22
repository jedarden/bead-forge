# bf-u4fxh: bf-update.timer Fleet Rollout — Audit Findings (BLOCKED, not closed)

## Outcome: cannot complete as specified

The task asked to deploy `bf-update.timer` to **lab** (Debian, `deploy/` variant) and
**kalshi-interserver VPS** (NixOS, `systemd/` variant), verify, and update the rollout docs.

A live audit on 2026-07-22 found that **both deployment targets are invalid** and the
task's premises do not match reality. Per project guidance ("if what you find contradicts
how it was described, surface it instead of proceeding"), this bead is left **open /
blocked** rather than closed. Details below.

> The original task brief referenced `docs/deploy/ROLLOUT.md`; that path does not exist.
> The real rollout docs are `deploy/ROLLOUT.md`, `docs/bf-update-rollout.md`,
> `docs/deployment.md`, `deploy/README.md`, and `systemd/README.md`.

---

## 1. This host is `lab` (NixOS) — and the timer is ALREADY deployed

The task calls this host "the Hetzner host", but it is not:

```
$ hostname            → lab
$ /etc/os-release ID  → nixos   (NixOS 25.05 "Warbler")
$ tailscale ip -4     → 100.81.129.38
$ tailscale status    → 100.81.129.38  lab  (jose.cabanero@)
```

A **separate** node `hetzner-ex44` (100.72.170.64) exists on the mesh and is the actual
Hetzner box (it answers from a `2a01:4f9:` Hetzner IPv6).

The bf-update auto-update mechanism on **lab is already fully and correctly deployed** —
using the **NixOS** `systemd/` variant:

```
$ grep ExecStart ~/.config/systemd/user/bf-update.service
ExecStart=/run/current-system/sw/bin/bash %h/.local/bin/bf-update.sh     ← correct NixOS variant

$ systemctl --user status bf-update.timer
● bf-update.timer - Hourly check for bf updates
     Active: active (waiting) since 2026-06-11   ← enabled + active

$ cat ~/.local/bin/.bf-version
v0.3.0
```

**Therefore the task's "deploy the `deploy/` (Debian) variant to lab" is both moot and
harmful**: lab is NixOS, already has the correct variant, and the Debian variant would
break it (`deploy/README.md` explicitly warns the Debian unit fails on NixOS with
"bash: not found"). There is no deployment to perform on lab.

---

## 2. `kalshi-interserver` does not exist on this network

No host by that name is reachable from lab:

- Not resolvable via DNS: `getent hosts kalshi-interserver` → nothing
  (also tried `kalshi-interserver.tail1b1987.ts.net`, `interserver`, `kalshi-interserver-vps`)
- Not in `/etc/hosts`
- No `~/.ssh/config`
- **Not in the Tailscale mesh at all** — scanned all 73 tailnet nodes (online *and*
  offline). The only kalshi-named nodes are `iad-kalshi`, `kalshi-weather-server`,
  `kubectl-proxy-iad-kalshi`, `traefik-iad-kalshi` — none is "interserver".

`deploy/ROLLOUT.md` claims kalshi-interserver's access is "Tailscale mesh", but no such
node is present. It may have been removed, renamed, decommissioned, or live on a
different tailnet. **There is no SSH path to it from here, so the timer cannot be
deployed.**

Other kalshi-adjacent hosts were probed and are also unreachable from lab:
- `kalshi-weather-server` → host key added, then `Permission denied (publickey,keyboard-interactive)`
- `hetzner-ex44` → `Permission denied (publickey)` as `coding@`; `tailscale ssh` → host key unknown

---

## 3. Root cause: the repo has ZERO published GitHub Releases — auto-update is broken fleet-wide

This is the most important finding. The task assumed the timer was "running hourly at
v0.3.0, matching the latest GitHub release." It is not:

```
GET /repos/jedarden/bead-forge            → 200   (repo exists, public, default_branch=main)
GET /repos/jedarden/bead-forge/releases   → [ ]   (EMPTY — zero published Releases)
GET /repos/jedarden/bead-forge/releases/latest → 404
GET /repos/jedarden/bead-forge/tags       → v0.3.0, v0.2.0, v0.1.0   (git tags exist)
x-ratelimit-remaining: 46/60              → NOT a rate-limit problem
```

Git **tags** exist, but no GitHub **Releases** are published against them. Since
`releases/latest` 404s, `~/.local/bin/bf-update.sh` parses `null` and fails:

```
Downloading bf-linux-x86_64 from release null...
jq: error (at <stdin>:5): Cannot iterate over null (null)
Main process exited, code=exited, status=5/NOTINSTALLED
Failed to start Update bf binary from GitHub releases.
```

This is **deterministic and recurring** — every hourly invocation fails (24h sample):

```
Jul 21 18:01  … release null … status=5/NOTINSTALLED  Failed
Jul 21 19:03  … release null … status=5/NOTINSTALLED  Failed
Jul 21 20:01  … release null … status=5/NOTINSTALLED  Failed
Jul 21 21:00  … release null … status=5/NOTINSTALLED  Failed
Jul 21 22:02  … release null … status=5/NOTINSTALLED  Failed
Jul 21 23:04  … release null … status=5/NOTINSTALLED  Failed
Jul 22 00:04  … release null … status=5/NOTINSTALLED  Failed
```

**Implication:** even on lab (fully deployed), the auto-update does not actually work —
there is no release to fetch. Deploying the timer to *more* hosts (the task's ask) would
not fix this; every host would fail identically. The `bead-forge-build` Argo Workflow is
supposed to publish the `bf-linux-x86_64` release asset, but no Release currently exists.

---

## What would unblock this

1. **Publish a GitHub Release** for `v0.3.0` (re-run `bead-forge-build`, or cut one
   manually with the `bf-linux-x86_64` + `SHA256SUMS` assets). This is the actual
   blocker for auto-update working *anywhere*, and is out of scope for this
   timer-rollout bead.
2. **Clarify `kalshi-interserver`** — what node it maps to today (it is absent from
   this tailnet), or confirm it is decommissioned and drop it from the fleet table.
3. **Reconcile the "Hetzner (this host)" vs `lab` identity** in the docs — the box these
   docs live on is named `lab` and is NixOS, not "Hetzner/Debian".

## What this bead DID deliver

- `notes/bf-u4fxh.md` (this file) — the audit above.
- Truthful corrections to the rollout docs' **lab** rows (was "Debian / not deployed";
  actually NixOS / deployed, with auto-update currently failing due to §3), plus an
  audit callout pointing here. No host I could not reach was claimed deployed or
  redeployed, and the working NixOS install on lab was left untouched.
