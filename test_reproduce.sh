#!/bin/bash
set -e

# Create a temporary workspace
TEST_DIR="/tmp/test_bf_2hqt_manual"
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

echo "=== Initializing workspace ==="
/home/coding/bead-forge/target/release/bf init

echo "=== Creating initial bead ==="
/home/coding/bead-forge/target/release/bf create "bf-test" "Test Bead" --type task

echo "=== Flushing to JSONL ==="
/home/coding/bead-forge/target/release/bf sync --flush-only

echo "=== Checking unflushed count (should be 0) ==="
/home/coding/bead-forge/target/release/bf doctor

echo "=== Running doctor --repair ==="
/home/coding/bead-forge/target/release/bf doctor --repair --force

echo "=== Checking unflushed count after repair (should be 0) ==="
/home/coding/bead-forge/target/release/bf doctor

echo ""
echo "=== Testing sync import scenario ==="
TEST_DIR2="/tmp/test_bf_2hqt_import"
rm -rf "$TEST_DIR2"
mkdir -p "$TEST_DIR2"
cd "$TEST_DIR2"

/home/coding/bead-forge/target/release/bf init
/home/coding/bead-forge/target/release/bf create "bf-test2" "Test Bead 2" --type task
/home/coding/bead-forge/target/release/bf sync --flush-only

echo "=== Running sync import ==="
/home/coding/bead-forge/target/release/bf sync --import

echo "=== Checking unflushed count after import (should be 0) ==="
/home/coding/bead-forge/target/release/bf doctor
