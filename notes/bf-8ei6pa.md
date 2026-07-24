# Bead bf-8ei6pa: Cargo Test Execution with Output Capture

## Summary

Implemented core functionality to run `cargo test` and capture all stdout/stderr output to bead-specific trace directories.

## What Was Implemented

### 1. New Methods in `TraceManager` (`src/trace.rs`)

#### `run_cargo_test_to_bead_trace()`
- Executes `cargo test` in a specified workspace directory
- Captures both stdout and stderr streams separately
- Writes output to bead-specific trace directory using the naming scheme from bf-177v7f:
  - `.beads/traces/{bead_id}/metadata.json`
  - `.beads/traces/{bead_id}/stdout.txt`
  - `.beads/traces/{bead_id}/stderr.txt`
- Automatically updates metadata with execution results (exit code, duration, outcome)
- Returns `BeadTestResult` with captured output and trace directory path

#### `run_cargo_test_to_bead_trace_with_args()`
- Same as above but supports custom cargo test arguments
- Allows filtering specific tests or running with different options

### 2. New Result Type

#### `BeadTestResult`
- `exit_code: i32` - Exit code from cargo test
- `duration_ms: u64` - Execution time in milliseconds
- `bead_trace_dir: PathBuf` - Path to bead trace directory
- `stdout: String` - Captured stdout content
- `stderr: String` - Captured stderr content

## Acceptance Criteria Status

- ✅ cargo test command is executed in ~/NEEDLE directory (via `workspace_dir` parameter)
- ✅ Both stdout and stderr streams are captured (separate files)
- ✅ Output is written to trace file using the naming scheme from bead bf-177v7f
- ✅ Command runs to completion even if tests fail (uses `Command::output()` which waits for completion)
- ✅ Basic shell command execution infrastructure is in place

## Tests Added

Added 3 comprehensive tests in `src/trace.rs`:
1. `test_run_cargo_test_to_bead_trace` - Successful test execution
2. `test_run_cargo_test_to_bead_trace_with_failure` - Failing test execution
3. `test_run_cargo_test_to_bead_trace_with_args` - Custom arguments

All 16 trace module tests pass.

## Example Usage

```rust
use bead_forge::{TraceManager, TraceMetadata};
use std::path::Path;

let manager = TraceManager::for_current_workspace()?;

let metadata = TraceMetadata {
    bead_id: Some("bf-8ei6pa".to_string()),
    agent: "needle-worker".to_string(),
    ..Default::default()
};

let result = manager.run_cargo_test_to_bead_trace(
    Path::new("/home/coding/NEEDLE"),
    "bf-8ei6pa",
    &metadata
)?;

println!("Exit code: {}", result.exit_code);
println!("Duration: {}ms", result.duration_ms);
println!("Output written to: {}", result.bead_trace_dir.display());
```

## Files Modified

- `src/trace.rs` - Added new methods and result types
- `src/lib.rs` - Exported `BeadTestResult`

## Notes

This is the core execution logic for running tests and capturing their output. The infrastructure supports both timestamped cargo test logs (for general logging) and bead-specific traces (for tracking individual bead execution).
