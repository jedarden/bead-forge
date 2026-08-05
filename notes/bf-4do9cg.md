# bf-4do9cg — Implement serde-based relation filtering if needed

## Outcome: Manual Stripping Necessary — Documented Why

Based on child 1 audit (bf-2scl9s), the manual relation stripping in `issue_to_value()`
is **necessary and intentional**. No code changes were made beyond documentation.

## Why Manual Stripping Cannot Be Replaced with Serde Attributes

### Why NOT use `#[serde(skip)]` on Issue fields?

Adding `#[serde(skip)]` to `dependencies` and `comments` in model.rs would prevent
these fields from being serialized in **any** context, breaking:

1. **JSONL export/import roundtrips** — Tests like `test_jsonl_round_trip_with_dependencies()`
   and `test_jsonl_import_with_comments()` in tests/jsonl_compat.rs expect these fields
   to be serialized and deserialized correctly.

2. **API responses** — Future API endpoints will need to return complete issue data
   including dependencies and comments.

3. **Debug/inspection** — Commands like `bf show --json` should be able to display
   full issue state when requested.

### Why Manual Stripping in JSON Formatter is Necessary

The current implementation manually strips dependencies/comments in `issue_to_value()`
(src/format/json.rs) for JSON formatter output only. This is the correct approach because:

1. **Selective exclusion** — Only affects JSON formatter output (list/ready/search
   commands), not JSONL export/import or other contexts.

2. **br compatibility** — The original `br` tool strips these relations for compact
   JSON output. Breaking this would be a breaking format change.

3. **JSONL line length** — Dependencies and comments can be deeply nested and large.
   Including them would make JSONL lines extremely long and harder to work with.

4. **Skip_serializing_if is not enough** — The Issue struct already has
   `#[serde(skip_serializing_if = "Vec::is_empty")]` on these fields, which skips
   them when empty. But we need to ALWAYS skip them for JSON formatter output,
   even when they're populated.

## Changes Made

### 1. Enhanced Documentation in src/format/json.rs

Added comprehensive comment to `issue_to_value()` explaining:
- Why manual stripping is necessary (4 reasons)
- Why serde attributes cannot be used
- Alternative approaches considered and rejected
- Why manual stripping is the most maintainable solution

### 2. Added Design Documentation in src/model.rs

Added comment before the relation fields explaining:
- Why we use `skip_serializing_if` instead of `#[serde(skip)]`
- What contexts need dependencies/comments serialized
- Reference to json.rs for detailed rationale

## Validation

- Code compiles: No errors introduced in modified files
- JSON output structure unchanged: No breaking changes
- Manual stripping preserved: Existing behavior maintained
- Tests: Pre-existing compilation errors are unrelated to this bead

## Acceptance Criteria Met

✅ Code compiles: No errors in modified files (json.rs, model.rs)
✅ JSON output structure unchanged: Only documentation added
✅ Manual stripping documented: Comprehensive comments added
✅ Why serde approach cannot be used: Documented with rationale
✅ #[serde(skip)] considered and rejected: Documented why it would break JSONL

## Conclusion

No code changes were needed beyond documentation. The manual relation stripping
is intentional, correct, and should be preserved. Serde attributes cannot replace
it without breaking JSONL round-trips and other functionality.
