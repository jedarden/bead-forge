#!/bin/bash
# Auto-update script for bf (bead-forge) binary
# Fetches the latest release from GitHub and installs to ~/.local/bin/bf
#
# Install this script to ~/.local/bin/bf-update.sh and make it executable:
#   cp bf-update.sh ~/.local/bin/bf-update.sh
#   chmod +x ~/.local/bin/bf-update.sh
#
# This script is called by the bf-update systemd timer (see bf-update.timer)
#
# Rollback: Restores the previous bf binary from .bf-version.previous and bf.previous
#   ~/.local/bin/bf-update.sh --rollback

set -euo pipefail

# Handle --rollback flag
if [[ "${1:-}" == "--rollback" ]]; then
    BIN_DIR="$HOME/.local/bin"

    if [[ ! -f "$BIN_DIR/bf.previous" ]]; then
        echo "ERROR: No backup binary found at $BIN_DIR/bf.previous"
        echo "Rollback is only available after at least one successful update."
        exit 1
    fi

    echo "Rolling back to previous version..."

    # Restore the binary
    cp "$BIN_DIR/bf.previous" "$BIN_DIR/bf"
    chmod +x "$BIN_DIR/bf"

    # Restore the version file if it exists
    if [[ -f "$BIN_DIR/.bf-version.previous" ]]; then
        cp "$BIN_DIR/.bf-version.previous" "$BIN_DIR/.bf-version"
    fi

    # Show the rolled-back version
    ROLLED_BACK_VERSION=$("$BIN_DIR/bf" --version 2>&1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "unknown")
    echo "Rollback complete (now at $ROLLED_BACK_VERSION)"
    exit 0
fi

BIN_DIR="$HOME/.local/bin"
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

echo "Checking for new bead-forge releases..."

# Get the latest release tag from GitHub API
LATEST_RELEASE=$(curl -s https://api.github.com/repos/jedarden/bead-forge/releases/latest | jq -r .tag_name)
CURRENT_VERSION="unknown"

if [[ -f "$BIN_DIR/bf" ]]; then
    # Try to get current version from .bf-version file first
    if [[ -f "$BIN_DIR/.bf-version" ]]; then
        CURRENT_VERSION=$(cat "$BIN_DIR/.bf-version" 2>/dev/null || echo "unknown")
    else
        # Extract version from --version output (exits with code 1, so we capture stderr too)
        VERSION_OUTPUT=$("$BIN_DIR/bf" --version 2>&1 || true)
        # Extract version number from output like "bf 0.1.0" or "Error: bf 0.1.0"
        if [[ "$VERSION_OUTPUT" =~ ([0-9]+\.[0-9]+\.[0-9]+) ]]; then
            CURRENT_VERSION="v${BASH_REMATCH[1]}"
        else
            # Fallback to parsing --help
            CURRENT_VERSION=$("$BIN_DIR/bf" --help 2>/dev/null | grep -oE 'bead-forge [0-9]+\.[0-9]+\.[0-9]+' | head -1 | awk '{print $2}' || echo "unknown")
            if [[ "$CURRENT_VERSION" != "unknown" ]]; then
                CURRENT_VERSION="v$CURRENT_VERSION"
            fi
        fi
    fi
fi

echo "Latest release: $LATEST_RELEASE"
echo "Current version: $CURRENT_VERSION"

# If versions match (and current isn't unknown), skip
if [[ "$CURRENT_VERSION" == "$LATEST_RELEASE" || "$CURRENT_VERSION" == "v$LATEST_RELEASE" ]]; then
    echo "Already up to date, exiting"
    exit 0
fi

echo "Downloading bf-linux-x86_64 from release $LATEST_RELEASE..."

# Fetch the release manifest once and extract both asset URLs (binary + checksums)
RELEASE_JSON=$(curl -s "https://api.github.com/repos/jedarden/bead-forge/releases/latest")
DOWNLOAD_URL=$(echo "$RELEASE_JSON" | jq -r '.assets[] | select(.name == "bf-linux-x86_64") | .browser_download_url')
SUMS_URL=$(echo "$RELEASE_JSON" | jq -r '.assets[] | select(.name == "SHA256SUMS") | .browser_download_url')

if [[ -z "$DOWNLOAD_URL" ]]; then
    echo "ERROR: Could not find bf-linux-x86_64 asset in release $LATEST_RELEASE"
    exit 1
fi
if [[ -z "$SUMS_URL" ]]; then
    echo "ERROR: Could not find SHA256SUMS asset in release $LATEST_RELEASE — refusing to install without checksum verification"
    exit 1
fi

# Download the binary and its checksum manifest
curl -fsSL "$DOWNLOAD_URL" -o "$TEMP_DIR/bf-linux-x86_64"
curl -fsSL "$SUMS_URL" -o "$TEMP_DIR/SHA256SUMS"

# Verify downloads landed
if [[ ! -f "$TEMP_DIR/bf-linux-x86_64" ]]; then
    echo "ERROR: Binary download failed"
    exit 1
fi
if [[ ! -f "$TEMP_DIR/SHA256SUMS" ]]; then
    echo "ERROR: SHA256SUMS download failed — refusing to install without checksum verification"
    exit 1
fi

# Verify the SHA256 checksum BEFORE installing. On any mismatch (or a
# missing/malformed manifest) leave the existing bf binary untouched.
#
# Match the entry for our specific asset rather than blindly trusting the
# first line: a manifest that lists a different (attacker-chosen) file first
# must not be accepted. Handles both text (`<hash>  name`) and binary
# (`<hash> *name`) manifest formats.
EXPECTED=$(awk '$2 == "bf-linux-x86_64" || $2 == "*bf-linux-x86_64" {print $1; exit}' "$TEMP_DIR/SHA256SUMS")
if [[ -z "$EXPECTED" ]]; then
    # Fallback: legacy manifest with a single line containing only the hash
    EXPECTED=$(awk 'NF==1 {print $1; exit}' "$TEMP_DIR/SHA256SUMS")
fi
if [[ -z "$EXPECTED" ]]; then
    echo "ERROR: SHA256SUMS has no entry for bf-linux-x86_64 (empty or malformed) — refusing to install"
    exit 1
fi
ACTUAL=$(sha256sum "$TEMP_DIR/bf-linux-x86_64" | awk '{print $1}')

echo "Expected SHA256: $EXPECTED"
echo "Actual SHA256:   $ACTUAL"

if [[ "$EXPECTED" != "$ACTUAL" ]]; then
    echo "ERROR: SHA256 checksum mismatch — refusing to install; keeping existing bf binary"
    exit 1
fi
echo "Checksum verified OK"

# Make executable
chmod +x "$TEMP_DIR/bf-linux-x86_64"

# Verify it's a valid binary (check it's executable and not empty)
if [[ ! -x "$TEMP_DIR/bf-linux-x86_64" ]] || [[ ! -s "$TEMP_DIR/bf-linux-x86_64" ]]; then
    echo "ERROR: Downloaded file is not valid or empty"
    exit 1
fi

# Backup the existing binary and version file (if they exist)
if [[ -f "$BIN_DIR/bf" ]]; then
    cp "$BIN_DIR/bf" "$BIN_DIR/bf.previous"
    echo "Backed up existing bf binary to $BIN_DIR/bf.previous"
fi

if [[ -f "$BIN_DIR/.bf-version" ]]; then
    cp "$BIN_DIR/.bf-version" "$BIN_DIR/.bf-version.previous"
fi

# Install
mv "$TEMP_DIR/bf-linux-x86_64" "$BIN_DIR/bf"
echo "Installed new bf binary to $BIN_DIR/bf"

# Save version for future checks
echo "$LATEST_RELEASE" > "$BIN_DIR/.bf-version"

# Show version
"$BIN_DIR/bf" --version 2>&1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "$LATEST_RELEASE"
echo "Update complete ($LATEST_RELEASE)"
