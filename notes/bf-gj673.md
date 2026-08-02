# bf-gj673: Fix assignee-clearing gap

## Status
**COMPLETED** - Both child beads closed, all acceptance criteria met.

## Summary
Umbrella tracking bead for the assignee-clearing gap found during the 2026-07-21 lab NEEDLE fleet audit.

## Problem
The storage layer (`src/storage/sqlite.rs` `update_issue`) already correctly treats an empty/whitespace-only assignee as 'clear the field' (writes NULL). However, the CLI layer had contradictions that blocked this correct behavior.

## Child Beads Completed

### bf-4mj7l: Remove stale CLI-level assignee rejection
**Status:** CLOSED  
**Commit:** f769879  
**Summary:**
- Removed `validate_assignee()` calls from `cmd_create` and `cmd_update`
- Implemented `normalize_assignee()` in `src/validation.rs` to handle empty/whitespace assignee
- Empty/whitespace assignee now correctly normalizes to `None` for `bf create`
- Empty/whitespace assignee now correctly clears to NULL for `bf update`
- All 12 tests in `tests/test_assignee_validation.rs` pass (0 ignored)

### bf-2uhsk: bf reopen must clear the assignee field
**Status:** CLOSED  
**Summary:**
- `cmd_reopen` correctly clears assignee via `reopen_bead()` → `storage.reopen_issue()`
- `storage.reopen_issue()` sets `assignee = NULL` in SQL UPDATE
- Tests `test_reopen_clears_assignee` and `test_reopen_without_assignee_is_noop` pass
- Integration tests in `tests/test_close_reopen_integration.rs` pass (7/7 tests)

## Test Results
All relevant tests pass:
- `tests/test_assignee_validation.rs`: 12/12 passed (create/update with empty assignee)
- `src/reopen.rs`: 12/12 passed (reopen clears assignee)
- `tests/test_close_reopen_integration.rs`: 7/7 passed

## Acceptance Criteria Met
✅ `bf update --assignee ''` succeeds and clears assignee  
✅ `bf create --assignee ''` succeeds with no assignee  
✅ `bf reopen` clears stale assignee from closed beads  
✅ All tests pass with none ignored  
✅ cargo build clean  

## Implementation Verified
The fix is already in production:
- `cmd_create` uses `normalize_assignee()` (line 1548 in `src/cli/mod.rs`)
- `cmd_update` passes empty assignee through to storage layer (line 1832-1835 comment)
- `reopen_issue` SQL UPDATE sets `assignee = NULL` (src/storage/sqlite.rs)

## Notes
This was confirmed as a live-blocking issue for NEEDLE's bead-release-on-failure/timeout path on the lab fleet. The fix enables proper bead self-healing and worker release mechanisms.
