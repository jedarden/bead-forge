# Bug bf-1iz5: count_unflushed Over-Reporting After Repair-Import

## Issue Description

After running `doctor --repair` followed by `import`, `count_unflushed` equals the imported count even though `drift == 0`. The underlying cause is that `export_hashes` table is not populated during import/repair operations.

## Root Cause

When `repair()` or `import()` creates beads from JSONL:

1. They use `create_issue_tx()` which correctly does NOT mark beads dirty
2. They call `clear_dirty()` which correctly clears dirty marks
3. **BUT** they never populate the `export_hashes` table

This creates an inconsistent state where:
- `dirty_issues` is empty → `count_unflushed() == 0` ✓
- DB matches JSONL → `drift == 0` ✓
- **BUT** `export_hashes` is empty ✗

## Why `export_hashes` Matters

The `export_hashes` table tracks which beads have been exported to JSONL and their content hashes. It's used for:

1. **Incremental export detection** - `sync --flush` uses it to detect changed beads
2. **Efficiency** - Only re-export beads that changed since last export
3. **Consistency** - Ensures JSONL and DB stay in sync

When `export_hashes` is empty after import:
- Next `sync --flush` may behave unexpectedly
- All beads appear as "never exported"
- Performance degradation on large workspaces

## The Fix

After importing beads from JSONL, populate `export_hashes` for all imported beads:

```rust
// In repair() after line 418
storage.clear_dirty()?;

// NEW: Populate export_hashes since these beads came from JSONL
let all_issues = storage.list_all_issues()?;
update_export_hashes_for_issues(&storage, &all_issues)?;

// In import() after line 193
storage.clear_dirty()?;

// NEW: Populate export_hashes for all imported/updated beads
update_export_hashes_for_issues(&storage, &all_issues)?;
```

## Implementation Locations

1. **src/doctor.rs** - `repair()` function (after line 423)
2. **src/sync.rs** - `import()` function (after line 193)

Both functions need to call a helper that populates `export_hashes` for all beads currently in the database.

## Test Coverage

Existing tests already verify this behavior:
- `test_import_leaves_zero_unflushed` (doctor.rs:897)
- `test_repair_cycle_clears_unflushed_correctly` (doctor.rs:947)
- `test_import_clears_pre_existing_dirty_marks` (doctor.rs:992)

These tests verify `count_unflushed() == 0` after import, but don't verify `export_hashes` is populated. New tests should:
1. Verify `export_hashes` row count equals bead count after import
2. Verify hash values match the actual bead content hashes
