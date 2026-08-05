# CLI Test Suite Failure Report

**Date:** 2026-08-05
**Bead:** bf-4fbyxh
**Task:** Run CLI test suite and document failures

## Test Suite Summary

| Test Suite | Total Tests | Passed | Failed | Ignored |
|-----------|-------------|--------|--------|---------|
| cli_integration_crud | 45 | 37 | 8 | 0 |
| test_p0_epic_cli | 12 | 9 | 3 | 0 |
| comments_cli | 3 | 3 | 0 | 0 |
| comprehensive_label_cli | 16 | 15 | 0 | 1 |
| test_epic_with_labels_cli | 14 | 0 | 0 | 14 |
| **TOTAL** | **90** | **64** | **11** | **15** |

## Failures by Type

### Panic Failures (10)

All test failures were **panics** - assertion failures with explicit error messages.

### By CLI Command Affected

| Command | Failure Count | Tests |
|---------|---------------|-------|
| `bf create` | 2 | test_create_bead_invalid_type_error, test_create_bead_json_output |
| `bf show` (JSON) | 2 | test_p0_epic_json_serialization, test_p0_bead_json_output_format |
| `bf list` | 2 | test_p0_epic_label_filtering, test_p0_epic_priority_comparison |
| `bf label` | 2 | test_p0_epic_with_multiple_labels_cli, test_p0_bead_label_operations |
| `bf update` | 1 | test_reopen_open_bead_error |
| `bf search` | 1 | test_p0_bead_search_by_label |
| Lifecycle (multiple commands) | 1 | test_full_bead_lifecycle |

## Detailed Failure Analysis

### 1. **test_create_bead_invalid_type_error**
- **File:** tests/cli_integration_crud.rs:320
- **Type:** Panic
- **Message:** `create with invalid type should fail`
- **Root Cause:** Test expects `bf create` with invalid type to fail, but it succeeds instead
- **Impact:** Input validation is not working correctly for custom types

### 2. **test_create_bead_json_output**
- **File:** tests/cli_integration_crud.rs:292
- **Type:** Panic (Assertion)
- **Message:** `assertion failed: has_field(&json, "id")`
- **Root Cause:** JSON output from `bf create --format json` is missing the `id` field
- **Impact:** JSON serialization is incomplete for create command

### 3. **test_full_bead_lifecycle**
- **File:** tests/cli_integration_crud.rs:928
- **Type:** Panic (Assertion)
- **Message:** `assertion failed: stdout4.contains("status: open")`
- **Root Cause:** Expected "status: open" in output not found after bead lifecycle operations
- **Impact:** Status field may not be displaying correctly in text output

### 4. **test_p0_epic_json_serialization**
- **File:** tests/cli_integration_crud.rs:1196
- **Type:** Panic (Assertion)
- **Message:** `assertion failed: has_field(&epic_json, "id")`
- **Root Cause:** P0 epic JSON output is missing the `id` field
- **Impact:** Same as #2 - JSON serialization incomplete for epic type

### 5. **test_p0_epic_label_filtering**
- **File:** tests/cli_integration_crud.rs:1329
- **Type:** Panic
- **Message:** `should show critical label`
- **Root Cause:** Labels not appearing in filtered list output
- **Impact:** Label filtering in list command is broken

### 6. **test_p0_epic_priority_comparison**
- **File:** tests/cli_integration_crud.rs:1379
- **Type:** Panic
- **Message:** `P0 epic should appear before P1 epic`
- **Root Cause:** Priority sorting is not working correctly
- **Impact:** List command doesn't sort by priority properly

### 7. **test_p0_epic_with_multiple_labels_cli**
- **File:** tests/cli_integration_crud.rs:1254
- **Type:** Panic (Assertion)
- **Message:** `assertion 'left == right' failed: left: 6, right: 5`
- **Root Cause:** Expected 5 labels but found 6 (likely duplicate label issue)
- **Impact:** Label handling has duplication or counting issue

### 8. **test_reopen_open_bead_error**
- **File:** tests/cli_integration_crud.rs:798
- **Type:** Panic
- **Message:** `error should mention bead is already open`
- **Root Cause:** Test expects error when reopening an open bead, but no error occurs
- **Impact:** State validation missing for reopen command

### 9. **test_p0_bead_json_output_format**
- **File:** tests/test_p0_epic_cli.rs:296
- **Type:** Panic (Assertion)
- **Message:** `assertion 'left == right' failed: left: Null, right: "task"`
- **Root Cause:** Type field in JSON is Null instead of expected value "task"
- **Impact:** JSON output missing type field

### 10. **test_p0_bead_label_operations**
- **File:** tests/test_p0_epic_cli.rs:403
- **Type:** Panic
- **Message:** `label add failed: error: unrecognized subcommand 'bf-2jq'`
- **Root Cause:** Label command parsing error - bead ID is being treated as subcommand
- **Impact:** `bf label` command has incorrect argument parsing

### 11. **test_p0_bead_search_by_label**
- **File:** tests/test_p0_epic_cli.rs:539
- **Type:** Panic (Assertion)
- **Message:** `assertion failed: search_output.status.success()`
- **Root Cause:** Search command is failing (non-zero exit)
- **Impact:** Search functionality completely broken

## Categorized Failures

### JSON Output Issues (4 failures)
- test_create_bead_json_output - missing `id` field
- test_p0_epic_json_serialization - missing `id` field
- test_p0_bead_json_output_format - type field is Null
- test_full_bead_lifecycle - status field not in output

### Label Handling Issues (3 failures)
- test_p0_epic_label_filtering - labels not showing in filter
- test_p0_epic_with_multiple_labels_cli - duplicate/count issue
- test_p0_bead_label_operations - command parsing broken

### Command/Validation Issues (4 failures)
- test_create_bead_invalid_type_error - validation not enforced
- test_p0_epic_priority_comparison - sorting broken
- test_reopen_open_bead_error - state validation missing
- test_p0_bead_search_by_label - search completely broken

## Ignored Tests

15 tests were ignored due to **bf-3uk2w5** - pre-existing shared-test-workspace isolation defect. These are order-dependent false failures, not product bugs:
- test_epic_with_labels_cli: 14 tests ignored
- comprehensive_label_cli: 1 test ignored (test_special_characters_in_labels)

## Compiler Warnings

27 warnings in library code, mostly unused imports and variables. These do not affect functionality but should be cleaned up.

## Test Logs Location

- Full output: `/tmp/cli_test_output.log`
- P0 epic tests: `/tmp/p0_epic_test_output.log`
- Other CLI tests: `/tmp/other_cli_tests.log`

## Next Steps

This report documents failures only. No fixes were applied as per the task acceptance criteria ("pure execution and documentation - no fixes yet").

The failures fall into clear categories that can be addressed sequentially:
1. JSON output serialization (missing fields)
2. Label command implementation and parsing
3. Input validation (type checking, state transitions)
4. Search and filtering functionality
5. Sorting/priority comparison
