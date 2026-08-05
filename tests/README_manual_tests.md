# Manual Tests

This directory contains manual test scripts for verifying bead-forge functionality end-to-end.

## clear-assignee Test

### Purpose
Tests the `bf update --clear-assignee` flag functionality, which allows clearing an assignee from an existing bead.

### Test Script
`manual_test_clear_assignee.sh`

### Running the Test
```bash
./tests/manual_test_clear_assignee.sh
```

### Acceptance Criteria Verified
1. ✓ Creates a test bead with an assignee
2. ✓ Runs `bf update --clear-assignee` on the bead
3. ✓ Verifies the command succeeds without error
4. ✓ Confirms the assignee field is cleared in the output

### Test Workflow
1. Creates a temporary workspace
2. Initializes a `bf` workspace with `bf init`
3. Creates a test bead with an assignee set to "test-worker"
4. Runs `bf update --clear-assignee` to clear the assignee
5. Verifies the assignee field is set to `null` in the output
6. Displays the final bead state for manual inspection
7. Cleans up the temporary workspace

### Expected Output
The script should print:
- Initial bead ID
- Initial assignee value ("test-worker")
- Confirmation that the update command succeeded
- Final assignee value (null)
- A success message: "✓ All acceptance criteria passed!"
- Final bead state display
