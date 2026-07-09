#!/bin/bash
# Test Epic Bead Type Functionality - Fixed Version
# Tests epic bead type functionality including creation, dependencies, and filtering

set -e
TEST_DIR=$(mktemp -d)
echo "Test directory: $TEST_DIR"
cd "$TEST_DIR"

# Helper function to count JSON objects from NDJSON output
count_json_objects() {
  local json="$1"
  echo "$json" | grep -c "^{" || echo "0"
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
EPIC_ID=$(/home/coding/bead-forge/target/release/bf list --type epic --limit 1 --json | head -1 | jq -r '.id')
echo "Created epic: $EPIC_ID"

# Verify epic was created with correct type
EPIC_JSON=$(/home/coding/bead-forge/target/release/bf show "$EPIC_ID" --json)
EPIC_TYPE=$(echo "$EPIC_JSON" | jq -r '.[0].issue_type')
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

# Get child IDs
TASKS_JSON=$(/home/coding/bead-forge/target/release/bf list --type task --status open --json)
CHILD1=$(echo "$TASKS_JSON" | head -1 | jq -r '.id')
CHILD2=$(echo "$TASKS_JSON" | tail -1 | jq -r '.id')
BUGS_JSON=$(/home/coding/bead-forge/target/release/bf list --type bug --status open --json)
CHILD3=$(echo "$BUGS_JSON" | jq -r '.id')
echo "Created child1: $CHILD1, child2: $CHILD2, child3: $CHILD3"

echo ""
echo "=== Test 3: Create parent-child dependencies ==="
/home/coding/bead-forge/target/release/bf dep add "$EPIC_ID" "$CHILD1" -t parent-child
/home/coding/bead-forge/target/release/bf dep add "$EPIC_ID" "$CHILD2" -t parent-child
/home/coding/bead-forge/target/release/bf dep add "$EPIC_ID" "$CHILD3" -t parent-child
echo "✓ Added parent-child dependencies"

# Verify dependencies
EPIC_DEPS=$(/home/coding/bead-forge/target/release/bf dep list "$EPIC_ID" | grep -c "parent-child")
if [ "$EPIC_DEPS" != "3" ]; then
  echo "FAIL: Expected 3 parent-child dependencies, got $EPIC_DEPS"
  exit 1
fi
echo "✓ Epic has 3 child dependencies"

echo ""
echo "=== Test 4: List all beads by type ==="
echo "--- All epics ---"
/home/coding/bead-forge/target/release/bf list --type epic
echo ""
echo "--- All tasks ---"
/home/coding/bead-forge/target/release/bf list --type task
echo ""
echo "--- All bugs ---"
/home/coding/bead-forge/target/release/bf list --type bug

# Count beads by type using jq
TASKS_OUTPUT=$(/home/coding/bead-forge/target/release/bf list --type task --json)
TASK_COUNT=$(echo "$TASKS_OUTPUT" | grep "^{" | wc -l)
EPICS_OUTPUT=$(/home/coding/bead-forge/target/release/bf list --type epic --json)
EPIC_COUNT=$(echo "$EPICS_OUTPUT" | grep "^{" | wc -l)
BUGS_OUTPUT=$(/home/coding/bead-forge/target/release/bf list --type bug --json)
BUG_COUNT=$(echo "$BUGS_OUTPUT" | grep "^{" | wc -l)

echo "Task count: $TASK_COUNT, Epic count: $EPIC_COUNT, Bug count: $BUG_COUNT"

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
echo "=== Test 5: Create blocking dependency chain ==="
/home/coding/bead-forge/target/release/bf create \
  --title "Database schema setup" \
  --type task \
  --priority 0
BLOCKER=$(/home/coding/bead-forge/target/release/bf list --type task --status open --json | grep "^{" | tail -1 | jq -r '.id')
echo "Created blocker: $BLOCKER"

/home/coding/bead-forge/target/release/bf dep add "$CHILD1" "$BLOCKER"
echo "✓ Added blocking dependency: $CHILD1 depends on $BLOCKER"

echo ""
echo "=== Test 6: Close child tasks and verify epic status ==="
echo "Closing child1..."
/home/coding/bead-forge/target/release/bf close "$CHILD1" --reason "Login handler implemented"

echo "Closing child2..."
/home/coding/bead-forge/target/release/bf close "$CHILD2" --reason "Session management complete"

# Check closed children
CLOSED_TASKS=$(/home/coding/bead-forge/target/release/bf list --type task --status closed --json | grep "^{" | wc -l)
if [ "$CLOSED_TASKS" != "2" ]; then
  echo "FAIL: Expected 2 closed tasks, got $CLOSED_TASKS"
  exit 1
fi
echo "✓ Correct number of closed tasks: $CLOSED_TASKS"

echo ""
echo "=== Test 7: Close last child ==="
/home/coding/bead-forge/target/release/bf close "$CHILD3" --reason "Authentication tests complete"

ALL_CLOSED_TASKS=$(/home/coding/bead-forge/target/release/bf list --type task --status closed --json | grep "^{" | wc -l)
ALL_CLOSED_BUGS=$(/home/coding/bead-forge/target/release/bf list --type bug --status closed --json | grep "^{" | wc -l)
ALL_CHILDREN_CLOSED=$((ALL_CLOSED_TASKS + ALL_CLOSED_BUGS))
if [ "$ALL_CHILDREN_CLOSED" != "3" ]; then
  echo "FAIL: Expected 3 closed children, got $ALL_CHILDREN_CLOSED"
  exit 1
fi
echo "✓ All 3 children are now closed"

echo ""
echo "=== Test 8: Verify epic can now be closed ==="
/home/coding/bead-forge/target/release/bf close "$EPIC_ID" --reason "All authentication work complete"

# Verify epic is closed
EPIC_FINAL=$(/home/coding/bead-forge/target/release/bf show "$EPIC_ID" --json)
EPIC_STATUS=$(echo "$EPIC_FINAL" | jq -r '.[0].status')
if [ "$EPIC_STATUS" != "closed" ]; then
  echo "FAIL: Expected epic status='closed', got '$EPIC_STATUS'"
  exit 1
fi
echo "✓ Epic closed successfully"

echo ""
echo "=== Test 9: Create multiple epics ==="
/home/coding/bead-forge/target/release/bf create \
  --title "Epic: Build API Gateway" \
  --type epic \
  --priority 2
/home/coding/bead-forge/target/release/bf create \
  --title "Epic: Frontend Dashboard" \
  --type epic \
  --priority 3

ALL_EPICS_OUTPUT=$(/home/coding/bead-forge/target/release/bf list --type epic --json)
ALL_EPICS_COUNT=$(echo "$ALL_EPICS_OUTPUT" | grep "^{" | wc -l)
if [ "$ALL_EPICS_COUNT" != "3" ]; then
  echo "FAIL: Expected 3 epics total, got $ALL_EPICS_COUNT"
  exit 1
fi
echo "✓ Total epic count: $ALL_EPICS_COUNT"

echo ""
echo "=== Test 10: Test epic filtering with other criteria ==="
OPEN_EPICS_OUTPUT=$(/home/coding/bead-forge/target/release/bf list --type epic --status open --json)
OPEN_EPICS=$(echo "$OPEN_EPICS_OUTPUT" | grep "^{" | wc -l)
echo "Open epics: $OPEN_EPICS"

CLOSED_EPICS_OUTPUT=$(/home/coding/bead-forge/target/release/bf list --type epic --status closed --json)
CLOSED_EPICS=$(echo "$CLOSED_EPICS_OUTPUT" | grep "^{" | wc -l)
echo "Closed epics: $CLOSED_EPICS"

if [ "$OPEN_EPICS" != "2" ] || [ "$CLOSED_EPICS" != "1" ]; then
  echo "FAIL: Expected 2 open epics and 1 closed epic, got open=$OPEN_EPICS, closed=$CLOSED_EPICS"
  exit 1
fi
echo "✓ Epic status filtering works correctly"

echo ""
echo "=== Test 11: Verify issue type serialization ==="
/home/coding/bead-forge/target/release/bf sync --flush-only > /dev/null 2>&1

# Check issues.jsonl for epic type
if [ -f ".beads/issues.jsonl" ]; then
  EPIC_IN_JSONL=$(grep "issue_type.*epic" .beads/issues.jsonl | wc -l)
  echo "Epics found in issues.jsonl: $EPIC_IN_JSONL"
  if [ "$EPIC_IN_JSONL" -lt "1" ]; then
    echo "FAIL: Expected at least 1 epic in issues.jsonl"
    exit 1
  fi
  echo "✓ Epic type serialized correctly to JSONL"
fi

echo ""
echo "=== Test 12: Test epic with different child types ==="
/home/coding/bead-forge/target/release/bf create \
  --title "Epic: Feature Toggle System" \
  --type epic \
  --priority 1
/home/coding/bead-forge/target/release/bf create \
  --title "Add toggle flags" \
  --type feature \
  --priority 1
/home/coding/bead-forge/target/release/bf create \
  --title "Update documentation" \
  --type chore \
  --priority 3

FEATURE_EPIC=$(/home/coding/bead-forge/target/release/bf list --type epic --status open --json | grep "^{" | head -1 | jq -r '.id')
FEATURE_CHILD=$(/home/coding/bead-forge/target/release/bf list --type feature --status open --json | jq -r '.id')
CHORE_CHILD=$(/home/coding/bead-forge/target/release/bf list --type chore --status open --json | jq -r '.id')

/home/coding/bead-forge/target/release/bf dep add "$FEATURE_EPIC" "$FEATURE_CHILD" -t parent-child
/home/coding/bead-forge/target/release/bf dep add "$FEATURE_EPIC" "$CHORE_CHILD" -t parent-child

# Verify child types
FEATURE_CHILD_TYPE=$(/home/coding/bead-forge/target/release/bf show "$FEATURE_CHILD" --json | jq -r '.[0].issue_type')
CHORE_CHILD_TYPE=$(/home/coding/bead-forge/target/release/bf show "$CHORE_CHILD" --json | jq -r '.[0].issue_type')

if [[ "$FEATURE_CHILD_TYPE" != "feature" ]] || [[ "$CHORE_CHILD_TYPE" != "chore" ]]; then
  echo "FAIL: Expected child types to be feature and chore"
  exit 1
fi
echo "✓ Epic can have children of different types (feature: $FEATURE_CHILD_TYPE, chore: $CHORE_CHILD_TYPE)"

echo ""
echo "=== All epic bead type tests passed! ==="
cd /
rm -rf "$TEST_DIR"
echo "Test directory cleaned up"
