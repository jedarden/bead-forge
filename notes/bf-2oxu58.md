# Bead bf-2oxu58: Cargo Test Stdout Capture Implementation

## Status: COMPLETE ✓

## Summary

The cargo test stdout capture functionality was already fully implemented in `src/trace.rs`. This bead verified that the implementation meets all acceptance criteria.

## Implementation Overview

### Core Methods Implemented

1. **`TraceManager::run_cargo_test(workspace_dir)`**
   - Executes `cargo test` in the specified directory
   - Captures both stdout and stderr via `Command::output()`
   - Returns `CargoTestResult` with exit code, duration, and trace path
   - Location: `src/trace.rs:572`

2. **`TraceManager::run_cargo_test_to_bead_trace(workspace_dir, bead_id, metadata)`**
   - Executes `cargo test` with bead-specific trace directory
   - Creates timestamped trace directory: `.beads/traces/{bead_id}-{timestamp}/`
   - Writes structured trace files (metadata.json, stdout.txt, stderr.txt)
   - Returns `BeadTestResult` with detailed execution information
   - Location: `src/trace.rs:719`

3. **`TraceManager::run_cargo_test_with_args(workspace_dir, args)`**
   - Executes `cargo test` with custom arguments
   - Supports test filtering and custom cargo options
   - Location: `src/trace.rs:634`

4. **`TraceManager::run_cargo_test_to_bead_trace_with_args(...)`**
   - Combines bead trace directory support with custom arguments
   - Location: `src/trace.rs:799`

### Trace File Structure

Each cargo test execution creates a structured trace directory:

```
.beads/traces/{bead_id}-{timestamp}/
├── metadata.json  # Execution metadata (exit code, timing, outcome)
├── stdout.txt     # Captured standard output
└── stderr.txt     # Captured standard error
```

### Key Features

- **Process Execution**: Uses `std::process::Command` for reliable process spawning
- **Stream Capture**: `Command::output()` captures both stdout and stderr separately
- **Timing Records**: Captures start time, end time, and duration in milliseconds
- **Exit Code Tracking**: Records process exit code for success/failure detection
- **Error Handling**: Comprehensive error handling with `anyhow::Context`
- **Unique Directories**: Timestamp suffixes prevent collisions on multiple runs

## Test Coverage

All 32 trace tests pass successfully, covering:

- Basic cargo test execution in temporary workspaces
- Stdout capture with known output patterns
- Stderr capture with warnings and failures
- Independent stdout and stderr stream capture
- Multiple test runs creating distinct trace directories
- Bead trace directory creation and naming
- Metadata serialization and validation
- Custom argument support
- Empty stderr handling for clean tests

## Acceptance Criteria Verification

✅ **1. cargo test command executes in ~/NEEDLE directory**
   - Implemented via `Command::new("cargo").arg("test").current_dir(workspace_dir)`
   - Verified: Trace tests demonstrate execution in arbitrary directories

✅ **2. stdout stream is captured to trace file**
   - Implemented via `Command::output()` capturing stdout and stderr
   - Verified: Tests confirm stdout content is written to files

✅ **3. Basic trace file structure is written**
   - Implemented: metadata.json, stdout.txt, stderr.txt structure
   - Verified: All trace files contain expected structure

✅ **4. Command completes and execution starts**
   - Implemented: Full process lifecycle management with timing
   - Verified: All 32 trace tests pass successfully

## Usage Examples

### Basic Usage

```rust
use bead_forge::trace::TraceManager;

let manager = TraceManager::for_current_workspace()?;
let needle_dir = Path::new("/home/coding/NEEDLE");

// Run cargo test and capture output
let result = manager.run_cargo_test(needle_dir)?;

println!("Exit code: {}", result.exit_code);
println!("Duration: {}ms", result.duration_ms);
println!("Output: {}", result.trace_path.display());
```

### Bead-Specific Trace

```rust
use bead_forge::trace::{TraceManager, TraceMetadata};

let manager = TraceManager::for_current_workspace()?;
let metadata = TraceMetadata {
    bead_id: Some("bf-2oxu58".to_string()),
    agent: "needle-worker".to_string(),
    ..Default::default()
};

let result = manager.run_cargo_test_to_bead_trace(
    Path::new("/home/coding/NEEDLE"),
    "bf-2oxu58",
    &metadata
)?;

println!("Trace directory: {}", result.bead_trace_dir.display());
println!("Stdout length: {} bytes", result.stdout.len());
```

### With Custom Arguments

```rust
let result = manager.run_cargo_test_with_args(
    Path::new("/home/coding/NEEDLE"),
    &["--", "--nocapture", "specific_test"]
)?;
```

## Technical Implementation Details

### Process Execution

```rust
let output = Command::new("cargo")
    .arg("test")
    .current_dir(workspace_dir)
    .output()
    .context("Failed to execute cargo test")?;
```

### Stream Capture

```rust
let stdout = String::from_utf8_lossy(&output.stdout).to_string();
let stderr = String::from_utf8_lossy(&output.stderr).to_string();
```

### Trace File Writing

```rust
// metadata.json
serde_json::to_string_pretty(metadata)?;

// stdout.txt
fs::write(&stdout_path, stdout)?;

// stderr.txt
fs::write(&stderr_path, stderr)?;
```

## Files Modified

This bead verified existing implementation in:
- `src/trace.rs` - Full implementation of cargo test stdout capture (already complete)

## Test Results

All 32 trace tests pass:
- `test_run_cargo_test_in_temp_workspace` ✓
- `test_stdout_capture_with_known_output` ✓
- `test_stdout_capture_comprehensive` ✓
- `test_run_cargo_test_to_bead_trace` ✓
- `test_run_cargo_test_with_custom_args` ✓
- `test_stderr_capture_with_known_output` ✓
- `test_stderr_and_stdout_independent_capture` ✓
- And 25 more comprehensive tests ✓

## Conclusion

The cargo test stdout capture functionality is fully implemented, tested, and production-ready. The implementation in `src/trace.rs` provides comprehensive support for executing cargo test in arbitrary directories, capturing stdout and stderr streams, and writing structured trace files with timing and metadata information.
