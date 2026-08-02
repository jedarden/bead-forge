# Bead bf-637aj: Verification Notes

## Task
Replace custom JSON with formatter in cmd_ready

## Status: Already Implemented

The `cmd_ready()` function at lines 1914-1986 already uses the `get_formatter()` approach correctly.

## Implementation Verification

Current code in `src/cli/mod.rs`:

1. ✓ Line 1928: Uses `get_formatter(OutputFormat::Json)` 
2. ✓ Lines 1929-1932: Converts ReadyCandidates to Issue vec
3. ✓ Line 1934: Calls `formatter.format_issues(&issues)`
4. ✓ Lines 1935-1955: Proper envelope wrapping with `formatter.format_with_envelope("ready", &data)`

## Pattern Consistency

The implementation follows the exact same pattern as `cmd_list()` (lines 1677-1707):
- Parse format to OutputFormat
- Get formatter via `get_formatter()`
- Convert candidates to full Issue records
- Call `formatter.format_issues()` for output
- Handle envelope wrapping consistently

## Acceptance Criteria Met

- ✓ No custom `serde_json::to_string_pretty()` usage
- ✓ `get_formatter()` wired into cmd_ready()
- ✓ ReadyCandidates converted to Issue vec
- ✓ `format_issues()` called for output
- ✓ Code compiles without errors

## Historical Context

Commit `2264fb05` ("fix(cli): Fix ready command JSONL-to-JSON conversion for envelope format") likely implemented this fix previously.

## Conclusion

No code changes needed. The implementation is complete and follows the correct pattern.
