# Auto-Deployment Setup for bf Binary

This directory contains files for auto-deploying the `bf` binary to fleet hosts after each GitHub release.

It also contains the **`bf-checkpoint`** units — a systemd timer that periodically flushes `.beads/` to JSONL and commits `issues.jsonl` out-of-band ([ADR-1](../docs/plan/plan.md#adr-1-periodic-beads-git-checkpoint-timer-2026-07-20)). These ship alongside the `bf-update` units and are installed the same way.

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
- **`bf-checkpoint.sh`** - Out-of-band script: flushes SQLite→JSONL and commits `issues.jsonl` (ADR-1)
- **`bf-checkpoint.service`** - Systemd user service that runs the checkpoint script
- **`bf-checkpoint.timer`** - Systemd timer that triggers the service hourly

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

## bf-checkpoint — periodic `.beads/` git checkpoint (ADR-1)

The `bf-checkpoint` units are the out-of-band companion to `bf-update`. Where `bf-update.timer` keeps the **binary** current, `bf-checkpoint.timer` keeps the **workspace** current: every hour it runs `bf-checkpoint.sh`, which flushes SQLite→JSONL (`bf sync --flush-only`) and, if `.beads/issues.jsonl` changed, stages **only** that file and commits it as `chore(beads): auto-checkpoint …`. It never touches `beads.db` and never runs on the `bf` claim/close hot path (ADR-1).

### Inert by default

The timer can be enabled and active with **no side effects**. `bf-checkpoint.sh` exits 0 immediately unless `checkpoint.enabled: true` is set in `.beads/config.yaml` for the target workspace — so a freshly deployed timer stays dormant until a maintainer opts in:

```yaml
# .beads/config.yaml
checkpoint:
  enabled: true            # master switch; default false
  interval_minutes: 60     # min gap between commits; default 60
  push: false              # git push after each commit; default false
```

**Push is off by default** too. Commits stay local unless `checkpoint.push: true` (persistent) or the `--push` one-shot flag is passed to the script. Do **not** add `--push` to the unit's `ExecStart` — that would force-push on every timer fire.

### Installation (Debian/Portable hosts)

```bash
# Install the script (Debian/Ubuntu variant — #!/bin/bash)
cp bf-checkpoint.sh ~/.local/bin/bf-checkpoint.sh
chmod +x ~/.local/bin/bf-checkpoint.sh

# Install service and timer
cp bf-checkpoint.service ~/.config/systemd/user/
cp bf-checkpoint.timer ~/.config/systemd/user/

# Reload systemd and enable timer
systemctl --user daemon-reload
systemctl --user enable bf-checkpoint.timer
systemctl --user start bf-checkpoint.timer

# Verify
systemctl --user status bf-checkpoint.timer
```

On NixOS, install `scripts/bf-checkpoint.sh` instead and copy the `systemd/bf-checkpoint.{service,timer}` variant (hardcoded `/run/current-system/sw/bin/bash`) — see [`../systemd/README.md`](../systemd/README.md). The target workspace is pinned to `~/.local/bin`'s owning repo via `BF_WORKSPACE=%h/bead-forge` in the unit.

### Check status

```bash
systemctl --user list-timers bf-checkpoint.timer
journalctl --user -u bf-checkpoint.service -n 50
```

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
