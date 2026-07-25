#!/bin/bash
# Trace output capture helper for bead-forge cargo tests
# Usage: ./capture-test.sh <test-id> <cargo-command>

set -e

if [ $# -lt 2 ]; then
    echo "Usage: $0 <test-id> <cargo-command>" >&2
    echo "Example: $0 bf-17jqtq-test cargo test --lib" >&2
    exit 1
fi

TEST_ID="$1"
shift
CARGO_CMD="$@"

# Generate timestamp and create trace directory
TIMESTAMP=$(date +"%Y%m%d-%H%M%S")
TRACE_DIR="/home/coding/bead-forge/.beads/traces/${TEST_ID}-${TIMESTAMP}"
mkdir -p "$TRACE_DIR"

# Create metadata file
METADATA_FILE="$TRACE_DIR/metadata.json"
cat > "$METADATA_FILE" <<EOF
{
  "test_id": "${TEST_ID}",
  "timestamp": "$(date -Iseconds)",
  "command": "cargo ${CARGO_CMD}",
  "workspace": "/home/coding/bead-forge"
}
EOF

# Run cargo command and capture output
echo "Capturing output to: $TRACE_DIR"
cargo ${CARGO_CMD} > "$TRACE_DIR/stdout.txt" 2> "$TRACE_DIR/stderr.txt" || true

# Update metadata with exit status
EXIT_CODE=$?
cat > "$METADATA_FILE" <<EOF
{
  "test_id": "${TEST_ID}",
  "timestamp": "$(date -Iseconds)",
  "command": "cargo ${CARGO_CMD}",
  "workspace": "/home/coding/bead-forge",
  "exit_code": ${EXIT_CODE}
}
EOF

echo "Trace capture complete: $TRACE_DIR"
echo "  stdout: $TRACE_DIR/stdout.txt"
echo "  stderr: $TRACE_DIR/stderr.txt"
echo "  metadata: $TRACE_DIR/metadata.json"
