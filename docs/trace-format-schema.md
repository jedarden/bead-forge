# Cargo Test Output Trace File Format Specification

## Overview

This specification defines the JSON schema and file structure for capturing cargo test execution output in the bead-forge trace system. The format is designed to be compatible with the existing bead-forge trace infrastructure while providing comprehensive test execution metadata.

## File Structure

Each test execution creates a dedicated directory under `.beads/traces/{trace_id}/` containing three files:

```
.beads/traces/{trace_id}/
├── metadata.json       # Execution metadata and timing information
├── stdout.txt          # Standard output from test execution
└── stderr.txt          # Standard error from test execution
```

### Directory Naming Convention

- **Format:** `{trace_id}` where `trace_id` follows the pattern `bf-{bead_id}-{timestamp}` or `bf-{bead_id}-{timestamp}-{counter}`
- **Example:** `bf-38dsw0-20260724-151645-1`

## JSON Schema: metadata.json

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Cargo Test Output Trace Metadata",
  "type": "object",
  "required": [
    "trace_id",
    "trace_format",
    "bead_id",
    "exit_code",
    "outcome",
    "duration_ms",
    "captured_at",
    "test_command",
    "stdout_bytes",
    "stderr_bytes"
  ],
  "properties": {
    "trace_id": {
      "type": "string",
      "description": "Unique identifier for this trace execution",
      "pattern": "^bf-[a-z0-9]+(-\\d{8}-\\d{6}(-\\d+)?)?$",
      "examples": ["bf-38dsw0-20260724-151645", "bf-38dsw0-20260724-151645-1"]
    },
    "trace_format": {
      "type": "string",
      "enum": ["cargo_test_output"],
      "description": "Format identifier for cargo test output traces"
    },
    "bead_id": {
      "type": "string",
      "description": "Bead ID that this trace is associated with",
      "pattern": "^bf-[a-z0-9]+$",
      "examples": ["bf-38dsw0"]
    },
    "test_name": {
      "type": "string",
      "description": "Descriptive name for the test run",
      "examples": ["all_tests", "verify_cli_commands", "test_show_basic"]
    },
    "exit_code": {
      "type": "integer",
      "description": "Process exit code (0 = success, non-zero = failure)",
      "minimum": 0,
      "examples": [0, 1, 101]
    },
    "outcome": {
      "type": "string",
      "enum": ["success", "failure", "error"],
      "description": "High-level outcome of the test execution"
    },
    "execution_time": {
      "type": "object",
      "description": "Detailed timing information for test execution",
      "required": ["start_time", "end_time", "duration_ms"],
      "properties": {
        "start_time": {
          "type": "string",
          "format": "date-time",
          "description": "ISO 8601 timestamp when test execution started",
          "examples": ["2026-07-24T15:16:45.123456789Z"]
        },
        "end_time": {
          "type": "string",
          "format": "date-time",
          "description": "ISO 8601 timestamp when test execution ended",
          "examples": ["2026-07-24T15:16:47.234567890Z"]
        },
        "duration_ms": {
          "type": "number",
          "description": "Execution duration in milliseconds (including sub-millisecond precision)",
          "minimum": 0,
          "examples": [2111, 157, 5423]
        },
        "compilation_duration_ms": {
          "type": "number",
          "description": "Time spent compiling tests (if available)",
          "minimum": 0,
          "examples": [1500, 0]
        },
        "test_execution_duration_ms": {
          "type": "number",
          "description": "Time spent running tests (excluding compilation)",
          "minimum": 0,
          "examples": [611, 157]
        }
      }
    },
    "captured_at": {
      "type": "string",
      "format": "date-time",
      "description": "ISO 8601 timestamp when the trace was captured/written",
      "examples": ["2026-07-24T15:16:47.234567890Z"]
    },
    "test_command": {
      "type": "string",
      "description": "Full command that was executed",
      "examples": ["cargo test -- -q", "cargo test test_specific_function --test test_module"]
    },
    "test_arguments": {
      "type": "array",
      "description": "Individual arguments passed to the test command",
      "items": {
        "type": "string"
      },
      "examples": [["test", "--", "-q"]]
    },
    "stdout_bytes": {
      "type": "integer",
      "description": "Size of stdout output in bytes",
      "minimum": 0,
      "examples": [177, 5420, 0]
    },
    "stderr_bytes": {
      "type": "integer",
      "description": "Size of stderr output in bytes",
      "minimum": 0,
      "examples": [0, 256]
    },
    "cargo_metadata": {
      "type": "object",
      "description": "Cargo-specific information",
      "properties": {
        "cargo_version": {
          "type": "string",
          "description": "Version of cargo used",
          "examples": ["cargo 1.75.0"]
        },
        "rustc_version": {
          "type": "string",
          "description": "Version of rustc used",
          "examples": ["rustc 1.75.0"]
        },
        "target_triple": {
          "type": "string",
          "description": "Target triple for compilation",
          "examples": ["x86_64-unknown-linux-gnu"]
        },
        "profile": {
          "type": "string",
          "description": "Compilation profile used",
          "examples": ["test", "debug", "release"]
        }
      }
    },
    "test_summary": {
      "type": "object",
      "description": "Parsed test summary from output (if available)",
      "properties": {
        "total_tests": {
          "type": "integer",
          "minimum": 0,
          "examples": [42, 1]
        },
        "passed": {
          "type": "integer",
          "minimum": 0,
          "examples": [40, 1]
        },
        "failed": {
          "type": "integer",
          "minimum": 0,
          "examples": [2, 0]
        },
        "ignored": {
          "type": "integer",
          "minimum": 0,
          "examples": [5, 0]
        },
        "measured": {
          "type": "integer",
          "minimum": 0,
          "examples": [10, 0]
        }
      }
    },
    "workspace_info": {
      "type": "object",
      "description": "Workspace and repository information",
      "properties": {
        "workspace_root": {
          "type": "string",
          "description": "Path to the workspace root",
          "examples": ["/home/coding/bead-forge"]
        },
        "git_branch": {
          "type": "string",
          "description": "Git branch at time of test",
          "examples": ["needle/bf-38dsw0"]
        },
        "git_commit": {
          "type": "string",
          "description": "Short git commit hash",
          "examples": ["a1b2c3d4"]
        }
      }
    },
    "host_info": {
      "type": "object",
      "description": "Host system information",
      "properties": {
        "hostname": {
          "type": "string",
          "description": "System hostname"
        },
        "os": {
          "type": "string",
          "description": "Operating system",
          "examples": ["Linux"]
        },
        "arch": {
          "type": "string",
          "description": "System architecture",
          "examples": ["x86_64"]
        }
      }
    }
  }
}
```

## Field Compatibility and Migration

### Required Fields (Core)

The following fields are **required** for all cargo test traces:

- `trace_id` — Unique identifier
- `trace_format` — Must be `"cargo_test_output"`
- `bead_id` — Associated bead identifier
- `exit_code` — Process exit code
- `outcome` — High-level success/failure
- `execution_time.duration_ms` — Execution duration in milliseconds
- `captured_at` — Capture timestamp
- `test_command` — Command that was executed
- `stdout_bytes` — Size of stdout
- `stderr_bytes` — Size of stderr

### Optional Fields (Extended)

The following fields are **optional** but recommended for enhanced functionality:

- `test_name` — Descriptive name for the test run
- `execution_time.start_time` — Start timestamp
- `execution_time.end_time` — End timestamp
- `execution_time.compilation_duration_ms` — Compilation time
- `execution_time.test_execution_duration_ms` — Pure test execution time
- `test_arguments` — Arguments array for programmatic parsing
- `cargo_metadata` — Cargo version and build information
- `test_summary` — Parsed test results (passed/failed/ignored counts)
- `workspace_info` — Git and workspace context
- `host_info` — System information

## Compatibility with Existing bead-forge Trace System

### Format Identifier

The `trace_format: "cargo_test_output"` distinguishes this format from other trace types:
- `"claude_json"` — Agent execution traces
- `"test_output"` — Generic test output traces  
- `"cargo_test_output"` — **This format: Cargo-specific test traces**

### Shared Fields

These fields maintain compatibility with the broader bead-forge trace system:

- `trace_id` — Follows bf-* naming convention
- `bead_id` — Links traces to beads
- `exit_code` — Standard exit code field
- `outcome` — Standard outcome field
- `duration_ms` — Standard duration field (preferred location: `execution_time.duration_ms` for backward compatibility)

### File Layout Compatibility

The three-file layout (`metadata.json`, `stdout.txt`, `stderr.txt`) is consistent across all trace formats in bead-forge.

## Example Complete metadata.json

```json
{
  "trace_id": "bf-38dsw0-20260724-151645",
  "trace_format": "cargo_test_output",
  "bead_id": "bf-38dsw0",
  "test_name": "verify_cli_commands",
  "exit_code": 0,
  "outcome": "success",
  "execution_time": {
    "start_time": "2026-07-24T15:16:45.123456789Z",
    "end_time": "2026-07-24T15:16:47.234567890Z",
    "duration_ms": 2111,
    "compilation_duration_ms": 1500,
    "test_execution_duration_ms": 611
  },
  "captured_at": "2026-07-24T15:16:47.234567890Z",
  "test_command": "cargo test test_show_basic --test test_show_command",
  "test_arguments": ["test", "test_show_basic", "--test", "test_show_command"],
  "stdout_bytes": 177,
  "stderr_bytes": 0,
  "cargo_metadata": {
    "cargo_version": "cargo 1.75.0",
    "rustc_version": "rustc 1.75.0",
    "target_triple": "x86_64-unknown-linux-gnu",
    "profile": "test"
  },
  "test_summary": {
    "total_tests": 1,
    "passed": 1,
    "failed": 0,
    "ignored": 0,
    "measured": 0
  },
  "workspace_info": {
    "workspace_root": "/home/coding/bead-forge",
    "git_branch": "needle/bf-38dsw0",
    "git_commit": "a1b2c3d4e5f6"
  },
  "host_info": {
    "hostname": "hazel",
    "os": "Linux",
    "arch": "x86_64"
  }
}
```

## Minimum Viable metadata.json

For backward compatibility and simplicity, a minimal trace can omit optional fields:

```json
{
  "trace_id": "bf-38dsw0-20260724-151645",
  "trace_format": "cargo_test_output",
  "bead_id": "bf-38dsw0",
  "exit_code": 0,
  "outcome": "success",
  "execution_time": {
    "duration_ms": 2111
  },
  "captured_at": "2026-07-24T15:16:47.234567890Z",
  "test_command": "cargo test -- -q",
  "stdout_bytes": 177,
  "stderr_bytes": 0
}
```

## stdout.txt Format

The `stdout.txt` file contains the raw, unmodified standard output from the cargo test execution, including:

- Compilation warnings and errors
- Test execution output
- Individual test results
- Pass/fail status messages
- Timing information (if using `--nocapture` or `-Z unstable-options --formatting=json`)

**Important:** This is the raw output from cargo test, not parsed or reformatted.

## stderr.txt Format

The `stderr.txt` file contains the raw standard error output from cargo test, which typically includes:

- Compiler error messages
- Linker errors
- Warnings that are output to stderr
- Any other error output

## Usage Examples

### Creating a Trace

```rust
use bead_forge::trace::{TraceManager, TraceMetadata, ExecutionTime};

let manager = TraceManager::for_current_workspace()?;

let execution_time = ExecutionTime {
    start_time: Some("2026-07-24T15:16:45.123456789Z".to_string()),
    end_time: Some("2026-07-24T15:16:47.234567890Z".to_string()),
    duration_ms: Some(2111),
    compilation_duration_ms: Some(1500),
    test_execution_duration_ms: Some(611),
};

let metadata = TraceMetadata {
    trace_id: Some("bf-38dsw0-20260724-151645".to_string()),
    trace_format: Some("cargo_test_output".to_string()),
    bead_id: Some("bf-38dsw0".to_string()),
    test_name: Some("verify_cli_commands".to_string()),
    exit_code: Some(0),
    outcome: "success".to_string(),
    execution_time: Some(execution_time),
    captured_at: "2026-07-24T15:16:47.234567890Z".to_string(),
    test_command: "cargo test test_show_basic --test test_show_command".to_string(),
    stdout_bytes: Some(177),
    stderr_bytes: Some(0),
    ..Default::default()
};

manager.write_bead_trace(
    "bf-38dsw0-20260724-151645",
    &metadata,
    "Test stdout output here...",
    "Test stderr output here..."
)?;
```

### Reading a Trace

```rust
use bead_forge::trace::TraceManager;

let manager = TraceManager::for_current_workspace()?;
let metadata = manager.read_bead_metadata("bf-38dsw0-20260724-151645")?;
let stdout = manager.read_bead_stdout("bf-38dsw0-20260724-151645")?;
let stderr = manager.read_bead_stderr("bf-38dsw0-20260724-151645")?;

println!("Test outcome: {}", metadata.outcome);
println!("Duration: {}ms", metadata.execution_time.as_ref().unwrap().duration_ms.unwrap());
```

## Schema Validation

The JSON schema can be validated using standard JSON Schema validators:

```bash
# Using ajv (JSON Schema validator)
npm install -g ajv-cli
ajv validate -m coq --spec=docs/trace-format-schema.json docs/metadata.json

# Using jsonschema Python package
pip install jsonschema
python -m jsonschema --instance .beads/traces/bf-38dsw0-20260724-151645/metadata.json --schema docs/trace-format-schema.json
```

## Migration from Generic test_output Format

Existing traces using `trace_format: "test_output"` can be migrated to `cargo_test_output` by:

1. Updating `trace_format` to `"cargo_test_output"`
2. Moving `duration_ms` → `execution_time.duration_ms`
3. Adding optional `execution_time.start_time` and `execution_time.end_time` if available
4. Adding optional `cargo_metadata`, `test_summary`, and other enhanced fields

Backward compatibility is maintained by keeping the top-level `duration_ms` field as an alias for `execution_time.duration_ms`.

## Version History

- **v1.0** (2026-07-24) — Initial cargo test output trace format specification
  - Defined core required fields
  - Added optional extended fields for enhanced metadata
  - Established JSON schema for validation
  - Documented compatibility with existing bead-forge trace system

## See Also

- [Test Output Capture Mechanism](test-output-capture.md) — Implementation guide for using this format
- [Trace Management](../README.md#trace-management) — Overview of bead-forge trace infrastructure
- [src/trace.rs](../src/trace.rs) — Rust implementation of trace handling