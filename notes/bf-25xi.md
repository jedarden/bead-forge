# bf-25xi: Test velocity_stats empty fallback

## Summary

The test `test_claim_fallback_to_1800s_when_velocity_stats_empty` already exists in `/home/coding/bead-forge/tests/claim_fallback.rs` (lines 448-512).

## Test Verification

The test correctly verifies that `bf claim` falls back to the 1800s default when velocity_stats is empty for an unknown model/harness combination:

1. ✅ Creates a TempWorkspace with test beads
2. ✅ Sets WorkerMetadata with `model: Some("unknown-model")` and `harness: Some("unknown-harness")`
3. ✅ Calls `bead_forge::claim::claim()` with the unknown model/harness
4. ✅ Asserts a bead is successfully claimed (no error)
5. ✅ Verifies velocity_stats table remains empty for the model/harness
6. ✅ Test passes

## Implementation Details

The fallback logic is implemented in `src/claim.rs:242` using:
```sql
) / COALESCE(vs.p50_seconds, 1800) DESC
```

When no matching row exists in velocity_stats for the (model, harness, issue_type) tuple, COALESCE returns 1800 as the default p50_seconds value.

## Status

Test already implemented and passing. No code changes required.
