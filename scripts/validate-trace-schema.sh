#!/usr/bin/env bash
# Validate cargo test trace metadata files against the JSON schema
# Usage: scripts/validate-trace-schema.sh <trace-directory>
# Example: scripts/validate-trace-schema.sh .beads/traces/bf-38dsw0-20260724-151645

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SCHEMA_FILE="${WORKSPACE_ROOT}/docs/schemas/cargo-test-output-trace.json"

if [ ! -f "$SCHEMA_FILE" ]; then
    echo "Error: Schema file not found at $SCHEMA_FILE"
    exit 1
fi

# Check if ajv-cli is installed
if ! command -v ajv &> /dev/null; then
    echo "Error: ajv-cli is not installed. Install it with:"
    echo "  npm install -g ajv-cli"
    exit 1
fi

# Function to validate a single trace directory
validate_trace() {
    local trace_dir="$1"
    local metadata_file="${trace_dir}/metadata.json"

    if [ ! -f "$metadata_file" ]; then
        echo "Error: metadata.json not found in $trace_dir"
        return 1
    fi

    echo "Validating: $metadata_file"

    if ajv validate -m coq --spec="$SCHEMA_FILE" "$metadata_file" 2>&1; then
        echo "✓ Valid: $metadata_file"
        return 0
    else
        echo "✗ Invalid: $metadata_file"
        return 1
    fi
}

# Main logic
if [ -z "$1" ]; then
    echo "Usage: $0 <trace-directory>"
    echo ""
    echo "Examples:"
    echo "  $0 .beads/traces/bf-38dsw0-20260724-151645"
    echo "  $0 .beads/traces/bf-38dsw0-20260724-151645/metadata.json"
    echo "  $0 --all   # Validate all traces in .beads/traces"
    echo ""
    exit 1
fi

if [ "$1" = "--all" ]; then
    echo "Validating all traces in .beads/traces/..."
    echo ""

    valid_count=0
    invalid_count=0

    for trace_dir in .beads/traces/*/; do
        if [ -d "$trace_dir" ]; then
            if validate_trace "$trace_dir"; then
                ((valid_count++))
            else
                ((invalid_count++))
            fi
        fi
    done

    echo ""
    echo "Summary: $valid_count valid, $invalid_count invalid"

    if [ "$invalid_count" -gt 0 ]; then
        exit 1
    fi
else
    # Validate a single trace
    trace_input="$1"

    # If a metadata.json file is specified, use its directory
    if [[ "$trace_input" =~ metadata\.json$ ]]; then
        trace_dir="$(dirname "$trace_input")"
    else
        trace_dir="$trace_input"
    fi

    validate_trace "$trace_dir"
fi