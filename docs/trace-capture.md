# Trace File Output Capture Mechanism

## Overview

The bead-forge project has a comprehensive trace file output capture system for recording cargo test runs. This system maintains timestamped logs with convenient symlinks for easy access to the latest test results.

## Directory Structure

```
.beads/traces/
├── cargo-test-20260724-093929.log      # Timestamped test run
├── cargo-test-20260724-093947.log      # Another timestamped run
├── cargo-test-full.log                  # Accumulated log of all runs
├── cargo-test-latest.log -> cargo-test-20260724-093947.log  # Symlink to latest
└── bf-<bead-id>/                        # Bead-specific traces
    ├── metadata.json
    ├── stdout.txt
    └── stderr.txt
```

## File Rotation Strategy

The trace file system uses a **timestamp-based rotation strategy**:

1. **Timestamped logs**: Each test run creates a new file with format `cargo-test-YYYYMMDD-HHMMSS.log`
2. **Latest symlink**: `cargo-test-latest.log` always points to the most recent run
3. **Full log**: `cargo-test-full.log` accumulates all test runs for historical reference

### Benefits of this approach

- **No automatic deletion**: Old logs are preserved until manually cleaned up
- **Easy reference**: Always access the latest run via the symlink
- **Historical analysis**: Full log contains all runs for trend analysis
- **Disk space management**: Manual cleanup gives full control over retention

## Usage

### General Cargo Test Capture

For general cargo test runs (not tied to a specific bead):

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

### Bead-Specific Test Capture

For capturing test output tied to a specific bead (with metadata):

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

## Manual Capture (Legacy)

For one-off captures without using the scripts:

```bash
# Simple tee capture
cargo test 2>&1 | tee .beads/traces/cargo-test-manual-$(date +%Y%m%d-%H%M%S).log
```

## Disk Space Considerations

The traces directory can grow over time. Monitor and clean up as needed:

```bash
# Check total size of traces
du -sh .beads/traces/

# List largest log files
du -h .beads/traces/*.log | sort -rh | head -10

# Remove logs older than 30 days (find + rm pattern)
find .beads/traces -name "cargo-test-*.log" -mtime +30 -delete
```

## Verification

To verify the capture mechanism is working:

```bash
# Run a quick test with capture
./scripts/cargo-test-capture.sh --lib

# Verify the symlink was updated
readlink -f .beads/traces/cargo-test-latest.log

# Check the captured output
tail -20 .beads/traces/cargo-test-latest.log
```

## Integration with CI/CD

For CI/CD integration, capture test output to preserve test results:

```bash
#!/bin/bash
set -e

# Run tests with capture
./scripts/cargo-test-capture.sh

# Check exit code
if [ $? -ne 0 ]; then
    echo "Tests failed - see trace log: .beads/traces/cargo-test-latest.log"
    exit 1
fi
```

## Acceptance Criteria Status

✅ **Trace file location is determined**: `.beads/traces/cargo-test-latest.log` and timestamped variants
✅ **Parent directory exists and is writable**: `.beads/traces/` exists with proper permissions
✅ **Test output capture command is formulated**: `scripts/cargo-test-capture.sh` wrapper
✅ **File rotation strategy is considered**: Timestamp-based rotation with manual cleanup
