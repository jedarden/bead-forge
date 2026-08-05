# SQLite Schema NULL Handling Investigation for Assignee Field

## Investigation Summary

**NULL supported: YES** ✅

The SQLite schema properly supports NULL values in the assignee column.

## Schema Evidence

### Column Definition
From `src/storage/schema.rs:23`:
```sql
assignee TEXT,
```

The assignee field is defined as:
- Type: `TEXT`
- **NOT NULL constraint**: ABSENT (allows NULL)
- **DEFAULT value**: None (defaults to NULL when not specified)
- **CHECK constraints**: None specific to assignee

### Index Optimization
From `src/storage/schema.rs:60`:
```sql
CREATE INDEX IF NOT EXISTS idx_issues_assignee 
ON issues(assignee) WHERE assignee IS NOT NULL;
```

This partial index only indexes non-NULL assignee values for performance, but **does not prevent** NULL values from being stored. This is a common SQLite optimization.

## Application-Level Handling

### Historical Context (bf-29wxxl)
From `src/doctor.rs:38-42`:
```rust
// Beads whose assignee is the literal empty string instead of NULL -- see bf-29wxxl.
// `bf create`/`bf update` normalize this to NULL now (bf-4mj7l/bf-2uhsk), but rows
// written before that fix still carry "", and any consumer testing `assignee is not
// None` reads them as already-claimed. Repaired by `bf doctor --reconcile`.
pub empty_assignee_ids: Vec<String>,
```

This confirms:
1. **NULL is the canonical value** for unassigned beads
2. Empty string ("") was a historical bug (now fixed)
3. The normalization logic converts "" → NULL
4. Doctor module can repair legacy beads

## Constraints Verification

### CHECK Constraints
The only CHECK constraint in the issues table (lines 49-53) validates the relationship between `status` and `closed_at`:
```sql
CHECK (
    (status = 'closed' AND closed_at IS NOT NULL) OR
    (status = 'tombstone') OR
    (status NOT IN ('closed', 'tombstone') AND closed_at IS NULL)
)
```

No CHECK constraint references `assignee`.

### Triggers
No triggers in the schema that would prevent NULL assignee values.

### Foreign Keys
The assignee column has no foreign key constraints (it's a free-text field referencing a user identifier).

## Conclusion

**NULL is fully supported and is the correct value for unassigned beads.**

The schema design:
- Allows NULL values (no NOT NULL constraint)
- Treats NULL as "unassigned"
- Uses NULL (not empty string) as the canonical unassigned value
- Provides repair functionality for legacy empty-string values

No code changes required. The schema correctly handles NULL assignee values.
