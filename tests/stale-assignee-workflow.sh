#!/usr/bin/env bash
# End-to-end test for stale assignee clearing workflow
# This test simulates a bead with a stale assignee and verifies the fix

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BEAD_FORGE="$REPO_DIR/target/bin/bf"
export PATH="$PATH:$REPO_DIR/target/bin"

TEST_DIR="$(mktemp -d)"

echo "=== Test directory: $TEST_DIR ==="

# Cleanup on exit
trap "rm -rf '$TEST_DIR'" EXIT

# Initialize test workspace
cd "$TEST_DIR"
$BEAD_FORGE init --prefix test

echo "=== Step 1: Create test bead with stale assignee 'dead-worker-X' ==="
BEAD_ID=$($BEAD_FORGE create \
  --type task \
  --status open \
  --assignee "dead-worker-X" \
  --priority 2 \
  --title "Stale assignee test bead" \
  --description "This bead has a stale assignee from a dead worker" \
  --format json | jq -r '.data.id')

echo "Created bead: $BEAD_ID"

# Verify the assignee is set
ASSIGNEE=$($BEAD_FORGE show "$BEAD_ID" --format json | jq -r '.data.assignee')
echo "Assignee: $ASSIGNEE"

if [ "$ASSIGNEE" != "dead-worker-X" ]; then
  echo "FAIL: Expected assignee 'dead-worker-X', got '$ASSIGNEE'"
  exit 1
fi

echo "=== Step 2: Verify the bead appears in ready list (BUG) ==="
READY_BEADS=$($BEAD_FORGE ready --limit 0 --format json)
echo "Ready beads:"
echo "$READY_BEADS" | jq -r '.data[] | select(.id == "'"$BEAD_ID"'") | .id'

# Check if our bead appears in ready list
if echo "$READY_BEADS" | jq -e '.data[] | select(.id == "'"$BEAD_ID"'")' > /dev/null; then
  echo "BUG CONFIRMED: Bead with stale assignee appears in ready list!"
  echo "This is the bug we're testing - it should NOT appear."
else
  echo "Bead does not appear in ready list (expected after fix)"
fi

echo "=== Step 3: Clear the assignee using bf update --clear-assignee ==="
$BEAD_FORGE update "$BEAD_ID" --clear-assignee

# Verify the assignee is now NULL
ASSIGNEE=$($BEAD_FORGE show "$BEAD_ID" --format json | jq -r '.data.assignee')
echo "Assignee after clearing: $ASSIGNEE"

if [ "$ASSIGNEE" != "null" ] && [ "$ASSIGNEE" != "" ]; then
  echo "FAIL: Expected assignee to be null, got '$ASSIGNEE'"
  exit 1
fi

echo "=== Step 4: Verify the bead is still discoverable ==="
# The bead should now be a valid candidate for claiming
READY_BEADS=$($BEAD_FORGE ready --limit 0 --format json)
if echo "$READY_BEADS" | jq -e '.data[] | select(.id == "'"$BEAD_ID"'")' > /dev/null; then
  echo "SUCCESS: Bead is discoverable in ready list after clearing assignee"
else
  echo "FAIL: Bead is not discoverable after clearing assignee"
  exit 1
fi

echo "=== Step 5: Verify the bead can be searched and listed ==="
# Search by assignee null should find it
NULL_ASSIGNEE=$($BEAD_FORGE search --assignee "" --format json | jq -r '.data[] | select(.id == "'"$BEAD_ID"'") | .id')
if [ -n "$NULL_ASSIGNEE" ]; then
  echo "SUCCESS: Bead found when searching for unassigned beads"
else
  echo "WARNING: Search by empty assignee didn't find the bead"
fi

# List should show it
LISTED=$($BEAD_FORGE list --format json | jq -r '.data[] | select(.id == "'"$BEAD_ID"'") | .id')
if [ -n "$LISTED" ]; then
  echo "SUCCESS: Bead appears in list"
else
  echo "FAIL: Bead doesn't appear in list"
  exit 1
fi

echo "=== Step 6: Verify no other fields were affected ==="
TITLE=$($BEAD_FORGE show "$BEAD_ID" --format json | jq -r '.data.title')
STATUS=$($BEAD_FORGE show "$BEAD_ID" --format json | jq -r '.data.status')
PRIORITY=$($BEAD_FORGE show "$BEAD_ID" --format json | jq -r '.data.priority')

if [ "$TITLE" != "Stale assignee test bead" ]; then
  echo "FAIL: Title changed unexpectedly to '$TITLE'"
  exit 1
fi

if [ "$STATUS" != "open" ]; then
  echo "FAIL: Status changed unexpectedly to '$STATUS'"
  exit 1
fi

if [ "$PRIORITY" != "2" ]; then
  echo "FAIL: Priority changed unexpectedly to '$PRIORITY'"
  exit 1
fi

echo "=== All checks passed! ==="
echo "=== Workflow verified successfully ==="

exit 0
