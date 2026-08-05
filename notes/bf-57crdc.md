# Assignee Serialization Review (bf-57crdc)

## Task
Apply assignee field serialization fixes to all JSON export code paths identified in child bead bf-6bmvsf.

## Investigation Result

After thorough review, **NO CHANGES ARE NEEDED**. All serialization paths are working correctly.

## What the Child Bead (bf-6bmvsf) Found

The child bead completed a comprehensive investigation of all 7 Issue serialization code paths and concluded:

> **NO ISSUES FOUND** - All serialization paths correctly handle the assignee field:
> - JSONL export/import: Uses standard serde, field appears when set
> - CLI JSON output: Ensures field is always present (null when unset)
> - Manual JSON construction: Explicitly includes assignee
> 
> The dual serialization strategy is intentional and correct for different use cases.

## Current Implementation Status

### Model (src/model.rs:469) ✅ CORRECT
```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub assignee: Option<String>,
```
This correctly omits `assignee` from JSON when `None`.

### JSONL Export (src/jsonl.rs) ✅ CORRECT
All functions use standard `serde_json::to_writer(&mut writer, issue)` which respects the model's `skip_serializing_if` attribute. Result: `assignee` is omitted when `None` (compact storage format).

### CLI Display Output (src/format/json.rs) ✅ CORRECT
The `ensure_display_fields()` function intentionally ensures `assignee` is ALWAYS present in CLI JSON output (null when unset). This provides stable structure for downstream consumers.

From the code comments:
```rust
/// The `Issue` struct skips `assignee` when `None` and `labels` when empty so
/// the on-disk JSONL stays compact and `bd`-compatible. That is the wrong shape
/// for CLI consumers of `ready`/`list`/`search --format json: a downstream
/// filter that deserializes into a struct with optional `assignee`/`labels`
/// fields cannot tell an omitted key from a genuinely unset value. We therefore
/// normalize the display output so `assignee` is always emitted (`null` when
/// unset) and `labels` is always an array (`[]` when empty).
```

## Conclusion

**NO FIXES NEEDED** - All paths correctly handle the assignee field:

1. **Storage Format (JSONL)**: Uses standard serde serialization, `assignee` omitted when `None` ✅
2. **Display Format (CLI)**: Intentionally ensures stable structure, `assignee` always present (null when unset) ✅
3. **Manual Construction**: Explicitly includes `assignee` field ✅

The dual serialization strategy is **intentional and correct** for different use cases (compact storage vs. stable display structure).
