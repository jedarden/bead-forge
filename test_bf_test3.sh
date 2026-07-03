#!/bin/bash
# Test 3: Comments functionality smoke test
# Verifies that the bf CLI can add and list comments on beads

set -e

bf_build_path="/home/coding/bead-forge/target/debug/bf"

# Create temp workspace
TEMP_DIR=$(mktemp -d)
echo "Test 3: Using temp dir: $TEMP_DIR"

ORIGINAL_DIR=$(pwd)
cd "$TEMP_DIR"
mkdir -p .beads

echo "Test 3: Initializing workspace..."
$bf_build_path --workspace . init --prefix test

echo "Test 3: Creating a bead..."
BEAD_ID=$($bf_build_path --workspace . create --title "Test bead for bf-test3" --type task --priority 2 --description "Test description" | tr -d '\n')
echo "Created bead: $BEAD_ID"

if [ -z "$BEAD_ID" ]; then
    echo "✗ Failed to create bead"
    exit 1
fi

echo "Test 3: Adding a comment to the bead..."
COMMENT_OUTPUT=$($bf_build_path --workspace . comments add "$BEAD_ID" "This is a test comment" 2>&1)
if echo "$COMMENT_OUTPUT" | grep -q "Added comment"; then
    echo "✓ Comment added successfully"
else
    echo "✗ Failed to add comment"
    echo "$COMMENT_OUTPUT"
    exit 1
fi

echo "Test 3: Listing comments for the bead..."
COMMENTS_LIST=$($bf_build_path --workspace . comments list "$BEAD_ID" 2>&1)
if echo "$COMMENTS_LIST" | grep -q "This is a test comment"; then
    echo "✓ Comment found in list"
else
    echo "✗ Comment not found in list"
    echo "$COMMENTS_LIST"
    exit 1
fi

echo "Test 3: Adding multiple comments..."
$bf_build_path --workspace . comments add "$BEAD_ID" "Second comment" > /dev/null
$bf_build_path --workspace . comments add "$BEAD_ID" "Third comment" > /dev/null

COMMENT_COUNT=$($bf_build_path --workspace . comments list "$BEAD_ID" 2>&1 | grep -c "comment" || true)
if [ "$COMMENT_COUNT" -ge 3 ]; then
    echo "✓ Multiple comments added and listed (count: $COMMENT_COUNT)"
else
    echo "✗ Expected at least 3 comments, got $COMMENT_COUNT"
    exit 1
fi

# Cleanup
cd "$ORIGINAL_DIR"
rm -rf "$TEMP_DIR"

echo "✓ Test 3 passed: comments functionality is working"
