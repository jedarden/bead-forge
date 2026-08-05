# Clear-Assignee Functionality Inventory

## Overview
This document catalogs all clear-assignee functionality in the bead-forge codebase, including functions, methods, CLI flags, and tests.

---

## 1. Model Layer (`src/model.rs`)

### `Issue::clear_assignee()` Method
**Location:** `src/model.rs:841-847`

```rust
pub fn clear_assignee(&self, actor: String) -> IssueChanges {
    IssueChanges {
        assignee: Some(String::new()),
        actor: Some(actor),
        ..Default::default()
    }
}
```

**Purpose:** Creates an `IssueChanges` struct with the assignee set to `Some(String::new())` (empty string). This is the canonical way to signal "clear assignee" to the storage layer.

**Contract:**
- Returns `IssueChanges` with `assignee: Some("")`
- Storage layer interprets `Some("")` as "clear to NULL"
- For full semantics with event recording, use `Storage::clear_assignee` directly (NOTE: this method doesn't actually exist - see storage layer below)

**Related:** Similar pattern methods exist for `close()` and `reopen()` operations.

---

## 2. Storage Layer (`src/storage/sqlite.rs`)

### Update Logic - Assignee Clearing
**Location:** `src/storage/sqlite.rs:646-654`

```rust
if let Some(ref assignee) = changes.assignee {
    if assignee.trim().is_empty() {
        // Clearing stores NULL, never an empty string that would
        // read back as "assigned" and hide the bead from claiming.
        updates.push("assignee = NULL");
    } else {
        updates.push("assignee = ?");
        params.push(Box::new(assignee.clone()));
    }
}
```

**Purpose:** In `update_issue()`, checks if `assignee` is an empty string (after trimming) and converts it to SQL `NULL` instead of storing an empty string.

**Why:** Empty strings would read back as "assigned" and hide the bead from claiming operations.

### Event Recording
**Location:** `src/storage/sqlite.rs:700-713`

```rust
if let Some(ref new_assignee) = changes.assignee {
    let new_val = if new_assignee.trim().is_empty() {
        None
    } else {
        Some(new_assignee.as_str())
    };
    if current_assignee.as_deref() != new_val {
        let actor = changes.actor.as_deref().unwrap_or("cli");
        // Creates assignee_changed event...
    }
}
```

**Purpose:** Records an `assignee_changed` event when the assignee field changes, including when it's cleared (empty string → `None`).

### Filter Query Support
**Location:** `src/storage/sqlite.rs:249-257`

```rust
if let Some(ref assignee) = filter.assignee {
    if assignee.is_empty() {
        // Empty-string filter selects unassigned beads.
        query.push_str(" AND (i.assignee IS NULL OR i.assignee = '')");
    } else {
        query.push_str(&format!(" AND i.assignee = ?{}", param_idx));
        params.push(assignee.clone());
        param_idx += 1;
    }
}
```

**Purpose:** In `list_issues()`, allows filtering for unassigned beads by passing an empty string as the `assignee` filter value.

### Test: `test_assignee_clear_and_null_persistence()`
**Location:** `src/storage/sqlite.rs:2669-2709`

**Tests:**
1. Creating an issue with an assignee
2. Clearing assignee via `IssueChanges` with `assignee = Some(String::new())`
3. Verifying `assignee` becomes `None` (NULL in database)
4. Using `Issue::clear_assignee()` method
5. Verifying it also produces NULL

---

## 3. CLI Layer (`src/cli/mod.rs`)

### `--clear-assignee` Flag
**Location:** `src/cli/mod.rs:192`

```rust
clear_assignee: bool,
```

**Purpose:** Boolean flag in `Update` command struct. Mutually exclusive with `--assignee` (enforced by clap).

### Flag to Value Conversion
**Location:** `src/cli/mod.rs:1206-1213`

```rust
// --clear-assignee is sugar for --assignee "": both flow the
// empty-string "clear to NULL" signal into update_issue. clap
// guarantees the two flags are mutually exclusive.
let assignee = if clear_assignee {
    Some(String::new())
} else {
    assignee
};
```

**Purpose:** Converts the boolean `--clear-assignee` flag into `Some(String::new())` which flows through to storage as the clear signal.

**Design Intent:** Discoverable sugar for `--assignee ""` - frees an open bead with a stale assignee without requiring claim-then-reclaim.

---

## 4. Validation Layer (`src/validation.rs`)

### `normalize_assignee()` Function
**Location:** `src/validation.rs:37-41`

```rust
pub fn normalize_assignee(assignee: Option<&str>) -> Option<String> {
    assignee
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
```

**Purpose:** Normalizes assignee input for `bf create` by trimming whitespace and collapsing empty/whitespace-only values to `None`.

**IMPORTANT:** NOT used by `bf update`! The update command's `--assignee` value is three-valued:
- `None` = leave unchanged
- `Some("")` = clear to NULL
- `Some(x)` = set to x

**Where Used:**
- `bf create` - to derive new bead's assignee
- NOT used in `bf update` (to preserve the "clear" intent)

---

## 5. Reopen Module (`src/reopen.rs`)

### Reopen Clears Assignee
**Location:** `src/reopen.rs:13-14` (doc comments)

**Purpose:** The `reopen_bead()` function clears the assignee when reopening a closed bead, as the assignee is considered stale from when it was closed.

### Test: `test_reopen_clears_assignee()`
**Location:** `src/reopen.rs:240-279`

**Tests:**
1. Creates a closed bead with an assignee
2. Reopens the bead
3. Verifies assignee is cleared (None)

---

## 6. Tests

### Unit Tests

#### `tests/update_flags.rs::test_cli_update_clear_assignee_flag()`
**Location:** `tests/update_flags.rs:602-626`

**Purpose:** Tests `bf update --clear-assignee` flag functionality.

**Acceptance:**
1. Creates bead with assignee
2. Runs `update --clear-assignee`
3. Verifies assignee is null

#### `tests/update_flags.rs::test_cli_update_clear_assignee_conflicts_with_assignee()`
**Location:** `tests/update_flags.rs:629-657`

**Purpose:** Tests that `--clear-assignee` and `--assignee` are mutually exclusive.

**Acceptance:**
1. Attempts to use both flags together
2. Verifies command fails with clap conflict error

#### `tests/cli_integration_crud.rs::test_update_clear_assignee()`
**Location:** `tests/cli_integration_crud.rs:645-656`

**Purpose:** Integration test for clear-assignee through the full CLI.

**Acceptance:**
1. Creates bead with assignee
2. Runs `bf update --clear-assignee`
3. Verifies command succeeds

### Manual Test

#### `tests/manual_test_clear_assignee.sh`
**Location:** `tests/manual_test_clear_assignee.sh`

**Purpose:** End-to-end shell test for `bf update --clear-assignee`.

**Acceptance Criteria:**
1. Create a test bead with an assignee
2. Run `bf update --clear-assignee` on the bead
3. Verify the command succeeds without error
4. Confirm the assignee field is cleared (null) in output

---

## 7. Data Flow Summary

### Clear Assignee Flow

```
CLI: --clear-assignee flag
         ↓
CLI: Convert to Some(String::new())
         ↓
Storage: update_issue() receives Some("")
         ↓
Storage: Empty string check → assignee = NULL
         ↓
Storage: Event recorded (assignee_changed)
         ↓
Result: Database stores NULL, bead appears unassigned
```

### Model Layer Shortcut

```
issue.clear_assignee(actor) → IssueChanges { assignee: Some(""), actor }
         ↓
Pass to storage.update_issue()
         ↓
Same flow as above
```

---

## 8. Configuration Options

### CLI Flags
- `--clear-assignee` (update command): Boolean flag to clear assignee
- `--assignee ""` (update command): Equivalent alternative syntax
- `--assignee <value>` (update/create): Set assignee to specific value

### Mutually Exclusive Flags
- `--clear-assignee` and `--assignee` cannot be used together (enforced by clap)

---

## 9. Key Implementation Notes

1. **Empty String to NULL Mapping:** The critical invariant is that empty assignee strings are always mapped to NULL in the database, never stored as empty strings.

2. **Three-Valued Update Logic:** `bf update` uses three-valued logic for assignee:
   - `None` = no change
   - `Some("")` = clear
   - `Some(value)` = set

3. **Normalization Split:** `normalize_assignee()` is used by `bf create` but NOT `bf update` to preserve the clear intent.

4. **Reopen Side Effect:** Reopening a closed bead automatically clears the assignee as a side effect (stale assignee from previous work).

5. **Event Recording:** All assignee changes, including clears, generate `assignee_changed` events with proper old_value/new_value tracking.

---

## Summary Table

| Layer | Component | Location | Purpose |
|-------|-----------|----------|---------|
| Model | `Issue::clear_assignee()` | src/model.rs:841 | Creates IssueChanges with empty string |
| Storage | Empty string → NULL | src/storage/sqlite.rs:646 | Maps empty to NULL in SQL |
| Storage | Event recording | src/storage/sqlite.rs:700 | Records assignee_changed event |
| CLI | `--clear-assignee` flag | src/cli/mod.rs:192 | User-facing clear flag |
| CLI | Flag conversion | src/cli/mod.rs:1209 | Converts bool to Some("") |
| Validation | `normalize_assignee()` | src/validation.rs:37 | Normalizes for create only |
| Reopen | Auto-clear on reopen | src/reopen.rs:13 | Clears stale assignee |
| Tests | Multiple unit/integration tests | tests/*.rs | Test coverage |

---

## Test Coverage

✅ **Covered:**
- Unit test for `Issue::clear_assignee()` method
- Unit test for storage layer empty string → NULL persistence
- Unit test for CLI `--clear-assignee` flag
- Unit test for `--clear-assignee`/`--assignee` mutual exclusivity
- Integration test for full CLI flow
- Manual end-to-end shell test
- Reopen auto-clear assignee test

⚠️ **Potential Gaps:**
- No explicit test for empty string filter query in `list_issues()`
- No test for event recording on assignee clear (though covered implicitly in other tests)
- No test for interaction between assignee clear and other fields in same update

---

Generated: 2026-08-05
Bead: bf-4xu8ib
