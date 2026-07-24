# Test Output Capture Mechanism

## Overview

The test output capture mechanism provides a way to record test execution results with detailed timing, output, and metadata to `.beads/traces/` directories. This is useful for:

- Verifying test execution works correctly
- Tracking test performance over time
- Archiving test results for debugging
- Providing audit trails for test runs

## Usage

### Basic Usage

```bash
# Capture all tests
bash scripts/capture-test-output.sh bf-001

# Capture specific test with custom name
bash scripts/capture-test-output.sh bf-002 my_test "cargo test test_specific_function"

# Capture tests matching a pattern
bash scripts/capture-test-output.sh bf-003 show_tests "cargo test test_show"
```

### Arguments

1. **BEAD_ID** (required): Identifier for the trace directory (e.g., `bf-001`, `test-run-1`)
2. **TEST_NAME** (optional): Descriptive name for the test run (default: `all_tests`)
3. **TEST_COMMAND** (optional): The actual test command to run (default: `cargo test -- -q`)

## Output Structure

Each test run creates a directory in `.beads/traces/{BEAD_ID}/` containing:

### metadata.json

```json
{
  "bead_id": "bf-001",
  "test_name": "my_test",
  "exit_code": 0,
  "outcome": "success",
  "duration_ms": 255,
  "captured_at": "2026-07-24T11:49:43.238134665Z",
  "trace_format": "test_output",
  "test_command": "cargo test test_function",
  "stdout_bytes": 177,
  "stderr_bytes": 0
}
```

### stdout.txt

Full test output including:
- Compilation warnings and errors
- Test execution results
- Pass/fail status
- Timing information

### stderr.txt

Standard error output from the test run (if any).

## Metadata Fields

- `bead_id`: Identifier for this trace
- `test_name`: Descriptive name of the test run
- `exit_code`: Process exit code (0 = success)
- `outcome`: "success" or "failure"
- `duration_ms`: Execution time in milliseconds
- `captured_at`: ISO 8601 timestamp of capture completion
- `trace_format`: Always "test_output"
- `test_command`: The full command that was executed
- `stdout_bytes`: Size of stdout output in bytes
- `stderr_bytes`: Size of stderr output in bytes

## Examples

### Running a Single Test

```bash
bash scripts/capture-test-output.sh bf-show-basic test_show_basic \
  "cargo test test_show_basic_text_format --test test_show_command"
```

Output:
```
Capturing test output for test_show_basic...
Trace directory: .beads/traces/bf-show-basic
Test command: cargo test test_show_basic_text_format --test test_show_command

✓ Test output captured to:
  .beads/traces/bf-show-basic/

Results:
  Exit code: 0
  Outcome: success
  Duration: 255ms

Files:
  metadata.json: .beads/traces/bf-show-basic/metadata.json
  stdout.txt: .beads/traces/bf-show-basic/stdout.txt (177 bytes)
  stderr.txt: .beads/traces/bf-show-basic/stderr.txt (0 bytes)
```

### Running All Tests Matching a Pattern

```bash
bash scripts/capture-test-output.sh bf-all-show all_show_tests "cargo test test_show"
```

### Verifying Previous Captures

```bash
# List all trace directories
ls -la .beads/traces/

# View metadata for a specific run
cat .beads/traces/bf-001/metadata.json

# View test output
cat .beads/traces/bf-001/stdout.txt
```

## Integration with NEEDLE

The test output capture mechanism is designed to work with NEEDLE workflow beads:

1. Test execution beads can capture their results
2. Trace IDs can be stored as bead annotations
3. Results can be referenced in documentation beads

## Implementation Details

- **Timing**: Uses nanosecond precision timing converted to milliseconds
- **Platform Support**: Works on Linux with `script` command, with fallback for other systems
- **Error Handling**: Captures both successful and failed test runs
- **File Sizes**: Tracks output sizes to verify capture completeness

## Future Enhancements

Potential improvements:
- JSON streaming output format compatibility with NEEDLE agent traces
- Automatic trace ID generation from bead IDs
- Integration with `bf test` command
- HTML report generation from trace data
- Performance trend analysis across multiple runs
