# bf-3rjyxo: remove_label implementation verification

Verified that the `remove_label` method is correctly implemented in `src/storage/sqlite.rs` (lines 1462-1480).

## Implementation details

The method follows the exact pattern of `add_label`:

1. **Trimming**: `let trimmed_label = label.trim();`
2. **Validation**: Returns error for empty/whitespace-only labels
3. **Transaction**: Uses `with_immediate_transaction` for proper locking
4. **Dual deletion**: Deletes from both `labels` and `bead_labels` tables
5. **Dirty marking**: Calls `mark_dirty_tx(tx, issue_id)` after deletion

## Code

```rust
pub fn remove_label(&self, issue_id: &str, label: &str) -> Result<()> {
    let trimmed_label = label.trim();
    if trimmed_label.is_empty() {
        return Err(anyhow::anyhow!("Label cannot be empty or whitespace only"));
    }

    self.with_immediate_transaction(|tx| {
        tx.execute(
            "DELETE FROM labels WHERE issue_id = ?1 AND label = ?2",
            params![issue_id, trimmed_label],
        )?;
        tx.execute(
            "DELETE FROM bead_labels WHERE bead_id = ?1 AND label = ?2",
            params![issue_id, trimmed_label],
        )?;
        mark_dirty_tx(tx, issue_id)?;
        Ok(())
    })
}
```

All acceptance criteria satisfied. Code compiles cleanly.
