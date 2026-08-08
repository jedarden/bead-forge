# Envelope Format Specification

## Overview

All `bf` commands that support `--json` or `--format json` emit a **stable envelope shape** that wraps command-specific data. This envelope provides versioning, command identification, and optional warnings for machine-readable consumers.

## Envelope Structure

```json
{
  "version": 1,
  "kind": "<command>",
  "data": <command-specific data>,
  "warning": "<optional warning message>"
}
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `version` | `number` | Yes | Envelope version (currently `1`). Enables future compatibility. |
| `kind` | `string` | Yes | Command identifier (e.g., `"list"`, `"ready"`, `"claim"`, `"create"`). |
| `data` | `varies` | Yes | Command-specific data (object, array, string, number, or boolean). |
| `warning` | `string` | No | Warning message (present only when auto-flush fails or other non-fatal errors occur). |

### Version Field Requirements

- **Current version**: `1`
- **Type**: Integer (serialized as number, not string)
- **Purpose**: Future compatibility - enables structural changes while maintaining parser compatibility
- **Validation**: Consumers should reject envelopes with unknown versions
- **Updates**: Incremented only when breaking changes to the envelope structure occur

### Kind Field Enumerations

The `kind` field identifies the command that generated the envelope. All standard `bf` commands use their command name as the kind value:

| Kind Value | Command | Description |
|------------|---------|-------------|
| `"list"` | `bf list` | List beads with optional filters |
| `"ready"` | `bf ready` | List unblocked, open beads ready for claiming |
| `"show"` | `bf show <id>` | Show full details of a single bead |
| `"claim"` | `bf claim` | Claim a bead (atomic dequeue) |
| `"create"` | `bf create` | Create a new bead |
| `"update"` | `bf update` | Update an existing bead |
| `"close"` | `bf close` | Close a bead |
| `"reopen"` | `bf reopen` | Reopen a closed bead |
| `"delete"` | `bf delete` | Delete a bead |
| `"stats"` | `bf stats` | Show workspace statistics |
| `"velocity"` | `bf velocity` | Show velocity statistics |
| `"search"` | `bf search` | Search beads by query |
| `"recent"` | `bf recent` | List recently updated beads |
| `"batch"` | `bf batch` | Execute batch operations |
| `"log"` | `bf log` | Show operation history |

## Command-Specific Data Shapes

### Listing Commands (Array Data)

Commands that list multiple beads emit a **JSON array** in the `data` field:

| Command | `data` Shape | Empty Result | Example |
|---------|---------------|---------------|---------|
| `list` | `[{bead}, ...]` | `[]` | See below |
| `ready` | `[{bead}, ...]` | `[]` | See below |
| `search` | `[{bead}, ...]` | `[]` | See below |
| `recent` | `[{bead}, ...]` | `[]` | See below |
| `velocity` | `[{stat}, ...]` | `[]` | See below |

#### Example: `bf list --format json`

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
      "labels": ["phase-1", "backend"],
      "created_at": "2026-07-22T15:54:16Z",
      "updated_at": "2026-07-22T15:54:16Z"
    },
    {
      "id": "bf-def456",
      "title": "Add session tests",
      "status": "open",
      "priority": 1,
      "issue_type": "task",
      "assignee": null,
      "labels": [],
      "created_at": "2026-07-22T16:00:00Z",
      "updated_at": "2026-07-22T16:00:00Z"
    }
  ]
}
```

#### Example: `bf ready --format json` (empty result)

```json
{
  "version": 1,
  "kind": "ready",
  "data": []
}
```

### Single Object Commands

Commands that operate on or return a single bead emit a **single object** in the `data` field:

| Command | `data` Shape | Example |
|---------|---------------|---------|
| `show` | `{bead}` | See below |
| `claim` | `{claim_result}` | See below |
| `create` | `{id: "bf-xxx"}` | See below |
| `update` | `{id: "bf-xxx"}` | See below |
| `close` | `{id: "bf-xxx"}` | See below |
| `reopen` | `{id: "bf-xxx"}` | See below |
| `delete` | `{id: "bf-xxx"}` | See below |
| `stats` | `{total: ..., open: ..., ...}` | See below |

#### Example: `bf show bf-abc123 --format json`

```json
{
  "version": 1,
  "kind": "show",
  "data": {
    "id": "bf-abc123",
    "title": "Implement auth flow",
    "description": "Add OAuth2 authentication",
    "status": "open",
    "priority": 2,
    "issue_type": "task",
    "assignee": null,
    "labels": ["phase-1", "backend"],
    "created_at": "2026-07-22T15:54:16Z",
    "updated_at": "2026-07-22T15:54:16Z",
    "dependencies": [],
    "comments": []
  }
}
```

#### Example: `bf claim --assignee worker-7 --format json` (successful claim)

```json
{
  "version": 1,
  "kind": "claim",
  "data": {
    "bead_id": "bf-abc123",
    "assignee": "worker-7",
    "reclaimed": 0,
    "title": "Implement auth flow",
    "priority": 2,
    "downstream_impact": 5
  }
}
```

#### Example: `bf claim --assignee worker-7 --format json` (no beads available)

```json
{
  "version": 1,
  "kind": "claim",
  "data": {}
}
```

#### Example: `bf create --title "New task" --format json`

```json
{
  "version": 1,
  "kind": "create",
  "data": {
    "id": "bf-new123"
  }
}
```

#### Example: `bf stats --format json`

```json
{
  "version": 1,
  "kind": "stats",
  "data": {
    "total": 100,
    "open": 50,
    "in_progress": 30,
    "closed": 20,
    "by_type": {
      "task": 60,
      "bug": 20,
      "feature": 20
    },
    "by_priority": {
      "0": 10,
      "1": 20,
      "2": 40,
      "3": 20,
      "4": 10
    }
  }
}
```

#### Example: `bf stats --format json` (no breakdowns)

```json
{
  "version": 1,
  "kind": "stats",
  "data": {
    "total": 100,
    "open": 50,
    "in_progress": 30,
    "closed": 20
  }
}
```

### Warning Field

The `warning` field is present **only when a non-fatal error occurs** during command execution. The most common case is auto-flush failure (when `sync.auto_flush` is enabled but the incremental JSONL export fails).

#### Example: Auto-flush failure warning

```json
{
  "version": 1,
  "kind": "update",
  "data": {
    "id": "bf-abc123"
  },
  "warning": "auto-flush failed: 3 beads not exported to JSONL. Run 'bf sync --flush-only' to retry."
}
```

#### Warning Field Behavior

- **Absent when**: No warnings, clean execution
- **Present when**: Non-fatal errors occur (auto-flush failures, partial batch completions, etc.)
- **Purpose**: Provides recovery guidance without failing the command
- **Format**: Human-readable message string
- **Consumers**: Should surface warnings to users while treating the command as successful

## Data Field Type Variations

The `data` field can hold different JSON types depending on the command:

| Data Type | Commands | Example |
|------------|----------|---------|
| `array` | `list`, `ready`, `search`, `recent`, `velocity`, `batch` | `[{...}, {...}]` |
| `object` | `show`, `claim`, `stats`, `create`, `update`, `close` | `{key: value, ...}` |
| `string` | (rare, custom commands) | `"output text"` |
| `number` | (rare, count commands) | `42` |
| `boolean` | (rare, status checks) | `true` |
| `null` | (rare, no-op commands) | `null` |

## Command-Specific Variations

### Bead Field Stripping (JSON Formatter)

When emitting bead data in JSON format, the following fields are **stripped** from the `Issue` struct for `br` compatibility and to keep output compact:

- `dependencies` — Relations are stripped to empty array, then skipped via `skip_serializing_if`
- `comments` — Relations are stripped to empty array, then skipped via `skip_serializing_if`
- `events` — Skipped when empty via `skip_serializing_if = "Vec::is_empty"`

This stripping is **intentional and cannot be replaced** with serde `#[serde(skip)]` attributes because:
1. We need to preserve these fields for JSONL export/import roundtrips
2. We need to include them in API responses and debugging contexts
3. We only want to exclude them for JSON formatter output (list/ready/search commands)
4. br compatibility requires keeping these fields out of command output

### Empty Result Handling

Different commands handle empty results differently:

| Command | Empty `data` Value | Rationale |
|---------|-------------------|------------|
| `list` | `[]` | Consistent array shape for parsing |
| `ready` | `[]` | Explicit empty candidate list |
| `search` | `[]` | No matches found |
| `recent` | `[]` | No recent activity |
| `velocity` | `[]` | No velocity data available |
| `claim` | `{}` | No beads available (empty object, not array) |
| `show` | *error* | Bead not found (command fails) |

### Claim Command Variations

The `claim` command's `data` field varies based on the claim outcome:

#### Dry-run claim (preview mode)

```json
{
  "version": 1,
  "kind": "claim",
  "data": {
    "bead_id": "bf-abc123",
    "assignee": "worker-7",
    "title": "Implement auth flow",
    "priority": 2,
    "downstream_impact": 5,
    "workspace": "/path/to/workspace",
    "dry_run": true
  }
}
```

#### Multi-workspace claim

```json
{
  "version": 1,
  "kind": "claim",
  "data": {
    "bead_id": "bf-xyz789",
    "assignee": "worker-7",
    "reclaimed": 1,
    "workspace": "/path/to/workspace"
  }
}
```

#### Standard single-workspace claim

```json
{
  "version": 1,
  "kind": "claim",
  "data": {
    "bead_id": "bf-abc123",
    "assignee": "worker-7",
    "reclaimed": 0
  }
}
```

**Note**: The `workspace` field is **omitted** for single-workspace claims (only present when claiming from multiple workspaces via `--any` or `--workspace-paths`).

### Batch Command Data Shape

The `batch` command emits an array of operation results in the `data` field:

```json
{
  "version": 1,
  "kind": "batch",
  "data": [
    {
      "op": 0,
      "status": "ok",
      "id": "bf-new1",
      "message": "Created bead bf-new1"
    },
    {
      "op": 1,
      "status": "ok",
      "id": "bf-new2",
      "message": "Created bead bf-new2"
    },
    {
      "op": 2,
      "status": "ok",
      "message": "ok: bf-parent blocked by bf-new1"
    },
    {
      "op": 3,
      "status": "ok",
      "message": "Closed bead bf-parent"
    }
  ]
}
```

#### Batch error case (transaction rolled back)

```json
{
  "version": 1,
  "kind": "batch",
  "data": [
    {
      "op": 0,
      "status": "ok",
      "id": "bf-new1",
      "message": "Created bead bf-new1"
    },
    {
      "op": 1,
      "status": "error",
      "error": "Bead not found: bf-missing",
      "message": null
    }
  ]
}
```

**Important**: When any operation in a batch fails, the **entire transaction rolls back** and no partial state is committed. The envelope includes both successful and failed operation results, but the database is unchanged.

## Serialization Rules

### Field Presence

- **`version`**: Always present, always `1`
- **`kind`**: Always present, matches command name
- **`data`**: Always present, type varies by command
- **`warning`**: Present only when a non-fatal error occurred

### Empty Field Handling

Empty collections and `None` values use `skip_serializing_if`:

- Empty `dependencies`, `comments`, `events`, `labels` are **omitted** from output
- `None` values for optional fields (description, design, etc.) are **omitted**
- **Exception**: `assignee` and `labels` are **always present** in command JSON output (normalized to `null`/`[]`), even when omitted in the JSONL artifact

### Compact vs Pretty Serialization

Envelopes support both serialization modes:

- **Compact** (`to_json_compact`): Single line, no extra whitespace (default for `--format json`)
- **Pretty** (`to_json`): Multi-line with indentation (used for debugging)

## Versioning and Compatibility

### Version 1 Specification

Current envelope version is **1**. This specification documents version 1.

### Future Versions

When breaking changes to the envelope structure are necessary:

1. Increment the `VERSION` constant in `src/format/envelope.rs`
2. Update this specification document
3. Document the migration path from previous versions
4. Maintain backwards compatibility where possible

### Compatibility Rules

**Consumers should**:
- Reject envelopes with unknown `version` values
- Ignore unknown fields in the envelope structure (forward compatibility)
- Treat missing `warning` field as "no warning"
- Validate `kind` against known commands (fail gracefully on unknown kinds)

**Producers should**:
- Never omit `version`, `kind`, or `data` fields
- Use `skip_serializing_if` for optional fields only
- Include `warning` only when non-fatal errors occur
- Maintain backward compatibility within the same major version

## Examples by Command Category

### Lifecycle Commands

#### `bf create --format json`

```json
{
  "version": 1,
  "kind": "create",
  "data": {
    "id": "bf-new123"
  }
}
```

#### `bf update bf-abc123 --status in_progress --format json`

```json
{
  "version": 1,
  "kind": "update",
  "data": {
    "id": "bf-abc123"
  }
}
```

#### `bf close bf-abc123 --reason "Completed" --format json`

```json
{
  "version": 1,
  "kind": "close",
  "data": {
    "id": "bf-abc123"
  }
}
```

#### `bf reopen bf-abc123 --format json`

```json
{
  "version": 1,
  "kind": "reopen",
  "data": {
    "id": "bf-abc123"
  }
}
```

#### `bf delete bf-abc123 --format json`

```json
{
  "version": 1,
  "kind": "delete",
  "data": {
    "id": "bf-abc123"
  }
}
```

### Query Commands

#### `bf list --status open --format json`

```json
{
  "version": 1,
  "kind": "list",
  "data": [
    {
      "id": "bf-abc123",
      "title": "Open task 1",
      "status": "open",
      "priority": 2,
      "issue_type": "task",
      "assignee": null,
      "labels": [],
      "created_at": "2026-07-22T15:54:16Z",
      "updated_at": "2026-07-22T15:54:16Z"
    }
  ]
}
```

#### `bf ready --limit 5 --format json`

```json
{
  "version": 1,
  "kind": "ready",
  "data": [
    {
      "id": "bf-xyz789",
      "title": "Unblocked task",
      "status": "open",
      "priority": 1,
      "issue_type": "task",
      "assignee": null,
      "labels": ["urgent"],
      "created_at": "2026-07-22T14:00:00Z",
      "updated_at": "2026-07-22T14:00:00Z"
    }
  ]
}
```

#### `bf search "auth" --format json`

```json
{
  "version": 1,
  "kind": "search",
  "data": [
    {
      "id": "bf-abc123",
      "title": "Implement auth flow",
      "status": "open",
      "priority": 2,
      "issue_type": "task",
      "assignee": null,
      "labels": ["phase-1"],
      "created_at": "2026-07-22T15:54:16Z",
      "updated_at": "2026-07-22T15:54:16Z"
    }
  ]
}
```

#### `bf show bf-abc123 --format json`

```json
{
  "version": 1,
  "kind": "show",
  "data": {
    "id": "bf-abc123",
    "title": "Implement auth flow",
    "description": "Add OAuth2 authentication with refresh tokens",
    "status": "open",
    "priority": 2,
    "issue_type": "task",
    "assignee": null,
    "labels": ["phase-1", "backend"],
    "created_at": "2026-07-22T15:54:16Z",
    "updated_at": "2026-07-22T15:54:16Z",
    "dependencies": [],
    "comments": []
  }
}
```

### Analytics Commands

#### `bf stats --format json`

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
      "task": 90,
      "bug": 30,
      "feature": 30
    },
    "by_priority": {
      "0": 15,
      "1": 30,
      "2": 60,
      "3": 30,
      "4": 15
    }
  }
}
```

#### `bf velocity --format json`

```json
{
  "version": 1,
  "kind": "velocity",
  "data": [
    {
      "model": "claude-opus-4-7",
      "harness": "needle",
      "issue_type": "task",
      "sample_count": 142,
      "p50_seconds": 480,
      "p90_seconds": 1320,
      "avg_seconds": 600.5,
      "last_updated": "2026-07-22T15:00:00Z"
    },
    {
      "model": "claude-sonnet-4-6",
      "harness": "needle",
      "issue_type": "task",
      "sample_count": 87,
      "p50_seconds": 1080,
      "p90_seconds": 2700,
      "avg_seconds": 1350.2,
      "last_updated": "2026-07-22T15:00:00Z"
    }
  ]
}
```

## Testing and Validation

### Unit Tests

Comprehensive unit tests for envelope structure and serialization are located in:

- `src/format/envelope.rs` — Core envelope structure tests
- `src/format/json.rs` — JSON formatter tests

### Integration Tests

Integration tests verify envelope wrapping for specific commands:

- **List/Show tests**: Verify `list` and `show` commands emit correct envelopes
- **Claim/Stats tests**: Verify `claim` and `stats` commands emit correct envelopes
- Tests validate: structure, metadata fields, data shapes, and round-trip serialization

### Validation Checklist

When adding a new command or modifying envelope behavior:

- [ ] Verify envelope has all required fields (`version`, `kind`, `data`)
- [ ] Verify `version` is `1` (or current VERSION constant)
- [ ] Verify `kind` matches command name
- [ ] Verify `data` shape matches command specification
- [ ] Verify `warning` field appears only when needed
- [ ] Verify empty result handling matches specification
- [ ] Add/update unit tests in `src/format/envelope.rs`
- [ ] Add/update integration tests for command-specific behavior
- [ ] Update this specification document if structure changed

## References

- **Implementation**: `src/format/envelope.rs`
- **Formatter trait**: `src/format/mod.rs`
- **JSON formatter**: `src/format/json.rs`
- **Data models**: `src/model.rs`
- **User documentation**: `docs/README.md`
- **Implementation plan**: `docs/plan/plan.md` (Phase 7.7: JSON contract discipline)

## Migration from Non-Envelope Output

Prior to envelope standardization, commands emitted raw JSON without the wrapper. The envelope was added to:

1. Provide versioning for future compatibility
2. Enable machine-readable command identification
3. Support non-fatal error warnings
4. Fix inconsistencies between commands (some emitted arrays, others objects, others NDJSON)

### Migration Path

**Old behavior** (pre-envelope):
```bash
bf list --format json
# Output: NDJSON (one JSON object per line, no wrapper)
{"id":"bf-1",...}
{"id":"bf-2",...}
```

**New behavior** (envelope):
```bash
bf list --format json
# Output: Single envelope with array data
{"version":1,"kind":"list","data":[{"id":"bf-1",...},{"id":"bf-2",...}]}
```

**Compatibility**: Parsers expecting the old NDJSON format need to be updated to handle the envelope structure. The envelope is a breaking change from the previous format but provides a stable, machine-readable contract going forward.
