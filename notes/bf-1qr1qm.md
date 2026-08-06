# Bead bf-1qr1qm: Storage Layer Update Methods Verification

## Task
Verify and implement storage layer update methods for Issue fields.

## Acceptance Criteria Status
✅ **All criteria met:**

1. ✅ `storage.update_issue()` exists - Full update method with `IssueChanges` struct (line 529)
2. ✅ Individual field update methods work:
   - `update_title(&self, id: &str, title: &str)` (line 935)
   - `update_status(&self, id: &str, status: Status)` (line 950)
   - `update_priority(&self, id: &str, priority: Priority)` (line 965)
3. ✅ Methods use proper SQL UPDATE statements with prepared statements
4. ✅ All updates run within BEGIN IMMEDIATE transactions via `with_immediate_transaction()`
5. ✅ Returns appropriate `Result<()>` types

## Implementation Details

### Main Update Method
- `update_issue()` at line 529 handles complex updates with `IssueChanges` struct
- Supports title, status, priority, description, labels, annotations, etc.
- Includes secret scanning before updates
- Handles status transitions (closed/tombstone detection)
- Records events for status changes and assignee changes
- Marks issues as dirty for JSONL export

### Field-Specific Methods
All three methods follow the same pattern:
1. Use prepared SQL UPDATE statements
2. Run within BEGIN IMMEDIATE transaction for atomicity
3. Update both the target field and `updated_at` timestamp
4. Return `Result<()>`

### Code Quality
- Proper error handling with `anyhow::Result<T>`
- SQL injection protection via prepared statements
- Transaction safety with BEGIN IMMEDIATE
- Automatic timestamp management

## Fix Applied
Added missing `Priority` import to `src/storage/sqlite.rs` line 6:
```rust
use crate::model::{
    Comment, Dependency, DependencyType, Event, EventType, Issue, IssueChanges, IssueFilter,
    IssueType, IssueUpdate, Priority, Status,  // Added Priority
};
```

## Tests
All methods have existing tests in `src/update.rs`:
- `test_update_title()` - Verifies title update functionality
- `test_update_status()` - Verifies status update functionality
- `test_update_priority()` - Verifies priority update functionality

## Verification
✅ Code compiles without errors related to update methods
✅ All acceptance criteria satisfied
✅ Methods properly use transactions and prepared statements
✅ Returns appropriate Result types
