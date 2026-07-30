# Bead bf-1r8rb: Assignee and Labels Normalization

## Status: VERIFIED - Implementation Already Complete

### Summary
This bead verifies that assignee and labels normalization in JSON output is working correctly.

### Implementation Location
The normalization is implemented in `src/format/json.rs`:

1. **`issue_to_value()` function** (lines 27-37):
   - Clones the issue
   - Strips dependencies/comments for br compatibility  
   - Calls `ensure_display_fields()` to normalize output

2. **`ensure_display_fields()` function** (lines 40-45):
   - Ensures `assignee` field is always present (`null` when None, string value when set)
   - Ensures `labels` field is always present (empty array `[]` when no labels, array when labels exist)

### Acceptance Criteria Met
✅ assignee field always present (null when None, string value when set)
✅ labels field always present as empty array when no labels
✅ labels field present as array when labels exist
✅ Display normalization ensures br compatibility (bf-1wj)

### Test Coverage
The test `test_json_formatter_assignee_and_labels_normalization` in `tests/test_json_formatter.rs` verifies:
- Issues with no assignee/labels output `assignee: null, labels: []`
- Issues with assignee/labels output the correct values
- Both `format_issue()` and `format_issues()` paths are covered

### Historical Note
This functionality was originally implemented in bead bf-1wj (commit e8ed49d: "fix(bf-1wj): always emit assignee/labels in ready/list/search --format json"). This bead serves as verification that the implementation is correct and complete.
