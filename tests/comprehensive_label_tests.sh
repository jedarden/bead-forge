#!/bin/bash
# Comprehensive label tests for bead-forge CLI
# Tests all label operations including edge cases, persistence, and integration

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

BF="./target/debug/bf"
TEST_WORKSPACE=$(mktemp -d)
export BF_WORKSPACE="$TEST_WORKSPACE/.beads"

# Counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Helper functions
pass() {
    echo -e "${GREEN}✓${NC} $1"
    ((TESTS_PASSED++))
}

fail() {
    echo -e "${RED}✗${NC} $1"
    echo -e "${RED}  Error: $2${NC}"
    ((TESTS_FAILED++))
}

info() {
    echo -e "${YELLOW}ℹ${NC} $1"
}

# Set up test workspace
setup_workspace() {
    mkdir -p "$BF_WORKSPACE"
    # No initialization needed - bf creates workspace on first use
}

# Cleanup
cleanup() {
    rm -rf "$TEST_WORKSPACE"
}

# Create a test bead
create_bead() {
    local title="$1"
    local labels="$2"
    local cmd="$BF -w $BF_WORKSPACE create --title \"$title\" --type task --priority 2"
    if [ -n "$labels" ]; then
        cmd="$cmd $labels"
    fi
    local output
    output=$(eval "$cmd" 2>&1)
    if [ $? -eq 0 ]; then
        echo "$output" | head -1 | tr -d '\n'
    else
        echo "ERROR: Failed to create bead" >&2
        return 1
    fi
}

# Close a test bead
close_bead() {
    local bead_id="$1"
    $BF -w "$BF_WORKSPACE" close "$bead_id" --reason "Test cleanup" >/dev/null 2>&1
}

# Run a test
run_test() {
    local test_name="$1"
    ((TESTS_RUN++))
    echo ""
    info "Running: $test_name"
}

# Check if JSON array contains value
json_contains() {
    local json="$1"
    local value="$2"
    echo "$json" | grep -q "\"$value\""
}

# Count items in JSON array
json_count() {
    local json="$1"
    # Count commas + 1 for array length
    local commas
    commas=$(echo "$json" | grep -o ',' | wc -l)
    echo $((commas + 1))
}

trap cleanup EXIT

setup_workspace

echo "=========================================="
echo "Comprehensive Label Tests for bead-forge"
echo "=========================================="
echo "Test workspace: $TEST_WORKSPACE"
echo ""

# =============================================================================
# Basic Label Operations
# =============================================================================

run_test "Label: Add single label"
BEAD_ID=$(create_bead "test single label add" "")
if $BF -w "$BF_WORKSPACE" label add "$BEAD_ID" --label urgent >/dev/null 2>&1; then
    LABELS=$($BF -w "$BF_WORKSPACE" labels "$BEAD_ID" --format json 2>/dev/null)
    if json_contains "$LABELS" "urgent"; then
        pass "test_label_add_single"
    else
        fail "test_label_add_single" "Expected label 'urgent', got: $LABELS"
    fi
else
    fail "test_label_add_single" "Failed to add label"
fi
close_bead "$BEAD_ID"

run_test "Label: Add multiple labels"
BEAD_ID=$(create_bead "test multiple label add" "")
if $BF -w "$BF_WORKSPACE" label add "$BEAD_ID" --label urgent --label backend --label phase-1 >/dev/null 2>&1; then
    LABELS=$($BF -w "$BF_WORKSPACE" labels "$BEAD_ID" --format json 2>/dev/null)
    if json_contains "$LABELS" "urgent" && json_contains "$LABELS" "backend" && json_contains "$LABELS" "phase-1"; then
        pass "test_label_add_multiple"
    else
        fail "test_label_add_multiple" "Expected 3 labels, got: $LABELS"
    fi
else
    fail "test_label_add_multiple" "Failed to add labels"
fi
close_bead "$BEAD_ID"

run_test "Label: Add duplicate labels (idempotent)"
BEAD_ID=$(create_bead "test duplicate label add" "")
if $BF -w "$BF_WORKSPACE" label add "$BEAD_ID" --label urgent --label urgent >/dev/null 2>&1; then
    LABELS=$($BF -w "$BF_WORKSPACE" labels "$BEAD_ID" --format json 2>/dev/null)
    # Count occurrences of "urgent"
    URGENG_COUNT=$(echo "$LABELS" | grep -o "urgent" | wc -l)
    if [ "$URGENG_COUNT" -eq 1 ]; then
        pass "test_label_add_duplicate_idempotent"
    else
        fail "test_label_add_duplicate_idempotent" "Expected 1 instance of 'urgent', got count: $URGENG_COUNT"
    fi
else
    fail "test_label_add_duplicate_idempotent" "Failed to add labels"
fi
close_bead "$BEAD_ID"

run_test "Label: Remove single label"
BEAD_ID=$(create_bead "test single label remove" "--label urgent --label backend --label bug")
if $BF -w "$BF_WORKSPACE" label remove "$BEAD_ID" --label urgent >/dev/null 2>&1; then
    LABELS=$($BF -w "$BF_WORKSPACE" labels "$BEAD_ID" --format json 2>/dev/null)
    if ! json_contains "$LABELS" "urgent" && json_contains "$LABELS" "backend" && json_contains "$LABELS" "bug"; then
        pass "test_label_remove_single"
    else
        fail "test_label_remove_single" "Label 'urgent' not removed, got: $LABELS"
    fi
else
    fail "test_label_remove_single" "Failed to remove label"
fi
close_bead "$BEAD_ID"

run_test "Label: Remove multiple labels"
BEAD_ID=$(create_bead "test multiple label remove" "--label urgent --label backend --label bug --label phase-1")
if $BF -w "$BF_WORKSPACE" label remove "$BEAD_ID" --label urgent --label bug >/dev/null 2>&1; then
    LABELS=$($BF -w "$BF_WORKSPACE" labels "$BEAD_ID" --format json 2>/dev/null)
    if ! json_contains "$LABELS" "urgent" && ! json_contains "$LABELS" "bug" && json_contains "$LABELS" "backend" && json_contains "$LABELS" "phase-1"; then
        pass "test_label_remove_multiple"
    else
        fail "test_label_remove_multiple" "Expected 2 labels after removal, got: $LABELS"
    fi
else
    fail "test_label_remove_multiple" "Failed to remove labels"
fi
close_bead "$BEAD_ID"

run_test "Label: Remove nonexistent label (idempotent)"
BEAD_ID=$(create_bead "test remove nonexistent label" "--label backend")
if $BF -w "$BF_WORKSPACE" label remove "$BEAD_ID" --label nonexistent >/dev/null 2>&1; then
    LABELS=$($BF -w "$BF_WORKSPACE" labels "$BEAD_ID" --format json 2>/dev/null)
    if json_contains "$LABELS" "backend"; then
        pass "test_label_remove_nonexistent_idempotent"
    else
        fail "test_label_remove_nonexistent_idempotent" "Original label 'backend' was removed"
    fi
else
    fail "test_label_remove_nonexistent_idempotent" "Remove should succeed"
fi
close_bead "$BEAD_ID"

run_test "Label: Remove idempotent (remove same label twice)"
BEAD_ID=$(create_bead "test idempotent removal" "--label urgent")
$BF -w "$BF_WORKSPACE" label remove "$BEAD_ID" --label urgent >/dev/null 2>&1
if $BF -w "$BF_WORKSPACE" label remove "$BEAD_ID" --label urgent >/dev/null 2>&1; then
    LABELS=$($BF -w "$BF_WORKSPACE" labels "$BEAD_ID" --format json 2>/dev/null)
    # Empty array is "[]"
    if [ "$LABELS" = "[]" ]; then
        pass "test_label_remove_idempotent"
    else
        fail "test_label_remove_idempotent" "Expected no labels, got: $LABELS"
    fi
else
    fail "test_label_remove_idempotent" "Second removal should succeed"
fi
close_bead "$BEAD_ID"

# =============================================================================
# Label List Operations
# =============================================================================

run_test "Label: List empty bead labels"
BEAD_ID=$(create_bead "test empty label list" "")
LABELS=$($BF -w "$BF_WORKSPACE" labels "$BEAD_ID" --format json 2>/dev/null)
if [ "$LABELS" = "[]" ]; then
    pass "test_label_list_empty_bead"
else
    fail "test_label_list_empty_bead" "Expected empty array, got: $LABELS"
fi
close_bead "$BEAD_ID"

run_test "Label: List all unique labels"
BEAD1=$(create_bead "label list bead 1" "--label urgent --label backend")
BEAD2=$(create_bead "label list bead 2" "--label urgent --label frontend")
OUTPUT=$($BF -w "$BF_WORKSPACE" label list 2>/dev/null)
# Should contain at least 3 unique labels: urgent, backend, frontend
if echo "$OUTPUT" | grep -q "urgent" && echo "$OUTPUT" | grep -q "backend" && echo "$OUTPUT" | grep -q "frontend"; then
    pass "test_label_list_all_unique"
else
    fail "test_label_list_all_unique" "Expected labels urgent, backend, frontend in: $OUTPUT"
fi
close_bead "$BEAD1"
close_bead "$BEAD2"

run_test "Label: List labels for specific bead via 'label list'"
BEAD_ID=$(create_bead "test label list with id" "--label urgent --label backend")
OUTPUT=$($BF -w "$BF_WORKSPACE" label list "$BEAD_ID" 2>/dev/null)
LINE_COUNT=$(echo "$OUTPUT" | grep -c '.' || echo 0)
if [ "$LINE_COUNT" -ge 2 ]; then
    pass "test_label_list_with_bead_id"
else
    fail "test_label_list_with_bead_id" "Expected at least 2 labels, got line count: $LINE_COUNT"
fi
close_bead "$BEAD_ID"

# =============================================================================
# Label Integration
# =============================================================================

run_test "Label: Create bead with labels"
BEAD_ID=$(create_bead "test create with labels" "--label urgent --label backend")
LABELS=$($BF -w "$BF_WORKSPACE" labels "$BEAD_ID" --format json 2>/dev/null)
if json_contains "$LABELS" "urgent" && json_contains "$LABELS" "backend"; then
    pass "test_create_with_labels"
else
    fail "test_create_with_labels" "Expected 2 labels, got: $LABELS"
fi
close_bead "$BEAD_ID"

run_test "Label: Show command includes labels"
BEAD_ID=$(create_bead "test show labels" "--label urgent --label backend")
SHOW_OUTPUT=$($BF -w "$BF_WORKSPACE" show "$BEAD_ID" --format json 2>/dev/null)
if echo "$SHOW_OUTPUT" | grep -q '"labels"'; then
    pass "test_show_includes_labels"
else
    fail "test_show_includes_labels" "Show output should include labels field"
fi
close_bead "$BEAD_ID"

run_test "Label: Search by label"
BEAD1=$(create_bead "search test bead 1" "--label urgent --label backend")
BEAD2=$(create_bead "search test bead 2" "--label frontend")
SEARCH_OUTPUT=$($BF -w "$BF_WORKSPACE" search --label urgent --format json 2>/dev/null)
if echo "$SEARCH_OUTPUT" | grep -q "$BEAD1"; then
    pass "test_search_by_label"
else
    fail "test_search_by_label" "Search should find bead1 with 'urgent' label"
fi
close_bead "$BEAD1"
close_bead "$BEAD2"

run_test "Label: Search by multiple labels (OR logic)"
BEAD1=$(create_bead "multi-label search 1" "--label urgent --label backend")
BEAD2=$(create_bead "multi-label search 2" "--label urgent")
BEAD3=$(create_bead "multi-label search 3" "--label backend")
SEARCH_OUTPUT=$($BF -w "$BF_WORKSPACE" search --label urgent --label backend --format json 2>/dev/null)
# Should find all three beads (OR logic)
FOUND1=$(echo "$SEARCH_OUTPUT" | grep -c "$BEAD1" || echo 0)
FOUND2=$(echo "$SEARCH_OUTPUT" | grep -c "$BEAD2" || echo 0)
FOUND3=$(echo "$SEARCH_OUTPUT" | grep -c "$BEAD3" || echo 0)
if [ "$FOUND1" -ge 1 ] && [ "$FOUND2" -ge 1 ] && [ "$FOUND3" -ge 1 ]; then
    pass "test_search_by_multiple_labels"
else
    fail "test_search_by_multiple_labels" "Should find all 3 beads, counts: $FOUND1, $FOUND2, $FOUND3"
fi
close_bead "$BEAD1"
close_bead "$BEAD2"
close_bead "$BEAD3"

# =============================================================================
# Label Output Formats
# =============================================================================

run_test "Label: Labels shortcut text format"
BEAD_ID=$(create_bead "test labels shortcut text" "--label urgent --label backend")
OUTPUT=$($BF -w "$BF_WORKSPACE" labels "$BEAD_ID" 2>/dev/null)
LINE_COUNT=$(echo "$OUTPUT" | grep -c '.' || echo 0)
if [ "$LINE_COUNT" -ge 2 ]; then
    pass "test_labels_shortcut_text_format"
else
    fail "test_labels_shortcut_text_format" "Expected at least 2 lines, got: $LINE_COUNT"
fi
close_bead "$BEAD_ID"

run_test "Label: Labels shortcut JSON format"
BEAD_ID=$(create_bead "test labels shortcut json" "--label urgent --label backend")
LABELS=$($BF -w "$BF_WORKSPACE" labels "$BEAD_ID" --format json 2>/dev/null)
if json_contains "$LABELS" "urgent" && json_contains "$LABELS" "backend"; then
    pass "test_labels_shortcut_json_format"
else
    fail "test_labels_shortcut_json_format" "Expected 2 labels, got: $LABELS"
fi
close_bead "$BEAD_ID"

# =============================================================================
# Label Persistence
# =============================================================================

run_test "Label: Labels persist through sync"
BEAD_ID=$(create_bead "test label persistence" "")
$BF -w "$BF_WORKSPACE" label add "$BEAD_ID" --label urgent --label backend >/dev/null 2>&1
$BF -w "$BF_WORKSPACE" sync --flush-only >/dev/null 2>&1
LABELS=$($BF -w "$BF_WORKSPACE" labels "$BEAD_ID" --format json 2>/dev/null)
if json_contains "$LABELS" "urgent" && json_contains "$LABELS" "backend"; then
    pass "test_labels_persist_through_sync"
else
    fail "test_labels_persist_through_sync" "Expected 2 labels after sync, got: $LABELS"
fi
close_bead "$BEAD_ID"

# =============================================================================
# Edge Cases
# =============================================================================

run_test "Label: Special characters in labels"
BEAD_ID=$(create_bead "test special char labels" "")
$BF -w "$BF_WORKSPACE" label add "$BEAD_ID" --label "bug:critical" >/dev/null 2>&1
$BF -w "$BF_WORKSPACE" label add "$BEAD_ID" --label "feature/auth" >/dev/null 2>&1
$BF -w "$BF_WORKSPACE" label add "$BEAD_ID" --label "ui-component" >/dev/null 2>&1
LABELS=$($BF -w "$BF_WORKSPACE" labels "$BEAD_ID" --format json 2>/dev/null)
if json_contains "$LABELS" "bug:critical" && json_contains "$LABELS" "feature/auth" && json_contains "$LABELS" "ui-component"; then
    pass "test_label_with_special_characters"
else
    fail "test_label_with_special_characters" "Special char labels not found in: $LABELS"
fi
close_bead "$BEAD_ID"

run_test "Label: Large number of labels"
BEAD_ID=$(create_bead "test many labels" "")
LABEL_ADDED=0
for i in {1..50}; do
    if $BF -w "$BF_WORKSPACE" label add "$BEAD_ID" --label "label-$i" >/dev/null 2>&1; then
        ((LABEL_ADDED++))
    fi
done
if [ "$LABEL_ADDED" -eq 50 ]; then
    pass "test_large_number_of_labels"
else
    fail "test_large_number_of_labels" "Failed to add all 50 labels, added: $LABEL_ADDED"
fi
close_bead "$BEAD_ID"

run_test "Label: Remove all labels from bead"
BEAD_ID=$(create_bead "test remove all labels" "--label urgent")
$BF -w "$BF_WORKSPACE" label remove "$BEAD_ID" --label urgent >/dev/null 2>&1
LABELS=$($BF -w "$BF_WORKSPACE" labels "$BEAD_ID" --format json 2>/dev/null)
if [ "$LABELS" = "[]" ]; then
    pass "test_label_remove_all_labels"
else
    fail "test_label_remove_all_labels" "Expected empty array, got: $LABELS"
fi
close_bead "$BEAD_ID"

run_test "Label: Remove from empty label list"
BEAD_ID=$(create_bead "test remove from empty list" "")
if $BF -w "$BF_WORKSPACE" label remove "$BEAD_ID" --label urgent >/dev/null 2>&1; then
    LABELS=$($BF -w "$BF_WORKSPACE" labels "$BEAD_ID" --format json 2>/dev/null)
    if [ "$LABELS" = "[]" ]; then
        pass "test_label_remove_empty_label_list"
    else
        fail "test_label_remove_empty_label_list" "Expected empty array, got: $LABELS"
    fi
else
    fail "test_label_remove_empty_label_list" "Remove should succeed"
fi
close_bead "$BEAD_ID"

# =============================================================================
# Summary
# =============================================================================

echo ""
echo "=========================================="
echo "Test Summary"
echo "=========================================="
echo "Tests run:    $TESTS_RUN"
echo -e "Tests passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests failed: ${RED}$TESTS_FAILED${NC}"
echo "=========================================="

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
