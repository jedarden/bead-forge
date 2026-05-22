# Bead bf-2wqp Verification: Critical Path Cache Integration

## Task
Verify that claim score integrates critical_path_cache float bonus as specified in plan §4B.5.

## Findings

The critical_path_cache integration was **already implemented** in `src/claim.rs`. All three claim queries properly join and use the cache:

### 1. Velocity-aware claim (lines 166-194)
```sql
LEFT JOIN critical_path_cache c ON c.bead_id = i.id
...
ORDER BY (
    COALESCE(COUNT(d.issue_id), 0) * 3.0
    + (4 - i.priority) * 2.0
    + 1000.0 / (COALESCE(c.float, 999) + 1)
) / COALESCE(vs.p50_seconds, 1800) DESC
```

### 2. Standard claim (lines 254-281)
```sql
LEFT JOIN critical_path_cache c ON c.bead_id = i.id
...
1000.0 / (COALESCE(c.float, 999) + 1) as critical_path_bonus
...
ORDER BY
    downstream_impact DESC,
    critical_path_bonus DESC,
    i.priority ASC,
    i.created_at ASC
```

### 3. get_ready_candidates (lines 360-405)
```sql
LEFT JOIN critical_path_cache c ON c.bead_id = i.id
...
1000.0 / (COALESCE(c.float, 999) + 1) as critical_path_bonus
...
ORDER BY
    downstream_impact DESC,
    critical_path_bonus DESC,
    i.priority ASC,
    i.created_at ASC
```

## Verification

- All claim tests pass, including:
  - `test_critical_path_bonus_in_claim` — verifies zero-float beads get bonus=1000
  - `test_critical_path_zero_float_outranks_high_priority` — verifies bonus outweighs priority

## Conclusion

No code changes required. The bead description appears to have been written against an earlier version of the code where this integration was missing. The implementation now matches plan §4B.5 specification.
