# Bead bf-5pib46: Add stdout and stderr capture to trace files

## Status: ALREADY IMPLEMENTED ✓

The stdout and stderr capture functionality for cargo test execution is **already fully implemented** in `src/trace.rs`. All acceptance criteria are met.

## Verification

### Implementation Location
- **File**: `src/trace.rs`
- **Key methods**:
  - `TraceManager::write_stdout()` (lines 188-198)
  - `TraceManager::write_stderr()` (lines 200-211)
  - `TraceManager::write_bead_trace()` (lines 214-225)
  - `TraceManager::run_cargo_test_to_bead_trace()` (lines 498-559)

### Acceptance Criteria Verification

✅ **Both stdout and stderr are captured during test execution**
- Uses `Command::output()` which captures both streams automatically
- Implementation: `output.stdout` and `output.stderr` in cargo test methods
- Code location: `src/trace.rs:513-515` and `529-530`

✅ **Output streams are written to trace file**
- Separate files: `stdout.txt` and `stderr.txt` 
- Combined format: timestamped `.log` files with section headers
- Implementation: `write_stdout()` and `write_stderr()` methods

✅ **Trace file is created in .beads/traces/ directory**
- Bead-specific format: `.beads/traces/{bead_id}/stdout.txt` and `stderr.txt`
- Generic format: `.beads/traces/cargo-test-YYYYMMDD-HHMMSS.log`
- Implementation: `ensure_traces_dir()` and `bead_trace_dir()` methods

✅ **Both output streams are complete and untruncated**
- `Command::output()` waits for process completion
- Returns complete `Vec<u8>` for both streams
- No size limits or truncation in implementation
- Code location: `src/trace.rs:529-530` using `String::from_utf8_lossy()`

### Evidence from Working System

1. **Trace files exist and have content**:
   ```
   .beads/traces/bf-45bcdx/
   ├── metadata.json (372 bytes)
   ├── stdout.txt (1.4MB, complete output)
   └── stderr.txt (288 bytes, 2 lines)
   
   .beads/traces/bf-4rpfs/
   ├── metadata.json
   ├── stdout.txt (7198 lines)
   └── stderr.txt (2 lines)
   ```

2. **Tests pass**:
   ```
   test trace::tests::test_run_cargo_test_to_bead_trace ... ok
   test trace::tests::test_run_cargo_test_to_bead_trace_with_args ... ok
   test trace::tests::test_run_cargo_test_to_bead_trace_with_failure ... ok
   ```

3. **Implementation verification**:
   - All 16 trace tests pass
   - No compilation errors
   - Production trace files show complete capture

## Technical Implementation Details

The core capture mechanism uses Rust's `std::process::Command`:

```rust
// Execute cargo test, capturing both stdout and stderr
let output = Command::new("cargo")
    .arg("test")
    .current_dir(workspace_dir)
    .output()
    .with_context(|| {
        format!(
            "Failed to execute cargo test in workspace: {}",
            workspace_dir.display()
        )
    })?;

// Convert output to strings (complete, untruncated)
let stdout = String::from_utf8_lossy(&output.stdout).to_string();
let stderr = String::from_utf8_lossy(&output.stderr).to_string();

// Write to bead trace directory
self.write_bead_trace(bead_id, &exec_metadata, &stdout, &stderr)?;
```

This approach:
- Is atomic (no race conditions between streams)
- Captures complete output (no truncation)
- Separates stdout and stderr properly
- Works across platforms (Linux, macOS, Windows)

## Related Verification

This functionality was previously verified in bead **bf-33e0ts** with the same acceptance criteria. The current bead (bf-5pib46) is a split-child that depends on bf-3rf1aa (basic execution) being implemented first.

## Conclusion

No additional implementation needed. The functionality is complete, tested, and working in production. All acceptance criteria are met by the existing implementation in `src/trace.rs`.