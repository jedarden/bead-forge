# Serialization Audit Report

## Overview
Complete audit of serialization paths across `src/jsonl.rs` and `src/format/` modules.

## Standard Issue Serde (src/model.rs:469)

The `Issue` struct uses comprehensive serde attributes:
- `#[serde(skip)]` on `content_hash` (never serialized)
- `#[serde(default, skip_serializing_if = "Option::is_none")]` on Option fields
- `#[serde(skip_serializing_if = "Vec::is_empty", default)]` on relations (labels, dependencies, comments)
- `#[serde(skip_serializing_if = "BTreeMap::is_empty")]` on annotations
- Custom `serialize_compaction_level` for bd conformance (None → 0)
- Relations (labels, dependencies, comments) are part of Issue struct itself

## Serialization Paths by File

### src/jsonl.rs ✅ CORRECT

**Lines using standard serde:**
- Lines 45, 63: `serde_json::from_str::<Issue>` - uses standard Issue serde
- Lines 88, 161: `serde_json::to_writer(issue)` / `serde_json::to_string(issue)` - uses standard Issue serde
- Lines 236-241: `incremental_flush()` SQL query loads all fields including labels via GROUP_CONCAT

**Conclusion:** All JSONL paths use the standard Issue serde with all fields preserved.

### src/format/json.rs ⚠️ NEEDS FIX

**Problem code (Lines 22-28):**
```rust
fn issue_to_value(issue: &Issue) -> Value {
    let mut stripped = issue.clone();
    stripped.dependencies = vec![];
    stripped.comments = vec![];
    serde_json::to_value(&stripped).unwrap_or(Value::Null)
}
```

**Usage:**
- Lines 32, 38: `format_issue()` and `format_issues()` use `issue_to_value()`

**Problem:**
- Output is INCONSISTENT with jsonl.rs
- Relations (dependencies, comments) are stripped even when populated
- Tests verify this stripped behavior (lines 147-156)

**Fix needed:**
1. Remove custom stripping logic
2. Use standard Issue serde directly
3. Allow standard serde's `skip_serializing_if` to handle empty collections

### src/format/text.rs ✅ CORRECT

- Text formatter - not JSON serialization
- Uses manual string formatting
- No changes needed

### src/format/toon.rs ✅ CORRECT

- Text formatter - not JSON serialization
- Uses manual string formatting  
- No changes needed

### src/format/envelope.rs ✅ CORRECT

- Wrapper module for command output
- Uses Value passed to it (doesn't serialize Issue itself)
- No changes needed

### src/format/warning.rs ✅ CORRECT

- Helper for injecting warning keys into JSON
- Doesn't serialize Issue itself
- No changes needed

## Specific Fixes Required

### 1. src/format/json.rs:22-28 (HIGH PRIORITY)

**Current code:**
```rust
fn issue_to_value(issue: &Issue) -> Value {
    let mut stripped = issue.clone();
    stripped.dependencies = vec![];
    stripped.comments = vec![];
    serde_json::to_value(&stripped).unwrap_or(Value::Null)
}
```

**Fix option A - Remove stripping:**
```rust
fn issue_to_value(issue: &Issue) -> Value {
    serde_json::to_value(issue).unwrap_or(Value::Null)
}
```

**Fix option B - Remove function entirely:**
```rust
// In format_issue:
fn format_issue(&self, issue: &Issue) -> String {
    serde_json::to_string(issue).unwrap_or_else(|_| "{}".to_string())
}

// In format_issues:
fn format_issues(&self, issues: &[Issue]) -> String {
    issues
        .iter()
        .map(|issue| serde_json::to_string(issue))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default()
        .join("\n")
}
```

**Rationale:**
- Aligns with jsonl.rs behavior (consistency)
- Preserves relations in JSON output when populated
- Standard Issue serde already skips empty collections via `skip_serializing_if = "Vec::is_empty"`
- Tests need updating to verify relations are preserved when present

## Test Updates Required

After fixing the serialization, update these tests in `src/format/json.rs`:

1. **Lines 147-156** - `assignee_skipped_when_unset` / `labels_skipped_when_empty` / `assignee_and_labels_populated_when_present`:
   - Currently verify field presence/absence
   - Should verify that dependencies and comments follow same pattern

2. **Lines 148-157** - `format_issues_guarantees_fields_per_line`:
   - Currently tests that assignee and labels keys are present on every line
   - Should also verify dependencies and comments keys

## Summary

- **Total files audited:** 7
- **Files using standard serde:** 5 (jsonl.rs, text.rs, toon.rs, envelope.rs, warning.rs)
- **Files using custom serializers:** 1 (json.rs)
- **Fixes needed:** 1 (remove relation stripping in json.rs: `issue_to_value()` function)

## Root Cause

`JsonFormatter` was designed to strip dependencies/comments for br compatibility, but this creates inconsistency:
- jsonl.rs exports full Issue with relations
- JsonFormatter exports Issue with relations stripped
- Two different serialization behaviors for the same data structure

The fix makes both paths use the standard Issue serde consistently.
