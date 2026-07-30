# Integration Test Run Summary - bf-2bxiw

## Execution Date
2026-07-23

## Task
Run full integration test suite (105 test files) in manageable batches to identify failures.

## Results Summary

### Test Statistics
- **Total test files executed:** 105
- **Library unit tests:** 272 passed ✅
- **Total integration tests:** ~350+ tests
- **Pass rate:** ~89% (~311+ passed / ~39 failed)
- **Total execution time:** ~21 seconds

### Test Batches Run

#### Batch 1: CLI Command Tests (28s)
- ✅ 33 passed
- ❌ 1 failed: `test_cli_update_description_file_missing_file_errors`

#### Batch 2: Label Tests Part 1 (13s)
- ✅ ALL PASSED

#### Batch 3: Label Tests Part 2 (10s)
- ✅ ALL PASSED

#### Batch 4: Autoflush Tests (10s)
- ✅ ALL PASSED

#### Batch 5: Doctor & Recovery (7s)
- ✅ 4 passed
- ❌ 1 failed: `flush_failure_carries_json_warning`

#### Batch 6: Batch & Claim (9s)
- ✅ ALL PASSED

#### Batch 7: Envelope Tests (7s)
- ✅ 11 passed
- ❌ 30 failed (envelope_coverage.rs)

#### Batch 8: Priority & Default (4s)
- ✅ ALL PASSED

#### Batch 9: JSON Format (4s)
- ✅ ALL PASSED

#### Batch 10: Functional Tests (8s)
- ✅ ALL PASSED

#### Batch 11: Secret Scanning (16s)
- ✅ 70 passed
- ❌ 6 failed (secret rejection not working)

#### Batch 12: Velocity & Migration (10s)
- ✅ 9 passed
- ❌ 1 failed: `flush_failure_surfaces_warning_in_json_output`

#### Batch 13: Remaining Tests (3s)
- ✅ ALL PASSED

#### Batch 14: br Isolation (3s)
- ✅ ALL PASSED

## Detailed Failures

### 1. CLI Command Tests (1 failure)
**File:** tests/update_flags.rs:909
- **Test:** `test_cli_update_description_file_missing_file_errors`
- **Issue:** After failed --description-file, got empty String("") instead of unset description

### 2. Doctor & Recovery (1 failure)
**File:** tests/recovery_and_exit_criteria.rs:346
- **Test:** `flush_failure_carries_json_warning`
- **Issue:** --json output missing created id field

### 3. Envelope Coverage (30 failures) - MAJOR
**File:** tests/envelope_coverage.rs
**Pattern:** Envelope format not being applied to JSON outputs

Claim & Stats (11):
- envelope_claim_and_stats_consistent_structure
- envelope_claim_bead_id_is_valid
- envelope_claim_json_has_metadata_fields
- envelope_claim_json_returns_claim_result
- envelope_claim_no_beads_returns_empty_object
- envelope_claim_reflects_assignee
- envelope_stats_empty_returns_zero_stats
- envelope_stats_fields_are_numeric
- envelope_stats_json_has_metadata_fields
- envelope_stats_json_returns_stats_result
- envelope_stats_reflects_bead_count

Command Structure (7):
- envelope_batch_command_has_stable_structure
- envelope_batch_empty_emits_empty_array
- envelope_claim_command_has_stable_structure
- envelope_claim_no_bead_emits_empty_object
- envelope_data_field_always_present
- envelope_kind_matches_command
- envelope_list_command_has_stable_structure

List/Show (3):
- envelope_list_items_match_show_structure
- envelope_list_json_has_metadata_fields
- envelope_list_json_returns_array_data

Ready/Search/Recent/Stats/Velocity (5):
- envelope_ready_command_has_stable_structure
- envelope_recent_command_has_stable_structure
- envelope_recent_empty_emits_empty_array
- envelope_search_command_has_stable_structure
- envelope_search_empty_emits_empty_array

Velocity (2):
- envelope_velocity_command_has_stable_structure
- envelope_velocity_empty_emits_empty_array

Stats/Version (2):
- envelope_stats_command_has_stable_structure
- envelope_version_is_always_one

### 4. Kill Worker (1 failure)
**File:** tests/kill_worker_preserves_beads.rs:379
- **Test:** `flush_failure_surfaces_warning_in_json_output`
- **Issue:** --json output missing created id field

### 5. Secret Scanning (6 failures)
**File:** tests/secret_scanning.rs
**Pattern:** Secret rejection not enforced in create_issue()

- integration_refuses_azure_key (line 1159)
- integration_refuses_github_gho_token (line 1180)
- integration_refuses_github_ghr_token (line 1246)
- integration_refuses_github_ghs_token (line 1224)
- integration_refuses_github_ghu_token (line 1202)
- integration_refuses_github_pat_token (line 943)

## Passing Categories

✅ **Autoflush tests** (6 files, all passed)
✅ **Batch & Claim tests** (3 files, all passed)
✅ **Label tests** (12 files, all passed)
✅ **Priority/Default tests** (7 files, all passed)
✅ **JSON format tests** (6 files, all passed)
✅ **br isolation tests** (3 files, all passed)
✅ **Library unit tests** (272 tests, all passed)
✅ **Many other functional tests** (30+ files)

## Conclusions

1. **Core functionality is healthy** - bead creation, updates, labels, dependencies, batch operations, claiming all work correctly
2. **Envelope format is the main issue** - 30 failures indicate JSON outputs not using envelope wrapper (version/kind/data fields)
3. **Secret scanning not enforced** - 6 failures show create_issue() accepts secrets when it should reject them
4. **Minor JSON field issues** - 2 failures show missing IDs in error/warning JSON outputs

## Recommendations

1. Fix envelope format implementation to wrap all --json outputs
2. Enable secret scanning rejection in create_issue()
3. Ensure error/warning JSON outputs include required fields
