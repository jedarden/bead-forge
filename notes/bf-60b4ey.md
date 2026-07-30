# JSON Test Helper Infrastructure - Bead bf-60b4ey

## Status: COMPLETE ✅

The JSON test helper infrastructure requested in this bead has been fully implemented in `src/cli/tests/json_output.rs`. The infrastructure is comprehensive and production-ready.

## Acceptance Criteria Verification

### ✅ 1. Create shared JSON validation helper functions
**Location:** `src/cli/tests/json_output.rs` - `json_validation` module

**Functions provided:**
- `parse_json(json: &str) -> Value` - Parse JSON with panic on error
- `try_parse_json(json: &str) -> Result<Value, Error>` - Parse JSON with Result
- `parse_jsonl(jsonl: &str) -> Vec<Value>` - Parse JSONL into values
- `assert_valid_json(json: &str)` - Assert JSON is valid
- `assert_valid_jsonl(jsonl: &str)` - Assert JSONL is valid
- `has_field(json: &Value, field: &str) -> bool` - Check if field exists
- `get_string(json: &Value, field: &str) -> String` - Get string field
- `get_int(json: &Value, field: &str) -> i64` - Get integer field
- `get_bool(json: &Value, field: &str) -> bool` - Get boolean field
- `get_array(json: &Value, field: &str) -> Vec<Value>` - Get array field
- `get_object(json: &Value, field: &str) -> Value` - Get object field
- `get_string_optional(json: &Value, field: &str) -> Option<String>` - Get optional string
- `get_int_optional(json: &Value, field: &str) -> Option<i64>` - Get optional int
- `assert_json_eq(actual: &Value, expected: &Value)` - Compare JSON with good errors
- `assert_required_fields(json: &Value, fields: &[&str], context: &str)` - Check required fields

### ✅ 2. Implement helper to validate JSON structure (JSONL vs array format)
**Location:** `src/cli/tests/json_output.rs` - `format_detection` module

**Functions provided:**
- `detect_format(output: &str) -> JsonFormat` - Detect output format
- `assert_format(output: &str, expected: JsonFormat)` - Assert format matches
- `is_valid_jsonl(output: &str) -> bool` - Check if valid JSONL
- `is_valid_json_object(output: &str) -> bool` - Check if valid JSON object
- `is_valid_json_array(output: &str) -> bool` - Check if valid JSON array

**Format types supported:**
- `JsonFormat::SingleObject` - Single JSON object
- `JsonFormat::Array` - JSON array
- `JsonFormat::JsonL` - JSONL (newline-delimited JSON)
- `JsonFormat::EmptyArray` - Empty array `[]`
- `JsonFormat::Empty` - Empty string

### ✅ 3. Implement helper to check required fields are present
**Location:** `src/cli/tests/json_output.rs` - `json_validation::assert_required_fields`

**Function signature:**
```rust
pub fn assert_required_fields(json: &Value, fields: &[&str], context: &str)
```

**Usage example:**
```rust
json_validation::assert_required_fields(
    &json,
    &["id", "title", "status", "priority"],
    "show command"
);
```

### ✅ 4. Implement helper to test empty result sets
**Location:** `src/cli/tests/json_output.rs` - `format_detection` module

**Support for empty results:**
- `JsonFormat::EmptyArray` - Detects `[]` (used by empty list/ready)
- `JsonFormat::Empty` - Detects empty string (used by empty search)
- `is_valid_jsonl()` - Returns `true` for empty strings and `[]`
- `is_valid_json_array()` - Returns `true` for `[]`

**Test examples included:**
- `test_list_command_json_empty_results` - Tests empty list returns `[]`
- `test_search_command_json_empty_results` - Tests empty search returns empty string
- `test_ready_command_json_empty_results` - Tests empty ready returns `[]`

### ✅ 5. Add module to src/cli/tests/mod.rs
**Location:** `src/cli/tests/mod.rs`

**Module declarations:**
```rust
pub mod json_output;
pub mod show_json_tests;
pub mod list_ready_recent_json_tests;
```

**Convenience re-exports:**
```rust
pub use json_output::{
    test_workspace,
    bf_binary,
    bf_command,
    json_validation,
    format_detection,
    fixtures,
    envelope,
    capture,
};
```

### ✅ 6. Infrastructure is reusable across list/ready/recent/show tests
**Current usage:**
- `show_json_tests.rs` - Uses all helper modules for show command tests
- `list_ready_recent_json_tests.rs` - Uses all helper modules for list/ready/recent tests
- `json_output.rs` - Contains comprehensive integration tests using all helpers

**Helper modules available:**
- `json_validation` - JSON parsing and field access
- `format_detection` - Format detection and validation
- `fixtures` - Test bead creation (create_bead, create_bead_with_labels, etc.)
- `envelope` - Envelope wrapping validation
- `capture` - Command output capture
- `test_workspace` - Test workspace management
- `bf_binary` - Binary path resolution
- `bf_command` - Command builder

## Test Coverage

The infrastructure includes comprehensive test coverage:
- Infrastructure self-tests (test_json_validation_helpers, test_jsonl_validation, etc.)
- Format detection tests (test_detect_single_object, test_detect_jsonl, etc.)
- Command integration tests (show, list, search, ready, recent)
- Special character handling tests
- Empty result handling tests
- Envelope validation tests

## Command Format Reference

| Command | Format | Description |
|---------|--------|-------------|
| `show` | `[{...}]` | Single bead wrapped in array |
| `list` | JSONL | Multiple beads, newline-delimited |
| `search` | JSONL | Search results, newline-delimited |
| `ready` | JSONL or `[]` | Unblocked beads or empty array |
| `recent` | Envelope | Recent beads with envelope wrapping |
| `claim` | Object | Single object with bead_id field |

## Usage Examples

### Basic JSON validation
```rust
let json = json_validation::parse_json(output);
json_validation::assert_required_fields(&json, &["id", "title", "status"], "show command");
```

### Format detection
```rust
format_detection::assert_format(output, format_detection::JsonFormat::JsonL);
assert!(format_detection::is_valid_jsonl(output));
```

### Creating test beads
```rust
let bead_id = fixtures::create_bead("Test bead");
let bead_id = fixtures::create_bead_with_labels("Feature", &["enhancement", "ui"]);
let bead_id = fixtures::create_bead_with_assignee("Bug", "alice");
```

### Command execution
```rust
let output = capture::capture_stdout(
    bf_command().arg("show").arg(bead_id).arg("--format").arg("json")
);
```

## Files Modified

This bead verified existing infrastructure. The only minor changes were:
- `src/cli/tests/json_output.rs` - Removed `#[ignore]` from `test_ready_command_json_limit`
- `src/cli/tests/mod.rs` - Added `pub mod list_ready_recent_json_tests;`

These are documentation/cleanup changes, not new infrastructure.

## Conclusion

The JSON test helper infrastructure is **complete and production-ready**. All acceptance criteria have been met. The infrastructure is comprehensive, well-documented, and actively used across multiple test files.
