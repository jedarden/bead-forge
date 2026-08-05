# src/jsonl.rs Serialization Verification

## Task: Fix src/jsonl.rs export to use standard Issue serde

## Findings

**Status: ✅ ALREADY CORRECT - No changes needed**

All export functions in `src/jsonl.rs` already use standard Issue serde serialization:

### Export Functions Using Standard Serde

1. **`export_jsonl` (line 88)**
   ```rust
   serde_json::to_writer(&mut writer, issue)
   ```
   ✅ Uses standard serde Issue serialization

2. **`export_jsonl_merge` (line 161)**
   ```rust
   serde_json::to_string(issue)
   ```
   ✅ Uses standard serde Issue serialization

3. **`stream_issues` (line 45)**
   ```rust
   serde_json::from_str::<Issue>(&line)
   ```
   ✅ Uses standard serde Issue deserialization

4. **`import_jsonl` (line 63)**
   ```rust
   serde_json::from_str::<Issue>(&line)
   ```
   ✅ Uses standard serde Issue deserialization

### Verification Tests Passed

- ✅ `export_jsonl_writes_multiple_beads_sorted` - Standard serialization works
- ✅ `empty_labels_array_skipped_in_jsonl` - `skip_serializing_if` respected
- ✅ `labels_are_exported_to_jsonl` - Labels included when present
- ✅ `labels_roundtrip_through_jsonl` - Full roundtrip preserves data

### Serde Attributes Respected

The standard Issue serde includes these attributes from `model.rs:469`:
- `#[serde(default, skip_serializing_if = "Option::is_none")]` - Optional fields
- `#[serde(skip_serializing_if = "Vec::is_empty", default)]` - Collections
- Custom `serialize_compaction_level` - bd conformance (None → 0)

All are correctly applied during export.

## Conclusion

No code changes required in `src/jsonl.rs`. The export functions already use standard Issue serde serialization as required.

**Note:** The audit identified `src/format/json.rs` as needing fixes (custom relation stripping), but `src/jsonl.rs` is correct.
