# bead-forge Auto-Deployment

## Overview

The `bf` binary is automatically deployed to the lab server after each GitHub release via a systemd user timer.

## How It Works

1. **CI Release**: The `bead-forge-build` workflow builds the binary and creates a GitHub release with the `bf-linux-x86_64` asset.

2. **Auto-Update Script**: The lab server runs `~/.local/bin/bf-update` which:
   - Queries GitHub API for the latest release
   - Downloads `bf-linux-x86_64` if newer than current
   - Installs to `~/.local/bin/bf`
   - Runs automatically daily via systemd timer

3. **Manual Update**: To force an update immediately:
   ```bash
   bf-update
   ```

## Files

- `~/.local/bin/bf-update` - Update script
- `~/.config/systemd/user/bf-update.service` - Systemd service unit
- `~/.config/systemd/user/bf-update.timer` - Systemd timer (daily check)

## Checking Timer Status

```bash
# Check if timer is active
systemctl --user status bf-update.timer

# View last run logs
journalctl --user -u bf-update.service -n 20

# Manually trigger update
systemctl --user start bf-update.service
```

## Source

The update script lives in `scripts/bf-update.sh` in the bead-forge repository.
