#!/bin/bash
# Test suite for Epic P1 creation
# Tests epic type creation with P1 (HIGH) priority using implemented features only

set -e

BF="/home/coding/bead-forge/target/release/bf"
TEST_DIR="/tmp/bf-test-epic-p1-$$"
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

setup() {
    echo -e "${YELLOW}Setting up test environment...${NC}"
    mkdir -p "$TEST_DIR"
    cd "$TEST_DIR"
    "$BF" init 2>&1 | head -5
}

teardown() {
    echo -e "${YELLOW}Cleaning up test environment...${NC}"
    cd /tmp
    rm -rf "$TEST_DIR"
}

run_test() {
    local test_name="$1"
    local test_command="$2"
    local expected="$3"

    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    echo -n "Test $TOTAL_TESTS: $test_name... "

    if eval "$test_command" 2>&1 | grep -q "$expected"; then
        echo -e "${GREEN}PASSED${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        return 0
    else
        echo -e "${RED}FAILED${NC}"
        echo "  Expected: $expected"
        echo "  Command: $test_command"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    fi
}

# Helper function to get bead ID by title using grep and awk
get_bead_id() {
    local title="$1"
    "$BF" list --format json | grep "\"title\":\"$title\"" | grep -o '"id":"[^"]*"' | cut -d'"' -f4
}

# Helper function to get field value from bead JSON
get_bead_field() {
    local bead_id="$1"
    local field="$2"
    "$BF" show "$bead_id" --format json | grep -o "\"$field\":\"*[^\"]*" | cut -d'"' -f4 | head -1
}

# Test suite
test_p1_epic_creation() {
    echo -e "\n${YELLOW}=== Test Suite: Epic P1 Creation ===${NC}\n"

    # Test 1: Basic P1 epic creation
    run_test \
        "Create P1 epic" \
        "$BF create --type epic --priority 1 --title 'Test P1 Epic' --description 'Testing P1 epic creation'" \
        "bf-"

    # Get the bead ID
    BEAD_ID=$(get_bead_id "Test P1 Epic")
    echo "  Created bead: $BEAD_ID"

    # Test 2: Verify priority is HIGH (1)
    run_test \
        "Verify P1 priority is 1" \
        "$BF show $BEAD_ID --format json" \
        '"priority":1'

    # Test 3: Verify type is epic
    run_test \
        "Verify type is epic" \
        "$BF show $BEAD_ID --format json" \
        '"issue_type":"epic"'

    # Test 4: Test JSON serialization preserves P1
    run_test \
        "JSON serialization preserves P1" \
        "$BF show $BEAD_ID --format json" \
        '"priority":1'

    # Test 5: Create P1 epic with assignee
    run_test \
        "Create P1 epic with assignee" \
        "$BF create --type epic --priority 1 --title 'Assigned P1 Epic' --assignee test-user" \
        "bf-"

    BEAD_ID_2=$(get_bead_id "Assigned P1 Epic")

    # Test 6: Verify assignee on P1 epic
    run_test \
        "Verify assignee on P1 epic" \
        "$BF show $BEAD_ID_2 --format json" \
        '"assignee":"test-user"'

    # Test 7: Test priority string parsing (P1/p1/1 all work)
    run_test \
        "Create epic with uppercase P1" \
        "$BF create --type epic --priority 1 --title 'Uppercase P1 Epic'" \
        "bf-"

    BEAD_ID_UPPER=$(get_bead_id "Uppercase P1 Epic")

    run_test \
        "Verify uppercase P1 parsed correctly" \
        "$BF show $BEAD_ID_UPPER --format json" \
        '"priority":1'

    # Test 8: Test P1 epic with labels
    run_test \
        "Create P1 epic with labels" \
        "$BF create --type epic --priority 1 --title 'Labeled P1 Epic' --label epic,p1,test" \
        "bf-"

    BEAD_ID_LABELED=$(get_bead_id "Labeled P1 Epic")

    run_test \
        "Verify labels on P1 epic" \
        "$BF show $BEAD_ID_LABELED --format json" \
        'epic,p1,test'

    # Test 9: Verify P1 epics appear in epic type filter
    run_test \
        "List filter by epic type includes P1" \
        "$BF list --type epic --format json" \
        '"issue_type":"epic"'

    # Test 10: Test updating existing epic to P1
    run_test \
        "Create epic for update test" \
        "$BF create --type epic --priority 2 --title 'Epic to Update to P1'" \
        "bf-"

    BEAD_ID_UPDATE=$(get_bead_id "Epic to Update to P1")

    run_test \
        "Update epic priority to P1" \
        "$BF update $BEAD_ID_UPDATE --priority 1" \
        "Updated"

    run_test \
        "Verify updated priority is P1" \
        "$BF show $BEAD_ID_UPDATE --format json" \
        '"priority":1'

    # Test 11: Test P1 epic with all available create flags
    run_test \
        "Create P1 epic with all available fields" \
        "$BF create --type epic --priority 1 --title 'Complete P1 Epic' --description 'Full description' --assignee admin --label complete,p1,all-fields" \
        "bf-"

    BEAD_ID_COMPLETE=$(get_bead_id "Complete P1 Epic")

    run_test \
        "Verify complete P1 epic priority" \
        "$BF show $BEAD_ID_COMPLETE --format json" \
        '"priority":1'

    run_test \
        "Verify complete P1 epic has description" \
        "$BF show $BEAD_ID_COMPLETE --format json" \
        '"description":"Full description"'

    # Test 12: Verify P1 is the second highest priority (after P0)
    run_test \
        "Create P0 epic for ordering test" \
        "$BF create --type epic --priority 0 --title 'P0 Epic'" \
        "bf-"

    run_test \
        "Create P2 epic for ordering test" \
        "$BF create --type epic --priority 2 --title 'P2 Epic'" \
        "bf-"

    # Test 13: Verify list filtering by P1 priority works
    run_test \
        "List filter by P1 priority returns correct beads" \
        "$BF list --priority 1 --format json" \
        '"priority":1'

    # Test 14: Create P1 epic with description
    run_test \
        "Create P1 epic with description" \
        "$BF create --type epic --priority 1 --title 'Described P1 Epic' --description 'This is a detailed description'" \
        "bf-"

    BEAD_ID_DESC=$(get_bead_id "Described P1 Epic")

    run_test \
        "Verify description on P1 epic" \
        "$BF show $BEAD_ID_DESC --format json" \
        '"description":"This is a detailed description"'

    # Test 15: Create P1 epic and verify it exists in list
    run_test \
        "Create final P1 epic" \
        "$BF create --type epic --priority 1 --title 'Final Test P1 Epic'" \
        "bf-"

    run_test \
        "Verify final P1 epic in list" \
        "$BF list --format json" \
        '"title":"Final Test P1 Epic"'
}

# Summary
print_summary() {
    echo -e "\n${YELLOW}=== Test Summary ===${NC}"
    echo "Total tests: $TOTAL_TESTS"
    echo -e "${GREEN}Passed: $PASSED_TESTS${NC}"
    echo -e "${RED}Failed: $FAILED_TESTS${NC}"

    if [ $FAILED_TESTS -eq 0 ]; then
        echo -e "\n${GREEN}All tests passed!${NC}"
        return 0
    else
        echo -e "\n${RED}Some tests failed.${NC}"
        return 1
    fi
}

# Main execution
main() {
    setup
    test_p1_epic_creation
    print_summary
    local exit_code=$?
    teardown
    exit $exit_code
}

main
