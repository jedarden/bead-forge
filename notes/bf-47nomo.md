# Empty Label Field Handling Tests (bf-47nomo)

Added comprehensive tests for empty label field handling during JSONL import.

## Tests Added

### `test_label_import_empty_array`
Tests import with explicit empty labels array `"labels": []` - correctly imports as empty Vec.

### `test_label_import_null_field_rejected`
Tests that `"labels": null` is **rejected** during import with appropriate error message.
The current `Vec<String>` field with `#[serde(default)]` handles missing fields but
rejects null values, which is the expected behavior.

### `test_label_import_mixed_empty_scenarios`
Tests multiple valid empty label scenarios in a single import:
- Missing labels field (defaults to empty)
- Empty array `"labels": []` (imports as empty)
- Valid labels (preserved correctly)

## Coverage Summary

All acceptance criteria met:
- ✅ Test empty label field handling on import
- ✅ Test null label field handling (rejected)
- ✅ Test missing labels array handling (defaults to empty)
- ✅ Test empty array handling (imports as empty)
- ✅ All tests pass with `cargo test`

## Behavior

- **Missing field** → `#[serde(default)]` provides empty `Vec<String>`
- **`"labels": []`** → Deserializes as empty `Vec<String>`
- **`"labels": null`** → Rejected with deserialization error
- **`"labels": ["a", "b"]`** → Normal deserialization

Total label import tests: **9 passed**
