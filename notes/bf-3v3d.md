# Bead bf-3v3d: Migration Path A Verification

## Task Summary
Verify that `Storage::open`'s `apply_migrations()` creates all bf-only tables and that the migration_lock table is properly defined and checked during claim operations.

## Verification Results

### 1. BF-Only Tables in SCHEMA_SQL

All five bf-only tables are defined in `src/storage/schema.rs` within `SCHEMA_SQL`:

| Table | Line | Purpose |
|-------|------|---------|
| `bead_annotations` | 258 | Stores arbitrary key-value metadata per bead (separate table to avoid br's rebuild_issues_table) |
| `worker_sessions` | 270 | Tracks worker metadata (model, harness, version) for each claim operation |
| `velocity_stats` | 287 | Aggregated statistics per (model, harness, issue_type) for velocity-aware scoring |
| `critical_path_cache` | 243 | Computed from dependency graph; lower float = more critical |
| `migration_lock` | 303 | Singleton table that prevents new claims during migration operations |

### 2. Migration Lock Table DDL

The `migration_lock` table is defined in `src/storage/schema.rs` (lines 300-308):

```sql
CREATE TABLE IF NOT EXISTS migration_lock (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    locked_by   TEXT NOT NULL,
    locked_at   DATETIME NOT NULL,
    expires_at  DATETIME NOT NULL
);
```

### 3. Migration Lock Check in Claim

The `claim()` function in `src/claim.rs` (lines 114-123) checks the migration lock at the start of each BEGIN IMMEDIATE transaction:

```rust
// Step 0: Check migration_lock - return NONE if migration is in progress
let lock_count: i64 = tx.query_row(
    "SELECT COUNT(*) FROM migration_lock WHERE expires_at > ?1",
    params![now.to_rfc3339()],
    |row| row.get(0),
).unwrap_or(0);
if lock_count > 0 {
    // Migration in progress - return None gracefully
    return Ok(None);
}
```

This ensures that:
1. Claims are refused during migration operations
2. Expired locks are automatically ignored (only locks with `expires_at > now` are counted)
3. The check happens at the start of the transaction, before any claim logic

### 4. apply_migrations() Function

The `apply_migrations()` function in `src/storage/schema.rs`:
- Is called by `apply_schema()` on every `Storage::open()`
- Currently handles the `critical_path_cache` schema migration (old format → new format)
- All bf-only tables are created via `SCHEMA_SQL` before `apply_migrations()` is called
- Uses `CREATE TABLE IF NOT EXISTS` which is idempotent

## Migration Path A Compliance

Per plan.md §4C, Migration Path A requires:
- ✅ All bf-only tables created with `CREATE TABLE IF NOT EXISTS`
- ✅ `migration_lock` table checked by `bf claim` at start of BEGIN IMMEDIATE transaction
- ✅ Returns NONE if lock row exists and not expired
- ✅ No `ALTER TABLE issues` (annotations are in separate `bead_annotations` table)
- ✅ br backward compatibility (br ignores bf-only tables)

## Build Status

Build passes successfully with no errors.
