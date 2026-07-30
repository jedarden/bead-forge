# NEEDLE Workspace Trace Capture Configuration

## Overview

Comprehensive trace capture for cargo test execution is fully configured and operational in the bead-forge NEEDLE workspace.

## Acceptance Criteria Status

✅ **Trace capture output directory is configured and writable**
- Directory: `.beads/traces/`
- Permissions: Drwxrwxr-x (writable)
- Test: Confirmed writable via `test -w`

✅ **Cargo test command is configured to capture both stdout and stderr**
- Script: `scripts/cargo-test-capture.sh` (general capture)
- Script: `scripts/capture-test-output.sh` (bead-specific capture)
- Both stdout and stderr are captured separately with byte counts

✅ **Test execution timing will be recorded**
- Nanosecond precision timing converted to milliseconds
- Stored in `metadata.json` under `duration_ms` field
- Verified: 7ms duration captured successfully

✅ **Configuration is documented for reproducibility**
- Documentation: `docs/trace-capture.md`
- Documentation: `docs/test-output-capture.md`
- This note: `notes/bf-45f3jg.md`

## Trace Capture System

### Directory Structure

```
.beads/traces/
├── cargo-test-20260724-233328.log        # Timestamped general test run
├── cargo-test-latest.log -> ...         # Symlink to latest general run
├── cargo-test-full.log                  # Accumulated log of all runs
└── bf-<bead-id>/                        # Bead-specific traces
    ├── metadata.json                     # Test metadata
    ├── stdout.txt                       # Standard output
    └── stderr.txt                       # Standard error
```

### Capture Scripts

#### 1. General Test Capture (`scripts/cargo-test-capture.sh`)

For general cargo test runs not tied to a specific bead:

```bash
# Run all tests with output capture
./scripts/cargo-test-capture.sh

# Run specific tests with output capture
./scripts/cargo-test-capture.sh --lib

# Run tests for a specific module
./scripts/cargo-test-capture.sh test_autoflush
```

**Output locations:**
- Latest run: `.beads/traces/cargo-test-latest.log`
- Timestamped: `.beads/traces/cargo-test-YYYYMMDD-HHMMSS.log`
- Accumulated: `.beads/traces/cargo-test-full.log`

#### 2. Bead-Specific Capture (`scripts/capture-test-output.sh`)

For capturing test output tied to a specific bead with detailed metadata:

```bash
# Capture output for a bead's test run
./scripts/capture-test-output.sh bf-1234 test_show_command \
    cargo test test_show_command

# Capture output for all tests
./scripts/capture-test-output.sh bf-1234 all_tests
```

**Output locations:**
- `.beads/traces/bf-1234/`
  - `metadata.json` - Test metadata (exit code, duration, outcome)
  - `stdout.txt` - Standard output
  - `stderr.txt` - Standard error

### Metadata Format

```json
{
  "bead_id": "bf-45f3jg-test",
  "test_name": "test_trace_capture",
  "exit_code": 0,
  "outcome": "success",
  "duration_ms": 7,
  "captured_at": "2026-07-25T03:33:47.381331230Z",
  "trace_format": "test_output",
  "test_command": "cargo test test_version_display --test test_version_display",
  "stdout_bytes": 0,
  "stderr_bytes": 122
}
```

## Verification Results

### System Verification

```bash
# Directory exists and is writable
$ test -w /home/coding/bead-forge/.beads/traces && echo "Writable"
Writable

# Scripts are executable
$ test -x scripts/cargo-test-capture.sh && echo "Executable"
Executable

$ test -x scripts/capture-test-output.sh && echo "Executable"
Executable
```

### Test Run Verification

```bash
$ ./scripts/capture-test-output.sh bf-45f3jg-test test_trace_capture \
    "cargo test test_version_display --test test_version_display"

Capturing test output for test_trace_capture...
Trace directory: .beads/traces/bf-45f3jg-test
Test command: cargo test test_version_display --test test_version_display

✓ Test output captured to:
  .beads/traces/bf-45f3jg-test/

Results:
  Exit code: 0
  Outcome: success
  Duration: 7ms

Files:
  metadata.json: .beads/traces/bf-45f3jg-test/metadata.json
  stdout.txt: .beads/traces/bf-45f3jg-test/stdout.txt (0 bytes)
  stderr.txt: .beads/traces/bf-45f3jg-test/stderr.txt (122 bytes)
```

## Usage in NEEDLE Workflow

### For Test Execution Beads

When a NEEDLE bead involves running tests, use the trace capture system:

1. **General test runs**: Use `cargo-test-capture.sh` for routine testing
2. **Bead-specific validation**: Use `capture-test-output.sh` to create trace evidence for bead completion
3. **CI/CD integration**: Both scripts preserve exit codes for automated validation

### Trace ID Annotation

After test capture, annotate the bead with the trace directory:

```bash
bf comment <bead-id> --text "Test trace: .beads/traces/bf-<id>/"
```

## Disk Space Management

Monitor trace directory size:

```bash
# Check total size
du -sh .beads/traces/

# List largest files
du -h .beads/traces/*.log | sort -rh | head -10

# Clean old logs (optional)
find .beads/traces -name "cargo-test-*.log" -mtime +30 -delete
```

## Conclusion

The NEEDLE workspace trace capture configuration is complete and fully operational. All acceptance criteria have been met:

- ✅ Output directory configured and writable
- ✅ Cargo test commands configured for stdout/stderr capture
- ✅ Test execution timing recorded with nanosecond precision
- ✅ Configuration documented for reproducibility

The system is ready for comprehensive test trace capture in NEEDLE workflow beads.
