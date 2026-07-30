#!/usr/bin/env python3
"""
Validate cargo test trace metadata files against the JSON schema.

Usage:
    python3 scripts/validate-trace.py <trace-directory>
    python3 scripts/validate-trace.py .beads/traces/bf-38dsw0-20260724-151645
    python3 scripts/validate-trace.py --all

Requirements (optional, for full validation):
    pip install jsonschema

Without jsonschema, performs basic JSON validation and required field checking.
"""

import sys
import json
import re
from pathlib import Path

def load_schema():
    """Load the cargo test output trace schema."""
    schema_path = Path(__file__).parent.parent / "docs" / "schemas" / "cargo-test-output-trace.json"
    with open(schema_path, "r") as f:
        return json.load(f)

def validate_basic_fields(metadata):
    """
    Perform basic validation of required fields without jsonschema dependency.
    Returns (is_valid, error_message).
    """
    required_fields = [
        "trace_id",
        "trace_format",
        "bead_id",
        "exit_code",
        "outcome",
        "execution_time",
        "captured_at",
        "test_command",
        "stdout_bytes",
        "stderr_bytes"
    ]

    # Check required fields
    for field in required_fields:
        if field not in metadata:
            return False, f"Missing required field: {field}"

    # Validate trace_format
    if metadata["trace_format"] != "cargo_test_output":
        return False, f"Invalid trace_format: {metadata['trace_format']} (must be 'cargo_test_output')"

    # Validate outcome
    if metadata["outcome"] not in ["success", "failure", "error"]:
        return False, f"Invalid outcome: {metadata['outcome']} (must be 'success', 'failure', or 'error')"

    # Validate trace_id pattern
    if not re.match(r"^bf-[a-z0-9]+(-\d{8}-\d{6}(-\d+)?)?$", metadata["trace_id"]):
        return False, f"Invalid trace_id format: {metadata['trace_id']} (should match bf-{{id}}-{{timestamp}} or bf-{{id}}-{{timestamp}}-{{counter}})"

    # Validate bead_id pattern
    if not re.match(r"^bf-[a-z0-9]+$", metadata["bead_id"]):
        return False, f"Invalid bead_id format: {metadata['bead_id']} (should match bf-{{id}})"

    # Validate execution_time has duration_ms
    if "execution_time" not in metadata or "duration_ms" not in metadata["execution_time"]:
        return False, "execution_time.duration_ms is required"

    # Validate types
    if not isinstance(metadata["exit_code"], int):
        return False, "exit_code must be an integer"

    if not isinstance(metadata["stdout_bytes"], int):
        return False, "stdout_bytes must be an integer"

    if not isinstance(metadata["stderr_bytes"], int):
        return False, "stderr_bytes must be an integer"

    if not isinstance(metadata["execution_time"]["duration_ms"], (int, float)):
        return False, "execution_time.duration_ms must be a number"

    return True, None

def validate_trace_file(metadata_file, schema):
    """
    Validate a single metadata.json file against the schema.
    Returns (is_valid, error_message).
    """
    try:
        # First, validate JSON
        with open(metadata_file, "r") as f:
            metadata = json.load(f)

        # Try full schema validation if jsonschema is available
        try:
            from jsonschema import validate, ValidationError
            validate(instance=metadata, schema=schema)
            return True, None
        except ImportError:
            # Fallback to basic validation
            return validate_basic_fields(metadata)
        except ValidationError as e:
            return False, str(e)

    except json.JSONDecodeError as e:
        return False, f"Invalid JSON: {e}"

def validate_trace_directory(trace_dir, schema):
    """Validate metadata.json in a trace directory."""
    metadata_file = Path(trace_dir) / "metadata.json"

    if not metadata_file.exists():
        return False, f"metadata.json not found in {trace_dir}"

    return validate_trace_file(metadata_file, schema)

def main():
    if len(sys.argv) < 2:
        print("Usage: python3 scripts/validate-trace.py <trace-directory>")
        print("       python3 scripts/validate-trace.py --all")
        print("")
        print("Examples:")
        print("  python3 scripts/validate-trace.py .beads/traces/bf-38dsw0-20260724-151645")
        print("  python3 scripts/validate-trace.py --all")
        print("")
        print("Note: Full JSON Schema validation requires 'pip install jsonschema'")
        print("      Without it, basic required field validation is performed.")
        sys.exit(1)

    # Load schema
    try:
        schema = load_schema()
    except FileNotFoundError as e:
        print(f"Error: Schema file not found: {e}")
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error: Schema file is invalid JSON: {e}")
        sys.exit(1)

    # Validate
    if sys.argv[1] == "--all":
        print("Validating all traces in .beads/traces/...")
        print("")

        traces_dir = Path(".beads/traces")
        if not traces_dir.exists():
            print("Error: .beads/traces/ directory not found")
            sys.exit(1)

        valid_count = 0
        invalid_count = 0

        for trace_dir in sorted(traces_dir.iterdir()):
            if trace_dir.is_dir():
                is_valid, error = validate_trace_directory(trace_dir, schema)
                if is_valid:
                    print(f"✓ Valid: {trace_dir.name}/metadata.json")
                    valid_count += 1
                else:
                    print(f"✗ Invalid: {trace_dir.name}/metadata.json")
                    print(f"  Error: {error}")
                    invalid_count += 1

        print("")
        print(f"Summary: {valid_count} valid, {invalid_count} invalid")

        if invalid_count > 0:
            sys.exit(1)

    else:
        trace_input = sys.argv[1]
        trace_path = Path(trace_input)

        # If a metadata.json file is specified, use its directory
        if trace_path.name == "metadata.json":
            trace_dir = trace_path.parent
        else:
            trace_dir = trace_path

        if not trace_dir.exists():
            print(f"Error: Directory not found: {trace_dir}")
            sys.exit(1)

        metadata_file = trace_dir / "metadata.json"
        print(f"Validating: {metadata_file}")

        is_valid, error = validate_trace_directory(trace_dir, schema)

        if is_valid:
            print("✓ Valid trace metadata")
        else:
            print(f"✗ Invalid trace metadata: {error}")
            sys.exit(1)

if __name__ == "__main__":
    main()