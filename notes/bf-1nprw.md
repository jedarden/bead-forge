# Investigation Report: bf ready returning zero results (bf-1nprw)

## Issue Summary

**Reported:** 2026-07-20  
**Observed:** `bf ready --limit 500 --json` returned empty array despite 54 open-status beads  
**Investigated:** 2026-07-25

## Investigation Findings

### Current State (2026-07-25)
- **Total open beads:** 55
- **Unblocked (ready):** 35 beads  
- **Blocked (not ready):** 20 beads
- **`bf ready` correctly returns:** 35 beads

### Root Cause Analysis

The original bug report was **NOT a bug in the query** - the `bf ready` query is working correctly.

The query correctly filters beads using this logic:
```sql
AND NOT EXISTS (
    SELECT 1 FROM dependencies blocker_dep
    INNER JOIN issues blocker ON blocker.id = blocker_dep.depends_on_id
    WHERE blocker_dep.issue_id = i.id
    AND blocker_dep.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
    AND blocker.status NOT IN ('closed', 'tombstone', 'done', 'completed')
)
```

### Why the Original Report Showed Zero Results

On 2026-07-20, all 54 open beads genuinely had unclosed blockers. Sample beads mentioned in the report:

1. **bf-127ow "Test Epic 1"** (now closed)
   - Was blocked by: `bf-ncms2` (status=blocked)
   - "blocked" is NOT a terminal status → correctly filtered out

2. **bf-s9tt7 "Test epic creation"** (now closed)  
   - Was blocked by: `bf-2mjro` (status=blocked)
   - Same issue as above

### Terminal Status List

The query treats these as terminal (non-blocking):
- `closed`
- `tombstone`
- `done`
- `completed`

Statuses that **DO block**:
- `open`
- `blocked`
- `in_progress`

### Regression Test

A comprehensive regression test was added in commit `b78b254`:
- `test_ready_includes_zero_dependency_open_beads_bf_1nprw`
- Tests that standalone open beads with zero dependencies appear in ready output
- Tests that beads blocked by open/blocker blockers are correctly excluded
- Tests that beads blocked by status=blocked blockers are correctly excluded
- **Status:** ✅ PASSING

## Conclusion

**No bug found.** The original report was a case where all open beads genuinely had unclosed blockers. The query logic is correct and the regression test confirms this behavior is working as intended.

## Files Examined

- `src/claim.rs` - Contains `get_ready_candidates()` function used by `bf ready`
- `src/claim.rs:1172-1237` - Regression test
- `.beads/beads.db` - Verified query behavior directly

## Verification Steps Performed

1. Ran `bf ready --limit 500 --json` → returns 35 beads ✅
2. Counted open beads: 55 total
3. Counted unblocked beads via direct SQL: 35 ✅  
4. Verified regression test passes ✅
5. Sampled blocked beads and confirmed their blockers have non-terminal statuses ✅
