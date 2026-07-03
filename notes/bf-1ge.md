# bf-1ge: Verify bf show command implementation

## Task
Implement the bf show command that displays bead details.

## Finding
The `bf show` command is **already fully implemented** in `src/cli/mod.rs` (lines 100-112 for CLI structure, lines 750-753 for command dispatch, and lines 1081-1142 for implementation).

## Verification Results

### Acceptance Criteria - All Met ✅

1. **`bf show <bead-id>` shows all bead fields**
   - Verified with `bf show bf-2cnr`
   - Shows: ID, title, status, priority, type, description, assignee, labels

2. **`bf show --format json` returns structured JSON**
   - Verified JSON output includes comprehensive fields:
   - id, title, description, design, acceptance_criteria, notes, status, priority, issue_type, assignee, created_at, updated_at, source_repo, compaction_level, labels

3. **Validates bead ID exists**
   - Verified with `bf show nonexistent-bead`
   - Returns proper error: "Bead not found: nonexistent-bead"

4. **Reads from SQLite database**
   - Implementation uses `storage.get_issue(id)` from `src/storage/sqlite.rs:151`
   - Falls back to archive search via `find_bead_in_archives()` if not found in database

## Implementation Details

The `cmd_show()` function (lines 1081-1142):
- Loads metadata and opens SQLite database
- Fetches issue by ID (with archive fallback)
- Supports three formats: text (default), json, toon
- JSON format strips dependencies/comments for NEEDLE compatibility
- Returns "Bead not found" error if ID doesn't exist in DB or archives

## Test Results
```bash
$ ./target/debug/bf show bf-2cnr
ID: bf-2cnr
Title: Bug test
Status: in_progress
Priority: P0
Type: bug
Description: Critical bug
Assignee: echo-test-test-worker
Labels: phase-1, urgent

$ ./target/debug/bf show bf-2cnr --format json
[{"id":"bf-2cnr","title":"Bug test",...}]

$ ./target/debug/bf show nonexistent-bead
Error: Bead not found: nonexistent-bead
```

## Conclusion
The bf show command is complete and meets all acceptance criteria. No implementation work was required.
