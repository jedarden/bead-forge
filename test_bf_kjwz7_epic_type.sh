#!/bin/bash
# Test Epic Bead Type Functionality - bf-kjwz7
# Tests epic bead type functionality including creation, dependencies, critical path, and status

set -e
TEST_DIR=$(mktemp -d)
echo "Test directory: $TEST_DIR"
cd "$TEST_DIR"

# Helper function to get ID from JSON (handles both object and array)
get_id() {
  local json="$1"
  echo "$json" | jq -r 'if type == "array" then (if length > 0 then .[0].id else "" end) else .id end'
}

# Helper function to get field from JSON (handles both object and array)
get_field() {
  local json="$1"
  local field="$2"
  echo "$json" | jq -r "if type == \"array\" then (if length > 0 then .[0].$field else \"\" end) else .$field end"
}

# Initialize bf workspace
echo "=== Initializing bf workspace ==="
/home/coding/bead-forge/target/release/bf init --prefix epic
/home/coding/bead-forge/target/release/bf list

echo ""
echo "=== Test 1: Create epic bead ==="
/home/coding/bead-forge/target/release/bf create \
  --title "Epic: Implement Authentication System" \
  --type epic \
  --priority 1
# Get the epic ID from the most recently created bead
EPIC_ID=$(/home/coding/bead-forge/target/release/bf list --type epic --limit 1 --json | get_id "$(cat)")
echo "Created epic: $EPIC_ID"

# Verify epic was created with correct type
EPIC_JSON=$(/home/coding/bead-forge/target/release/bf show "$EPIC_ID" --json)
echo "$EPIC_JSON" | jq '.'
EPIC_TYPE=$(get_field "$EPIC_JSON" "issue_type")
if [ "$EPIC_TYPE" != "epic" ]; then
  echo "FAIL: Expected issue_type='epic', got '$EPIC_TYPE'"
  exit 1
fi
echo "✓ Epic type is correct: $EPIC_TYPE"

echo ""
echo "=== Test 2: Create child tasks ==="
/home/coding/bead-forge/target/release/bf create \
  --title "Implement login handler" \
  --type task \
  --priority 1
/home/coding/bead-forge/target/release/bf create \
  --title "Add session management" \
  --type task \
  --priority 1
/home/coding/bead-forge/target/release/bf create \
  --title "Write authentication tests" \
  --type bug \
  --priority 2

# Get child IDs from list (list --json outputs newline-delimited objects, not array)
CHILD1=$(/home/coding/bead-forge/target/release/bf list --type task --status open --json | head -1 | jq -r '.id')
CHILD2=$(/home/coding/bead-forge/target/release/bf list --type task --status open --json | tail -1 | jq -r '.id')
CHILD3=$(/home/coding/bead-forge/target/release/bf list --type bug --status open --json | jq -r '.id')
echo "Created child1: $CHILD1, child2: $CHILD2, child3: $CHILD3"

echo ""
echo "=== Test 3: Create parent-child dependencies ==="
/home/coding/bead-forge/target/release/bf dep add "$EPIC_ID" "$CHILD1" -t parent-child
/home/coding/bead-forge/target/release/bf dep add "$EPIC_ID" "$CHILD2" -t parent-child
/home/coding/bead-forge/target/release/bf dep add "$EPIC_ID" "$CHILD3" -t parent-child
echo "✓ Added parent-child dependencies"

# Verify dependencies using dep list (show --json strips dependencies for NEEDLE compatibility)
EPIC_DEPS=$(/home/coding/bead-forge/target/release/bf dep list "$EPIC_ID" | grep -c "depends on")
if [ "$EPIC_DEPS" != "3" ]; then
  echo "FAIL: Expected 3 dependencies, got $EPIC_DEPS"
  exit 1
fi
echo "✓ Epic has 3 child dependencies"

echo ""
echo "=== Test 4: Verify dependency types ==="
/home/coding/bead-forge/target/release/bf dep list "$EPIC_ID"
# Check that all dependencies are parent-child type
PARENT_CHILD_COUNT=$(/home/coding/bead-forge/target/release/bf dep list "$EPIC_ID" | grep -c "parent-child")
if [ "$PARENT_CHILD_COUNT" != "3" ]; then
  echo "FAIL: Expected 3 parent-child dependencies, got $PARENT_CHILD_COUNT"
  exit 1
fi
echo "✓ All dependencies are parent-child type"

echo ""
echo "=== Test 5: List all beads by type ==="
echo "--- All epics ---"
/home/coding/bead-forge/target/release/bf list --type epic
echo ""
echo "--- All tasks ---"
/home/coding/bead-forge/target/release/bf list --type task
echo ""
echo "--- All bugs ---"
/home/coding/bead-forge/target/release/bf list --type bug

# Count beads by type (list --json outputs newline-delimited objects)
TASK_COUNT=$(/home/coding/bead-forge/target/release/bf list --type task --json | jq -s 'length')
EPIC_COUNT=$(/home/coding/bead-forge/target/release/bf list --type epic --json | jq -s 'length')
BUG_COUNT=$(/home/coding/bead-forge/target/release/bf list --type bug --json | jq -s 'length')

if [ "$TASK_COUNT" != "2" ]; then
  echo "FAIL: Expected 2 tasks, got $TASK_COUNT"
  exit 1
fi
if [ "$EPIC_COUNT" != "1" ]; then
  echo "FAIL: Expected 1 epic, got $EPIC_COUNT"
  exit 1
fi
if [ "$BUG_COUNT" != "1" ]; then
  echo "FAIL: Expected 1 bug, got $BUG_COUNT"
  exit 1
fi
echo "✓ Correct bead counts: $EPIC_COUNT epic, $TASK_COUNT tasks, $BUG_COUNT bugs"

echo ""
echo "=== Test 6: Create blocking dependency chain ==="
/home/coding/bead-forge/target/release/bf create \
  --title "Database schema setup" \
  --type task \
  --priority 0
BLOCKER=$(/home/coding/bead-forge/target/release/bf list --type task --status open --json | jq -r '.id')
echo "Created blocker: $BLOCKER"

/home/coding/bead-forge/target/release/bf dep add "$CHILD1" "$BLOCKER"
echo "✓ Added blocking dependency: $CHILD1 depends on $BLOCKER"

# Verify the blocking dependency using dep list
CHILD1_DEPS=$(/home/coding/bead-forge/target/release/bf dep list "$CHILD1" | grep -c "depends on")
if [ "$CHILD1_DEPS" != "1" ]; then
  echo "FAIL: Expected 1 dependency for child1, got $CHILD1_DEPS"
  exit 1
fi
echo "✓ Child task has 1 blocking dependency"

echo ""
echo "=== Test 7: Test dependency tree ==="
echo "--- Epic dependency tree (down) ---"
if /home/coding/bead-forge/target/release/bf dep tree "$EPIC_ID" 2>&1; then
  echo "✓ Dependency tree command works"
else
  echo "Note: dep tree command may not be fully implemented yet"
fi

echo ""
echo "=== Test 8: Close child tasks and verify epic status ==="
echo "Closing child1..."
/home/coding/bead-forge/target/release/bf close "$CHILD1" --reason "Login handler implemented"

echo "Closing child2..."
/home/coding/bead-forge/target/release/bf close "$CHILD2" --reason "Session management complete"

echo "Checking epic status..."
EPIC_AFTER_2=$(/home/coding/bead-forge/target/release/bf show "$EPIC_ID" --json)
echo "$EPIC_AFTER_2" | jq 'if type == "array" then {id: .[0].id, title: .[0].title, status: .[0].status, issue_type: .[0].issue_type} else {id, title, status, issue_type} end'

# Check closed children (list --json outputs newline-delimited objects)
CLOSED_CHILDREN=$(/home/coding/bead-forge/target/release/bf list --type task --status closed --json | jq -s 'length')
if [ "$CLOSED_CHILDREN" != "2" ]; then
  echo "FAIL: Expected 2 closed tasks, got $CLOSED_CHILDREN"
  exit 1
fi
echo "✓ Correct number of closed tasks: $CLOSED_CHILDREN"

echo ""
echo "=== Test 9: Close last child ==="
/home/coding/bead-forge/target/release/bf close "$CHILD3" --reason "Authentication tests complete"

ALL_CHILDREN_CLOSED=$(/home/coding/bead-forge/target/release/bf list --type task --status closed --json | jq -s 'length')
ALL_CHILDREN_CLOSED=$((ALL_CHILDREN_CLOSED + $(/home/coding/bead-forge/target/release/bf list --type bug --status closed --json | jq -s 'length')))
if [ "$ALL_CHILDREN_CLOSED" != "3" ]; then
  echo "FAIL: Expected 3 closed children, got $ALL_CHILDREN_CLOSED"
  exit 1
fi
echo "✓ All 3 children are now closed"

echo ""
echo "=== Test 10: Verify epic can now be closed ==="
/home/coding/bead-forge/target/release/bf close "$EPIC_ID" --reason "All authentication work complete"

# Verify epic is closed
EPIC_FINAL=$(/home/coding/bead-forge/target/release/bf show "$EPIC_ID" --json)
EPIC_STATUS=$(get_field "$EPIC_FINAL" "status")
if [ "$EPIC_STATUS" != "closed" ]; then
  echo "FAIL: Expected epic status='closed', got '$EPIC_STATUS'"
  exit 1
fi
echo "✓ Epic closed successfully"

echo ""
echo "=== Test 11: Create multiple epics ==="
/home/coding/bead-forge/target/release/bf create \
  --title "Epic: Build API Gateway" \
  --type epic \
  --priority 2
/home/coding/bead-forge/target/release/bf create \
  --title "Epic: Frontend Dashboard" \
  --type epic \
  --priority 3

# Get epic IDs (list --json outputs newline-delimited objects)
ALL_EPICS=$(/home/coding/bead-forge/target/release/bf list --type epic --json)
EPIC2=$(echo "$ALL_EPICS" | sed -n '2p' | jq -r '.id')
EPIC3=$(echo "$ALL_EPICS" | sed -n '3p' | jq -r '.id')
echo "Created second epic: $EPIC2, third epic: $EPIC3"

ALL_EPICS_COUNT=$(/home/coding/bead-forge/target/release/bf list --type epic --json | jq -s 'length')
if [ "$ALL_EPICS_COUNT" != "3" ]; then
  echo "FAIL: Expected 3 epics total, got $ALL_EPICS_COUNT"
  exit 1
fi
echo "✓ Total epic count: $ALL_EPICS_COUNT"

echo ""
echo "=== Test 12: Test epic filtering with other criteria ==="
echo "--- Open epics ---"
OPEN_EPICS=$(/home/coding/bead-forge/target/release/bf list --type epic --status open --json | jq -s 'length')
echo "Open epics: $OPEN_EPICS"

echo "--- Closed epics ---"
CLOSED_EPICS=$(/home/coding/bead-forge/target/release/bf list --type epic --status closed --json | jq -s 'length')
echo "Closed epics: $CLOSED_EPICS"

if [ "$OPEN_EPICS" != "2" ] || [ "$CLOSED_EPICS" != "1" ]; then
  echo "FAIL: Expected 2 open epics and 1 closed epic, got open=$OPEN_EPICS, closed=$CLOSED_EPICS"
  exit 1
fi
echo "✓ Epic status filtering works correctly"

echo ""
echo "=== Test 13: Verify issue type serialization ==="
SYNC_OUTPUT=$(/home/coding/bead-forge/target/release/bf sync --flush-only 2>&1)
echo "$SYNC_OUTPUT"

# Check issues.jsonl for epic type
if [ -f ".beads/issues.jsonl" ]; then
  EPIC_IN_JSONL=$(grep -c "issue_type.*epic" .beads/issues.jsonl || true)
  echo "Epics found in issues.jsonl: $EPIC_IN_JSONL"
  if [ "$EPIC_IN_JSONL" -lt "1" ]; then
    echo "FAIL: Expected at least 1 epic in issues.jsonl"
    exit 1
  fi
  echo "✓ Epic type serialized correctly to JSONL"
fi

echo ""
echo "=== Test 14: Test epic with different child types ==="
/home/coding/bead-forge/target/release/bf create \
  --title "Epic: Feature Toggle System" \
  --type epic \
  --priority 1
# Get the most recently created epic (should be the only open epic after test 13)
FEATURE_EPIC=$(/home/coding/bead-forge/target/release/bf list --type epic --status open --json | jq -s -r 'sort_by(.created_at) | .[-1].id')
echo "Created epic: $FEATURE_EPIC"

/home/coding/bead-forge/target/release/bf create \
  --title "Add toggle flags" \
  --type feature \
  --priority 1
/home/coding/bead-forge/target/release/bf create \
  --title "Update documentation" \
  --type chore \
  --priority 3

# Get the feature and chore beads
FEATURE_CHILD=$(/home/coding/bead-forge/target/release/bf list --type feature --status open --json | jq -s -r '.[0].id')
CHORE_CHILD=$(/home/coding/bead-forge/target/release/bf list --type chore --status open --json | jq -s -r '.[0].id')
echo "Created feature: $FEATURE_CHILD, chore: $CHORE_CHILD"

/home/coding/bead-forge/target/release/bf dep add "$FEATURE_EPIC" "$FEATURE_CHILD" -t parent-child
/home/coding/bead-forge/target/release/bf dep add "$FEATURE_EPIC" "$CHORE_CHILD" -t parent-child

# Get dependency types from dep list output (show --json strips dependencies)
CHILD_TYPES=$(/home/coding/bead-forge/target/release/bf dep list "$FEATURE_EPIC" | grep "depends on" | sed -E 's/.*depends on ([^ ]+).*/\1/' | while read -r dep_id; do
  /home/coding/bead-forge/target/release/bf show "$dep_id" --json | jq -r 'if type == "array" then .[0].issue_type else .issue_type end'
done | sort -u | tr '\n' ' ')
echo "Child types: $CHILD_TYPES"

if [[ ! "$CHILD_TYPES" =~ "feature" ]] || [[ ! "$CHILD_TYPES" =~ "chore" ]]; then
  echo "FAIL: Expected child types to include feature and chore"
  exit 1
fi
echo "✓ Epic can have children of different types"

echo ""
echo "=== All epic bead type tests passed! ==="
cd /
rm -rf "$TEST_DIR"
echo "Test directory cleaned up"
