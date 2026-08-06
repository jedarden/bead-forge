# bead bf-4z8k8z: update_title() method

## Finding
The `update_title()` field-specific method already exists in the codebase.

## Location
File: `src/storage/sqlite.rs`  
Lines: 913-944

## Implementation Details
The method is already properly implemented with the following characteristics:

1. **Signature**: `pub fn update_title(&self, id: &str, title: &str) -> Result<()>`
2. **SQL Query**: Direct UPDATE statement `"UPDATE issues SET title = ?, updated_at = ? WHERE id = ?"`
3. **Transaction Safety**: Uses `with_immediate_transaction()` for atomic operations
4. **Field Preservation**: Only updates `title` and `updated_at` columns
5. **Error Handling**: Returns `Result<()>` for proper error propagation

## Code
```rust
pub fn update_title(&self, id: &str, title: &str) -> Result<()> {
    let query = "UPDATE issues SET title = ?, updated_at = ? WHERE id = ?";
    let now = Utc::now();

    // Execute within a BEGIN IMMEDIATE transaction for atomicity
    self.with_immediate_transaction(|tx| {
        tx.execute(query, params![title, now.to_rfc3339(), id])?;
        Ok(())
    })
}
```

## Conclusion
Bead can be closed - acceptance criteria already met.
