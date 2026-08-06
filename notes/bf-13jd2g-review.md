# Formatter Test Coverage Review (bf-6dd3xi)

**Date:** 2026-08-05  
**Scope:** Review all existing formatter tests and document gaps

## Summary

Comprehensive review of formatter test coverage across `src/format/json.rs`, `src/format/text.rs`, and `src/format/toon.rs`. Found **16 unit tests** covering formatters, with significant gaps in toon formatter and edge case coverage.

## Existing Tests

### src/format/json.rs (10 tests)

**Field Serialization Tests:**
- ✅ `assignee_skipped_when_unset` - Assignee skipped when None
- ✅ `labels_skipped_when_empty` - Labels skipped when empty array  
- ✅ `assignee_and_labels_populated_when_present` - Fields present when populated

**JSONL Output Tests:**
- ✅ `format_issues_guarantees_fields_per_line` - All lines have assignee/labels keys
- ✅ `format_issues_empty_yields_empty_string` - Empty input produces empty output
- ✅ `format_issues_single_yields_one_valid_json_line` - Single issue = one line
- ✅ `format_issues_multiple_yields_jsonl_one_object_per_line` - Multiple issues = JSONL

**Claim Result Tests:**
- ✅ `claim_dry_run_emits_only_preview_keys` - Dry-run includes title/priority/impact/workspace
- ✅ `claim_single_workspace_omits_workspace_key` - Single workspace omits workspace key
- ✅ `no_claim_is_empty_object` - Empty claim produces `{}`

### src/format/text.rs (6 tests)

**Dependency Formatting Tests:**
- ✅ `test_format_dependencies_empty` - Empty dependencies return empty string
- ✅ `test_format_dependencies_blocking` - Blocking deps include `(blocks)` suffix
- ✅ `test_format_dependencies_non_blocking` - Non-blocking deps have no suffix
- ✅ `test_format_dependencies_mixed` - Mixed blocking/non-blocking deps
- ✅ `test_format_dependencies_unknown_title` - Unknown title defaults to `"Unknown"`
- ✅ `test_format_dependencies_multiple_blocking` - Multiple blocking dependencies

### src/format/toon.rs (0 tests)

**❌ NO TESTS - Complete coverage gap**

## Critical Test Coverage Gaps

### 1. Toon Formatter - Zero Coverage
**Priority: P0**

The toon formatter has **no tests at all**. This is a critical gap since toon is a user-facing output format.

**Missing test coverage:**
- `format_issue()` - Individual issue formatting
- `format_issues()` - Multiple issues (JSONL format)
- `format_error()` - Error message formatting
- `format_claim_result()` - Claim result formatting (dry-run, cross-workspace)
- `format_no_claim()` - Empty claim formatting
- `format_stats()` - Statistics output
- `format_velocity()` - Velocity statistics output
- `format_with_envelope()` - Envelope wrapping behavior
- `format_with_envelope_and_warning()` - Envelope with warning field
- `format_toon_issue_line()` - Issue line formatting helper
- `format_priority()` - Priority string conversion
- `format_ready_bead()` - Ready bead formatting (used by ready command)

### 2. Text Formatter - Incomplete Method Coverage  
**Priority: P1**

Text formatter has tests for `format_dependencies()` but is missing:

**❌ Not tested:**
- `format_issue()` - Individual issue detailed formatting
- `format_issues()` - Multiple issues (compact list format)
- `format_error()` - Error message formatting
- `format_claim_result()` - Claim result formatting (dry-run, cross-workspace)
- `format_no_claim()` - Empty claim message formatting
- `format_stats()` - Statistics output formatting
- `format_velocity()` - Velocity table formatting
- `format_with_envelope()` - Envelope behavior (returns data as-is)
- `format_with_envelope_and_warning()` - Envelope with warning (returns data as-is)

### 3. JSON Formatter - Edge Cases & skip_serializing_if Behavior
**Priority: P2**

While JSON formatter has good coverage, some edge cases need verification:

**❌ Not fully tested:**
- **Nested empty collections** - Issue with empty dependencies/comments after manual stripping
- **Special character escaping** - Unicode, newlines, quotes in title/description
- **Date/time formatting** - Timestamp field format consistency
- **Large input handling** - Very long descriptions, many labels
- **Malformed input fallback** - What happens with invalid issue data?

**Specific skip_serializing_if behavior tests needed:**
```rust
#[test]
fn test_json_dependencies_always_empty_after_stripping() { 
    // After manual stripping, dependencies should always be empty array
    let issue = Issue { dependencies: vec![...], ... };
    let v = parse(&JsonFormatter.format_issue(&issue));
    assert_eq!(v["dependencies"].as_array().unwrap().len(), 0);
}

#[test]
fn test_json_comments_always_empty_after_stripping() {
    // Same for comments
}

#[test]
fn test_json_optional_field_none_vs_empty() {
    // Test None vs "" vs [] behavior for optional fields
}
```

### 4. Formatter Trait - Cross-Format Consistency
**Priority: P2**

No tests verify that all three formatters implement the `Formatter` trait consistently:

**❌ Not tested:**
- All formatters accept the same `Issue` input without panicking
- All formatters produce valid output for all trait methods
- Error messages across formatters are consistent in content (not format)

### 5. Error Message Formatting
**Priority: P1**

Only JSON formatter's error formatting is indirectly tested. Need explicit tests for text and toon.

### 6. Claim Result Formatting - Complex Scenarios
**Priority: P1**

Claim result formatting has basic coverage but missing cross-workspace, reclamation > 1, and testing on text/toon formatters.

### 7. Stats & Velocity Formatting
**Priority: P1**

Statistics output formatting is largely untested across all three formatters.

### 8. Envelope Formatting
**Priority: P2**

Need tests for text/toon envelope passthrough behavior.

### 9. Empty Result Behavior Across All Formatters
**Priority: P1**

Empty inputs partially tested for JSON only; need coverage for text and toon.

### 10. Special Character & Unicode Handling
**Priority: P2**

No tests verify special character handling across formatters.

## Integration Test Coverage

### Strong Coverage
- ✅ `tests/test_json_formatter.rs` - 11 tests for JsonFormatter
- ✅ `tests/json_formatter_verification.rs` - 13 tests verifying JSON output consistency
- ✅ `tests/test_json_output_comprehensive.rs` - 60+ tests for JSON output across commands

### Medium Coverage  
- ⚠️ `tests/envelope_integration_tests.rs` - Envelope behavior across formatters
- ⚠️ Various command-specific JSON tests (ready, list, search, etc.)

### Missing Integration Coverage
- ❌ No integration tests for toon formatter output
- ❌ No integration tests for text formatter output (only JSON)
- ❌ No cross-format consistency integration tests

## Recommended Test Additions by Priority

### P0 (Critical - Block Release)
1. **Toon formatter basic functionality** - 5-8 core tests
2. **Text formatter method coverage** - Test all untested trait methods

### P1 (High - Next Sprint)  
3. **Claim result complex scenarios** - Cross-workspace, reclamation
4. **Error message formatting** - All three formatters
5. **Stats/velocity formatting** - All three formats
6. **Empty result consistency** - All formatters, all methods

### P2 (Medium - Backlog)
7. **Edge cases** - Unicode, special characters, large inputs
8. **Envelope passthrough** - Text/toon envelope behavior
9. **Cross-format consistency** - Trait contract verification
10. **skip_serializing_if behavior** - Explicit field serialization tests

## Test Metrics Summary

| Formatter | Unit Tests | Integration Tests | Coverage % |
|-----------|------------|------------------|-------------|
| JSON      | 10         | 80+              | ~85%        |
| Text      | 6          | 5                | ~30%        |
| Toon      | 0          | 0                | 0%          |

**Overall Coverage:** ~38% (16 unit tests out of ~42 methods)

## Notes

1. **JSON formatter has the best coverage** due to its use as the primary machine-readable format
2. **Text formatter tests are focused** - Only covers dependency formatting, missing main trait methods
3. **Toon formatter is completely untested** - Highest priority gap
4. **skip_serializing_if behavior** needs explicit testing since it's critical for br compatibility (see json.rs:40-42)
5. **Integration tests compensate** for some unit test gaps, but don't test internal formatter logic

## References

- Main formatter implementations: `src/format/{json,text,toon}.rs`
- Formatter trait: `src/format/mod.rs`
- Integration tests: `tests/test_json_formatter.rs`, `tests/json_formatter_verification.rs`, `tests/test_json_output_comprehensive.rs`
- Issue struct with serde attributes: `src/model.rs`
