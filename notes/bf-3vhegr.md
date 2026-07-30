# Test Output Capture Mechanism (bf-3vhegr)

## Summary

Implemented a test output capture mechanism that records test execution results with detailed timing, output, and metadata to `.beads/traces/` directories.

## What Was Implemented

### 1. Capture Script (`scripts/capture-test-output.sh`)

A bash script that:
- Accepts bead ID, test name, and test command as arguments
- Creates dedicated trace directories in `.beads/traces/{BEAD_ID}/`
- Captures stdout/stderr with precise timing
- Generates comprehensive metadata.json file
- Provides human-readable summary output

### 2. Trace Output Format

Each test run creates three files:

**metadata.json:**
```json
{
  "bead_id": "bf-3vhegr-test2",
  "test_name": "test_show_basic",
  "exit_code": 0,
  "outcome": "success",
  "duration_ms": 255,
  "captured_at": "2026-07-24T11:49:43.238134665Z",
  "trace_format": "test_output",
  "test_command": "cargo test test_show_basic_text_format --test test_show_command",
  "stdout_bytes": 177,
  "stderr_bytes": 0
}
```

**stdout.txt:** Full test output including compilation warnings, test results, and pass/fail status

**stderr.txt:** Standard error output (if any)

### 3. Documentation (`docs/test-output-capture.md`)

Comprehensive documentation covering:
- Usage examples and command syntax
- Output format specification
- Integration patterns with NEEDLE
- Platform support details

## Verification

Successfully tested with:
- Single test: `test_show_basic_text_format` (255ms, 177 bytes output)
- Multiple tests: `test_show*` pattern (481ms, 46,330 bytes output)
- Verified metadata timestamps (captured_at, duration_ms)
- Confirmed trace directory structure matches expected format

## Acceptance Criteria Met

✅ Create output capture script or command  
✅ Verify capture works with a small test run  
✅ Output is written to .beads/traces/  
✅ File includes full test output including timestamps  

## Usage Examples

```bash
# Capture specific test
bash scripts/capture-test-output.sh bf-001 test_show \
  "cargo test test_show_basic_text_format --test test_show_command"

# Capture all tests matching pattern  
bash scripts/capture-test-output.sh bf-002 show_tests "cargo test test_show"

# Capture all tests
bash scripts/capture-test-output.sh bf-003 all_tests
```

## Implementation Details

- **Precision Timing:** Uses nanosecond timing converted to milliseconds
- **Platform Support:** Works with Linux `script` command, with fallback
- **Error Handling:** Captures both successful and failed runs
- **File Size Tracking:** Monitors output sizes for completeness verification

## Files Created

- `scripts/capture-test-output.sh` (2,687 bytes)
- `docs/test-output-capture.md` (comprehensive documentation)
- `notes/bf-3vhegr.md` (this file)
