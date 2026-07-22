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
| **lab (this host)** | NixOS | `systemd/` | ✅ Deployed & active — but **service fails every run: repo has 0 published Releases** (audit 2026-07-22, see [`notes/bf-u4fxh.md`](../notes/bf-u4fxh.md)) |
| **Hetzner (`hetzner-ex44`)** | ? | ? | Unverified from `lab` (SSH denied) |
| **kalshi-interserver VPS** | NixOS? | `systemd/` | Not found on Tailscale mesh (audit 2026-07-22) |

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
5. **Download if needed**: If a newer version exists, downloads `bf-linux-x86_64` and its `SHA256SUMS` manifest from the release
6. **Checksum verification**: Verifies the binary's SHA256 against `SHA256SUMS` **before** installing — on any mismatch (or missing manifest) it fails loudly and leaves the existing `bf` in place
7. **Version tracking**: Saves installed version to `~/.local/bin/.bf-version` for next check

## GitHub API Authentication (optional)

By default the update script queries the GitHub API **unauthenticated**, which is rate-limited to **60 requests/hour per source IP**. That budget is shared across every host sharing this server's egress IP plus any other unauthenticated GitHub API traffic from the same host. With one active host the hourly timer is comfortably under budget, but as the fleet grows (and since this script is a copyable template for other repos' auto-update timers) you may want to authenticate, which raises the limit to **5000 requests/hour per token**.

The script automatically uses a token when it can find one, with this precedence:

1. **`$GITHUB_TOKEN`** environment variable.
2. **Token file** at `$BF_GITHUB_TOKEN_FILE` (default `~/.config/bf-update/github-token`) — a plain-text file containing only the token.

If neither is present, the script falls back to unauthenticated requests, so it keeps working with **zero setup**. The token is sent only as a `curl` `Authorization: Bearer` header to `api.github.com` for the release-metadata lookups.

### Provide a token via file

```bash
mkdir -p ~/.config/bf-update
chmod 700 ~/.config/bf-update
# Write a GitHub personal access token (classic or fine-grained — this is a
# public repo, so read-only access needs no special scopes) into the file:
$EDITOR ~/.config/bf-update/github-token
chmod 600 ~/.config/bf-update/github-token
```

A custom path can be set with the `$BF_GITHUB_TOKEN_FILE` env var.

### Provide a token via the systemd unit

Add an `Environment=` line to `bf-update.service`:

```ini
Environment=GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxx
```

Then reload and restart:

```bash
systemctl --user daemon-reload
systemctl --user restart bf-update.timer
```

> Protect the token file with restrictive permissions (`chmod 600`) and never commit it to the repo.

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
