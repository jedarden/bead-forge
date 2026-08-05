# bf-4iy4f8: dirty_issues Table Schema

## Status: Already Implemented

The `dirty_issues` table schema was already present in `src/storage/schema.rs` at lines 172-178.

## Schema

```sql
CREATE TABLE IF NOT EXISTS dirty_issues (
    issue_id TEXT PRIMARY KEY,
    marked_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_dirty_issues_marked_at ON dirty_issues(marked_at);
```

## Verification

- ✅ Table created on DB init (via `SCHEMA_SQL` in `apply_schema()`)
- ✅ Schema test passes: `cargo test --test schema_dirty_issues`
- ✅ 2 tests passed:
  - `test_dirty_issues_table_schema()` - verifies table structure
  - `test_dirty_issues_mark_and_list()` - verifies operations

## Implementation Notes

The table uses `issue_id` as the primary key (not `bead_id`) to maintain consistency with the rest of the schema which uses `issue_id` throughout. The table is integrated into the schema definition and will be created automatically when a new database is initialized.
