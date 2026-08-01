# bf-1sx1: Implement bf reopen command

## Summary

The `bf reopen` command was already fully implemented in the codebase. All acceptance criteria are met and comprehensive tests exist and pass.

## Implementation Location

- **CLI definition**: `src/cli/mod.rs` lines 232-239 (Reopen command variant)
- **Handler**: `src/cli/mod.rs` lines 1857-1881 (`cmd_reopen` function)
- **Storage layer**: `src/storage/sqlite.rs` `update_issue` method handles all state transitions

## Key Implementation Details

### Command Handler (cmd_reopen)
```rust
fn cmd_reopen(beads_dir: &PathBuf, id: &str, no_auto_flush: bool) -> Result<()> {
    let config = load_config(beads_dir)?;
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    let changes = IssueChanges {
        status: Some(Status::Open),
        assignee: Some(String::new()),  // Clear stale assignee
        ..Default::default()
    };

    storage.update_issue(id, &changes)?;
    autoflush_after_mutation(beads_dir, &config, no_auto_flush);
    println!("Reopened bead {}", id);

    Ok(())
}
```

### Automatic Field Clearing
The `update_issue` method in `src/storage/sqlite.rs` automatically clears closed-related fields when transitioning from terminal to non-terminal status (lines 545-553):

```rust
if !matches!(status, Status::Closed | Status::Tombstone) {
    updates.push("closed_at = NULL");
    updates.push("close_reason = NULL");
    updates.push("closed_by_session = NULL");
}
```

## Acceptance Criteria Verification

All 8 acceptance criteria are met:

1. ✅ Command signature matches `br reopen <id>`
2. ✅ Transitions bead status from 'closed' to 'open'
3. ✅ Clears closed_at timestamp
4. ✅ Clears close_reason field
5. ✅ Marks bead as dirty in SQLite
6. ✅ Uses with_immediate_transaction for atomicity
7. ✅ Returns error if bead not found
8. ✅ Clears stale assignee (critical for workflow)

## Additional Features

### Event Recording
- Creates 'reopened' event when transitioning from closed to open
- Records old_value (closed) and new_value (open) for audit trail

### Idempotent Operation
- Can safely reopen an already-open bead (no-op behavior)
- Prevents errors in automated workflows

### Auto-flush Integration
- Respects --no-auto-flush flag
- Marks dirty for JSONL export by default
- Tested in autoflush_comprehensive_mutations.rs

## Test Coverage

Comprehensive test coverage exists:

### Unit Tests (tests/close_reopen.rs)
- test_reopen_bead_changes_status_to_open
- test_close_then_reopen_creates_reopened_event
- test_multiple_close_reopen_cycles
- test_reopen_clears_assignee
- test_reopen_with_no_assignee_is_noop

### Integration Tests (tests/test_close_reopen.rs)
- test_close_and_reopen_bead
- test_reopen_in_progress_bead
- test_reopen_nonexistent_bead

### Autoflush Tests (tests/autoflush_comprehensive_mutations.rs)
- reopen_autoflush_resets_status
- reopen_no_autoflush_doesnt_modify_jsonl

All tests pass successfully.

## Workflow Integration

The reopen command is critical for needle workflows:

1. **Worker claiming**: Beads must be open to be claimed
2. **Stale assignee clearing**: Reopen clears foreign assignees that would block claiming
3. **Iteration support**: Allows re-attempting completed work with fresh context

The implementation ensures that reopened beads are immediately claimable by clearing both the status (open) and the assignee (NULL), matching br's behavior.
