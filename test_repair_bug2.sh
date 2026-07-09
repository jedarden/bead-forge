#!/bin/bash
set -e

# Create temp workspace
TEMP_DIR=$(mktemp -d)
echo "Using temp dir: $TEMP_DIR"

cd "$TEMP_DIR"
mkdir -p .beads

# Initialize a beads workspace
/home/coding/bead-forge/target/release/bf --workspace . init 2>&1 || echo "Init may have failed, continuing..."

# Create a test bead via direct SQL
sqlite3 .beads/beads.db <<EOSQL
INSERT INTO issues (id, content_hash, title, status, priority, issue_type, created_at, updated_at, created_by)
VALUES ('bf-test', 'abc123', 'Test Bead', 'open', 1, 'task', datetime('now'), datetime('now'), 'test');
EOSQL

# Flush to JSONL
/home/coding/bead-forge/target/release/bf --workspace . sync --flush-only

echo "=== Before repair ==="
sqlite3 .beads/beads.db "SELECT COUNT(*) as dirty_count FROM dirty_issues;"

# Run repair
/home/coding/bead-forge/target/release/bf --workspace . doctor --repair --force

echo "=== After repair ==="
sqlite3 .beads/beads.db "SELECT COUNT(*) as dirty_count FROM dirty_issues;"

echo "=== Now run import-only ==="
/home/coding/bead-forge/target/release/bf --workspace . sync --import-only

echo "=== After import-only ==="
sqlite3 .beads/beads.db "SELECT COUNT(*) as dirty_count FROM dirty_issues;"

echo "=== Run bf doctor ==="
/home/coding/bead-forge/target/release/bf --workspace . doctor

# Cleanup
cd /home/coding/bead-forge
rm -rf "$TEMP_DIR"
