# Assignee Field Serialization Contract

**Document Version:** 1.0  
**Date:** 2026-08-05  
**Status:** Accepted  

## Overview

This document specifies the exact JSON serialization behavior for the `assignee` field across all output paths in bead-forge. The contract is enforced by the `Issue` struct's Serde attributes and applies uniformly to all serialization contexts.

## Single Source of Truth

**Location:** `src/model.rs:469-470`

```rust
/// Assigned user.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub assignee: Option<String>,
```

## Serialization Contract

### Field Definition

- **Type:** `Option<String>`
- **Default:** `None`
- **Serde attribute:** `skip_serializing_if = "Option::is_none"`

### Behavior Matrix

| Rust Value | JSON Output | Interpretation |
|------------|--------------|----------------|
| `None` | **Absent** (field not present) | No assignee assigned |
| `Some("alice")` | `"assignee": "alice"` | Assigned to "alice" |
| `Some("")` | `"assignee": ""` | Assigned to empty string (rare, transitional state) |

### Key Contract Rules

1. **Never serializes as `null`**: The field is either present with a string value or absent entirely. The JSON `null` value never appears in output.

2. **None → Absent**: When `assignee` is `None`, the field is completely omitted from JSON output. This is the canonical representation of "unassigned."

3. **Empty string is valid**: `Some("")` serializes as an empty string. This is a transitional state used internally during clear operations (see "Clearing Assignee" below).

## All Serialization Paths

The following paths all use the same `Issue` struct and thus obey this contract:

### 1. JSONL Export (`src/jsonl.rs`)
- **Function:** `export_jsonl()`
- **Method:** `serde_json::to_writer(issue)`
- **Output:** One JSON object per line, assignee field follows contract

### 2. CLI Commands
All commands that output JSON use the same serialization:

- **`bf show --json`**: Single issue JSON
- **`bf list --json`**: Array of issues JSON
- **`bf search --json`**: Array of matching issues JSON
- **`bf ready --json`**: Array of ready candidates (converted to full Issue)
- **`bf batch`**: BatchResult objects with nested Issue data

### 3. API Responses
- **Internal APIs**: All functions returning `Issue` serialize identically
- **Error responses**: Errors containing Issue data follow contract

## Storage and Clearing Behavior

### Clearing the Assignee Field

When clearing an assignee via CLI or API:

1. **Input:** User provides empty assignee value (e.g., `bf update bf-123 --assignee ""`)
2. **Internal:** `IssueChanges.assignee = Some(String::new())`
3. **Storage layer:** Detects empty string and sets database field to `NULL`
4. **Model after reload:** `assignee = None`
5. **Serialization:** Field is absent from JSON

### Storage Layer Contract

From `src/storage/sqlite.rs` (approximate location):

```rust
if let Some(ref assignee) = assignee {
    if assignee.trim().is_empty() {
        updates.push("assignee = NULL");
        // Clear to NULL in database
    } else {
        updates.push("assignee = ?");
        params.push(Box::new(assignee.clone()));
    }
}
```

**Database Representation:**
- Assigned: `assignee` column contains the username string
- Unassigned: `assignee` column is `NULL`

## Import/Export Roundtrip

### Import (JSONL → Database)

When importing from JSONL (`src/jsonl.rs:import_jsonl`):

```rust
let issue: Issue = serde_json::from_str(&line)?;
// upsert to database
```

**Input handling:**
- Absent field → `assignee = None` → Database `NULL`
- Present field with value → `assignee = Some(value)` → Database stores value
- Present field with empty string → `assignee = Some("")` → Database stores empty string

### Export (Database → JSONL)

When exporting to JSONL (`src/jsonl.rs:export_jsonl`):

```rust
for issue in &issues {
    serde_json::to_writer(&mut writer, issue)?;
    writer.write_all(b"\n")?;
}
```

**Output behavior:**
- Database `NULL` → Model `None` → Field absent from JSON
- Database value → Model `Some(value)` → Field present with value

### Roundtrip Guarantees

**Lossless roundtrip:** A bead exported to JSONL and re-imported will produce the same database state, with one exception:

- Empty string assignee (`Some("")`) is preserved in the roundtrip but should not occur in normal usage.

## Validation and Normalization

### Input Normalization

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

### Display in Text Output

When displaying assignees in non-JSON formats (text, table, etc.):

- **Assigned:** Shows the assignee name
- **Unassigned:** Shows "-" or "(unassigned)" depending on context

## Command-Specific Behavior

### bf create
```bash
bf create "New task" --assignee "alice"
```
- Creates bead with `assignee = Some("alice")`
- JSON output: `"assignee": "alice"`

### bf update
```bash
bf update bf-123 --assignee "bob"    # Set assignee
bf update bf-123 --assignee ""       # Clear assignee
```
- Setting: Updates to `Some(value)`
- Clearing: Transitions through `Some("")` → database `NULL` → `None`

### bf list --assignee
```bash
bf list --assignee "alice"    # Filter by assignee
bf list --assignee ""         # Filter unassigned beads
```
- Empty string filter matches beads where assignee is `None`

### bf claim
```bash
bf claim
```
- Claims bead and sets `assignee = Some(<claimer>)`
- Uses claimant identity from config/session

## Compliance Matrix

| Command | Absent Field | Null Value | Empty String | Valid Value |
|---------|--------------|------------|--------------|-------------|
| `create` | ✓ (default) | N/A | Normalized to None | ✓ |
| `update` | Skipped | N/A | Clears to None | ✓ |
| `show` | ✓ (if None) | Never | Rare (see above) | ✓ |
| `list` | ✓ (if None) | Never | Rare (see above) | ✓ |
| `export` | ✓ (if None) | Never | Rare (see above) | ✓ |
| `import` | Becomes None | N/A | Becomes Some("") | ✓ |

## Test Coverage

### Unit Tests (src/model.rs)

- `test_assignee_field_in_json_when_none`: Verifies field is absent when `None`
- `test_assignee_field_in_json_when_some`: Verifies field is present when `Some(value)`

### Integration Tests (tests/test_assignee.rs)

- `test_create_bead_with_assignee`: Creation with assignee
- `test_create_bead_without_assignee`: Creation without assignee
- `test_update_bead_assignee`: Updating assignee
- `test_clear_bead_assignee`: Clearing assignee via empty string
- `test_list_beads_by_assignee`: Filtering by assignee
- `test_list_unassigned_beads`: Filtering for None/unassigned

## Deviations and Special Cases

### No Deviations

This contract applies uniformly across all serialization paths. There are no command-specific deviations. The `Issue` struct is the single source of truth, and all paths serialize through the same Serde implementation.

### Edge Cases

1. **Empty string assignee**: Technically valid but rare. Should only appear transiently during clear operations before normalization.

2. **Whitespace-only assignee**: Normalized to `None` during input processing.

3. **Special characters**: Assignee can contain any valid UTF-8 string, including:
   - Email addresses: `"alice@example.com"`
   - Spaces: `"Alice Smith"`
   - Hyphens: `"alice-worker-1"`

## Comparison to br (beads_rust)

This contract is **br-compatible**. The upstream `beads_rust` implementation uses the same Serde attributes and serialization behavior. Key compatibility points:

1. Both use `skip_serializing_if = "Option::is_none"`
2. Both interpret empty string as "clear to NULL"
3. Both never serialize as `null`
4. JSONL roundtrip is lossless between implementations

## Migration Notes

When migrating from older versions or alternative implementations:

1. **Check for `null` values**: Legacy data might contain `"assignee": null` in JSON. These should be normalized to absent fields during migration.

2. **Empty strings**: Legacy data might contain empty assignee strings. These should be normalized to `None` / absent during migration.

3. **Validation**: Add validation during import to reject or normalize unexpected states.

## Future Considerations

### Potential Changes (None Currently Planned)

If future requirements demand different behavior (e.g., always serializing the field), the following would need updating:

1. **Serde attribute** in `src/model.rs`
2. **This document** (contract version bump)
3. **All tests** to match new behavior
4. **Migration logic** for existing data

### Backwards Compatibility

Any change to this contract must maintain backwards compatibility with:

- Existing JSONL exports
- CLI tools parsing bead-forge JSON output
- External integrations expecting current format

## References

- **Model definition:** `src/model.rs:469-470`
- **Storage layer:** `src/storage/sqlite.rs` (assignee handling)
- **Tests:** `tests/test_assignee.rs`, `src/model.rs` (unit tests)
- **Batch operations:** `src/batch.rs` (assignee in batch ops)
- **JSONL handling:** `src/jsonl.rs` (import/export)

---

**Document History:**

| Version | Date | Changes | Author |
|---------|------|---------|--------|
| 1.0 | 2026-08-05 | Initial documentation of assignee serialization contract | bf-7o29bw investigation |
