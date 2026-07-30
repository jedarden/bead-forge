# systemd Units for bf Auto-Update (NixOS Variant)

These systemd units enable automatic hourly updates of the `bf` binary from GitHub releases.

This directory also ships the **`bf-checkpoint`** units (NixOS variant) — the out-of-band companion to `bf-update` that periodically flushes `.beads/` to JSONL and commits `issues.jsonl` (ADR-1). Install them the same way as the `bf-update` units (see [bf-checkpoint install](#bf-checkpoint--periodic-beads-git-checkpoint-adr-1) below).

> **⚠️ NixOS-Specific Variant** — This directory contains the **NixOS-specific** systemd units that hardcode `/run/current-system/sw/bin/bash`. For Debian/Ubuntu/portable hosts, use the `deploy/` directory instead.

## Host Variants

| Directory | OS Type | Service Path | Bash Path |
|-----------|---------|---------------|-----------|
| `systemd/` | **NixOS** (hardcoded) | `ExecStart=/run/current-system/sw/bin/bash %h/.local/bin/bf-update.sh` | Hardcoded NixOS bash |
| `deploy/` | **Debian/Ubuntu** (portable) | `ExecStart=%h/.local/bin/bf-update.sh` | Relies on `PATH=/usr/bin:/bin` |

### Host Detection

```bash
# Check if NixOS
if [ -d /nix/var/nix/profiles/system ] || [ -d /run/current-system ]; then
    echo "NixOS host → use systemd/ variant (this directory)"
else
    echo "Debian/Portable host → use deploy/ variant instead"
fi
```

### Current Fleet Deployment

| Host | OS | Variant | Timer Status |
|------|-----|---------|--------------|
| **lab (this host)** | NixOS | `systemd/` | ✅ Deployed & active — but **service fails every run: repo has 0 published Releases** (audit 2026-07-22, see [`../notes/bf-u4fxh.md`](../notes/bf-u4fxh.md)) |
| **kalshi-interserver VPS** | NixOS? | `systemd/` | Not found on Tailscale mesh (audit 2026-07-22) |
| **Hetzner (`hetzner-ex44`)** | ? | ? | Unverified from `lab` (SSH denied) |

## Installation (NixOS hosts)

```bash
# Copy the script to ~/.local/bin
cp scripts/bf-update.sh ~/.local/bin/bf-update.sh
chmod +x ~/.local/bin/bf-update.sh

# Copy systemd units to ~/.config/systemd/user
cp systemd/bf-update.{service,timer} ~/.config/systemd/user/

# Reload systemd and enable the timer
systemctl --user daemon-reload
systemctl --user enable bf-update.timer
systemctl --user start bf-update.timer

# Verify it's running
systemctl --user status bf-update.timer
```

## Usage

The timer runs hourly and automatically:
- Checks GitHub API for the latest bead-forge release
- Downloads `bf-linux-x86_64` and its `SHA256SUMS` manifest if newer than the installed version
- Verifies the binary's SHA256 against `SHA256SUMS` **before** installing — on any mismatch (or missing manifest) it fails loudly and leaves the existing `bf` in place
- Installs it to `~/.local/bin/bf`
- Saves version to `~/.local/bin/.bf-version` for comparison

### Manual Update

To trigger an update immediately:

```bash
~/.local/bin/bf-update.sh
# or
systemctl --user start bf-update.service
```

### Check Status

```bash
# View timer status
systemctl --user status bf-update.timer

# View last run logs
journalctl --user -u bf-update.service -n 20

# List next scheduled run
systemctl --user list-timers | grep bf-update
```

## bf-checkpoint — periodic `.beads/` git checkpoint (ADR-1)

The NixOS `bf-checkpoint` units are the out-of-band companion to `bf-update`. The timer runs `bf-checkpoint.sh` hourly, which flushes SQLite→JSONL (`bf sync --flush-only`) and, if `.beads/issues.jsonl` changed, stages **only** that file and commits it as `chore(beads): auto-checkpoint …`. It never touches `beads.db` and never runs on the `bf` claim/close hot path ([ADR-1](../docs/plan/plan.md#adr-1-periodic-beads-git-checkpoint-timer-2026-07-20)). The Debian/Ubuntu portable variant lives in [`../deploy/README.md`](../deploy/README.md).

**Inert by default:** the timer can be enabled with no side effect — `bf-checkpoint.sh` exits 0 unless `checkpoint.enabled: true` is set in `.beads/config.yaml`. **Push is off by default** too (set `checkpoint.push: true` to enable; never add `--push` to the unit).

### Installation (NixOS hosts)

```bash
# Install the NixOS script variant (#!/usr/bin/env bash)
cp ../scripts/bf-checkpoint.sh ~/.local/bin/bf-checkpoint.sh
chmod +x ~/.local/bin/bf-checkpoint.sh

# Install NixOS service + timer (hardcoded /run/current-system/sw/bin/bash)
cp bf-checkpoint.service ~/.config/systemd/user/
cp bf-checkpoint.timer ~/.config/systemd/user/

systemctl --user daemon-reload
systemctl --user enable bf-checkpoint.timer
systemctl --user start bf-checkpoint.timer

# Verify
systemctl --user status bf-checkpoint.timer
```

## How It Works

1. **CI Release**: The `bead-forge-build` Argo Workflow builds the binary and creates a GitHub release with the `bf-linux-x86_64` asset.
2. **Auto-Update**: The systemd timer runs `bf-update.sh` hourly, which queries GitHub API and installs updates automatically.
3. **Version Tracking**: The script stores the installed version in `~/.local/bin/.bf-version` to avoid unnecessary downloads.

## Files

- `~/.local/bin/bf-update.sh` - Update script
- `~/.config/systemd/user/bf-update.service` - Systemd service unit
- `~/.config/systemd/user/bf-update.timer` - Systemd timer unit
- `~/.local/bin/.bf-version` - Installed version tracker
- `~/.local/bin/bf-checkpoint.sh` - Out-of-band checkpoint script (from `scripts/`)
- `~/.config/systemd/user/bf-checkpoint.service` - Systemd checkpoint service unit
- `~/.config/systemd/user/bf-checkpoint.timer` - Systemd checkpoint timer unit
