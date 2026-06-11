# bead-forge Auto-Deployment

## Overview

The `bf` binary is automatically deployed to the lab server after each GitHub release via a systemd user timer.

## How It Works

1. **CI Release**: The `bead-forge-build` workflow builds the binary and creates a GitHub release with the `bf-linux-x86_64` asset.

2. **Auto-Update Script**: The lab server runs `~/.local/bin/bf-update.sh` which:
   - Queries GitHub API for the latest release
   - Downloads `bf-linux-x86_64` if newer than current
   - Installs to `~/.local/bin/bf`
   - Runs automatically hourly via systemd timer

3. **Manual Update**: To force an update immediately:
   ```bash
   ~/.local/bin/bf-update.sh
   # or
   systemctl --user start bf-update.service
   ```

## Files

- `~/.local/bin/bf-update.sh` - Update script (source in `scripts/bf-update.sh`)
- `~/.config/systemd/user/bf-update.service` - Systemd service unit (source in `systemd/bf-update.service`)
- `~/.config/systemd/user/bf-update.timer` - Systemd timer (hourly check, source in `systemd/bf-update.timer`)

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
