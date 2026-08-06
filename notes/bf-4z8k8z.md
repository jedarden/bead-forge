# bf-4z8k8z: Add update_title() field-specific method

## Task
Add `Issue::update_title()` method that updates only the title field.

## Finding
The `update_title()` method **already exists** in the codebase:

**Location**: `src/storage/sqlite.rs:935-944`

**Implementation**:
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

## Verification Against Acceptance Criteria

| Criterion | Status |
|-----------|--------|
| Method exists: fn update_title(&self, id: &str, title: &str) -> Result<()> | ✅ Implemented |
| Executes direct SQL UPDATE for title column | ✅ Uses direct UPDATE |
| Preserves all other fields (status, priority, description, etc.) | ✅ Only updates title and updated_at |
| Runs within transaction | ✅ Uses with_immediate_transaction |
| Returns Result type | ✅ Returns Result<()> |

## Conclusion
This bead was already completed in a prior implementation. No code changes required.
