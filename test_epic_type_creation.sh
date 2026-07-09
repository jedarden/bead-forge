#!/bin/bash
# Test script for epic type creation (bf-471tl)
# This script comprehensively tests the epic type functionality

set -e

echo "=== Epic Type Creation Test Suite ==="
echo ""

# Test 1: Create epic with priority P0
echo "Test 1: Creating epic with priority P0..."
EPIC_ID=$(bf create --title "Test epic P0 creation" --type epic --priority 0 --description "Testing epic with critical priority")
echo "Created epic: $EPIC_ID"
echo ""

# Test 2: Verify epic details
echo "Test 2: Verifying epic details..."
EPIC_DETAILS=$(bf show "$EPIC_ID")
echo "$EPIC_DETAILS"
if echo "$EPIC_DETAILS" | grep -q "Type: epic"; then
    echo "✓ Epic type correctly set"
else
    echo "✗ Epic type not set correctly"
    exit 1
fi
if echo "$EPIC_DETAILS" | grep -q "Priority: P0"; then
    echo "✓ Priority correctly set"
else
    echo "✗ Priority not set correctly"
    exit 1
fi
echo ""

# Test 3: Filter by epic type
echo "Test 3: Filtering beads by epic type..."
EPIC_LIST=$(bf list --type epic)
EPIC_COUNT=$(echo "$EPIC_LIST" | wc -l)
echo "Found $EPIC_COUNT epic beads"
if [ "$EPIC_COUNT" -gt 0 ]; then
    echo "✓ Epic type filtering works"
    echo "Sample epic beads:"
    echo "$EPIC_LIST" | head -5
else
    echo "✗ Epic type filtering failed"
    exit 1
fi
echo ""

# Test 4: Create epic with different priorities
echo "Test 4: Creating epic with priority P1..."
EPIC_ID_P1=$(bf create --title "Test epic P1 creation" --type epic --priority 1)
echo "Created epic: $EPIC_ID_P1"
if bf show "$EPIC_ID_P1" | grep -q "Priority: P1"; then
    echo "✓ Epic with P1 priority created successfully"
else
    echo "✗ Epic with P1 priority failed"
    exit 1
fi
echo ""

# Test 5: Create epic with default parameters
echo "Test 5: Creating epic with default priority..."
EPIC_DEFAULT=$(bf create --title "Test epic default priority" --type epic)
echo "Created epic: $EPIC_DEFAULT"
if bf show "$EPIC_DEFAULT" | grep -q "Priority: P2"; then
    echo "✓ Epic with default priority (P2) created successfully"
else
    echo "✗ Epic with default priority failed"
    exit 1
fi
echo ""

# Test 6: Test epic with labels
echo "Test 6: Creating epic with labels..."
EPIC_WITH_LABELS=$(bf create --title "Test epic with labels" --type epic --label test --label epic-test)
echo "Created epic: $EPIC_WITH_LABELS"
LABELS_OUTPUT=$(bf labels "$EPIC_WITH_LABELS")
if echo "$LABELS_OUTPUT" | grep -q "test" && echo "$LABELS_OUTPUT" | grep -q "epic-test"; then
    echo "✓ Epic with labels created successfully"
else
    echo "✗ Epic with labels failed"
    exit 1
fi
echo ""

# Test 7: Verify epic in JSON output
echo "Test 7: Testing epic serialization in JSON format..."
EPIC_JSON=$(bf show "$EPIC_ID" --json)
if echo "$EPIC_JSON" | python3 -c "import sys, json; data = json.load(sys.stdin); epics = [x for x in data if x.get('issue_type') == 'epic']; sys.exit(0 if len(epics) > 0 else 1)" 2>/dev/null; then
    echo "✓ Epic serializes correctly in JSON format"
else
    echo "✗ Epic JSON serialization failed"
    exit 1
fi
echo ""

# Test 8: Test epic with description
echo "Test 8: Creating epic with description..."
EPIC_WITH_DESC=$(bf create --title "Test epic with description" --type epic --description "This is a test epic with a detailed description")
echo "Created epic: $EPIC_WITH_DESC"
if bf show "$EPIC_WITH_DESC" | grep -q "Description: This is a test epic with a detailed description"; then
    echo "✓ Epic with description created successfully"
else
    echo "✗ Epic with description failed"
    exit 1
fi
echo ""

echo "=== All Epic Type Tests Passed ✓ ==="
echo ""
echo "Summary:"
echo "- Epic type creation works correctly"
echo "- Priority handling works (P0, P1, P2)"
echo "- Filtering by epic type works"
echo "- Labels work with epic beads"
echo "- JSON serialization works"
echo "- Descriptions work with epic beads"
echo ""
echo "Total epic beads in system: $(bf list --type epic | wc -l)"
