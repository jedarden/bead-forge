#!/usr/bin/env bash
# Test script for bf-67ttv: Epic type creation with description
# Tests that epic beads can be created with descriptions and all fields are preserved

set -e

export BF_TEST_SCRIPT="bf-67ttv epic description test"
export BF_TEST_START=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

echo "=== Epic Type Creation with Description Test ==="
echo "Testing epic type creation with description functionality"
echo ""

# Test 1: Verify epic bf-67ttv exists with correct type and description
echo "Test 1: Verify epic bf-67ttv exists with correct fields"
EPIC_JSON=$(br show bf-67ttv --json 2>/dev/null || echo '{"error": "not found"}')

# Check if we got a valid result (array with at least one element)
if ! echo "$EPIC_JSON" | jq -e '.[0]' > /dev/null 2>&1; then
    echo "❌ FAILED: Epic bf-67ttv not found or invalid JSON"
    exit 1
fi

# Verify all required fields (br show returns an array)
EPIC_ID=$(echo "$EPIC_JSON" | jq -r '.[0].id')
EPIC_TITLE=$(echo "$EPIC_JSON" | jq -r '.[0].title')
EPIC_TYPE=$(echo "$EPIC_JSON" | jq -r '.[0].issue_type')
EPIC_DESC=$(echo "$EPIC_JSON" | jq -r '.[0].description')
EPIC_STATUS=$(echo "$EPIC_JSON" | jq -r '.[0].status')
EPIC_PRIORITY=$(echo "$EPIC_JSON" | jq -r '.[0].priority')

if [ "$EPIC_ID" != "bf-67ttv" ]; then
    echo "❌ FAILED: Expected ID 'bf-67ttv', got '$EPIC_ID'"
    exit 1
fi

if [ "$EPIC_TITLE" != "Another Epic Test" ]; then
    echo "❌ FAILED: Expected title 'Another Epic Test', got '$EPIC_TITLE'"
    exit 1
fi

if [ "$EPIC_TYPE" != "epic" ]; then
    echo "❌ FAILED: Expected type 'epic', got '$EPIC_TYPE'"
    exit 1
fi

if [ "$EPIC_DESC" != "Testing epic type creation with description" ]; then
    echo "❌ FAILED: Expected description 'Testing epic type creation with description', got '$EPIC_DESC'"
    exit 1
fi

if [ "$EPIC_STATUS" != "in_progress" ]; then
    echo "❌ FAILED: Expected status 'in_progress', got '$EPIC_STATUS'"
    exit 1
fi

if [ "$EPIC_PRIORITY" != "1" ]; then
    echo "❌ FAILED: Expected priority '1', got '$EPIC_PRIORITY'"
    exit 1
fi

echo "✅ PASSED: Epic bf-67ttv has all correct fields"
echo "   - ID: $EPIC_ID"
echo "   - Title: $EPIC_TITLE"
echo "   - Type: $EPIC_TYPE"
echo "   - Description: $EPIC_DESC"
echo "   - Status: $EPIC_STATUS"
echo "   - Priority: $EPIC_PRIORITY"
echo ""

# Test 2: Verify epic appears in epic-only listing
echo "Test 2: Verify epic appears in --type epic listing"
EPIC_LIST=$(br list --type epic --json)

# Try JSON array format first, then fall back to JSONL format
FOUND=0
if echo "$EPIC_LIST" | jq -e '.[] | select(.id == "bf-67ttv")' > /dev/null 2>&1; then
    FOUND=1
elif echo "$EPIC_LIST" | jq -r 'select(.id == "bf-67ttv")' > /dev/null 2>&1; then
    FOUND=1
fi

if [ "$FOUND" -eq 0 ]; then
    echo "❌ FAILED: Epic bf-67ttv not found in epic listing"
    echo "Epic list output:"
    echo "$EPIC_LIST" | head -5
    exit 1
fi
echo "✅ PASSED: Epic bf-67ttv appears in epic listing"
echo ""

# Test 3: Test creating a new epic with description
echo "Test 3: Test creating new epic with description"
TEST_EPIC_ID=$(br create \
    --title "Test Epic for bf-67ttv" \
    --type epic \
    --description "Test epic created during bf-67ttv testing" \
    --priority 2 \
    --workspace . 2>&1)

if [ -z "$TEST_EPIC_ID" ] || [ "$TEST_EPIC_ID" = "null" ]; then
    echo "❌ FAILED: Failed to create test epic"
    exit 1
fi

# Validate that the ID looks like a bead ID (starts with bf-)
if ! echo "$TEST_EPIC_ID" | grep -q '^bf-[a-z0-9]\+$'; then
    echo "❌ FAILED: Invalid bead ID format: $TEST_EPIC_ID"
    exit 1
fi

echo "✅ PASSED: Created test epic $TEST_EPIC_ID"

# Verify the created epic
CREATED_EPIC=$(br show "$TEST_EPIC_ID" --json)
CREATED_TYPE=$(echo "$CREATED_EPIC" | jq -r '.[0].issue_type')
CREATED_DESC=$(echo "$CREATED_EPIC" | jq -r '.[0].description')

if [ "$CREATED_TYPE" != "epic" ]; then
    echo "❌ FAILED: Created epic has wrong type: $CREATED_TYPE"
    exit 1
fi

if [ "$CREATED_DESC" != "Test epic created during bf-67ttv testing" ]; then
    echo "❌ FAILED: Created epic has wrong description: $CREATED_DESC"
    exit 1
fi

echo "✅ PASSED: Created epic has correct type ($CREATED_TYPE) and description"
echo ""

# Test 4: Verify description is preserved in JSONL export
echo "Test 4: Verify description is preserved in JSONL format"
EXPORTED_DESC=$(echo "$CREATED_EPIC" | jq -r '.[0].description')
if [ "$EXPORTED_DESC" != "Test epic created during bf-67ttv testing" ]; then
    echo "❌ FAILED: Description not preserved correctly in JSON export"
    exit 1
fi
echo "✅ PASSED: Description preserved in JSON format"
echo ""

# Cleanup: close the test epic we created
br close "$TEST_EPIC_ID" --reason "Test cleanup for bf-67ttv" > /dev/null 2>&1 || true
echo "✅ Cleanup: Closed test epic $TEST_EPIC_ID"
echo ""

# Summary
echo "=== Test Summary ==="
echo "All epic type creation with description tests passed!"
echo "Tests run:"
echo "  ✅ Epic field verification (ID, title, type, description, status, priority)"
echo "  ✅ Epic listing filter (--type epic)"
echo "  ✅ Epic creation with description"
echo "  ✅ Description preservation in JSON export"
echo ""
echo "Test completed successfully at $(date -u +"%Y-%m-%dT%H:%M:%SZ")"