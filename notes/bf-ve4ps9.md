# Clear-Assignee Test Recommendations

**Bead ID:** bf-ve4ps9  
**Date:** 2026-08-05  
**Purpose:** Specific test recommendations for clear-assignee functionality gaps

## Overview

Based on the clear-assignee functionality inventory (bf-4xu8ib) and test verification (bf-31dgab), this document provides specific test recommendations for each identified coverage gap.

---

## Gap 1: Batch Operations with Clear-Assignee

**Priority:** HIGH  
**Gap:** No test coverage for `bf batch` with `--clear-assignee`

### Test Recommendations

#### 1.1 Batch Create with Clear-Assignee
**File:** `tests/test_batch.rs` (new file) or extend existing `tests/batch.rs`

**Test Name:** `test_batch_clear_assignee_on_create()`

**What it should verify:**
- Create multiple beads via `bf batch` with `--clear-assignee` flag
- Verify all created beads have `NULL` assignee in database
- Verify no `assignee_changed` events are generated (initial state is NULL)
- Verify JSONL export contains `"assignee": null` for all beads

**Priority:** HIGH - Batch operations are critical for NEEDLE fleet automation

#### 1.2 Batch Update with Clear-Assignee
**Test Name:** `test_batch_clear_assignee_on_update()`

**What it should verify:**
- Create beads with assignees set
- Use `bf batch` to update multiple beads with `--clear-assignee`
- Verify all affected beads have `NULL` assignee in database
- Verify `assignee_changed` events generated for each bead with correct old_value
- Verify JSONL export reflects cleared assignees

**Priority:** HIGH - Core workflow for fleet worker handoff

#### 1.3 Batch Mixed Operations (Assign + Clear)
**Test Name:** `test_batch_mixed_assignee_operations()`

**What it should verify:**
- Create beads with various assignees
- Use `bf batch` to simultaneously:
  - Set assignee on some beads (`--assignee "worker1"`)
  - Clear assignee on others (`--clear-assignee`)
  - Leave assignee unchanged on rest
- Verify each bead has correct final assignee state
- Verify `assignee_changed` events only for beads with actual changes

**Priority:** MEDIUM - Edge case but tests batch operation granularity

---

## Gap 2: Combined Operations with Clear-Assignee

**Priority:** HIGH  
**Gap:** Limited testing of `--clear-assignee` with other flags

### Test Recommendations

#### 2.1 Clear-Assignee with Status Change
**Test Name:** `test_update_clear_assignee_with_status()`

**What it should verify:**
- Create bead with assignee
- Run `bf update <id> --clear-assignee --status blocked`
- Verify assignee cleared AND status changed in single transaction
- Verify both `assignee_changed` and `status_changed` events generated
- Verify atomicity - both changes succeed or both fail

**Priority:** HIGH - Common workflow pattern

#### 2.2 Clear-Assignee with Priority Change
**Test Name:** `test_update_clear_assignee_with_priority()`

**What it should verify:**
- Create bead with assignee
- Run `bf update <id> --clear-assignee --priority p2`
- Verify assignee cleared AND priority changed
- Verify both events generated correctly

**Priority:** MEDIUM - Less common but should work

#### 2.3 Clear-Assignee with Multiple Field Updates
**Test Name:** `test_update_clear_assignee_with_multiple_fields()`

**What it should verify:**
- Create bead with assignee
- Run `bf update <id> --clear-assignee --status in-progress --priority p1 --due-at 2026-08-10`
- Verify all fields updated correctly
- Verify all corresponding events generated
- Verify transaction atomicity

**Priority:** MEDIUM - Stress test for multi-field updates

#### 2.4 Clear-Assignee with Dependency Changes
**Test Name:** `test_update_clear_assignee_with_dependency_ops()`

**What it should verify:**
- Create bead with assignee
- Run `bf update <id> --clear-assignee --dep-add-blocker bf-other`
- Verify assignee cleared AND dependency added
- Verify both `assignee_changed` and `dependency_added` events

**Priority:** HIGH - Dependency operations are critical for workflow

---

## Gap 3: Error Handling for Clear-Assignee

**Priority:** MEDIUM  
**Gap:** No specific tests for clear-assignee on non-existent beads

### Test Recommendations

#### 3.1 Clear-Assignee on Non-Existent Bead
**Test Name:** `test_update_clear_assignee_non_existent_bead()`

**What it should verify:**
- Run `bf update bf-nonexistent --clear-assignee`
- Verify command fails with appropriate error message
- Verify error message is user-friendly (not internal SQLite error)
- Verify no database changes occurred

**Priority:** MEDIUM - Basic error handling

#### 3.2 Clear-Assignee on Already-Unassigned Bead
**Test Name:** `test_update_clear_assignee_already_unassigned()`

**What it should verify:**
- Create bead without assignee
- Run `bf update <id> --clear-assignee`
- Verify command succeeds (idempotent operation)
- Verify no `assignee_changed` event generated (no actual change)
- Verify bead remains in valid state

**Priority:** LOW - Idempotency is good to verify

#### 3.3 Clear-Assignee with Concurrent Modification
**Test Name:** `test_update_clear_assignee_concurrent_modification()`

**What it should verify:**
- Create bead with assignee
- Simulate concurrent update (modify bead in database between read and write)
- Run `bf update <id> --clear-assignee`
- Verify proper conflict detection/resolution
- Verify database consistency maintained

**Priority:** LOW - Edge case, transactions should handle this

#### 3.4 Clear-Assignee on Closed Bead
**Test Name:** `test_update_clear_assignee_on_closed_bead()`

**What it should verify:**
- Create bead with assignee, close it
- Run `bf update <id> --clear-assignee`
- Verify behavior - should this succeed or fail?
- Verify event handling for closed bead state

**Priority:** MEDIUM - Tests state transition constraints

---

## Gap 4: JSONL Export Serialization

**Priority:** HIGH  
**Gap:** No verification that cleared assignee serializes correctly to JSONL

### Test Recommendations

#### 4.1 JSONL Export After Clear-Assignee
**Test Name:** `test_jsonl_export_after_clear_assignee()`

**What it should verify:**
- Create bead with assignee
- Run `bf update <id> --clear-assignee`
- Run `bf sync --flush-only`
- Read JSONL file, verify cleared assignee serializes as `null` not `""`
- Verify no `"assignee": ""` empty strings in export

**Priority:** HIGH - Critical for data contract compliance

#### 4.2 JSONL Import of Cleared Assignee
**Test Name:** `test_jsonl_import_cleared_assignee()`

**What it should verify:**
- Create JSONL file with bead having `"assignee": null`
- Run `bf sync --import`
- Verify bead imported with `NULL` assignee in database
- Verify round-trip consistency (export → import → export)

**Priority:** HIGH - Tests data contract compliance

#### 4.3 JSONL Envelope Format with Cleared Assignee
**Test Name:** `test_jsonl_envelope_clear_assignee()`

**What it should verify:**
- Create bead with assignee
- Run `bf update <id> --clear-assignee --json`
- Verify JSON output shows `"assignee": null` in envelope
- Verify envelope format wrapper doesn't interfere with null serialization

**Priority:** MEDIUM - Tests JSON API contract

#### 4.4 JSONL Export Consistency After Multiple Operations
**Test Name:** `test_jsonl_export_clear_assignee_multiple_ops()`

**What it should verify:**
- Create bead with assignee
- Run `bf update <id> --clear-assignee`
- Run `bf update <id> --assignee "worker2"`
- Run `bf update <id> --clear-assignee` again
- Run `bf sync --flush-only`
- Verify JSONL shows final state correctly (null assignee)
- Verify all intermediate events captured correctly

**Priority:** MEDIUM - Tests serialization of state transitions

---

## Additional Recommendations

### High Priority

#### A.1 Reopen Side Effect Verification
**Test Name:** `test_reopen_clears_assignee_comprehensive()`

**What it should verify:**
- Create bead with assignee
- Close bead
- Reopen bead
- Verify assignee automatically cleared
- Verify `assignee_changed` event generated with old_value but no new_value
- Verify this behavior is consistent with contract documentation

**Priority:** HIGH - Critical for workflow, should verify documented behavior

#### A.2 Storage Layer Clear-Assignee Edge Cases
**Test Name:** `test_storage_clear_assignee_whitespace_variations()`

**What it should verify:**
- Test assignee clearing with various whitespace inputs:
  - Empty string `""`
  - Single space `" "`
  - Multiple spaces `"   "`
  - Tabs and newlines `"\t\n"`
- Verify all normalize to `NULL` in database
- Verify `assignee_changed` events show `new_value: null`

**Priority:** HIGH - Tests data contract for whitespace handling

### Medium Priority

#### B.1 Clear-Assignee Event Tracking
**Test Name:** `test_clear_assignee_event_tracking()`

**What it should verify:**
- Create bead with assignee
- Run `bf update <id> --clear-assignee --actor "system-test"`
- Verify `assignee_changed` event:
  - Contains correct `old_value` (original assignee)
  - Contains correct `new_value` (null)
  - Contains correct `actor`
  - Contains timestamp
- Verify event queryable via `bf events` command

**Priority:** MEDIUM - Tests audit trail functionality

#### B.2 Clear-Assignee in Different Output Formats
**Test Name:** `test_clear_assignee_output_formats()`

**What it should verify:**
- Create bead with assignee
- Run `bf update <id> --clear-assignee`
- Test output in all formats:
  - Default text format
  - JSON format
  - Toon format (if supported)
  - Envelope format
- Verify cleared assignee displayed correctly in each format

**Priority:** MEDIUM - Tests UX consistency

---

## Implementation Priority Summary

### Phase 1 (Critical - Implement First)
1. **Gap 4.1:** JSONL Export After Clear-Assignee (HIGH)
2. **Gap 1.2:** Batch Update with Clear-Assignee (HIGH)
3. **Gap 2.1:** Clear-Assignee with Status Change (HIGH)
4. **A.1:** Reopen Side Effect Verification (HIGH)
5. **A.2:** Storage Layer Whitespace Variations (HIGH)

### Phase 2 (Important - Implement Second)
6. **Gap 1.1:** Batch Create with Clear-Assignee (HIGH)
7. **Gap 2.4:** Clear-Assignee with Dependency Changes (HIGH)
8. **Gap 4.2:** JSONL Import of Cleared Assignee (HIGH)
9. **Gap 3.1:** Clear-Assignee on Non-Existent Bead (MEDIUM)

### Phase 3 (Nice-to-Have - Implement Last)
10. **Gap 1.3:** Batch Mixed Operations (MEDIUM)
11. **Gap 2.2, 2.3:** Combined field operations (MEDIUM)
12. **Gap 3.2, 3.3, 3.4:** Error handling edge cases (LOW/MEDIUM)
13. **Gap 4.3, 4.4:** Advanced JSONL scenarios (MEDIUM)
14. **B.1, B.2:** Event tracking and output formats (MEDIUM)

---

## Testing Infrastructure Notes

1. **Test File Organization:**
   - Batch operations → `tests/test_batch.rs` (new)
   - Combined operations → extend `tests/update_flags.rs`
   - Error handling → extend `tests/test_p0_bug_critical.rs`
   - JSONL serialization → extend `tests/test_claim_create_update_json.rs`

2. **Test Helpers Needed:**
   - Helper to verify `NULL` in database (not empty string)
   - Helper to count and verify event types
   - Helper to parse and validate JSONL content
   - Helper to run batch operations with mixed op types

3. **Test Isolation:**
   - Each test should use a separate workspace directory
   - Clean up workspace after each test
   - Verify no cross-test contamination

4. **Assertion Patterns:**
   - Always verify database state directly (not just CLI output)
   - Always verify event generation for state-changing operations
   - Always verify JSONL serialization for data operations
   - Use `assert_eq!` and `assert_matches!` for clear failure messages

---

## Next Steps

1. ✅ Document recommendations (this file)
2. ⏳ Implement Phase 1 tests (critical gaps)
3. ⏳ Implement Phase 2 tests (important gaps)
4. ⏳ Implement Phase 3 tests (nice-to-have gaps)
5. ⏳ Run full test suite and verify all new tests pass
6. ⏳ Update clear-assignee inventory (bf-4xu8ib) with test completion status
7. ⏳ Close related tracking beads

---

## Related Documentation

- **bf-4xu8ib:** Clear-Assignee Functionality Inventory
- **bf-31dgab:** Clear-Assignee Test Verification
- **docs/assignee-serialization-contract.md:** Data contract specification
- **docs/batch-json-schema.md:** Batch API specification
- **tests/manual_test_clear_assignee.sh:** Manual test script

---

**Status:** Recommendations documented, ready for implementation
