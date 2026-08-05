# clear-assignee Test Coverage Inventory

## Overview
Complete inventory of existing test coverage for the `--clear-assignee` flag across the bead-forge codebase.

## Tests Found

### 1. CLI Integration Tests (`tests/update_flags.rs`)

#### `test_cli_update_clear_assignee_flag()` (line 602)
**Purpose**: Tests the explicit `--clear-assignee` flag as discoverable sugar for `--assignee ""`
**Coverage**:
- Creates bead with assignee
- Updates with `--clear-assignee`
- Verifies assignee is cleared (null in JSON)
**Status**: ✅ PASS

#### `test_cli_update_clear_assignee_conflicts_with_assignee()` (line 629)
**Purpose**: Ensures `--clear-assignee` and `--assignee` are mutually exclusive
**Coverage**:
- Attempts to pass both flags together
- Verifies clap rejects with conflict error
- Checks stderr contains "cannot be used with"
**Status**: ✅ PASS

#### `test_cli_update_assignee_empty_clears()` (line 575)
**Purpose**: Regression guard for bf-276 - empty string clears assignee
**Coverage**:
- Sets assignee to "claude-dead-worker"
- Updates with empty assignee string
- Verifies assignee is cleared to null
**Status**: ✅ PASS

### 2. Display Tests (`tests/test_show_assignee_display.rs`)

#### `test_show_assignee_cleared_via_update()` (line 279)
**Purpose**: Verifies assignee field is hidden after clearing in CLI output
**Coverage**:
- Creates bead with assignee
- Clears with `--clear-assignee`
- Verifies output no longer contains "Assignee:" line
**Status**: ✅ PASS

### 3. JSON Output Tests (`tests/test_claim_create_update_json.rs`)

#### `test_update_json_clear_assignee()` (around line 50+)
**Purpose**: Validates JSON output format when clearing assignee
**Coverage**:
- Updates with `--clear-assignee --json`
- Verifies assignee is cleared (empty or null)
**Status**: ✅ PASS

### 4. Assignee Validation Tests (`tests/test_assignee_validation.rs`)

#### `test_update_bead_with_empty_assignee_succeeds_clears_assignee()` (line 208)
**Purpose**: Tests empty string normalization clears assignee
**Coverage**:
- Creates bead with assignee "alice"
- Updates with empty string
- Verifies assignee is unset (NULL)
**Status**: ✅ PASS

#### `test_update_bead_with_whitespace_only_assignee_succeeds_clears_assignee()` (line 237)
**Purpose**: Tests whitespace-only normalization clears assignee
**Coverage**:
- Creates bead with assignee "alice"
- Updates with whitespace-only string
- Verifies assignee is unset (NULL)
**Status**: ✅ PASS

#### `test_update_bead_without_assignee_accepted()` (line 282)
**Purpose**: Tests that omitting `--assignee` doesn't clear existing assignee
**Coverage**:
- Creates bead with assignee "alice"
- Updates without specifying assignee
- Verifies "alice" is still set (not cleared)
**Status**: ✅ PASS

### 5. Close/Reopen Tests (`tests/close_reopen.rs`)

#### `test_reopen_clears_assignee()` (line 119)
**Purpose**: Tests that reopening a closed bead clears the assignee
**Coverage**:
- Uses storage API directly
- Sets `assignee: Some(String::new())` on reopen
- Verifies assignee is cleared to NULL
**Status**: ✅ PASS

### 6. Close/Reopen Integration (`tests/test_close_reopen_integration.rs`)

#### `test_reopen_clears_assignee()` (line 135)
**Purpose**: Integration test for `bf reopen` clearing assignee
**Coverage**:
- Full CLI workflow test
- Assigns bead, closes it, then reopens
- Verifies assignee is cleared after reopen
**Status**: ✅ PASS

### 7. CLI CRUD Integration (`tests/cli_integration_crud.rs`)

#### `test_update_clear_assignee()` (around line 180+)
**Purpose**: End-to-end test for update with clear-assignee
**Coverage**:
- Creates bead
- Updates with `--clear-assignee`
- Verifies success via exit code and JSON output
**Status**: ✅ PASS

## Source Code Implementation

### CLI Definition (`src/cli/mod.rs`)

**Line 191**: `clear_assignee: bool` - CLI flag definition

**Line 1204-1210**: Implementation logic
```rust
let assignee = if clear_assignee {
    Some(String::new())  // Cleared = empty string, normalized to NULL in storage
} else {
    changes.assignee
};
```

**Validation**: Mutual exclusion with `--assignee` enforced via clap

## What's Tested ✅

1. **Basic flag functionality**: `--clear-assignee` clears the assignee field
2. **Conflict detection**: `--clear-assignee` conflicts with `--assignee`
3. **Empty string normalization**: `--assignee ""` also clears assignee
4. **Whitespace normalization**: Whitespace-only strings clear assignee
5. **JSON output format**: Cleared assignee appears as `null` in JSON
6. **Display output**: Cleared assignee hides the "Assignee:" line
7. **Reopen behavior**: `bf reopen` clears assignee automatically
8. **Preservation of unspecified fields**: Omitting `--assignee` doesn't clear
9. **Integration**: Full CLI workflow from create → assign → clear → verify

## Coverage Gaps ❓

### Potential Missing Tests

1. **Batch operations**: `bf batch` with `--clear-assignee` - not found
2. **Combined operations**: `--clear-assignee` with other flags in same update
3. **Non-existent bead**: `bf update <nonexistent> --clear-assignee` error handling
4. **JSONL export**: Verify cleared assignee serializes correctly to JSONL
5. **Multiple clear attempts**: Clearing an already-cleared assignee
6. **Race conditions**: Concurrent clear operations (though this is unlikely with SQLite)
7. **Format variants**: Toon format output after clearing assignee
8. **Edge cases**:
   - Very long assignee names before clearing
   - Unicode assignees before clearing
   - Assignee with special characters before clearing

### Non-Gaps (Already Covered)

- Empty string clearing: ✅ `test_cli_update_assignee_empty_clears()`
- Whitespace clearing: ✅ `test_update_bead_with_whitespace_only_assignee_succeeds_clears_assignee()`
- Conflict with `--assignee`: ✅ `test_cli_update_clear_assignee_conflicts_with_assignee()`
- Reopen clearing: ✅ Both storage and integration tests
- Preservation when unspecified: ✅ `test_update_bead_without_assignee_accepted()`

## Summary

**Total test files with clear-assignee coverage**: 7
**Total test functions**: 11+
**Coverage quality**: Good for core functionality
**Main gaps**: Batch operations, combined operations, edge cases

## Recommendation

The core `--clear-assignee` functionality is well-tested. The main gaps are:
1. Batch operations (`bf batch` with clear-assignee)
2. Combined flag operations (clear-assignee + other flags together)
3. More edge case testing (special characters, long names, etc.)

These gaps are not critical for the core functionality but would improve overall test robustness.
