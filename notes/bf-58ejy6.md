# Batch Operations Test Output Format Analysis

## Test Location
`tests/test_p0_multilabel_cli.rs:370-419` (test_p0_batch_operations_with_labels)

## Test Commands
The test runs:
```bash
bf batch --stdin
```
**without** specifying `--format json`

## Actual Output Format

### Default (Text Format)
When `--format` is not specified, batch outputs text format:
```
[op 0] ok: bf-ui4
[op 1] ok: bf-jx2
[op 2] ok: bf-kp8
```

### JSON Format (when `--format json` is specified)
```json
{"version":1,"kind":"batch","data":[{"id":"bf-ui4","message":"Created bead bf-ui4","op":0,"status":"ok"}]}
```

This is an envelope structure with:
- `version`: 1
- `kind`: "batch" 
- `data`: Array of BatchResult objects
- `warning`: Optional field (present only on auto-flush failure)

## Code Location
The output format is generated in:
- `src/cli/mod.rs` - `cmd_batch()` function (lines ~1340-1380)
- `src/format/json.rs` - `JsonFormatter::format_with_envelope()` (lines 64-73)

## Format Structure Details

### BatchResult Structure
```rust
pub struct BatchResult {
    pub op: usize,        // Operation index
    pub status: String,   // "ok" or "error"
    pub id: Option<String>,  // Created bead ID (for create ops)
    pub error: Option<String>, // Error message (if status == "error")
    pub message: Option<String>, // Success message
}
```

### Envelope Generation
```rust
// In cmd_batch(), when OutputFormat::Json:
let json_array = serde_json::to_string(&results).unwrap_or_default();
println!("{}", formatter.format_with_envelope("batch", &json_array));
```

The envelope wraps the serialized array of BatchResult objects.

## Test Behavior
The test **does not currently validate** the batch command's stdout output. It only:
1. Checks `output.status.success()` - exit code is 0
2. Uses `bf list --priority 0 --json --envelope` to verify beads were created

## Key Findings
1. **No format mismatch in the test** - the test doesn't check stdout format
2. **Default format is text** - human-readable `[op N] ok: id` format
3. **JSON format uses envelope** - structured response with version/kind/data
4. **No bug in output** - the code correctly implements both formats

The actual batch output is working correctly. The test simply doesn't validate the stdout format, focusing instead on successful execution and verifying the created beads via `bf list`.
