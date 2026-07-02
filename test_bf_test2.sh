#!/bin/bash
# Test 2: Basic bead CRUD operations smoke test
# Verifies that the bf CLI can create, list, show, update, and close beads

set -e

bf_build_path="/home/coding/bead-forge/target/debug/bf"

# Create temp workspace
TEMP_DIR=$(mktemp -d)
echo "Test 2: Using temp dir: $TEMP_DIR"

ORIGINAL_DIR=$(pwd)
cd "$TEMP_DIR"
mkdir -p .beads

echo "Test 2: Initializing workspace..."
$bf_build_path --workspace . init --prefix test

echo "Test 2: Creating a bead..."
BEAD_ID=$($bf_build_path --workspace . create --title "Test bead for bf-test2" --type task --priority 2 --description "Test description" | tr -d '\n')
echo "Created bead: $BEAD_ID"

if [ -z "$BEAD_ID" ]; then
    echo "✗ Failed to create bead"
    exit 1
fi

echo "Test 2: Listing beads..."
BEADS=$($bf_build_path --workspace . list --format json)
if echo "$BEADS" | grep -q "$BEAD_ID"; then
    echo "✓ Bead found in list"
else
    echo "✗ Bead not found in list"
    echo "$BEADS"
    exit 1
fi

echo "Test 2: Showing bead details..."
BEAD_DETAILS=$($bf_build_path --workspace . show "$BEAD_ID" --format json)
if echo "$BEAD_DETAILS" | grep -q "Test bead for bf-test2"; then
    echo "✓ Bead title correct"
else
    echo "✗ Bead title incorrect"
    echo "$BEAD_DETAILS"
    exit 1
fi

echo "Test 2: Updating bead status..."
$bf_build_path --workspace . update "$BEAD_ID" --status in_progress > /dev/null
UPDATED_STATUS=$($bf_build_path --workspace . show "$BEAD_ID" --format json | grep -o '"status":"[^"]*"' | cut -d'"' -f4)
if [ "$UPDATED_STATUS" = "in_progress" ]; then
    echo "✓ Bead status updated to in_progress"
else
    echo "✗ Bead status not updated correctly: $UPDATED_STATUS"
    exit 1
fi

echo "Test 2: Closing bead..."
$bf_build_path --workspace . close "$BEAD_ID" --reason "Test completed" > /dev/null
CLOSED_STATUS=$($bf_build_path --workspace . show "$BEAD_ID" --format json | grep -o '"status":"[^"]*"' | cut -d'"' -f4)
if [ "$CLOSED_STATUS" = "closed" ]; then
    echo "✓ Bead closed successfully"
else
    echo "✗ Bead not closed: $CLOSED_STATUS"
    exit 1
fi

echo "Test 2: Counting beads..."
COUNT=$($bf_build_path --workspace . count)
if [ "$COUNT" -ge 1 ]; then
    echo "✓ Bead count correct: $COUNT"
else
    echo "✗ Bead count incorrect: $COUNT"
    exit 1
fi

# Cleanup
cd "$ORIGINAL_DIR"
rm -rf "$TEMP_DIR"

echo "✓ Test 2 passed: bead CRUD operations are functional"
