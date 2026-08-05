# Serde-Based Relation Filtering Implementation

## Task
Implement serde-based relation filtering based on child 1 audit findings (bead bf-2scl9s).

## Child 1 Audit Findings

The comprehensive audit (bf-2scl9s) documented all custom Issue serialization logic in `src/format/` and concluded:

### Key Findings:

1. **Manual Stripping is Necessary** - The `issue_to_value()` function in `src/format/json.rs` intentionally strips `dependencies` and `comments` for:
   - **br compatibility** - Matches original br tool behavior
   - **Compact JSONL output** - Prevents extremely long lines
   - **Format consistency** - Expected by existing tests

2. **Current Serde Attributes are Correct** - `model.rs` already has appropriate `skip_serializing_if` attributes:
   - `#[serde(skip_serializing_if = "Vec::is_empty")]` on labels, dependencies, comments, events
   - `#[serde(default, skip_serializing_if = "Option::is_none")]` on optional fields
   - Custom serializer for `compaction_level` (bd conformance)

3. **Alternative Approaches Were Considered and Rejected**:
   - `#[serde(skip)]` would prevent serialization in ALL contexts (including JSONL export/import)
   - Custom serde serializers would be more complex than current manual stripping
   - Separate output structs would require duplicating the entire Issue struct

### Audit Recommendation:
> "✅ **DO NOT REMOVE** the `issue_to_value()` function that strips dependencies and comments."
> 
> "The audit found **no issues requiring fixes**. All custom serialization logic is intentional and appropriate."

## Implementation Status

### ✅ Complete - No Changes Needed

The current implementation is exactly what the child 1 audit recommended:

1. **model.rs (lines 556-583)**: Relation fields have correct serde attributes:
   ```rust
   #[serde(skip_serializing_if = "Vec::is_empty", default)]
   pub labels: Vec<String>,
   #[serde(skip_serializing_if = "Vec::is_empty", default)]
   pub dependencies: Vec<Dependency>,
   #[serde(skip_serializing_if = "Vec::is_empty", default)]
   pub comments: Vec<Comment>,
   ```

2. **format/json.rs (lines 16-69)**: Comprehensive documentation explains why manual stripping is necessary, with detailed rationale covering:
   - Selective exclusion (JSON formatter only, not JSONL export)
   - br compatibility requirements
   - JSONL line length concerns
   - Why `skip_serializing_if = "Vec::is_empty"` is not sufficient

3. **Documentation**: Both the audit notes (bf-2scl9s.md) and inline code comments thoroughly document the rationale.

## Why Serde Attributes Alone Are Insufficient

The `#[serde(skip_serializing_if = "Vec::is_empty")]` attribute only skips serialization when the collection is empty. However, the JSON formatter needs to **always** exclude relations (dependencies, comments) even when they contain data, while preserving them for:
- JSONL export/import roundtrips
- API responses
- Debug/inspection contexts

This selective exclusion requires the manual stripping approach in `issue_to_value()`.

## Compilation Issues

Note: There are unrelated compilation errors in the codebase (batch.rs, bead_store.rs) that prevent `cargo build` and `cargo test` from succeeding. These are separate from the serde relation filtering issue and should be addressed independently.

## Conclusion

**Child 1 found that manual relation stripping is necessary** - the current implementation is correct and matches the audit's recommendations. No changes to serde attributes or manual stripping logic are needed.
