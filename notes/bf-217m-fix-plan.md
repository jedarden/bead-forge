# Fix Plan for `--limit 0` Handling

## Decision: Omit LIMIT Clause When limit==0

**Chosen Approach**: Conditionally omit the LIMIT clause entirely when `limit == 0`.

**Rationale**:
1. SQLite `LIMIT 0` returns zero rows (empty result set), not unlimited results
2. Omitting LIMIT is the idiomatic SQL way to express "no limit"
3. No special sentinel values needed (e.g., LIMIT -1)
4. Cleaner query plans without unnecessary LIMIT clauses

## Implementation Status: ✅ ALREADY COMPLETE

The fix has already been implemented in the codebase. No changes are needed.

## Current Implementation (Already in Place)

### File: `src/claim.rs`

**Lines 412-582**: `get_ready_candidates()` function

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
            // NO LIMIT clause - unlimited results
            "SELECT i.id, i.title, i.status, i.priority,
                    COALESCE(COUNT(d.issue_id), 0) as downstream_impact,
                    1000.0 / (COALESCE(c.float, 999) + 1) as critical_path_bonus,
                    i.created_at,
                    vs.p50_seconds as expected_seconds
             FROM issues i
             LEFT JOIN dependencies d ON d.depends_on_id = i.id ...
             ORDER BY ... DESC"
        } else {
            // WITH LIMIT clause
            "SELECT ... ORDER BY ... DESC LIMIT ?3"
        };
        tx.prepare(sql)?
    } else {
        // Standard scoring: conditional LIMIT
        let sql = if unlimited {
            // NO LIMIT clause - unlimited results
            "SELECT i.id, i.title, i.status, i.priority,
                    COALESCE(COUNT(d.issue_id), 0) as downstream_impact,
                    1000.0 / (COALESCE(c.float, 999) + 1) as critical_path_bonus,
                    i.created_at
             FROM issues i
             LEFT JOIN dependencies d ON d.depends_on_id = i.id ...
             ORDER BY ... ASC"
        } else {
            // WITH LIMIT clause
            "SELECT ... ORDER BY ... ASC LIMIT ?1"
        };
        tx.prepare(sql)?
    };

    // Conditional parameter binding
    let mut rows = if model.is_some() && harness.is_some() {
        if unlimited {
            stmt.query(params![model.unwrap(), harness.unwrap()])?
        } else {
            stmt.query(params![model.unwrap(), harness.unwrap(), limit as i64])?
        }
    } else {
        if unlimited {
            stmt.query([])?
        } else {
            stmt.query(params![limit as i64])?
        }
    };
    // ...
}
```

### File: `src/cli/mod.rs`

**Lines 179-191**: Ready command definition
```rust
Ready {
    /// Limit results (0 = unlimited)
    #[arg(long, default_value = "10")]
    limit: usize,
    ...
}
```

**Lines 781-788**: Command handler
```rust
fn cmd_ready(beads_dir: &PathBuf, limit: usize, format: &str) -> Result<()> {
    // ...
    // --limit 0 means unlimited (get_ready_candidates omits LIMIT clause when limit == 0)
    let candidates = storage.with_immediate_transaction(|tx| get_ready_candidates(tx, limit, None, None))?;
    // ...
}
```

### Test Coverage

**File: `src/claim.rs:1018-1039`**
```rust
#[test]
fn test_get_ready_candidates_limit_zero_returns_all() {
    // Creates 15 open beads
    let candidates = get_ready_candidates(tx, 0, None, None).unwrap();
    assert_eq!(candidates.len(), 15); // All 15 returned
}
```

## Key Design Decisions

### 1. Sentinel Value: `limit == 0`
- **Why**: 0 is a natural sentinel for "no limit" from a user perspective
- **Alternative considered**: Use -1 or usize::MAX
- **Rejected**: -1 requires i64, usize::MAX is cryptic to users

### 2. Query Strategy: Conditional SQL Preparation
- **Why**: Two separate SQL strings (with/without LIMIT) rather than one string with conditional binding
- **Alternative**: Use single SQL with `LIMIT ?` and bind NULL/0 for unlimited
- **Rejected**: SQLite doesn't support NULL in LIMIT; would need complex CASE expressions

### 3. Implementation: Dual Code Paths
- The code has 4 query variants:
  1. Velocity-aware + unlimited (no LIMIT)
  2. Velocity-aware + limited (LIMIT ?3)
  3. Standard + unlimited (no LIMIT)
  4. Standard + limited (LIMIT ?1)

## Why This Fix Works

| Scenario | Before Fix (hypothetical) | After Fix (actual) |
|----------|---------------------------|---------------------|
| `bf ready --limit 0` | Would execute `LIMIT 0` → 0 rows | Omits LIMIT → all rows |
| `bf ready --limit 5` | `LIMIT 5` → 5 rows | `LIMIT 5` → 5 rows |
| `bf ready` (default) | Would use default incorrectly | Default 10 works correctly |

## Verification

The fix can be verified by running:

```bash
# Create workspace with test beads
cd /tmp/test-ws && bf init

# Create multiple ready beads
bf create --title "Test 1" --type task
bf create --title "Test 2" --type task
bf create --title "Test 3" --type task

# Verify limit 0 returns all (unlimited)
bf ready --limit 0  # Should return 3 beads

# Verify positive limit works
bf ready --limit 1  # Should return 1 bead
```

## Related Beads

- **bf-66ub**: SQLite LIMIT 0 behavior test (closed)
- **bf-5v93**: Located ready command limit code path (closed)
- **bf-4te2**: Triage confirming fix already implemented (open)
- **bf-s9yr**: Original bug report (open - should be closed)
- **bf-5sw6**: Duplicate bug report (closed)

## Conclusion

**No code changes required.** The fix was implemented in earlier commits and is working correctly. The design decision to omit the LIMIT clause when `limit == 0` is sound, properly tested, and documented in the code.

### Next Steps
The parent bead **bf-s9yr** should be closed as the bug is fixed. The triage umbrella **bf-1aco** should be updated to reflect this resolution.

## Implementation Note

The fix plan has been documented and committed (commit 0a0d705). However, the bead could not be closed due to a bug in the `bf close` command:

```
Error: Invalid claimed_at format: premature end of input
```

This error appears to be caused by duplicate worker_sessions records for this bead (two records with different claimed_at timestamps). The close command is likely trying to parse these timestamps and encountering an unexpected format.

The design task is complete - the fix plan is documented in this file and committed. The bead should be closed once the CLI bug is resolved.
