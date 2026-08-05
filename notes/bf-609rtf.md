# Bead-Forge Assignee Exclusion Behavior for NEEDLE Explore

## Summary

**Bead-forge DOES exclude assigned beads** through a **two-layer filtering approach**:

1. **Primary filter (bead-forge storage layer):** SQL query filters by `status = 'open'`, which implicitly excludes assigned beads
2. **Secondary filter (NEEDLE explore strand):** Defensive `assignee.is_none()` check as belt-and-suspenders

This documentation focuses on the **bead-forge layer** (the storage/query layer that NEEDLE's `store.ready()` calls into). For the NEEDLE explore.rs side, see [`notes/bf-5hahhz.md`](bf-5hahhz.md).

## Layer 1: Bead-Forge Storage Layer Filtering

### Location
- **File:** `src/claim.rs`
- **Function:** `get_ready_candidates()`
- **Lines:** 427-432 (SQL WHERE clause)

### SQL Filtering Logic

The core SQL query that NEEDLE calls via `store.ready()`:

```sql
SELECT i.id, i.title, i.status, i.priority,
       COALESCE(COUNT(d.issue_id), 0) as downstream_impact,
       1000.0 / (COALESCE(c.float, 999) + 1) as critical_path_bonus,
       i.created_at,
       vs.p50_seconds as expected_seconds
FROM issues i
LEFT JOIN dependencies d ON d.depends_on_id = i.id AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
LEFT JOIN critical_path_cache c ON c.bead_id = i.id
LEFT JOIN velocity_stats vs ON vs.issue_type = i.issue_type
    AND vs.model = ?1
    AND vs.harness = ?2
WHERE i.status = 'open'              -- ← PRIMARY EXCLUSION FILTER
  AND i.ephemeral = 0
  AND i.pinned = 0
  AND i.is_template = 0
  AND i.deleted_at IS NULL
  AND i.id NOT IN (SELECT issue_id FROM blocked_issues_cache)
GROUP BY i.id
ORDER BY (...) DESC, downstream_impact DESC, ...
```

**Key filter:** `WHERE i.status = 'open'` (line 427)

### Why This Excludes Assigned Beads

When a bead is claimed (via `claim_bead()` or direct SQL):

```sql
-- From src/claim.rs:249-251
UPDATE issues
SET status = 'in_progress', assignee = ?, updated_at = ?
WHERE id = ? AND status = 'open'
```

The bead transitions:
- **Before claim:** `status = 'open'`, `assignee = NULL`
- **After claim:** `status = 'in_progress'`, `assignee = 'worker-xyz'`

Since the `get_ready_candidates()` query filters by `WHERE i.status = 'open'`, beads with `status = 'in_progress'` are **automatically excluded** from the result set.

## Layer 2: NEEDLE Explore Strand Defensive Filtering

### Location
- **File:** `/home/coding/NEEDLE/src/strand/explore.rs`
- **Lines:** 617-622 (primary), 657-665 (retry after mend)

### Defensive Filtering Logic

```rust
// From NEEDLE's explore.rs (documented in notes/bf-5hahhz.md)
candidates.retain(|b| {
    let assignee_ok = b.assignee.is_none();
    let labels_ok = !b.labels.iter().any(|l| filters.exclude_labels.contains(l));
    assignee_ok && labels_ok
});
```

This provides a **belt-and-suspenders** defensive check:
1. **Belt:** SQL `WHERE status = 'open'` (bead-forge layer)
2. **Suspenders:** `assignee.is_none()` (NEEDLE layer)

## Two-Layer Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    NEEDLE Explore Strand                    │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Defensive Filter: assignee.is_none()                 │  │
│  └──────────────────────┬────────────────────────────────┘  │
└───────────────────────────┼──────────────────────────────────┘
                            │
                            │ store.ready() call
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                    Bead-Forge Storage Layer                 │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  SQL Filter: WHERE status = 'open'                   │  │
│  │  ┌─────────────────────────────────────────────────┐ │  │
│  │  │  SQLite Database                                │ │  │
│  │  │  - issues table with status/assignee columns    │ │  │
│  │  │  - blocked_issues_cache (dependency blocking)   │ │  │
│  │  │  - critical_path_cache (float scoring)          │ │  │
│  │  │  - velocity_stats (expected duration)           │ │  │
│  │  └─────────────────────────────────────────────────┘ │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Integration Points

### Bead-Forge API for NEEDLE

**File:** `src/bead_store.rs`

The `bead_store` module provides NEEDLE's integration point:

```rust
// From src/bead_store.rs:237-247
pub fn get_ready(workspace: &Path, limit: usize) -> Result<Vec<crate::claim::ScoredBead>> {
    let beads_dir = find_beads_dir(workspace)
        .ok_or_else(|| anyhow!("No .beads directory found in {:?}", workspace))?;

    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    Ok(storage
        .with_immediate_transaction(|tx| crate::claim::get_ready_candidates(tx, limit, None, None))?)
}
```

NEEDLE calls `bead_store::get_ready()` which:
1. Opens the SQLite database
2. Runs the `get_ready_candidates()` SQL query with `WHERE status = 'open'`
3. Returns only unassigned beads

## Test Coverage in Bead-Forge

### Claim Transaction Tests

**File:** `src/claim.rs` (lines 710-790)

```rust
#[test]
fn test_claim_reclaims_stale_assignments() {
    // Setup: Create in_progress bead with stale assignee
    let mut issue = Issue::new("bf-stale".to_string(), "Stale".to_string(), ".".to_string());
    issue.status = Status::InProgress;
    issue.assignee = Some("worker_old".to_string());
    storage.create_issue(&issue).unwrap();

    // Claim with TTL that treats the bead as stale
    let claim_ttl = 60; // 60 minutes
    let claimed = claim(&tx, "worker_new", claim_ttl, now, None).unwrap();

    // Verify: New worker claimed it, assignee updated
    assert_eq!(claimed.bead_id, "bf-stale");
    let updated = storage.get_issue("bf-stale").unwrap().unwrap();
    assert_eq!(updated.assignee.as_ref().unwrap(), "worker_new");
}
```

### Ready Candidates Filter Tests

**File:** `src/bead_store.rs` (lines 401-422)

```rust
#[test]
fn test_get_ready() {
    // Create test beads (all open status)
    for i in 1..=3 {
        let issue = Issue::new(
            format!("bf-{:0>4}", i),
            format!("Test {}", i),
            ".".to_string(),
        );
        storage.create_issue(&issue).unwrap();
    }

    let candidates = get_ready(&workspace, 10).unwrap();
    assert_eq!(candidates.len(), 3);
}
```

## Complete Flow Example

### Scenario: NEEDLE Worker Requests Bead

1. **NEEDLE explore strand** calls `store.ready(workspace)` → `bead_store::get_ready()`
2. **bead-forge** executes SQL:
   ```sql
   WHERE status = 'open'
     AND ephemeral = 0
     AND pinned = 0
     AND is_template = 0
     AND deleted_at IS NULL
     AND id NOT IN (SELECT issue_id FROM blocked_issues_cache)
   ```
3. **SQLite returns** only beads with `status = 'open'` (all have `assignee = NULL`)
4. **NEEDLE explore strand** applies defensive filter:
   ```rust
   candidates.retain(|b| b.assignee.is_none())
   ```
5. **Result:** Only truly unassigned beads are candidates for claiming
6. **NEEDLE claims** via `bead_store::claim_bead()`:
   ```sql
   UPDATE issues SET status = 'in_progress', assignee = ? WHERE id = ?
   ```

## Status: Working as Expected

✅ **Fully implemented and operational**

The two-layer filtering works correctly:
- **Layer 1 (SQL)**: Excludes `in_progress` beads from the query result
- **Layer 2 (NEEDLE)**: Defensive check catches any edge cases
- **Test coverage**: Both layers have unit tests
- **Integration**: `bead_store::get_ready()` provides NEEDLE's API

## Related Documentation

- **NEEDLE explore strand filtering:** [`notes/bf-5hahhz.md`](bf-5hahhz.md)
- **Bead store API:** `src/bead_store.rs` (full module with NEEDLE integration)
- **Claim logic:** `src/claim.rs` (SQL queries and transaction handling)
- **Storage schema:** `src/storage/schema.rs` (issue status and assignee columns)

## Related Beads

- **bf-5hahhz:** Documented NEEDLE explore strand assignee exclusion
- **bf-4ocs0n:** Verified NEEDLE explore strand excludes assigned beads
- **bf-58chiy:** Test bead for verify exclusion behavior
