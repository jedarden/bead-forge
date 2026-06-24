# bf-irdm: systemd unit path divergence + fleet rollout documentation

## Completed: 2026-06-24

### Problem
Two variants of `bf-update.service` existed with different bash path handling:
- `systemd/bf-update.service` — hardcoded NixOS path (`/run/current-system/sw/bin/bash`)
- `deploy/bf-update.service` — portable for Debian (relies on `PATH=/usr/bin:/bin`)

The deploy/ variant was installed on Hetzner Debian host on 2026-06-21, but there was no documentation explaining which variant to use on which host type, or how to deploy to remaining fleet hosts.

### Solution Implemented

#### 1. Documentation Created
- **`deploy/README.md`** — Comprehensive documentation including:
  - Host variants table explaining the two service types
  - Host detection bash snippet for automatic OS detection
  - Current fleet deployment status table
  - Installation instructions per variant
  - CI/CD integration notes

- **`deploy/ROLLOUT.md`** — Fleet rollout guide with:
  - Quick host detection script
  - Per-host deployment instructions (Hetzner ✅, kalshi-interserver ⚠️, lab ⚠️)
  - Verification checklist
  - Troubleshooting guide for OS-specific issues
  - Rollout status tracking table

#### 2. Host Detection Logic
```bash
# Check if NixOS
if [ -d /nix/var/nix/profiles/system ] || [ -d /run/current-system ]; then
    echo "NixOS host → use systemd/ variant"
else
    echo "Debian/Portable host → use deploy/ variant"
fi
```

#### 3. Fleet Status
| Host | OS | Variant | Service | Timer | Status |
|------|-----|---------|---------|-------|--------|
| Hetzner | Debian | deploy/ | ✅ | ✅ | Complete (2026-06-21) |
| kalshi-interserver | NixOS | systemd/ | ⚠️ | ❌ | Pending timer deployment |
| lab | Debian | deploy/ | ❌ | ❌ | Pending full deployment |

### Remaining Actions (Manual)
The documentation is complete. Actual deployment to remaining hosts requires manual SSH access:
1. **kalshi-interserver VPS** — Deploy `systemd/` variant with timer
2. **lab** — Full deployment of `deploy/` variant (service + timer)

See `deploy/ROLLOUT.md` for step-by-step deployment commands for each host.

### Files Modified
- `deploy/README.md` — Added host variants section, host detection, fleet status
- `deploy/ROLLOUT.md` — Created comprehensive fleet rollout guide
