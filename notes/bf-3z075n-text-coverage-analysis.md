# Text Formatter Test Coverage Analysis

**Bead:** bf-3z075n  
**Date:** 2026-08-05  
**Scope:** Comprehensive test coverage analysis for `src/format/text.rs`

## Executive Summary

The `TextFormatter` has **10 public API methods** and **4 helper functions**. Current test coverage shows significant gaps in unit testing for the main Formatter trait methods, relying primarily on integration tests for envelope handling.

### Coverage Overview
- **Fully Tested (unit tests):** 4/14 (29%)
- **Integration Tested:** 10/14 (71%)
- **No Unit Tests:** 10/14 (71%)
- **Critical Gaps:** Core formatting methods lack direct unit tests

---

## 1. TextFormatter Trait Methods

### 1.1 `format_issue(&self, issue: &Issue) -> String`
**Status:** ⚠️ **NO UNIT TESTS** (only integration coverage)

**What it does:**
- Formats single issue with all fields (ID, title, status, priority, type)
- Conditionally includes description, assignee, labels
- Formats timestamps in UTC

**Test Coverage:**
- ❌ No unit tests in `src/format/text.rs`
- ✅ Indirectly tested via `tests/envelope/text_format.rs::show_envelope_shows_detailed_info`

**Missing Edge Cases:**
1. Empty description (Some vs None)
2. Missing assignee
3. Empty labels array
4. Long titles/descriptions (truncation?)
5. Special characters in title/description
6. Unicode/non-ASCII characters
7. Different timestamps (UTC formatting verification)
8. All optional fields present
9. No optional fields present

**Test Priority:** 🔴 HIGH - Core display method

---

### 1.2 `format_issues(&self, issues: &[Issue]) -> String`
**Status:** ⚠️ **NO UNIT TESTS** (only integration coverage)

**What it does:**
- Formats multiple issues in compact list format
- One line per issue: `[ID] Title - Status (Priority)`

**Test Coverage:**
- ❌ No unit tests
- ✅ Indirectly tested via `list` integration tests

**Missing Edge Cases:**
1. Empty array (should return empty string)
2. Single issue
3. Multiple issues (order preservation)
4. Issues with special characters in titles
5. Very long issue arrays (100+ issues)
6. Issues with same titles
7. Unicode titles

**Test Priority:** 🔴 HIGH - Core list display method

---

### 1.3 `format_error(&self, message: &str) -> String`
**Status:** ⚠️ **NO UNIT TESTS**

**What it does:**
- Prefixes error message with "Error: " and newline

**Test Coverage:**
- ❌ No unit tests
- ⚠️ No integration tests found

**Missing Edge Cases:**
1. Empty message
2. Multi-line error messages
3. Special characters in message
4. Very long error messages
5. Error messages with newlines

**Test Priority:** 🟡 MEDIUM - Simple but untested

---

### 1.4 `format_claim_result(&self, result: &ClaimResultOutput) -> String`
**Status:** ⚠️ **NO UNIT TESTS** (only integration coverage)

**What it does:**
- Three different output modes based on `result` fields:
  - **Dry-run preview:** `{id} (priority=N, impact=N, workspace=PATH)`
  - **Cross-workspace claim:** `{id} (workspace: PATH)`
  - **Single-workspace claim:** Just the `{id}`

**Test Coverage:**
- ❌ No unit tests
- ✅ Indirectly tested via `tests/envelope/text_format.rs` claim tests

**Missing Edge Cases:**
1. Dry-run with all fields populated
2. Dry-run with missing priority/impact (should show 0)
3. Cross-workspace with empty workspace path
4. Cross-workspace with special characters in path
5. Single-workspace (minimal case)
6. All optional fields None vs Some with empty values
7. Unicode workspace paths
8. Very long workspace paths

**Test Priority:** 🔴 HIGH - Complex conditional logic

---

### 1.5 `format_no_claim(&self) -> String`
**Status:** ⚠️ **NO UNIT TESTS** (only integration coverage)

**What it does:**
- Returns static "No beads available to claim" message

**Test Coverage:**
- ❌ No unit tests
- ✅ Indirectly tested via `claim_envelope_empty_workspace`

**Missing Edge Cases:**
1. Simple output verification (exact string match)

**Test Priority:** 🟢 LOW - Simple static string

---

### 1.6 `format_stats(&self, stats: &StatsOutput) -> String`
**Status:** ⚠️ **NO UNIT TESTS** (delegates to helper, which has no unit tests)

**What it does:**
- Delegates to `format_stats_text` helper

**Test Coverage:**
- ❌ No unit tests for wrapper
- ❌ No unit tests for `format_stats_text` helper
- ✅ Integration tests via envelope/stats tests

**Missing Edge Cases:**
- See `format_stats_text` below

**Test Priority:** 🔴 HIGH - Delegates to untested helper

---

### 1.7 `format_velocity(&self, stats: &[VelocityStats]) -> String`
**Status:** ⚠️ **NO UNIT TESTS** (delegates to helper, which has no unit tests)

**What it does:**
- Delegates to `format_velocity_text` helper

**Test Coverage:**
- ❌ No unit tests for wrapper
- ❌ No unit tests for `format_velocity_text` helper

**Missing Edge Cases:**
- See `format_velocity_text` below

**Test Priority:** 🟡 MEDIUM - Velocity is a specialized feature

---

### 1.8 `format_with_envelope(&self, kind: &str, data: &str) -> String`
**Status:** ⚠️ **NO UNIT TESTS** (but extensively integration tested)

**What it does:**
- No-op for text formatter (returns `data` as-is)
- Comment: "Text formatter doesn't support envelope wrapping"

**Test Coverage:**
- ❌ No unit tests
- ✅ Extensively integration tested in `tests/envelope/text_format.rs`:
  - `stats_envelope_outputs_plain_text`
  - `stats_envelope_output_matches_no_envelope`
  - `claim_envelope_outputs_plain_text`
  - `list_envelope_outputs_plain_text`
  - `ready_envelope_outputs_plain_text`
  - `show_envelope_outputs_plain_text`

**Missing Edge Cases:**
1. Empty `data` string
2. Empty `kind` string
3. Special characters in data
4. Very long data strings

**Test Priority:** 🟡 MEDIUM - Well covered by integration tests

---

### 1.9 `format_with_envelope_and_warning(&self, kind: &str, data: &str, warning: Option<&str>) -> String`
**Status:** ⚠️ **NO UNIT TESTS** (but extensively integration tested)

**What it does:**
- No-op for text formatter (returns `data` as-is)
- Warning parameter ignored (text formatter has no warning display)

**Test Coverage:**
- ❌ No unit tests
- ✅ Integration tests cover the behavior (output matches non-envelope)

**Missing Edge Cases:**
1. Warning Some vs None
2. Empty warning string
3. Warning with special characters

**Test Priority:** 🟢 LOW - No-op for text formatter

---

## 2. Public Helper Functions

### 2.1 `format_stats_text(stats: &StatsOutput) -> String`
**Status:** ⚠️ **NO UNIT TESTS** (only integration coverage)

**What it does:**
- Formats aggregate stats with optional breakdowns
- Conditional sections: by_type, by_priority, by_assignee, by_label
- Special case: empty assignee shows "(no assigned beads)"

**Test Coverage:**
- ❌ No unit tests
- ✅ Integration tested via envelope stats tests

**Missing Edge Cases:**
1. Basic stats with no breakdowns
2. All breakdowns present
3. Each breakdown individually
4. Empty by_type map
5. Empty by_priority map
6. Empty by_assignee map (special message)
7. Empty by_label map
8. All counts zero
9. Very large counts
10. Special characters in type/assignee/label names
11. Unicode in type/assignee/label names
12. Very long type/assignee/label names

**Test Priority:** 🔴 HIGH - Complex conditional logic, widely used

---

### 2.2 `format_velocity_text(stats: &[VelocityStats]) -> String`
**Status:** ⚠️ **NO UNIT TESTS**

**What it does:**
- Formats velocity statistics as fixed-width table
- Header: Model, Harness, Type, Samples, P50(s), P90(s), Avg(s)
- One row per (model, harness, issue_type) cohort
- Special message when empty: "No velocity statistics available yet."

**Test Coverage:**
- ❌ No unit tests
- ❌ No integration tests found

**Missing Edge Cases:**
1. Empty stats array (special message)
2. Single stat row
3. Multiple stats rows (order preservation)
4. Optional percentiles present (Some values)
5. Optional percentiles missing (None → "-")
6. All optional fields missing
7. All optional fields present
8. Very long model/harness/type names (truncation?)
9. Very large sample counts
10. Floating point precision (P50, P90, Avg)
11. Unicode in names
12. Special characters in names

**Test Priority:** 🔴 HIGH - Complex formatting, no tests at all

---

### 2.3 `format_dependencies(dependencies: &[Dependency]) -> String`
**Status:** ✅ **FULLY UNIT TESTED** (5 tests)

**What it does:**
- Formats dependencies with optional "(blocks)" suffix
- Empty array returns empty string
- Blocking dependencies get "(blocks)" suffix
- Non-blocking dependencies don't

**Test Coverage:**
- ✅ `test_format_dependencies_empty` - Empty array
- ✅ `test_format_dependencies_blocking` - Single blocking
- ✅ `test_format_dependencies_non_blocking` - Single non-blocking
- ✅ `test_format_dependencies_mixed` - Multiple mixed
- ✅ `test_format_dependencies_unknown_title` - Missing title
- ✅ `test_format_dependencies_multiple_blocking` - Multiple blocking

**Missing Edge Cases:**
1. Very long dependency lists (50+)
2. Special characters in titles
3. Unicode titles
4. Different dependency types (all blocking types)
5. All non-blocking types

**Test Priority:** 🟢 LOW - Well covered

---

### 2.4 `format_dependencies_display(dependencies: &[DependencyDisplay]) -> String`
**Status:** ⚠️ **NO UNIT TESTS**

**What it does:**
- Similar to `format_dependencies` but uses storage `DependencyDisplay` struct
- Includes bead title from JOIN with issues table
- Blocking dependencies get "(blocks)" suffix
- Empty array returns empty string

**Test Coverage:**
- ❌ No unit tests

**Missing Edge Cases:**
1. Empty array
2. Single blocking dependency
3. Single non-blocking dependency
4. Mixed blocking/non-blocking
5. Multiple blocking
6. Empty bead_id
7. Empty title
8. Special characters in title
9. Unicode in title
10. Different dep_type values (blocks vs others)

**Test Priority:** 🟡 MEDIUM - Used by storage layer, but simple logic

---

## 3. Test Coverage Matrix

| Method/Fn | Unit Tests | Integration Tests | Coverage Score | Priority |
|-----------|-----------|-------------------|----------------|----------|
| `format_issue` | ❌ | ✅ | 50% | 🔴 HIGH |
| `format_issues` | ❌ | ✅ | 50% | 🔴 HIGH |
| `format_error` | ❌ | ❌ | 0% | 🟡 MEDIUM |
| `format_claim_result` | ❌ | ✅ | 50% | 🔴 HIGH |
| `format_no_claim` | ❌ | ✅ | 50% | 🟢 LOW |
| `format_stats` | ❌ | ✅ | 50% | 🔴 HIGH |
| `format_velocity` | ❌ | ❌ | 0% | 🔴 HIGH |
| `format_with_envelope` | ❌ | ✅ | 50% | 🟡 MEDIUM |
| `format_with_envelope_and_warning` | ❌ | ✅ | 50% | 🟢 LOW |
| `format_stats_text` | ❌ | ✅ | 50% | 🔴 HIGH |
| `format_velocity_text` | ❌ | ❌ | 0% | 🔴 HIGH |
| `format_dependencies` | ✅ | ✅ | 100% | 🟢 LOW |
| `format_dependencies_display` | ❌ | ❌ | 0% | 🟡 MEDIUM |

**Overall Coverage:** 15% unit test coverage (2/13 methods with unit tests)

---

## 4. Testing Checklist

### Phase 1: Critical Path (HIGH Priority)

- [ ] **bf-3z075n-t1**: `format_issue` unit tests
  - [ ] Basic case with all required fields
  - [ ] With optional description
  - [ ] With optional assignee
  - [ ] With labels
  - [ ] With all optional fields
  - [ ] Empty description (None vs Some(""))
  - [ ] Special characters in fields
  - [ ] Unicode characters
  - [ ] Timestamp formatting verification

- [ ] **bf-3z075n-t2**: `format_issues` unit tests
  - [ ] Empty array
  - [ ] Single issue
  - [ ] Multiple issues
  - [ ] Special characters in titles
  - [ ] Unicode titles
  - [ ] Order preservation

- [ ] **bf-3z075n-t3**: `format_claim_result` unit tests
  - [ ] Dry-run preview (all fields)
  - [ ] Dry-run with missing priority/impact
  - [ ] Cross-workspace claim
  - [ ] Single-workspace claim (minimal)
  - [ ] Empty workspace path
  - [ ] Special characters in workspace

- [ ] **bf-3z075n-t4**: `format_stats_text` unit tests
  - [ ] Basic stats (no breakdowns)
  - [ ] With by_type breakdown
  - [ ] With by_priority breakdown
  - [ ] With by_assignee breakdown (empty case)
  - [ ] With by_label breakdown
  - [ ] All breakdowns present
  - [ ] All counts zero
  - [ ] Special characters in breakdown keys
  - [ ] Unicode in keys

- [ ] **bf-3z075n-t5**: `format_velocity_text` unit tests
  - [ ] Empty array (special message)
  - [ ] Single row
  - [ ] Multiple rows
  - [ ] All percentiles present
  - [ ] Some percentiles missing
  - [ ] All percentiles missing
  - [ ] Long names (truncation verification)
  - [ ] Floating point precision

### Phase 2: Important Gaps (MEDIUM Priority)

- [ ] **bf-3z075n-t6**: `format_error` unit tests
  - [ ] Basic error message
  - [ ] Empty message
  - [ ] Multi-line message
  - [ ] Special characters

- [ ] **bf-3z075n-t7**: `format_dependencies_display` unit tests
  - [ ] Empty array
  - [ ] Single blocking
  - [ ] Single non-blocking
  - [ ] Mixed blocking/non-blocking
  - [ ] Special characters in title
  - [ ] Unicode title

- [ ] **bf-3z075n-t8**: `format_with_envelope` unit tests
  - [ ] Basic no-op behavior
  - [ ] Empty data
  - [ ] Special characters
  - [ ] Verify data returned unchanged

### Phase 3: Nice to Have (LOW Priority)

- [ ] **bf-3z075n-t9**: `format_no_claim` unit test
  - [ ] Exact string match

- [ ] **bf-3z075n-t10**: `format_with_envelope_and_warning` unit test
  - [ ] Warning Some vs None
  - [ ] Verify data returned unchanged

### Phase 4: Edge Case Expansion

- [ ] **bf-3z075n-t11**: Performance/stress tests
  - [ ] 100+ issues in `format_issues`
  - [ ] 100+ dependencies in `format_dependencies`
  - [ ] Very long strings (1000+ chars)
  - [ ] Deep nesting in data structures

---

## 5. Test Structure Recommendations

### File Organization
```
src/format/text.rs              # Keep existing unit tests for format_dependencies
tests/text_formatter_unit.rs   # NEW: Dedicated unit tests for all methods
tests/text_formatter_integration.rs  # NEW: End-to-end CLI tests
tests/envelope/text_format.rs   # Existing: Keep envelope tests
```

### Test Naming Convention
```rust
#[test]
fn test_<method>_<scenario>_<expected>() {
    // e.g., test_format_issue_with_all_fields_full_output()
}
```

### Test Data Pattern
```rust
mod test_data {
    pub fn mock_issue() -> Issue { ... }
    pub fn mock_issue_with_labels() -> Issue { ... }
    pub fn mock_claim_result_dry_run() -> ClaimResultOutput { ... }
}
```

---

## 6. Recommendations

### Immediate Actions
1. **Create dedicated unit test file** for `TextFormatter` methods
2. **Prioritize high-value methods**: `format_issue`, `format_issues`, `format_claim_result`, `format_stats_text`, `format_velocity_text`
3. **Add benchmark tests** for large inputs (100+ issues)

### Medium-term Goals
1. **Expand edge case coverage** for special characters, Unicode, empty inputs
2. **Add property-based tests** using proptest for invariants (e.g., output always contains expected fields)
3. **Document expected output format** in docstrings with examples

### Long-term Goals
1. **Achieve 80%+ unit test coverage** for text formatter
2. **Add snapshot tests** for complex formatting (velocity table, stats breakdowns)
3. **Integration test suite** covering all CLI commands with --format text

---

## 7. Dependencies on Other Beads

This analysis identifies test gaps but does not implement them. Beads needed:

- **Unit test infrastructure bead**: Setup test framework, test data helpers
- **format_issue tests bead**: Implement `format_issue` unit tests
- **format_issues tests bead**: Implement `format_issues` unit tests
- **format_claim_result tests bead**: Implement claim formatting unit tests
- **format_stats_text tests bead**: Implement stats formatting unit tests
- **format_velocity_text tests bead**: Implement velocity formatting unit tests

Each bead should include:
1. Unit tests for the specific method
2. Test data helpers for that method
3. Documentation of edge cases covered
4. Commit with test code only

---

## 8. Notes

- **Labels text format tests** (`tests/test_labels_text_format.rs`) are all ignored due to workspace isolation issue (bf-3uk2w5). These should be re-enabled after fixing that defect.
- **Envelope integration tests** provide good coverage for the no-op behavior of envelope methods, reducing urgency for unit tests in that area.
- **Velocity formatting** is completely untested at unit level - this is a significant gap since it has complex table formatting logic.
- **Stats formatting** is complex (four optional breakdown sections) but has no unit tests, only integration tests.

---

**End of Analysis**
