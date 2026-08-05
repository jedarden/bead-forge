# Test Bead Claiming with Dependencies (bf-2smtbv)

## Summary
Comprehensive test coverage for bead claiming with dependency blocking behavior.

## What Was Implemented
Created `tests/test_claim_with_dependencies.rs` with 6 integration tests:

1. **test_basic_claim_single_bead**: Verifies basic claim functionality - creates a bead, claims it, and validates status changes to `in_progress` with correct assignee.

2. **test_claim_creates_dependency_link**: Tests that dependencies can be created between beads using `add_dependency` and are properly stored.

3. **test_dependencies_appear_in_show**: Validates that when retrieving a bead, its dependencies are included in the Issue structure.

4. **test_claim_blocked_by_open_dependency**: Confirms that beads with open blockers are excluded from ready candidates - the core blocking behavior.

5. **test_claim_unblocked_after_blocker_closes**: Tests that once a blocker closes, the dependent becomes available for claiming.

6. **test_claim_prioritizes_high_downstream_impact**: Verifies that the claim scoring algorithm prioritizes beads with more downstream impact (more dependents).

## Test Results
All 6 tests passing:
- `test_claim_blocked_by_open_dependency` ✓
- `test_basic_claim_single_bead` ✓  
- `test_claim_creates_dependency_link` ✓
- `test_claim_prioritizes_high_downstream_impact` ✓
- `test_claim_unblocked_after_blocker_closes` ✓
- `test_dependencies_appear_in_show` ✓

## Key Validations
- Claiming uses `BEGIN IMMEDIATE` transactions for atomicity
- Dependencies are stored in the `dependencies` table with proper types
- `get_ready_candidates` excludes beads with open blockers
- Downstream impact scoring works correctly (higher count = higher priority)
- Status transitions work correctly (open → in_progress upon claim)
- Custom terminal statuses (like "completed") satisfy dependencies

## Files Modified
- Created: `tests/test_claim_with_dependencies.rs` (245 lines)
- Modified: `Cargo.toml` (added test target definition)

## Technical Notes
- Uses `tempfile` crate for isolated test databases
- Uses `with_immediate_transaction` for all claim operations
- Tests cover both storage layer (`get_dependencies`) and claim layer (`get_ready_candidates`, `claim`)
