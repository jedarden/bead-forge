# Auto-Deployment of bf Binary

## Overview

The `bf` binary is automatically deployed to the lab server (`~/.local/bin/bf`) after each successful CI release via a **pull-based** mechanism using systemd timers.

## Architecture

```
GitHub Release → Systemd Timer (hourly) → Update Script → ~/.local/bin/bf
```

### Components

1. **CI/CD Workflow** (`bead-forge-build-workflowtemplate.yml`)
   - Builds `bf` binary from source
   - Creates GitHub release with `bf-linux-x86_64` asset
   - Emits note: "Binary will be auto-deployed to lab server via bf-update systemd timer (hourly check)"

2. **Systemd Timer** (`~/.config/systemd/user/bf-update.timer`)
   - Runs 5 minutes after boot
   - Runs every hour thereafter (`OnUnitActiveSec=1h`)
   - Triggers `bf-update.service`

3. **Systemd Service** (`~/.config/systemd/user/bf-update.service`)
   - Executes `/home/coding/bead-forge/deploy/bf-update.sh`
   - Runs as user service (no root required)

4. **Update Script** (`deploy/bf-update.sh`)
   - Fetches latest release tag from GitHub API
   - Compares with installed version (stored in `~/.local/bin/.bf-version`)
   - Downloads `bf-linux-x86_64` asset if newer
   - Installs to `~/.local/bin/bf`
   - Updates version file

## Verification

### Check timer status:
```bash
systemctl --user status bf-update.timer
```

Expected output:
```
● bf-update.timer - Hourly check for bf binary updates
     Loaded: loaded
     Active: active (waiting)
```

### View recent update runs:
```bash
journalctl --user -u bf-update.service -n 10 --no-pager
```

### Manually trigger update:
```bash
systemctl --user start bf-update.service
```

### Check installed version:
```bash
cat ~/.local/bin/.bf-version
```

## Current Status

✅ **FULLY OPERATIONAL** as of 2026-06-11

- Timer: Active (runs hourly)
- Script: Syntactically valid, functional
- Last run: Successfully checked for v0.1.0
- Install location: `~/.local/bin/bf`

## Deployment Timeline

After a new release is created by CI:
1. **0-60 minutes**: Systemd timer picks up new release (on next hourly check)
2. **~1 minute**: Update script downloads binary from GitHub
3. **Immediate**: Binary installed to `~/.local/bin/bf`

## Advantages of Pull-Based Approach

1. **No SSH required**: CI runner doesn't need lab server credentials
2. **Self-healing**: Recover from failed deploys on next hourly check
3. **Idempotent**: Safe to run multiple times
4. **Low complexity**: Simple script, no complex orchestration
5. **Always up-to-date**: Hourly checks ensure latest version

## Alternative Approaches Considered

| Approach | Status | Reason |
|----------|--------|--------|
| Push from CI workflow | ❌ Rejected | Requires SSH access from CI runner to lab server |
| GitHub Actions post-release | ❌ Rejected | Banned per CLAUDE.md |
| Sidecar deployment pod | ❌ Rejected | Overkill for single binary |
| Shared storage + cron | ❌ Rejected | More complex than direct download |

## Troubleshooting

### Timer not running:
```bash
systemctl --user enable bf-update.timer
systemctl --user start bf-update.timer
```

### Manual update test:
```bash
bash deploy/bf-update.sh
```

### Check download URL manually:
```bash
VERSION="0.1.0"
curl -I "https://github.com/jedarden/bead-forge/releases/download/v${VERSION}/bf-linux-x86_64"
```

### Verify binary after install:
```bash
file ~/.local/bin/bf
# Should output: ELF 64-bit LSB executable, x86-64, ...
```

## Implementation Notes

- The workflow template was updated on 2026-06-11 to document the auto-deployment mechanism
- All infrastructure (timer, service, script) was already in place and operational
- No code changes were required—only verification and documentation
- Deployment latency is <60 minutes (acceptable for tooling updates)
