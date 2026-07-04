#!/bin/bash
# Test bf-3cd8: Basic bead operations test
# This test verifies core bead create/update/close operations

set -e

bf_build_path="./target/debug/bf"
test_title="Test bead B operations"
test_bead_id=""

echo "=== Test bf-3cd8: Basic bead operations ==="

# Test 1: Verify bf binary exists
echo "Test 1: Verifying bf CLI exists..."
if [ ! -f "$bf_build_path" ]; then
    echo "ERROR: bf binary not found at $bf_build_path"
    exit 1
fi
echo "✓ bf binary found"

# Test 2: Build the project
echo "Test 2: Building bead-forge..."
cargo build --quiet 2>&1 | grep -E "^error" && exit 1 || true
echo "✓ Build successful"

# Test 3: Create a test bead
echo "Test 3: Creating test bead..."
# Create command outputs text, not JSON, so we parse the bead ID from output
create_output=$($bf_build_path create --title "$test_title" --type task --priority 2 2>/dev/null)
test_bead_id=$(echo "$create_output" | grep -oE 'bf-[a-z0-9]+')
if [ -z "$test_bead_id" ]; then
    echo "✗ Failed to create test bead"
    echo "Output was: $create_output"
    exit 1
fi
echo "✓ Created test bead: $test_bead_id"

# Test 4: Show the bead
echo "Test 4: Showing test bead..."
$bf_build_path show "$test_bead_id" --json > /tmp/bf_3cd8_show.json 2>/dev/null || true
# bf show --json returns an array, so we access the first element's id
if [ -s /tmp/bf_3cd8_show.json ] && jq -e '.[0].id' /tmp/bf_3cd8_show.json > /dev/null 2>&1; then
    echo "✓ Show command succeeded"
else
    echo "✗ Show command failed"
    cat /tmp/bf_3cd8_show.json
    exit 1
fi

# Test 5: Update the bead description
echo "Test 5: Updating bead description..."
$bf_build_path update "$test_bead_id" --description "Test description updated" > /dev/null 2>&1
# Access first element's description field
if $bf_build_path show "$test_bead_id" --json | jq -e '.[0].description == "Test description updated"' > /dev/null 2>&1; then
    echo "✓ Update succeeded"
else
    echo "✗ Update failed"
    exit 1
fi

# Test 6: List beads (should include our test bead)
echo "Test 6: Listing beads..."
# list --json outputs JSONL (one JSON per line), so we grep for the ID
if $bf_build_path list --json | grep -q "\"id\":\"$test_bead_id\""; then
    echo "✓ List includes test bead"
else
    echo "✗ List does not include test bead"
    exit 1
fi

# Test 7: Close the test bead
echo "Test 7: Closing test bead..."
$bf_build_path close "$test_bead_id" --reason "Test completed" > /dev/null 2>&1
# Access first element's status field
if $bf_build_path show "$test_bead_id" --json | jq -e '.[0].status == "closed"' > /dev/null 2>&1; then
    echo "✓ Close succeeded"
else
    echo "✗ Close failed"
    exit 1
fi

# Test 8: Count beads
echo "Test 8: Counting beads..."
bead_count=$($bf_build_path count 2>/dev/null | tr -d '\n')
if [ -n "$bead_count" ]; then
    echo "✓ Count returned: $bead_count beads"
else
    echo "✗ Count failed"
    exit 1
fi

echo "=== All tests for bf-3cd8 passed ==="
echo "Test bead $test_bead_id was created, tested, and closed during this test"
