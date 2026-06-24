#!/bin/bash
# Test script to verify bf ready --limit 0 behavior

set -e

# Use the local bf binary
BF_BIN="$(dirname "$0")/target/debug/bf"

# Create a temporary workspace
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

# Initialize a workspace
"$BF_BIN" init --prefix test

# Create 15 test beads
for i in {1..15}; do
    "$BF_BIN" create --title "Test bead $i" --type task > /dev/null
done

echo "Created 15 beads"
echo ""

# Test with limit=0 (should return all)
echo "Testing: bf ready --limit 0"
COUNT=$("$BF_BIN" ready --limit 0 | wc -l)
echo "Returned $COUNT beads"
if [ "$COUNT" -eq 15 ]; then
    echo "✓ PASS: limit=0 returned all beads"
else
    echo "✗ FAIL: limit=0 returned $COUNT beads, expected 15"
fi
echo ""

# Test with limit=5
echo "Testing: bf ready --limit 5"
COUNT=$("$BF_BIN" ready --limit 5 | wc -l)
echo "Returned $COUNT beads"
if [ "$COUNT" -eq 5 ]; then
    echo "✓ PASS: limit=5 returned 5 beads"
else
    echo "✗ FAIL: limit=5 returned $COUNT beads, expected 5"
fi
echo ""

# Test with default limit (should be 10)
echo "Testing: bf ready (default limit)"
COUNT=$("$BF_BIN" ready | wc -l)
echo "Returned $COUNT beads"
if [ "$COUNT" -eq 10 ]; then
    echo "✓ PASS: default limit returned 10 beads"
else
    echo "✗ FAIL: default limit returned $COUNT beads, expected 10"
fi
echo ""

# Cleanup
cd /
rm -rf "$TEMP_DIR"

echo "Test complete"
