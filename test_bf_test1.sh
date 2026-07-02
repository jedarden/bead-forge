#!/bin/bash
# Test 1: Basic bead-forge CLI smoke test
# Verifies that the bf CLI can be invoked and shows help

set -e

bf_build_path="./target/debug/bf"

echo "Test 1: Verifying bf CLI exists and is executable..."
if [ ! -f "$bf_build_path" ]; then
    echo "ERROR: bf binary not found at $bf_build_path"
    exit 1
fi

echo "Test 1: Running bf --help..."
$bf_build_path --help &> /tmp/bf_help.txt || true
if [ -s /tmp/bf_help.txt ] && grep -q "bead-forge" /tmp/bf_help.txt; then
    echo "✓ bf --help succeeded"
else
    echo "✗ bf --help failed"
    cat /tmp/bf_help.txt
    exit 1
fi

echo "Test 1: Running bf version..."
$bf_build_path --version &> /tmp/bf_version.txt || true
if [ -s /tmp/bf_version.txt ] && grep -q "bf" /tmp/bf_version.txt; then
    echo "✓ bf --version succeeded"
else
    echo "✗ bf --version failed"
    cat /tmp/bf_version.txt
    exit 1
fi

echo "✓ Test 1 passed: bf CLI is functional"
