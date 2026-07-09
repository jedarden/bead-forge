# Bead bf-2grah: Storage Layer close_issue Method

## Implementation Status: COMPLETE

The `Storage::close_issue()` method is fully implemented in `src/storage/sqlite.rs` (lines 660-709).

## Acceptance Criteria Verification

All acceptance criteria are met:

1. ✅ **Method signature**: `pub fn close_issue(&self, id: &str, reason: &str, actor: &str) -> Result<()>`
2. ✅ **Updates status to 'closed'**: Line 691 sets `status = 'closed'`
3. ✅ **Sets closed_at**: Line 691 sets `closed_at = ?` with `now.to_rfc3339()`
4. ✅ **Sets close_reason**: Line 691 sets `close_reason = ?` with the provided reason
5. ✅ **Sets closed_by_session**: Line 691 sets `closed_by_session = ?` with the actor
6. ✅ **Records Closed event**: Lines 694-697 insert event with type 'closed'
7. ✅ **Returns error if not found**: Lines 663-672 check existence and return error
8. ✅ **Idempotent**: Lines 674-686 return early with `Ok(())` if already closed
9. ✅ **Uses BEGIN IMMEDIATE**: Line 661 uses `self.with_immediate_transaction`

## Additional Features

The implementation also includes:
- Marks issue as dirty for export (dirty_issues table)
- Updates worker session with close time for velocity tracking
- Invalidates critical path cache (closing can unblock dependents)
- Recomputes critical paths after closure

## Implementation Details

```rust
pub fn close_issue(&self, id: &str, reason: &str, actor: &str) -> Result<()> {
    self.with_immediate_transaction(|tx| {
        // Check if bead exists
        let exists: bool = tx
            .query_row(
                "SELECT 1 FROM issues WHERE id = ?1",
                params![id],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if !exists {
            return Err(anyhow!("Bead not found: {}", id));
        }

        // Check if already closed for idempotence
        let current_status: Option<String> = tx
            .query_row(
                "SELECT status FROM issues WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .ok();

        if current_status.as_deref() == Some("closed") {
            // Already closed - idempotent, return success
            return Ok(());
        }

        // Close the bead
        let now = Utc::now();
        tx.execute(
            "UPDATE issues SET status = 'closed', closed_at = ?, close_reason = ?, closed_by_session = ?, updated_at = ? WHERE id = ?",
            params![now.to_rfc3339(), reason, actor, now.to_rfc3339(), id],
        )?;
        // ... event recording, dirty marking, cache invalidation ...
        Ok(())
    })
}
```
