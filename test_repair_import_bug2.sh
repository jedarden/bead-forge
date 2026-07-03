#!/bin/bash
set -e

# Create temp workspace
TEMP_DIR=$(mktemp -d)
echo "Using temp dir: $TEMP_DIR"

cd "$TEMP_DIR"
mkdir -p .beads

# Initialize a beads workspace
/home/coding/bead-forge/target/release/bf --workspace . init 2>&1 || true

# Create a test bead via direct SQL
sqlite3 .beads/beads.db <<EOSQL
INSERT INTO issues (id, content_hash, title, status, priority, issue_type, created_at, updated_at, created_by)
VALUES ('bf-test', 'abc123', 'Test Bead', 'open', 1, 'task', datetime('now'), datetime('now'), 'test');
EOSQL

echo "=== After creating bead (no flush yet) ==="
sqlite3 .beads/beads.db "SELECT issue_id FROM dirty_issues;"
sqlite3 .beads/beads.db "SELECT issue_id FROM export_hashes;"

# Flush to JSONL (this should populate export_hashes)
/home/coding/bead-forge/target/release/bf --workspace . sync --flush-only

echo "=== After flush (export_hashes should be populated, dirty should be cleared) ==="
sqlite3 .beads/beads.db "SELECT issue_id FROM dirty_issues;"
sqlite3 .beads/beads.db "SELECT issue_id FROM export_hashes;"

# Now run doctor --repair (deletes DB, imports from JSONL)
/home/coding/bead-forge/target/release/bf --workspace . doctor --repair --force

echo "=== After repair (export_hashes should STILL be empty if not populated by import) ==="
sqlite3 .beads/beads.db "SELECT issue_id FROM dirty_issues;"
sqlite3 .beads/beads.db "SELECT issue_id FROM export_hashes;"

echo "=== Run doctor to check unflushed count ==="
/home/coding/bead-forge/target/release/bf --workspace . doctor

# Cleanup
cd /home/coding/bead-forge
rm -rf "$TEMP_DIR"
