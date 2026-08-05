# Clear-Assignee Functionality Inventory

**Bead ID:** bf-4xu8ib  
**Date:** 2026-08-05  
**Purpose:** Comprehensive inventory of all clear-assignee functionality in the bead-forge codebase

## Overview

Clear-assignee functionality allows users and automated systems to remove an assignee from a bead, setting it to `NULL` (unassigned state). This is critical for NEEDLE fleet worker coordination and manual workflow management.

---

## 1. Core Implementation Functions

### 1.1 CLI Layer (`src/cli/mod.rs`)

#### Command-Line Flag
- **Location:** Line 192
- **Definition:** `clear_assignee: bool`
- **Purpose:** Boolean flag for `bf update` command
- **Conflicts:** Mutually exclusive with `--assignee` flag (enforced by clap)
- **Documentation:** "Clear the assignee (set to unassigned). Equivalent to --assignee "" but more discoverable"

#### Flag Processing Logic
- **Location:** Lines 1198-1210
- **Implementation:**
```rust
let assignee = if clear_assignee {
    Some(String::new())  // Empty string signals "clear to NULL"
} else {
    assignee
};
```
- **Behavior:** Converts `--clear-assignee` flag to empty string, which flows into storage layer as "clear to NULL" signal

---

### 1.2 Model Layer (`src/model.rs`)

#### Issue::clear_assignee() Method
- **Location:** Lines 830-847
- **Signature:** `pub fn clear_assignee(&self, actor: String) -> IssueChanges`
- **Purpose:** Convenience method to create assignee-clearing changes
- **Returns:** `IssueChanges` struct with `assignee: Some(String::new())`
- **Actor Tracking:** Includes actor parameter for event logging
- **Documentation Notes:** 
  - Method docs note: "For full assignee-clear semantics with event recording, use `Storage::clear_assignee` directly instead"
  - However, no such `Storage::clear_assignee` method exists (documentation references planned but unimplemented feature)

---

### 1.3 Storage Layer (`src/storage/sqlite.rs`)

#### Core Clearing Logic in update_issue()
- **Location:** Lines 637-646
- **Implementation:**
```rust
if let Some(ref assignee) = changes.assignee {
    if assignee.trim().is_empty() {
        // Clearing stores NULL, never an empty string
        updates.push("assignee = NULL");
    } else {
        updates.push("assignee = ?");
        params.push(Box::new(assignee.clone()));
    }
}
```
- **Behavior:** 
  - Empty/whitespace-only string → `NULL` in database
  - Non-empty string → stored as-is
  - `None` → assignee field not touched (no update)

#### Event Recording for Assignee Changes
- **Location:** Lines 690-703
- **Implementation:**
```rust
// Record assignee_changed event when assignee changes
if let Some(ref new_assignee) = changes.assignee {
    let new_val = if new_assignee.trim().is_empty() {
        None
    } else {
        Some(new_assignee.as_str())
    };
    if current_assignee.as_deref() != new_val {
        // Insert assignee_changed event...
    }
}
```
- **Behavior:** Generates `assignee_changed` event tracking old_value → new_value

#### Automatic Clearing in reopen_issue()
- **Location:** Line 997 (within SQL UPDATE statement)
- **Behavior:** Automatically clears assignee when reopening closed beads
- **Rationale:** Assignee from previous closure is "stale" and should be reset for new claiming cycle

---

### 1.4 Validation Layer (`src/validation.rs`)

#### normalize_assignee() Function
- **Location:** Lines 7-42
- **Signature:** `pub fn normalize_assignee(value: Option<&str>) -> Option<String>`
- **Purpose:** Normalizes assignee values for `bf create` command
- **Behavior:**
  - `None` → `None` (no assignee)
  - `Some("value")` → `Some("value")` (trimmed)
  - `Some("")` → `None` (empty collapses to None)
  - `Some("  ")` → `None` (whitespace-only collapses to None)
- **Important:** Used by `bf create` but NOT by `bf update` to preserve clear intent (`Some("")` = clear)

---

## 2. Test Coverage

### 2.1 Manual Tests

#### End-to-End Shell Test
- **Location:** `tests/manual_test_clear_assignee.sh`
- **Coverage:**
  - Creating bead with assignee
  - Executing `bf update --clear-assignee`
  - Verifying assignee is cleared in final output
  - Interactive human verification required

### 2.2 Automated Tests

#### CLI Integration Tests
- **`tests/update_flags.rs`:**
  - `test_cli_update_clear_assignee_flag()` (line 602)
  - `test_cli_update_clear_assignee_conflicts_with_assignee()` (line 629)

- **`tests/cli_integration_crud.rs`:**
  - `test_update_clear_assignee()` (line 645)

- **`tests/test_claim_create_update_json.rs`:**
  - `test_update_json_clear_assignee()` (line 619)

- **`tests/test_show_assignee_display.rs`:**
  - Clear-assignee display verification

- **`tests/test_p0_bug_critical.rs`:**
  - `test_p0_bug_clear_assignee()` (line 142)

#### Unit Tests
- **`src/storage/sqlite.rs`:**
  - `test_assignee_clear_and_null_persistence()` (line 2661)
  - Tests both direct `IssueChanges` method and `Issue::clear_assignee()` convenience method
  - Verifies `NULL` persistence (not empty string)

- **`src/reopen.rs`:**
  - `test_reopen_clears_assignee()` (line 240)
  - Verifies automatic assignee clearing on reopen

---

## 3. Configuration and Options

### 3.1 Command-Line Interface

#### `bf update` Command
```bash
# Clear assignee using dedicated flag
bf update <bead-id> --clear-assignee

# Clear assignee using empty string (equivalent)
bf update <bead-id> --assignee ""

# Flags conflict (clap enforces mutual exclusivity)
bf update <bead-id> --clear-assignee --assignee "alice"  # ERROR
```

#### Flag Configuration
- **Type:** Boolean flag (`--clear-assignee`)
- **Default:** `false`
- **Conflicts:** `--assignee` (cannot be used together)
- **Processing:** Converted to empty string internally

### 3.2 Programmatic API

#### Direct IssueChanges
```rust
let mut changes = IssueChanges::default();
changes.assignee = Some(String::new());  // Signals "clear to NULL"
changes.actor = Some("system".to_string());
storage.update_issue("bf-123", &changes)?;
```

#### Convenience Method
```rust
let issue = storage.get_issue("bf-123")?;
let changes = issue.clear_assignee("system".to_string());
storage.update_issue("bf-123", &changes)?;
```

---

## 4. Key Design Patterns

### 4.1 Three-Valued Logic for Assignee Updates

| Value | Meaning | Database Result |
|-------|---------|-----------------|
| `None` | Leave unchanged | No UPDATE on assignee column |
| `Some("")` | Clear assignee | Sets assignee to NULL |
| `Some("value")` | Set assignee | Sets assignee to "value" |

### 4.2 Event Recording Contract

All assignee changes generate `assignee_changed` events with:
- **old_value:** Previous assignee (or None)
- **new_value:** New assignee (or None for cleared)
- **actor:** Who made the change

Empty string assigns serialize as `None` in events.

### 4.3 Reopen Side Effect

**Behavior:** Reopening closed beads (`bf reopen` or `Storage::reopen_issue`) automatically clears assignee.

**Rationale:** 
- Assignee represents the worker who last closed the bead
- On reopen, the bead should be available for claiming by any worker
- Prevents "stale assignee" blocking new claims

**Implementation:** 
- Both `src/reopen.rs` (command layer) and `Storage::reopen_issue()` (storage layer)
- SQL UPDATE statement sets `assignee = NULL` as part of reopen transaction

---

## 5. Documentation References

### 5.1 Contract Documentation
- **`docs/assignee-serialization-contract.md`:** Full contract specification
- **`docs/batch-json-schema.md`:** Batch API assignee clearing behavior
- **`docs/README.md`:** User-facing command reference
- **`docs/plan/plan.md`:** Implementation plan and known bugs

### 5.2 Tracking Beads
Multiple beads track clear-assignee work:
- **bf-4xu8ib:** This inventory
- **bf-5wun8h:** Test coverage inventory  
- **bf-5n92ir:** Test results and verification
- **bf-4fxgm1:** Test coverage summary
- **bf-gj673:** Assignee-clearing gap tracking

---

## 6. Discovered Gaps

### 6.1 Coverage Gaps
1. **Batch operations:** No test coverage for `bf batch` with `--clear-assignee`
2. **Combined operations:** Limited testing of `--clear-assignee` with other flags
3. **Error handling:** No specific tests for clear-assignee on non-existent beads
4. **JSONL export:** No verification that cleared assignee serializes correctly to JSONL

### 6.2 Documentation Issues
1. **Missing Storage method:** Documentation references `Storage::clear_assignee` but no such method exists
2. **Inconsistent terminology:** Some docs say "clear to NULL", others say "clear to unassigned"

---

## Summary

Clear-assignee functionality is **well-implemented across all layers** of the bead-forge stack:

✅ **CLI Layer:** Dedicated `--clear-assignee` flag with proper conflict handling  
✅ **Model Layer:** Convenience method for programmatic use  
✅ **Storage Layer:** Robust NULL persistence with event recording  
✅ **Validation Layer:** Proper normalization for create vs. update semantics  
✅ **Reopen Behavior:** Automatic clearing on bead reopen  
✅ **Test Coverage:** Comprehensive coverage of core use cases  

**Status:** Production-ready with minor documentation inconsistencies and some edge case test gaps.

---

**Next Steps:**
1. Document discovered test gaps in tracking beads
2. Consider adding `Storage::clear_assignee()` method for API completeness
3. Add edge case tests for batch operations and combined flags
4. Verify JSONL export serialization of cleared assignees
