# bf-54a: Velocity-Aware Claim Scoring Verification

## Finding

Velocity-aware claim scoring **is already implemented** in `src/claim.rs` (lines 148-183).

## Implementation Details

### Trigger Condition
Velocity-aware scoring activates when both `--model` and `--harness` flags are provided during claim:

```rust
if model.is_some() && harness.is_some() {
    // Velocity-aware query
}
```

### SQL Query (lines 155-183)
```sql
SELECT i.id
FROM issues i
LEFT JOIN dependencies d ON d.depends_on_id = i.id
    AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
LEFT JOIN critical_path_cache c ON c.bead_id = i.id
LEFT JOIN velocity_stats vs ON vs.issue_type = i.issue_type
    AND vs.model = ?1
    AND vs.harness = ?2
WHERE i.status = 'open'
  AND i.ephemeral = 0
  AND i.pinned = 0
  AND i.is_template = 0
  AND i.deleted_at IS NULL
  AND NOT EXISTS (
      -- no open blockers subquery
  )
GROUP BY i.id
ORDER BY (
    COALESCE(COUNT(d.issue_id), 0) * 3.0           -- downstream_impact * 3
    + (4 - i.priority) * 2.0                        -- priority_weight * 2
    + 1000.0 / (COALESCE(c.float, 999) + 1)        -- critical_path_bonus
) / COALESCE(vs.p50_seconds, 1800) DESC            -- divide by p50 with fallback
LIMIT 1
```

### Scoring Formula
```
score = (impact * 3.0 + (4 - priority) * 2.0 + critical_path_bonus) / p50_seconds
```

Where:
- **impact**: `COUNT(d.issue_id)` — number of downstream blocked beads
- **priority_weight**: `(4 - priority)` — inverts priority (Critical=0 → weight=4, Backlog=4 → weight=0)
- **critical_path_bonus**: `1000.0 / (float + 1)` — from critical_path_cache, higher for critical-path beads
- **p50_seconds**: `vs.p50_seconds` from velocity_stats, defaults to 1800s (30 min) when no data

### Key Features
1. **JOIN with velocity_stats** on `(model, harness, issue_type)` triple
2. **Fallback behavior**: Uses `COALESCE(vs.p50_seconds, 1800)` when no velocity data exists
3. **Throughput optimization**: Maximizes `(impact per second)` not just raw priority
4. **Standard fallback**: When model/harness not provided, uses original scoring without velocity division

## Verification Status
✅ Velocity-aware scoring is correctly implemented
✅ Uses velocity_stats table as specified
✅ Formula matches plan §4B.6 intent
✅ Graceful fallback to 1800s default when no velocity data
✅ Code compiles without errors
