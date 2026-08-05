# Verification: NEEDLE Explore Strand Excludes Assigned Beads

## Task Summary

Verified that NEEDLE's explore strand correctly excludes beads with non-empty assignees from the candidate pool.

## Test Bead Details

**Bead ID:** bf-bheo5h
**Title:** Test bead for stale assignee simulation
**Status:** open
**Priority:** P2
**Assignee:** dead-worker-X (stale assignee simulation)

## Verification Results

### 1. Bead Appears in General Queries ✅

```bash
$ bf list | grep bf-bheo5h
[bf-bheo5h] Test bead for stale assignee simulation - open (P2)
```

The bead is present in the general bead list and queryable by ID.

### 2. Bead Details Confirmed ✅

```bash
$ bf show bf-bheo5h --format json --envelope | jq '.data | {id, assignee, status}'
{
  "id": "bf-bheo5h",
  "assignee": "dead-worker-X",
  "status": "open"
}
```

### 3. Bead Excluded from Discoverable List ✅

```bash
$ bf ready | grep bf-bheo5h
# (no output - bead is excluded)
```

**Critical Finding:** The bead with assignee `dead-worker-X` does NOT appear in the `bf ready` output. This confirms that the NEEDLE explore strand exclusion logic is implemented and working correctly.

## NEEDLE Explore Strand Logic

According to `docs/stale-assignee-workflow.md`, NEEDLE's explore strand uses SQL queries like:

```sql
SELECT id, title, priority FROM issues 
WHERE status = 'open' 
AND assignee IS NULL  -- Only unassigned beads
AND id NOT IN (SELECT blocked FROM dependencies WHERE blocker IN (...))
ORDER BY priority, created_at;
```

**Key Point:** The `assignee IS NULL` filter automatically excludes beads with non-NULL assignees from the candidate pool.

## Implementation Status

✅ **Exclusion logic is implemented and working correctly**

- The `bf ready` command (which NEEDLE uses) correctly filters out beads with non-empty assignees
- Beads with stale assignees are invisible to the fleet until the assignee is cleared
- This is confirmed by:
  1. Test bead exists in general queries (`bf list`)
  2. Test bead does NOT appear in discoverable queries (`bf ready`)
  3. Documentation confirms the SQL filter `WHERE assignee IS NULL`
  4. Comprehensive test suite validates the workflow: `tests/stale_assignee_clearing_workflow.rs`

## Impact

This behavior is **by design** and essential for NEEDLE's concurrent claiming model:

1. **Prevents duplicate claims:** Multiple workers cannot claim the same bead
2. **Enables worker crash recovery:** When a worker crashes, beads remain assigned to the dead worker
3. **Requires manual remediation:** Stale assignees must be cleared with `bf update --clear-assignee`
4. **Maintains data integrity:** No race conditions in the candidate pool

## Related Documentation

- Full workflow: `docs/stale-assignee-workflow.md`
- Test implementation: `tests/stale_assignee_clearing_workflow.rs`
- CLI reference: `docs/README.md`

## Verification Date

2026-08-05
