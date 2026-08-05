# BF-13JD2G: Formatter Tests for Serde Behavior

## Summary

Verified and enhanced test coverage for standard serde `skip_serializing_if` behavior across all formatters.

## Existing Test Coverage

### src/format/json.rs Tests

The JSON formatter has comprehensive tests for serde behavior:

1. **Option Fields (skip_serializing_if = "Option::is_none")**
   - `assignee_skipped_when_unset()` - Verifies assignee field is omitted when None
   - `assignee_and_labels_populated_when_present()` - Verifies assignee appears when Some
   - `optional_fields_skipped_when_none()` - Comprehensive test covering all major Option fields

2. **Vec Fields (skip_serializing_if = "Vec::is_empty")**
   - `labels_skipped_when_empty()` - Verifies labels field is omitted when empty
   - `assignee_and_labels_populated_when_present()` - Verifies labels appear when populated
   - `dependencies_field_skipped_when_empty()` - Verifies dependencies are omitted
   - `comments_field_skipped_when_empty()` - Verifies comments are omitted
   - `events_field_skipped_when_empty()` - Verifies events are omitted

3. **BTreeMap Fields (skip_serializing_if = "BTreeMap::is_empty")**
   - `annotations_field_skipped_when_empty()` - Verifies annotations are omitted when empty
   - `annotations_field_included_when_populated()` - Verifies annotations appear when populated

4. **Boolean Fields (skip_serializing_if = "is_false")**
   - `boolean_fields_skipped_when_false()` - Verifies ephemeral, pinned, is_template omitted when false
   - `boolean_fields_included_when_true()` - Verifies boolean fields appear when true

5. **Additional Coverage**
   - `format_issues_guarantees_fields_per_line()` - Ensures consistent field presence across JSONL lines
   - `format_issues_empty_yields_empty_string()` - Empty input handling
   - `format_issues_single_yields_one_valid_json_line()` - Single issue formatting
   - `format_issues_multiple_yields_jsonl_one_object_per_line()` - JSONL format
   - Claim result tests with proper field omission for dry_run, workspace, and reclaimed

### src/model.rs Tests

Core serde behavior tests at the Issue model level:

- `test_empty_vectors_skipped_in_serialization()` - Verifies empty vectors (labels, dependencies, comments) are skipped
- `test_assignee_field_in_json_when_none()` - Verifies assignee is skipped when None
- `test_assignee_field_in_json_when_some()` - Verifies assignee appears when Some
- `test_compaction_level_serializes_as_zero_when_none()` - Special serialization for compaction_level
- `test_full_issue_with_all_fields()` - Comprehensive roundtrip test

### src/format/text.rs Tests

Text formatter tests focus on manual formatting functions:

- `test_format_dependencies_empty()` - Empty dependency formatting
- `test_format_dependencies_blocking()` - Blocking dependency format
- `test_format_dependencies_non_blocking()` - Non-blocking dependency format
- `test_format_dependencies_mixed()` - Mixed dependency types
- `test_format_dependencies_unknown_title()` - Missing title handling
- `test_format_dependencies_multiple_blocking()` - Multiple blockers

Note: Text formatter doesn't use serde serialization - it uses manual string formatting, so serde skip_serializing_if behavior doesn't apply.

### src/format/toon.rs Tests

Toon formatter uses the same dependency formatting as text formatter:

- Inherits text::format_dependencies() for dependency formatting
- No separate serde serialization - uses manual formatting like text formatter

## Issue Struct Serde Attributes

The Issue struct (src/model.rs) has these skip_serializing_if patterns:

### Option Fields (skip_serializing_if = "Option::is_none")
- description
- design  
- acceptance_criteria
- notes
- assignee ✓ tested
- owner
- estimated_minutes
- created_by
- closed_at
- close_reason
- closed_by_session
- due_at
- defer_until
- external_ref
- source_system
- source_repo
- deleted_at
- deleted_by
- delete_reason
- original_type
- compacted_at
- compacted_at_commit
- original_size
- sender

### Vec Fields (skip_serializing_if = "Vec::is_empty")
- labels ✓ tested
- dependencies ✓ tested
- comments ✓ tested
- events ✓ tested

### BTreeMap Fields (skip_serializing_if = "BTreeMap::is_empty")
- annotations ✓ tested

### Boolean Fields (skip_serializing_if = "is_false")
- ephemeral ✓ tested
- pinned ✓ tested
- is_template ✓ tested

### Special Cases
- compaction_level: Uses custom serializer `serialize_compaction_level` to always serialize as integer (0 when None)

## Test Coverage Completeness

✅ **Complete** - All major skip_serializing_if patterns are tested:
- Option fields with is_none
- Vec fields with is_empty  
- BTreeMap fields with is_empty
- Boolean fields with custom is_false function
- Special serialization for compaction_level

## Standard Serde Behavior Verified

The tests verify that serde's standard `skip_serializing_if` attribute behavior works correctly:

1. **Empty collections omitted**: Vec and BTreeMap fields are omitted when empty
2. **None omitted**: Option fields are omitted when None
3. **False omitted**: Boolean fields with custom skip function are omitted when false
4. **Values included**: All fields appear when they have actual values
5. **Roundtrip safety**: Full issue serialization/deserialization preserves all data

## Notes

- The JSON formatter manually strips dependencies and comments via `issue_to_value()` for br compatibility and JSONL line length (documented in function comments)
- The standard serde attributes still apply to all other fields
- Text and Toon formatters don't use serde - they use manual string formatting
