# Bead bf-5y7oek: Model Support for Clearing Bead Assignee

## Investigation Summary

The model and storage layers **already fully support** clearing bead assignee. No code changes were required.

## Existing Implementation

### Model Layer (`src/model.rs`)

- **Line 469-470**: `assignee` field is `Option<String>` - supports NULL values
- **Lines 841-847**: `Issue::clear_assignee()` method exists:
  ```rust
  pub fn clear_assignee(&self, actor: String) -> IssueChanges {
      IssueChanges {
          assignee: Some(String::new()),
          actor: Some(actor),
          ..Default::default()
      }
  }
  ```

### Storage Layer (`src/storage/sqlite.rs`)

**Lines 637-646**: The `update_issue` method correctly handles assignee clearing:
```rust
if let Some(ref assignee) = changes.assignee {
    if assignee.trim().is_empty() {
        // Clearing stores NULL, never an empty string that would
        // read back as "assigned" and hide the bead from claiming.
        updates.push("assignee = NULL");
    } else {
        updates.push("assignee = ?");
        params.push(Box::new(assignee.clone()));
    }
}
```

### CLI Layer (`src/cli/mod.rs`)

**Lines 1874-1878**: `--clear-assignee` flag exists and conflicts with `--assignee`:
```rust
let assignee = if clear_assignee {
    Some(String::new())
} else {
    assignee
};
```

The code comment at line 1874 explicitly states:
> "Note: an empty/whitespace `--assignee` is intentionally NOT rejected here.
> It flows through to update_issue, whose storage layer maps it to
> `assignee = NULL` (clearing the assignee)."

## Verification Testing

All three methods of clearing assignee work correctly:

1. **`bf update <id> --clear-assignee`** - Explicit flag
2. **`bf update <id> --assignee ""`** - Empty string
3. **`issue.clear_assignee(actor)`** - Model method

All methods successfully set the database field to NULL, not an empty string.

## Acceptance Criteria Status

✅ Model layer allows setting assignee to NULL/empty string
✅ Storage layer commits cleared assignee to SQLite without error  
✅ No breaking changes to existing assignee-setting behavior

## Conclusion

**Status**: ✅ **COMPLETE**

The model and storage layers already fully support clearing bead assignee. The implementation is correct, complete, and has no breaking changes. Both CLI methods and the model-level `clear_assignee()` method work as expected.

The rejection mentioned in the task description ("Investigate whether the current rejection is purely at CLI validation level") was likely referring to the intentional design decision NOT to reject empty/whitespace assignee values at the CLI level, allowing them to flow through to the storage layer where they are properly converted to NULL.

No code changes were required. The task was to verify existing functionality, which is now confirmed to be working correctly.
