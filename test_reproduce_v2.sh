#!/bin/bash
set -e

# Create a temp workspace
TEMP_DIR=$(mktemp -d)
echo "Testing in: $TEMP_DIR"

cd "$TEMP_DIR"
mkdir -p .beads

# Initialize with bf
bf init --prefix bf

# Create a bead
bf create --title "Test Bead"

# Flush to JSONL
bf sync --flush-only

echo "=== After flush ==="
bf doctor

# Run doctor --repair
bf doctor --repair

echo ""
echo "=== After repair ==="
bf doctor

echo ""
echo "=== Checking dirty_issues table directly ==="
sqlite3 .beads/beads.db "SELECT name FROM sqlite_master WHERE type='table' AND name='dirty_issues';" || true
sqlite3 .beads/beads.db "SELECT COUNT(*) FROM dirty_issues;" || true
sqlite3 .beads/beads.db "SELECT * FROM dirty_issues;" || true

# Clean up
cd -
rm -rf "$TEMP_DIR"
