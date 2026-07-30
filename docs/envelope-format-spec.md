# bead-forge Envelope Format Specification

## Overview

All `bf` commands that support `--json` or `--format json` emit output wrapped in a **unified JSON envelope**. This envelope provides:

- **Version stability**: The `version` field enables future compatibility
- **Command identification**: The `kind` field identifies the command so consumers can parse `data` correctly
- **Error/warning metadata**: The optional `warning` field signals problems (e.g., auto-flush failure)

## Envelope Structure

```json
{
  "version": 1,
  "kind": "<command>",
  "data": <command-specific data>,
  "warning": "<optional warning message>"
}
```

### Field Descriptions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `version` | `number` | **Always** | Envelope version (currently `1`). Enables future format evolution. |
| `kind` | `string` | **Always** | Command identifier (e.g., `"list"`, `"claim"`, `"create"`). Tells consumers how to parse `data`. |
| `data` | *varies* | **Always** | Command-specific payload. Can be an object, array, string, number, or other JSON value. |
| `warning` | `string` | **Optional** | Present only when a non-fatal problem occurred (e.g., auto-flush failure). Omitted from JSON when `null`. |

## Command-Specific `data` Shapes

The `data` field shape varies by command. This section documents the expected structure for each command type.

### List-like Commands (array results)

Commands that return multiple items emit a **JSON array** in the `data` field:

| Command | `data` shape | Empty result |
|---------|-------------|--------------|
| `list` | `[{...}, {...}]` | `[]` |
| `ready` | `[{...}, {...}]` | `[]` |
| `search` | `[{...}, {...}]` | `[]` |
| `recent` | `[{...}, {...}]` | `[]` |
| `velocity` | `[{...}, {...}]` | `[]` |

**Example: `bf list --json`**

```json
{
  "version": 1,
  "kind": "list",
  "data": [
    {
      "id": "bf-abc123",
      "title": "Implement auth flow",
      "status": "open",
      "priority": 2,
      "issue_type": "task",
      "assignee": null,
      "labels": ["phase-1", "urgent"],
      "created_at": "2026-07-22T15:54:16Z",
      "updated_at": "2026-07-22T15:54:16Z"
    },
    {
      "id": "bf-def456",
      "title": "Fix session bug",
      "status": "in_progress",
      "priority": 0,
      "issue_type": "bug",
      "assignee": "worker-7",
      "labels": [],
      "created_at": "2026-07-21T10:30:00Z",
      "updated_at": "2026-07-22T14:20:00Z"
    }
  ]
}
```

**Example: `bf ready --json` (empty result)**

```json
{
  "version": 1,
  "kind": "ready",
  "data": []
}
```

### Single-Object Commands

Commands that return a single item or status emit a **JSON object** in the `data` field:

| Command | `data` shape | Description |
|---------|-------------|-------------|
| `show` | `{...}` | Full bead object with all fields |
| `claim` | `{...}` | Claim result object (bead_id, assignee, optional fields) |
| `stats` | `{...}` | Aggregate counts with optional breakdowns |

**Example: `bf show bf-abc123 --json`**

```json
{
  "version": 1,
  "kind": "show",
  "data": {
    "id": "bf-abc123",
    "title": "Implement auth flow",
    "description": "Add OAuth2 authentication for third-party providers",
    "status": "open",
    "priority": 2,
    "issue_type": "task",
    "assignee": null,
    "owner": null,
    "labels": ["phase-1", "urgent"],
    "created_at": "2026-07-22T15:54:16Z",
    "updated_at": "2026-07-22T15:54:16Z",
    "dependencies": [],
    "comments": []
  }
}
```

**Example: `bf claim --assignee worker-7 --json` (successful claim)**

```json
{
  "version": 1,
  "kind": "claim",
  "data": {
    "bead_id": "bf-abc123",
    "assignee": "worker-7",
    "title": "Implement auth flow",
    "priority": 2,
    "reclaimed": 0,
    "workspace": "/home/coding/FORGE"
  }
}
```

**Example: `bf claim --assignee worker-7 --json` (no beads available)**

```json
{
  "version": 1,
  "kind": "claim",
  "data": {}
}
```

**Example: `bf stats --json --by-type`**

```json
{
  "version": 1,
  "kind": "stats",
  "data": {
    "total": 150,
    "open": 75,
    "in_progress": 45,
    "closed": 30,
    "by_type": {
      "task": 100,
      "bug": 30,
      "feature": 15,
      "chore": 5
    }
  }
}
```

### Mutation Commands

Commands that mutate state return a simple acknowledgment object:

| Command | `data` shape | Description |
|---------|-------------|-------------|
| `create` | `{"id": "bf-xxx"}` | ID of newly created bead |
| `update` | `{"id": "bf-xxx"}` | ID of updated bead |
| `close` | `{"id": "bf-xxx"}` | ID of closed bead |
| `reopen` | `{"id": "bf-xxx"}` | ID of reopened bead |
| `delete` | `{"id": "bf-xxx"}` | ID of deleted bead |

**Example: `bf create --title "Fix login bug" --type bug --json`**

```json
{
  "version": 1,
  "kind": "create",
  "data": {
    "id": "bf-new123"
  }
}
```

**Example: `bf close bf-abc123 --reason "Completed" --json`**

```json
{
  "version": 1,
  "kind": "close",
  "data": {
    "id": "bf-abc123"
  }
}
```

### Batch Operations

The `batch` command emits an array of operation results:

**Example: `bf batch --json '[{"op": "create", "title": "Task 1"}, {"op": "close", "id": "bf-123"}]'`**

```json
{
  "version": 1,
  "kind": "batch",
  "data": [
    {
      "op": 0,
      "status": "ok",
      "id": "bf-new456",
      "message": "Created bead bf-new456"
    },
    {
      "op": 1,
      "status": "ok",
      "message": "Closed bead bf-123"
    }
  ]
}
```

## The `warning` Field

The `warning` field is present **only when a non-fatal problem occurs**. The most common case is an auto-flush failure after a successful mutation.

**When present**: The `warning` key is included in the JSON with a string value.

**When absent**: The `warning` key is omitted entirely (not `null`, just missing). This is controlled by `#[serde(skip_serializing_if = "Option::is_none")]`.

**Example: Auto-flush failure**

```json
{
  "version": 1,
  "kind": "create",
  "data": {
    "id": "bf-new789"
  },
  "warning": "auto-flush failed: write error. Bead created in database but not written to issues.jsonl. Run 'bf sync --flush-only' to recover."
}
```

## Version Field

The `version` field is **always `1`** in the current implementation. This field exists to enable future format evolution:

- If the envelope structure changes in a backward-incompatible way, the `version` will be incremented.
- Consumers should check `version` first and reject unknown versions.
- Version `1` envelopes will remain supported indefinitely for backward compatibility.

## Parsing Guidelines

### For Command Authors

When writing code that consumes `bf --json` output:

1. **Parse the envelope first**: Extract `version`, `kind`, and `data` fields.
2. **Validate version**: Reject envelopes with `version != 1` (or handle known versions explicitly).
3. **Dispatch on `kind`**: Use the `kind` field to determine how to parse `data`.
4. **Handle warnings**: Check for the `warning` field and log/display it to the user if present.

### Example Parsing Code (Python)

```python
import json
import sys

def parse_bf_output(output: str):
    """Parse bf --json output and dispatch based on kind."""
    envelope = json.loads(output)
    
    # Validate version
    if envelope.get("version") != 1:
        raise ValueError(f"Unsupported envelope version: {envelope.get('version')}")
    
    # Check for warnings
    if "warning" in envelope:
        print(f"WARNING: {envelope['warning']}", file=sys.stderr)
    
    # Dispatch based on kind
    kind = envelope["kind"]
    data = envelope["data"]
    
    if kind in ("list", "ready", "search", "recent", "velocity"):
        # List commands: data is an array of beads
        return handle_list_command(kind, data)
    elif kind == "show":
        # Show: data is a single bead object
        return handle_show_command(data)
    elif kind == "claim":
        # Claim: data is a claim result object (may be empty {})
        return handle_claim_command(data)
    elif kind in ("create", "update", "close", "reopen", "delete"):
        # Mutations: data has the bead ID
        return handle_mutation_command(kind, data)
    elif kind == "stats":
        # Stats: data is aggregate counts
        return handle_stats_command(data)
    elif kind == "batch":
        # Batch: data is an array of operation results
        return handle_batch_command(data)
    else:
        raise ValueError(f"Unknown command kind: {kind}")
```

### Example Parsing Code (Rust)

```rust
use serde_json::Value;

fn parse_bf_output(output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let envelope: Value = serde_json::from_str(output)?;
    
    // Validate version
    let version = envelope.get("version")
        .and_then(|v| v.as_u64())
        .ok_or("Missing or invalid version field")?;
    if version != 1 {
        return Err(format!("Unsupported envelope version: {}", version).into());
    }
    
    // Check for warnings
    if let Some(warning) = envelope.get("warning").and_then(|w| w.as_str()) {
        eprintln!("WARNING: {}", warning);
    }
    
    // Dispatch based on kind
    let kind = envelope.get("kind")
        .and_then(|k| k.as_str())
        .ok_or("Missing or invalid kind field")?;
    let data = envelope.get("data").ok_or("Missing data field")?;
    
    match kind {
        "list" | "ready" | "search" | "recent" | "velocity" => {
            // data is an array
            let items = data.as_array().ok_or("data must be an array")?;
            println!("Received {} items from {}", items.len(), kind);
        }
        "show" | "claim" | "stats" => {
            // data is an object
            let obj = data.as_object().ok_or("data must be an object")?;
            println!("Received {} result: {}", kind, 
                obj.get("id").or_else(|| obj.get("bead_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no id)"));
        }
        "create" | "update" | "close" | "reopen" | "delete" => {
            // mutation ack
            if let Some(id) = data.get("id").and_then(|v| v.as_str()) {
                println!("{} completed for bead {}", kind, id);
            }
        }
        _ => {
            return Err(format!("Unknown command kind: {}", kind).into());
        }
    }
    
    Ok(())
}
```

## Backward Compatibility

### NDJSON to Array Migration

**Historical context**: Prior to envelope implementation, list-like commands (`list`, `ready`, `search`, `recent`) emitted **NDJSON** (one JSON object per line, no array wrapper). This format was problematic for consumers using `json.loads()` on the entire output.

**Current behavior**: All list-like commands now emit a **JSON array** wrapped in the envelope `data` field. This provides:

- Consistent parsing across all list commands
- Clear empty-result handling (`[]` instead of empty stdout)
- Machine-readable command identification via the `kind` field

### Migration Guide for Old Code

**Old code (NDJSON parsing):**

```python
# Does NOT work with envelope format
output = subprocess.check_output(["bf", "list", "--format", "json"], text=True)
beads = []
for line in output.splitlines():
    if line.strip():
        beads.append(json.loads(line))
```

**New code (envelope parsing):**

```python
# Works with current envelope format
output = subprocess.check_output(["bf", "list", "--json"], text=True)
envelope = json.loads(output)
assert envelope["kind"] == "list"
beads = envelope["data"]  # Already a list
```

## Implementation Reference

- **Envelope struct**: `src/format/envelope.rs` — `JsonEnvelope` with `version`, `kind`, `data`, and optional `warning` fields
- **Formatter trait**: `src/format/mod.rs` — `format_with_envelope()` and `format_with_envelope_and_warning()` methods
- **Tests**: `src/format/envelope.rs` (unit tests) and `tests/envelope/` (integration tests per command)

## Summary

The envelope format provides a stable, machine-readable wrapper for all `bf --json` output:

1. **Three required fields**: `version` (always 1), `kind` (command name), `data` (command-specific payload)
2. **One optional field**: `warning` (present only on non-fatal problems like auto-flush failure)
3. **Dispatch pattern**: Parse `kind` to determine how to interpret `data`
4. **Future-proof**: The `version` field enables backward-compatible evolution

All commands with `--json` or `--format json` support this envelope — no exceptions.
