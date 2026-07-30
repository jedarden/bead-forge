# bf-1il7g: Verify JsonFormatter::format_issues implementation

## Task
Implement JsonFormatter::format_issues for JSONL output.

## Finding
The implementation was already complete in `src/format/json.rs` (lines 52-59).

## Implementation Details
The `format_issues` method:
1. Iterates over each issue in the slice
2. Converts each to JSON using `issue_to_value()` (which strips dependencies/comments and ensures display fields)
3. Collects all JSON strings into a Vec
4. Joins with newlines to produce JSONL output

## Test Results
All 12 tests in `tests/test_json_formatter.rs` pass:
- `test_json_formatter_multiple_issues` - verifies JSONL output (one JSON per line)
- `test_json_formatter_empty_issues` - verifies empty input produces empty output
- `test_json_formatter_strips_dependencies_and_comments` - verifies stripping behavior
- And 9 other tests verifying the complete formatter behavior

## Acceptance Criteria Met
✓ format_issues outputs one JSON object per line (JSONL format)
✓ Empty issue list produces empty string output
✓ dependencies and comments are stripped from output
✓ Each line is valid JSON parseable as Issue
