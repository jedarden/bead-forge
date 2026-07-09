#!/bin/bash
# Test to reproduce the bug: doctor --repair import -> count_unflushed==imported though drift==0

set -e

# Create a temporary workspace
TEMP_DIR=$(mktemp -d)
echo "Testing in $TEMP_DIR"
cd "$TEMP_DIR"

# Initialize workspace
mkdir -p .beads
cat > .beads/metadata.json <<'EOF'
{"database": "beads.db", "jsonl_export": "issues.jsonl"}
