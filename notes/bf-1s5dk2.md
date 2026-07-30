# Claim-Related Test Cases Documentation

Generated for bead `bf-1s5dk2`

## Overview
This document catalogs all claim-related test cases in the bead-forge codebase, organized by test file and category.

## Core Claim Functionality Tests

### 1. `src/claim.rs` - Unit Tests
**File:** `src/claim.rs`

| Test Function | Line | Purpose |
|---------------|------|---------|
| `test_claim_basic` | 727 | Tests basic claim functionality |
| `test_claim_no_candidates` | 755 | Tests claim behavior when no beads available |
| `test_claim_reclaims_stale` | 767 | Tests automatic reclamation of stale claims |
| `test_concurrent_claim_no_double_claim` | 813 | Tests that concurrent claims don't double-claim |
| `test_critical_path_bonus_in_claim` | 882 | Tests critical path scoring in claim selection |

### 2. `src/bead_store.rs` - Claim Bead Storage Tests
**File:** `src/bead_store.rs`

| Test Function | Line | Purpose |
|---------------|------|---------|
| `test_claim_bead_basic` | 313 | Tests basic bead claiming via storage layer |
| `test_claim_bead_priority_ordering` | 343 | Tests that higher priority beads are claimed first |
| `test_claim_bead_empty_workspace` | 370 | Tests claim behavior with empty workspace |

## Concurrent Claim Tests

### 3. `tests/concurrent_claim.rs` - Atomic Concurrent Claiming
**File:** `tests/concurrent_claim.rs`

| Test Function | Line | Purpose |
|---------------|------|---------|
| `test_concurrent_claim_no_duplicates` | 51 | Verifies no duplicate claims under concurrent load |
| `test_concurrent_claim_priority_ordering` | 130 | Tests priority ordering under concurrent access |
| `test_concurrent_claim_empty_workspace` | 189 | Tests concurrent claim with no beads available |
| `test_concurrent_claim_stale_reclamation` | 219 | Tests stale bead reclamation under concurrency |

### 4. `tests/claim_race.rs` - Race Condition Tests
**File:** `tests/claim_race.rs`

| Test Function | Line | Purpose |
|---------------|------|---------|
| `test_thundering_herd_20_workers_no_duplicates` | 18 | Main thundering herd test with 20 workers |
| `test_concurrent_claim_priority_preserved` | 127 | Tests priority preservation under race conditions |
| `test_concurrent_claim_with_dependencies` | 209 | Tests claiming with dependency constraints |
| `test_concurrent_claim_empty_workspace` | 367 | Tests concurrent claim with empty workspace |
| `test_rapid_claim_release_cycle` | 418 | Tests rapid claim/release cycles |
| `test_concurrent_claim_with_pinned_beads` | 492 | Tests claiming with pinned beads |
| `test_concurrent_claim_with_ephemeral_beads` | 557 | Tests claiming with ephemeral beads |
| `test_high_frequency_claim_attempts` | 622 | Tests high-frequency claim attempts |

### 5. `tests/fleet_concurrency.rs` - Fleet-Level Concurrency
**File:** `tests/fleet_concurrency.rs`

| Test Function | Line | Purpose |
|---------------|------|---------|
| `fleet_concurrent_claims_no_double_claim` | 171 | Tests fleet-level concurrent claiming |
| `parse_claimed_id` | 62 | Helper to parse claimed bead IDs |

## Fallback Mechanism Tests

### 6. `tests/claim_fallback.rs` - Fallback Behavior
**File:** `tests/claim_fallback.rs`

| Test Function | Line | Purpose |
|---------------|------|---------|
| `test_claim_fallback_any_exhausted_primary_workspace` | 9 | Tests fallback when primary workspace exhausted |
| `test_claim_fallback_any_primary_has_beads_no_fallback` | 61 | Tests no fallback when primary has beads |
| `test_claim_fallback_any_empty_all_workspaces` | 108 | Tests fallback when all workspaces empty |
| `test_claim_fallback_any_selects_from_available_workspace` | 140 | Tests workspace selection in fallback |
| `test_claim_fallback_any_with_dependencies` | 179 | Tests fallback with dependency constraints |
| `test_claim_fallback_any_pinned_beads_respected` | 235 | Tests that pinned beads are respected in fallback |
| `test_claim_fallback_any_multiple_workspaces` | 289 | Tests fallback across multiple workspaces |
| `test_cli_claim_fallback_any_exhausted_workspace` | 329 | Tests CLI-level fallback behavior |
| `test_claim_fallback_to_1800s_when_velocity_stats_empty` | 442 | Tests fallback to default TTL when velocity stats unavailable |

## Format Tests

### 7. `tests/json_formatter_verification.rs` - JSON Format
**File:** `tests/json_formatter_verification.rs`

| Test Function | Line | Purpose |
|---------------|------|---------|
| `claim_emits_single_object` | 214 | Verifies claim emits single JSON object |
| `claim_dry_run_emits_preview_object` | 234 | Tests dry-run claim emits preview object |
| `claim_empty_emits_empty_object` | 255 | Tests claim emits empty object when no beads |

### 8. `tests/envelope_coverage.rs` - Envelope Format Tests
**File:** `tests/envelope_coverage.rs`

| Test Function | Line | Purpose |
|---------------|------|---------|
| `envelope_claim_command_has_stable_structure` | 229 | Tests envelope structure stability |
| `envelope_claim_no_bead_emits_empty_object` | 247 | Tests envelope empty object behavior |
| `envelope_claim_json_returns_claim_result` | 853 | Tests envelope returns claim result |
| `envelope_claim_json_has_metadata_fields` | 880 | Tests envelope metadata fields |
| `envelope_claim_no_beads_returns_empty_object` | 912 | Tests empty behavior in envelope |
| `envelope_claim_and_stats_consistent_structure` | 1005 | Tests structure consistency |
| `envelope_claim_bead_id_is_valid` | 1046 | Tests bead ID validity in envelope |
| `envelope_claim_reflects_assignee` | 1089 | Tests assignee reflection in envelope |

### 9. `tests/envelope/claim_stats.rs` - Claim Envelope Tests
**File:** `tests/envelope/claim_stats.rs`

| Test Function | Line | Purpose |
|---------------|------|---------|
| `claim_envelope_has_stable_structure` | 255 | Tests envelope structure stability |
| `claim_envelope_metadata_fields` | 275 | Tests envelope metadata fields |
| `claim_envelope_successful_case` | 313 | Tests successful claim case |
| `claim_envelope_empty_workspace` | 337 | Tests empty workspace case |
| `claim_envelope_data_fields` | 353 | Tests data field structure |
| `claim_envelope_kind_matches_command` | 381 | Tests kind field matches command |
| `claim_envelope_version_always_one` | 397 | Tests version field is always 1 |
| `claim_envelope_structure_consistency` | 413 | Tests overall structure consistency |

### 10. `tests/envelope/text_format.rs` - Text Format Tests
**File:** `tests/envelope/text_format.rs`

| Test Function | Line | Purpose |
|---------------|------|---------|
| `claim_envelope_outputs_plain_text` | 178 | Tests plain text output |
| `claim_envelope_output_matches_no_envelope` | 195 | Tests output matches non-envelope |
| `claim_envelope_empty_workspace` | 218 | Tests empty workspace in text format |
| `claim_envelope_outputs_bead_id` | 231 | Tests bead ID output |
| `claim_envelope_structure_consistency` | 243 | Tests structure consistency |

### 11. `tests/envelope/toon_format.rs` - Toon Format Tests
**File:** `tests/envelope/toon_format.rs`

| Test Function | Line | Purpose |
|---------------|------|---------|
| `claim_envelope_outputs_plain_text` | 178 | Tests plain text output in toon format |
| `claim_envelope_output_matches_no_envelope` | 195 | Tests output matches non-envelope |
| `claim_envelope_empty_workspace` | 218 | Tests empty workspace case |
| `claim_envelope_outputs_bead_id` | 231 | Tests bead ID output |
| `claim_envelope_structure_consistency` | 243 | Tests structure consistency |

### 12. `tests/envelope/non_json.rs` - Non-JSON Format Tests
**File:** `tests/envelope/non_json.rs`

| Test Function | Line | Purpose |
|---------------|------|---------|
| `text_claim_outputs_plain_text_not_json` | 92 | Tests text format outputs plain text |
| `toon_claim_outputs_plain_text_not_json` | 161 | Tests toon format outputs plain text |

### 13. `src/format/envelope.rs` - Envelope Format Unit Tests
**File:** `src/format/envelope.rs`

| Test Function | Line | Purpose |
|---------------|------|---------|
| `claim_command_emits_result_object` | 425 | Tests claim command emits result object |
| `claim_json_envelope_has_stable_structure` | 888 | Tests JSON envelope structure stability |
| `claim_json_envelope_metadata_fields_present` | 911 | Tests metadata fields presence |
| `claim_json_envelope_successful_claim_case` | 940 | Tests successful claim case |
| `claim_json_envelope_empty_when_no_bead_available` | 968 | Tests empty behavior |
| `claim_json_envelope_roundtrip_serialization` | 984 | Tests serialization roundtrip |

### 14. `src/format/json.rs` - JSON Format Unit Tests
**File:** `src/format/json.rs`

| Test Function | Line | Purpose |
|---------------|------|---------|
| `claim_dry_run_emits_only_preview_keys` | 224 | Tests dry-run emits only preview keys |
| `claim_single_workspace_omits_workspace_key` | 248 | Tests workspace key omission |
| `no_claim_is_empty_object` | 268 | Tests empty object on no claim |

## Integration Tests

### 15. `tests/dirty_tracking.rs` - State Tracking Tests
**File:** `tests/dirty_tracking.rs`

| Test Function | Line | Purpose |
|---------------|------|---------|
| `claim_marks_dirty` | 175 | Tests that claim operation marks workspace dirty |

### 16. `tests/autoflush_batch_claim_delete.rs` - Auto-Flush Tests
**File:** `tests/autoflush_batch_claim_delete.rs`

| Test Function | Line | Purpose |
|---------------|------|---------|
| `claim_flushes_claimed_bead_state` | 224 | Tests claim flushes claimed bead state |
| `claim_flush_failure_warns_without_failing` | 247 | Tests claim flush failure handling |

### 17. `tests/test_critical_path_cache_invalidation.rs` - Cache Tests
**File:** `tests/test_critical_path_cache_invalidation.rs`

| Test Function | Line | Purpose |
|---------------|------|---------|
| `test_critical_path_cache_invalidated_on_claim` | 6 | Tests cache invalidation on claim |
| `test_critical_path_cache_invalidated_on_reclaim` | 74 | Tests cache invalidation on reclaim |

### 18. `tests/test_envelope_helpers_usage.rs` - Helper Tests
**File:** `tests/test_envelope_helpers_usage.rs`

| Test Function | Line | Purpose |
|---------------|------|---------|
| `example_validate_claim_envelope_with_warning` | 96 | Tests claim envelope validation with warnings |

## Summary

**Total claim-related test files:** 18
**Total claim-related test functions:** 71

### Test Categories:
- **Core Functionality:** 8 tests
- **Concurrency/Race Conditions:** 11 tests  
- **Fallback Mechanism:** 9 tests
- **Format Tests (JSON/Envelope/Text/Toon):** 37 tests
- **Integration/Auto-flush:** 6 tests

### Test Coverage Areas:
1. Basic claim functionality and empty workspace handling
2. Priority ordering and scoring (critical path)
3. Concurrent access and race conditions (thundering herd scenarios)
4. Stale bead reclamation and TTL handling
5. Cross-workspace fallback mechanism
6. Dependency and pinned bead constraints
7. JSON/Envelope/Text/Toon format outputs
8. Cache invalidation
9. Dirty tracking and auto-flush behavior
10. Fleet-level concurrency

All tests use either direct storage layer calls or CLI invocation patterns, ensuring both library-level and end-to-end claim behavior is tested.
