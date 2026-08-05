# NEEDLE Explore Strand Assignee Exclusion Investigation

## Task
Investigate NEEDLE explore strand logic to verify whether it excludes beads with non-empty assignees.

## Findings

### 1. Explore Strand Entry Point
The NEEDLE explore strand uses the `claim_bead()` function from `src/bead_store.rs`, which internally calls the `claim()` function from `src/claim.rs`.

### 2. Candidate Selection Query
The core logic is in `get_ready_candidates()` function at `src/claim.rs:404-550`. 

**SQL Query WHERE clause** (velocity-aware version, lines 427-432):
```sql
WHERE i.status = 'open'
  AND i.ephemeral = 0
  AND i.pinned = 0
  AND i.is_template = 0
  AND i.deleted_at IS NULL
  AND i.id NOT IN (SELECT issue_id FROM blocked_issues_cache)
```

**Standard version** (lines 484-489) - same WHERE clause:
```sql
WHERE i.status = 'open'
  AND i.ephemeral = 0
  AND i.pinned = 0
  AND i.is_template = 0
  AND i.deleted_at IS NULL
  AND i.id NOT IN (SELECT issue_id FROM blocked_issues_cache)
```

### 3. NO Assignee Filtering
**Critical Finding:** There is **NO assignee filtering** in the explore strand query. The WHERE clause does NOT include:
- `AND i.assignee IS NULL`
- `AND i.assignee = ''`
- Any assignee-related condition

### 4. Comparison with General List Function
The general `list_issues()` function in `src/storage/sqlite.rs:207-299` DOES support assignee filtering (lines 240-249):
```rust
if let Some(ref assignee) = filter.assignee {
    if assignee.is_empty() {
        query.push_str(" AND (i.assignee IS NULL OR i.assignee = '')");
    } else {
        query.push_str(&format!(" AND i.assignee = ?{}", param_idx));
        params.push(assignee.clone());
        param_idx += 1;
    }
}
```

But this filtering is **NOT used** by the explore strand's `get_ready_candidates()` function.

## Conclusion

**The NEEDLE explore strand does NOT exclude beads with non-empty assignees.**

Any bead with `status='open'` will appear in the explore strand candidate list, regardless of whether it has an assignee set. This means:

1. Beads with `assignee="some-worker"` but `status="open"` will be visible to NEEDLE explore
2. These beads can be claimed by other workers
3. This could lead to unintended claim behavior if beads are manually assigned but kept in "open" status

## Code Locations

- **Explore strand query:** `src/claim.rs:404-550` (get_ready_candidates function)
- **WHERE clause:** Lines 427-432 (velocity-aware), 484-489 (standard)
- **General list filtering:** `src/storage/sqlite.rs:240-249` (assignee filter in list_issues)
- **Issue model:** `src/model.rs:429-528` (assignee field at line 469-470)
