# Fix Plan for --limit 0 Handling

## Issue Date
2026-07-02

## Problem Statement

Based on findings from bead **bf-66ub**, SQLite's `LIMIT 0` returns **zero rows** (empty result set), NOT unlimited results. Current code has inconsistent handling of `limit == 0`:

- ✅ **Correct**: `src/claim.rs` - omits LIMIT clause when `limit == 0`
- ❌ **Incorrect**: `src/storage/sqlite.rs` lines 223-224, 1130-1131 - would add `LIMIT 0` which returns empty results

## Design Decision

**Approach**: Omit LIMIT clause entirely when `limit == 0`

**Rationale**:
1. `LIMIT 0` in SQLite means "return zero rows" (empty result), not unlimited
2. Omitting LIMIT clause is the correct way to represent "no limit" in SQL
3. This is already the pattern used correctly in `src/claim.rs` and partially in `src/storage/sqlite.rs:1266`

## Files to Change

### 1. src/storage/sqlite.rs

#### Location 1: `list_issues()` method (lines 223-224)

**Current Code:**
```rust
if let Some(limit) = filter.limit {
    query.push_str(&format!(" LIMIT {}", limit));
}
```

**Issue:** Adds `LIMIT 0` when `filter.limit == Some(0)`, returning empty results

**Fix:**
```rust
if let Some(limit) = filter.limit {
    if limit > 0 {
        query.push_str(&format!(" LIMIT {}", limit));
    }
}
```

#### Location 2: `list_events_filtered()` method (lines 1130-1131)

**Current Code:**
```rust
if let Some(l) = limit {
    sql.push_str(&format!(" LIMIT {}", l));
}
```

**Issue:** Adds `LIMIT 0` when `limit == Some(0)`, returning empty results

**Fix:**
```rust
if let Some(l) = limit {
    if l > 0 {
        sql.push_str(&format!(" LIMIT {}", l));
    }
}
```

#### Location 3: `search_issues()` method (lines 1266-1267)

**Current Code (already correct):**
```rust
if limit > 0 {
    sql.push_str(&format!(" LIMIT {}", limit));
}
```

**Status:** ✅ No change needed - already handles `limit == 0` correctly

### 2. src/claim.rs

#### Location: `get_ready_candidates()` method (lines 418-549)

**Current Code (already correct):**
```rust
// limit == 0 means unlimited - omit LIMIT clause
let unlimited = limit == 0;

// SQL variants use conditional string selection based on `unlimited`
let sql = if unlimited {
    // SQL without LIMIT clause
} else {
    // SQL with LIMIT ?1 or LIMIT ?3
};
```

**Status:** ✅ No change needed - already handles `limit == 0` correctly by using two complete SQL query variants

## Implementation Pseudocode

### For src/storage/sqlite.rs list_issues():
```rust
// Replace lines 223-225
if let Some(limit) = filter.limit {
    if limit > 0 {
        query.push_str(&format!(" LIMIT {}", limit));
    }
}
```

### For src/storage/sqlite.rs list_events_filtered():
```rust
// Replace lines 1130-1132
if let Some(l) = limit {
    if l > 0 {
        sql.push_str(&format!(" LIMIT {}", l));
    }
}
```

## Testing Strategy

1. **Unit Test**: Add test for `limit == 0` returning all matching rows
2. **Integration Test**: Create workspace with 20+ beads, verify `bf list --limit 0` returns all
3. **Edge Cases**: Test `limit == Some(0)` for both `list_issues()` and `list_events_filtered()`

## Verification Steps

After applying fix:
1. Run `cargo test` to ensure no regressions
2. Create test workspace with 15 beads
3. Run `bf list --limit 0` and verify all 15 beads returned
4. Run `bf log --limit 0` and verify all events returned
5. Run `bf ready --limit 0` and verify all ready candidates returned

## Alternative Approaches Considered

### Option A: Use LIMIT -1 for unlimited
**Rejected**: SQLite does not support `LIMIT -1` (syntax error)

### Option B: Use very large number (e.g., LIMIT 9223372036854775807)
**Rejected**: 
- Still technically a limit
- Could cause performance issues or unexpected behavior
- Not idiomatic SQL

### Option C: Treat 0 as "no limit" at application level
**Rejected**: Doesn't fix the root cause - `LIMIT 0` in SQL returns zero rows

## Dependencies

- Depends on: **bf-66ub** (closed) - SQLite LIMIT 0 behavior research
- Blocks: Implementation bead (to be created after this design)

## Summary

**Change Scope**: 2 locations in `src/storage/sqlite.rs`
**Lines to Change**: ~6 lines total (3 lines per location)
**Risk Level**: Low - localized changes to SQL query building
**Confidence**: High - pattern already proven correct in `search_issues()` and `get_ready_candidates()`
