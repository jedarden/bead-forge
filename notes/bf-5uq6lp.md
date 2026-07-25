# bf-5uq6lp: Add stdout capture to test execution

## Summary

Successfully verified that stdout capture is working correctly for cargo test execution in bead-forge. The existing `TraceManager` implementation in `src/trace.rs` already provides comprehensive stdout capture functionality.

## Acceptance Criteria Met

✅ **AC1**: cargo test stdout is captured to trace file
- Verified via `TraceManager::run_cargo_test_to_bead_trace_with_args()`
- stdout content captured via `Command::output()` and written to `stdout.txt`

✅ **AC2**: Standard output from test modules appears in trace
- Verified with test project containing `println!` statements in test modules
- Test output lines (MODULE_STDOUT_LINE_1/2/3) correctly captured

✅ **AC3**: Trace file shows complete stdout output
- `stdout.txt` file contains full 324-byte stdout output
- File content exactly matches captured stdout in memory

✅ **AC4**: No stdout output is lost during execution
- All expected stdout lines preserved during capture
- Total 14 stdout lines captured including 3 test module output lines

## Implementation Details

The existing `TraceManager` provides multiple methods for stdout capture:

1. **`run_cargo_test()`** - Basic cargo test execution with stdout capture
2. **`run_cargo_test_with_args()`** - Cargo test with custom arguments
3. **`run_cargo_test_to_bead_trace()`** - Full trace capture to bead directory
4. **`run_cargo_test_to_bead_trace_with_args()`** - Full trace with custom args (e.g., `--nocapture`)

The key to capturing test module stdout is using `--nocapture` argument:
```rust
let result = trace_manager.run_cargo_test_to_bead_trace_with_args(
    &workspace_dir,
    bead_id,
    &metadata,
    &["--", "--nocapture"] // CRITICAL: shows test stdout
)?;
```

## Trace File Structure

Each execution creates a timestamped bead trace directory:
```
.beads/traces/bf-5uq6lp-20260725-005548-024/
├── metadata.json    # Execution info (exit code, timing, etc.)
├── stdout.txt       # Captured stdout (324 bytes)
└── stderr.txt       # Captured stderr (0 bytes for clean tests)
```

## Verification

Created comprehensive verification script at `examples/verify_test_module_stdout.rs` that:
- Creates test project with stdout-producing test modules
- Runs cargo test with `--nocapture` to capture test stdout
- Verifies all acceptance criteria
- Confirms trace file structure and content integrity

All 32 trace module unit tests pass, including:
- `test_stdout_capture_with_known_output`
- `test_stdout_capture_comprehensive`  
- `test_run_cargo_test_to_bead_trace_with_args`
- `test_run_cargo_test_to_bead_trace`

## Files Modified

- `examples/verify_test_module_stdout.rs` - New comprehensive verification script

## Files Verified Working

- `src/trace.rs` - Existing stdout capture implementation (lines 572-862)
- All existing trace capture methods work correctly

## Notes

No changes were required to the core implementation - the existing `TraceManager` already handles stdout capture correctly. The bead focused on verification and documentation of the existing functionality.
