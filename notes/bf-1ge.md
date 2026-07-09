# bf-1ge: Implement bf show command

## Implementation Status: ✅ COMPLETE

The `bf show` command was already fully implemented in `src/cli/mod.rs` (lines 1081-1142).

## Acceptance Criteria Verification

All acceptance criteria have been met:

1. ✅ **bf show <bead-id> shows all bead fields**
   - Text format displays: ID, title, status, priority, type, description, assignee, labels
   - All populated fields are shown in a clean, readable format

2. ✅ **bf show --format json returns structured JSON**
   - Returns JSON array with complete bead data
   - Includes all fields: id, title, status, priority, issue_type, description, design, acceptance_criteria, notes, created_at, updated_at, labels, etc.
   - NEEDLE-compatible: strips dependencies and comments from JSON output

3. ✅ **Validates bead ID exists**
   - Returns clear error message: "Bead not found: {id}"
   - Exit code 1 for non-existent beads
   - Falls back to searching archive files if not in SQLite

4. ✅ **Reads from SQLite database**
   - Uses `storage.get_issue()` to retrieve from SQLite
   - Loads all related data: labels, dependencies, comments, annotations

## Testing Results

```bash
# Text format (default)
$ ./target/debug/bf show bf-2cnr
ID: bf-2cnr
Title: Bug test
Status: open
Priority: P0
Type: bug
Description: Critical bug
Labels: phase-1, urgent

# JSON format
$ ./target/debug/bf show bf-2cnr --format json
[{"id":"bf-2cnr","title":"Bug test",...}]

# Error handling
$ ./target/debug/bf show nonexistent-bead-123
Error: Bead not found: nonexistent-bead-123
```

## Command Implementation

Location: `src/cli/mod.rs:1081-1142`

Key features:
- Supports three output formats: text (default), json, toon
- Falls back to archive files if bead not in database
- NEEDLE compatibility for JSON output (wraps in array, strips deps/comments)
- Clean, readable text output with field labels

## Related Commands

- `bf list` - Lists beads with filtering options
- `bf update` - Updates bead fields
- `bf close` - Closes beads
