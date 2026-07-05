#!/bin/bash
# Test script for bead B operations
# Tests comprehensive operations on beads with "B" in their names

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BF_CMD="$SCRIPT_DIR/target/debug/bf"
TEST_DIR="/tmp/bead_b_test_$$"
mkdir -p "$TEST_DIR"

echo "=== Bead B Operations Test Suite ==="
echo "Test directory: $TEST_DIR"
echo ""

# Create test workspace
cd "$TEST_DIR"
mkdir -p .beads

# Test 1: Create bead B
echo "Test 1: Create bead with 'B' in title"
BEAD_ID=$($BF_CMD create --title "Test Bead B Operations" --description "Testing bead operations for beads with B in name" --type task --label test --label verification)
echo "Created bead: $BEAD_ID"
echo ""

# Test 2: Show bead B
echo "Test 2: Show bead details"
$BF_CMD show "$BEAD_ID"
echo ""

# Test 3: Update description
echo "Test 3: Update bead description"
$BF_CMD update "$BEAD_ID" --description "Updated description for Test Bead B operations testing"
echo "Description updated"
echo ""

# Test 4: Add label
echo "Test 4: Add label to bead"
$BF_CMD label add "$BEAD_ID" --label bead-b-test
echo "Label added"
echo ""

# Test 5: Update status
echo "Test 5: Update bead status to in_progress"
$BF_CMD update "$BEAD_ID" --status in_progress
echo "Status updated"
echo ""

# Test 6: Update priority
echo "Test 6: Update bead priority to P1"
$BF_CMD update "$BEAD_ID" --priority 1
echo "Priority updated"
echo ""

# Test 7: Show in JSON format
echo "Test 7: Show bead in JSON format"
$BF_CMD show "$BEAD_ID" --format json
echo ""

# Test 8: Search for beads with B
echo "Test 8: Search for beads with 'Bead B' in title"
$BF_CMD search "Bead B" | head -5
echo ""

# Test 9: List with filters
echo "Test 9: List beads with filters (status=open, type=task)"
cd "$TEST_DIR"  # Ensure we're in test workspace
$BF_CMD list --status open --type task | head -5
echo ""

# Test 10: Close bead
echo "Test 10: Close bead"
$BF_CMD close "$BEAD_ID" --reason "Test bead B operations completed successfully"
echo "Bead closed"
echo ""

# Test 11: Verify closed state
echo "Test 11: Verify bead is closed"
$BF_CMD show "$BEAD_ID" | grep -q "Status: closed" && echo "✓ Bead is closed" || echo "✗ Bead close failed"
echo ""

# Cleanup
cd /
rm -rf "$TEST_DIR"

echo "=== All tests completed successfully ==="
