# Triage: count_unflushed over-report after repair-import

## Bug Report
**Issue**: `doctor --repair import -> count_unflushed==imported though drift==0`

## Analysis

### Key Functions
1. **`create_issue()`** (sqlite.rs:289-379)
   - Used for normal user operations
   - DOES add to `dirty_issues` table (line 370-373)

2. **`create_issue_tx()`** (sqlite.rs:1527-1619)
   - Used for import/repair operations
   - Does NOT add to `dirty_issues` table
   - This is CORRECT - imports from JSONL shouldn't mark beads dirty

3. **`count_unflushed()`** (doctor.rs:234-257)
   - Simply counts rows in `dirty_issues` table
   - `SELECT COUNT(*) FROM dirty_issues`

4. **`check_consistency_with_hash()`** (doctor.rs:180-228)
   - Computes drift by comparing actual content (content_hash) between DB and JSONL
   - Independent of `dirty_issues` table

### Import/Repair Flow
Both `sync::import()` and `doctor::repair()` follow this pattern:
1. Call `create_issue_tx()` for each bead (doesn't add to dirty_issues)
2. Call `rebuild_blocked_cache()`
3. Call `clear_dirty()` (removes all rows from dirty_issues)

**Expected Result**: After import/repair, `dirty_issues` table should be empty, so `count_unflushed == 0`.

### Current Test Status
All existing tests PASS:
- `test_import_leaves_zero_unflushed` ✓
- `test_repair_cycle_clears_unflushed_correctly` ✓
- `test_import_clears_pre_existing_dirty_marks` ✓

## Hypothesis

The bug report mentions `drift==0` but `count_unflushed==imported`. This suggests:
- Drift computation shows DB and JSONL are in sync (correct)
- But `dirty_issues` table has unexpected rows (incorrect)

### Possible Causes

1. **Transaction Isolation Issue**: If `clear_dirty()` is called from one connection while another has uncommitted inserts in `dirty_issues`, there might be a visibility issue.

2. **Wrong Function Call**: Maybe somewhere in the code, `create_issue()` is being called instead of `create_issue_tx()` during import.

3. **Race Condition**: If there are concurrent operations, one might be adding to `dirty_issues` while another is clearing it.

4. **Test vs Production Difference**: The tests pass, but maybe the actual CLI behavior differs from the test behavior.

## Code Flow Analysis

### `sync::import()` Flow (sync.rs:118-196)
```rust
// Check for unflushed beads and warn (lines 129-147)
let dirty_issues = storage.list_dirty_issues()?;

// Stream import with content_hash comparison (lines 150-181)
let result = storage.with_immediate_transaction(|tx| {
    import_jsonl(&jsonl_path, |issue| {
        match existing {
            None => {
                Storage::create_issue_tx(tx, &issue)?;  // Does NOT add to dirty_issues
                Ok(UpsertResult::New)
            }
            Some(existing_issue) => {
                if incoming_hash == existing_hash {
                    Ok(UpsertResult::Unchanged)
                } else if issue.updated_at > existing_issue.updated_at {
                    Storage::update_issue_from_json_tx(tx, &issue)?;  // Does NOT add to dirty_issues
                    Ok(UpsertResult::Updated)
                } else {
                    Ok(UpsertResult::Unchanged)
                }
            }
        }
    })
})?;

// Clear dirty marks (line 188)
storage.clear_dirty()?;
```

### `doctor::repair()` Flow (doctor.rs:302-426)
```rust
// Delete old database
std::fs::remove_file(&db_path)?;

// Create new database and import JSONL (lines 409-416)
let result = storage.with_immediate_transaction(|tx| {
    import_jsonl(&jsonl_path, |issue| {
        Storage::create_issue_tx(tx, &issue)?;
        Ok(UpsertResult::New)
    })
})?;

// Clear dirty marks (line 423)
storage.clear_dirty()?;
```

## Key Findings

1. **Both `create_issue_tx()` and `update_issue_from_json_tx()` correctly do NOT add to `dirty_issues`**
   - This is by design - imports from JSONL are the source of truth

2. **Both `sync::import()` and `doctor::repair()` call `clear_dirty()` at the end**
   - This should leave `dirty_issues` table empty

3. **All existing tests PASS**
   - `test_import_leaves_zero_unflushed` ✓
   - `test_repair_cycle_clears_unflushed_correctly` ✓

## Root Cause Hypothesis

The bug is likely **NOT** in the basic import/repair flow (which works correctly in tests).

Possible scenarios:
1. **CLI-level issue**: The `bf import` or `bf doctor --repair` CLI commands might have different behavior than the Rust function calls
2. **Workspace state issue**: The bug might occur in a specific workspace state that the tests don't cover
3. **Timing issue**: There might be a race condition if multiple processes are accessing the database
4. **Misunderstanding of bug**: The bug description might refer to a different scenario than what we're testing

## Next Steps

1. Test the actual CLI commands to see if behavior differs from Rust tests
2. Create a test that simulates the exact user scenario described in the bug
3. Check if there's workspace-level state that affects import behavior
4. Verify the bug is still reproducible in the current codebase
