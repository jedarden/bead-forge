# bf-update.timer Fleet Rollout Guide

This document provides step-by-step deployment instructions for each fleet host.

> **⚠️ AUDIT 2026-07-22 (bead bf-u4fxh) — premises did not match reality.** A live audit
> found: (1) the host this repo lives on is named **`lab`** and is **NixOS** — it already
> has the timer deployed with the correct `systemd/` NixOS variant, so the "deploy
> `deploy/` (Debian) variant to lab" step is moot and would break it; (2) **no host named
> `kalshi-interserver` exists** on the Tailscale mesh (all 73 nodes scanned), so its
> timer cannot be deployed from here; (3) the repo currently has **zero published GitHub
> Releases** (`releases/latest` → 404), so `bf-update.service` fails every hourly run on
> every host — deploying the timer to more hosts would not fix the underlying
> auto-update. Full evidence and remediation in [`notes/bf-u4fxh.md`](../notes/bf-u4fxh.md).
> The status table below has been corrected to the verified state; unverified rows are
> annotated as such.

## Quick Host Detection

```bash
# Run this to determine which variant to use
if [ -d /nix/var/nix/profiles/system ] || [ -d /run/current-system ]; then
    echo "NixOS host → use systemd/ variant"
    cd ~/bead-forge && cp systemd/bf-update.{service,timer} ~/.config/systemd/user/
else
    echo "Debian/Portable host → use deploy/ variant"
    cd ~/bead-forge && cp deploy/bf-update.{service,timer} ~/.config/systemd/user/
fi
```

## bf-checkpoint.timer — deploys identically (inert by default)

The `bf-checkpoint` units ship alongside `bf-update` and deploy the **same way**: same host-variant split (`deploy/` for Debian/Ubuntu, `systemd/` for NixOS), same `~/.config/systemd/user/` install target, same `OnCalendar=hourly` cadence. Per-host, after the `bf-update` copy block above, also copy:

```bash
# Debian/Portable host
cp deploy/bf-checkpoint.sh ~/.local/bin/bf-checkpoint.sh
chmod +x ~/.local/bin/bf-checkpoint.sh
cp deploy/bf-checkpoint.{service,timer} ~/.config/systemd/user/

# NixOS host (script comes from scripts/, units from systemd/)
cp scripts/bf-checkpoint.sh ~/.local/bin/bf-checkpoint.sh
chmod +x ~/.local/bin/bf-checkpoint.sh
cp systemd/bf-checkpoint.{service,timer} ~/.config/systemd/user/

systemctl --user daemon-reload
systemctl --user enable --now bf-checkpoint.timer
```

**Safe to roll out everywhere immediately** — unlike `bf-update`, the checkpoint timer is **inert by default**: `bf-checkpoint.sh` exits 0 unless `checkpoint.enabled: true` is set in a workspace's `.beads/config.yaml`. Deploying the timer does not commit or push anything until a maintainer opts in per workspace. Push is off by default too (`checkpoint.push: true` to enable). See [`README.md`](README.md#bf-checkpoint--periodic-beads-git-checkpoint-adr-1).

## Per-Host Deployment

### 1. Hetzner (this host) ✅ COMPLETE

**OS:** Debian  
**Variant:** `deploy/`  
**Status:** Timer installed and running since 2026-06-21  
**Timer Path:** `~/.config/systemd/user/bf-update.timer`

```bash
# Verify status
systemctl --user status bf-update.timer
systemctl --user list-timers | grep bf-update
```

### 2. kalshi-interserver VPS (NixOS) ⚠️ PENDING

**OS:** NixOS  
**Variant:** `systemd/` (hardcoded `/run/current-system/sw/bin/bash`)  
**Access:** Tailscale mesh  
**Action Required:** Deploy timer

```bash
# SSH to kalshi-interserver via Tailscale
ssh kalshi-interserver

# Clone/update bead-forge repo
cd ~/bead-forge
git pull

# Deploy NixOS variant
cp systemd/bf-update.{service,timer} ~/.config/systemd/user/
cp scripts/bf-update.sh ~/.local/bin/
chmod +x ~/.local/bin/bf-update.sh

# Enable and start timer
systemctl --user daemon-reload
systemctl --user enable bf-update.timer
systemctl --user start bf-update.timer

# Verify
systemctl --user status bf-update.timer
```

### 3. lab (Debian) ⚠️ PENDING

**OS:** Debian  
**Variant:** `deploy/` (portable PATH)  
**Access:** Local network  
**Action Required:** Full deployment (service + timer)

```bash
# SSH to lab
ssh lab

# Clone/update bead-forge repo
cd ~/bead-forge
git pull

# Deploy portable variant
cp deploy/bf-update.{service,timer} ~/.config/systemd/user/
cp deploy/bf-update.sh ~/.local/bin/
chmod +x ~/.local/bin/bf-update.sh

# Enable and start timer
systemctl --user daemon-reload
systemctl --user enable bf-update.timer
systemctl --user start bf-update.timer

# Verify
systemctl --user status bf-update.timer
```

## Verification Checklist

After deploying to each host, verify:

```bash
# 1. Timer is enabled and active
systemctl --user status bf-update.timer

# 2. Next run scheduled
systemctl --user list-timers | grep bf-update

# 3. Service works manually
systemctl --user start bf-update.service
journalctl --user -u bf-update.service -n 20

# 4. Version tracking works
cat ~/.local/bin/.bf-version
```

## Troubleshooting

### Timer won't start

```bash
# Check for syntax errors
systemctl --user daemon-reload

# View detailed logs
journalctl --user -u bf-update.service -n 50 --no-pager
```

### Script fails on NixOS

Ensure the script uses the NixOS variant with hardcoded bash:
```bash
# Should use: ExecStart=/run/current-system/sw/bin/bash %h/.local/bin/bf-update.sh
grep ExecStart ~/.config/systemd/user/bf-update.service
```

### Script fails on Debian

Ensure the service relies on PATH:
```bash
# Should use: ExecStart=%h/.local/bin/bf-update.sh
# And PATH=/usr/bin:/bin
grep -E "ExecStart|PATH" ~/.config/systemd/user/bf-update.service
```

## Rollout Status

| Host | OS | Variant | Service | Timer | Status |
|------|-----|---------|---------|-------|--------|
| lab (this host) | NixOS | systemd/ | ✅ | ✅ | Deployed (active/waiting); **service failing every run — repo has 0 published Releases** (audit 2026-07-22) |
| Hetzner (`hetzner-ex44`) | ? | ? | ? | ? | Unverified from `lab` (SSH denied); separate node on mesh. Do not assume "this host" == Hetzner. |
| kalshi-interserver | NixOS? | systemd/ | ? | ? | **Not found** on Tailscale mesh (audit 2026-07-22) — clarify/retire this row |
