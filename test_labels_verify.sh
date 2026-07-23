#!/bin/bash
# Test script to verify label export/import round-trip for bead-forge
# Tests that labels export correctly and can be imported back

set -e

echo "=== Testing Label Export/Import Round-trip ==="

# Use current directory (bead-forge project)
WORKSPACE=/home/coding/bead-forge
cd "$WORKSPACE"

# Build bead-forge first
echo "1. Building bead-forge..."
cargo build --quiet --bin bf 2>&1 | grep -v "warning:" || true

# Create a test workspace directory
TEST_WS=$(mktemp -d)
echo "2. Creating test workspace: $TEST_WS"
cd "$TEST_WS"

# Initialize bead-forge database using the built binary
echo "3. Initializing bead-forge database..."
$WORKSPACE/target/debug/bf init

# Create test beads with various label configurations
echo "4. Creating test beads with labels..."

# Bead with multiple labels
BF1=$($WORKSPACE/target/debug/bf create --title "Multi-label bead" \
    --label "phase-1" \
    --label "storage" \
    --label "critical" \
    --label "test-label" --no-auto-flush | grep -oE 'bf-[a-z0-9]+' | head -1)

echo "  Created $BF1 with 4 labels"

# Bead with single label
BF2=$($WORKSPACE/target/debug/bf create --title "Single-label bead" \
    --label "phase-2" --no-auto-flush | grep -oE 'bf-[a-z0-9]+' | head -1)

echo "  Created $BF2 with 1 label"

# Bead with no labels
BF3=$($WORKSPACE/target/debug/bf create --title "No-label bead" --no-auto-flush | grep -oE 'bf-[a-z0-9]+' | head -1)

echo "  Created $BF3 with 0 labels"

# Verify labels in database
echo "5. Verifying labels in bead_labels table (after creation)..."
sqlite3 .beads/beads.db "SELECT bead_id, label FROM bead_labels ORDER BY bead_id, label;"

# Export to JSONL
echo "6. Exporting to JSONL..."
$WORKSPACE/target/debug/bf sync --flush-only

# Check JSONL content
echo "7. Checking JSONL content for labels..."
echo "  Bead 1 labels:"
grep "\"id\":\"$BF1\"" .beads/issues.jsonl | grep -o '"labels":\[[^]]*\]' || echo "    (no labels field - empty)"
echo "  Bead 2 labels:"
grep "\"id\":\"$BF2\"" .beads/issues.jsonl | grep -o '"labels":\[[^]]*\]'
echo "  Bead 3 labels:"
grep "\"id\":\"$BF3\"" .beads/issues.jsonl | grep -o '"labels":\[[^]]*\]' || echo "    (no labels field - empty)"

# Backup the original JSONL
cp .beads/issues.jsonl issues.jsonl.backup

# Save label counts before destruction
BEFORE_COUNT=$(sqlite3 .beads/beads.db "SELECT COUNT(*) FROM bead_labels;")
echo "8. Total labels before: $BEFORE_COUNT"

# Test 1: Delete all issues and re-import (clean import scenario)
echo "9. Testing clean import (deleting all issues)..."
sqlite3 .beads/beads.db "DELETE FROM issues;"  # Cascades to delete labels, deps, comments

# Import from JSONL
$WORKSPACE/target/debug/bf sync --import-only

# Verify labels were restored
echo "10. Verifying labels after import..."
sqlite3 .beads/beads.db "SELECT bead_id, label FROM bead_labels ORDER BY bead_id, label;"

AFTER_IMPORT=$(sqlite3 .beads/beads.db "SELECT COUNT(*) FROM bead_labels;")
echo "  Total labels after import: $AFTER_IMPORT"

if [ "$BEFORE_COUNT" -eq "$AFTER_IMPORT" ]; then
    echo "  ✓ Label count matches!"
else
    echo "  ✗ Label count mismatch! Expected $BEFORE_COUNT, got $AFTER_IMPORT"
    exit 1
fi

# Detailed verification
echo "11. Detailed verification of each bead..."

# Bead 1 should have 4 labels
BF1_LABELS=$(sqlite3 .beads/beads.db "SELECT COUNT(*) FROM bead_labels WHERE bead_id='$BF1';")
if [ "$BF1_LABELS" -eq 4 ]; then
    echo "  ✓ $BF1: 4 labels (correct)"
else
    echo "  ✗ $BF1: expected 4 labels, got $BF1_LABELS"
    exit 1
fi

# Bead 2 should have 1 label
BF2_LABELS=$(sqlite3 .beads/beads.db "SELECT COUNT(*) FROM bead_labels WHERE bead_id='$BF2';")
if [ "$BF2_LABELS" -eq 1 ]; then
    echo "  ✓ $BF2: 1 label (correct)"
else
    echo "  ✗ $BF2: expected 1 label, got $BF2_LABELS"
    exit 1
fi

# Bead 3 should have 0 labels
BF3_LABELS=$(sqlite3 .beads/beads.db "SELECT COUNT(*) FROM bead_labels WHERE bead_id='$BF3';")
if [ "$BF3_LABELS" -eq 0 ]; then
    echo "  ✓ $BF3: 0 labels (correct)"
else
    echo "  ✗ $BF3: expected 0 labels, got $BF3_LABELS"
    exit 1
fi

# Verify specific label values
echo "12. Verifying specific label values..."
BF1_PHASE1=$(sqlite3 .beads/beads.db "SELECT COUNT(*) FROM bead_labels WHERE bead_id='$BF1' AND label='phase-1';")
BF1_STORAGE=$(sqlite3 .beads/beads.db "SELECT COUNT(*) FROM bead_labels WHERE bead_id='$BF1' AND label='storage';")
BF1_CRITICAL=$(sqlite3 .beads/beads.db "SELECT COUNT(*) FROM bead_labels WHERE bead_id='$BF1' AND label='critical';")
BF1_TEST=$(sqlite3 .beads/beads.db "SELECT COUNT(*) FROM bead_labels WHERE bead_id='$BF1' AND label='test-label';")

if [ "$BF1_PHASE1" -eq 1 ] && [ "$BF1_STORAGE" -eq 1 ] && [ "$BF1_CRITICAL" -eq 1 ] && [ "$BF1_TEST" -eq 1 ]; then
    echo "  ✓ All labels for $BF1 are correct"
else
    echo "  ✗ Label values mismatch for $BF1"
    echo "    phase-1: $BF1_PHASE1 (expected 1)"
    echo "    storage: $BF1_STORAGE (expected 1)"
    echo "    critical: $BF1_CRITICAL (expected 1)"
    echo "    test-label: $BF1_TEST (expected 1)"
    exit 1
fi

BF2_PHASE2=$(sqlite3 .beads/beads.db "SELECT COUNT(*) FROM bead_labels WHERE bead_id='$BF2' AND label='phase-2';")
if [ "$BF2_PHASE2" -eq 1 ]; then
    echo "  ✓ Label for $BF2 is correct"
else
    echo "  ✗ Label value mismatch for $BF2"
    exit 1
fi

# Test 2: Verify that re-exporting produces identical JSONL
echo "13. Testing that re-export produces identical JSONL..."
$WORKSPACE/target/debug/bf sync --flush-only
if diff -q issues.jsonl.backup .beads/issues.jsonl; then
    echo "  ✓ Re-exported JSONL is identical to original"
else
    echo "  ✗ Re-exported JSONL differs from original"
    echo "  Differences:"
    diff issues.jsonl.backup .beads/issues.jsonl || true
    exit 1
fi

echo ""
echo "=== All label export/import round-trip tests passed! ==="

# Cleanup
cd /
rm -rf "$TEST_WS"

exit 0
