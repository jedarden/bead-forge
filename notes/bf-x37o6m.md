# Test Output Capture Mechanism (bf-x37o6m)

## Overview
Set up test runner script in ~/NEEDLE that captures all cargo test output to trace files in `.beads/traces/`.

## Implementation

### Script Location
`~/NEEDLE/scripts/run-tests-with-capture.sh` (which is `/home/coding/NEEDLE/scripts/run-tests-with-capture.sh`)

### Features
- Captures both stdout and stderr from `cargo test`
- Writes to timestamped trace files: `.beads/traces/cargo-test-YYYYMMDD-HHMMSS.log`
- Creates symlink to latest trace: `.beads/traces/cargo-test-latest.log`
- Displays colored output during test execution
- Generates summary with test results and warning counts
- Exits with cargo test's exit code for CI/CD compatibility

### Usage
```bash
# Run all tests
~/NEEDLE/scripts/run-tests-with-capture.sh

# Run library tests only
~/NEEDLE/scripts/run-tests-with-capture.sh --lib

# Run specific test
~/NEEDLE/scripts/run-tests-with-capture.sh test::test_name

# Pass additional args to test binary
~/NEEDLE/scripts/run-tests-with-capture.sh -- --nocapture
```

### Trace Files
- Location: `/home/coding/bead-forge/.beads/traces/`
- Naming: `cargo-test-YYYYMMDD-HHMMSS.log`
- Latest: `cargo-test-latest.log` (symlink)

### Verification
Script tested successfully with `--lib` flag:
- ✅ Captures all test output (52K log file created)
- ✅ Creates timestamped trace files
- ✅ Creates symlink to latest trace
- ✅ Generates summary with test results
- ✅ Exits with correct exit code
- ✅ Counts compiler warnings

### Example Output
```
[INFO] Traces directory: /home/coding/bead-forge/.beads/traces
[INFO] Trace file: /home/coding/bead-forge/.beads/traces/cargo-test-20260724-093947.log
[INFO] Starting cargo test run...
...
[INFO] Test summary: test result: FAILED. 273 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out
[WARNING] Found 43 compiler warnings
[INFO] Latest trace symlink: /home/coding/bead-forge/.beads/traces/cargo-test-latest.log
[INFO] Trace file size: 52K
```

## Completion Status
✅ All acceptance criteria met:
- Test runner script created at ~/NEEDLE/scripts/run-tests-with-capture.sh
- Script ensures .beads/traces/ directory exists
- Script captures both stdout and stderr from cargo test
- Output written to timestamped trace files
- Script is executable (chmod +x)
- Script can be run independently (verified)