# Bead bf-2wqp: Critical Path Cache Integration

## Finding

The critical_path_cache integration described in this bead was **already implemented** in the codebase.

## Verification

The `get_ready_candidates` function in `src/claim.rs` (lines 359-405) contains:

1. **LEFT JOIN critical_path_cache** (line 367):
   ```sql
   LEFT JOIN critical_path_cache c ON c.bead_id = i.id
   ```

2. **Float bonus calculation** (line 363):
   ```sql
   1000.0 / (COALESCE(c.float, 999) + 1) as critical_path_bonus
   ```

3. **ORDER BY with bonus** (line 383):
   ```sql
   ORDER BY downstream_impact DESC, critical_path_bonus DESC, ...
   ```

4. **Result population** (line 399):
   ```rust
   critical_float: row.get(5)?  // Populated from critical_path_bonus
   ```

The same integration exists in:
- Velocity-aware claim query (line 171)
- Standard claim query (line 261)

## Behavior

- Zero-float beads get `1000.0 / (0 + 1) = 1000.0` bonus
- Float-5 beads get `1000.0 / (5 + 1) ≈ 166.7` bonus
- Non-critical beads get `1000.0 / (999 + 1) ≈ 1.0` bonus

This ensures critical-path beads (float=0) always outrank non-critical beads regardless of their listed priority, matching Plan §4B.5.

## Git History

The feature was implemented in commit `ac82d69` (2026-05-08):
```
feat(critical-path): complete critical path DAG float computation

Implements Plan §4B.5: Two-pass walk on dependency DAG using recursive CTEs.
- Claim scoring: 1000.0 / (float + 1) bonus for zero-float beads
- Cache recompute on dep add/remove and status changes
```

## Test Verification

All claim-related tests pass:
```
test claim::tests::test_critical_path_bonus_in_claim ... ok
test claim::tests::test_critical_path_zero_float_outranks_high_priority ... ok
test claim::tests::test_claim_basic ... ok
test claim::tests::test_claim_reclaims_stale ... ok
test claim::tests::test_concurrent_claim_no_double_claim ... ok
```

## Conclusion

No code changes were needed. The feature described in this bead was already fully implemented in May 2026 as part of Plan §4B.5.
