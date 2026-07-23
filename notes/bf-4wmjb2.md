# Label Export/Import Round-trip Verification (bf-4wmjb2)

## Summary
Verified that labels export correctly from bead_labels table to JSONL and can be imported back with complete data preservation.

## Test Results

### Integration Test (`test_labels_verify.sh`)
✅ **PASSED** - All acceptance criteria met:

1. **Export includes labels for beads** - Labels from bead_labels table are serialized to JSONL
   - Multi-label bead: `"labels":["critical","phase-1","storage","test-label"]`
   - Single-label bead: `"labels":["phase-2"]`
   - No-label bead: labels field skipped (as expected with `skip_serializing_if`)

2. **Import reads labels array and populates bead_labels table** - Import from JSONL correctly restores all labels
   - 5 labels imported back to bead_labels table
   - All label values preserved exactly

3. **Round-trip preserves all label data** - Full cycle test (create → export → delete → import → verify)
   - Label count: 5 before = 5 after ✓
   - Multi-label bead (bf-3jr): 4 labels preserved ✓
   - Single-label bead (bf-5u2): 1 label preserved ✓
   - No-label bead (bf-6vo): 0 labels preserved ✓

4. **Multi-label beads covered** - Test verified bead with 4 different labels
   - All labels stored in bead_labels table
   - All labels exported to JSONL
   - All labels imported back correctly
   - Specific label values verified: phase-1, storage, critical, test-label

5. **Re-export produces identical JSONL** - After import, re-exporting produces byte-for-byte identical JSONL

## Implementation Details

### Export Path
- `src/storage/sqlite.rs`: `get_issue()`, `list_issues()`, `list_all_issues()`, `list_dirty_issues()` use `GROUP_CONCAT(bl.label)` to query labels from `bead_labels` table
- `src/model.rs`: `Issue` struct has `labels: Vec<String>` field with `#[serde(skip_serializing_if = "Vec::is_empty")]`
- Labels serialized as JSON array in JSONL

### Import Path
- `src/jsonl.rs`: `import_jsonl()` parses JSONL and calls upsert callback
- `src/storage/sqlite.rs`: `sync_from_jsonl()` dispatches to `create_issue_tx()` (new) or `update_issue_from_json_tx()` (update)
- Both methods insert labels into BOTH `labels` table (br-compatible) AND `bead_labels` table (bf-specific)
- Lines 728-733 in sqlite.rs: Labels inserted during import

### Storage Schema
- `bead_labels` table: `CREATE TABLE IF NOT EXISTS bead_labels (bead_id TEXT NOT NULL REFERENCES issues(id) ON DELETE CASCADE, label TEXT NOT NULL, PRIMARY KEY (bead_id, label))`
- Separate from br-compatible `labels` table to avoid schema conflicts

## Test Script
Created `test_labels_verify.sh` that:
- Creates test beads with 0, 1, and 4 labels
- Exports to JSONL with `bf sync --flush-only`
- Deletes all issues from database
- Imports from JSONL with `bf sync --import-only`
- Verifies label counts and specific values
- Confirms re-export produces identical JSONL

## Conclusion
Label export and import round-trip is working correctly. All acceptance criteria verified and passed.
