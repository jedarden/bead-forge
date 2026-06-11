#!/usr/bin/env bash
# Auto-update script for bf binary
# Fetches the latest release from GitHub and installs to ~/.local/bin/bf

set -euo pipefail

INSTALL_DIR="$HOME/.local/bin"
BINARY_NAME="bf"
VERSION_FILE="$INSTALL_DIR/.bf-version"
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

echo "Checking for latest bf release..."

# Fetch latest version info from GitHub API
LATEST_RELEASE_URL="https://api.github.com/repos/jedarden/bead-forge/releases/latest"
VERSION_INFO=$(curl -fsSL "$LATEST_RELEASE_URL")
LATEST_VERSION=$(echo "$VERSION_INFO" | grep -m1 '"tag_name"' | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/' | sed 's/^v//')

if [[ -z "$LATEST_VERSION" ]]; then
    echo "Failed to fetch latest version"
    exit 1
fi

echo "Latest version: v${LATEST_VERSION}"

# Check current installed version (if any)
if [[ -f "$VERSION_FILE" ]]; then
    CURRENT_VERSION=$(cat "$VERSION_FILE")
    echo "Current version: v${CURRENT_VERSION}"

    if [[ "$CURRENT_VERSION" == "$LATEST_VERSION" ]]; then
        echo "Already up to date"
        exit 0
    fi
else
    echo "No version file found - will install latest"
fi

# Download the binary
DOWNLOAD_URL="https://github.com/jedarden/bead-forge/releases/download/v${LATEST_VERSION}/bf-linux-x86_64"
echo "Downloading from ${DOWNLOAD_URL}..."
curl -fsSL "$DOWNLOAD_URL" -o "$TEMP_DIR/$BINARY_NAME"

# Verify it's a valid binary (optional check)
if command -v file >/dev/null 2>&1; then
    if ! file "$TEMP_DIR/$BINARY_NAME" | grep -qE "(ELF|executable)"; then
        echo "Downloaded file is not a valid binary"
        exit 1
    fi
else
    # Basic check: ensure file is not empty and has execute permission will be set
    if [[ ! -s "$TEMP_DIR/$BINARY_NAME" ]]; then
        echo "Downloaded file is empty"
        exit 1
    fi
fi

# Install
chmod +x "$TEMP_DIR/$BINARY_NAME"
mkdir -p "$INSTALL_DIR"
mv "$TEMP_DIR/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
echo "$LATEST_VERSION" > "$VERSION_FILE"

echo "Successfully installed bf v${LATEST_VERSION} to $INSTALL_DIR/$BINARY_NAME"
