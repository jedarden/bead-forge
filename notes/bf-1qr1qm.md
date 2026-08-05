# Bead bf-1qr1qm: Storage Layer Update Methods Verification

## Task
Implement storage layer update methods (Issue::update_title(), update_status(), update_priority())

## Verification Results
All required methods already exist in `src/storage/sqlite.rs` and meet the acceptance criteria:

### 1. Individual Field Update Methods ✅
- **`update_title()`** - Line 533-539
- **`update_status()`** - Line 545-551
- **`update_priority()`** - Line 557-563

All three methods are convenience wrappers that call `update_issue()` with appropriate `IssueChanges` structs.

### 2. Core Update Method ✅
- **`update_issue()`** - Line 565-862
  - Accepts an `IssueChanges` struct with optional field updates
  - Validates bead existence before updating
  - Handles secret scanning before applying changes
  - Processes status transitions (closed state handling)
  - Records events for status changes and assignee changes
  - Marks beads as dirty for JSONL export

### 3. SQL UPDATE Statements ✅
- Line 733: `UPDATE issues SET {} WHERE id = ?`
- Dynamic query construction based on fields being updated
- Proper parameter binding for SQL safety
- Preserves all other fields not in the update

### 4. Transaction Handling ✅
- Line 614: `self.with_immediate_transaction(|tx| {`
- All updates run within BEGIN IMMEDIATE transactions
- Exponential backoff on SQLITE_BUSY (see `with_immediate_transaction()` implementation)
- Proper rollback on error

### 5. Return Types ✅
- All methods return `Result<()>`
- Error handling via `anyhow::Error`
- Secret detection returns `SecretError` wrapped in `Result`

## Implementation Details

### Method Signatures
```rust
pub fn update_title(&self, id: &str, title: String) -> Result<()>
pub fn update_status(&self, id: &str, status: Status) -> Result<()>
pub fn update_priority(&self, id: &str, priority: i32) -> Result<()>
pub fn update_issue(&self, id: &str, changes: &IssueChanges) -> Result<()>
```

### Example Usage
```rust
// Update title
storage.update_title("bf-abc123", "New title".to_string())?;

// Update status
storage.update_status("bf-abc123", Status::InProgress)?;

// Update priority
storage.update_priority("bf-abc123", 1)?;

// Or use update_issue directly for multiple fields
let changes = IssueChanges {
    title: Some("New title".to_string()),
    status: Some(Status::InProgress),
    priority: Some(1),
    ..Default::default()
};
storage.update_issue("bf-abc123", &changes)?;
```

## Additional Features
The implementation also includes:
- Automatic `updated_at` timestamp updates
- Cascade status transition handling (unblocking dependents)
- Critical path cache invalidation
- Blocked issues cache rebuilding
- Event recording for audit trails

## Conclusion
All acceptance criteria are met. The storage layer update methods are properly implemented with:
- Correct SQL UPDATE statements
- Proper transaction handling (BEGIN IMMEDIATE)
- Appropriate Result types
- Additional safety features (secret scanning, validation)
