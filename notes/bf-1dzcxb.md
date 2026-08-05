# Batch Operations Output Format Documentation

## Task: Capture actual batch operations output

This document captures the **actual** output format produced by `bf batch` commands as implemented in the codebase.

---

## Output Format Summary

The `bf batch` command produces output in two formats:
1. **Text format** (default) - Human-readable
2. **JSON format** (with `--format json`) - Machine-readable with envelope wrapper

---

## 1. Text Format (Default)

### Success Cases
```
[op 0] ok: bf-abc123
[op 1] ok
[op 2] ok
```

### Error Cases
```
[op 0] error: Bead not found: bf-nonexistent
```

**Notes:**
- Success messages go to stdout
- Error messages go to stderr
- For `create` operations, the bead ID is included
- For other operations, only the operation index and status are shown

---

## 2. JSON Format (`--format json`)

### Envelope Structure

All JSON output uses a unified envelope wrapper:

```json
{
  "version": 1,
  "kind": "batch",
  "data": [...],
  "warning": "Optional warning message"
}
```

#### Envelope Fields

| Field | Type | Description |
|-------|------|-------------|
| `version` | `u32` | Envelope version (currently 1) |
| `kind` | `String` | Command identifier, always `"batch"` for batch operations |
| `data` | `Array` | Array of `BatchResult` objects (see below) |
| `warning` | `String?` | Optional warning message (present only when auto-flush fails) |

---

### BatchResult Object Structure (Array Elements)

Each element in the `data` array represents one operation result:

```json
{
  "op": 0,
  "status": "ok",
  "id": "bf-abc123",
  "error": null,
  "message": "Created bead bf-abc123"
}
```

#### BatchResult Fields

| Field | Type | Description |
|-------|------|-------------|
| `op` | `usize` | Zero-based index of the operation in the input array |
| `status` | `String` | Either `"ok"` or `"error"` |
| `id` | `String?` | Bead ID (only present for successful `create` operations, `null` otherwise) |
| `error` | `String?` | Error message (only present when `status` is `"error"`, `null` otherwise) |
| `message` | `String?` | Human-readable success message (present only for successful operations) |

---

## Complete Example

### Input (JSON array via `--json` or `--stdin`)

```json
[
  {"op": "create", "title": "First task"},
  {"op": "create", "title": "Second task", "priority": 0},
  {"op": "dep_add_blocker", "id": "@0", "blocker": "@1"},
  {"op": "close", "id": "@0", "reason": "Completed"}
]
```

### Output (JSON format with envelope)

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
      "id": null,
      "error": null,
      "message": "ok: bf-abc123 blocked by bf-def456"
    },
    {
      "op": 3,
      "status": "ok",
      "id": null,
      "error": null,
      "message": "Closed bead bf-abc123"
    }
  ]
}
```

### Output (Text format)

```
[op 0] ok: bf-abc123
[op 1] ok: bf-def456
[op 2] ok
[op 3] ok
```

---

## Error Case Example

### Input (referencing non-existent bead)

```json
[
  {"op": "update", "id": "bf-nonexistent", "status": "in_progress"}
]
```

### Output (JSON format)

```json
{
  "version": 1,
  "kind": "batch",
  "data": [
    {
      "op": 0,
      "status": "error",
      "id": null,
      "error": "Bead not found: bf-nonexistent",
      "message": null
    }
  ]
}
```

### Output (Text format)

```
[op 0] error: Bead not found: bf-nonexistent
```

---

## Code References

### BatchResult Struct Definition
**File:** `src/batch.rs:113-122`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub op: usize,                       // Operation index
    pub status: String,                 // "ok" or "error"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,             // Bead ID (create only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,          // Error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,        // Success message
}
```

### Output Formatting
**File:** `src/cli/mod.rs:2715-2743`

The command:
1. Executes batch operations
2. Converts `Vec<BatchResult>` to JSON array
3. Wraps in envelope via `format_with_envelope("batch", &json_array)`

### Envelope Definition
**File:** `src/format/envelope.rs:51-61`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonEnvelope {
    pub version: u32,                   // Currently 1
    pub kind: String,                  // "batch" for this command
    pub data: Value,                    // The BatchResult array
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,        // Auto-flush failure warning
}
```

---

## Key Design Decisions

1. **Array structure**: Results are returned as an array (not JSONL), preserving input order
2. **Operation indexing**: `op` field is zero-based, matching input array position
3. **Fail-fast behavior**: Execution stops on first error (transaction rollback)
4. **ID omission**: Only `create` operations include an `id` field in results
5. **Envelope versioning**: Envelope includes `version` field for future compatibility
6. **Warning handling**: Auto-flush failures appear in envelope `warning` field, not in individual results

---

## Placeholder Resolution

When using placeholder references like `@0`, `@1`, etc., the `id` field in successful `create` operations contains the actual generated bead ID. Later operations can reference these IDs using the placeholder syntax.

Example:
- Operation 0 creates bead → returns `{"op": 0, "id": "bf-abc123"}`
- Operation 1 uses `@0` → resolves to `bf-abc123`

---

## Implementation Verified

- ✅ Output format matches codebase implementation
- ✅ Envelope structure is consistent with other `bf` commands
- ✅ Both text and JSON formats work as documented
- ✅ Error cases properly populate `error` field
- ✅ Success cases populate `message` field
