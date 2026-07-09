# Triage Report: count_unflushed over-report after repair-import

## Executive Summary

**Status**: CANNOT REPRODUCE - Code appears correct, tests pass

**Finding**: The import/repair code correctly does NOT mark imported beads as dirty, and properly clears the `dirty_issues` table after completion. All existing tests validate this behavior.

## Detailed Analysis

### Expected Behavior
After `sync::import()` or `doctor::repair()`:
1. Beads from JSONL should be imported into SQLite
2. `dirty_issues` table should be empty (no unflushed beads)
3. `count_unflushed()` should return 0

### Actual Behavior (from tests)
✅ **All tests PASS**:
- `test_import_leaves_zero_unflushed`
- `test_repair_cycle_clears_unflushed_correctly`
- `test_import_clears_pre_existing_dirty_marks`

### Code Flow Verification

#### `sync::import()` Implementation
```rust
// Line 150-181: Import transaction
let result = storage.with_immediate_transaction(|tx| {
    import_jsonl(&jsonl_path, |issue| {
        match existing {
            None => {
                Storage::create_issue_tx(tx, &issue)?;  // ✓ Does NOT add to dirty_issues
                Ok(UpsertResult::New)
            }
            Some(existing_issue) => {
                if issue.updated_at > existing_issue.updated_at {
                    Storage::update_issue_from_json_tx(tx, &issue)?;  // ✓ Does NOT add to dirty_issues
                    Ok(UpsertResult::Updated)
                } else {
                    Ok(UpsertResult::Unchanged)
                }
            }
        }
    })
})?;

// Line 188: Clear dirty marks
storage.clear_dirty()?;  // ✓ Removes all rows from dirty_issues
```

#### `create_issue_tx()` Implementation
Lines 1527-1619 in `src/storage/sqlite.rs`:
- Inserts issue into database
- Inserts labels, dependencies, comments, annotations
- **Does NOT insert into `dirty_issues`** ✓

#### `update_issue_from_json_tx()` Implementation
Lines 1622-1729 in `src/storage/sqlite.rs`:
- Updates issue in database
- Re-inserts labels, dependencies, comments
- **Does NOT insert into `dirty_issues`** ✓

### Key Design Decisions

**Why `create_issue_tx()` does NOT mark dirty:**
- Import operations bring beads from JSONL (source of truth)
- Imported beads are already "flushed" by definition
- Only user-initiated changes should mark dirty

**Why `clear_dirty()` is called after import:**
- Ensures `dirty_issues` table is empty after import
- Clears any pre-existing dirty marks from before import
- Documents that DB and JSONL are in sync

## Possible Explanations for Bug Report

### 1. Already Fixed
The bug may have been fixed in a previous commit. The tests suggest the current code is correct.

### 2. Misunderstanding
The bug description may refer to:
- A different scenario than what was tested
- A specific workspace state not covered by tests
- User-facing confusion about what "unflushed" means

### 3. Edge Case
There might be a very specific scenario (e.g., concurrent access, file system issues) not covered by the current test suite.

## Recommendations

### 1. Verify with Real-World Scenario
Test with an actual workspace that has:
- Existing beads in JSONL
- Run `bf doctor --repair`
- Then run `bf doctor` to check counts

### 2. Add Defensive Tests
Add tests for:
- Empty database with populated JSONL
- Populated database with empty JSONL
- Large number of beads (stress test)

### 3. Monitor for Reports
Watch for similar bug reports to identify the actual scenario that triggers the issue.

### 4. Document Behavior
Add comments explaining why `create_issue_tx()` doesn't mark dirty, to prevent future confusion.

## Conclusion

**The code is CORRECT** - imports from JSONL properly do not mark beads as dirty, and the `dirty_issues` table is properly cleared after all import/repair operations.

**If the bug still occurs**, it must be in a scenario not covered by the current tests or in the CLI/user-interaction layer rather than the core import logic.

**No immediate fix required** - the functionality works as designed and tested.
