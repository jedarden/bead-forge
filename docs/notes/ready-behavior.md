# Ready Queue Behavior Specification

## Overview

The ready queue is the core mechanism that determines which beads are available for workers to claim. It implements a sophisticated scoring algorithm that balances **downstream impact**, **priority**, **critical path position**, and **velocity data** to surface the most valuable work first.

## What Beads Appear in the Ready Queue

### Inclusion Criteria

A bead appears in the ready queue if and only if ALL of the following are true:

1. **Status is `open`**
   - Explicitly filters for `status = 'open'`
   - Excludes: `closed`, `in_progress`, `blocked`, `deferred`, `draft`, `tombstone`
   - Excludes custom terminal statuses: `done`, `completed`

2. **Not ephemeral**: `ephemeral = 0`
   - Ephemeral beads are temporary tracking beads not meant for workers

3. **Not pinned**: `pinned = 0`
   - Pinned beads are reserved for manual assignment or special handling

4. **Not a template**: `is_template = 0`
   - Template beads are patterns, not actionable work

5. **Not deleted**: `deleted_at IS NULL`
   - Soft-deleted beads are excluded

6. **Not blocked**: `id NOT IN (SELECT issue_id FROM blocked_issues_cache)`
   - The `blocked_issues_cache` materialized view tracks beads with unresolved blocking dependencies
   - A bead is blocked if it has at least one unclosed blocker in a blocking dependency type

### Blocking Dependency Types

The following dependency types cause blocking:
- `blocks` — direct blocking dependency
- `parent-child` — child cannot start until parent completes
- `conditional-blocks` — conditional blocking dependency
- `waits-for` — waiting for another bead to complete

Non-blocking dependency types (do NOT prevent ready queue inclusion):
- `relates-to`
- `related`
- `discovered-from`

## Ranking Algorithm

The ready queue orders candidates by a multi-level scoring system:

### Primary Score: Combined Impact Score

```
combined_score = (downstream_impact × 3.0) + ((4 - priority) × 2.0) + critical_path_bonus
```

**Components:**
- `downstream_impact`: Count of beads that directly depend on this one
- `priority`: 0=Critical through 4=Backlog (lower = more important)
- `critical_path_bonus`: `1000.0 / (float + 1)` from critical path cache

**Critical Path Bonus:**
- Float = 0 (on critical path): bonus ≈ 1000
- Float = 5: bonus ≈ 167
- Float = 999 (no critical path data): bonus ≈ 1

### Velocity-Aware Adjustment (when velocity data available)

When `velocity_stats` has p50 duration data for the (model, harness, issue_type) combination:

```
velocity_adjusted_score = combined_score / p50_seconds
```

**Fallback:** When no velocity data exists, uses `p50_seconds = 1800` (30 minutes)

**Effect:** Prefers beads with lower expected duration at the same impact level—high-impact quick wins surface first.

### Ordering Precedence (from highest to lowest priority)

1. **velocity_adjusted_score DESC** (when velocity data available)
   - Higher impact-per-unit-time is better

2. **downstream_impact DESC**
   - Beads blocking more dependents are prioritized

3. **critical_float ASC** (lower is better)
   - Zero-float beads (critical path) outrank non-critical

4. **priority ASC** (lower number is better)
   - P0 before P1 before P2, etc.
   - NULL priority treated as 999 (lowest)

5. **created_at ASC** (older is better / FIFO tiebreaker)
   - Within identical scores, older beads surface first

## Implementation Details

### Claim vs Ready Query

Both `claim()` and `get_ready_candidates()` use the **same scoring logic**, ensuring that:
- The ready list shows what will be claimed
- No surprise ordering when a worker claims

### blocked_issues_cache Materialized View

This cache table is maintained incrementally on:
- Dependency addition/removal
- Status changes (affecting blocker states)
- Bead creation/deletion

**Rebuild query:**
```sql
INSERT INTO blocked_issues_cache (issue_id, blocked_by, blocked_at)
SELECT DISTINCT
    d.issue_id,
    i.id as blocked_by,
    CURRENT_TIMESTAMP as blocked_at
FROM dependencies d
INNER JOIN issues i ON i.id = d.depends_on_id
WHERE d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
  AND i.status NOT IN ('closed', 'tombstone', 'done', 'completed')
  AND i.status != 'blocked'  -- status=blocked blockers also block
```

### Critical Path Cache

The `critical_path_cache` table stores computed float (slack) values:
- `float = 0`: On critical path (no slack)
- `float > 0`: Can be delayed without extending project timeline

Computed via two-pass algorithm:
1. **Forward pass:** Calculate earliest start (ES)
2. **Backward pass:** Calculate latest start (LS)
3. **Float = LS - ES**

Invalidated and recomputed when the dependency graph changes.

## Test Scenarios

### Status Filtering

| Test ID | Scenario | Expected Behavior |
|---------|----------|-------------------|
| S1 | Closed bead | Excluded from ready queue |
| S2 | Tombstone bead | Excluded |
| S3 | Blocked status bead | Excluded |
| S4 | In-progress bead | Excluded |
| S5 | Deferred bead | Excluded |
| S6 | Draft bead | Excluded |
| S7 | Custom status "done" | Excluded (terminal alias) |
| S8 | Custom status "completed" | Excluded (terminal alias) |

### Dependency Filtering

| Test ID | Scenario | Expected Behavior |
|---------|----------|-------------------|
| D1 | Single open blocker | Dependent excluded, blocker included |
| D2 | Closed blocker | Dependent included |
| D3 | Tombstone blocker | Dependent included |
| D4 | "done" status blocker | Dependent included |
| D5 | "completed" status blocker | Dependent included |
| D6 | Two open blockers | Dependent excluded, both blockers included |
| D7 | One open + one closed blocker | Dependent excluded |
| D8 | All blockers closed | Dependent included |
| D9 | Transitive chain (A→B→C) | Only A ready |
| D10 | Diamond (A→B, A→C, B→D, C→D) | Only A ready |

### Dependency Type Behavior

| Test ID | Dependency Type | Expected Behavior |
|---------|----------------|-------------------|
| T1 | blocks | Dependent excluded until blocker closed |
| T2 | parent-child | Child excluded until parent closed |
| T3 | conditional-blocks | Dependent excluded |
| T4 | waits-for | Waiter excluded |
| T5 | relates-to | Both beads ready (non-blocking) |
| T6 | related | Both beads ready |
| T7 | discovered-from | Both beads ready |

### Attribute Filtering

| Test ID | Attribute | Expected Behavior |
|---------|-----------|-------------------|
| A1 | ephemeral=1 | Excluded |
| A2 | pinned=1 | Excluded |
| A3 | is_template=1 | Excluded |
| A4 | deleted_at IS NOT NULL | Excluded |

### Ordering and Priority

| Test ID | Scenario | Expected Order |
|---------|----------|----------------|
| O1 | P0 newer vs P1 older | P0 first (priority trumps age) |
| O2 | Same priority, different ages | Older first (FIFO) |
| O3 | NULL priority vs numbered | NULL priority last (999) |
| O4 | Zero-float P4 vs high-priority non-critical | Zero-float P4 wins (critical path dominates) |

### Critical Path Float

| Test ID | Scenario | Expected Float |
|---------|----------|----------------|
| F1 | Linear chain A→B→C→D | All zero (critical path) |
| F2 | Diamond with equal paths | All zero |
| F3 | Diamond with extra downstream bead | Shorter path gets positive float |
| F4 | Parallel tasks | Both zero (no slack) |

### Regression Tests

| Test ID | Issue | Scenario |
|---------|-------|----------|
| R1 | bf-wre | Custom "completed" status blocker must unblock dependent |
| R2 | bf-1nprw | Zero-dependency open beads must appear in ready output |

### Edge Cases

| Test ID | Scenario | Expected Behavior |
|---------|----------|-------------------|
| E1 | Empty workspace | Empty ready queue |
| E2 | Only closed beads | Empty ready queue |
| E3 | Assigned open bead | Included (assignment doesn't affect readiness) |
| E4 | No dependencies | All open beads ready |
| E5 | Self-block attempt | Rejected by add_dependency |
| E6 | limit=0 parameter | Unlimited results (no LIMIT clause) |
| E7 | Bead with closed blocker becomes open | Appears in next ready query |

## Velocity-Aware Scoring (Advanced)

When velocity statistics are available:

### Expected Duration Lookup

Query matches on `(issue_type, model, harness)`:
```sql
SELECT p50_seconds
FROM velocity_stats
WHERE issue_type = ?1
  AND model = ?2
  AND harness = ?3
```

### Scoring Formula

```
velocity_score = combined_score / COALESCE(p50_seconds, 1800)
```

**Effect:** A P0 bug with 5-minute p50 will outrank a P0 feature with 2-hour p50 when both have similar downstream impact.

### Without Velocity Data

Reverts to standard ordering:
1. downstream_impact DESC
2. critical_path_bonus DESC
3. priority ASC
4. created_at ASC

## Performance Considerations

### blocked_issues_cache Purpose

Without the cache, the ready query would require an O(n×m) correlated subquery:
```sql
-- SLOW: checks every bead against every dependency
SELECT i.id
FROM issues i
WHERE NOT EXISTS (
    SELECT 1 FROM dependencies d
    WHERE d.issue_id = i.id
      AND d.depends_on_id IN (
        SELECT id FROM issues WHERE status NOT IN (terminal)
      )
)
```

With the cache, it's an O(n) indexed lookup:
```sql
-- FAST: single indexed NOT IN check
SELECT i.id
FROM issues i
WHERE i.id NOT IN (SELECT issue_id FROM blocked_issues_cache)
```

### Index Requirements

Essential indexes for ready query performance:
- `issues(status, ephemeral, pinned, is_template, deleted_at)`
- `blocked_issues_cache(issue_id)`
- `dependencies(depends_on_id, type)` for downstream impact count
- `critical_path_cache(bead_id, float)` for bonus calculation

## Migration and Backward Compatibility

### Terminal Status Detection

The ready query uses a hardcoded SQL list for terminal statuses:
```sql
i.status NOT IN ('closed', 'tombstone', 'done', 'completed')
```

**Custom terminal statuses** (like "completed" from bf-wre) require explicit inclusion here.

### Status.is_terminal() Method

The Rust `Status::is_terminal()` method must match the SQL list:
```rust
pub fn is_terminal(&self) -> bool {
    matches!(self, Status::Closed | Status::Tombstone | Status::Custom(s) if s == "done" || s == "completed")
}
```

**Critical:** These two lists MUST stay synchronized to avoid bugs like bf-wre.

## Related Commands

### `bf ready`
Lists ready candidates using `get_ready_candidates()`, optionally limited and formatted.

### `bf claim`
Claims the top ready candidate atomically using `claim()` in a `BEGIN IMMEDIATE` transaction.

### `bf claim-any`
Scores candidates across multiple workspaces, claims the global winner.

## Implementation References

- **Core logic:** `src/claim.rs` — `claim()`, `get_ready_candidates()`, `Score` struct
- **Critical path:** `src/critical_path.rs` — `compute_all_critical_paths()`
- **Cache maintenance:** `src/storage/sqlite.rs` — dependency/status updates rebuild `blocked_issues_cache`
- **Ready command:** `src/cli/ready.rs` — `run_ready()`
- **Tests:** `src/storage/sqlite/ready_queue_tests.rs` — comprehensive test coverage

## Future Enhancements

Potential improvements to the ready queue:

1. **Multi-variant support** — Score different (model, harness) combinations separately
2. **Worker-specific queues** — Ready lists filtered by worker capability tags
3. **Time-of-day weighting** — Boost certain priority levels during specific hours
4. **Machine learning scoring** — Train on historical completion data to predict true impact
5. **User preference override** — Allow manual priority boost/penalty for specific beads

---

**Document Version:** 1.0  
**Last Updated:** 2026-08-10  
**Maintained By:** bead-forge development team
