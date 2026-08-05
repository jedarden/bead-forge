# Clear-Assignee Test Coverage Inventory

## Summary
Found **6 test files** containing **8 test methods** that test clear-assignee functionality.

## Test Files

### 1. `tests/test_show_assignee_display.rs`
**Test Methods:**
- `test_show_assignee_cleared_via_update` (lines 279-324)

**Coverage:**
- CLI-level `--clear-assignee` flag
- Text format display verification
- Show command output after clearing

**Assertions:**
- Update with `--clear-assignee` succeeds
- Show output does not contain "Assignee:" line after clearing
- Initial assignee is set correctly before clearing

---

### 2. `tests/test_p0_bug_critical.rs`
**Test Methods:**
- `test_p0_bug_clear_assignee` (lines 141-164)

**Coverage:**
- Storage-level `clear_assignee()` method on Issue model
- P0 critical bug with assignee clearing
- Priority preservation after clearing

**Assertions:**
- Initial assignee is set correctly ("stale-assignee")
- After clearing, assignee is None
- P0 priority remains CRITICAL (unchanged)

---

### 3. `tests/update_flags.rs`
**Test Methods:**
- `test_cli_update_assignee_empty_clears` (lines 575-599)
- `test_cli_update_clear_assignee_flag` (lines 602-626)
- `test_cli_update_clear_assignee_conflicts_with_assignee` (lines 629-657)

**Coverage:**
- CLI `--assignee ""` (empty string) clears assignee
- CLI `--clear-assignee` flag behavior
- Mutual exclusivity of `--assignee` and `--clear-assignee`
- Regression test for bf-276 (stale assignee cleanup)
- JSON output verification

**Assertions:**
- Empty string `--assignee ""` clears the field (sets to null)
- `--clear-assignee` flag clears the field (sets to null)
- Command fails when both `--assignee` and `--clear-assignee` are provided
- Error message contains "cannot be used with" for conflict
- JSON output shows null after clearing

---

### 4. `tests/test_p0_no_labels.rs`
**Test Methods:**
- `test_p0_clear_assignee_without_labels` (lines 148-176)
- `test_p0_closed_and_reopened_without_labels` (lines 209-251)

**Coverage:**
- P0 bead clearing assignee without any labels
- Clearing assignee via IssueChanges with empty string
- Close/reopen cycle clears assignee
- Labels remain empty throughout operations

**Assertions:**
- Initial assignee is set correctly
- Initial labels length is 0
- After clearing, assignee is None
- Priority remains CRITICAL after clearing
- Labels remain empty (length 0) after clearing
- After reopen, assignee is None (cleared)
- Status transitions correctly (open → closed → open)

---

### 5. `tests/cli_integration_crud.rs`
**Test Methods:**
- `test_update_clear_assignee` (lines 645-665)
- `test_reopen_resets_closed_bead_to_open` (lines 750-777)

**Coverage:**
- CLI integration for `--clear-assignee` flag
- Full CRUD lifecycle with assignee clearing
- Reopen command behavior with assignee clearing
- JSON format verification after clearing

**Assertions:**
- Create with assignee succeeds
- Update with `--clear-assignee` succeeds
- JSON output shows assignee as null after clearing
- Show command verifies null in JSON output
- Reopen clears assignee automatically
- After reopen, assignee is null or missing
- close_reason and closed_at are null after reopen

---

### 6. `tests/test_claim_create_update_json.rs`
**Test Methods:**
- `test_update_json_clear_assignee` (lines 619-658)

**Coverage:**
- JSON output format for `--clear-assignee`
- Round-trip verification (set → clear → verify)
- CLI integration with `--json` flag

**Assertions:**
- Initial assignee is set correctly ("worker-1")
- Update with `--clear-assignee --json` succeeds
- Assignee field is either null or empty string after clearing
- Show command verifies cleared state

**Note:** Test is currently ignored due to `bf-3uk2w5: pre-existing shared-test-workspace isolation defect`

---

## Coverage Analysis

### Well-Covered Scenarios:
✅ CLI `--clear-assignee` flag behavior
✅ CLI `--assignee ""` (empty string) behavior  
✅ Storage-level `clear_assignee()` method
✅ JSON output after clearing
✅ Text format display after clearing
✅ P0 priority preservation during clearing
✅ Conflict detection (`--assignee` vs `--clear-assignee`)
✅ Reopen clearing assignee automatically
✅ Labels preservation during clearing
✅ Regression test for bf-276 (stale assignee cleanup)

### Missing or Light Coverage:
❓ Clear-assignee on different issue types (only bug/epic/task tested)
❓ Clear-assignee with dependencies
❓ Clear-assignee with comments
❓ Clear-assignee persistence across database reopen
❓ Clear-assignee in batch operations
❓ Clear-assignee with custom fields (annotations)
❓ Clear-assignee concurrent access scenarios
❓ Clear-assignee error handling (non-existent bead, etc.)

## Test Method Distribution

| Test File | Test Methods | Coverage Focus |
|----------|-------------|----------------|
| `test_show_assignee_display.rs` | 1 | Display verification |
| `test_p0_bug_critical.rs` | 1 | Storage API, P0 bugs |
| `update_flags.rs` | 3 | CLI flags, conflicts, regression |
| `test_p0_no_labels.rs` | 2 | P0 without labels |
| `cli_integration_crud.rs` | 2 | Full lifecycle, reopen |
| `test_claim_create_update_json.rs` | 1 | JSON output |

## Total Test Count: 8 test methods across 6 files
