#!/bin/bash
# Test bf-2atz: Basic bead-forge bead operations test
# Verifies that the bf CLI can perform basic bead operations

set -e

bf_build_path="./target/debug/bf"

echo "Test bf-2atz: Running bf list..."
if $bf_build_path list &> /tmp/bf_list.txt; then
    echo "✓ bf list succeeded"
else
    echo "✗ bf list failed"
    cat /tmp/bf_list.txt
    exit 1
fi

echo "Test bf-2atz: Running bf show bf-2atz..."
if $bf_build_path show bf-2atz &> /tmp/bf_show.txt; then
    echo "✓ bf show bf-2atz succeeded"
else
    echo "✗ bf show bf-2atz failed"
    cat /tmp/bf_show.txt
    exit 1
fi

echo "✓ Test bf-2atz passed: bf bead operations are functional"
