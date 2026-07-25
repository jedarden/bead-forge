# JSON Output Tests for Search Command

## Summary
Added comprehensive JSON output tests for the `bf search` command and fixed a bug in the search filter logic.

## Changes Made

### 1. Bug Fix (src/storage/sqlite.rs)
Fixed a bug in the `search_issues` function where multiple status and type filters were being AND-ed together instead of OR-ed together. The CLI documentation states "Multiple values for --status, --type, and --label are OR-combined", but the implementation was incorrectly using AND logic.

**Before:** `AND i.status = ?1 AND i.status = ?2` (can never match)
**After:** `AND (i.status = ?1 OR i.status = ?2)` (correct OR logic)

### 2. New Test File (tests/test_search_json_filters.rs)
Added comprehensive JSON output tests covering:
- Multiple status filters (OR-combined)
- Multiple type filters (OR-combined) 
- Multiple label filters (OR-combined)
- Priority range filters (--priority-min, --priority-max)
- Combined filters (multiple filter types together)
- Text query combined with filters
- Limit parameter with JSON output
- Assignee filter
- Edge cases (no matches, wildcards, etc.)

## Test Results
All 10 new tests pass:
- test_search_json_assignee_filter
- test_search_json_combined_filters
- test_search_json_multiple_label_filters
- test_search_json_limit_parameter
- test_search_json_multiple_type_filters
- test_search_json_no_matches_with_filters
- test_search_json_multiple_status_filters
- test_search_json_text_query_with_filters
- test_search_json_priority_range_filters
- test_search_json_wildcard_text_with_filters

All existing search-related tests also continue to pass.

## Acceptance Criteria Met
✅ Add tests for search command --json output (JSONL array format)
✅ Validate JSON structure validity
✅ Test required fields are present in output
✅ Test search with various query patterns
✅ Test empty search results
✅ cargo test passes for new tests
