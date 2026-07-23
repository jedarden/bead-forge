#!/bin/bash
# Test bf-5go7x7: Label output format and persistence tests
# Acceptance criteria:
# - Labels shortcut command in text format (default)
# - Labels shortcut command in JSON format
# - Labels persist through sync --flush-only
# - Verify labels remain after sync operation

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BF_BIN="${CARGO_BIN_EXE_bf:-$PROJECT_ROOT/target/debug/bf}"
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"

echo "Test 1: Labels shortcut command in text format (default)"
"$BF_BIN" init --prefix test >/dev/null 2>&1
BEAD_ID=$("$BF_BIN" create --title "Test labels text" --type task --priority 2 --label urgent --label backend | tr -d '\n')

# Test labels shortcut in text format (default)
LABELS_OUTPUT=$("$BF_BIN" labels "$BEAD_ID")
if echo "$LABELS_OUTPUT" | grep -q "urgent" && echo "$LABELS_OUTPUT" | grep -q "backend"; then
    echo "✓ Text format output works: labels displayed correctly"
else
    echo "✗ Text format output failed"
    exit 1
fi

# Verify format is plain text (one label per line)
LABEL_COUNT=$(echo "$LABELS_OUTPUT" | grep -c . || true)
if [ "$LABEL_COUNT" -eq 2 ]; then
    echo "✓ Text format: correct number of lines (2 labels, 2 lines)"
else
    echo "✗ Text format: expected 2 lines, got $LABEL_COUNT"
    exit 1
fi

echo ""
echo "Test 2: Labels shortcut command in JSON format"
BEAD_ID2=$("$BF_BIN" create --title "Test labels json" --type task --priority 2 --label phase-1 --label bug | tr -d '\n')

# Test labels shortcut in JSON format
JSON_OUTPUT=$("$BF_BIN" labels "$BEAD_ID2" --format json)

# Verify JSON is valid and contains both labels
if echo "$JSON_OUTPUT" | jq -e '. | length == 2' > /dev/null 2>&1 && \
   echo "$JSON_OUTPUT" | jq -e '. | map(select(. == "phase-1" or . == "bug")) | length == 2' > /dev/null 2>&1; then
    echo "✓ JSON format output works: valid JSON with correct labels"
else
    echo "✗ JSON format output failed"
    echo "Got: $JSON_OUTPUT"
    exit 1
fi

echo ""
echo "Test 3: Labels persist through sync --flush-only"
BEAD_ID3=$("$BF_BIN" create --title "Test labels persistence" --type task --priority 2 | tr -d '\n')

# Add labels to the bead
"$BF_BIN" label add "$BEAD_ID3" --label frontend --label feature  >/dev/null 2>&1

# Verify labels were added
BEFORE_SYNC=$("$BF_BIN" labels "$BEAD_ID3" --format json)
if echo "$BEFORE_SYNC" | jq -e '. | length == 2' > /dev/null 2>&1; then
    echo "✓ Labels added before sync: 2 labels present"
else
    echo "✗ Failed to add labels before sync"
    exit 1
fi

# Perform sync --flush-only
"$BF_BIN" sync --flush-only >/dev/null 2>&1
echo "✓ Sync --flush-only completed"

# Verify labels are still present after sync
AFTER_SYNC=$("$BF_BIN" labels "$BEAD_ID3" --format json)
if echo "$AFTER_SYNC" | jq -e '. | length == 2' > /dev/null 2>&1; then
    echo "✓ Labels persist after sync: 2 labels still present"
else
    echo "✗ Labels lost after sync"
    echo "Before: $BEFORE_SYNC"
    echo "After:  $AFTER_SYNC"
    exit 1
fi

echo ""
echo "Test 4: Verify labels remain after full sync operation"
BEAD_ID4=$("$BF_BIN" create --title "Test labels full sync" --type task --priority 2 | tr -d '\n')

# Add labels
"$BF_BIN" label add "$BEAD_ID4" --label critical --label performance  >/dev/null 2>&1

# Get labels before full sync
BEFORE_FULL_SYNC=$("$BF_BIN" labels "$BEAD_ID4" --format json)
LABELS_BEFORE_COUNT=$(echo "$BEFORE_FULL_SYNC" | jq '. | length')
echo "✓ Before full sync: $LABELS_BEFORE_COUNT labels"

# Perform a full bidirectional sync (not just flush-only)
"$BF_BIN" sync >/dev/null 2>&1
echo "✓ Full bidirectional sync completed"

# Verify labels are still present
AFTER_FULL_SYNC=$("$BF_BIN" labels "$BEAD_ID4" --format json)
LABELS_AFTER_COUNT=$(echo "$AFTER_FULL_SYNC" | jq '. | length')
if [ "$LABELS_AFTER_COUNT" -eq "$LABELS_BEFORE_COUNT" ]; then
    echo "✓ Labels remain after full sync: $LABELS_AFTER_COUNT labels (same count)"
else
    echo "✗ Label count changed after full sync (was $LABELS_BEFORE_COUNT, now $LABELS_AFTER_COUNT)"
    exit 1
fi

# Verify the specific labels are still there
if echo "$AFTER_FULL_SYNC" | jq -e '. | map(select(. == "critical" or . == "performance")) | length == 2' > /dev/null 2>&1; then
    echo "✓ Specific labels verified after full sync: critical and performance both present"
else
    echo "✗ Expected labels not found after full sync"
    exit 1
fi

echo ""
echo "All tests passed! ✓"
echo "Cleanup: $TEST_DIR"
rm -rf "$TEST_DIR"
