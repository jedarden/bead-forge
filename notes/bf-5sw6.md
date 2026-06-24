# bf-5sw6: Verify limit=0 Fix Already Implemented

## Task
Verify that `bf ready --limit 0` returns all unblocked ready beads (unlimited) instead of returning nothing.

## Root Cause
The original br bug was that `limit=0` was passed directly to SQL `LIMIT 0` clause, which returns no rows.

## Fix Status
**ALREADY IMPLEMENTED** ✅

## Implementation Details

### Code Changes in `src/claim.rs`
The `get_ready_candidates()` function at line 412 handles `limit=0` as unlimited:

```rust
pub fn get_ready_candidates(
    tx: &Connection,
    limit: usize,
    model: Option<&str>,
    harness: Option<&str>,
) -> Result<Vec<ScoredBead>> {
    // limit=0 means unlimited - we'll use two different SQL queries
    let unlimited = limit == 0;
```

When `limit == 0`, the function uses SQL queries without a `LIMIT` clause.

### Help Text Documentation
The `bf ready` help text clearly documents the semantics:

```
--limit <LIMIT>    Limit results (0 = unlimited) [default: 10]
```

### Test Coverage

#### Unit Tests (`src/claim.rs`)
- `test_get_ready_candidates_limit_zero_returns_all`: Creates 15 beads, verifies all are returned with limit=0
- `test_get_ready_candidates_respects_limit`: Verifies normal limit behavior (limit=5 returns exactly 5)

#### Integration Tests (`tests/limit_zero.rs`)
- `test_ready_limit_zero_returns_all`: Comprehensive test with 15 beads
- `test_ready_limit_zero_direct_sql_check`: Documents that raw SQL `LIMIT 0` returns 0 rows

### Manual Verification
```bash
$ ./target/release/bf ready --limit 0 | wc -l
4

$ ./target/release/bf ready --limit 5000 | wc -l
4

$ ./target/release/bf ready --limit 2 | wc -l
2
```

## Summary
The fix was already implemented with:
1. ✅ `limit=0` treated as unlimited (no LIMIT clause in SQL)
2. ✅ Help text documents "0 = unlimited"
3. ✅ Unit tests cover both unlimited and normal limit behavior
4. ✅ Integration tests verify the behavior end-to-end
5. ✅ Manual testing confirms correct behavior

No additional changes needed.
