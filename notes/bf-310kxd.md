# Batch Operations Output Format - Code Locations

## Summary

Located all code responsible for batch operations output format generation in bead-forge CLI.

## Primary Data Structure

**File:** `src/batch.rs` (lines 113-122)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub op: usize,              // Operation index
    pub status: String,         // "ok" or "error"
    pub id: Option<String>,     // Created bead ID (for create ops)
    pub error: Option<String>,  // Error message (for failed ops)
    pub message: Option<String>, // Human-readable result message
}
```

## Output Generation Function

**File:** `src/batch.rs` (lines 191-435)

Function: `execute_batch()`
- Returns: `Result<Vec<BatchResult>>`
- Each operation in the batch returns a `BatchResult` struct
- The struct is serialized to JSON for JSON output mode

## CLI Output Formatting

**File:** `src/cli/mod.rs` (lines 2713-2794)

### Text Format Output (lines 2726-2742)

```rust
// Print results in human-readable format
for result in results {
    if result.status == "ok" {
        if let Some(id) = result.id {
            println!("[op {}] ok: {}", result.op, id);
        } else {
            println!("[op {}] ok", result.op);
        }
    } else {
        eprintln!("[op {}] error: {}", result.op, result.error.unwrap_or_default());
    }
}
```

**Output format:** `[op <index>] ok: <id>` or `[op <index>] ok` or `[op <index>] error: <message>`

### JSON Format Output (lines 2719-2723)

```rust
crate::format::OutputFormat::Json => {
    let formatter = get_formatter(output_format);
    // Convert Vec<BatchResult> to JSON array (not JSONL) for envelope wrapping
    let json_array = serde_json::to_string(&results).unwrap_or_default();
    println!("{}", formatter.format_with_envelope("batch", &json_array));
}
```

**Output format:** JSON array wrapped in envelope:
```json
{
  "version": "1",
  "kind": "batch",
  "data": [
    {"op": 0, "status": "ok", "id": "bf-123", "message": "Created bead bf-123"},
    {"op": 1, "status": "ok", "message": "ok: bf-123 blocked by bf-456"}
  ]
}
```

## Mitosis Output Formatting

**File:** `src/cli/mod.rs` (lines 2774-2790)

Function: `cmd_mitosis()`

### JSON Format
```rust
"json" => {
    println!("{}", serde_json::to_string_pretty(&results)?);
}
```

### Text Format
```rust
_ => {
    // Print child IDs that were created
    for result in &results {
        if let Some(child_id) = &result.id {
            println!("Created child: {}", child_id);
        }
    }
    println!("Parent bead {} closed with {} children", id, results.len() - 2);
}
```

## Command Path

Batch operations are invoked via the `bf batch` command:

1. CLI parsing: `src/cli/mod.rs` - `cmd_batch()` function
2. Input parsing: `src/batch.rs` - `parse_stdin()` or JSON deserialization
3. Execution: `src/batch.rs` - `execute_batch()`
4. Output formatting: `src/cli/mod.rs` - `cmd_batch()` (lines 2719-2743)

## Format Module

**Directory:** `src/format/`

- `mod.rs` - Defines `Formatter` trait and `OutputFormat` enum
- `json.rs` - JSON formatter implementation with envelope support
- `text.rs` - Text formatter implementation
- `envelope.rs` - JSON envelope wrapper for structured output

Note: Batch results use the standard JSON serialization of `BatchResult` via `serde_json::to_string()` - there's no custom batch-specific formatter logic in the format module.
