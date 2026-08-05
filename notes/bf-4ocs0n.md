# NEEDLE Explore Strand Assignee Exclusion Verification

## Task
Verify that the NEEDLE explore strand would exclude beads with non-empty assignees.

## Test Infrastructure Used
- **Test bead ID**: `bf-wu6wp4`
- **Title**: "Test bead with assignee for NEEDLE explore strand exclusion"
- **Assignee**: `test-agent`
- **Status**: `open`
- **Created by**: `bf-3joukp` (Child 1 of 4 split from bf-4ocs0n)

## Verification Results

### 1. ✓ Test bead appears in general queries
The test bead `bf-wu6wp4` appears in general `bf list` queries:
```bash
$ bf list --status open --type test | grep bf-wu6wp4
[bf-wu6wp4] Test bead with assignee for NEEDLE explore strand exclusion - open (P2)
```

### 2. ✗ NEEDLE explore strand does NOT exclude assigned beads

**Critical Finding**: The `get_ready_candidates()` function in `src/claim.rs` does **NOT** filter out beads with non-empty assignees, despite documentation stating it should.

#### What the code does (src/claim.rs:427-432)
```sql
WHERE i.status = 'open'
  AND i.ephemeral = 0
  AND i.pinned = 0
  AND i.is_template = 0
  AND i.deleted_at IS NULL
  AND i.id NOT IN (SELECT issue_id FROM blocked_issues_cache)
```

**Missing**: `AND i.assignee IS NULL` condition

#### What documentation expects (docs/stale-assignee-workflow.md:213-223)
```sql
SELECT id, title, priority FROM issues 
WHERE status = 'open' 
AND assignee IS NULL  -- Only unassigned beads
AND id NOT IN (SELECT blocked FROM dependencies WHERE blocker IN (...))
ORDER BY priority, created_at;
```

Documentation explicitly states:
> Line 5: "A bead with a non-empty `assignee` field is excluded from the ready/claim list, effectively making it invisible to the fleet."

### 3. Other parts support assignee filtering
The `list_issues()` function in `src/storage/sqlite.rs` DOES support assignee filtering (lines 241-250):
```rust
if let Some(ref assignee) = filter.assignee {
    if assignee.is_empty() {
        // Empty-string filter selects unassigned beads.
        query.push_str(" AND (i.assignee IS NULL OR i.assignee = '')");
    } else {
        query.push_str(&format!(" AND i.assignee = ?{}", param_idx));
        params.push(assignee.clone());
        param_idx += 1;
    }
}
```

## Conclusion

**Current Behavior**: NEEDLE explore strand does NOT exclude assigned beads.

**Root Cause**: Implementation gap in `src/claim.rs` - both velocity-aware and standard scoring queries in `get_ready_candidates()` are missing the `assignee IS NULL` filter condition.

**Impact**: Assigned beads with `status='open'` will appear in the ready/claim candidate list, contrary to documented NEEDLE explore strand behavior.

**Evidence from Previous Beads**:
- `bf-5nflyq`: Found that `get_ready_candidates()` does NOT explicitly filter by assignee
- `bf-5mdrqa`: Found that SQL WHERE clause does NOT filter by assignee field
- `bf-5hahhz`: Incorrectly claimed exclusion is implemented at explore.rs:617-622 (file does not exist)

## Recommendations

1. **Add assignee filtering** to both velocity-aware and standard scoring queries in `get_ready_candidates()`
2. **Add corresponding tests** to ensure assigned beads are excluded from ready/claim lists
3. **Update documentation** if the current behavior (no filtering) is intentional
4. **Review claim() function** to ensure it also excludes assigned beads (though it transitions status to 'in_progress', which should handle this implicitly)

## Files Involved

- `src/claim.rs` - Lines 219-240 (velocity-aware), 301-322 (standard)
- `src/storage/sqlite.rs` - Lines 241-250 (assignee filtering in list_issues)
- `docs/stale-assignee-workflow.md` - Documentation expecting assignee filtering
- `tests/stale_assignee_clearing_workflow.rs` - End-to-end tests for the workflow

Test Date: 2026-08-05
Investigated By: bf-4ocs0n verification task
