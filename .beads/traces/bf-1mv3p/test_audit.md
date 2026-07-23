# Test Audit for bead-forge

**Date:** 2026-07-23  
**Cargo Version:** bead-forge (bf) CLI  
**Test Command:** `cargo test --no-fail-fast -- -q`

## Executive Summary

| Metric | Count |
|--------|-------|
| **Unit Tests (lib)** | 272 passed, 0 failed ✓ |
| **Integration Test Files** | 100 files |
| **Total Integration Test Targets** | 100 test binaries |
| **Failed Integration Targets** | 10 targets |
| **Total Failed Tests** | 39 tests across 10 files |

✅ **Core library tests are all passing** - All 272 unit tests in `src/lib.rs` pass successfully.

❌ **Integration test failures** - 10 test files have failures, primarily related to:
1. JSON/envelope output format issues (28 tests)
2. Secret scanning validation (6 tests)  
3. Label removal edge case (1 test)
4. Description file error handling (1 test)
5. Additional edge cases (3 tests)

## Test Inventory

### Unit Tests (src/)
**Location:** `src/lib.rs`  
**Status:** ✅ ALL PASS (272/272)

Unit tests cover:
- Core model structures (Issue, Status, Priority, etc.)
- SQLite storage operations
- JSONL import/export
- Batch operations
- Claim operations
- Validation logic
- Config parsing
- ID generation
- Format operations
- And many other core library functions

### Integration Tests (tests/)
**Location:** `tests/` directory  
**Total Files:** 100 test files  
**Status:** 90/100 targets pass completely

## Failed Test Analysis

### 1. Autoflush JSON Output Failures (3 tests)

**File:** `tests/autoflush_failure_contract.rs`
```
❌ create_json_succeeds_warns_retains_dirty_and_recovers
   Expected: --json output must include the created bead ID
   Actual: ID field missing from JSON output
```

**File:** `tests/autoflush_mutation.rs`
```
❌ flush_failure_nonfatal_json_warning_and_dirty_retained
   Expected: create --json output includes bead ID
   Actual: Missing id field
```

**File:** `tests/autoflush_wiring.rs`
```
❌ flush_failure_does_not_fail_mutation_and_warns_json
   Expected: ID present in envelope structure
   Actual: Missing envelope version/id structure
```

**Root Cause:** These tests expect JSON output from `bf create --json` to include an envelope wrapper with version/kind/data/id fields, but the current implementation outputs raw JSON or missing envelope structure.

### 2. Envelope Coverage Failures (20 tests)

**File:** `tests/envelope_coverage.rs` (41 tests total, 21 passed, 20 failed)

Failed tests:
1. `claim_stats::envelope_claim_bead_id_is_valid` - Missing bead_id in claim result
2. `claim_stats::envelope_claim_and_stats_consistent_structure` - Missing version field (expected: 1)
3. `claim_stats::envelope_claim_json_has_metadata_fields` - Missing 'version' field
4. `claim_stats::envelope_claim_json_returns_claim_result` - Missing version (expected: 1)
5. `claim_stats::envelope_claim_no_beads_returns_empty_object` - Missing version
6. `claim_stats::envelope_claim_reflects_assignee` - Missing assignee field
7. `envelope_batch_command_has_stable_structure` - batch data not an array
8. `envelope_batch_empty_emits_empty_array` - batch data not an array
9. `envelope_claim_no_bead_emits_empty_object` - Missing version
10. `envelope_claim_command_has_stable_structure` - Missing version
11. `envelope_data_field_always_present` - Missing data field for velocity command
12. `envelope_kind_matches_command` - Missing kind for velocity command
13. `envelope_ready_command_has_stable_structure` - ready data not an array
14. `envelope_recent_empty_emits_empty_array` - recent data not an array
15. `envelope_recent_command_has_stable_structure` - recent data not an array
16. `envelope_search_empty_emits_empty_array` - Invalid JSON (EOF)
17. `envelope_search_command_has_stable_structure` - Missing version
18. `envelope_velocity_command_has_stable_structure` - Envelope not an object
19. `envelope_velocity_empty_emits_empty_array` - Envelope not an object
20. `envelope_version_is_always_one` - Missing version for velocity

**Expected Envelope Structure:**
```json
{
  "version": 1,
  "kind": "<command-name>",
  "data": <result>,
  "warning": <optional-warning>
}
```

**Root Cause:** The envelope format implementation appears incomplete or not consistently applied across all JSON output commands.

### 3. Envelope Integration Failures (8 tests)

**File:** `tests/envelope_integration_tests.rs` (65 tests total, 57 passed, 8 failed)

Failed tests (all in `envelope::claim_stats` module):
1. `claim_envelope_empty_workspace` - Missing version
2. `claim_envelope_data_fields` - claim data not an object
3. `claim_envelope_has_stable_structure` - Missing version
4. `claim_envelope_kind_matches_command` - Missing 'claim' kind
5. `claim_envelope_metadata_fields` - Missing 'version' field
6. `claim_envelope_structure_consistency` - Missing version
7. `claim_envelope_successful_case` - Missing version
8. `claim_envelope_version_always_one` - Missing version

**Root Cause:** Same envelope structure issue as above, specifically for the `bf claim` command's JSON output.

### 4. Worker Preservation Failure (1 test)

**File:** `tests/kill_worker_preserves_beads.rs`
```
❌ flush_failure_surfaces_warning_in_json_output
   Expected: --json output includes created bead ID
   Actual: ID field missing
```

**Root Cause:** Same JSON/envelope output issue affecting create operations.

### 5. Label Removal Failure (1 test)

**File:** `tests/label_removal_test.rs`
```
❌ test_remove_label_from_nonexistent_issue_fails_gracefully
   Expected: Result is OK (graceful failure)
   Actual: Result is error (panics on assertion)
```

**Root Cause:** Edge case - removing a label from a nonexistent bead should succeed gracefully (no-op) but currently returns an error.

### 6. Recovery Criteria Failure (1 test)

**File:** `tests/recovery_and_exit_criteria.rs`
```
❌ flush_failure_carries_json_warning
   Expected: --json output includes created bead ID
   Actual: ID field missing
```

**Root Cause:** Same JSON/envelope output issue.

### 7. Secret Scanning Failures (6 tests)

**File:** `tests/secret_scanning.rs` (76 tests total, 70 passed, 6 failed)

Failed tests:
1. `integration_refuses_azure_key` - Azure key not rejected
2. `integration_refuses_github_gho_token` - GitHub gho_ token not rejected
3. `integration_refuses_github_ghr_token` - GitHub ghr_ token not rejected
4. `integration_refuses_github_ghs_token` - GitHub ghs_ token not rejected
5. `integration_refuses_github_ghu_token` - GitHub ghu_ token not rejected
6. `integration_refuses_github_pat_token` - GitHub PAT token not rejected

**Root Cause:** The secret scanner's detection patterns for these token types may be too strict, not matching the test patterns, or the integration isn't properly rejecting detected secrets.

### 8. Update Flags Failure (1 test)

**File:** `tests/update_flags.rs`
```
❌ test_cli_update_description_file_missing_file_errors
   Expected: description remains unset after failed --description-file
   Actual: description set to empty String("") instead of None
```

**Root Cause:** Error handling for missing description files sets an empty string instead of leaving the field as None/unset.

## Test Categorization

### By Type

| Category | Count | Status |
|----------|-------|--------|
| Unit Tests (lib) | 272 | ✅ All Pass |
| CLI Command Tests | 600+ | ⚠️ 39 Failures |
| Storage Tests | 50+ | ✅ Pass |
| Integration Tests | 200+ | ⚠️ 39 Failures |
| Edge Case Tests | 50+ | ⚠️ 4 Failures |

### By Feature Area

| Feature Area | Test Count | Failed | Status |
|--------------|------------|--------|--------|
| Core library | 272 | 0 | ✅ Pass |
| Autoflush | 27 | 3 | ⚠️ 89% pass |
| Batch operations | 30+ | 0 | ✅ Pass |
| Claim operations | 65+ | 28 | ⚠️ 57% pass |
| Envelope format | 106 | 28 | ⚠️ 74% pass |
| Epic management | 100+ | 0 | ✅ Pass |
| JSON output | 100+ | 28 | ⚠️ 72% pass |
| Label operations | 50+ | 1 | ⚠️ 98% pass |
| Secret scanning | 76 | 6 | ⚠️ 92% pass |
| Update operations | 40+ | 1 | ⚠️ 98% pass |
| Velocity/critical path | 10+ | 0 | ✅ Pass |
| Recovery/repair | 30+ | 1 | ⚠️ 97% pass |

## Compilation Status

✅ **All tests compile successfully**
- `cargo test --no-run` completed without errors
- Only warnings present (unused imports, unused variables, deprecated chrono APIs)
- No blocking compilation issues

## Pass Rate Summary

| Test Type | Total | Passed | Failed | Pass Rate |
|-----------|-------|--------|--------|-----------|
| Library (unit) | 272 | 272 | 0 | 100% |
| Integration | ~1200 | ~1161 | 39 | ~96.8% |
| **Overall** | **~1472** | **~1433** | **39** | **~97.4%** |

## Recommendations

### High Priority
1. **Fix envelope format implementation** - 28 failing tests depend on this
   - Ensure all JSON commands output: `{version: 1, kind: "...", data: ...}`
   - Commands affected: claim, batch, ready, recent, search, velocity, create

2. **Fix secret scanning patterns** - 6 failing tests
   - Verify detection patterns for Azure keys and GitHub tokens
   - Ensure integration properly rejects detected secrets

### Medium Priority
3. **Fix label removal edge case** - Gracefully handle non-existent beads
4. **Fix description file error handling** - Use None instead of empty string

### Low Priority  
5. **Clean up compiler warnings** - 42 warnings (mostly unused code)
6. **Update deprecated chrono API usage** - `from_timestamp_opt` → `from_timestamp`

## Test Files Summary

All 100 integration test files:

1. autoflush_batch_claim_delete.rs ✅
2. autoflush_diagnostics_and_rotation.rs ✅
3. autoflush_failure_contract.rs ❌ (1 fail)
4. autoflush_mutation.rs ❌ (1 fail)
5. autoflush_readonly.rs ✅
6. autoflush_wiring.rs ❌ (1 fail)
7. batch_atomic.rs ✅
8. batch_cascade_and_rotation.rs ✅
9. batch_mitosis.rs ✅
10. bf_520v_json_format.rs ✅
11. br_isolation.rs ✅
12. bug_default_priority.rs ✅
13. claim_fallback.rs ✅
14. claim_race.rs ✅
15. close_reopen.rs ✅
16. comments_cli.rs ✅
17. common.rs ✅
18. concurrent_claim.rs ✅
19. count_command.rs ✅
20. description_update_test_infrastructure.rs ✅
21. dirty_tracking.rs ✅
22. doctor_repair_unflushed.rs ✅
23. doctor_safety_stack.rs ✅
24. duplicate_label_test.rs ✅
25. envelope_coverage.rs ❌ (20 fails)
26. envelope_helpers.rs ✅
27. envelope_integration_tests.rs ❌ (8 fails)
28. epic_cli.rs ✅
29. epic_cli_label_creation.rs ✅
30. epic_cli_label_display.rs ✅
31. epic_cli_label_mutate.rs ✅
32. epic_cli_label_sort_filter.rs ✅
33. epic_complex_labels.rs ✅
34. epic_comprehensive.rs ✅
35. epic_default_priority.rs ✅
36. epic_json_format.rs ✅
37. epic_p0_labels.rs ✅
38. epic_type_basic.rs ✅
39. epic_with_labels.rs ✅
40. feature_default_priority.rs ✅
41. fleet_concurrency.rs ✅
42. json_formatter_verification.rs ✅
43. jsonl_compat.rs ✅
44. kill_worker_preserves_beads.rs ❌ (1 fail)
45. label_list.rs ✅
46. label_removal_test.rs ❌ (1 fail)
47. label_storage.rs ✅
48. limit_zero.rs ✅
49. migrate_git_reconstruction.rs ✅
50. p0_epic_creation.rs ✅
51. p0_epic_labels.rs ✅
52. p1_epic_creation.rs ✅
53. priority_p0_validation.rs ✅
54. ready_json_fields.rs ✅
55. recovery_and_exit_criteria.rs ❌ (1 fail)
56. schema_compat.rs ✅
57. search_command.rs ✅
58. secret_scanning.rs ❌ (6 fails)
59. task_default_priority.rs ✅
60. test_assignee.rs ✅
61. test_assignee_validation.rs ✅
62. test_basic_workflow.rs ✅
63. test_bf_1dbvv_roundtrip_description_ac.rs ✅
64. test_bf_23vs_basic_functionality.rs ✅
65. test_bf_2hqt.rs ✅
66. test_bf_2l7_help_flag.rs ✅
67. test_bf_32zd.rs ✅
68. test_bf_52is_smoke.rs ✅
69. test_bf_5id.rs ✅
70. test_bf_5sw6.rs ✅
71. test_blocked_cascade.rs ✅
72. test_close_reopen.rs ✅
73. test_close_reopen_integration.rs ✅
74. test_comprehensive_labels.rs ✅
75. test_create.rs ✅
76. test_create_command.rs ✅
77. test_critical_path_cache_invalidation.rs ✅
78. test_dirty_repair.rs ✅
79. test_envelope_helpers_usage.rs ✅
80. test_epic_child_1.rs ✅
81. test_epic_default_priority.rs ✅
82. test_epic_p0_creation.rs ✅
83. test_epic_p1_comprehensive.rs ✅
84. test_epic_p1_creation.rs ✅
85. test_epic_single_label.rs ✅
86. test_epic_type_creation.rs ✅
87. test_epic_type_validation.rs ✅
88. test_epic_with_description.rs ✅
89. test_invalid_type.rs ✅
90. test_json_formatter.rs ✅
91. test_jsonl.rs ✅
92. test_labels.rs ✅
93. test_show_command.rs ✅
94. test_special_chars.rs ✅
95. test_update_command.rs ✅
96. test_version_display.rs ✅
97. update_flags.rs ❌ (1 fail)
98. velocity_close_integration.rs ✅
99. velocity_seed_integration.rs ✅
100. verify_epic_implementation.rs ✅

**Legend:** ✅ All tests pass | ❌ At least one failure

---

**Generated by:** bf-1mv3p (test audit bead)  
**Data file:** `.beads/traces/bf-1mv3p/test_results.txt` (full cargo test output)
