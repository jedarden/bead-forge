# Bead bf-2a1dtc: Show Error Case Test

## Task
Implement show error case test

## Finding
The test was already implemented in `src/cli/tests/json_output.rs` at lines 1579-1608.

## Verification
Verified that the existing test `test_show_command_json_nonexistent_bead` meets all acceptance criteria:

1. ✅ Uses clearly non-existent bead ID: `"does-not-exist"`
2. ✅ Invokes show command with `--json` flag
3. ✅ Asserts command failure via `assert!(!success)`
4. ✅ Validates error message mentions "not found" or "Bead not found"
5. ✅ Follows existing patterns using `capture::capture_failed_command()`
6. ✅ Properly named as `test_show_command_json_nonexistent_bead`

The test properly validates that:
- The command fails for non-existent beads
- Stderr contains an informative error message about the bead not being found
- Stdout is empty (no JSON output for errors)

No code changes were required.
