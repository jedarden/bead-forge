# Bead bf-p6s39j: remove_label operation

## Task
Implement remove_label operation for bead labels.

## Verification Summary

The `remove_label` operation is **already implemented** in `src/storage/sqlite.rs` (lines 1799-1823). All acceptance criteria are met:

### 1. ✅ remove_label(bead_id, key) operation implemented
- Location: `src/storage/sqlite.rs:1799-1823`
- Signature: `pub fn remove_label(&self, issue_id: &str, label: &str) -> Result<()>`

### 2. ✅ Proper transaction handling
- Uses `self.with_immediate_transaction()` for atomic write operations
- Handles `SQLITE_BUSY` with exponential backoff

### 3. ✅ Cascading behavior via FK
- Both `labels` and `bead_labels` tables have `ON DELETE CASCADE` foreign keys
- Verified in `test_cascade_delete_on_issue_removal`
- Schema checks confirm FK constraints reference `issues(id)` with cascade

### 4. ✅ Unit tests pass
All 13 tests in `tests/storage_labels.rs` pass:
- test_remove_label_basic
- test_remove_label_uses_immediate_transaction
- test_remove_nonexistent_label_is_idempotent
- test_remove_label_from_nonexistent_issue
- test_remove_label_whitespace_handling
- test_remove_empty_label_fails
- test_remove_last_label
- test_remove_multiple_labels_sequentially
- test_remove_label_case_sensitive
- test_remove_label_special_characters
- test_remove_label_marks_dirty
- test_remove_label_both_tables
- test_cascade_delete_on_issue_removal

### 5. ✅ Idempotent removal of non-existent labels
- Removing non-existent labels succeeds without error (0 rows affected)
- Tested in `test_remove_nonexistent_label_is_idempotent`
- Tested in `test_remove_label_from_nonexistent_issue`

## Implementation Details

The remove_label operation:
1. Trims whitespace from the label parameter
2. Validates label is not empty/whitespace-only
3. Deletes from both `labels` and `bead_labels` tables
4. Only marks dirty if actual rows were deleted (idempotent)
5. Uses `BEGIN IMMEDIATE` transaction for proper concurrency handling

## Test Results
```
running 13 tests
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.57s
```

No code changes were needed - implementation was already complete and correct.
