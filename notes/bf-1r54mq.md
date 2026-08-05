# bf-1r54mq: Incremental Flush Dirty Issues Query - VERIFICATION

## Task
Implement incremental_flush() dirty_issues query logic

## Implementation Status: ✅ COMPLETE

The feature was already fully implemented in `src/jsonl.rs`.

### Implementation Details

**Function:** `get_dirty_issue_ids()` (lines 225-237)
```rust
pub fn get_dirty_issue_ids(conn: &rusqlite::Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare_cached(
        "SELECT issue_id FROM dirty_issues ORDER BY marked_at ASC"
    )?;

    let mut rows = stmt.query([])?;
    let mut ids = Vec::new();
    while let Some(row) = rows.next()? {
        ids.push(row.get(0)?);
    }

    Ok(ids)
}
```

**Usage:** Called in `incremental_flush()` at line 245:
```rust
let dirty_ids = get_dirty_issue_ids(conn)?;
```

### Acceptance Criteria Verification

✅ **Reads dirty_issues table into Vec<String>**
- Returns `Result<Vec<String>>`
- Collects all `issue_id` values from `dirty_issues` table

✅ **Uses prepared statement**
- Uses `conn.prepare_cached()` for efficient statement reuse
- Uses parameterized query with `query([])`

✅ **Basic test verifies query returns correct IDs**
- Test `get_dirty_issue_ids_returns_correct_ids` (lines 1652-1711)
- Verifies empty result when no dirty issues
- Verifies correct IDs returned after marking beads dirty
- Verifies ordering by `marked_at` (oldest first)
- Verifies empty result after clearing dirty marks

### Test Results

```
test jsonl::tests::get_dirty_issue_ids_returns_correct_ids ... ok
```

### Additional Context

The function also includes sorting by `marked_at ASC` to provide predictable ordering (oldest dirty beads first), which is useful for flush operations.

No code changes were required - the feature was already implemented and tested.
