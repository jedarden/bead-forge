#!/bin/bash
# Test bf-13yz: Basic bead-forge functionality test
# This test verifies the core CLI functionality of bead-forge

set -e

bf_build_path="./target/debug/bf"

echo "=== Test bf-13yz: Basic bead-forge functionality ==="

# Test 1: Binary exists
echo "Test 1: Verifying bf CLI exists and is executable..."
if [ ! -f "$bf_build_path" ]; then
    echo "ERROR: bf binary not found at $bf_build_path"
    exit 1
fi
echo "✓ bf binary found"

# Test 2: Help command
echo "Test 2: Running bf --help..."
$bf_build_path --help &> /tmp/bf_13yz_help.txt || true
if [ -s /tmp/bf_13yz_help.txt ] && grep -q "bead-forge" /tmp/bf_13yz_help.txt; then
    echo "✓ bf --help succeeded"
else
    echo "✗ bf --help failed"
    cat /tmp/bf_13yz_help.txt
    exit 1
fi

# Test 3: Version command
echo "Test 3: Running bf --version..."
$bf_build_path --version &> /tmp/bf_13yz_version.txt || true
if [ -s /tmp/bf_13yz_version.txt ] && grep -q "bf" /tmp/bf_13yz_version.txt; then
    echo "✓ bf --version succeeded"
else
    echo "✗ bf --version failed"
    cat /tmp/bf_13yz_version.txt
    exit 1
fi

# Test 4: Check that bead system is initialized
echo "Test 4: Checking bead system initialization..."
if [ -d ".beads" ] && [ -f ".beads/beads.db" ]; then
    echo "✓ Bead system is initialized"
else
    echo "✗ Bead system not properly initialized"
    exit 1
fi

echo "=== All tests for bf-13yz passed ==="
