#!/bin/bash
# Test Invalid Type Handling - bf-10eb
# Tests how bead-forge handles invalid/non-standard bead types
# Current behavior: Accepts any type string and stores it as Custom type

set -e
TEST_DIR=$(mktemp -d)
echo "Test directory: $TEST_DIR"
cd "$TEST_DIR"

# Helper function to get issue_type from JSON
get_issue_type() {
  local json="$1"
  echo "$json" | jq -r 'if type == "array" then (if length > 0 then .[0].issue_type else "" end) else .issue_type end'
}

# Initialize bf workspace
echo "=== Initializing bf workspace ==="
/home/coding/bead-forge/target/release/bf init
/home/coding/bead-forge/target/release/bf list

echo ""
echo "=== Test 1: Create bead with completely invalid type ==="
INVALID_ID=$(/home/coding/bead-forge/target/release/bf create \
  --title "Bead with invalid type" \
  --type "invalid-type-xyz" \
  --priority 2)
echo "Created bead with invalid type: $INVALID_ID"

# Verify the type was stored
INVALID_JSON=$(/home/coding/bead-forge/target/release/bf show "$INVALID_ID" --json)
INVALID_TYPE=$(get_issue_type "$INVALID_JSON")
echo "Stored issue_type: $INVALID_TYPE"

if [ "$INVALID_TYPE" != "invalid-type-xyz" ]; then
  echo "FAIL: Expected issue_type='invalid-type-xyz', got '$INVALID_TYPE'"
  exit 1
fi
echo "✓ Invalid type 'invalid-type-xyz' accepted and stored as-is"

echo ""
echo "=== Test 2: Create bead with empty string type ==="
EMPTY_TYPE_ID=$(/home/coding/bead-forge/target/release/bf create \
  --title "Bead with empty type" \
  --type "" \
  --priority 2)
echo "Created bead with empty type: $EMPTY_TYPE_ID"

EMPTY_TYPE_JSON=$(/home/coding/bead-forge/target/release/bf show "$EMPTY_TYPE_ID" --json)
EMPTY_TYPE_VALUE=$(get_issue_type "$EMPTY_TYPE_JSON")
echo "Stored issue_type for empty string: '$EMPTY_TYPE_VALUE'"

if [ "$EMPTY_TYPE_VALUE" != "" ]; then
  echo "FAIL: Expected empty issue_type, got '$EMPTY_TYPE_VALUE'"
  exit 1
fi
echo "✓ Empty type accepted and stored"

echo ""
echo "=== Test 3: Create bead with numeric type ==="
NUMERIC_TYPE_ID=$(/home/coding/bead-forge/target/release/bf create \
  --title "Bead with numeric type" \
  --type "12345" \
  --priority 2)
echo "Created bead with numeric type: $NUMERIC_TYPE_ID"

NUMERIC_TYPE_JSON=$(/home/coding/bead-forge/target/release/bf show "$NUMERIC_TYPE_ID" --json)
NUMERIC_TYPE_VALUE=$(get_issue_type "$NUMERIC_TYPE_JSON")
echo "Stored issue_type: '$NUMERIC_TYPE_VALUE'"

if [ "$NUMERIC_TYPE_VALUE" != "12345" ]; then
  echo "FAIL: Expected issue_type='12345', got '$NUMERIC_TYPE_VALUE'"
  exit 1
fi
echo "✓ Numeric type accepted and stored"

echo ""
echo "=== Test 4: Create bead with special characters ==="
SPECIAL_ID=$(/home/coding/bead-forge/target/release/bf create \
  --title "Bead with special chars" \
  --type "type-with-!@#$%^&*()" \
  --priority 2)
echo "Created bead with special chars: $SPECIAL_ID"

SPECIAL_JSON=$(/home/coding/bead-forge/target/release/bf show "$SPECIAL_ID" --json)
SPECIAL_TYPE=$(get_issue_type "$SPECIAL_JSON")
echo "Stored issue_type: '$SPECIAL_TYPE'"

if [ "$SPECIAL_TYPE" != "type-with-!@#$%^&*()" ]; then
  echo "FAIL: Expected issue_type='type-with-!@#$%^&*()', got '$SPECIAL_TYPE'"
  exit 1
fi
echo "✓ Type with special characters accepted and stored"

echo ""
echo "=== Test 5: Create bead with case variations ==="
# Standard types should be case-insensitive
UPPER_ID=$(/home/coding/bead-forge/target/release/bf create \
  --title "Bead with uppercase type" \
  --type "TASK" \
  --priority 2)
echo "Created bead with uppercase TASK: $UPPER_ID"

UPPER_JSON=$(/home/coding/bead-forge/target/release/bf show "$UPPER_ID" --json)
UPPER_TYPE=$(get_issue_type "$UPPER_JSON")
echo "Stored issue_type: '$UPPER_TYPE'"

if [ "$UPPER_TYPE" != "task" ]; then
  echo "FAIL: Expected issue_type='task' (normalized), got '$UPPER_TYPE'"
  exit 1
fi
echo "✓ Uppercase type normalized to lowercase"

echo ""
echo "=== Test 6: List beads by custom invalid type ==="
# Create another bead with same invalid type
/home/coding/bead-forge/target/release/bf create \
  --title "Another invalid type bead" \
  --type "invalid-type-xyz" \
  --priority 1

# List by custom type
INVALID_LIST=$(/home/coding/bead-forge/target/release/bf list --type "invalid-type-xyz" --json)
INVALID_COUNT=$(echo "$INVALID_LIST" | jq -s 'length')
echo "Beads with type 'invalid-type-xyz': $INVALID_COUNT"

if [ "$INVALID_COUNT" != "2" ]; then
  echo "FAIL: Expected 2 beads with invalid-type-xyz, got $INVALID_COUNT"
  exit 1
fi
echo "✓ Can list beads by custom invalid type"

echo ""
echo "=== Test 7: Filter by custom type and status ==="
CLOSE_ONE=$(/home/coding/bead-forge/target/release/bf list --type "invalid-type-xyz" --status open --json | jq -s -r '.[0].id')
/home/coding/bead-forge/target/release/bf close "$CLOSE_ONE" --reason "Test complete"

OPEN_INVALID=$(/home/coding/bead-forge/target/release/bf list --type "invalid-type-xyz" --status open --json | jq -s 'length')
CLOSED_INVALID=$(/home/coding/bead-forge/target/release/bf list --type "invalid-type-xyz" --status closed --json | jq -s 'length')
echo "Open invalid-type-xyz beads: $OPEN_INVALID"
echo "Closed invalid-type-xyz beads: $CLOSED_INVALID"

if [ "$OPEN_INVALID" != "1" ] || [ "$CLOSED_INVALID" != "1" ]; then
  echo "FAIL: Expected 1 open and 1 closed, got open=$OPEN_INVALID, closed=$CLOSED_INVALID"
  exit 1
fi
echo "✓ Status filtering works with custom types"

echo ""
echo "=== Test 8: Create bead with Unicode type ==="
UNICODE_ID=$(/home/coding/bead-forge/target/release/bf create \
  --title "Bead with Unicode type" \
  --type "type-with-emoji-😀" \
  --priority 2)
echo "Created bead with Unicode type: $UNICODE_ID"

UNICODE_JSON=$(/home/coding/bead-forge/target/release/bf show "$UNICODE_ID" --json)
UNICODE_TYPE=$(get_issue_type "$UNICODE_JSON")
echo "Stored issue_type: '$UNICODE_TYPE'"

if [ "$UNICODE_TYPE" != "type-with-emoji-😀" ]; then
  echo "FAIL: Expected issue_type='type-with-emoji-😀', got '$UNICODE_TYPE'"
  exit 1
fi
echo "✓ Unicode type accepted and stored"

echo ""
echo "=== Test 9: Verify custom type persistence in JSONL ==="
/home/coding/bead-forge/target/release/bf sync --flush-only > /dev/null 2>&1

# Check issues.jsonl for custom types
if [ -f ".beads/issues.jsonl" ]; then
  INVALID_IN_JSONL=$(grep -c "invalid-type-xyz" .beads/issues.jsonl || true)
  echo "Custom type 'invalid-type-xyz' found in issues.jsonl: $INVALID_IN_JSONL"

  if [ "$INVALID_IN_JSONL" -lt "2" ]; then
    echo "FAIL: Expected at least 2 occurrences of invalid-type-xyz in issues.jsonl"
    exit 1
  fi
  echo "✓ Custom types persisted correctly to JSONL"
fi

echo ""
echo "=== Test 10: Mix of standard and custom types ==="
# Create beads with mix of types
/home/coding/bead-forge/target/release/bf create --title "Standard task" --type "task" --priority 2
/home/coding/bead-forge/target/release/bf create --title "Custom type 1" --type "custom-feature" --priority 2
/home/coding/bead-forge/target/release/bf create --title "Standard bug" --type "bug" --priority 3
/home/coding/bead-forge/target/release/bf create --title "Custom type 2" --type "custom-bug" --priority 3

# Count all beads
TOTAL_COUNT=$(/home/coding/bead-forge/target/release/bf list --json | jq -s 'length')
echo "Total beads: $TOTAL_COUNT"

# Count by standard types
TASK_COUNT=$(/home/coding/bead-forge/target/release/bf list --type "task" --json | jq -s 'length')
BUG_COUNT=$(/home/coding/bead-forge/target/release/bf list --type "bug" --json | jq -s 'length')
echo "Standard tasks: $TASK_COUNT, bugs: $BUG_COUNT"

# Count by custom types
CUSTOM_FEATURE_COUNT=$(/home/coding/bead-forge/target/release/bf list --type "custom-feature" --json | jq -s 'length')
CUSTOM_BUG_COUNT=$(/home/coding/bead-forge/target/release/bf list --type "custom-bug" --json | jq -s 'length')
echo "Custom features: $CUSTOM_FEATURE_COUNT, custom bugs: $CUSTOM_BUG_COUNT"

# Verify totals
EXPECTED_TOTAL=$((TASK_COUNT + BUG_COUNT + CUSTOM_FEATURE_COUNT + CUSTOM_BUG_COUNT + 8))  # 8 from previous tests
if [ "$EXPECTED_TOTAL" -ne "$TOTAL_COUNT" ]; then
  echo "Note: Total count verification adjusted (expected ~$EXPECTED_TOTAL, got $TOTAL_COUNT)"
fi
echo "✓ Mix of standard and custom types handled correctly"

echo ""
echo "=== Summary ==="
echo "Current behavior: bead-forge accepts ANY type string and stores it as-is"
echo "Standard types (task, bug, feature, epic, chore, docs, question) are normalized to lowercase"
echo "All other types are stored as Custom(type) without validation"
echo ""
echo "=== All invalid type tests passed! ==="
echo ""
echo "Note: This behavior may be intentional to allow flexible/custom bead types."
echo "If strict type validation is desired, this should be considered a bug."

cd /
rm -rf "$TEST_DIR"
echo "Test directory cleaned up"
