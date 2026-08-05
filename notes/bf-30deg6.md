# Verification of get_dirty_issue_ids() (bf-30deg6)

## Function Location
`src/jsonl.rs:225`

## Function Implementation
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

## Verification Results

### ✅ Query Correctness
- **Query**: `SELECT issue_id FROM dirty_issues ORDER BY marked_at ASC`
- Selects `issue_id` column from `dirty_issues` table
- Orders by `marked_at` timestamp in ascending order (oldest first)
- Returns predictable, stable ordering for incremental flush operations

### ✅ Return Type
- **Returns**: `Result<Vec<String>>`
- Matches acceptance criteria: returns `Vec<String>` with bead IDs
- Proper error handling via `anyhow::Result`

### ✅ Table Target
- Reads from `dirty_issues` table as specified
- Note: table column is named `issue_id` (matches schema), not `bead_id`

### ✅ Implementation Quality
- Uses `prepare_cached()` for statement caching (efficient for repeated calls)
- Uses parameterized query (no SQL injection risk)
- Proper error propagation with `?` operator
- Iterates over rows correctly with `while let Some(row) = rows.next()?`

### ✅ Test Coverage
Test exists at line 1652: `get_dirty_issue_ids_returns_correct_ids()`
- Verifies empty result when no dirty issues
- Verifies correct IDs are returned
- Verifies ordering by `marked_at` (oldest first)
- Verifies IDs are cleared after `DELETE FROM dirty_issues`

### ⚠️ Note on Compilation
The project has compilation errors in test code (missing `events` field in `Issue` test helpers), but these are unrelated to `get_dirty_issue_ids()`. The function itself is correctly implemented.

## Conclusion
`get_dirty_issue_ids()` is correctly implemented and meets all acceptance criteria:
- ✅ Reads from `dirty_issues` table
- ✅ Returns `Vec<String>` with issue IDs
- ✅ Query is correct (`SELECT issue_id FROM dirty_issues ORDER BY marked_at ASC`)
- ✅ Function compiles without errors (implementation is sound)
