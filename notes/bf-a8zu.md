# Auto-deploy bf binary to lab server (bf-a8zu)

## Implementation

Created systemd timer-based auto-deployment mechanism on the lab server using files tracked in the bead-forge repository.

### Components Created

1. **`deploy/bf-update.sh`** - Update script (tracked in repo) that:
   - Checks GitHub API for latest release using curl + grep/sed
   - Compares with installed version (stored in `~/.local/bin/.bf-version`)
   - Downloads and installs new `bf-linux-x86_64` binary if available
   - No external dependencies (no jq, no gh CLI, no file command required)
   - Uses temporary directory with automatic cleanup

2. **`deploy/bf-update.service`** - Systemd user service that runs the update script
   - ExecStart points to `/home/coding/bead-forge/deploy/bf-update.sh`
   - Tracked in repo and installed to `~/.config/systemd/user/`

3. **`deploy/bf-update.timer`** - Systemd timer that triggers the service:
   - Runs 5 minutes after boot
   - Then hourly thereafter
   - AccuracySec=1min for predictable scheduling
   - Enabled and active as of 2026-06-11

4. **`deploy/README.md`** - Complete documentation of the deployment setup

### Verification

```bash
# Timer is active and scheduled
systemctl --user status bf-update.timer
systemctl --user list-timers | grep bf-update

# Manual test succeeded
/home/coding/bead-forge/deploy/bf-update.sh  # Downloaded v0.1.0
/home/coding/bead-forge/deploy/bf-update.sh  # Detected already up-to-date

# Service logs show successful execution
journalctl --user -u bf-update.service -n 20
```

### Workflow Template Note Updated

Updated `bead-forge-build-workflowtemplate.yml` in declarative-config to reflect the auto-deployment mechanism is now active:
```
echo "Binary auto-deployed to lab server via bf-update systemd timer (hourly check)"
```

## How It Works

1. CI builds `bf` binary and creates GitHub release (existing)
2. Lab server's `bf-update.timer` fires hourly (enabled on lab server)
3. `/home/coding/bead-forge/deploy/bf-update.sh` queries GitHub API for latest release
4. If newer than installed version, downloads and installs to `~/.local/bin/bf`
5. Version stored in `~/.local/bin/.bf-version` for comparison

No manual intervention required after each release. The entire deployment infrastructure is tracked in the bead-forge repository under `deploy/`.
