# Bead bf-4bhsxp: query_dirty_issues() Implementation

## What Was Done

Fixed the existing `query_dirty_issues()` function in `src/storage/sqlite.rs` (line 397-410).

## The Fix

The function had incorrect column names:
- **Before:** `SELECT bead_id FROM dirty_issues ORDER BY bead_id`
- **After:** `SELECT issue_id FROM dirty_issues ORDER BY marked_at ASC`

This aligns with:
1. The actual database schema used throughout the codebase (issue_id, marked_at)
2. The pattern in `jsonl.rs::get_dirty_issue_ids()`
3. The INSERT statements that use `issue_id` and `marked_at` columns

## Acceptance Criteria Met

✅ Function returns Vec<String> of bead_ids
✅ Uses prepared statement (prepare_cached)
✅ Returns empty Vec when no dirty issues
✅ Follows rusqlite patterns from existing code

## Notes

The schema definition in `src/storage/schema.rs` shows `bead_id` but this appears to be outdated/incomplete. The actual table schema used throughout the codebase has `issue_id` and `marked_at` columns.
