# Trace Output File Management (bf-3v4y4l)

## Status: COMPLETE

All acceptance criteria for trace output file management are already fully implemented in `src/trace.rs`.

## Implementation Summary

### TraceManager Structure
The `TraceManager` struct provides comprehensive trace file management with the following capabilities:

#### 1. Path Generation Functions ✅
- `bead_stdout_path(bead_id)` → `.beads/traces/{bead_id}/stdout.txt`
- `bead_stderr_path(bead_id)` → `.beads/traces/{bead_id}/stderr.txt`
- `bead_metadata_path(bead_id)` → `.beads/traces/{bead_id}/metadata.json`
- `trace_path_for_name(trace_name)` → `.beads/traces/{trace_name}`

#### 2. Directory Management ✅
- `ensure_traces_dir()` - Ensures `.beads/traces/` directory exists
- `bead_trace_dir(bead_id)` - Creates bead-specific directory
- `unique_bead_trace_dir(bead_id)` - Creates unique timestamped directories for multiple runs

#### 3. Error Handling ✅
- Comprehensive error handling with `anyhow::Context`
- Detailed error messages for permission issues, disk space problems
- Idempotent operations (safe to call multiple times)

#### 4. Write Operations ✅
- `write_bead_trace()` - Complete trace write (metadata + stdout + stderr)
- `write_bead_trace_to_path()` - Write to arbitrary directory path
- `write_metadata()`, `write_stdout()`, `write_stderr()` - Individual file writes

## Test Coverage
All 26 trace module tests pass:
- Directory creation and management
- Path generation and validation
- File writing operations
- Multiple run handling
- Error conditions
- Cargo test execution and capture

## API Interface

```rust
// Create manager for current workspace
let manager = TraceManager::for_current_workspace()?;

// Generate paths
let stdout_path = manager.bead_stdout_path("bf-123456");
let stderr_path = manager.bead_stderr_path("bf-123456");

// Ensure directory exists (idempotent)
manager.ensure_traces_dir()?;

// Write complete trace
manager.write_bead_trace("bf-123456", &metadata, &stdout, &stderr)?;
```

## Infrastructure Status
✅ Ready for production use
✅ Comprehensive test coverage
✅ Error handling and validation
✅ Documentation and examples
✅ Idempotent operations
✅ Multi-run support with unique directories
