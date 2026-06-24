# Auto-Deployment Setup for bf Binary

This directory contains files for auto-deploying the `bf` binary to fleet hosts after each GitHub release.

## Host Variants

There are **two** systemd service variants for different OS environments:

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

### Current Fleet Deployment

| Host | OS | Variant | Timer Status |
|------|-----|---------|--------------|
| **Hetzner (this host)** | Debian | `deploy/` | ✅ Installed 2026-06-21 |
| **kalshi-interserver VPS** | NixOS | `systemd/` | ⚠️ Service only, timer needed |
| **lab** | Debian | `deploy/` | ⚠️ Not deployed |

## Components

- **`bf-update.sh`** - Update script that fetches the latest release from GitHub and installs it
- **`bf-update.service`** - Systemd user service that runs the update script
- **`bf-update.timer`** - Systemd timer that triggers the service hourly

## Installation (Debian/Portable hosts)

The systemd units are already installed in `~/.config/systemd/user/`. To reinstall:

```bash
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

## How it works

1. **Timer triggers hourly**: The `bf-update.timer` unit triggers `bf-update.service` every hour
2. **Update script runs**: The service executes `bf-update.sh` using the system bash
3. **Version check**: Script fetches latest version from GitHub API and compares to installed version
4. **Version detection priority**: 
   - First checks `~/.local/bin/.bf-version` file (most reliable)
   - Falls back to parsing `bf --version` output
   - Final fallback to parsing `bf --help` output
5. **Download if needed**: If a newer version exists, downloads and installs to `~/.local/bin/bf`
6. **Version tracking**: Saves installed version to `~/.local/bin/.bf-version` for next check

## Manual update

To trigger an update immediately (without waiting for the timer):

```bash
/home/coding/bead-forge/deploy/bf-update.sh
```

## Check status

```bash
# Check when timer last ran
systemctl --user list-timers bf-update.timer

# View service logs
journalctl --user -u bf-update.service -n 50
```

## CI/CD Integration

The `bead-forge-build` workflow template in `jedarden/declarative-config` creates GitHub releases. The systemd timer will pick up new releases within an hour of publication.
