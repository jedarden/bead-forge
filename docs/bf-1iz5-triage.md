# Bug Triage: count_unflushed Over-reporting After Repair-Import

## Reproduction

```bash
# Create a workspace with JSONL containing 2 beads
# Run: bf doctor --repair --force
# Result: count_unflushed == 2 (should be 0)
```

**Confirmed:** After `doctor --repair`, ALL beads are marked as dirty, even though they just came FROM JSONL (so they're already flushed).

## Root Cause Analysis

### Execution Flow

1. `doctor --repair` calls `repair()` in `src/doctor.rs:302`
2. `repair()` deletes old database, creates new one (`src/doctor.rs:393-409`)
3. `repair()` calls `import_jsonl()` with `storage.create_issue()` as callback (`src/doctor.rs:411-414`)
4. `create_issue()` in `src/storage/sqlite.rs:289` marks every new bead as dirty:
   ```rust
   // Line 369-373 in src/storage/sqlite.rs
   tx.execute(
       "INSERT OR REPLACE INTO dirty_issues (issue_id, marked_at) VALUES (?1, ?2)",
       params![&issue.id, chrono::Utc::now().to_rfc3339()],
   )?;
   ```
5. `export_hashes` table is never populated during repair
6. After import completes:
   - All beads are in `dirty_issues` table
   - `export_hashes` table is empty
7. `count_unflushed()` counts all beads in `dirty_issues` (`src/doctor.rs:234-257`)

### The Bug

**Assumption in `create_issue()`:** "New beads need to be flushed to JSONL"

**Reality during repair:** Beads are coming FROM JSONL, so they're already flushed

## Impact

- `bf doctor check` incorrectly reports unflushed beads after repair
- Users may run unnecessary `bf sync --flush-only` operations
- False positives in health checks

## Fix Plan

### Option 1: Post-Import Cleanup (Recommended)

After `import_jsonl()` in `repair()`, add:

```rust
// In src/doctor.rs, after line 417 (rebuild_blocked_cache)

// Populate export_hashes for all imported beads (they came from JSONL)
storage.with_immediate_transaction(|tx| {
    let now = Utc::now().to_rfc3339();
    for issue in imported_issues {
        let hash = issue.content_hash();
        tx.execute(
            "INSERT OR REPLACE INTO export_hashes (issue_id, content_hash, exported_at)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![&issue.id, &hash, &now],
        )?;
    }
    Ok(())
})?;

// Clear dirty_issues - all beads just came from JSONL
storage.clear_dirty()?;
```

**Pros:**
- Minimal change to existing code
- Explicitly marks the post-repair state as "clean"
- Works with existing `create_issue()` logic

**Cons:**
- Requires collecting imported issues during `import_jsonl()`

### Option 2: Separate Import Path

Create `create_issue_clean()` or `create_issue_from_jsonl()` that doesn't mark dirty:

```rust
pub fn create_issue_from_jsonl(&self, issue: &Issue) -> Result<()> {
    // Same as create_issue but WITHOUT the dirty_issues INSERT
    // Used for imports from JSONL where beads are already flushed
}
```

Then use it in `repair()`:
```rust
let result = import_jsonl(&jsonl_path, |issue| {
    storage.create_issue_from_jsonl(issue)?;
    Ok(UpsertResult::New)
})?;
```

**Pros:**
- Semantically clearer
- Reusable for other import scenarios

**Cons:**
- Code duplication
- More complex API surface

### Option 3: Repair-Exclusive SQL Path

In `repair()`, use raw SQL INSERT instead of `create_issue()`:

```rust
let result = storage.with_immediate_transaction(|tx| {
    import_jsonl(&jsonl_path, |issue| {
        // Raw INSERT without dirty_issues marking
        tx.execute("INSERT INTO issues (...) VALUES (...)", [...])?;
        // ... labels, dependencies, comments, annotations ...
        Ok(UpsertResult::New)
    })
})?;
```

**Pros:**
- No changes to public API
- Most direct fix

**Cons:**
- Duplicates INSERT logic from `create_issue_tx()`
- Maintenance burden (schema changes need updates in two places)

## Computation Details

### `count_unflushed` (src/doctor.rs:234-257)

```rust
fn count_unflushed(db_path: &Path) -> Result<usize> {
    let conn = Connection::open(db_path)?;
    
    let table_exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='dirty_issues'",
        [],
        |row| row.get(0),
    )?;
    
    if table_exists == 0 {
        return Ok(0);
    }
    
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM dirty_issues",
        [],
        |row| row.get(0),
    )?;
    
    Ok(count as usize)
}
```

Simply counts rows in `dirty_issues` table.

### `export_hashes` (src/sync.rs:220-235)

```rust
fn update_export_hashes_for_issues(storage: &Storage, issues: &[Issue]) -> Result<()> {
    storage.with_immediate_transaction(|tx| {
        let now = Utc::now().to_rfc3339();
        
        for issue in issues {
            let hash = issue.content_hash();
            tx.execute(
                "INSERT OR REPLACE INTO export_hashes (issue_id, content_hash, exported_at)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![&issue.id, &hash, &now],
            )?;
        }
        
        Ok(())
    })
}
```

Populates `export_hashes` table after successful export to JSONL.

### Why Import Doesn't Mark Flushed

The `import()` function in `src/sync.rs:118` intentionally does NOT populate `export_hashes`:

```rust
pub fn import(workspace_dir: &Path) -> Result<SyncResult> {
    // ... import logic ...
    
    // Rebuild blocked cache after import
    storage.rebuild_blocked_cache()?;
    
    // NOTE: No export_hashes population here
    // Import is for bringing JSONL changes INTO SQLite
    // Export hashes are only updated during flush operations
}
```

This is correct for normal import (JSONL may have remote changes). But during `repair`, we're rebuilding FROM JSONL, so all beads are implicitly flushed.

## Recommended Fix

**Use Option 1** with a modification to track imported issues:

```rust
// In src/doctor.rs, repair() function
// Replace lines 411-417 with:

let mut imported_issues: Vec<Issue> = Vec::new();

let result = storage.with_immediate_transaction(|tx| {
    import_jsonl(&jsonl_path, |issue| {
        let imported = issue.clone();
        Storage::create_issue_tx(tx, &issue)?;
        imported_issues.push(imported);
        Ok(UpsertResult::New)
    })
})?;

// Rebuild blocked cache after import
storage.rebuild_blocked_cache()?;

// Mark all imported beads as flushed (they came from JSONL)
if !imported_issues.is_empty() {
    update_export_hashes_for_issues(&storage, &imported_issues)?;
    storage.clear_dirty()?;
}

Ok(result.imported)
```

**Why this works:**
1. Uses `create_issue_tx()` instead of `create_issue()` to avoid dirty marking during import
2. Collects imported issues for post-processing
3. Populates `export_hashes` table (marks beads as exported to JSONL)
4. Clears `dirty_issues` table (all beads are clean after repair)

## Testing

Add test to `src/doctor.rs`:

```rust
#[test]
fn test_repair_marks_beads_as_flushed() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");
    
    init_workspace(&beads_dir, "bf").unwrap();
    let metadata = load_metadata(&beads_dir).unwrap();
    let db_path = beads_dir.join(&metadata.database);
    let jsonl_path = beads_dir.join(&metadata.jsonl_export);
    
    // Create JSONL with 2 beads
    let issue1 = Issue { /* ... */ };
    let issue2 = Issue { /* ... */ };
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&jsonl_path).unwrap();
        writeln!(file, "{}", serde_json::to_string(&issue1).unwrap()).unwrap();
        writeln!(file, "{}", serde_json::to_string(&issue2).unwrap()).unwrap();
    }
    
    // Repair from JSONL
    let imported = repair(workspace, false, false).unwrap();
    assert_eq!(imported, 2);
    
    // Verify no unflushed beads
    let storage = Storage::open(&db_path).unwrap();
    let dirty = storage.list_dirty_issues().unwrap();
    assert_eq!(dirty.len(), 0);
    
    // Verify export_hashes populated
    let count: i64 = storage.conn.lock().unwrap()
        .query_row("SELECT COUNT(*) FROM export_hashes", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2);
}
```

## Related Beads

- Umbrella: bf-3k50
