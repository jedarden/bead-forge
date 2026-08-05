# NEEDLE Explore Strand: Assigned Bead Exclusion Behavior

## Summary

**The NEEDLE explore strand DOES exclude assigned beads.** The exclusion logic is **fully implemented and operational**.

## Implementation Details

### Code Location
- **File:** `/home/coding/NEEDLE/src/strand/explore.rs`
- **Primary filtering:** Lines 617-622
- **Retry filtering (after cross-workspace mend):** Lines 657-665

### Filtering Mechanism

The explore strand uses a **defensive belt-and-suspenders filtering approach**:

```rust
// Defensive belt-and-suspenders filtering.
// The store.ready() method receives exclude_labels in its Filters,
// but some backend implementations may not filter correctly.
// This ensures excluded/assigned beads are never returned as candidates.
candidates.retain(|b| {
    let assignee_ok = b.assignee.is_none();
    let labels_ok = !b.labels.iter().any(|l| filters.exclude_labels.contains(l));
    assignee_ok && labels_ok
});
```

### Filter Configuration

The explore strand configures `Filters` with:
- `assignee: None` - No assignee filter at store level (defensive filter handles it)
- `exclude_labels: ["deferred", "human", "blocked"]` - Labels to exclude
- `exclude_ids: HashSet::new()` - No specific ID exclusions

### Exclusion Logic

A bead is **excluded** if it fails **EITHER** condition:
1. **Assigned:** `b.assignee.is_none()` returns `false`
2. **Excluded label:** Bead has any of "deferred", "human", or "blocked" labels

Only beads where **BOTH conditions pass** are returned as candidates.

### Cross-Workspace Mend Integration

When no ready candidates are found initially, the explore strand:
1. Runs `cleanup_orphaned_in_progress()` to release orphaned beads
2. Re-queries the store with the same filters
3. **Applies the same defensive filtering** to the retry results
4. Only returns beads that pass both assignee and label checks

This ensures that even after releasing orphaned beads, only truly unassigned and unlabeled beads are claimed.

## Test Coverage

The implementation has **comprehensive unit test coverage** including:

### 1. Basic Exclusion Test
- **Test:** `deadlock_scenario_assigned_beads_allow_advancement()`
- **Scenario:** Workspace 1 has only assigned beads, Workspace 2 has valid unassigned beads
- **Expected:** Strand advances past Workspace 1 and returns Workspace 2's candidates
- **Status:** ✅ Implemented and passing

### 2. Label Exclusion Test
- **Test:** `deadlock_scenario_excluded_beads_allow_advancement()`
- **Scenario:** Workspace 1 has beads with "blocked" label, Workspace 2 has valid beads
- **Expected:** Strand filters out labeled beads and advances to Workspace 2
- **Status:** ✅ Implemented and passing

### 3. Edge Case: Both Assigned AND Labeled
- **Test:** `deadlock_scenario_excluded_and_assigned_beads_allow_advancement()`
- **Scenario:** Workspace 1 has beads that are BOTH assigned AND labeled, Workspace 2 has valid beads
- **Expected:** Strand filters out doubly-unclaimable beads and advances to Workspace 2
- **Status:** ✅ Implemented and passing

### 4. Aggregation Test
- **Test:** `aggregates_candidates_across_all_workspaces()`
- **Scenario:** Multiple workspaces each have valid candidates
- **Expected:** Strand aggregates candidates from ALL workspaces in a single cycle
- **Status:** ✅ Implemented and passing (fixes bf-4df1e / bf-47bfm)

## Design Rationale

The defensive filtering exists because:
1. **Backend implementations may vary:** Different bead store implementations (bf, br) may not all filter correctly
2. **Data consistency:** Ensures explore strand never returns beads that can't be claimed
3. **Fleet-wide starvation prevention:** A single workspace with bad candidates must not block the entire fleet from scanning other workspaces (fixed in bf-4df1e / bf-47bfm)

## Related Beads

- **bf-4df1e / bf-47bfm:** Fixed fleet starvation where early-excluded beads prevented scanning remaining workspaces
- **bf-1d64q:** Documented the multi-workspace deadlock scenario
- **bf-6anj4:** Implemented per-cycle workspace shuffle for better de-herding

## Conclusion

The NEEDLE explore strand **correctly and consistently excludes assigned beads** through:
1. Primary defensive filtering after initial `ready()` query
2. Identical defensive filtering after cross-workspace mend retry
3. Comprehensive test coverage of all exclusion scenarios
4. Multi-workspace aggregation to prevent starvation

**Status:** ✅ Fully implemented, tested, and operational.
