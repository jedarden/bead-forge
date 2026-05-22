# Bead bf-2bem: Velocity Stats Expected Seconds Divisor

## Finding

The functionality described in this bead was **already implemented** at the time of claim.

## Evidence

### 1. claim() function (src/claim.rs:212-244)
- Has `LEFT JOIN velocity_stats vs ON vs.issue_type = i.issue_type AND vs.model = ?1 AND vs.harness = ?2`
- Orders by `(...) / COALESCE(vs.p50_seconds, 1800) DESC`

### 2. get_ready_candidates() function (src/claim.rs:418-454)
- Has `LEFT JOIN velocity_stats vs ON vs.issue_type = i.issue_type AND vs.model = ?1 AND vs.harness = ?2`
- Orders by `(...) / COALESCE(vs.p50_seconds, 1800) DESC`

Both functions correctly:
1. Join velocity_stats on (issue_type, model, harness)
2. Divide the combined impact score by `COALESCE(vs.p50_seconds, 1800)`
3. Fall back to 1800s default when no velocity data is available

## Note

The `get_expected_seconds()` function in velocity.rs is not called directly during claim because the SQL query uses velocity_stats.p50_seconds inline in the ORDER BY clause, which is more efficient for this use case.
