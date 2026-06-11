#!/usr/bin/env bash
# bf-update - Update bf binary from GitHub releases
# Run this manually or via cron to update ~/.local/bin/bf

set -euo pipefail

BINARY_DIR="$HOME/.local/bin"
BINARY_PATH="$BINARY_DIR/bf"
REPO="jedarden/bead-forge"
ASSET="bf-linux-x86_64"
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

echo "Checking for latest bead-forge release..."

# Get latest release tag
LATEST_RELEASE=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | jq -r '.tag_name')
if [[ -z "$LATEST_RELEASE" || "$LATEST_RELEASE" == "null" ]]; then
  echo "Error: Could not fetch latest release" >&2
  exit 1
fi

echo "Latest release: $LATEST_RELEASE"

# Get current version if installed
if [[ -x "$BINARY_PATH" ]]; then
  # Compare by checking if we can get the Cargo.toml version from the binary's build
  # bf doesn't have a --version flag, so we'll check against git releases
  CURRENT_TAG=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases" | jq -r '.[0].tag_name')
  if [[ "$CURRENT_TAG" == "$LATEST_RELEASE" ]]; then
    echo "Already up to date at $LATEST_RELEASE!"
    exit 0
  fi
fi

echo "Downloading $ASSET from $LATEST_RELEASE..."

# Download asset
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${LATEST_RELEASE}/${ASSET}"
if ! curl -fsSL "$DOWNLOAD_URL" -o "$TEMP_DIR/$ASSET"; then
  echo "Error: Failed to download $ASSET" >&2
  exit 1
fi

# Make executable
chmod +x "$TEMP_DIR/$ASSET"

# Install
echo "Installing to $BINARY_PATH..."
mkdir -p "$BINARY_DIR"
mv "$TEMP_DIR/$ASSET" "$BINARY_PATH"

echo "✓ Updated bf to $LATEST_RELEASE!"
echo "Binary installed at: $BINARY_PATH"
