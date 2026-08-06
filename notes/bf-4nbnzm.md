# Bead bf-4nbnzm: Basic CRUD Operations for Issues

## Status: Complete

The basic CRUD operations for issues were already fully implemented in `src/storage/sqlite.rs`. This task only required minor fixes to compile existing code.

## Changes Made

### 1. Fixed Missing Import (src/storage/sqlite.rs:4-7)
Added `Priority` to the imports from `crate::model`:
```rust
use crate::model::{
    Comment, Dependency, DependencyType, Event, EventType, Issue, IssueChanges, IssueFilter,
    IssueType, IssueUpdate, Priority, Status,  // Added Priority
};
```

### 2. Fixed SecretError Field Visibility (src/storage/sqlite.rs:22-25)
Changed the tuple struct field from private to public:
```rust
#[derive(Debug, thiserror::Error)]
#[error("secret detected: {0}")]
pub struct SecretError(pub String);  // Added `pub`
```

## Implemented CRUD Operations

### get_issue() (line 184)
- **Signature**: `pub fn get_issue(&self, id: &str) -> Result<Option<Issue>>`
- **Returns**: `Ok(Some(Issue))` if found, `Ok(None)` if not found
- **NOT_FOUND handling**: Returns `Ok(None)` instead of error when bead doesn't exist
- **Row mapping**: Uses `row_to_issue_conn()` to map rusqlite rows to Issue model

### create_issue() (line 412)
- **Signature**: `pub fn create_issue(&self, issue: &Issue) -> Result<()>`
- **Transaction**: Uses `with_immediate_transaction()` for atomic writes
- **Features**:
  - Secret scanning before insert
  - Computes content_hash if not set
  - Inserts issue, labels, dependencies, comments, annotations atomically
  - Records 'created' event for br parity
  - Marks as dirty for JSONL export
  - Invalidates critical path cache

### update_issue() (line 529)
- **Signature**: `pub fn update_issue(&self, id: &str, changes: &IssueChanges) -> Result<()>`
- **NOT_FOUND handling**: Returns `Err(anyhow!("Bead not found: {}", id))` if bead doesn't exist
- **Transaction**: Uses `with_immediate_transaction()` for atomic writes
- **Features**:
  - Validates bead existence before update
  - Secret scanning on changed fields
  - Dynamic SET clause based on IssueChanges fields
  - Handles assignee clearing (empty string → NULL)
  - Status transition handling (closed fields, reopen detection)
  - Creates events for status/assignee changes
  - Invalidates caches when status changes
  - Marks as dirty for export

## Acceptance Criteria Verification

| Criterion | Status | Implementation |
|-----------|--------|----------------|
| Implement get_issue() in src/storage/sqlite.rs | ✅ | Line 184 |
| Implement create_issue() in src/storage/sqlite.rs | ✅ | Line 412 |
| Implement update_issue() in src/storage/sqlite.rs | ✅ | Line 529 |
| Use with_immediate_transaction() for write operations | ✅ | Both create (line 428) and update (line 578) use it |
| Map rusqlite rows to Issue model from src/model.rs | ✅ | row_to_issue_conn() at line 1320 |
| Handle NOT_FOUND errors appropriately | ✅ | get_issue returns Ok(None), update_issue returns "Bead not found" error |
| Include basic error types for storage operations | ✅ | Uses Result<T> from crate::error, custom SecretError |

## Notes

- The CRUD operations were already fully implemented with proper error handling
- All operations use the transaction wrapper for atomicity
- Row mapping is handled by `row_to_issue_conn()` and `row_to_issue_partial()` methods
- Error handling uses the standard `Result<T>` type from `crate::error`
- NOT_FOUND is handled gracefully for reads and explicitly for writes
