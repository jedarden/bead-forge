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

# Check dirty_issues table before repair
echo "=== dirty_issues table before repair ==="
sqlite3 .beads/beads.db "SELECT COUNT(*) FROM dirty_issues;" 2>/dev/null || echo "No dirty_issues table"

# Run doctor --repair
bf doctor --repair

echo "=== After repair ==="
bf doctor

# Check dirty_issues table after repair
echo "=== dirty_issues table after repair ==="
sqlite3 .beads/beads.db "SELECT * FROM dirty_issues;" 2>/dev/null || echo "No dirty_issues table"

# Check if table exists
sqlite3 .beads/beads.db "SELECT name FROM sqlite_master WHERE type='table' AND name='dirty_issues';" 2>/dev/null || echo "Query failed"

# Clean up
cd -
rm -rf "$TEMP_DIR"
