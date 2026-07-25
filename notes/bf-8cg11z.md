# Show Command Error Handling Analysis

## Task: Examine show command error handling for non-existent bead IDs

## Implementation Location

The show command is implemented in `src/cli/mod.rs`:
- CLI definition: lines 129-145 (Commands::Show enum)
- Handler function: `cmd_show()` at line 1676

## Current Error Handling Path

### Code Flow (lines 1676-1688)

```rust
fn cmd_show(beads_dir: &PathBuf, id: &str, format: &str, envelope: bool) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    let issue = match storage.get_issue(id)? {
        Some(i) => i,
        None => {
            // Search archives
            find_bead_in_archives(beads_dir, id)?
                .ok_or_else(|| anyhow!("Bead not found: {}", id))?
        }
    };
    // ... rest of function
}
```

### Error Handling Steps

1. **Primary lookup**: Try to get the issue from SQLite database via `storage.get_issue(id)?`
2. **Fallback lookup**: If not found in DB, search archive files via `find_bead_in_archives(beads_dir, id)?`
3. **Error generation**: If not found in archives either, return an error:
   ```rust
   anyhow!("Bead not found: {}", id)
   ```

## JSON Error Response Structure

### Current Behavior

**Actual Output** (when running `bf show --json bf-nonexistent`):
```bash
bf show --json bf-nonexistent 2>&1
# Output: Error: Bead not found: bf-nonexistent
# Exit code: 1
```

**Key observations**:
- The error is printed as **plain text**, not JSON
- Error goes to **stderr** (handled by anyhow's default error handler)
- Exit code is **1** (error)
- The `--json` flag is **ignored** in error cases

### JSON Formatter's Error Method

The `JsonFormatter` has a `format_error()` method defined in `src/format/json.rs` (line 61):

```rust
fn format_error(&self, message: &str) -> String {
    serde_json::json!({"error": message}).to_string()
}
```

**Would produce**:
```json
{"error":"Bead not found: bf-nonexistent"}
```

However, **this method is never called** by the show command (or any other command). Errors propagate up as `Result::Err` and are handled by anyhow's default error handler.

### Envelope Wrapper

The `--envelope` flag is also ignored in error cases. When successful, show can wrap output:
```json
{
  "version": 1,
  "kind": "show",
  "data": {...}
}
```

But errors bypass this entirely.

## Existing Error Constants/Helpers

### No Command-Level Error Constants

- No error constants defined for bead operations
- No `ErrorCode` or `ErrorKind` enum for command errors
- Errors use plain `anyhow!()` macros with string messages

### Storage Layer Errors

The storage layer (`src/storage/sqlite.rs`) has:
- Custom error type: `SecretDetected` (line 22-23)
- Error detection function: `is_busy_error()` (line 2089)

These are internal to storage and not surfaced as structured errors to CLI users.

## Error Response Structure

### Structure (what `format_error()` would produce)

```json
{
  "error": "<error message string>"
}
```

**Single field**:
- `error`: Human-readable error message string

### Example (if implemented)

```json
{
  "error": "Bead not found: bf-nonexistent"
}
```

## Summary

- **Error handling**: Uses anyhow's `Result` type with plain string errors
- **JSON output**: Errors are **NOT** JSON-formatted when `--json` is specified
- **Exit code**: 1 for errors, 0 for success
- **Error destination**: stderr (via anyhow)
- **Format**: Plain text string, NOT JSON
- **JSON formatter exists but unused**: `JsonFormatter::format_error()` is defined but never called
- **No error constants**: No structured error codes or types for command errors

The show command does **NOT** currently return structured JSON errors. It returns plain text errors to stderr regardless of the `--json` flag.