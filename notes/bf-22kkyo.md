# Clear-Assignee Test Coverage Inventory

## Test Files and Coverage

### 1. `tests/test_assignee_validation.rs`
**Purpose:** Tests assignee field normalization and clearing behavior

**Test Methods:**
- `test_create_bead_with_empty_assignee_succeeds_unassigned` - Empty string during create → NULL
- `test_create_bead_with_whitespace_only_assignee_succeeds_unassigned` - Whitespace-only during create → NULL  
- `test_create_bead_with_tab_whitespace_assignee_succeeds_unassigned` - Tab-whitespace during create → NULL
- `test_update_bead_with_empty_assignee_succeeds_clears_assignee` - Empty string during update clears assignee
- `test_update_bead_with_whitespace_only_assignee_succeeds_clears_assignee` - Whitespace-only during update clears assignee

**Assertions:**
- Empty/whitespace assignee is normalized to NULL (not stored as empty string)
- Update with empty/whitespace clears existing assignee
- Uses `assignee_is_unset()` helper to verify JSON output lacks `"assignee":"` pattern

### 2. `tests/update_flags.rs`
**Purpose:** CLI flag behavior for assignee clearing

**Test Methods:**
- `test_cli_update_assignee_empty_clears()` - Tests `--assignee ""` clears the field
- `test_cli_update_clear_assignee_flag()` - Tests explicit `--clear-assignee` flag
- `test_cli_update_clear_assignee_conflicts_with_assignee()` - Tests mutual exclusion of `--assignee` and `--clear-assignee`

**Assertions:**
- Empty string via `--assignee ""` sets assignee to NULL
- `--clear-assignee` flag sets assignee to NULL
- clap rejects both flags together (ambiguous: set vs clear)
- Regression guard for bf-276 (prevents stranded open beads with stale assignees)

### 3. `tests/test_bf_o3puei.rs`
**Purpose:** Storage layer persistence of cleared assignees

**Test Methods:**
- `test_assignee_clear_persists_as_null()` - Full cycle: create → clear → read as NULL
- `test_assignee_set_after_clear()` - NULL → Some transition (reassign after clear)
- `test_empty_string_assignee_becomes_null()` - Verifies empty string becomes NULL, not empty string

**Assertions:**
- Cleared assignee persists as None (NULL in database)
- Re-reading from database confirms NULL persistence
- Assignee can be set again after clearing (NULL → Some)
- Empty string stored as NULL, not Some("")

### 4. `tests/close_reopen.rs`
**Purpose:** Assignee clearing behavior on bead reopen

**Test Methods:**
- `test_reopen_clears_assignee()` - Tests that reopen clears assignee
- `test_reopen_with_no_assignee_is_noop()` - Tests reopen with no assignee is no-op

**Assertions:**
- Assignee is preserved on close
- Reopen with empty string clears assignee to NULL
- Uses IssueChanges with `assignee: Some(String::new())` to trigger clear

### 5. `tests/cli_integration_crud.rs`
**Purpose:** High-level CLI integration tests for CRUD operations

**Test Methods:**
- `test_update_clear_assignee()` - Tests `bf update --clear-assignee` via CLI

**Assertions:**
- `bf update --clear-assignee` succeeds
- JSON output shows assignee as null after clearing

### 6. `tests/test_show_assignee_display.rs`
**Purpose:** Display behavior for cleared assignees

**Test Methods:**
- `test_show_assignee_cleared_via_update()` - Tests `bf show` output after clearing

**Assertions:**
- After `--clear-assignee`, `bf show` output does NOT contain "Assignee:" line
- Verifies visual display hides cleared assignee

### 7. `tests/test_p0_bug_critical.rs`
**Purpose:** P0 bug handling with assignee clearing

**Test Methods:**
- `test_p0_bug_clear_assignee()` - Tests clearing assignee on critical bugs

**Assertions:**
- P0 bug priority preserved after assignee clear
- `clear_assignee()` method on IssueChanges works correctly

### 8. `tests/test_p0_no_labels.rs`
**Purpose:** P0 bugs without labels, testing assignee clearing

**Test Methods:**
- `test_p0_clear_assignee_without_labels()` - Tests clearing assignee on unlabeled P0 bugs

**Assertions:**
- Assignee cleared to None
- Priority maintained as CRITICAL
- Labels remain empty (0 labels)

### 9. `tests/test_close_reopen_integration.rs`
**Purpose:** Integration tests for close/reopen cycle

**Test Methods:**
- `test_reopen_clears_assignee()` - Regression for bf-2uhsk

**Assertions:**
- Reopen must clear assignee to NULL/absent in JSON
- Regression guard for bf-2uhsk (prevents stale assignees on reopened beads)

### 10. `tests/test_claim_create_update_json.rs`
**Purpose:** JSON output format for cleared assignees

**Test Methods:**
- `test_update_json_clear_assignee()` - Tests `--clear-assignee --json` output

**Assertions:**
- JSON output shows assignee as empty or null after clearing
- Uses `--json` flag format validation

### 11. `tests/test_assignee.rs`
**Purpose:** Direct storage layer assignee clearing

**Test Methods:**
- `test_clear_bead_assignee()` - Direct storage API test

**Assertions:**
- Empty string clears assignee field in storage

## Coverage Summary

**Clearing Mechanisms Tested:**
1. Empty string (`--assignee ""`) via CLI
2. Explicit flag (`--clear-assignee`) via CLI  
3. Storage layer (IssueChanges with empty string)
4. Reopen operation (clears as side effect)

**Assertion Categories:**
- **Normalization:** Empty/whitespace → NULL (not empty string)
- **Persistence:** NULL survives database roundtrip
- **CLI behavior:** Flags work correctly, conflicts rejected
- **Display:** Cleared assignee hidden from show output
- **JSON format:** Missing/null assignee in JSON output
- **Reassignment:** NULL → Some transition works
- **Priority preservation:** P0/P1 priorities maintained through clear
- **Reopen side effect:** Assignee cleared on reopen (bf-2uhsk)

**Regression Guards:**
- bf-276: Empty `--assignee` must clear (not reject)
- bf-2uhsk: Reopen must clear assignee

**Test Count:** 18 test methods across 11 files
