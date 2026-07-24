#!/usr/bin/env bash
# Test output capture script for bead-forge
# Captures cargo test output with timestamps to .beads/traces/

set -euo pipefail

# Usage function
usage() {
    echo "Usage: $0 <bead-id> [-- <cargo-test-args>]"
    echo "Example: $0 bf-3vhegr -- --test test_show_command"
    echo "Example: $0 bf-3vhegr -- test_show_basic_text_format"
    echo ""
    echo "Arguments:"
    echo "  bead-id          The bead ID to associate with this test run"
    echo "  cargo-test-args  Arguments to pass to cargo test (after --)"
    exit 1
}

# Check arguments
if [ $# -lt 1 ]; then
    usage
fi

BEAD_ID="$1"
shift

# Parse cargo test arguments
CARGO_ARGS=()
while [ $# -gt 0 ]; do
    case "$1" in
        --)
            shift
            CARGO_ARGS=("$@")
            break
            ;;
        *)
            CARGO_ARGS+=("$1")
            shift
            ;;
    esac
done

# Set up trace directory
TRACE_DIR=".beads/traces/${BEAD_ID}"
mkdir -p "${TRACE_DIR}"

# Metadata file
METADATA_FILE="${TRACE_DIR}/metadata.json"
STDOUT_FILE="${TRACE_DIR}/stdout.txt"
STDERR_FILE="${TRACE_DIR}/stderr.txt"
TIMESTAMPS_FILE="${TRACE_DIR}/output_with_timestamps.txt"

# Start time
START_TIME=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
START_EPOCH=$(date +%s)

echo "Capturing test output for bead: ${BEAD_ID}"
echo "Trace directory: ${TRACE_DIR}"
echo "Cargo test args: ${CARGO_ARGS:+${CARGO_ARGS[*]}}"
echo ""

# Run cargo test with timestamped output
# Using ts command from moreutils for precise timestamps
if command -v ts &> /dev/null; then
    # ts is available - use it for precise timestamps
    cargo test "${CARGO_ARGS[@]}" 2> >(ts '[%Y-%m-%d %H:%M:%S]' > "${STDERR_FILE}") | ts '[%Y-%m-%d %H:%M:%S]' > "${STDOUT_FILE}"
    EXIT_CODE=${PIPESTATUS[0]}
else
    # Fallback: prepend timestamps manually
    cargo test "${CARGO_ARGS[@]}" 2> >(
        while IFS= read -r line; do
            echo "[$(date '+%Y-%m-%d %H:%M:%S')] ${line}"
        done > "${STDERR_FILE}"
    ) | while IFS= read -r line; do
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] ${line}"
    done > "${STDOUT_FILE}"
    EXIT_CODE=${PIPESTATUS[0]}
fi

# End time
END_TIME=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
END_EPOCH=$(date +%s)
DURATION_MS=$(( (END_EPOCH - START_EPOCH) * 1000 ))

# Determine outcome
if [ ${EXIT_CODE} -eq 0 ]; then
    OUTCOME="success"
else
    OUTCOME="failure"
fi

# Create metadata.json
cat > "${METADATA_FILE}" << EOF
{
  "bead_id": "${BEAD_ID}",
  "test_type": "cargo_test",
  "exit_code": ${EXIT_CODE},
  "outcome": "${OUTCOME}",
  "duration_ms": ${DURATION_MS},
  "start_time": "${START_TIME}",
  "end_time": "${END_TIME}",
  "cargo_args": [$(printf '"%s",' "${CARGO_ARGS[@]}" | sed 's/,$//')],
  "captured_at": "${END_TIME}",
  "trace_format": "test_output_v1"
}
EOF

# Create combined output with timestamps
cat > "${TIMESTAMPS_FILE}" << EOF
# Test Output for Bead: ${BEAD_ID}
# Start: ${START_TIME}
# End: ${END_TIME}
# Duration: ${DURATION_MS}ms
# Exit Code: ${EXIT_CODE}
# Outcome: ${OUTCOME}
#
# === STDOUT ===
EOF
cat "${STDOUT_FILE}" >> "${TIMESTAMPS_FILE}"
echo "" >> "${TIMESTAMPS_FILE}"
echo "# === STDERR ===" >> "${TIMESTAMPS_FILE}"
cat "${STDERR_FILE}" >> "${TIMESTAMPS_FILE}"

# Print summary
echo ""
echo "=== Test Capture Summary ==="
echo "Bead ID: ${BEAD_ID}"
echo "Outcome: ${OUTCOME}"
echo "Exit Code: ${EXIT_CODE}"
echo "Duration: ${DURATION_MS}ms"
echo "Trace files created:"
echo "  - ${METADATA_FILE}"
echo "  - ${STDOUT_FILE}"
echo "  - ${STDERR_FILE}"
echo "  - ${TIMESTAMPS_FILE}"
echo ""

# Show a preview of the output
echo "=== Output Preview (first 20 lines) ==="
head -20 "${TIMESTAMPS_FILE}"
echo ""

exit ${EXIT_CODE}
