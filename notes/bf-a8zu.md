# Auto-Deployment Implementation for bf Binary

## Status: Complete ✓

## Implementation Summary

The auto-deployment system for the `bf` binary to the lab server is fully operational.

## Architecture

**Passive Polling Model** (chosen due to infrastructure constraints):
- CI runs in iad-ci cluster (isolated from lab server)
- Lab server polls GitHub for new releases via curl
- No shared storage or direct connectivity between CI and lab
- No gh CLI dependency on lab server

## Components

### 1. Update Script (`~/bead-forge/deploy/bf-update.sh`)
- Queries GitHub API for latest release version
- Downloads `bf-linux-x86_64` asset via `curl`
- Installs to `~/.local/bin/bf`
- Tracks installed version in `~/.local/bin/.bf-version`
- Idempotent (skips if already up-to-date)

### 2. Systemd Timer (`~/.config/systemd/user/bf-update.timer`)
- **Frequency**: Hourly (OnUnitActiveSec=1h)
- **Boot delay**: 5 minutes after boot (OnBootSec=5min)
- **Status**: Active and enabled

```ini
[Timer]
OnBootSec=5min
OnUnitActiveSec=1h
AccuracySec=1s
```

### 3. Update Service (`~/.config/systemd/user/bf-update.service`)
- Runs `~/bead-forge/deploy/bf-update.sh`
- Triggered by timer
- Logs to systemd journal

### 4. CI Integration (`bead-forge-build-workflowtemplate.yml`)
- Line 61 documents the auto-deployment mechanism
- CI creates GitHub release with `bf-linux-x86_64` asset
- Lab server picks up within 1 hour (max)

## Deployment Timeline

**Maximum deployment latency**: 1 hour 5 minutes
- Timer runs hourly
- 5-minute initial boot delay

## Verification

```bash
# Check timer status
systemctl --user status bf-update.timer

# Manual trigger
systemctl --user start bf-update.service

# View logs
journalctl --user -u bf-update.service -n 50

# Check installed version
cat ~/.local/bin/.bf-version
```

## Why This Approach?

Given infrastructure constraints:
- ❌ No SSH access from CI to lab (banned per CLAUDE.md)
- ❌ No shared NFS/S3 storage between iad-ci and lab server
- ❌ Lab server not on same cluster as CI
- ✓ Lab server has curl and GitHub API access
- ✓ Systemd timers are reliable and persistent
- ✓ No external dependencies (gh CLI not required)

## Implementation Date

2026-06-11

## Files Modified

- `deploy/bf-update.sh` - Update script (uses curl + GitHub API)
- `deploy/bf-update.service` - Systemd service unit
- `deploy/bf-update.timer` - Systemd timer unit
- `deploy/README.md` - Documentation
- `~/.config/systemd/user/bf-update.service` - Installed on lab server
- `~/.config/systemd/user/bf-update.timer` - Installed on lab server

## Test Results

✓ Timer active and running hourly
✓ Script successfully fetches version info from GitHub API
✓ Version comparison working (v0.1.0 detected as current)
✓ Binary installed at ~/.local/bin/bf
✓ Service logs clean
