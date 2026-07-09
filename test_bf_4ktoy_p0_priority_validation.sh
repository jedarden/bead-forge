#!/bin/bash
# Test Epic P0 Critical Priority Validation - bf-4ktoy
# Comprehensive test for P0 priority handling in epic beads

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

echo "=== Epic P0 Critical Priority Validation Test Suite ==="
echo ""

# Initialize bf workspace
echo "=== Initializing bf workspace ==="
/home/coding/bead-forge/target/release/bf init --prefix p0test
echo "✓ Workspace initialized"
echo ""

# Test 1: Create epic with P0 critical priority
echo "=== Test 1: Create epic with P0 critical priority ==="
EPIC_P0=$(/home/coding/bead-forge/target/release/bf create \
  --title "Epic: Critical System Security Fix" \
  --type epic \
  --priority 0 \
  --description "Critical security patch for authentication vulnerability")
echo "Created epic: $EPIC_P0"

# Verify epic was created
if [ -z "$EPIC_P0" ]; then
  echo "FAIL: Epic creation returned empty ID"
  exit 1
fi
echo "✓ Epic created with ID: $EPIC_P0"
echo ""

# Test 2: Verify priority in JSON output
echo "=== Test 2: Verify P0 priority in JSON output ==="
EPIC_JSON=$(/home/coding/bead-forge/target/release/bf show "$EPIC_P0" --json)
echo "$EPIC_JSON" | jq '.'

PRIORITY=$(get_field "$EPIC_JSON" "priority")
if [ "$PRIORITY" != "0" ]; then
  echo "FAIL: Expected priority='0', got '$PRIORITY'"
  exit 1
fi
echo "✓ Priority in JSON is correct: $PRIORITY (P0)"
echo ""

# Test 3: Verify issue type in JSON output
echo "=== Test 3: Verify epic type in JSON output ==="
ISSUE_TYPE=$(get_field "$EPIC_JSON" "issue_type")
if [ "$ISSUE_TYPE" != "epic" ]; then
  echo "FAIL: Expected issue_type='epic', got '$ISSUE_TYPE'"
  exit 1
fi
echo "✓ Issue type in JSON is correct: $ISSUE_TYPE"
echo ""

# Test 4: Verify P0 priority display in text output
echo "=== Test 4: Verify P0 priority display in text output ==="
EPIC_TEXT=$(/home/coding/bead-forge/target/release/bf show "$EPIC_P0")
echo "$EPIC_TEXT"

if echo "$EPIC_TEXT" | grep -q "Priority: P0"; then
  echo "✓ P0 priority displayed correctly in text output"
else
  echo "FAIL: P0 priority not displayed correctly in text output"
  echo "Expected to find 'Priority: P0' in output"
  exit 1
fi

if echo "$EPIC_TEXT" | grep -q "Type: epic"; then
  echo "✓ Epic type displayed correctly in text output"
else
  echo "FAIL: Epic type not displayed correctly in text output"
  exit 1
fi
echo ""

# Test 5: Create epics with different priorities for comparison
echo "=== Test 5: Create epics with P1, P2, P3 priorities ==="
EPIC_P1=$(/home/coding/bead-forge/target/release/bf create \
  --title "Epic: Feature Enhancement" \
  --type epic \
  --priority 1)
echo "Created P1 epic: $EPIC_P1"

EPIC_P2=$(/home/coding/bead-forge/target/release/bf create \
  --title "Epic: Documentation Updates" \
  --type epic \
  --priority 2)
echo "Created P2 epic: $EPIC_P2"

EPIC_P3=$(/home/coding/bead-forge/target/release/bf create \
  --title "Epic: Nice-to-have Features" \
  --type epic \
  --priority 3)
echo "Created P3 epic: $EPIC_P3"
echo "✓ Created epics with priorities P1, P2, P3"
echo ""

# Test 6: Verify each epic has correct priority
echo "=== Test 6: Verify all epic priorities ==="
for epic_id in "$EPIC_P0" "$EPIC_P1" "$EPIC_P2" "$EPIC_P3"; do
  epic_json=$(/home/coding/bead-forge/target/release/bf show "$epic_id" --json)
  priority=$(get_field "$epic_json" "priority")
  issue_type=$(get_field "$epic_json" "issue_type")

  if [ "$issue_type" != "epic" ]; then
    echo "FAIL: Epic $epic_id has incorrect issue_type: $issue_type"
    exit 1
  fi

  if [ "$priority" -lt 0 ] || [ "$priority" -gt 3 ]; then
    echo "FAIL: Epic $epic_id has invalid priority: $priority"
    exit 1
  fi

  echo "✓ Epic $epic_id: type=$issue_type, priority=P$priority"
done
echo ""

# Test 7: Filter by epic type and count
echo "=== Test 7: Count total epic beads ==="
ALL_EPICS=$(/home/coding/bead-forge/target/release/bf list --type epic --json)
EPIC_COUNT=$(echo "$ALL_EPICS" | grep -c '{' || true)
echo "Total epic count: $EPIC_COUNT"

if [ "$EPIC_COUNT" != "4" ]; then
  echo "FAIL: Expected 4 epics, got $EPIC_COUNT"
  exit 1
fi
echo "✓ Correct epic count: $EPIC_COUNT"
echo ""

# Test 8: Verify P0 is the highest priority
echo "=== Test 8: Verify P0 is highest priority ==="
# P0 should appear first when listing by priority (if sorting is implemented)
# For now, just verify we can filter and find P0 epics
P0_EPICS=$(/home/coding/bead-forge/target/release/bf list --type epic --priority 0 --json)
P0_COUNT=$(echo "$P0_EPICS" | grep -c '{' || true)

if [ "$P0_COUNT" != "1" ]; then
  echo "FAIL: Expected 1 P0 epic, got $P0_COUNT"
  exit 1
fi
echo "✓ Found exactly 1 P0 epic as expected"
echo ""

# Test 9: Create child tasks with P0 priority
echo "=== Test 9: Create child tasks with P0 priority ==="
TASK_P0_1=$(/home/coding/bead-forge/target/release/bf create \
  --title "Fix authentication bypass vulnerability" \
  --type task \
  --priority 0)
echo "Created P0 task 1: $TASK_P0_1"

TASK_P0_2=$(/home/coding/bead-forge/target/release/bf create \
  --title "Patch session hijacking issue" \
  --type task \
  --priority 0)
echo "Created P0 task 2: $TASK_P0_2"

# Verify tasks have P0 priority
for task_id in "$TASK_P0_1" "$TASK_P0_2"; do
  task_json=$(/home/coding/bead-forge/target/release/bf show "$task_id" --json)
  task_priority=$(get_field "$task_json" "priority")
  task_type=$(get_field "$task_json" "issue_type")

  if [ "$task_priority" != "0" ]; then
    echo "FAIL: Task $task_id should have priority 0, got $task_priority"
    exit 1
  fi

  echo "✓ Task $task_id: type=$task_type, priority=P0"
done
echo ""

# Test 10: Link children to epic
echo "=== Test 10: Link P0 tasks to P0 epic ==="
/home/coding/bead-forge/target/release/bf dep add "$EPIC_P0" "$TASK_P0_1" -t parent-child
/home/coding/bead-forge/target/release/bf dep add "$EPIC_P0" "$TASK_P0_2" -t parent-child
echo "✓ Linked P0 tasks to P0 epic"

# Verify dependencies
EPIC_DEPS=$(/home/coding/bead-forge/target/release/bf dep list "$EPIC_P0" | wc -l)
if [ "$EPIC_DEPS" != "2" ]; then
  echo "FAIL: Expected 2 dependencies, got $EPIC_DEPS"
  exit 1
fi
echo "✓ Epic has 2 child dependencies"
echo ""

# Test 11: Verify serialization to JSONL
echo "=== Test 11: Verify P0 priority serialization to JSONL ==="
SYNC_OUTPUT=$(/home/coding/bead-forge/target/release/bf sync --flush-only 2>&1)
echo "$SYNC_OUTPUT"

if [ -f ".beads/issues.jsonl" ]; then
  # Check for P0 priority in JSONL
  P0_IN_JSONL=$(grep '"priority":0' .beads/issues.jsonl | wc -l)
  echo "P0 priority entries in issues.jsonl: $P0_IN_JSONL"

  if [ "$P0_IN_JSONL" -lt "1" ]; then
    echo "FAIL: Expected at least 1 P0 priority entry in issues.jsonl"
    exit 1
  fi
  echo "✓ P0 priority serialized correctly to JSONL"

  # Check for epic type in JSONL
  EPIC_IN_JSONL=$(grep '"issue_type":"epic"' .beads/issues.jsonl | wc -l)
  echo "Epic type entries in issues.jsonl: $EPIC_IN_JSONL"

  if [ "$EPIC_IN_JSONL" -lt "1" ]; then
    echo "FAIL: Expected at least 1 epic type entry in issues.jsonl"
    exit 1
  fi
  echo "✓ Epic type serialized correctly to JSONL"
else
  echo "FAIL: issues.jsonl not found after sync"
  exit 1
fi
echo ""

# Test 12: Verify P0 epic details in comprehensive output
echo "=== Test 12: Verify comprehensive P0 epic details ==="
FINAL_CHECK=$(/home/coding/bead-forge/target/release/bf show "$EPIC_P0" --json)
echo "$FINAL_CHECK" | jq '.'

FINAL_PRIORITY=$(get_field "$FINAL_CHECK" "priority")
FINAL_TYPE=$(get_field "$FINAL_CHECK" "issue_type")
FINAL_TITLE=$(get_field "$FINAL_CHECK" "title")

if [ "$FINAL_PRIORITY" != "0" ]; then
  echo "FAIL: Final priority check failed: expected 0, got $FINAL_PRIORITY"
  exit 1
fi

if [ "$FINAL_TYPE" != "epic" ]; then
  echo "FAIL: Final type check failed: expected epic, got $FINAL_TYPE"
  exit 1
fi

if [[ ! "$FINAL_TITLE" =~ "Critical System Security Fix" ]]; then
  echo "FAIL: Final title check failed: title does not match"
  exit 1
fi

echo "✓ All P0 epic details verified correctly"
echo ""

echo "=== All P0 Priority Validation Tests Passed ✓ ==="
echo ""
echo "Summary:"
echo "- Epic creation with P0 priority works correctly"
echo "- Priority 0 is correctly stored and retrieved"
echo "- Issue type 'epic' is correctly set"
echo "- Text output displays P0 priority correctly"
echo "- JSON serialization works correctly"
echo "- Filtering by P0 priority works"
echo "- P0 tasks can be linked to P0 epics"
echo "- JSONL serialization preserves P0 priority"
echo ""

echo "Test statistics:"
echo "- Total epics created: $EPIC_COUNT"
echo "- P0 epics: 1"
echo "- P0 tasks: 2"
echo "- All priorities validated: P0, P1, P2, P3"
echo ""

# Cleanup
cd /
rm -rf "$TEST_DIR"
echo "✓ Test directory cleaned up"
echo ""
echo "=== Test Complete ==="
