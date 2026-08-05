# NEEDLE Explore Strand Assignee Filtering Investigation (bf-5nflyq)

## Task Summary
Investigate NEEDLE explore strand assignee filtering logic to understand how it handles beads with assignees.

## Key Findings

### 1. Primary Filtering Location
**File:** `/home/coding/bead-forge/src/claim.rs` - `get_ready_candidates()` function (lines 218-240)

### 2. SQL Query Analysis
The main SQL query in `get_ready_candidates()` does **NOT** include explicit assignee filtering:

```sql
SELECT i.id
FROM issues i
WHERE i.status = 'open'
  AND i.ephemeral = 0
  AND i.pinned = 0
  AND i.is_template = 0
  AND i.deleted_at IS NULL
  AND i.id NOT IN (SELECT issue_id FROM blocked_issues_cache)
GROUP BY i.id
ORDER BY ...
```

**Critical observation:** There is no `AND i.assignee IS NULL` clause in the WHERE condition.

### 3. How Assignee Filtering Actually Works

**Implicit filtering through claim mechanism:**
1. When a bead is claimed via `bf claim`, it gets:
   - `status = 'in_progress'`
   - `assignee = <worker_id>`
2. The query only selects `status = 'open'`, so claimed beads are excluded
3. The claim function (lines 249-252) atomically updates both status and assignee:
   ```sql
   UPDATE issues
   SET status = 'in_progress', assignee = ?, updated_at = ?
   WHERE id = ? AND status = 'open'
   ```

**Stale assignee reclamation:**
- The claim function automatically reclaims stale beads (lines 188-194):
  ```sql
  UPDATE issues
  SET status = 'open', assignee = NULL, updated_at = ?
  WHERE status = 'in_progress'
    AND updated_at < ?
  ```

### 4. Edge Case: Open Beads with Non-Null Assignees

**Important:** Beads can exist in a state where:
- `status = 'open'` 
- `assignee = 'some-worker'` (non-null)

This scenario can occur when:
- A bead is manually created with an assignee
- A bead's assignee is set via `bf update --assignee <worker>` while status='open'
- After stale reclamation, if the assignee was manually reset

**Test confirmation (from tests/ready_json_fields.rs:12-14):**
> "`ready` does NOT filter on assignee/labels (it selects on status='open'), so
> an open bead carrying a stale assignee and a label appears in the output WITH
> those fields populated."

### 5. Storage Layer Filtering Capability

**File:** `/home/coding/bead-forge/src/storage/sqlite.rs` - `list_issues()` function (lines 240-249)

The storage layer **DOES** support assignee filtering:

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

However, this capability is **NOT used** by the `get_ready_candidates()` function.

### 6. Documentation Confirmation

**File:** `/home/coding/bead-forge/docs/stale-assignee-workflow.md`

Lines 5-6 state:
> "A bead with a non-empty `assignee` field is excluded from the ready/claim list,
> effectively making it invisible to the fleet."

This documentation appears to describe the **intended** behavior rather than the **actual** implementation, as the SQL query does not explicitly enforce this exclusion.

## Acceptance Criteria Verification

1. ✅ **Located NEEDLE explore strand filtering code:** Found in `src/claim.rs::get_ready_candidates()`
2. ✅ **Identified assignee checking logic:** The function does NOT explicitly check assignee field
3. ✅ **Determined if assigned beads are excluded:** Assigned beads are implicitly excluded through the claim mechanism (status='in_progress'), but open beads with non-null assignees would still appear
4. ✅ **Noted filtering implementation status:** The assignee filtering is **not implemented** in the main SQL query; it relies on implicit behavior through status transitions

## Conclusion

The NEEDLE explore strand does **NOT** have explicit assignee filtering in its main candidate selection query. The filtering is implicit through the claim mechanism that changes bead status from 'open' to 'in_progress' when assigned. However, this means:

- Beads in `status='open'` with non-null assignees **WILL** appear in the ready candidate list
- NEEDLE workers must perform additional client-side filtering if they want to exclude assigned beads
- The documentation describes intended behavior that differs from the actual implementation

**Recommendation:** If explicit assignee filtering is required for NEEDLE's explore strand, the SQL query in `get_ready_candidates()` should include `AND i.assignee IS NULL` in its WHERE clause.
