# Labels Export to JSONL - Verification (bf-1sbr0a)

## Task
Export labels to JSONL during sync flush

## Status: ✅ ALREADY IMPLEMENTED

## Implementation Details

### 1. Database Layer (src/storage/sqlite.rs)
- **Line 280, 191, 305**: All list queries include `GROUP_CONCAT(bl.label) AS labels`
- **Lines 990-993**: Labels parsed from comma-separated GROUP_CONCAT result
- **Lines 1039-1047**: `load_labels_conn()` helper for loading individual labels

### 2. Model Layer (src/model.rs) 
- **Line 558**: `pub labels: Vec<String>` with serde serialization attributes
- Uses `#[serde(skip_serializing_if = "Vec::is_empty", default)]` to skip empty arrays

### 3. JSONL Export (src/jsonl.rs)
- **Lines 308-360**: Comprehensive tests verify labels export and roundtrip
- Labels are automatically included via Serde serialization of `Issue` struct
- No special handling needed - just serializes the `labels` field

### 4. Sync Layer (src/sync.rs)
- **Lines 601-724**: Tests verify labels import from JSONL
- **Lines 601-677**: `test_labels_import_from_jsonl()` confirms labels roundtrip

## Acceptance Criteria Verification

✅ **Labels queried from bead_labels table**: All queries use `LEFT JOIN bead_labels` + `GROUP_CONCAT`
✅ **Labels included in JSONL output**: Serde automatically serializes `labels` field
✅ **All labels exported (no truncation)**: GROUP_CONCAT doesn't truncate by default
✅ **Atomic with bead data**: Labels are part of same `Issue` struct and transaction
✅ **Backward-compatible with br**: Uses standard JSON array format

## Test Coverage

- `jsonl.rs::labels_are_exported_to_jsonl()` - Basic export test
- `jsonl.rs::labels_roundtrip_through_jsonl()` - Roundtrip test  
- `jsonl.rs::empty_labels_array_skipped_in_jsonl()` - Empty array handling
- `sync.rs::test_labels_import_from_jsonl()` - Full sync with labels
- `sync.rs::test_labels_import_idempotent()` - Import idempotence

## Code Paths

**Export path**:
```
sync::flush() → storage.list_all_issues() → row_to_issue_conn() → export_jsonl() → Issue serde::serialize
```

**Labels flow**:
1. `list_all_issues()` joins `bead_labels` table
2. SQLite aggregates with `GROUP_CONCAT(bl.label)`  
3. `row_to_issue_conn()` parses comma-separated result into `Vec<String>`
4. `export_jsonl()` serializes entire `Issue` struct to JSONL
5. Serde automatically includes `labels` array in JSON output

## Conclusion

Labels export to JSONL during sync flush is **fully implemented and tested**. 
No additional changes needed - the task requirements are already met by the existing codebase.
