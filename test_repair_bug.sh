#!/bin/bash
set -e

# Test to reproduce the count_unflushed over-reporting bug after repair-import

# Store the original directory and the bf binary path
ORIG_DIR="$(pwd)"
BF_BIN="$ORIG_DIR/target/debug/bf"

echo "Creating test workspace..."
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"

# Initialize bead-forge workspace
mkdir -p .beads
cat > .beads/config.yaml <<EOF
cli: bf
database: beads.db
jsonl_export: issues.jsonl
EOF

# Create some test beads in JSONL
cat > .beads/issues.jsonl <<EOF
{"content_hash":"abc123","id":"bf-test1","title":"Test Bead 1","description":"","design":"","acceptance_criteria":"","notes":"","status":"open","priority":3,"issue_type":"task","assignee":null,"owner":null,"estimated_minutes":null,"created_at":"2024-01-01T00:00:00Z","created_by":null,"updated_at":"2024-01-01T00:00:00Z","closed_at":null,"close_reason":null,"closed_by_session":null,"due_at":null,"defer_until":null,"external_ref":null,"source_system":null,"source_repo":".","deleted_at":null,"deleted_by":null,"delete_reason":null,"original_type":null,"compaction_level":0,"compacted_at":null,"compacted_at_commit":null,"original_size":null,"sender":null,"ephemeral":false,"pinned":false,"is_template":false,"labels":[],"dependencies":[],"comments":[],"annotations":{}}
{"content_hash":"def456","id":"bf-test2","title":"Test Bead 2","description":"","design":"","acceptance_criteria":"","notes":"","status":"open","priority":3,"issue_type":"task","assignee":null,"owner":null,"estimated_minutes":null,"created_at":"2024-01-01T00:00:00Z","created_by":null,"updated_at":"2024-01-01T00:00:00Z","closed_at":null,"close_reason":null,"closed_by_session":null,"due_at":null,"defer_until":null,"external_ref":null,"source_system":null,"source_repo":".","deleted_at":null,"deleted_by":null,"delete_reason":null,"original_type":null,"compaction_level":0,"compacted_at":null,"compacted_at_commit":null,"original_size":null,"sender":null,"ephemeral":false,"pinned":false,"is_template":false,"labels":[],"dependencies":[],"comments":[],"annotations":{}}
EOF

echo "Running doctor repair (import from JSONL)..."
"$BF_BIN" doctor --repair --force 2>&1 | head -5

echo ""
echo "Checking database state..."
sqlite3 .beads/beads.db "SELECT COUNT(*) as total_beads FROM issues WHERE deleted_at IS NULL;"
sqlite3 .beads/beads.db "SELECT COUNT(*) as dirty_beads FROM dirty_issues;"

echo ""
echo "Checking export_hashes table..."
sqlite3 .beads/beads.db "SELECT COUNT(*) as export_hash_count FROM export_hashes;"

echo ""
echo "Checking for the bug..."
DIRTY_COUNT=$(sqlite3 .beads/beads.db "SELECT COUNT(*) FROM dirty_issues;")
TOTAL_COUNT=$(sqlite3 .beads/beads.db "SELECT COUNT(*) FROM issues WHERE deleted_at IS NULL;")
EXPORT_HASH_COUNT=$(sqlite3 .beads/beads.db "SELECT COUNT(*) FROM export_hashes;")

echo "Total beads: $TOTAL_COUNT"
echo "Dirty beads: $DIRTY_COUNT"
echo "Export hashes: $EXPORT_HASH_COUNT"

if [ "$DIRTY_COUNT" -eq "$TOTAL_COUNT" ] && [ "$TOTAL_COUNT" -gt 0 ]; then
    echo ""
    echo "BUG REPRODUCED: All beads are marked as dirty after repair-import!"
    echo "Expected: 0 dirty beads (they came from JSONL, so they're already flushed)"
    echo "Actual: $DIRTY_COUNT dirty beads"
    echo ""
    echo "Root cause analysis:"
    echo "1. repair() calls import_jsonl() with storage.create_issue() as callback"
    echo "2. create_issue() marks every bead as dirty (line 369-373 in sqlite.rs)"
    echo "3. After import, all beads are in dirty_issues table"
    echo "4. count_unflushed() counts all beads as unflushed"
    echo ""
    echo "Additionally: export_hashes table is empty ($EXPORT_HASH_COUNT entries)"
    echo "This means no beads are marked as exported to JSONL"
else
    echo "No bug found or already fixed."
fi

# Cleanup
cd -
rm -rf "$TEST_DIR"
