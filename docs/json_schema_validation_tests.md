# JSON Schema Validation Tests

## Overview

Comprehensive JSON schema validation tests for bead-forge CLI commands, located in `src/cli/tests/json_schema_validation.rs`. These tests ensure that JSON output maintains proper schema even on errors, empty results maintain correct schema, all JSON fields are present and properly typed, and JSON output validates against expected schema structure.

## Test Results

✅ **All 23 JSON schema validation tests passing**

```bash
cargo test --lib json_schema_validation
```

## Test Coverage

### 1. Schema Consistency Across Error Cases ✅

Tests that verify JSON output maintains consistent schema structure even when commands fail:

- `test_show_json_schema_consistency_on_invalid_bead_id` - Invalid/non-existent bead IDs
- `test_update_json_schema_consistency_on_errors` - Update operations on invalid beads
- `test_command_json_schema_consistency_various_errors` - Various command error scenarios
- `test_error_responses_consistent_schema` - Consistent error response structure

**Coverage:**
- Invalid bead IDs (malformed, non-existent)
- Missing required arguments
- Dependency errors on non-existent beads
- Label operations on non-existent beads
- Comment operations on non-existent beads

### 2. Empty Results Schema Validation ✅

Tests that verify empty result sets maintain correct JSON schema:

- `test_list_json_empty_results_maintains_schema` - Empty list command output
- `test_search_json_empty_results_maintains_schema` - Empty search results
- `test_ready_json_empty_results_maintains_schema` - Empty ready bead results
- `test_show_json_empty_workspace` - Show command in empty workspace
- `test_empty_results_with_filters_maintain_schema` - Empty results with various filters

**Coverage:**
- Empty database state
- Filtered queries returning no results (status, type, assignee, priority filters)
- Proper handling of empty arrays vs empty strings

### 3. Field Presence and Type Validation ✅

Tests that verify all JSON fields are present and properly typed:

- `test_show_json_all_required_fields_present` - All required fields in show output
- `test_list_json_all_items_conform_to_schema` - All list items conform to schema
- `test_search_json_results_conform_to_schema` - Search results schema compliance
- `test_ready_json_results_conform_to_schema` - Ready results schema compliance
- `test_claim_json_schema_structure` - Claim command output structure

**Coverage:**
- Required fields: `id`, `title`, `status`, `priority`, `issue_type`, `created_at`, `updated_at`, `assignee`, `labels`
- Optional fields: `description`, `design`, `acceptance_criteria`, `notes`, `due_at`, `closed_at`
- Field type validation: string, number, array, null
- Display normalization (assignee and labels always present)

### 4. Schema Structure Validation ✅

Tests that verify JSON output structure matches expected format:

- `test_show_json_structure_matches_expected` - Show returns single-element array `[{...}]`
- `test_list_json_structure_matches_expected` - List returns JSONL (newline-delimited objects)
- `test_search_json_structure_matches_expected` - Search returns JSONL
- `test_create_json_envelope_schema` - Create command envelope structure

**Coverage:**
- Command-specific output formats:
  - `show`: `[{issue}]` (single-element array)
  - `list`: JSONL (newline-delimited objects)
  - `search`: JSONL
  - `ready`: JSONL or `[]` for empty
  - `claim`: Single object `{bead_id, assignee, reclaimed}`
  - `create`: Envelope `{version, kind, data, warning?}`

### 5. Edge Cases and Special Characters ✅

Tests that verify schema is maintained with edge cases:

- `test_schema_maintained_with_special_characters` - Quotes, apostrophes, symbols, backslashes
- `test_schema_maintained_with_unicode` - Unicode characters and emoji (café, 日本語, 🎉)
- `test_schema_with_very_long_values` - 10KB+ description fields
- `test_schema_with_minimal_fields` - Beads with only required fields

**Coverage:**
- Special character escaping in JSON
- Unicode preservation (emoji, international characters)
- Long field values
- Minimal vs maximal field populations

### 6. Cross-Command Consistency ✅

Tests that verify schema consistency across different commands:

- `test_same_bead_consistent_schema_across_commands` - Same bead has consistent schema in show vs list

**Coverage:**
- Field consistency between `show` and `list` commands
- Core fields remain identical across commands
- Schema compliance maintained across all output formats

## Schema Definition

### Issue Object Schema

All issue objects must conform to this schema:

```json
{
  "id": "string",
  "title": "string",
  "status": "string",
  "priority": "number",
  "issue_type": "string",
  "assignee": "string | null",
  "labels": ["string"],
  "created_at": "string (ISO 8601)",
  "updated_at": "string (ISO 8601)",
  "description": "string | null (optional)",
  "design": "string | null (optional)",
  "acceptance_criteria": "string | null (optional)",
  "notes": "string | null (optional)",
  "due_at": "string | null (optional)",
  "closed_at": "string | null (optional)"
}
```

### Required Fields

The following fields are **always present** in JSON output:
- `id`, `title`, `status`, `priority`, `issue_type`
- `created_at`, `updated_at`
- `assignee` (null if unassigned - display normalization)
- `labels` (empty array if none - display normalization)

### Command-Specific Output Formats

| Command | Format | Description |
|---------|--------|-------------|
| `show` | `[{...}]` | Single bead wrapped in array |
| `list` | JSONL | Multiple beads, newline-delimited |
| `search` | JSONL | Search results, newline-delimited |
| `ready` | JSONL or `[]` | Unblocked beads, `[]` if empty |
| `claim` | Object | `{bead_id, assignee, reclaimed, workspace?, dry_run?}` |
| `create` | Envelope | `{version: 1, kind: "create", data: {id}, warning?}` |

## Test Infrastructure

The tests use a comprehensive test infrastructure defined in `src/cli/tests/json_output.rs`:

### Validation Functions

- `json_validation::parse_json()` - Parse JSON string with panic on error
- `json_validation::assert_valid_json()` - Assert JSON is valid
- `json_validation::assert_required_fields()` - Check required fields exist
- `validate_issue_schema()` - Validate issue object against schema
- `validate_field_types()` - Validate field types in issue object

### Test Fixtures

- `fixtures::create_bead()` - Create test bead
- `fixtures::create_bead_with_labels()` - Create bead with labels
- `fixtures::create_bead_with_assignee()` - Create bead with assignee
- `fixtures::close_bead()` - Close test bead for cleanup

### Capture Functions

- `capture::capture_stdout()` - Capture command stdout
- `capture::capture_failed_command()` - Capture output from failed command
- `capture::capture_both()` - Capture both stdout and stderr

### Special Character Constants

- `SPECIAL_CHARACTERS_TITLE` - Title with quotes, apostrophes, symbols
- `UNICODE_TITLE` - Title with unicode and emoji
- `WHITESPACE_TITLE` - Title with newlines and tabs
- `LONG_TITLE` - Very long title for testing limits
- `JSON_LIKE_TITLE` - Title with JSON-like content

## Running the Tests

```bash
# Run all JSON schema validation tests
cargo test --lib json_schema_validation

# Run specific test
cargo test --lib test_show_json_schema_consistency_on_invalid_bead_id

# Run with output
cargo test --lib json_schema_validation -- --nocapture

# Run tests in sequence (for debugging)
cargo test --lib json_schema_validation -- --test-threads=1
```

## Acceptance Criteria Status

✅ **All acceptance criteria met:**

1. ✅ Test JSON output schema is consistent across all error cases
2. ✅ Test JSON output for empty results maintains correct schema  
3. ✅ Test that all JSON fields are present and properly typed
4. ✅ Test JSON output validates against expected schema structure
5. ✅ Tests located in `src/cli/tests/`

## Related Tests

Additional JSON-related test modules:
- `src/cli/tests/show_json_tests.rs` - Show command JSON output tests
- `src/cli/tests/list_ready_recent_json_tests.rs` - List/ready/recent JSON tests
- `src/cli/tests/search_json_tests.rs` - Search command JSON tests
- `src/cli/tests/edge_case_json_tests.rs` - Edge case JSON tests
- `src/cli/tests/error_json_tests.rs` - Error case JSON tests
- `src/cli/tests/json_output.rs` - Test infrastructure and helpers

## Conclusion

The JSON schema validation tests provide comprehensive coverage of bead-forge's JSON output, ensuring schema consistency across all commands, error cases, and edge cases. All 23 tests pass successfully, validating that the implementation maintains proper JSON schema as specified.
