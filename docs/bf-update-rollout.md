# bf-update Fleet Rollout Guide

## Overview

This document tracks the deployment status of `bf-update` systemd timer across the bead-forge fleet and provides rollout instructions for remaining hosts.

> **⚠️ AUDIT 2026-07-22 (bead bf-u4fxh) — fleet table was inaccurate.** Verified live: the
> host this repo lives on is **`lab`** and is **NixOS**, and it **already has the timer
> deployed** (`systemd/` variant, active/waiting) — so the prior "lab / Debian / not
> deployed" row was wrong on all three counts. **No `kalshi-interserver` node exists** on
> the Tailscale mesh. And the repo has **zero published GitHub Releases**, so
> `bf-update.service` fails every hourly run regardless of timer deployment. Full evidence
> and remediation in [`notes/bf-u4fxh.md`](../notes/bf-u4fxh.md). Tables below corrected
> to the verified state; unverified rows annotated.

## Host Variants

There are **two** systemd service variants for different OS environments:

| Directory | OS Type | Service Path | Bash Path |
|-----------|---------|---------------|-----------|
| `deploy/` | **Debian/Ubuntu** (portable) | `ExecStart=%h/.local/bin/bf-update.sh` | Relies on `PATH=/usr/bin:/bin` |
| `systemd/` | **NixOS** (hardcoded) | `ExecStart=/run/current-system/sw/bin/bash %h/.local/bin/bf-update.sh` | Hardcoded NixOS bash |

### Why Two Variants?

The NixOS variant hardcodes the bash path because NixOS uses a non-standard filesystem layout (`/run/current-system/sw/bin/bash`). The Debian variant is portable and works across standard Linux distributions.

## Fleet Deployment Status

| Host | OS | Variant | Service | Timer | Installed | Notes |
|------|-----|---------|---------|-------|-----------|-------|
| **lab (this host)** | NixOS | `systemd/` | ✅ | ✅ | active/waiting | Timer deployed & correct; **service failing every run — repo has 0 published Releases** (audit 2026-07-22) |
| **Hetzner (`hetzner-ex44`)** | ? | ? | ? | ? | unverified | Separate mesh node; SSH denied from `lab`. Do not assume "this host" == Hetzner. |
| **kalshi-interserver VPS** | NixOS? | `systemd/` | ? | ? | n/a | **Not found** on Tailscale mesh (audit 2026-07-22) — clarify/retire |

## Host Detection

To determine which variant to use on a host:

```bash
# Check if NixOS
if [ -d /nix/var/nix/profiles/system ] || [ -d /run/current-system ]; then
    echo "NixOS host → use systemd/ variant"
else
    echo "Debian/Portable host → use deploy/ variant"
fi
```

## Rollout Instructions

### 1. Debian/Ubuntu Hosts (deploy/ variant)

Use this for: **lab**, any future Debian hosts

```bash
# From the bead-forge repository
cd /path/to/bead-forge/deploy/

# Install service and timer
cp bf-update.service ~/.config/systemd/user/
cp bf-update.timer ~/.config/systemd/user/

# Make sure script is executable
chmod +x bf-update.sh

# Reload systemd and enable timer
systemctl --user daemon-reload
systemctl --user enable bf-update.timer
systemctl --user start bf-update.timer

# Verify
systemctl --user status bf-update.timer
systemctl --user list-timers bf-update.timer
```

### 2. NixOS Hosts (systemd/ variant)

Use this for: **kalshi-interserver VPS**, any future NixOS hosts

```bash
# From the bead-forge repository
cd /path/to/bead-forge/systemd/

# Copy the script to ~/.local/bin
cp ../scripts/bf-update.sh ~/.local/bin/bf-update.sh
chmod +x ~/.local/bin/bf-update.sh

# Copy systemd units to ~/.config/systemd/user
cp bf-update.{service,timer} ~/.config/systemd/user/

# Reload systemd and enable the timer
systemctl --user daemon-reload
systemctl --user enable bf-update.timer
systemctl --user start bf-update.timer

# Verify it's running
systemctl --user status bf-update.timer
systemctl --user list-timers bf-update.timer
```

## Verification Commands

```bash
# Check if timer is active
systemctl --user status bf-update.timer

# View last run logs
journalctl --user -u bf-update.service -n 20

# View next scheduled run
systemctl --user list-timers | grep bf-update

# Manually trigger update (test)
systemctl --user start bf-update.service

# Check installed version
~/.local/bin/bf --version
cat ~/.local/bin/.bf-version
```

## Current Installation Details

### Hetzner (Debian) - ✅ Complete

- **Variant:** `deploy/bf-update.service` (portable)
- **Installed:** 2026-06-21
- **Timer Status:** Active, running hourly
- **Next Run:** Scheduled automatically (persistent timer)
- **Service Path:** `~/.config/systemd/user/bf-update.service`
- **Timer Path:** `~/.config/systemd/user/bf-update.timer`

Service file content:
```ini
[Service]
Type=oneshot
Environment=HOME=%h
Environment=PATH=/usr/bin:/bin
Environment=TMPDIR=/tmp
ExecStart=%h/.local/bin/bf-update.sh
Nice=19
IOSchedulingClass=idle
IOSchedulingPriority=7
```

## Files Reference

| File | Purpose | Variant |
|------|---------|---------|
| `deploy/bf-update.service` | Debian systemd unit | Portable |
| `deploy/bf-update.timer` | Timer unit (shared) | Both |
| `systemd/bf-update.service` | NixOS systemd unit | NixOS-specific |
| `systemd/bf-update.timer` | Timer unit (shared) | Both |
| `scripts/bf-update.sh` | Update script | Shared |
| `deploy/bf-update.sh` | Update script copy | For deploy variant |

## Troubleshooting

### Timer not triggering

```bash
# Check if timer is enabled
systemctl --user is-enabled bf-update.timer

# Check timer status
systemctl --user status bf-update.timer

# Manually trigger to test
systemctl --user start bf-update.service

# Check logs
journalctl --user -u bf-update.service -n 50 --no-pager
```

### Wrong bash path on NixOS

If you see "bash: not found" errors, you're using the Debian variant on a NixOS host. Switch to the `systemd/` variant.

### PATH issues on Debian

If the script can't find commands, ensure the `deploy/bf-update.service` has `Environment=PATH=/usr/bin:/bin`.

## Rollout Checklist

- [x] **lab (this host)** - `systemd/` NixOS variant + timer already deployed & active (audit 2026-07-22)
- [ ] **Publish a GitHub Release** so `bf-update.service` can fetch it (currently 0 Releases → every run fails) — true blocker for auto-update anywhere
- [ ] **kalshi-interserver VPS** - node absent from Tailscale mesh; clarify what it maps to or retire the row
- [ ] Verify all hosts are receiving auto-updates after a release exists
- [ ] Document any host-specific issues
- [x] Reconcile "Hetzner (this host)" vs `lab` identity (audit 2026-07-22: this host is `lab`, NixOS)

## Related Documentation

- `deploy/README.md` - Debian/Ubuntu variant documentation
- `systemd/README.md` - NixOS variant documentation
- `docs/deployment.md` - General deployment overview
