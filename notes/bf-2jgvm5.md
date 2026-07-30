# JSON Output Test Patterns Review

## Task
Review JSON output test patterns in `src/cli/tests/json_output.rs`

## Test File Location and Structure
**File:** `src/cli/tests/json_output.rs`

The test file is organized into several modules:

### Core Module Structure
1. **Infrastructure** (`test_workspace`, `bf_binary`, `bf_command`) - Test workspace isolation and command building
2. **JSON Validation** (`json_validation` module) - JSON parsing and field assertions
3. **Fixtures** (`fixtures` module) - Test data and bead creation helpers
4. **Format Detection** (`format_detection` module) - Format type detection and validation
5. **Envelope Validation** (`envelope` module) - Envelope wrapping validation
6. **Output Capture** (`capture` module) - Command execution and output capture
7. **Infrastructure Tests** - Unit tests for test helpers
8. **Format Detection Tests** - Unit tests for format detection
9. **Command JSON Output Tests** - Integration tests for CLI commands

## Common Test Patterns

### Pattern 1: Basic JSON Structure Testing
```rust
// Parse and validate JSON structure
let json = json_validation::parse_json(output);
json_validation::assert_required_fields(&json, &["id", "title", "status"], "show command");
```

### Pattern 2: Format Detection and Validation
```rust
// Detect JSON output format (SingleObject, Array, JsonL, Empty, EmptyArray)
format_detection::assert_format(output, format_detection::JsonFormat::JsonL);

// Validate JSONL (common for list/ready/search commands)
format_detection::is_valid_jsonl(output);
```

### Pattern 3: Envelope Validation (for commands that wrap output)
```rust
// Validate envelope structure: {version: 1, kind: "<command>", data: {...}}
let envelope = envelope::validate_envelope(output, "create");
let data = envelope::get_envelope_data(&envelope);
if envelope::has_warning(&envelope) {
    let warning = envelope::get_warning(&envelope);
}
```

### Pattern 4: Using Test Fixtures
```rust
// Create test beads with various properties
let bead_id = fixtures::create_bead("Test bead");
let bead_id = fixtures::create_bead_with_labels("Feature", &["enhancement", "ui"]);
let bead_id = fixtures::create_bead_with_assignee("Bug", "alice");

// Use pre-defined special character test data
let bead_id = fixtures::create_bead(fixtures::SPECIAL_CHARACTERS_TITLE);
```

### Pattern 5: Command Execution and Output Capture
```rust
// Capture stdout from a command
let output = capture::capture_stdout(
    bf_command().arg("show").arg(bead_id).arg("--format").arg("json")
);

// Capture both stdout and stderr
let (stdout, stderr) = capture::capture_both(
    bf_command().arg("list").arg("--format").arg("json")
);

// Capture output even when command fails
let (stdout, stderr, success) = capture::capture_failed_command(
    bf_command().arg("show").arg(fake_id).arg("--format").arg("json")
);
```

### Pattern 6: Special Characters and Edge Cases
Always test with special characters to ensure proper JSON escaping:
- Quotes and apostrophes: `fixtures::SPECIAL_CHARACTERS_TITLE`
- Unicode/emoji: `fixtures::UNICODE_TITLE`
- Long titles: `fixtures::LONG_TITLE`
- JSON-like content: `fixtures::JSON_LIKE_TITLE`

## Command-specific JSON Output Formats

| Command | Format | Description |
|---------|--------|-------------|
| `show` | `[{...}]` | Single bead wrapped in array |
| `list` | JSONL | Multiple beads, newline-delimited |
| `search` | JSONL | Search results, newline-delimited |
| `ready` | JSONL | Unblocked beads, newline-delimited |
| `recent` | Envelope | Recent beads with envelope wrapping |
| `claim` | Object | Single object with bead_id field |
| `create` | String | Bead ID only (plain text) |

## Assertion Patterns

### Success Case Assertions
```rust
// Verify JSON is valid
json_validation::assert_valid_json(json_str);

// Verify required fields exist
json_validation::assert_required_fields(&json, &["id", "title", "status"], "context");

// Verify specific field values
assert_eq!(json_validation::get_string(&json, "id"), bead_id);

// Verify format type
format_detection::assert_format(output, format_detection::JsonFormat::JsonL);

// Verify envelope structure
let envelope = envelope::validate_envelope(output, "command_name");
```

### Error Case Assertions
```rust
// Capture failed command output
let (stdout, stderr, success) = capture::capture_failed_command(cmd);

// Assert command failed
assert!(!success, "Command should fail");

// Verify error message in stderr
assert!(stderr.contains("not found"), "stderr should mention error");

// Verify stdout is empty (no partial JSON output)
assert!(stdout.trim().is_empty(), "stdout should be empty for errors");
```

## Test Helpers and Fixtures

### JSON Validation Helpers
- `parse_json()` - Parse JSON string with panic on error
- `try_parse_json()` - Parse JSON returning Result
- `parse_jsonl()` - Parse JSONL into Vec<Value>
- `assert_valid_json()` - Assert JSON is valid
- `assert_valid_jsonl()` - Assert each line is valid JSON
- `get_string()`, `get_int()`, `get_bool()` - Typed field getters
- `get_array()`, `get_object()` - Complex field getters
- `get_string_optional()`, `get_int_optional()` - Optional field getters
- `assert_required_fields()` - Assert multiple fields exist

### Fixture Data Constants
- `SPECIAL_CHARACTERS_TITLE` - Quotes, apostrophes, symbols, backslashes
- `UNICODE_TITLE` - Unicode and emoji characters
- `WHITESPACE_TITLE` - Newlines and tabs
- `LONG_TITLE` - Very long title
- `JSON_LIKE_TITLE` - JSON-like content in text
- `EMPTY_TITLE` - Empty title edge case
- `SPECIAL_LABELS` - Labels with special characters
- `SPECIAL_ASSIGNEE` - Assignee with special characters

### Fixture Creation Helpers
- `create_bead()` - Create basic test bead
- `create_bead_with_labels()` - Create bead with labels
- `create_bead_with_assignee()` - Create bead with assignee
- `close_bead()` - Close a test bead
- `add_dependency()` - Add dependency between beads
- `claim_bead()` - Claim a bead for testing

## Key Learnings

1. **Display Normalization**: All commands ensure `assignee` and `labels` fields are always present (even if null/empty)
2. **NEEDLE Compatibility**: Some fields like `dependencies` and `comments` are stripped from JSON output
3. **Format Variety**: Different commands use different formats (array, JSONL, envelope, plain text)
4. **Special Characters**: Comprehensive special character testing is built into fixtures
5. **Workspace Isolation**: Tests use a shared workspace with proper database initialization
6. **Binary Resolution**: Tests handle both cargo test environment and manual runs
7. **Error Handling**: Separate pattern for testing failed commands with `capture_failed_command()`

## Verification Summary
- ✅ Identified 6+ common test patterns
- ✅ Documented assertion structures for both success and error cases
- ✅ Noted extensive test helpers and fixtures
- ✅ Confirmed test file location and modular structure
