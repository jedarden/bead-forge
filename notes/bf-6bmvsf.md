# Assignee Serialization Investigation (bf-6bmvsf)

## Task
Identify all code paths where Issues are serialized to JSON and verify assignee field handling.

## Issue Definition
From `src/model.rs` lines 469-470:
```rust
/// Assigned user.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub assignee: Option<String>,
```

The field has `skip_serializing_if = "Option::is_none"` which means:
- When `Some(value)` → field is serialized
- When `None` → field is skipped (not present in JSON)

This is CORRECT for compact JSONL storage and bd compatibility.

## All Serialization Code Paths

### 1. JSONL Export (`src/jsonl.rs`)

#### `export_jsonl` (line 63-89)
```rust
for issue in &issues {
    serde_json::to_writer(&mut writer, issue)?;
}
```
**Status:** ✅ CORRECT - Uses standard serde serialization, assignee handled correctly

#### `export_jsonl_merge` (line 107-172)
```rust
for issue in upserts {
    by_id.insert(issue.id.clone(), serde_json::to_string(issue)?);
}
```
**Status:** ✅ CORRECT - Uses standard serde serialization

#### `export_jsonl_dirty` (line 191-208)
Calls `export_jsonl_merge` internally
**Status:** ✅ CORRECT

#### `stream_issues` (line 29-36)
```rust
let line = line?;
serde_json::from_str::<Issue>(&line).map_err(Into::into)
```
**Status:** ✅ CORRECT - Import path, uses standard deserialization

#### `import_jsonl` (line 38-61)
```rust
let issue: Issue = serde_json::from_str(&line)?;
```
**Status:** ✅ CORRECT - Import path, uses standard deserialization

### 2. CLI JSON Output (`src/format/json.rs`)

#### `issue_to_value` (line 27-37)
Special function that strips dependencies/comments and ensures display fields:
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
```

**Status:** ✅ CORRECT - Calls `ensure_display_fields` which adds assignee if missing

#### `ensure_display_fields` (line 39-43)
```rust
fn ensure_display_fields(map: &mut Map<String, Value>) {
    map.entry("assignee").or_insert(Value::Null);
    map.entry("labels").or_insert_with(|| Value::Array(vec![]));
}
```
**Status:** ✅ CORRECT - Ensures assignee is ALWAYS present in CLI JSON output (null when unset)

#### `JsonFormatter::format_issue` (line 46-48)
```rust
fn format_issue(&self, issue: &Issue) -> String {
    serde_json::to_string(&issue_to_value(issue)).unwrap_or_else(|_| "{}".to_string())
}
```
**Status:** ✅ CORRECT - Uses `issue_to_value` which ensures assignee field

#### `JsonFormatter::format_issues` (line 50-57)
```rust
fn format_issues(&self, issues: &[Issue]) -> String {
    issues
        .iter()
        .map(|issue| serde_json::to_string(&issue_to_value(issue)))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default()
        .join("\n")
}
```
**Status:** ✅ CORRECT - Uses `issue_to_value` for each issue

### 3. CLI Command Output

#### `cmd_show` (src/cli/mod.rs: line ~1780)
```rust
let mut out = issue;
out.dependencies = vec![];
out.comments = vec![];
let formatter = get_formatter(OutputFormat::Json);
let json_str = formatter.format_issue(&out);
```
**Status:** ✅ CORRECT - Uses JsonFormatter which ensures assignee

#### `cmd_create` (src/cli/mod.rs: line ~1631)
**Manually constructs JSON** - needs verification:
```rust
let data = serde_json::json!({
    "id": id,
    "title": issue.title,
    "type": issue.issue_type.to_string(),
    "priority": issue.priority.0,
    "status": issue.status.to_string(),
    "description": issue.description,
    "assignee": issue.assignee,
    "labels": issue.labels
});
```
**Status:** ✅ CORRECT - Explicitly includes assignee field

#### `cmd_ready` (src/cli/mod.rs: line ~1967-2010)
```rust
let issues: Vec<Issue> = candidates
    .iter()
    .filter_map(|c| storage.get_issue(&c.id).ok().flatten())
    .collect();
let jsonl = formatter.format_issues(&issues);
```
**Status:** ✅ CORRECT - Uses JsonFormatter.format_issues

#### `cmd_list` (src/cli/mod.rs: line ~1654-1762)
```rust
let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
let formatter = get_formatter(output_format);
// ...
let jsonl = formatter.format_issues(&issues);
```
**Status:** ✅ CORRECT - Uses JsonFormatter.format_issues

### 4. Storage Layer (`src/storage/sqlite.rs`)

No direct serialization to JSON - uses rusqlite for database operations.

### 5. Sync Module (`src/sync.rs`)

Multiple serialization points in tests (lines 586, 644, 719, 725, 805):
```rust
writeln!(file, "{}", serde_json::to_string(&newer_issue).unwrap()).unwrap();
```
**Status:** ✅ CORRECT - Uses standard serde serialization

### 6. Rotate Module (`src/rotate.rs`)

Lines 157, 171, 520:
```rust
serde_json::to_writer(&mut writer, bead)?;
```
**Status:** ✅ CORRECT - Uses standard serde serialization

### 7. Other Modules

- `src/log.rs` line 143: Event serialization (not Issue)
- `src/claim.rs` lines 281, 365: Worker metadata (not Issue)
- `src/recovery.rs` line 130: Recovery manifest (not Issue)
- `src/robot_docs.rs` line 349: Robot docs (not Issue)
- `src/config.rs` line 330: Config metadata (not Issue)
- `src/timing.rs` lines 161, 208, 295, 454, 550: Timing state (not Issue)

**Status:** ✅ N/A - These don't serialize Issue structs

## Summary

### Paths with CORRECT assignee handling:
1. ✅ **JSONL Export** (`src/jsonl.rs`): All functions use standard serde serialization
2. ✅ **CLI JSON Output** (`src/format/json.rs`): Special handling ensures assignee is always present
3. ✅ **CLI Commands**: All commands use JsonFormatter or explicit JSON construction
4. ✅ **Sync Module**: Uses standard serde serialization
5. ✅ **Rotate Module**: Uses standard serde serialization

### Key Design Decision:
The codebase has TWO serialization strategies:

1. **Storage/JSONL Format** (`src/jsonl.rs`):
   - Uses standard serde serialization
   - `assignee` field has `skip_serializing_if = "Option::is_none"`
   - When `None`, field is OMITTED from JSON (compact format)
   - When `Some(value)`, field is present
   - This is CORRECT for disk storage and bd compatibility

2. **CLI Display Format** (`src/format/json.rs`):
   - Uses `issue_to_value()` which calls `ensure_display_fields()`
   - `assignee` field is ALWAYS present (even when `None` → `null`)
   - This is CORRECT for JSON consumers who need stable structure
   - Ensures downstream filters can distinguish "unset" from "omitted"

### Conclusion:
**NO ISSUES FOUND** - All serialization paths correctly handle the assignee field:
- JSONL export/import: Uses standard serde, field appears when set
- CLI JSON output: Ensures field is always present (null when unset)
- Manual JSON construction: Explicitly includes assignee

The dual serialization strategy is intentional and correct for different use cases.
