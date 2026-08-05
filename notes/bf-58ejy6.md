# Batch Output Format Investigation (bf-58ejy6)

## Task

Debug batch operations test output format in `test_p0_batch_operations_with_labels`.

## Investigation

### Test Location
File: `tests/test_p0_multilabel_cli.rs:370-419`

### Expected Output Format

Based on the envelope specification in `src/format/envelope.rs:36`:

```json
{
  "version": 1,
  "kind": "batch",
  "data": [
    {"op": 0, "status": "ok", "id": "bf-xxx", "message": "..."},
    {"op": 1, "status": "ok", "id": "bf-yyy", "message": "..."},
    {"op": 2, "status": "ok", "id": "bf-zzz", "message": "..."}
  ]
}
```

Where:
- `version`: Always `1` (envelope version)
- `kind`: Always `"batch"` (command identifier)
- `data`: Array of `BatchResult` objects

### BatchResult Structure

From `src/batch.rs:113-122`:

```rust
pub struct BatchResult {
    pub op: usize,              // Operation index (0-based)
    pub status: String,         // "ok" or "error"
    pub id: Option<String>,    // Bead ID (if successful)
    pub error: Option<String>, // Error message (if failed)
    pub message: Option<String>, // Optional message
}
```

### Output Generation Flow

From `src/cli/mod.rs:2714-2724`:

1. **Execute batch**: `let results = execute_batch(&storage, ops, beads_dir, no_auto_flush)?;`
   - Returns: `Vec<BatchResult>`

2. **Serialize to JSON**: `let json_array = serde_json::to_string(&results).unwrap_or_default();`
   - Converts `Vec<BatchResult>` to JSON array string
   - Example: `"[{\"op\":0,\"status\":\"ok\",\"id\":\"bf-xxx\"},...]"`

3. **Wrap in envelope**: `println!("{}", formatter.format_with_envelope("batch", &json_array));`
   - From `src/format/json.rs:105-114`:
     ```rust
     fn format_with_envelope(&self, kind: &str, data: &str) -> String {
         let json_value: Value = serde_json::from_str(data)
             .unwrap_or_else(|_| Value::String(data.to_string()));
         JsonEnvelope::new(kind, json_value).to_json_compact()
             .unwrap_or_else(|_| "{}".to_string())
     }
     ```
   - Parses JSON string back to `Value` (array)
   - Wraps in envelope: `{"version":1,"kind":"batch","data":[...]}`
   - Serializes to compact JSON

### Actual Output Format

The batch command output should be:

```json
{
  "version": 1,
  "kind": "batch",
  "data": [
    {"op": 0, "status": "ok", "id": "bf-xxx", "message": "Created bead bf-xxx"},
    {"op": 1, "status": "ok", "id": "bf-yyy", "message": "Created bead bf-yyy"},
    {"op": 2, "status": "ok", "id": "bf-zzz", "message": "Created bead bf-zzz"}
  ]
}
```

### Current Status

**Codebase has compilation errors** preventing test execution:

```
error[E0282]: type annotations needed
   --> src/doctor.rs:900:58

error[E0308]: mismatched types
   --> src/error.rs:202:25
   --> src/storage/sqlite.rs:1144:20
   --> src/sync.rs:120:12

error[E0599]: no method named `into_inner` found for struct `anyhow::Error`
   --> src/error.rs:232:39
```

These errors are unrelated to batch output format but must be fixed before the test can run.

## Conclusions

1. **Format is correct**: The batch output format follows the envelope specification correctly.
2. **Data structure**: `data` field contains a JSON array of `BatchResult` objects
3. **Serialization path**: `Vec<BatchResult>` → JSON string → parsed as Value → wrapped in envelope
4. **Test issue**: The test at line 402-403 doesn't actually verify the batch output format—it only checks that the command succeeds (`output.status.success()`), then uses `list` to verify beads were created.

## Next Steps

Fix the compilation errors in the codebase before the test can be executed to capture actual output.
