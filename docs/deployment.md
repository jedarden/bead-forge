# bead-forge Auto-Deployment

## Overview

The `bf` binary is automatically deployed to fleet hosts after each GitHub release via a systemd user timer. There are two systemd service variants for different OS environments.

## Host Variants

| Directory | OS Type | Service Path | Bash Path |
|-----------|---------|---------------|-----------|
| `deploy/` | **Debian/Ubuntu** (portable) | `ExecStart=%h/.local/bin/bf-update.sh` | Relies on `PATH=/usr/bin:/bin` |
| `systemd/` | **NixOS** (hardcoded) | `ExecStart=/run/current-system/sw/bin/bash %h/.local/bin/bf-update.sh` | Hardcoded NixOS bash |

### Host Detection

To determine which variant to use on a host:

```bash
# Check if NixOS
if [ -d /nix/var/nix/profiles/system ] || [ -d /run/current-system ]; then
    echo "NixOS host → use systemd/ variant"
else
    echo "Debian/Portable host → use deploy/ variant"
fi
```

### Current Fleet Deployment Status

| Host | OS | Variant | Status | Notes |
|------|-----|---------|--------|-------|
| **lab (this host)** | NixOS | `systemd/` | ✅ **Deployed & active** | Timer correct; **service fails every run — repo has 0 published Releases** (audit 2026-07-22, see [`../notes/bf-u4fxh.md`](../notes/bf-u4fxh.md)) |
| **Hetzner (`hetzner-ex44`)** | ? | ? | ❔ **Unverified** | Separate mesh node; SSH denied from `lab` — do not assume "this host" == Hetzner |
| **kalshi-interserver VPS** | NixOS? | `systemd/` | ❔ **Not found** | Absent from Tailscale mesh (audit 2026-07-22) — clarify or retire |

## How It Works

1. **CI Release**: The `bead-forge-build` workflow builds the binary and creates a GitHub release with the `bf-linux-x86_64` asset.

2. **Auto-Update Script**: Each host runs `~/.local/bin/bf-update.sh` which:
   - Queries GitHub API for the latest release
   - Downloads `bf-linux-x86_64` if newer than current
   - Backs up the binary being replaced to `~/.local/bin/bf.previous` (and its version to `.bf-version.previous`)
   - Installs to `~/.local/bin/bf`
   - Runs automatically hourly via systemd timer

3. **Manual Update**: To force an update immediately:
   ```bash
   ~/.local/bin/bf-update.sh
   # or
   systemctl --user start bf-update.service
   ```

4. **Rollback**: Every successful update first copies the outgoing `bf` to
   `bf.previous` (and its version to `.bf-version.previous`), so a bad release
   can be undone without re-downloading anything. To restore the previous
   binary on a host where a release broke `bf`:
   ```bash
   # Restore bf.previous → bf and .bf-version.previous → .bf-version
   ~/.local/bin/bf-update.sh --rollback

   # If the broken release is still the latest, also stop the timer or it will
   # re-install the bad binary within the hour:
   systemctl --user stop bf-update.timer
   ```
   `--rollback` is idempotent (running it twice restores the same binary). It
   exits with an error if no `bf.previous` exists (e.g. on a first install).

## Installation

### Debian/Ubuntu Hosts (deploy/ variant)

```bash
# From the bead-forge repository
cd deploy/

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
```

### NixOS Hosts (systemd/ variant)

```bash
# From the bead-forge repository
cd systemd/

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
```

## Checking Timer Status

```bash
# Check if timer is active
systemctl --user status bf-update.timer

# View last run logs
journalctl --user -u bf-update.service -n 20

# Manually trigger update
systemctl --user start bf-update.service
```

## Periodic `.beads/` Checkpoint (ADR-1)

Alongside the binary auto-updater, the fleet ships an **opt-in** periodic checkpoint timer that keeps each workspace's `.beads/` state committed to git. Full rationale and consequences are recorded in [ADR-1: Periodic `.beads/` git checkpoint timer](plan/plan.md#adr-1-periodic-beads-git-checkpoint-timer-2026-07-20).

### What it does

Every hour (default), `bf-checkpoint.timer` runs `bf-checkpoint.sh`, which:

1. **Flushes SQLite → JSONL** via `bf sync --flush-only` (the same explicit checkpoint the recovery path uses).
2. **Detects** whether `.beads/issues.jsonl` changed in git.
3. If it did, **stages only `.beads/issues.jsonl`** and commits it as `chore(beads): auto-checkpoint <UTC>` with a fixed identity.

It **never** stages `beads.db` (gitignored, rebuilt from JSONL via `bf sync --import`) and **never** runs on the `bf` claim/close hot path — it is invoked solely by the systemd timer, so it cannot regress claim latency or contend with the `BEGIN IMMEDIATE` write lock (ADR-1: out-of-band only).

### Disabled by default — opt in per workspace

> **New rollouts default to `checkpoint.enabled: false`.** A freshly enabled timer has **no side effects** — `bf-checkpoint.sh` prints `checkpoint disabled` and exits 0 until a maintainer sets `checkpoint.enabled: true` for that workspace.

```yaml
# .beads/config.yaml
checkpoint:
  enabled: false           # master switch — DEFAULT FALSE; opt in per workspace
  interval_minutes: 60     # min gap between commits (self-throttle), default 60
  push: false              # git push after each commit, default false
```

`push` is off by default too — commits stay local unless `checkpoint.push: true` (persistent) or the script is run with `--push` (one-shot). Do **not** add `--push` to the unit's `ExecStart`, which would force-push on every timer fire.

### Install / status

The units ship in `deploy/` (Debian/Ubuntu) and `systemd/` (NixOS) alongside the `bf-update` units and install the same way — see [`../deploy/README.md`](../deploy/README.md#bf-checkpoint--periodic-beads-git-checkpoint-adr-1) and [`../systemd/README.md`](../systemd/README.md#bf-checkpoint--periodic-beads-git-checkpoint-adr-1).

```bash
systemctl --user list-timers bf-checkpoint.timer
journalctl --user -u bf-checkpoint.service -n 50
```

## Source Files

- `~/.local/bin/bf-update.sh` - Update script (source in `scripts/bf-update.sh`)
- `~/.config/systemd/user/bf-update.service` - Systemd service unit
  - Debian: `deploy/bf-update.service`
  - NixOS: `systemd/bf-update.service`
- `~/.config/systemd/user/bf-update.timer` - Systemd timer (hourly check, source in both `deploy/` and `systemd/`)
