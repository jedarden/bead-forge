# JSON Formatter Examination (Bead bf-321gp)

## Overview
Examined the JSON formatter implementation in `src/format/json.rs` to understand current functionality and identify what's implemented vs. what might be missing.

## Current Implementation Status

### All Required Formatter Methods Are Implemented

The `JsonFormatter` implements all methods from the `Formatter` trait:

1. **`format_issue(&self, issue: &Issue) -> String`** ✓
   - Serializes a single issue to JSON
   - Applies display normalization (assignee always present, labels always array)

2. **`format_issues(&self, issues: &[Issue]) -> String`** ✓
   - Serializes multiple issues to JSONL format
   - One JSON object per line, newline-separated
   - Empty input yields empty string (not "[]")

3. **`format_error(&self, message: &str) -> String`** ✓
   - Returns `{"error": "message"}`

4. **`format_claim_result(&self, result: &ClaimResultOutput) -> String`** ✓
   - Serializes claim result to JSON

5. **`format_no_claim(&self) -> String`** ✓
   - Returns `{}`

6. **`format_stats(&self, stats: &StatsOutput) -> String`** ✓
   - Serializes stats to JSON

7. **`format_velocity(&self, stats: &[VelocityStats]) -> String`** ✓
   - Serializes velocity array to JSON

8. **`format_with_envelope(&self, kind: &str, data: &str) -> String`** ✓
   - Wraps data in `JsonEnvelope` with version/kind/data structure
   - Attempts to parse data as JSON; falls back to string if parsing fails

9. **`format_with_envelope_and_warning(&self, kind: &str, data: &str, warning: Option<&str>) -> String`** ✓
   - Same as above with optional warning field

## Helper Functions

- **`issue_to_value(issue: &Issue) -> Value`** - Converts Issue to serde JSON Value, strips dependencies/comments for br compatibility
- **`ensure_display_fields(map: &mut Map<String, Value>)`** - Ensures assignee (null when empty) and labels (empty array when empty) are always present

## Supporting Infrastructure

- **`JsonEnvelope`** in `src/format/envelope.rs` - Complete implementation with:
  - `new(kind, data)` constructor
  - `with_warning(message)` method
  - `to_json()` for pretty-printed output
  - `to_json_compact()` for compact output

## Test Results

### Integration Tests (tests/test_json_formatter.rs)
All 12 tests pass:
- `test_json_formatter_single_issue` ✓
- `test_json_formatter_multiple_issues` ✓
- `test_json_formatter_empty_issues` ✓
- `test_json_formatter_strips_dependencies_and_comments` ✓
- `test_json_formatter_error_formatting` ✓
- `test_output_format_from_str` ✓
- `test_output_format_as_str` ✓
- `test_get_formatter` ✓
- `test_format_with_envelope_single_issue` ✓
- `test_format_with_envelope_multiple_issues` ✓
- `test_format_with_envelope_and_warning` ✓
- `test_json_formatter_assignee_and_labels_normalization` ✓

### Unit Tests (src/format/json.rs)
All 10 tests pass:
- `assignee_null_when_unset` ✓
- `labels_empty_array_when_none` ✓
- `assignee_and_labels_populated_when_present` ✓
- `format_issues_guarantees_fields_per_line` ✓
- `format_issues_empty_yields_empty_string` ✓
- `format_issues_single_yields_one_valid_json_line` ✓
- `format_issues_multiple_yields_jsonl_one_object_per_line` ✓
- `claim_dry_run_emits_only_preview_keys` ✓
- `claim_single_workspace_omits_workspace_key` ✓
- `no_claim_is_empty_object` ✓

## Key Design Decisions

1. **Display Normalization**: `assignee` and `labels` are always present in output (null when empty, empty array when empty) for CLI consumer compatibility

2. **JSONL Format**: `format_issues` outputs one JSON object per line (NDJSON), not a JSON array

3. **Empty Input**: Empty issues vector produces empty string, not "[]"

4. **br Compatibility**: Dependencies and comments are stripped from serialized issues

5. **Envelope Parsing**: When wrapping data in an envelope, the formatter attempts to parse the data string as JSON first. If parsing fails, it stores the data as a string. This means:
   - Single issue JSON → parsed as object in envelope
   - JSONL (multiple issues) → stored as string in envelope (since concatenated JSONL is not valid JSON)

## Conclusion

The JSON formatter is **fully implemented** with all required methods working correctly. All tests pass. The implementation follows br compatibility requirements and provides proper envelope wrapping for JSON output.
