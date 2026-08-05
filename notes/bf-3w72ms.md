# Batch Operations Output Format Documentation

## Task: Document batch operations output format mismatch

**Bead ID:** bf-3w72ms  
**Date:** 2026-08-05

## Summary

This document describes the output format differences between what batch operations produce and what consumers expect, focusing on the JSON envelope structure.

## Expected Output Format (from envelope specification)

From `src/format/envelope.rs`, all `bf` commands with `--json` or `--format json` should emit:

```json
{
  "version": 1,
  "kind": "<command>",
  "data": <command-specific data>,
  "warning": "<optional auto-flush failure message>"
}
```

For the `batch` command specifically, the data shape should be (from envelope.rs:36):

```json
{
  "version": 1,
  "kind": "batch",
  "data": [
    {"op": <number>, "status": "<string>", "id": "<string?>", "error": "<string?>", "message": "<string?>"},
    ...
  ],
  "warning": "<optional>"
}
```

Where each element in the `data` array is a `BatchResult` object (from `src/batch.rs:112-122`):

```rust
pub struct BatchResult {
    pub op: usize,           // Operation index in the input array
    pub status: String,       // "ok" or "error"
    pub id: Option<String>,   // Bead ID (for create operations)
    pub error: Option<String>, // Error message (if status is "error")
    pub message: Option<String>, // Success message (if status is "ok")
}
```

## Actual Output Generation

From `src/cli/mod.rs:2713-2723`, the batch command produces output as follows:

```rust
let results = execute_batch(&storage, ops, beads_dir, no_auto_flush)?;

// Check if we should output JSON
let output_format = crate::format::OutputFormat::from_str(format).unwrap_or(crate::format::OutputFormat::Text);
match output_format {
    crate::format::OutputFormat::Json => {
        let formatter = get_formatter(output_format);
        // Convert Vec<BatchResult> to JSON array (not JSONL) for envelope wrapping
        let json_array = serde_json::to_string(&results).unwrap_or_default();
        println!("{}", formatter.format_with_envelope("batch", &json_array));
    }
    // ... text output handling
}
```

The envelope formatter (`src/format/json.rs:105-114`) processes this as:

```rust
fn format_with_envelope(&self, kind: &str, data: &str) -> String {
    // Parse the data string as JSON
    let json_value: Value =
        serde_json::from_str(data).unwrap_or_else(|_| Value::String(data.to_string()));

    // Wrap in envelope and serialize
    JsonEnvelope::new(kind, json_value)
        .to_json_compact()
        .unwrap_or_else(|_| "{}".to_string())
}
```

## Output Format Flow

1. `Vec<BatchResult>` → serialized to JSON string → `"[{...}, {...}]"`
2. JSON string → parsed back to `Value` → JSON array `[{...}, {...}]`
3. JSON array → wrapped in envelope → `{version: 1, kind: "batch", data: [{...}, {...}]}`

## Confirmed Format Characteristics

### Structure
- **Top-level**: JSON object (map), not a sequence
- **Envelope present**: Yes, when using `--format json`
- **Data type**: Array of operation result objects

### Field Names
- `version`: Number (always 1)
- `kind`: String (always "batch" for batch command)
- `data`: Array of BatchResult objects
- `warning`: String (optional, only present on auto-flush failure)

### BatchResult Object Fields
Each element in the data array contains:
- `op`: Number (zero-based index of operation in input)
- `status`: String ("ok" or "error")
- `id`: String or null (present only for successful create operations)
- `error`: String or null (present only when status is "error")
- `message`: String or null (present only when status is "ok")

## Example Output

For a batch of 3 successful create operations:

```json
{
  "version": 1,
  "kind": "batch",
  "data": [
    {
      "op": 0,
      "status": "ok",
      "id": "bf-abc123",
      "error": null,
      "message": "Created bead bf-abc123"
    },
    {
      "op": 1,
      "status": "ok",
      "id": "bf-def456",
      "error": null,
      "message": "Created bead bf-def456"
    },
    {
      "op": 2,
      "status": "ok",
      "id": "bf-ghi789",
      "error": null,
      "message": "Created bead bf-ghi789"
    }
  ]
}
```

For a mixed batch with an error:

```json
{
  "version": 1,
  "kind": "batch",
  "data": [
    {
      "op": 0,
      "status": "ok",
      "id": "bf-valid",
      "error": null,
      "message": "Created bead bf-valid"
    },
    {
      "op": 1,
      "status": "error",
      "id": null,
      "error": "Bead not found: bf-nonexistent",
      "message": null
    }
  ]
}
```

## Verification in Tests

The test `test_p0_batch_operations_with_labels` (`tests/test_p0_multilabel_cli.rs:378-427`):

1. Creates batch operations via `bf batch --stdin`
2. Verifies bead creation via `bf list --priority 0 --json --envelope`
3. Expects envelope with `data` as array of bead objects
4. Validates labels on each bead

The list command envelope format (from test expectations):
```json
{
  "data": [
    {"id": "...", "title": "...", "priority": 0, "labels": [...], ...},
    ...
  ]
}
```

## Notes

- The envelope is only added when `--format json` is used (or `--json` flag)
- Text output (default) uses human-readable format: `[op 0] ok: bf-xxx`
- All operations in a batch are atomic (all succeed or all fail)
- The batch output format is consistent with other commands in using the envelope structure
