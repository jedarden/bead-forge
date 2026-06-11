# Auto-deploy bf binary to lab server (bf-a8zu)

## Implementation

Created systemd timer-based auto-deployment mechanism on the lab server.

### Components Created

1. **`~/.local/bin/bf-update`** - Update script that:
   - Checks GitHub API for latest release
   - Compares with installed version (stored in `~/.config/bf-version`)
   - Downloads and installs new `bf-linux-x86_64` binary if available
   - Uses curl + jq for GitHub API queries
   - No `gh` CLI or `file` command required

2. **`~/.config/systemd/user/bf-update.service`** - Systemd service that runs the update script

3. **`~/.config/systemd/user/bf-update.timer`** - Systemd timer that triggers the service:
   - Runs 5 minutes after boot
   - Then hourly thereafter
   - Enabled and active as of 2026-06-11

### Verification

```bash
# Timer is active and scheduled
systemctl --user status bf-update.timer
systemctl --user list-timers | grep bf-update

# Manual test succeeded
~/.local/bin/bf-update  # Downloaded v0.1.0
~/.local/bin/bf-update  # Detected already up-to-date
```

### Workflow Template Note Updated

Updated `bead-forge-build-workflowtemplate.yml` in declarative-config to reflect the auto-deployment mechanism is now active.

## How It Works

1. CI builds `bf` binary and creates GitHub release (existing)
2. Lab server's `bf-update.timer` fires hourly
3. `bf-update` script queries GitHub API for latest release
4. If newer than installed version, downloads and installs to `~/.local/bin/bf`
5. Version stored in `~/.config/bf-version` for comparison

No manual intervention required after each release.
