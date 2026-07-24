# Claim-Related Test Filters

This document describes cargo test filters for isolating claim-related tests in the bead-forge project.

## All Claim Tests (Most Comprehensive)

```bash
cargo test --lib claim
```

**Runs:** 23 tests covering all claim-related functionality
- Core claim logic (claim.rs)
- Reclaim stale tests (doctor.rs)
- Format envelope tests for claim output
- JSON format tests for claim commands

## Core Claim Functionality Tests

```bash
cargo test --lib 'claim::tests::test_claim'
```

**Runs:** 3 tests
- `test_claim_basic` - Basic claim operation
- `test_claim_no_candidates` - Claim when no beads available
- `test_claim_reclaims_stale` - Reclaim stale in_progress beads

## Ready Candidates Tests

```bash
cargo test --lib 'ready'
```

**Runs:** 4 tests
- `test_get_ready_candidates_limit_zero_returns_all` - Unlimited limit behavior
- `test_get_ready_candidates_respects_limit` - Limited candidates
- `test_ready_includes_zero_dependency_open_beads_bf_1nprw` - Regression test for bf-1nprw
- `ready_command_empty_returns_array` - Format test for empty ready output

## Concurrent Claim Tests

```bash
cargo test --lib 'concurrent_claim'
```

**Runs:** 1 test
- `test_concurrent_claim_no_double_claim` - Prevents double-claiming under concurrency

## Critical Path Claim Tests

```bash
cargo test --lib 'critical_path'
```

**Runs:** 6 tests (including general critical path tests)
- `test_critical_path_bonus_in_claim` - Critical path bonus scoring
- `test_critical_path_zero_float_outranks_high_priority` - Zero float outranks high priority
- Plus general critical path computation tests

## Reclaim Stale Tests

```bash
cargo test --lib 'reclaim'
```

**Runs:** 2 tests
- `test_claim_reclaims_stale` - Core reclaim logic
- `test_reclaim_stale` - Doctor module reclaim test

## Claim Format Tests

### JSON Envelope Format Tests

```bash
cargo test --lib 'claim_json_envelope'
```

**Runs:** 5 tests
- `test_claim_json_envelope_empty_when_no_bead_available`
- `test_claim_json_envelope_has_stable_structure`
- `test_claim_json_envelope_metadata_fields_present`
- `test_claim_json_envelope_roundtrip_serialization`
- `test_claim_json_envelope_successful_claim_case`

### Stats Format Tests

```bash
cargo test --lib 'stats_json_envelope'
```

**Runs:** 3 tests
- `test_stats_json_envelope_aggregate_counts`
- `test_stats_json_envelope_has_stable_structure`
- `test_stats_json_envelope_metadata_fields_present`

### Command Format Tests

```bash
cargo test --lib 'claim_dry_run_emits_only_preview_keys'
cargo test --lib 'claim_single_workspace_omits_workspace_key'
cargo test --lib 'no_claim_is_empty_object'
cargo test --lib 'claim_command_emits_result_object'
```

**Runs:** Individual format validation tests

## Specific Regression Tests

### bf-wre (Completed Status Blocker)

```bash
cargo test --lib 'test_completed_status_blocker_unblocks_dependent'
```

**Purpose:** Ensures blockers with status="completed" satisfy dependencies

### bf-1nprw (Ready Includes Zero-Dependency Beads)

```bash
cargo test --lib 'test_ready_includes_zero_dependency_open_beads_bf_1nprw'
```

**Purpose:** Ensures standalone open beads appear in ready output

## Running Specific Test Modules

### All Tests in claim.rs

```bash
cargo test --lib 'claim::tests'
```

### All Tests in doctor.rs

```bash
cargo test --lib 'doctor::tests'
```

### All Tests in format modules

```bash
cargo test --lib 'format::envelope::claim_stats'
cargo test --lib 'format::envelope::tests'
cargo test --lib 'format::json::tests'
```

## Quick Reference

| Goal | Command |
|------|---------|
| **All claim tests** | `cargo test --lib claim` |
| **Core claim logic** | `cargo test --lib 'claim::tests::test_claim'` |
| **Ready candidates** | `cargo test --lib ready` |
| **Concurrent claims** | `cargo test --lib concurrent_claim` |
| **Critical path** | `cargo test --lib critical_path` |
| **Reclaim stale** | `cargo test --lib reclaim` |
| **Format tests** | `cargo test --lib 'claim_json_envelope'` |
| **Regression tests** | `cargo test --lib -- test_completed_status_blocker test_ready_includes_zero_dependency_open_beads` |

## Test Categories Summary

1. **Core Functionality (10 tests):** Basic claim, no candidates, reclaim, concurrent, critical path
2. **Ready Candidates (4 tests):** Limit handling, zero-dependency handling
3. **Format/Output (13 tests):** JSON envelope structure, stats, command output
4. **Regression (2 tests):** bf-wre, bf-1nprw

**Total: 23 claim-related tests**

## Practical Examples

### Example 1: Quick Claim Test Suite
```bash
# Run all claim tests for a comprehensive check
cargo test --lib claim
```

### Example 2: Isolated Core Logic Testing
```bash
# Test only core claim functionality (fastest subset)
cargo test --lib 'claim::tests::test_claim'
```

### Example 3: Regression Testing
```bash
# Run only regression tests to ensure no bug reintroduction
cargo test --lib -- test_completed_status_blocker test_ready_includes_zero_dependency_open_beads
```

### Example 4: Format Validation
```bash
# Test claim JSON output format stability
cargo test --lib 'claim_json_envelope'
```

### Example 5: Concurrency Testing
```bash
# Test concurrent claim prevention (important for multi-agent scenarios)
cargo test --lib 'concurrent_claim'
```

## Notes

- All filters use `--lib` to test library code only (not integration tests)
- Pattern matching is substring-based: `claim` matches any test name containing "claim"
- Use full test names for exact matches: `cargo test --lib -- test_claim_basic`
- Multiple specific tests: `cargo test --lib -- test1 test2 test3`
- Results show X passed; Y filtered out - Y is the number of non-matching tests
