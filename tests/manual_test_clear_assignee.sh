#!/bin/bash
# Manual end-to-end test for bf update --clear-assignee
# This test verifies the clear-assignee functionality works as expected
#
# Acceptance criteria:
# 1. Create a test bead with an assignee
# 2. Run bf update --clear-assignee on the bead
# 3. Verify the command succeeds without error
# 4. Confirm the assignee field is cleared in the output

set -e  # Exit on any error

echo "======================================"
echo "Manual Test: bf update --clear-assignee"
echo "======================================"

# Set up test workspace
TEST_DIR=$(mktemp -d)
echo "Test workspace: $TEST_DIR"

# Cleanup function
cleanup() {
    echo "Cleaning up test workspace..."
    rm -rf "$TEST_DIR"
}

trap cleanup EXIT

# Initialize bf workspace
cd "$TEST_DIR"
echo "Initializing bf workspace..."
bf init --prefix test > /dev/null 2>&1

# Step 1: Create a test bead with an assignee
echo ""
echo "Step 1: Creating test bead with assignee..."
BEAD_ID=$(bf create --title "Test Clear Assignee" --type task --priority 2 --assignee "test-worker" --json | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['id'])")
echo "Created bead: $BEAD_ID"

# Verify initial assignee
echo "Verifying initial assignee..."
ASSIGNEE=$(bf show "$BEAD_ID" --format json --envelope | python3 -c "import sys,json; data=json.load(sys.stdin); bead=data.get('data',{}); print(bead.get('assignee') if isinstance(bead,dict) else 'null')")
echo "Initial assignee: $ASSIGNEE"

if [ "$ASSIGNEE" != "test-worker" ]; then
    echo "FAIL: Initial assignee should be 'test-worker', got '$ASSIGNEE'"
    exit 1
fi

# Step 2: Run bf update --clear-assignee on the bead
echo ""
echo "Step 2: Running bf update --clear-assignee..."
if bf update "$BEAD_ID" --clear-assignee > /dev/null 2>&1; then
    echo "✓ Command succeeded without error"
else
    echo "✗ FAIL: bf update --clear-assignee failed"
    exit 1
fi

# Step 3: Verify the command succeeds without error (already done above)
echo ""
echo "Step 3: Command success verified ✓"

# Step 4: Confirm the assignee field is cleared in the output
echo ""
echo "Step 4: Verifying assignee field is cleared..."
FINAL_ASSIGNEE=$(bf show "$BEAD_ID" --format json --envelope | python3 -c "import sys,json; data=json.load(sys.stdin); bead=data.get('data',{}); print(bead.get('assignee') if isinstance(bead,dict) and bead.get('assignee') else 'null')")
echo "Final assignee value: $FINAL_ASSIGNEE"

if [ "$FINAL_ASSIGNEE" == "null" ] || [ -z "$FINAL_ASSIGNEE" ]; then
    echo "✓ Assignee field successfully cleared"
else
    echo "✗ FAIL: Assignee should be cleared (null), got '$FINAL_ASSIGNEE'"
    exit 1
fi

# Display the final bead state for manual inspection
echo ""
echo "Final bead state:"
bf show "$BEAD_ID"

echo ""
echo "======================================"
echo "✓ All acceptance criteria passed!"
echo "======================================"
exit 0
