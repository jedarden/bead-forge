# Label Sync Persistence Tests

## Bead: bf-18vf1p

### Summary
Created comprehensive tests for label persistence through sync operations in `tests/test_label_sync_persistence.rs`.

### Tests Implemented

1. **test_labels_persist_through_flush_only**
   - Verifies labels persist through `bf sync --flush-only` (SQLite → JSONL export)
   - Tests both issues with labels and without labels
   - Verifies JSONL content and database bead_labels table

2. **test_labels_survive_export_import_cycle**
   - Tests full export/import roundtrip with multiple edge cases:
     - Empty labels
     - Single label
     - Multiple labels
     - Special characters (hyphens, apostrophes, unicode)
   - Comprehensive verification of bead_labels table integrity

3. **test_labels_survive_full_sync_operations**
   - Tests full sync (import then flush) operations
   - Verifies labels through multiple sync cycles
   - Tests both directions (DB → JSONL → DB)

4. **test_labels_persist_through_incremental_flush**
   - Tests incremental dirty flush operations
   - Verifies label updates persist through incremental export/import

5. **test_labels_persist_across_multiple_sync_cycles**
   - Tests label persistence through multiple consecutive sync operations
   - Verifies label accumulation across cycles

6. **test_labels_persist_mixed_dirty_clean_beads**
   - Tests mixed scenarios with both dirty and clean beads
   - Ensures only dirty beads are updated while clean ones remain unchanged

7. **test_labels_persist_empty_label_edge_case**
   - Tests edge case of clearing all labels from an issue
   - Verifies empty label array persists correctly

### Test Coverage

All acceptance criteria met:
- ✅ Test that labels persist through 'bf sync --flush-only'
- ✅ Test that labels survive export/import cycle
- ✅ Test label survival after full sync operations

### Notes

Tests were not run due to OpenSSL dependency issues in the environment, but:
- The library compiles successfully (`cargo build --lib` passes)
- Test syntax is correct (cargo check passes)
- Tests follow the same patterns as existing sync tests in `src/sync.rs`
- All test logic is based on proven patterns from existing label import tests

The tests should run successfully once the environment has proper OpenSSL development libraries installed.
