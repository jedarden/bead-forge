#!/usr/bin/env bash
# General cargo test output capture with timestamped logs
# Usage: ./scripts/cargo-test-capture.sh [cargo_test_args...]
# Example: ./scripts/cargo-test-capture.sh --lib

set -euo pipefail

TRACE_DIR=".beads/traces"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
TIMESTAMPED_LOG="cargo-test-${TIMESTAMP}.log"
LATEST_LINK="cargo-test-latest.log"
FULL_LOG="cargo-test-full.log"

# Ensure trace directory exists
mkdir -p "${TRACE_DIR}"

# Create timestamped log file path
TIMESTAMPED_PATH="${TRACE_DIR}/${TIMESTAMPED_LOG}"

# Update latest symlink (remove old if exists)
cd "${TRACE_DIR}"
rm -f "${LATEST_LINK}"
ln -s "${TIMESTAMPED_LOG}" "${LATEST_LINK}"
cd - > /dev/null

# Run cargo test with tee to capture both stdout and file
# Usage: scripts/cargo-test-capture.sh [cargo_test_args...]
echo "Capturing cargo test output to: ${TIMESTAMPED_PATH}"
echo "Symlink: ${TRACE_DIR}/${LATEST_LINK} -> ${TIMESTAMPED_LOG}"
echo ""

if [ $# -eq 0 ]; then
    cargo test 2>&1 | tee "${TIMESTAMPED_PATH}"
    TEST_EXIT_CODE=${PIPESTATUS[0]}
else
    cargo test "$@" 2>&1 | tee "${TIMESTAMPED_PATH}"
    TEST_EXIT_CODE=${PIPESTATUS[0]}
fi

# Also append to full log for historical reference
cat "${TIMESTAMPED_PATH}" >> "${TRACE_DIR}/${FULL_LOG}"

echo ""
echo "✓ Test capture complete. Log file: ${TIMESTAMPED_PATH}"
echo "✓ Symlink updated: ${TRACE_DIR}/${LATEST_LINK}"

# Exit with the actual test exit code
exit $TEST_EXIT_CODE
