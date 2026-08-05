# Assignee Serialization Investigation (bf-6bmvsf)

## Summary

Investigated all code paths where Issues are serialized to JSON to identify assignee field handling. Found one path that correctly normalizes assignee and one path that relies on serde default behavior.

## Code Paths That Serialize Issue to JSON

### 1. JSONL Export (src/jsonl.rs) ✅ CORRECT FOR STORAGE
**Location:** Lines 76-78 (export), Line 52 (import)
**Method:** `serde_json::to_writer(&mut writer, issue)` and `serde_json::from_str::<Issue>()`
**Status:** RELIES ON STANDARD SERDE BEHAVIOR

- Uses standard serde serialization with `#[serde(skip_serializing_if = "Option::is_none")]`
- When assignee is None, the field is SKIPPED in JSONL output
- This is CORRECT for on-disk storage (compact, bd-compatible)
- Test at line 926 confirms import handles assignee correctly

### 2. CLI JSON Output (src/format/json.rs) ✅ CORRECT FOR DISPLAY
**Location:** Lines 27-43
**Method:** Custom `issue_to_value()` with `ensure_display_fields()`
**Status:** ACTIVELY NORMALIZES ASSIGN

```rust
fn ensure_display_fields(map: &mut Map<String, Value>) {
    map.entry("assignee").or_insert(Value::Null);
    map.entry("labels").or_insert_with(|| Value::Array(vec![]));
}
```

- Strips dependencies/comments (lines 28-30)
- **Guarantees assignee key is always present** (as null when unset)
- **Guarantees labels key is always present** (as empty array when unset)
- Tests at lines 123-134 verify this behavior

### 3. Text Output (src/format/text.rs) ✅ OK FOR TEXT
**Location:** Lines 20-22
**Method:** Conditional display
**Status:** CORRECT FOR TEXT FORMAT

```rust
if let Some(assignee) = &issue.assignee {
    s.push_str(&format!("Assignee: {}\n", assignee));
}
```

- Shows assignee line only when present (normal for text output)
- No issues here

### 4. Toon Output (src/format/toon.rs) ✅ OK FOR TOON
**Location:** Lines 21-23
**Method:** Conditional display
**Status:** CORRECT FOR TOON FORMAT

```rust
if let Some(assignee) = &issue.assignee {
    parts.push(format!("Assignee: {}", assignee));
}
```

- Shows assignee line only when present (normal for toon output)
- No issues here

### 5. Database Storage (src/storage/sqlite.rs) ✅ CORRECT
**Location:** Lines 162, 185, 272, 297 (SELECT), Lines 575-582 (UPDATE)
**Method:** Direct SQL parameter binding
**Status:** FULLY SUPPORTED

- assignee field properly read/written to SQLite
- Handles empty string as NULL (lines 576-579)
- Tracks assignee changes for events (lines 511-520)
- No issues here

### 6. Claim Result Output (src/format/mod.rs) ✅ CORRECT
**Location:** Lines 28-44
**Method:** Dedicated struct with required String field
**Status:** ALWAYS PRESENT

```rust
pub struct ClaimResultOutput {
    pub bead_id: String,
    pub assignee: String,  // Required field, not Option
    ...
}
```

- assignee is a required String field
- Always present in claim results
- No issues here

## Issue Model Definition (src/model.rs)

**Location:** Lines 469-470

```rust
/// Assigned user.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub assignee: Option<String>,
```

**Behavior:**
- When `Some(value)`: field is present in JSON with the value
- When `None`: field is SKIPPED in JSON (not present as null)

## Conclusion

**NO ISSUES FOUND** - All serialization paths handle assignee correctly:

1. **JSONL export**: Skips assignee when None (correct for compact on-disk storage)
2. **CLI JSON output**: Always includes assignee as null when unset (correct for display)
3. **Text/Toon output**: Shows assignee only when present (correct for human-readable output)
4. **Database**: Properly stores and retrieves assignee with NULL handling
5. **Claim results**: Always includes assignee as required field

The implementation correctly distinguishes between storage format (compact) and display format (normalized/complete).
