# bf-u4fxh: bf-update.timer Fleet Rollout — Current Status (2026-07-25)

## Executive Summary

The task requested deployment of `bf-update.timer` to **lab** (Debian) and **kalshi-interserver VPS** (NixOS). A live audit on 2026-07-25 confirms the findings from the previous 2026-07-22 audit: **this task cannot be completed as specified** due to three fundamental issues:

1. **Host identity mismatch**: "This host" is `lab` (NixOS), not a Hetzner Debian box
2. **Missing target host**: `kalshi-interserver` does not exist on the Tailscale mesh
3. **No releases to deploy**: The repo has **zero published GitHub Releases**, so auto-update fails everywhere

## Current Fleet Status

### ✅ lab (this host) — ALREADY DEPLOYED

**Hostname**: `lab`  
**OS**: NixOS 25.05 "Warbler"  
**Variant**: `systemd/` (NixOS-specific, hardcoded bash)  
**Timer**: ✅ Active and waiting  
**Service**: ❌ Fails every run (no GitHub releases)  
**Current version**: v0.3.0

```bash
$ systemctl --user status bf-update.timer
● bf-update.timer - Hourly check for bf updates
     Active: active (waiting) since Thu 2026-06-11 10:47:14 EDT

$ cat ~/.local/bin/.bf-version
v0.3.0

$ journalctl --user -u bf-update.service -n 5
Jul 25 07:01:31 lab bash[3449007]: Downloading bf-linux-x86_64 from release null...
Jul 25 07:01:31 lab bash[3449018]: jq: error (at <stdin>:5): Cannot iterate over null (null)
Jul 25 07:01:31 lab systemd[3278]: bf-update.service: Main process exited, code=exited, status=5/NOTINSTALLED
```

**Status**: The timer is correctly deployed with the proper NixOS `systemd/` variant. No deployment work is needed on lab.

### ❌ hetzner-ex44 — UNREACHABLE

**IP**: 100.72.170.64  
**Status**: Present on Tailscale mesh but **SSH access denied**

```bash
$ ping -c 2 hetzner-ex44
64 bytes from hetzner-ex44.tail1b1987.ts.net (100.72.170.64): icmp_seq=1 ttl=64 time=234 ms

$ ssh hetzner-ex44 "hostname"
coding@hetzner-ex44: Permission denied (publickey).
```

**Status**: Cannot verify or deploy without SSH credentials. This is a separate node from `lab`.

### ❌ kalshi-interserver VPS — DOES NOT EXIST

**Status**: **Absent from Tailscale mesh** (all 73+ nodes scanned)

```bash
$ tailscale status | grep -i kalshi
100.87.68.12     iad-kalshi
100.119.222.68   kubectl-proxy-iad-kalshi
100.93.235.82    traefik-iad-kalshi
```

**Status**: No host named `kalshi-interserver` exists on this network. The closest matches are `iad-kalshi` nodes, but none match the target hostname.

## Root Cause: Zero GitHub Releases

Even on hosts where the timer IS deployed (like `lab`), the `bf-update.service` fails every hour because the repository has no published releases:

```bash
$ curl -s https://api.github.com/repos/jedarden/bead-forge/releases/latest
404 Not Found

$ curl -s https://api.github.com/repos/jedarden/bead-forge/releases
[]
```

Git **tags** exist (`v0.3.0`, `v0.2.0`, `v0.1.0`) but no GitHub **Releases** are published against them. The `bf-update.sh` script expects release assets and fails when it finds `null`.

**Impact**: Auto-update is broken fleet-wide, regardless of timer deployment status.

## Documentation Status

The rollout documentation has been **corrected** to reflect the verified state:

- ✅ `deploy/ROLLOUT.md` — Updated with audit findings and corrected fleet table
- ✅ `docs/bf-update-rollout.md` — Updated with audit findings and corrected fleet table  
- ✅ `docs/deployment.md` — Shows correct lab status as NixOS/deployed
- ✅ `notes/bf-u4fxh.md` — Detailed audit from 2026-07-22

All documentation now correctly identifies:
- `lab` as NixOS with timer already deployed
- `kalshi-interserver` as absent from mesh
- `hetzner-ex44` as a separate unreachable node
- The GitHub Releases issue as the root cause

## What Would Enable Completion

1. **Publish a GitHub Release** for v0.3.0 with `bf-linux-x86_64` asset (or re-run `bead-forge-build` workflow)
2. **Clarify `kalshi-interserver`** — what node it maps to, or retire the row from fleet tables
3. **Provide SSH access** to `hetzner-ex44` for deployment verification

## Conclusion

This task **cannot be completed as specified** because:
- The target hosts either don't exist (`kalshi-interserver`) or are unreachable (`hetzner-ex44`)
- The host that IS reachable (`lab`) already has the correct deployment
- The underlying auto-update mechanism is broken due to missing GitHub releases

The documentation has been corrected to reflect reality. Further work requires either SSH credentials for the remaining nodes or clarification of the actual target hostnames.
