# bf-1cy: Implement bf update command

## Finding

The `bf update` command was **already fully implemented** in the existing codebase.

## Implementation Location

- CLI definition: `src/cli/mod.rs` lines 114-154 (Update command struct)
- Command handler: `cmd_update()` function at lines 1144-1188
- Storage layer: `storage.update_issue()` at `src/storage/sqlite.rs` lines 381-540
- Data model: `IssueChanges` struct at `src/model.rs` line 843

## Acceptance Criteria Verification

All acceptance criteria are met by the existing implementation:

1. ✅ `bf update <bead-id> --status <status>` updates status - VERIFIED by manual test
2. ✅ `bf update --title <title>` updates title - VERIFIED by manual test
3. ✅ `bf update --description <desc>` updates description - VERIFIED by manual test
4. ✅ Validates bead ID exists - checks existence before updating (sqlite.rs:383-396)
5. ✅ Writes to SQLite database with transaction - uses `with_immediate_transaction` (sqlite.rs:432)

## Supported Update Fields

The update command supports all these fields:
- `--title` - Bead title
- `--status` - Status (open, in_progress, blocked, deferred, draft, closed, tombstone, pinned)
- `--priority` - Priority level (0-4)
- `--assignee` - Assignee name
- `--description` - Description text
- `--acceptance-criteria` - Acceptance criteria
- `--notes` - Notes
- `--design` - Design documentation
- `--due-at` - Due date (RFC3339 format)

## Implementation Details

The `update_issue()` method:
1. Validates bead existence before updating
2. Scans for secrets in string fields (if enabled)
3. Builds dynamic UPDATE query based on provided fields
4. Handles labels and annotations as separate operations
5. Marks bead as dirty for JSONL export
6. Invalidates critical path cache on status changes
7. All wrapped in `BEGIN IMMEDIATE` transaction for atomicity

## Manual Testing Results

All tests passed successfully:
```bash
# Status update
bf update bf-485m --status in_progress  # ✓ Success
bf show bf-485m | grep Status            # ✓ Shows: Status: in_progress

# Title update
bf update bf-485m --title "New title"    # ✓ Success
bf show bf-485m | grep Title             # ✓ Shows updated title

# Description update
bf update bf-485m --description "New"    # ✓ Success

# Multiple field update
bf update bf-485m --title "X" --description "Y" --priority 0  # ✓ Success
```

## Work Completed

- Verified existing implementation meets all acceptance criteria
- Confirmed transaction safety with `with_immediate_transaction`
- Validated bead existence checking works correctly
- Manually tested all update operations
- Confirmed error handling for non-existent beads
