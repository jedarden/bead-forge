# bf-hwf66p: Dirty Issues Table and Tracking - Verification

## Task
Add dirty_issues table schema and basic tracking for export operations.

## Findings
This functionality was already fully implemented in the codebase:

### 1. Schema (src/storage/schema.rs:173-178)
```sql
CREATE TABLE IF NOT EXISTS dirty_issues (
    issue_id TEXT PRIMARY KEY,
    marked_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_dirty_issues_marked_at ON dirty_issues(marked_at);
```

### 2. Helper Functions
- **mark_dirty()** - Public API for marking a single issue dirty
- **clear_dirty()** - Public API for clearing all dirty flags after export
- **mark_dirty_tx()** - Transaction-level helper for atomic marking

### 3. Complete Coverage
All mutation operations mark dirty within their transactions:
- create_issue() - marks new beads dirty
- update_issue() - marks updated beads dirty
- close_issue() - marks closed beads dirty
- reopen_issue() - marks reopened beads dirty
- add_dependency() / remove_dependency() - marks dirty
- add_label() / remove_label() - marks dirty
- add_comment() - marks dirty
- set_annotation() / remove_annotation() / clear_annotations() - marks dirty

### 4. Atomicity
All operations use `with_immediate_transaction()` for proper locking and atomic commits.

## Verification
- ✅ dirty_issues table exists
- ✅ All write operations mark dirty
- ✅ cargo build clean
