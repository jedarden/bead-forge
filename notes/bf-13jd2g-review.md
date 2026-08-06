# Formatter Test Coverage Review (bf-6dd3xi)

**Date:** 2026-08-05
**Scope:** Review existing formatter tests across the codebase and document gaps

## Summary

The format module has **partial test coverage** with significant gaps in error handling, edge cases, and integration scenarios. While JSON formatter has reasonably good coverage for core functionality, text and toon formatters lack comprehensive tests.

## Files Reviewed

### 1. `src/format/json.rs` (322 lines)

#### Existing Tests (11 tests)

**skip_serializing_if behavior:**
- ✅ `assignee_skipped_when_unset` - Verifies assignee is omitted when None
- ✅ `labels_skipped_when_empty` - Verifies labels are omitted when empty vec
- ✅ `assignee_and_labels_populated_when_present` - Verifies fields appear when set

**format_issues output shape:**
- ✅ `format_issues_guarantees_fields_per_line` - Ensures consistent field presence per line
- ✅ `format_issues_empty_yields_empty_string` - Empty input produces empty output
- ✅ `format_issues_single_yields_one_valid_json_line` - Single issue outputs exactly one line
- ✅ `format_issues_multiple_yields_jsonl_one_object_per_line` - Multiple issues produce proper JSONL

**Claim result formatting:**
- ✅ `claim_dry_run_emits_only_preview_keys` - Dry-run includes only preview fields
- ✅ `claim_single_workspace_omits_workspace_key` - Normal claims omit workspace
- ✅ `no_claim_is_empty_object` - No claim returns `{}`

#### Critical Gaps

**Missing unit tests:**
- ❌ `format_error()` - No tests for error message JSON formatting
- ❌ `format_stats()` - No tests for StatsOutput serialization
- ❌ `format_velocity()` - No tests for VelocityStats array serialization
- ❌ `format_with_envelope()` - No tests for JsonEnvelope wrapping
- ❌ `format_with_envelope_and_warning()` - No tests for envelope + warning

**Missing integration tests:**
- ❌ `issue_to_value()` - No tests for manual dependencies/comments stripping
- ❌ Complex nested dependencies/comments verification
- ❌ Error handling when serde_json::to_string fails
- ❌ JsonEnvelope behavior (to_json_compact, with_warning methods)
- ❌ Non-UTF-8 character handling in issue fields

**Missing edge case tests:**
- ❌ Very long titles/descriptions that might produce oversized JSON lines
- ❌ Special characters in issue fields (quotes, backslashes, newlines)
- ❌ Unicode/emoji handling in all text fields
- ❌ Empty vs null distinction in JSON output
- ❌ Malformed data fallback behavior

---

### 2. `src/format/text.rs` (371 lines)

#### Existing Tests (6 tests)

**Dependency formatting:**
- ✅ `test_format_dependencies_empty` - Empty vec returns empty string
- ✅ `test_format_dependencies_blocking` - Blocking dependencies include "(blocks)"
- ✅ `test_format_dependencies_non_blocking` - Related dependencies omit "(blocks)"
- ✅ `test_format_dependencies_mixed` - Mixed dependency types formatted correctly
- ✅ `test_format_dependencies_unknown_title` - Missing title defaults to "Unknown"
- ✅ `test_format_dependencies_multiple_blocking` - Multiple blocking dependencies

#### Critical Gaps

**Missing Formatter trait method tests:**
- ❌ `format_issue()` - Full issue text rendering not tested
- ❌ `format_issues()` - List rendering not tested
- ❌ `format_error()` - Error message formatting not tested
- ❌ `format_claim_result()` - Claim result formatting not tested (3 branches)
- ❌ `format_no_claim()` - No claim message not tested
- ❌ `format_velocity()` - Velocity table formatting not tested
- ❌ `format_with_envelope()` - Envelope passthrough not tested
- ❌ `format_with_envelope_and_warning()` - Envelope+warning passthrough not tested

**Missing helper function tests:**
- ❌ `format_stats_text()` - Stats aggregation display not tested
- ❌ `format_velocity_text()` - Velocity table rendering not tested
- ❌ `format_dependencies_display()` - Storage DependencyDisplay formatting not tested

**Missing edge case tests:**
- ❌ Very long issue titles in list view
- ❌ Empty description handling
- ❌ Missing optional fields (description, assignee, labels)
- ❌ Date/time formatting edge cases
- ❌ Empty stats breakdowns (empty by_type, by_priority, etc.)
- ❌ Special characters in issue fields

---

### 3. `src/format/toon.rs` (135 lines)

#### Existing Tests (0 tests)

**❌ ZERO TESTS - CRITICAL GAP**

The toon formatter has **no test coverage at all**. This is a significant gap since toon is one of three core output formats.

#### Critical Gaps

**All Formatter trait methods untested:**
- ❌ `format_issue()` - Toon issue rendering not tested
- ❌ `format_issues()` - Toon list rendering not tested
- ❌ `format_error()` - Error messages not tested
- ❌ `format_claim_result()` - Claim results not tested (3 branches)
- ❌ `format_no_claim()` - No claim message not tested
- ❌ `format_stats()` - Stats not tested (delegates to text but not verified)
- ❌ `format_velocity()` - Velocity formatting not tested
- ❌ `format_with_envelope()` - Envelope passthrough not tested
- ❌ `format_with_envelope_and_warning()` - Envelope+warning not tested

**Missing helper function tests:**
- ❌ `format_toon_issue_line()` - Line formatting helper not tested
- ❌ `format_priority()` - Priority string formatting not tested
- ❌ `format_ready_bead()` - Ready bead display not tested
- ❌ `format_dependencies()` - Delegation to text not verified

**Missing edge case tests:**
- ❌ All edge cases from text.rs also apply here
- ❌ Toon-specific rendering differences from text not verified

---

## Specific Test Cases Needed for skip_serializing_if Behavior

The Issue struct uses `#[serde(skip_serializing_if)]` attributes. These should be explicitly tested:

### For all formatters (JSON, text, toon):

**Test that None/empty values are omitted:**
1. `assignee: None` → assignee key omitted
2. `assignee: Some("")` → assignee key omitted (if empty string treated as None)
3. `labels: []` → labels key omitted
4. `dependencies: []` → dependencies key omitted
5. `comments: []` → comments key omitted
6. `description: None` → description key omitted
7. `acceptance_criteria: None` → acceptance_criteria key omitted
8. `workspace: None` → workspace key omitted
9. `completion_reason: None` → completion_reason key omitted
10. `completion_comment: None` → completion_comment key omitted

**Test that present values ARE serialized:**
1. `assignee: Some("worker")` → key present with value
2. `labels: vec!["label1".to_string()]` → key present with array
3. Non-empty collections should always appear

**For JSON formatter specifically:**
- Verify JSON output doesn't include null values for skipped fields
- Verify JSON output doesn't include empty arrays for skipped fields
- Verify compact output (no pretty-printing)

---

## Other Format Modules

### `src/format/mod.rs`

**Missing tests:**
- ❌ `OutputFormat::from_str()` - Case sensitivity, invalid inputs
- ❌ `OutputFormat::as_str()` - String representation
- ❌ `get_formatter()` - Factory function returns correct formatter

### `src/format/envelope.rs` (42KB - large module)

**Missing tests:**
- ❌ JsonEnvelope serialization
- ❌ Envelope version handling
- ❌ Warning attachment behavior
- ❌ to_json_compact() method

### `src/format/table.rs`

**Missing tests:**
- ❌ TableFormatter implementation
- ❌ Table formatting for various data types

### `src/format/color.rs`

**Missing tests:**
- ❌ Color formatting functions
- ❌ Status color mapping
- ❌ Terminal color handling

### `src/format/warning.rs`

**Missing tests:**
- ❌ Warning formatting
- ❌ Stderr warning output

---

## Priority Recommendations

### P0 - Critical (Must Fix)
1. **Add basic tests for toon.rs** - Zero coverage is unacceptable
2. **Test error handling paths** - What happens when serialization fails?
3. **Test envelope wrapping** - Core feature with no tests

### P1 - High (Should Fix)
1. **Complete text.rs Formatter method tests** - Only dependencies are tested
2. **Test skip_serializing_if behavior exhaustively** - Document what gets omitted
3. **Add stats/velocity formatting tests** - Used in production but untested

### P2 - Medium (Nice to Have)
1. **Edge case tests** - Long strings, special characters, Unicode
2. **Integration tests** - Full command output verification
3. **Performance tests** - Large issue lists (1000+)

---

## Test Infrastructure Suggestions

1. **Add test helpers** - Common Issue::new() patterns, assertions
2. **Property-based tests** - Use proptest for edge case generation
3. **Golden file tests** - Compare output against known-good files
4. **Round-trip tests** - Parse → format → parse → compare
5. **Snapshot tests** - Use insta for output verification

---

## Conclusion

The formatter module has **adequate coverage for the happy path in JSON output**, but significant gaps remain:

- **toon.rs**: 0% test coverage
- **text.rs**: ~20% test coverage (dependencies only)
- **json.rs**: ~50% test coverage (core functionality, missing error handling)
- **Other modules**: Minimal to no coverage

**Estimated overall coverage:** ~30-40% of formatter code paths

This is insufficient for a critical output formatting layer. A comprehensive test suite is needed before considering the formatter module production-ready.
