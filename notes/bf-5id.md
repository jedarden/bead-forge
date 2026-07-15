# bf-5id Implementation Summary

## Problem
`close_issue()` never transitioned a dependent's status from 'blocked' to 'open' when its last blocker closed, causing sequential dependency chains (genesis + phase beads) to freeze after the first phase closed.

## Solution Implemented

### 1. Core Cascade Logic (src/storage/sqlite.rs:779-832)
The `close_issue()` function now:
- Finds all dependents of the bead being closed
- For each dependent at status='blocked':
  - Checks if it has any remaining non-terminal blockers
  - If no remaining blockers, transitions status to 'open'
  - Records a 'status_changed' event
  - Marks the bead as dirty for JSONL export
- Rebuilds the blocked_issues_cache to maintain consistency

### 2. Doctor Check (src/doctor.rs:145-175)
Added `check_stale_blocked_statuses()` function that:
- Finds beads with status='blocked' but zero non-terminal blockers
- Reports them in `bf doctor` output with remediation instructions
- Catches existing stale beads from before the fix

### 3. Test Coverage

#### Unit Tests (tests/test_bf_5id.rs)
- Direct storage API tests covering all edge cases
- Tests for single blocker, multiple blockers, dependency chains
- Tests for idempotence and non-blocked status handling
- All 5 tests passing

#### Integration Tests (tests/test_blocked_cascade.rs)  
- Full CLI workflow tests
- Covers single blocker, multiple blockers, three-phase chains
- Tests that cascade only affects status='blocked' beads
- All 4 tests passing

### 4. br Compatibility
The fix aligns with expected br behavior where closing a blocker should unblock dependents. This is a bug fix, not a behavioral enhancement.

## Verification
```bash
# Unit tests pass
cargo test --test test_bf_5id
# All 5 tests passed

# Integration tests pass  
cargo test --test test_blocked_cascade
# All 4 tests passed

# Build successful
cargo build
# No errors
```

## Files Modified/Created
- `src/storage/sqlite.rs` - Cascade logic (already implemented)
- `src/doctor.rs` - Doctor check (already implemented)
- `tests/test_bf_5id.rs` - Unit tests (already implemented)
- `tests/test_blocked_cascade.rs` - Integration tests (newly added)

## Commit
Commit b9b708d6: "test(bf-5id): Add integration tests for blocked->open cascade"
