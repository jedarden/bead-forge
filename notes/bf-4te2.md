# Triage: bf-4te2 - ready --limit 0 handling

## Finding

**Status:** ✅ **RESOLVED - Fix Already Implemented**

## Current Implementation (As of 2026-07-02)

### CLI Handler (`src/cli/mod.rs:1229-1271`)
```rust
fn cmd_ready(beads_dir: &PathBuf, limit: usize, format: &str) -> Result<()> {
    // ...
    // --limit 0 means unlimited (get_ready_candidates omits LIMIT clause when limit == 0)
    let candidates = storage.with_immediate_transaction(|tx| get_ready_candidates(tx, limit, None, None))?;
```
- Takes `limit: usize` with default value `10`
- Passes limit directly to `get_ready_candidates`
- Comment correctly documents the behavior

### Storage Layer (`src/claim.rs:412-582`)

The `get_ready_candidates` function correctly handles `limit == 0`:

```rust
pub fn get_ready_candidates(
    tx: &Connection,
    limit: usize,
    model: Option<&str>,
    harness: Option<&str>,
) -> Result<Vec<ScoredBead>>
{
    // limit == 0 means unlimited - omit LIMIT clause
    let unlimited = limit == 0;

    let mut stmt = if let (Some(_m), Some(_h)) = (model, harness) {
        // Velocity-aware SQL: conditional LIMIT
        let sql = if unlimited {
            "SELECT ... ORDER BY ... DESC"  // NO LIMIT clause
        } else {
            "SELECT ... ORDER BY ... DESC LIMIT ?3"
        };
        tx.prepare(sql)?
    } else {
        // Standard SQL: conditional LIMIT
        let sql = if unlimited {
            "SELECT ... ORDER BY ... ASC"  // NO LIMIT clause
        } else {
            "SELECT ... ORDER BY ... ASC LIMIT ?1"
        };
        tx.prepare(sql)?
    };

    // Conditional parameter binding
    let mut rows = if model.is_some() && harness.is_some() {
        if unlimited { stmt.query(params![m, h])? }
        else { stmt.query(params![m, h, limit as i64])? }
    } else {
        if unlimited { stmt.query([])? }
        else { stmt.query(params![limit as i64])? }
    };
    // ...
}
```

### Test Coverage (`src/claim.rs:1018-1039`)

```rust
#[test]
fn test_get_ready_candidates_limit_zero_returns_all() {
    // Creates 15 open beads
    let candidates = get_ready_candidates(tx, 0, None, None).unwrap();
    assert_eq!(candidates.len(), 15); // All 15 returned
}
```

**Test passes** - confirms `limit=0` returns all candidates.

## Behavior Summary

- `--limit 0` → No `LIMIT` clause in SQL → Returns all ready candidates (unlimited)
- `--limit N` (N > 0) → `LIMIT N` in SQL → Returns at most N candidates

### Why Omit LIMIT Instead of LIMIT 0?

In SQL, `LIMIT 0` returns **zero rows** (empty result set), not unlimited rows.
The correct interpretation of "unlimited" is to omit the LIMIT clause entirely,
which is exactly what the current code does.

## Conclusion

The fix recommended in the original triage note (Option 1) has already been implemented.
The code now correctly:
1. Detects `limit == 0` as the "unlimited" sentinel
2. Conditionally constructs SQL with or without LIMIT clause
3. Conditionally binds parameters based on whether LIMIT is needed
4. Has test coverage verifying the behavior

**No further action required.**
