# Label Export to JSONL - Implementation Verification

## Bead: bf-3ou3re

## Status: **ALREADY IMPLEMENTED**

Labels export to JSONL during flush is **already fully functional** in bead-forge.

## Implementation Timeline

- **2026-07-23 18:23:20** - Commit `ce84ead` (bead bf-3l64k2): "Add labels field to Issue JSON serialization"
- **2026-07-23 21:57:54** - Bead bf-3ou3re created (after implementation was complete)

## Implementation Components

### 1. Issue Struct (src/model.rs)
```rust
#[serde(skip_serializing_if = "Vec::is_empty", default)]
pub labels: Vec<String>,
```
- Labels field with proper serde attributes for JSON serialization
- Empty arrays are skipped in JSON output

### 2. Database Queries (src/storage/sqlite.rs)
All main query functions include label fetching:
- `get_issue()` - line 168-170
- `list_issues()` - line 191-193
- `list_all_issues()` - line 280-282
- `list_dirty_issues()` - line 305-308

Each uses:
```sql
GROUP_CONCAT(bl.label) AS labels
FROM issues i
LEFT JOIN bead_labels bl ON i.id = bl.bead_id
```

### 3. Label Parsing (src/storage/sqlite.rs:989-993)
```rust
let labels_str: Option<String> = row.get(36)?;
let labels: Vec<String> = labels_str
    .map(|s| s.split(',').map(String::from).collect())
    .unwrap_or_default();
```
Parses comma-separated labels from GROUP_CONCAT result.

### 4. JSONL Export (src/jsonl.rs)
- `export_jsonl()` and `export_jsonl_merge()` use `serde_json::to_writer()`
- Automatically includes the labels field during serialization
- Tests verify roundtrip compatibility (lines 308-360)

### 5. Sync Flush Flow (src/sync.rs)
```rust
pub fn flush_dirty(workspace_dir: &Path) -> Result<usize> {
    let dirty_issues = storage.list_dirty_issues()?; // includes labels
    let result = export_jsonl_dirty(
        &jsonl_path,
        || Ok(dirty_issues.clone()), // Issues with labels
        || storage.clear_dirty(),
    )?;
    // ...
}
```

## Verification Evidence

### 1. Labels in Current Export
```bash
$ grep '"labels"' .beads/issues.jsonl | head -2
{"id":"bf-10blr",...,"labels":["split-child"]}
{"id":"bf-10djn",...,"labels":["split-child"]}
```

### 2. Test Coverage
- `labels_are_exported_to_jsonl()` - Verifies labels export
- `labels_roundtrip_through_jsonl()` - Verifies import/export roundtrip
- `empty_labels_array_skipped_in_jsonl()` - Verifies empty arrays skipped

### 3. Build Status
- `cargo build` - Clean (0 errors)
- All serialization/deserialization compiles correctly

## Acceptance Criteria Met

✅ **Modify JSONL export to include labels field for each bead**
- Labels field on Issue struct with serde attributes
- SQL queries fetch labels from bead_labels table

✅ **Labels are written to JSONL as an array of strings**
- Serde serialization produces: `"labels":["label1","label2"]`
- Verified in actual issues.jsonl

✅ **All existing labels in bead_labels table are exported**
- All query functions use LEFT JOIN to fetch all labels
- GROUP_CONCAT ensures all labels per bead are included

✅ **JSONL format is compatible with import round-trip**
- Tests verify labels survive export/import cycle
- Format matches br's expectation

## Conclusion

The labels export functionality was implemented in commit `ce84ead` (bead bf-3l64k2) **before** bead bf-3ou3re was created. The implementation is complete, tested, and working in production. No code changes are required.
