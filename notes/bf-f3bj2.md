# bf-f3bj2: ReadyCandidate to Issue Conversion

## Implementation

Added `ReadyCandidate` struct and conversion function to `src/model.rs`.

### ReadyCandidate Struct

```rust
pub struct ReadyCandidate {
    pub id: String,
    pub title: String,
    pub priority: Priority,
    pub status: Status,
    pub created_at: DateTime<Utc>,
    pub labels: Vec<String>,
}
```

### Conversion Function

Two conversion methods provided:
1. `ReadyCandidate::to_issue()` - Instance method
2. `ready_candidate_to_issue()` - Standalone function

Both convert ReadyCandidate to full Issue with sensible defaults:
- All ReadyCandidate fields mapped directly to Issue fields
- `updated_at` set to current time
- Missing fields get `None` or `Default` values:
  - `description`, `design`, `acceptance_criteria`, `notes` → None
  - `issue_type` → IssueType::Task (default)
  - `assignee`, `owner`, `created_by` → None
  - `closed_at`, `close_reason` → None
  - `dependencies`, `comments` → Empty vectors
  - `annotations` → Empty BTreeMap
  - All timestamp/option fields → None
  - Boolean flags → false

### Tests Added

7 comprehensive tests covering:
- Basic conversion with all fields
- Default values for missing Issue fields
- `updated_at` timestamp behavior
- Standalone function behavior
- Serialization/deserialization
- Custom status handling
- Empty labels handling

All tests pass: ✓ 7 passed

### Compilation

Clean build with no errors or warnings from the new code.
