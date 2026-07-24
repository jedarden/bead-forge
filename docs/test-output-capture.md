# Test Output Capture Mechanism

## Overview

The test output capture mechanism provides a way to record `cargo test` execution results with precise timestamps and structured metadata. This is useful for:

- Recording test execution results for specific beads
- Debugging test failures with timestamped output
- Maintaining audit trails of test runs
- Providing evidence of test execution for acceptance criteria

## Location

The capture script is located at: `scripts/capture-test-output.sh`

## Usage

### Basic syntax

```bash
./scripts/capture-test-output.sh <bead-id> [-- <cargo-test-args>]
```

### Examples

**Run a specific test target:**
```bash
./scripts/capture-test-output.sh bf-3vhegr -- --test common
```

**Run a specific test function:**
```bash
./scripts/capture-test-output.sh bf-3vhegr -- --test common -- tests::test_assert_p0_epic
```

**Run multiple test targets:**
```bash
./scripts/capture-test-output.sh bf-3vhegr -- --test common --test batch_atomic
```

**Run tests with specific filter:**
```bash
./scripts/capture-test-output.sh bf-3vhegr -- -- autoflush
```

## Output Files

The script creates the following files in `.beads/traces/<bead-id>/`:

### 1. `metadata.json`
Structured metadata about the test run:
```json
{
  "bead_id": "bf-3vhegr",
  "test_type": "cargo_test",
  "exit_code": 0,
  "outcome": "success",
  "duration_ms": 1250,
  "start_time": "2026-07-24T11:45:55Z",
  "end_time": "2026-07-24T11:45:56Z",
  "cargo_args": ["--test","common"],
  "captured_at": "2026-07-24T11:45:56Z",
  "trace_format": "test_output_v1"
}
```

### 2. `stdout.txt`
Raw stdout output with timestamp prefixes:
```
[2026-07-24 07:45:55] running 15 tests
[2026-07-24 07:45:55] test tests::test_assert_p0_epic_display ... ok
[2026-07-24 07:45:55] test tests::test_assert_p0_epic ... ok
```

### 3. `stderr.txt`
Raw stderr output with timestamp prefixes (if any errors occur).

### 4. `output_with_timestamps.txt`
Combined output with header information:
```
# Test Output for Bead: bf-3vhegr
# Start: 2026-07-24T11:45:55Z
# End: 2026-07-24T11:45:56Z
# Duration: 1250ms
# Exit Code: 0
# Outcome: success
#
# === STDOUT ===
[timestamped output...]
# === STDERR ===
[timestamped errors...]
```

## Requirements

The script requires:
- `bash` (version 4+ for associative arrays)
- `cargo` (Rust build system)
- Optional: `ts` command from `moreutils` for precise timestamp formatting

If `ts` is not available, the script falls back to manual timestamp prepending using `date`.

## Exit Codes

The script returns the exit code from `cargo test`:
- `0`: All tests passed
- `non-zero`: Tests failed or error occurred

## Features

1. **Automatic directory creation**: Creates `.beads/traces/<bead-id>/` if it doesn't exist
2. **Timestamped output**: Every line of output gets a precise timestamp
3. **Duration tracking**: Records both start/end times and calculates duration
4. **Structured metadata**: Creates JSON metadata file for programmatic analysis
5. **Combined output**: Provides both separated stdout/stderr and combined view
6. **Flexible arguments**: Supports any `cargo test` arguments via `--` separator

## Integration with Beads

This script is designed to be used within bead development workflows:

1. Run tests after implementing a bead
2. Capture output evidence for acceptance criteria
3. Store trace files with bead ID for later reference
4. Use metadata for automated verification

## Example Workflow

```bash
# 1. Implement a bead
# ... code changes ...

# 2. Build and test
cargo build
./scripts/capture-test-output.sh bf-3vhegr -- --test common

# 3. Verify the trace output was created
ls -la .beads/traces/bf-3vhegr/

# 4. Review the combined output
cat .beads/traces/bf-3vhegr/output_with_timestamps.txt

# 5. Use the evidence for bead closure
br close bf-3vhegr --reason "Tests passed - see .beads/traces/bf-3vhegr/"
```

## Future Enhancements

Potential improvements for future versions:
- Add `--color` option for colored test output
- Support for parallel test execution with separate trace files
- Integration with CI/CD pipelines
- HTML report generation
- Test coverage analysis integration
