#!/usr/bin/bash
# Test epic functionality with labels
set -e

TITLE="Test Epic with Labels $(date +%s)"

echo "=== Test: Create Epic with Labels ==="
# Create an epic with initial labels and capture the generated ID
EPIC_ID=$(bf create \
  --type epic \
  --title "$TITLE" \
  --description "Test epic for label functionality" \
  --label phase-1 \
  --label test-epic \
  --label epic-test \
  --priority 0)

if [ -z "$EPIC_ID" ]; then
    echo "ERROR: Failed to create epic with labels"
    exit 1
fi

echo "✓ Created epic $EPIC_ID with labels"
echo ""

echo "=== Test: Show Epic (verify labels present) ==="
OUTPUT=$(bf show "$EPIC_ID" --json)
echo "$OUTPUT" | jq -e '.[0].labels | length >= 3' > /dev/null || {
    echo "ERROR: Epic doesn't have at least 3 labels"
    echo "$OUTPUT"
    exit 1
}

# Verify specific labels exist
echo "$OUTPUT" | jq -e '.[0].labels | index("phase-1") != null' > /dev/null || {
    echo "ERROR: Label 'phase-1' not found"
    exit 1
}
echo "$OUTPUT" | jq -e '.[0].labels | index("test-epic") != null' > /dev/null || {
    echo "ERROR: Label 'test-epic' not found"
    exit 1
}
echo "$OUTPUT" | jq -e '.[0].labels | index("epic-test") != null' > /dev/null || {
    echo "ERROR: Label 'epic-test' not found"
    exit 1
}

echo "✓ All initial labels verified present"
echo ""

echo "=== Test: List Labels on Epic ==="
bf label list "$EPIC_ID" | grep -q "phase-1" || {
    echo "ERROR: 'phase-1' not in label list"
    exit 1
}
bf label list "$EPIC_ID" | grep -q "test-epic" || {
    echo "ERROR: 'test-epic' not in label list"
    exit 1
}
echo "✓ Label list command works correctly"
echo ""

echo "=== Test: Add Additional Label to Epic ==="
bf label add "$EPIC_ID" --label additional-label || {
    echo "ERROR: Failed to add additional label"
    exit 1
}

# Verify new label exists
OUTPUT=$(bf show "$EPIC_ID" --json)
echo "$OUTPUT" | jq -e '.[0].labels | index("additional-label") != null' > /dev/null || {
    echo "ERROR: New label 'additional-label' not found after add"
    exit 1
}
echo "✓ Additional label added successfully"
echo ""

echo "=== Test: Remove Label from Epic ==="
bf label remove "$EPIC_ID" --label additional-label || {
    echo "ERROR: Failed to remove label"
    exit 1
}

# Verify label was removed
OUTPUT=$(bf show "$EPIC_ID" --json)
echo "$OUTPUT" | jq -e '.[0].labels | index("additional-label") == null' > /dev/null || {
    echo "ERROR: Label 'additional-label' still present after removal"
    exit 1
}
echo "✓ Label removed successfully"
echo ""

echo "=== Test: Verify Epic Type ==="
OUTPUT=$(bf show "$EPIC_ID" --json)
ISSUE_TYPE=$(echo "$OUTPUT" | jq -r '.[0].issue_type')
if [ "$ISSUE_TYPE" != "epic" ]; then
    echo "ERROR: Expected issue_type 'epic', got '$ISSUE_TYPE'"
    exit 1
fi
echo "✓ Epic type verified as 'epic'"
echo ""

echo "=== Test: Search Beads Filtered by Label ==="
# Test that we can find our epic by label (search returns JSONL)
SEARCH_OUTPUT=$(bf search --label phase-1 --format json)
echo "$SEARCH_OUTPUT" | grep -q "\"id\":\"$EPIC_ID\"" || {
    echo "ERROR: Epic not found when filtering by label 'phase-1'"
    exit 1
}
echo "✓ Epic found in label-filtered search"
echo ""

echo "=== Test: Search Beads Filtered by Type ==="
# Test that we can find our epic by type (search returns JSONL)
SEARCH_OUTPUT=$(bf search --type epic --format json)
echo "$SEARCH_OUTPUT" | grep -q "\"id\":\"$EPIC_ID\"" || {
    echo "ERROR: Epic not found when filtering by type 'epic'"
    exit 1
}
echo "✓ Epic found in type-filtered search"
echo ""

echo "=== Test: Search Beads Filtered by Both Type and Label ==="
# Test that we can find our epic by both type and label (search returns JSONL)
SEARCH_OUTPUT=$(bf search --type epic --label phase-1 --format json)
echo "$SEARCH_OUTPUT" | grep -q "\"id\":\"$EPIC_ID\"" || {
    echo "ERROR: Epic not found when filtering by both type 'epic' and label 'phase-1'"
    exit 1
}
echo "✓ Epic found in combined type+label filtered search"
echo ""

echo "=== Test: Close Epic ==="
bf close "$EPIC_ID" --reason "Test completed successfully" || {
    echo "ERROR: Failed to close epic"
    exit 1
}

# Verify epic is closed
OUTPUT=$(bf show "$EPIC_ID" --json)
STATUS=$(echo "$OUTPUT" | jq -r '.[0].status')
if [ "$STATUS" != "closed" ]; then
    echo "ERROR: Expected status 'closed', got '$STATUS'"
    exit 1
fi
echo "✓ Epic closed successfully"
echo ""

echo "=== All Tests Passed ==="
echo "Epic ID: $EPIC_ID"
echo "Test completed successfully!"
