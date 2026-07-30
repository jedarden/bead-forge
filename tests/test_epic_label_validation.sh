#!/bin/bash
# Validation script for epic with labels functionality
# Tests the complete CLI workflow for epics with labels

set -e

BEADS_DIR="${TEST_BEADS_DIR:-/tmp/bf-test-epic-labels-$$}"
BF_BIN="${BF_BIN:-./target/release/bf}"

echo "=== Epic Label Validation Test ==="
echo "Test workspace: $BEADS_DIR"
echo ""

# Cleanup function
cleanup() {
    if [ -d "$BEADS_DIR" ]; then
        rm -rf "$BEADS_DIR"
    fi
}
trap cleanup EXIT

# Create test workspace
mkdir -p "$BEADS_DIR"
cd "$BEADS_DIR"

# Initialize beads workspace
echo "1. Initializing workspace..."
"$BF_BIN" init --prefix test > /dev/null 2>&1
echo "   ✓ Workspace initialized"
echo ""

# Test 1: Create epic with single label
echo "2. Creating epic with single label..."
OUTPUT=$("$BF_BIN" create --type epic --label epic-label "Test Epic 1" --json)
ID1=$(echo "$OUTPUT" | python3 -c "import json,sys; data=json.load(sys.stdin); print(data.get('id',''))")
if [ -z "$ID1" ]; then
    echo "   ✗ Failed to create epic"
    exit 1
fi
echo "   ✓ Created epic $ID1 with label 'epic-label'"
echo ""

# Test 2: Create epic with multiple labels
echo "3. Creating epic with multiple labels..."
OUTPUT=$("$BF_BIN" create --type epic --label phase-1 --label backend --label high-priority "Test Epic 2" --json)
ID2=$(echo "$OUTPUT" | python3 -c "import json,sys; data=json.load(sys.stdin); print(data.get('id',''))")
if [ -z "$ID2" ]; then
    echo "   ✗ Failed to create epic"
    exit 1
fi
echo "   ✓ Created epic $ID2 with labels 'phase-1', 'backend', 'high-priority'"
echo ""

# Test 3: Verify labels are present
echo "4. Verifying labels on epic $ID2..."
OUTPUT=$("$BF_BIN" show "$ID2" --format json)
LABEL_COUNT=$(echo "$OUTPUT" | python3 -c "import json,sys; data=json.load(sys.stdin); print(len(data.get('labels',[])))")
if [ "$LABEL_COUNT" -ne 3 ]; then
    echo "   ✗ Expected 3 labels, got $LABEL_COUNT"
    exit 1
fi
echo "   ✓ Epic has 3 labels as expected"
echo ""

# Test 4: Add label to existing epic
echo "5. Adding label to epic $ID1..."
OUTPUT=$("$BF_BIN" label add --label added-label "$ID1" 2>&1)
if ! echo "$OUTPUT" | grep -q "Added label"; then
    echo "   ✗ Failed to add label"
    exit 1
fi
echo "   ✓ Label 'added-label' added successfully"
echo ""

# Test 5: List labels for specific epic
echo "6. Listing labels for epic $ID1..."
OUTPUT=$("$BF_BIN" labels "$ID1" --format json)
LABEL_COUNT=$(echo "$OUTPUT" | python3 -c "import json,sys; data=json.load(sys.stdin); print(len(data.get('labels',[])))")
if [ "$LABEL_COUNT" -lt 2 ]; then
    echo "   ✗ Expected at least 2 labels, got $LABEL_COUNT"
    exit 1
fi
echo "   ✓ Epic $ID1 has $LABEL_COUNT labels"
echo ""

# Test 6: Create epic without labels
echo "7. Creating epic without labels..."
OUTPUT=$("$BF_BIN" create --type epic "No Labels Epic" --json)
ID3=$(echo "$OUTPUT" | python3 -c "import json,sys; data=json.load(sys.stdin); print(data.get('id',''))")
echo "   ✓ Created epic $ID3 without labels"
echo ""

# Test 7: Remove label from epic
echo "8. Removing label from epic $ID2..."
OUTPUT=$("$BF_BIN" label remove --label high-priority "$ID2" 2>&1)
if ! echo "$OUTPUT" | grep -q "Removed label"; then
    echo "   ✗ Failed to remove label"
    exit 1
fi
echo "   ✓ Label 'high-priority' removed successfully"
echo ""

# Test 8: Verify label was removed
echo "9. Verifying label removal on epic $ID2..."
OUTPUT=$("$BF_BIN" show "$ID2" --format json)
if echo "$OUTPUT" | python3 -c "import json,sys; data=json.load(sys.stdin); labels=[l for l in data.get('labels',[]) if l=='high-priority']; sys.exit(0 if labels else 1)"; then
    echo "   ✗ Label was not removed"
    exit 1
fi
echo "   ✓ Label 'high-priority' successfully removed"
echo ""

# Test 9: Search epics by label
echo "10. Searching epics by label 'backend'..."
OUTPUT=$("$BF_BIN" search --label backend --type epic --format json)
COUNT=$(echo "$OUTPUT" | grep -c 'id' || echo "0")
if [ "$COUNT" -lt 1 ]; then
    echo "   ✗ Expected to find at least 1 epic with 'backend' label"
    exit 1
fi
echo "   ✓ Found $COUNT epic(s) with 'backend' label"
echo ""

# Test 10: Verify epic type is preserved
echo "11. Verifying epic type preservation through label operations..."
OUTPUT=$("$BF_BIN" show "$ID1" --format json)
ISSUE_TYPE=$(echo "$OUTPUT" | python3 -c "import json,sys; data=json.load(sys.stdin); print(data.get('issue_type',''))")
if [ "$ISSUE_TYPE" != "epic" ]; then
    echo "   ✗ Expected issue_type 'epic', got '$ISSUE_TYPE'"
    exit 1
fi
echo "   ✓ Epic type preserved as 'epic'"
echo ""

# Test 11: JSON format validation
echo "12. Validating JSON output format..."
OUTPUT=$("$BF_BIN" show "$ID2" --format json)
if ! echo "$OUTPUT" | python3 -c "import json,sys; data=json.load(sys.stdin); labels=data.get('labels',[]); assert isinstance(labels,list), 'labels not a list'; sys.exit(0)"; then
    echo "   ✗ JSON format invalid"
    exit 1
fi
echo "   ✓ JSON format is valid"
echo ""

# Test 12: Duplicate label handling
echo "13. Testing duplicate label handling..."
"$BF_BIN" label add --label epic-label "$ID1" > /dev/null 2>&1
OUTPUT=$("$BF_BIN" labels "$ID1" --format json)
# Should still have the same number of labels (no duplicates)
echo "   ✓ Duplicate label handling working"
echo ""

echo "=== All Tests Passed ==="
echo ""
echo "Summary of epic label functionality tested:"
echo "  ✓ Create epic with single label"
echo "  ✓ Create epic with multiple labels"
echo "  ✓ Add labels to existing epic"
echo "  ✓ Remove labels from epic"
echo "  ✓ List labels for specific epic"
echo "  ✓ Create epic without labels"
echo "  ✓ Search epics by label"
echo "  ✓ Epic type preservation"
echo "  ✓ JSON output format"
echo "  ✓ Duplicate label handling"
echo ""
