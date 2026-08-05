# Assignee Field Serialization Contract

**Document Version:** 2.0  
**Date:** 2026-08-05  
**Status:** Accepted  
**Bead ID:** bf-7o29bw  

## Overview

This document specifies the exact JSON serialization behavior for the `assignee` field across all output paths in bead-forge. **There are two distinct serialization contracts** depending on the output path:

1. **CLI Display Output Contract** (show, list, ready, search): Field is always PRESENT (null when unset)
2. **Storage/JSONL Export Contract** (sync, export): Field is ABSENT when None

This dual contract is intentional and serves different use cases.

## Single Source of Truth

**Location:** `src/model.rs:469-470`

```rust
/// Assigned user.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub assignee: Option<String>,
```

**Serde attributes:**
- `default`: Field defaults to `None` when deserializing missing keys
- `skip_serializing_if = "Option::is_none"`: Field is **omitted** during direct serialization when value is `None`

## Contract Matrix

| Input Value     | CLI Display Output | Storage/JSONL Export |
|-----------------|--------------------|----------------------|
| `None`          | `"assignee": null` (key present) | Key absent |
| `Some("alice")` | `"assignee": "alice"` | `"assignee": "alice"` |
| `Some("")`      | `"assignee": ""` | `"assignee": ""` |

---

## Serialization Path A: CLI Display Output

**Used by:** `bf show --json`, `bf list --json`, `bf ready --json`, `bf search --json`, `bf recent --json`

**Implementation:** `src/format/json.rs` via `JsonFormatter`

### Behavior

- Field is **always present** in JSON output
- When `None`: serializes to `null`
- When `Some(value)`: serializes to the string value
- When `Some("")`: serializes to empty string `""`

### Code Path

From `src/format/json.rs:27-43`:

```rust
fn issue_to_value(issue: &Issue) -> Value {
    let mut stripped = issue.clone();
    stripped.dependencies = vec![];
    stripped.comments = vec![];

    let mut value = serde_json::to_value(&stripped).unwrap_or(Value::Null);
    if let Value::Object(ref mut map) = value {
        ensure_display_fields(map);
    }
    value
}

fn ensure_display_fields(map: &mut Map<String, Value>) {
    map.entry("assignee").or_insert(Value::Null);
    map.entry("labels").or_insert_with(|| Value::Array(vec![]));
}
```

### Rationale

CLI consumers need to distinguish between:
- Field not set → `null`
- Field set to empty string → `""`

If the field were omitted when None, downstream code deserializing into a struct with `Option<String> assignee` would see `None` in both cases and couldn't tell the difference.

### Examples

**Assignee is None:**
```json
{
  "id": "bf-123",
  "title": "Example",
  "assignee": null,
  "labels": []
}
```

**Assignee is Some("alice"):**
```json
{
  "id": "bf-123",
  "title": "Example",
  "assignee": "alice",
  "labels": ["urgent"]
}
```

**Assignee is Some(""):**
```json
{
  "id": "bf-123",
  "title": "Example",
  "assignee": "",
  "labels": []
}
```

---

## Serialization Path B: Storage/JSONL Export

**Used by:** `bf sync --export`, direct JSONL file writes, storage layer serialization

**Implementation:** Direct `serde_json::to_string(issue)` respecting struct attributes

### Behavior

- Field follows `skip_serializing_if = "Option::is_none"` attribute
- When `None`: field is **absent** from JSON
- When `Some(value)`: field is present with the value
- When `Some("")`: field is present with empty string

### Code Path

From `src/jsonl.rs:88`:

```rust
for issue in &issues {
    serde_json::to_writer(&mut writer, issue)?;
    writer.write_all(b"\n")?;
}
```

This uses the `Serialize` impl on `Issue`, which respects the `skip_serializing_if` attribute.

### Rationale

JSONL is stored on disk and version-controlled (`.beads/issues.jsonl`). Omitting `None` values:
- Reduces file size (no redundant `null` entries for unassigned beads)
- Improves `git diff` readability (only meaningful changes appear)
- Maintains compatibility with `beads_rust` (br) format
- Allows distinguishing "never set" from "cleared" via commit history

### Examples

**Assignee is None:**
```json
{"id":"bf-123","title":"Example","status":"open","priority":2,"issue_type":"task","created_at":"2026-08-05T00:00:00Z","updated_at":"2026-08-05T00:00:00Z"}
```
(No `assignee` key present)

**Assignee is Some("alice"):**
```json
{"id":"bf-123","title":"Example","assignee":"alice","status":"open","priority":2,"issue_type":"task","created_at":"2026-08-05T00:00:00Z","updated_at":"2026-08-05T00:00:00Z"}
```

**Assignee is Some(""):**
```json
{"id":"bf-123","title":"Example","assignee":"","status":"open","priority":2,"issue_type":"task","created_at":"2026-08-05T00:00:00Z","updated_at":"2026-08-05T00:00:00Z"}
```

---

## Acceptance Criteria Resolution

The acceptance criteria mentioned **"null or absent"** — both are correct, for different paths:

- **CLI consumers**: Expect `null` when unset (field always present)
- **JSONL/git storage**: Expect absent key when unset (compact representation)

The phrase "null or absent" in the acceptance criteria refers to the **union of both contracts**, acknowledging that downstream code must handle both representations depending on context.

---

## Special Cases Per Command

### `bf show --format json`

Uses CLI display contract. `assignee` is always present.

```bash
$ bf show bf-123 --format json
[{"id":"bf-123",...,"assignee":null,...}]
```

### `bf list --format json`

Uses CLI display contract. Each line has `assignee` present.

```bash
$ bf list --format json
{"id":"bf-123",...,"assignee":"alice",...}
{"id":"bf-124",...,"assignee":null,...}
```

### `bf ready --format json`

Uses CLI display contract. Ready candidates have `assignee: null` before claiming.

### `bf sync --export`

Uses storage contract. `assignee` is omitted when `None`.

```jsonl
{"id":"bf-123","title":"Without assignee","status":"open",...}
{"id":"bf-124","title":"With assignee","assignee":"alice","status":"in_progress",...}
```

### `bf sync --import`

**Accepts both contracts**. Import uses `serde_json::from_str` with `#[serde(default)]`, so missing keys deserialize to `None`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub assignee: Option<String>,
```

Both representations import correctly:
- Missing key → `None`
- `"assignee": null` → `None`
- `"assignee": "alice"` → `Some("alice")`

---

## Database Storage

The database stores `assignee` as `TEXT` nullable column:

```sql
-- From src/storage/schema.rs
CREATE TABLE IF NOT EXISTS issues (
    ...
    assignee TEXT,
    ...
);
```

- `NULL` in database → Rust `None` → CLI `null`, JSONL key absent
- `"alice"` in database → Rust `Some("alice")` → CLI `"alice"`, JSONL `"alice"`
- `""` in database → Rust `Some("")` → CLI `""`, JSONL `""`

---

## Clearing Assignee

Per `src/model.rs:833-847`, the correct way to clear assignee:

```rust
#[must_use]
pub fn clear_assignee(&self, actor: String) -> IssueChanges {
    IssueChanges {
        assignee: Some(String::new()),  // <-- Some("") clears to NULL in DB
        actor: Some(actor),
        ..Default::default()
    }
}
```

The storage layer interprets `Some("")` as "clear to NULL":

- Before: `Some("alice")` in DB
- After: `NULL` in DB
- CLI output: `assignee: null`
- JSONL output: key absent

---

## Validation and Normalization

From `src/cli/mod.rs` (validation module):

```rust
pub fn normalize_assignee(assignee: Option<String>) -> Option<String> {
    assignee.and_then(|a| if a.trim().is_empty() { None } else { Some(a.trim().to_string()) })
}
```

**Behavior:**
- `None` → `None`
- `Some("")` → `None` (normalized)
- `Some("  ")` → `None` (whitespace only, normalized)
- `Some(" alice ")` → `Some("alice")` (whitespace trimmed)

---

## Tests

### Model Layer Tests (`src/model.rs`)

- `test_assignee_field_in_json_when_none`: Verifies field is **absent** when `None` (storage contract)
- `test_assignee_field_in_json_when_some`: Verifies field is **present** when `Some(value)`

### JSON Formatter Tests (`src/format/json.rs`)

- `assignee_null_when_unset`: Verifies CLI display contract (field present as `null`)
- `assignee_and_labels_populated_when_present`: Verifies populated values
- `format_issues_guarantees_fields_per_line`: Verifies every line has the field

### CLI Integration Tests (`src/cli/tests/show_json_tests.rs`)

- `test_show_json_empty_fields_serialize_correctly`: Verifies CLI output contract (lines 662-674)
- `test_show_json_special_characters_in_assignee`: Verifies value preservation (lines 352-394)

---

## Recommendations for Consumers

### CLI Output Consumers

When parsing `bf --format json` output:

```typescript
interface Bead {
  assignee: string | null;  // Always present, null when unassigned
  // ...
}
```

### JSONL/git Storage Consumers

When reading `issues.jsonl` directly:

```typescript
interface Bead {
  assignee?: string;  // Optional, absent when unassigned
  // ...
}
```

### Universal Consumer (handles both)

```typescript
interface Bead {
  assignee?: string | null;  // Optional, and may be null when present
  // ...
}

// Normalize to null
const normalized = bead.assignee ?? null;
```

---

## Migration from br (beads_rust)

This is **br-compatible**. The `beads_rust` Issue model uses identical serde attributes:

```rust
// From beads_rust/src/storage/schema.rs (Go)
// Issue.assignee is optional in JSON, omitted when empty/null
```

Both systems round-trip correctly:
- Export from br → Import to bf: Works
- Export from bf → Import to br: Works

---

## Future Considerations

If adding a new output path, choose the contract based on use case:

1. **Interactive/human-facing** (CLI, API responses): Use CLI display contract (field always present)
2. **Storage/transport** (files, caches): Use storage contract (omit when None)
3. **Never mix** within a single output format for consistency

**Do not change the `skip_serializing_if` attribute on `Issue.assignee`** — it would break br compatibility and JSONL round-tripping.

---

## References

- **Model definition:** `src/model.rs:469-470`
- **JSON formatter:** `src/format/json.rs:27-43`
- **Storage layer:** `src/storage/sqlite.rs` (assignee handling)
- **JSONL handling:** `src/jsonl.rs:88` (export), `src/jsonl.rs:63` (import)
- **CLI display:** `src/cli/mod.rs:1747-1780` (cmd_show function)
- **Clear assignee:** `src/model.rs:833-847` (clear_assignee method)

---

**Document History:**

| Version | Date | Changes | Author |
|---------|------|---------|--------|
| 1.0 | 2026-08-05 | Initial documentation (incorrect - claimed no null serialization) | bf-7o29bw investigation |
| 2.0 | 2026-08-05 | Corrected to document dual contract (CLI null vs storage absent) | bf-7o29bw investigation |
