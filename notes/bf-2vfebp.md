# bf-2vfebp: dirty_issues Table Implementation

## Task
Add `dirty_issues` table schema to track which issues need flushing.

## Implementation Status
✅ **COMPLETE** - Table already exists in `src/storage/schema.rs` (lines 173-178)

## Schema Definition
```sql
CREATE TABLE IF NOT EXISTS dirty_issues (
    issue_id TEXT PRIMARY KEY,
    marked_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_dirty_issues_marked_at ON dirty_issues(marked_at);
```

## Verification
- ✅ Table exists in schema.rs DDL
- ✅ Table exists in database (verified with sqlite3)
- ✅ Schema check passed: `issue_id TEXT PRIMARY KEY, marked_at DATETIME`

## Usage
The table is actively used throughout the codebase:
- `src/claim.rs` - INSERT when beads are claimed
- `src/sync.rs` - list_dirty_issues() to find beads needing flush
- `src/close.rs` - check if bead is dirty before closing
- `src/batch.rs` - mark beads dirty during batch operations

## Notes
The bead description specified `bead_id TEXT NOT NULL PRIMARY KEY`, but the implementation uses `issue_id` which is semantically equivalent (references the `issues` table). The implemented schema includes a `marked_at` timestamp for tracking when issues were marked dirty, which adds useful auditability.
