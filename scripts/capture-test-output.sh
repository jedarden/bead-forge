#!/bin/bash
# Test output capture script for bead-forge
# Captures test output with timestamps to .beads/traces/

set -e

# Parse arguments
BEAD_ID="${1:-bf-test}"
TEST_NAME="${2:-all_tests}"
shift 2

# Remaining arguments are the test command to run
TEST_CMD=("$@")

# Default to running all tests if no command provided
if [ ${#TEST_CMD[@]} -eq 0 ]; then
    TEST_CMD=(cargo test -- -q)
fi

# Create trace directory
TRACE_DIR=".beads/traces/${BEAD_ID}"
mkdir -p "$TRACE_DIR"

# Initialize metadata
METADATA_FILE="${TRACE_DIR}/metadata.json"
STDOUT_FILE="${TRACE_DIR}/stdout.txt"
STDERR_FILE="${TRACE_DIR}/stderr.txt"

# Record start time with high precision
START_TIME=$(date -u +"%Y-%m-%dT%H:%M:%S.%NZ")
START_NS=$(date +%s%N 2>/dev/null || echo "$(date +%s)000000000")

echo "Capturing test output for ${TEST_NAME}..."
echo "Trace directory: ${TRACE_DIR}"
echo "Test command: ${TEST_CMD[*]}"
echo ""

# Run tests and capture output with timing
# Use script command to capture with precise timing if available
if command -v script &> /dev/null; then
    # Linux with script command
    script -q -c "${TEST_CMD[*]}" /dev/null > "${STDOUT_FILE}" 2> "${STDERR_FILE}" || true
    EXIT_CODE=${?}
else
    # Fallback without script
    "${TEST_CMD[@]}" > "${STDOUT_FILE}" 2> "${STDERR_FILE}" || true
    EXIT_CODE=${?}
fi

# Record end time with high precision
END_TIME=$(date -u +"%Y-%m-%dT%H:%M:%S.%NZ")
END_NS=$(date +%s%N 2>/dev/null || echo "$(date +%s)000000000")

# Calculate duration in milliseconds
DURATION_MS=$(( (END_NS - START_NS) / 1000000 ))

# Determine outcome
OUTCOME="success"
if [ $EXIT_CODE -ne 0 ]; then
    OUTCOME="failure"
fi

# Get file sizes
STDOUT_SIZE=$(stat -c%s "${STDOUT_FILE}" 2>/dev/null || stat -f%z "${STDOUT_FILE}" 2>/dev/null || echo "0")
STDERR_SIZE=$(stat -c%s "${STDERR_FILE}" 2>/dev/null || stat -f%z "${STDERR_FILE}" 2>/dev/null || echo "0")

# Create metadata
cat > "${METADATA_FILE}" <<EOF
{
  "bead_id": "${BEAD_ID}",
  "test_name": "${TEST_NAME}",
  "exit_code": ${EXIT_CODE},
  "outcome": "${OUTCOME}",
  "duration_ms": ${DURATION_MS},
  "captured_at": "${END_TIME}",
  "trace_format": "test_output",
  "test_command": "${TEST_CMD[*]}",
  "stdout_bytes": ${STDOUT_SIZE},
  "stderr_bytes": ${STDERR_SIZE}
}
EOF

# Print summary
echo ""
echo "✓ Test output captured to:"
echo "  ${TRACE_DIR}/"
echo ""
echo "Results:"
echo "  Exit code: ${EXIT_CODE}"
echo "  Outcome: ${OUTCOME}"
echo "  Duration: ${DURATION_MS}ms"
echo ""
echo "Files:"
echo "  metadata.json: ${METADATA_FILE}"
echo "  stdout.txt: ${STDOUT_FILE} (${STDOUT_SIZE} bytes)"
echo "  stderr.txt: ${STDERR_FILE} (${STDERR_SIZE} bytes)"

# Exit with test exit code
exit $EXIT_CODE
