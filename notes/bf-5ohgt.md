# Bead bf-5ohgt: Multiple-issue JSONL formatting

## Status: Already Implemented

The `format_issues` method was already present in `src/format/json.rs` (lines 52-59).

### Implementation Details

```rust
fn format_issues(&self, issues: &[Issue]) -> String {
    issues
        .iter()
        .map(|issue| serde_json::to_string(&issue_to_value(issue)))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default()
        .join("\n")
}
```

This implementation:
- Iterates over each issue in the slice
- Serializes each issue using `issue_to_value` (which strips dependencies/comments and ensures assignee/labels normalization)
- Collects into Vec<String> with error handling
- Joins with "\n" to produce valid JSONL

### Verification

All acceptance criteria are met:
- ✓ `format_issues` produces valid JSONL (one Issue per line)
- ✓ Empty `issues` vec produces empty output (not '[]')
- ✓ `test_json_formatter_multiple_issues` passes
- ✓ `test_json_formatter_empty_issues` passes

Test run output:
```
running 12 tests
test test_json_formatter_multiple_issues ... ok
test test_json_formatter_empty_issues ... ok
...
test result: ok. 12 passed
```

### Conclusion

No code changes were needed. The implementation was already complete and working correctly.
