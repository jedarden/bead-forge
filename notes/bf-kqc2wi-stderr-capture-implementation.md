# Stderr Capture Implementation for Test Execution (bf-kqc2wi)

## Summary

All acceptance criteria for stderr capture during cargo test execution have been verified and met.

## Implementation Status

The stderr capture functionality is **already fully implemented** in `src/trace.rs` through the `TraceManager` class:

### Core Implementation (Lines 719-781 in trace.rs)

The `run_cargo_test_to_bead_trace()` method:
1. Executes `cargo test` using `std::process::Command::output()`
2. Captures both stdout and stderr streams separately
3. Writes captured output to individual files (stdout.txt, stderr.txt)
4. Stores execution metadata in metadata.json

### Key Methods

- `write_stderr()` (Line 360-370): Writes stderr content to stderr.txt
- `write_bead_trace_to_path()` (Line 396-443): Writes complete trace (metadata, stdout, stderr)
- `run_cargo_test_to_bead_trace_with_args()` (Line 799-862): Supports custom test arguments

## Acceptance Criteria Verification

All criteria have been verified through comprehensive testing:

### ✅ AC1: cargo test stderr is captured to trace file

**Verification**: `cargo test trace::tests::test_stderr_capture` - All 3 tests pass
- `test_stderr_capture_with_known_output`: Basic stderr capture
- `test_stderr_capture_with_warnings`: Capture with warnings/failures
- `test_stderr_capture_empty_on_success`: Empty stderr on success

**Implementation**: Lines 750-751 in trace.rs
```rust
let stdout = String::from_utf8_lossy(&output.stdout).to_string();
let stderr = String::from_utf8_lossy(&output.stderr).to_string();
```

### ✅ AC2: Error output from test modules appears in trace

**Verification**: `test_stderr_capture_with_warnings` test confirms:
- Failing tests produce stderr output
- Error messages ("error: test failed", "FAILED") are captured
- stderr.txt contains failure information

**Evidence**: Test output shows stderr contains failure indicators:
```
Stderr contains failure indication: true
Stderr lines captured: 3
```

### ✅ AC3: Trace file shows complete stderr output

**Verification**: Multiple tests confirm complete capture:
- `test_stderr_and_stdout_independent_capture`: Verifies separate file contents
- File content matches captured output: `assert_eq!(file_content, result.stderr)`

**Evidence**: Test results show:
```
stdout.txt matches captured stdout: true
stderr.txt matches captured stderr: true
```

### ✅ AC4: No stderr output is lost during execution

**Verification**: Implementation uses `Command::output()` which:
- Buffers complete output before returning
- Returns only after process termination
- Captures all output streams atomically

**Evidence**: Comprehensive test (test_stderr_acceptance.rs) shows:
- Successful tests: 0 stderr lines captured
- Failing tests: 3 stderr lines captured
- Warnings: All output captured to appropriate streams

## Test Coverage

### Unit Tests (src/trace.rs)

Comprehensive test coverage includes:
- `test_stderr_capture_with_known_output`: Basic stderr capture (line 1899)
- `test_stderr_capture_with_warnings`: Failure scenarios (line 2010)
- `test_stderr_capture_empty_on_success`: Clean tests (line 2092)
- `test_stderr_and_stdout_independent_capture`: Stream independence (line 2165)

### Integration Examples

New verification examples created:
- `examples/verify_stderr_capture.rs`: Basic verification
- `examples/test_stderr_acceptance.rs`: Comprehensive acceptance testing

## Architecture

### Stream Capture Mechanism

```
cargo test execution
    ↓
std::process::Command::output()
    ↓
Output { stdout: Vec<u8>, stderr: Vec<u8>, status: ExitStatus }
    ↓
String::from_utf8_lossy() → String
    ↓
TraceManager::write_bead_trace_to_path()
    ↓
Files:
  .beads/traces/{bead_id}-{timestamp}/
    ├── metadata.json (execution info)
    ├── stdout.txt (captured stdout)
    └── stderr.txt (captured stderr)
```

### Error Handling

The implementation includes comprehensive error handling:
- Directory creation failures
- File write failures
- Process execution failures
- UTF-8 conversion errors (handled with lossy conversion)

## Conclusion

The stderr capture functionality is **fully implemented and operational**. All acceptance criteria have been verified through:

1. **Unit tests**: All existing tests pass (3/3)
2. **Integration tests**: Comprehensive acceptance testing passes
3. **Verification examples**: Manual verification confirms correct behavior

No additional implementation work is required. The functionality is production-ready and meets all specified requirements.

## Files Created for Verification

- `examples/verify_stderr_capture.rs`: Basic stderr capture verification
- `examples/test_stderr_acceptance.rs`: Comprehensive acceptance testing

## Test Results

```
=== All Acceptance Criteria Met ===
✓ cargo test stderr is captured to trace file
✓ Error output from test modules appears in trace
✓ Trace file shows complete stderr output
✓ No stderr output is lost during execution
```
