# Claim-Related Test Modules in bead-forge

This document provides a comprehensive inventory of all claim and metadata-related tests in the bead-forge codebase, identified for bead `bf-3jrox2`.

## Summary Statistics
- **Total claim-related test files identified**: 5 dedicated files
- **Total claim-related test functions**: 50+ tests
- **Tests using WorkerMetadata**: 10+ tests
- **Integration test files**: 130+ files in tests/ directory

## Core Claim Module Tests

### File: `src/claim.rs` (Unit Tests)
**Location**: `/home/coding/bead-forge/src/claim.rs` (lines 714-1238)

**Test Functions**:
1. `test_claim_basic()` - Basic claim operation
2. `test_claim_no_candidates()` - Claim with no available beads
3. `test_claim_reclaims_stale()` - Stale bead reclamation
4. `test_concurrent_claim_no_double_claim()` - No duplicate claims under concurrency
5. `test_critical_path_bonus_in_claim()` - Critical path scoring in claims
6. `test_critical_path_zero_float_outranks_high_priority()` - Zero-float beats high priority
7. `test_get_ready_candidates_limit_zero_returns_all()` - Unlimited limit behavior
8. `test_get_ready_candidates_respects_limit()` - Limit parameter behavior
9. `test_completed_status_blocker_unblocks_dependent()` - Custom terminal status handling
10. `test_ready_includes_zero_dependency_open_beads_bf_1nprw()` - Zero-dependency beads in ready output

**Key Components**:
- `WorkerMetadata` struct definition (lines 8-14)
- `ClaimResult` struct (lines 17-22)
- `Score` struct for candidate ranking (lines 33-132)

## Bead Store Integration Tests

### File: `src/bead_store.rs` (Unit Tests)
**Location**: `/home/coding/bead-forge/src/bead_store.rs` (lines 286-435)

**Test Functions**:
1. `test_claim_bead_basic()` - Basic bead claiming via bead_store API
2. `test_claim_bead_priority_ordering()` - Priority-based claim ordering
3. `test_claim_bead_empty_workspace()` - Empty workspace handling
4. `test_get_ready()` - Ready candidates retrieval
5. `test_is_bead_ready()` - Bead readiness check
6. `test_is_bead_ready_blocked()` - Blocked bead readiness check

**API Functions Tested**:
- `claim_bead()` - High-level claim API
- `get_ready()` - Non-destructive candidate retrieval
- `is_bead_ready()` - Readiness verification

## Integration Test Files

### File: `tests/claim_fallback.rs`
**Location**: `/home/coding/bead-forge/tests/claim_fallback.rs`

**Test Functions** (9 tests):
1. `test_claim_fallback_any_exhausted_primary_workspace()` - Fallback when primary empty
2. `test_claim_fallback_any_primary_has_beads_no_fallback()` - No fallback when primary has beads
3. `test_claim_fallback_any_empty_all_workspaces()` - All workspaces empty
4. `test_claim_fallback_any_selects_from_available_workspace()` - Selects from available
5. `test_claim_fallback_any_with_dependencies()` - Dependencies respected in fallback
6. `test_claim_fallback_any_pinned_beads_respected()` - Pinned beads respected
7. `test_claim_fallback_any_multiple_workspaces()` - Multi-workspace fallback
8. `test_cli_claim_fallback_any_exhausted_workspace()` - CLI-level fallback test
9. `test_claim_fallback_to_1800s_when_velocity_stats_empty()` - Velocity stats fallback to 1800s

**Uses WorkerMetadata**: All tests construct and pass `WorkerMetadata` structs

### File: `tests/concurrent_claim.rs`
**Location**: `/home/coding/bead-forge/tests/concurrent_claim.rs`

**Test Functions** (4 tests):
1. `test_concurrent_claim_no_duplicates()` - 20 workers, 20 beads, zero duplicates
2. `test_concurrent_claim_priority_ordering()` - Priority preserved under concurrency
3. `test_concurrent_claim_empty_workspace()` - Empty workspace handling
4. `test_concurrent_claim_stale_reclamation()` - Stale reclamation under concurrency

### File: `tests/claim_race.rs`
**Location**: `/home/coding/bead-forge/tests/claim_race.rs`

**Test Functions** (9 tests):
1. `test_thundering_herd_20_workers_no_duplicates()` - High-concurrency stress test
2. `test_concurrent_claim_priority_preserved()` - Priority under load
3. `test_concurrent_claim_with_dependencies()` - Dependencies respected
4. `test_concurrent_stale_reclamation()` - Stale reclamation under load
5. `test_concurrent_claim_empty_workspace()` - Empty workspace handling
6. `test_rapid_claim_release_cycle()` - Claim -> work -> close cycles
7. `test_concurrent_claim_with_pinned_beads()` - Pinned beads respected
8. `test_concurrent_claim_with_ephemeral_beads()` - Ephemeral beads excluded
9. `test_high_frequency_claim_attempts()` - Stress test with rapid attempts

### File: `tests/autoflush_batch_claim_delete.rs`
**Location**: `/home/coding/bead-forge/tests/autoflush_batch_claim_delete.rs`

**Claim-Specific Tests** (2 tests):
1. `claim_flushes_claimed_bead_state()` - Auto-flush writes claimed state
2. `claim_flush_failure_warns_without_failing()` - Flush failure doesn't fail claim

**Related Context**: This file also tests `batch`, `mitosis`, and `delete` operations

### File: `tests/envelope/claim_stats.rs`
**Location**: `/home/coding/bead-forge/tests/envelope/claim_stats.rs`

**Claim Envelope Tests** (8 tests):
1. `claim_envelope_has_stable_structure()` - Stable envelope structure
2. `claim_envelope_metadata_fields()` - Metadata field presence
3. `claim_envelope_successful_case()` - Successful claim envelope
4. `claim_envelope_empty_workspace()` - Empty workspace envelope
5. `claim_envelope_data_fields()` - Data field validation
6. `claim_envelope_kind_matches_command()` - Kind field matches
7. `claim_envelope_version_always_one()` - Version field validation
8. `claim_envelope_structure_consistency()` - Structure consistency

## Metadata-Related Code

### WorkerMetadata Usage Locations:
1. **`src/claim.rs`** (lines 8-14) - Struct definition
2. **`src/bead_store.rs`** (line 37) - Import and usage
3. **`src/cli/mod.rs`** - CLI command construction
4. **`tests/claim_fallback.rs`** - 9 test functions
5. **`src/claim.rs`** - Core claim logic

## Doctor Module Tests

### File: `src/doctor.rs` (line ~242)
**Test Function**:
- `test_reclaim_stale()` - Stale bead reclamation in doctor operations

## Additional Test Coverage

### Files Indirectly Related to Claiming:
- `tests/velocity_close_integration.rs` - Velocity tracking affects claim scoring
- `tests/fleet_concurrency.rs` - Fleet-level concurrency
- `tests/kill_worker_preserves_beads.rs` - Worker termination handling
- `tests/ready_json_fields.rs` - Ready command JSON output

## Test Categories Summary

### By Functionality:
1. **Basic Claim Operations** (8 tests)
   - Basic claiming, empty workspace, no candidates
   
2. **Concurrency & Race Conditions** (15 tests)
   - Thundering herd, duplicate prevention, priority preservation
   
3. **Fallback & Multi-Workspace** (9 tests)
   - Cross-workspace claiming, fallback behavior
   
4. **Metadata & Worker Info** (10+ tests)
   - WorkerMetadata handling, model/harness tracking
   
5. **Stale Reclamation** (4 tests)
   - TTL-based reclamation, concurrent reclamation
   
6. **Scoring & Priority** (6 tests)
   - Critical path, velocity-aware, downstream impact
   
7. **Envelope/JSON Output** (8 tests)
   - JSON structure, metadata fields, envelope validation
   
8. **Integration/E2E** (6 tests)
   - CLI-level testing, auto-flush behavior

## Key Test Patterns

### WorkerMetadata Construction Pattern:
```rust
let worker_metadata = bead_forge::claim::WorkerMetadata {
    worker_id: "worker-1".to_string(),
    model: Some("claude-sonnet-4-6".to_string()),
    harness: Some("needle".to_string()),
    harness_version: Some("0.5.2".to_string()),
};
```

### Concurrent Claim Pattern:
```rust
let storage = Arc::new(storage);
let claimed_ids = Arc::new(Mutex::new(Vec::new()));

// Spawn N workers, collect claims, verify no duplicates
```

### Claim-Then-Verify Pattern:
```rust
let result = claim(tx, worker, ttl, now, Some(&metadata))?;
assert!(result.is_some());
let issue = storage.get_issue(&claimed.bead_id).unwrap();
assert_eq!(issue.status, Status::InProgress);
```

## Files NOT Directly Related (but tested):
- `tests/test_labels_*.rs` - Label management (separate from claiming)
- `tests/epic_*.rs` - Epic-specific workflows
- `tests/batch_*.rs` - Batch operations (separate claim tests exist)

## Acceptance Criteria Status:
✅ **Complete list of claim-related test modules/functions** - 50+ tests identified
✅ **File paths for each identified test** - All paths documented
✅ **Test names clearly documented** - All test functions listed

## Test Execution Commands:
```bash
# Run all claim-related unit tests
cargo test claim

# Run specific test file
cargo test --test claim_fallback
cargo test --test concurrent_claim
cargo test --test claim_race

# Run envelope tests
cargo test --test envelope::claim_stats
```

---

**Generated**: 2026-07-24  
**Bead**: bf-3jrox2  
**Purpose**: Comprehensive inventory of claim and metadata-related test coverage
