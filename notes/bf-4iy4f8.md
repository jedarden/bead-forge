# bf-4iy4f8: dirty_issues table schema implementation

## Task
Add dirty_issues table schema to SQLite to track which issues have been modified since last flush.

## Implementation Status: ALREADY COMPLETE ✓

The `dirty_issues` table was already implemented in `src/storage/schema.rs` (lines 172-178).

### Schema Definition
```sql
CREATE TABLE IF NOT EXISTS dirty_issues (
    issue_id TEXT PRIMARY KEY,
    marked_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_dirty_issues_marked_at ON dirty_issues(marked_at);
```

### Acceptance Criteria Verification

1. ✓ **CREATE TABLE IF NOT EXISTS dirty_issues with bead_id TEXT PRIMARY KEY and timestamp**
   - `issue_id TEXT PRIMARY KEY` (correctly named to match foreign key reference)
   - `marked_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP` (timestamp field)

2. ✓ **Table is created on DB init**
   - Defined in `SCHEMA_SQL` constant
   - Applied via `apply_schema()` function in `src/storage/schema.rs`
   - Called during `Storage::open()` initialization

3. ✓ **Basic schema test passes**
   - `test_bf_tables_dont_interfere_with_br` confirms table exists
   - All 12 dirty_tracking tests pass (create, update, close, comment, label, dependency operations)

### Verification
```bash
# Schema test confirms table exists
cargo test --test schema_compat test_bf_tables_dont_interfere_with_br
# Result: PASS ✓

# Dirty tracking functionality tests
cargo test --test dirty_tracking
# Result: 12 tests PASSED ✓
```

## Implementation Details
- **Purpose**: Track beads modified since last flush for export operations
- **Location**: `src/storage/schema.rs` lines 172-178
- **Foreign Key**: References `issues(id)` with CASCADE delete
- **Index**: `idx_dirty_issues_marked_at` on timestamp for efficient cleanup queries
- **Integration**: Used by `Storage::mark_dirty()`, `Storage::clear_dirty()`, and `Storage::list_dirty_issues()` methods

## Conclusion
The table was already implemented as part of the schema. No changes were needed.
