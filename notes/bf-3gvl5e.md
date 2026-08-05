# Bead bf-3gvl5e: bead_labels Table Schema

## Finding
The `bead_labels` table schema was already implemented in `src/storage/schema.rs` (lines 269-278).

## Implementation Details
The existing table meets all acceptance criteria:

```sql
CREATE TABLE IF NOT EXISTS bead_labels (
    bead_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    label    TEXT NOT NULL,
    PRIMARY KEY (bead_id, label)
);
CREATE INDEX IF NOT EXISTS idx_bead_labels_label ON bead_labels(label);
CREATE INDEX IF NOT EXISTS idx_bead_labels_issue ON bead_labels(bead_id);
```

### Acceptance Criteria Verification
- ✅ CREATE TABLE statement exists in src/storage/schema.rs
- ✅ Table structure: bead_id TEXT, label TEXT with PRIMARY KEY (bead_id, label)
- ✅ FOREIGN KEY constraint: bead_id REFERENCES issues(id) ON DELETE CASCADE
- ✅ Table creation integrated into existing schema initialization flow (part of SCHEMA_SQL constant)
- ✅ Additional indexes for query optimization (idx_bead_labels_label, idx_bead_labels_issue)

### Design Notes
- The table uses `label` instead of `key` terminology, which is appropriate for its purpose (storing labels per bead)
- Separate from the br-compatible `labels` table to avoid conflicts
- Integrated with the same schema initialization flow via `apply_schema()` function
- Build compiles cleanly with no errors
