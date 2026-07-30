# JSONL Compatibility Tests - bf-9xps6

## Task
Fix JSONL compatibility tests.

## Verification

All 30 JSONL compatibility tests pass successfully:

```
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
```

## Test Coverage

The `tests/jsonl_compat.rs` file tests:

1. **Round-trip tests**: Simple bead, all fields, dependencies, closed beads
2. **Import tests**: Empty files, mixed statuses, comments, issue types, optional fields
3. **Export tests**: Dirty-only export
4. **Content hash tests**: Recomputation and change detection
5. **Timestamp preservation**: created_at, updated_at
6. **Unicode and special characters**: UTF-8 encoding, JSON escaping
7. **Issue types**: task, bug, feature, epic

## Implementation Status

The following methods are correctly implemented in `src/storage/sqlite.rs`:

- `sync_from_jsonl()` - Imports issues from JSONL file
- `sync_to_jsonl()` - Exports issues to JSONL (with dirty_only option)
- `list_all_issues()` - Lists all issues in the database
- `count_issues()` - Counts total issues
- `mark_dirty()` - Marks an issue as dirty for export
- `clear_dirty()` - Clears all dirty flags

The `src/jsonl.rs` module provides the core JSONL import/export primitives:

- `import_jsonl()` - Generic import with upsert callback
- `export_jsonl()` - Full export (all issues)
- `export_jsonl_merge()` - Incremental/dirty-only export
- `export_jsonl_dirty()` - Dirty-only export with auto-clear

## Conclusion

No changes were required. All JSONL compatibility functionality is correctly implemented and all tests pass.
