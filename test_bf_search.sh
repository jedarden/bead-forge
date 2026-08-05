#!/usr/bin/env bash
# Test script for bf search command functionality
# Bead: bf-2xyb9r

set -euo pipefail

TMP_WORKSPACE=$(mktemp -d)
trap "rm -rf $TMP_WORKSPACE" EXIT

echo "=== Test Bead D: bf search functionality ==="
echo

# Initialize test workspace
echo "1. Initializing test workspace..."
bf init --workspace "$TMP_WORKSPACE" > /dev/null 2>&1
echo "✓ Workspace initialized"
echo

# Create test beads with different content
echo "2. Creating test beads..."
BF_AUTH_ID=$(bf create --workspace "$TMP_WORKSPACE" --title "Fix authentication bug" --type bug --priority 0 2>&1)
echo "✓ Created $BF_AUTH_ID: 'Fix authentication bug'"

BF_DB_ID=$(bf create --workspace "$TMP_WORKSPACE" --title "Add database migration script" --type task --priority 1 2>&1)
echo "✓ Created $BF_DB_ID: 'Add database migration script'"

BF_UI_ID=$(bf create --workspace "$TMP_WORKSPACE" --title "Fix UI rendering bug" --type bug --priority 0 2>&1)
echo "✓ Created $BF_UI_ID: 'Fix UI rendering bug'"

BF_TEST_ID=$(bf create --workspace "$TMP_WORKSPACE" --title "Write unit tests for API" --type task --priority 2 2>&1)
echo "✓ Created $BF_TEST_ID: 'Write unit tests for API'"
echo

# Test 1: Search for "authentication" keyword
echo "3. Testing search for 'authentication' keyword..."
RESULT=$(bf search --workspace "$TMP_WORKSPACE" authentication 2>&1)
if echo "$RESULT" | grep -q "$BF_AUTH_ID"; then
    echo "✓ Found authentication bead ($BF_AUTH_ID)"
else
    echo "✗ Failed to find authentication bead"
    echo "Result: $RESULT"
    exit 1
fi
echo

# Test 2: Search for "database" keyword
echo "4. Testing search for 'database' keyword..."
RESULT=$(bf search --workspace "$TMP_WORKSPACE" database 2>&1)
if echo "$RESULT" | grep -q "$BF_DB_ID"; then
    echo "✓ Found database bead ($BF_DB_ID)"
else
    echo "✗ Failed to find database bead"
    echo "Result: $RESULT"
    exit 1
fi
echo

# Test 3: Search for "bug" keyword (should find multiple)
echo "5. Testing search for 'bug' keyword (multiple results)..."
RESULT=$(bf search --workspace "$TMP_WORKSPACE" bug 2>&1)
BUG_COUNT=$(echo "$RESULT" | grep -c "bf-" || true)
if [ "$BUG_COUNT" -ge 2 ]; then
    echo "✓ Found multiple bug beads ($BUG_COUNT bugs found)"
else
    echo "✗ Expected at least 2 bug beads, found $BUG_COUNT"
    echo "Result: $RESULT"
    exit 1
fi
echo

# Test 4: Search with non-existent keyword
echo "6. Testing search with non-existent keyword..."
RESULT=$(bf search --workspace "$TMP_WORKSPACE" nonexistentkeyword123 2>&1)
if [ -z "$RESULT" ] || ! echo "$RESULT" | grep -q "bf-"; then
    echo "✓ Correctly returns no results for non-existent keyword"
else
    echo "✗ Should not return results for non-existent keyword"
    echo "Result: $RESULT"
    exit 1
fi
echo

# Test 5: Search with type filter
echo "7. Testing search with type filter..."
RESULT=$(bf search --workspace "$TMP_WORKSPACE" --type bug 2>&1)
BUG_COUNT=$(echo "$RESULT" | grep -c "bf-" || true)
if [ "$BUG_COUNT" -eq 2 ]; then
    echo "✓ Type filter returns exactly 2 bugs"
else
    echo "✗ Expected 2 bug beads, found $BUG_COUNT"
    echo "Result: $RESULT"
    exit 1
fi
echo

# Test 6: Search with priority filter
echo "8. Testing search with priority filter..."
RESULT=$(bf search --workspace "$TMP_WORKSPACE" --priority-min 0 --priority-max 0 2>&1)
CRITICAL_COUNT=$(echo "$RESULT" | grep -c "bf-" || true)
if [ "$CRITICAL_COUNT" -eq 2 ]; then
    echo "✓ Priority filter returns exactly 2 critical (0) priority beads"
else
    echo "✗ Expected 2 critical priority beads, found $CRITICAL_COUNT"
    echo "Result: $RESULT"
    exit 1
fi
echo

# Test 7: Combined search (keyword + filter)
echo "9. Testing combined search (keyword + type filter)..."
RESULT=$(bf search --workspace "$TMP_WORKSPACE" bug --type bug 2>&1)
if echo "$RESULT" | grep -q "$BF_AUTH_ID" && echo "$RESULT" | grep -q "$BF_UI_ID"; then
    echo "✓ Combined search found both bug beads"
else
    echo "✗ Combined search failed to find expected beads"
    echo "Result: $RESULT"
    exit 1
fi
echo

echo "=== All tests passed! ==="
echo "Summary:"
echo "  ✓ Workspace initialization"
echo "  ✓ Bead creation (4 beads)"
echo "  ✓ Keyword search (authentication)"
echo "  ✓ Keyword search (database)"
echo "  ✓ Multi-result search (bug)"
echo "  ✓ Empty search handling"
echo "  ✓ Type filter"
echo "  ✓ Priority filter"
echo "  ✓ Combined search"
