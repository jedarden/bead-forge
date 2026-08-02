# Bead bf-655xo: Default Values for Created Beads

## Summary

Verified that newly created beads have the correct default values:
- **status**: `open` (via `Status::default()`)
- **priority**: `2` (via `Priority::default()` → `Priority::MEDIUM`)
- **type**: `task` (via `IssueType::default()`)

## Implementation Details

The defaults are implemented through Rust's `Default` trait:

### 1. Status Default (src/model.rs:41)
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Open,
    // ... other variants
}
```

### 2. Priority Default (src/model.rs:141-145)
```rust
impl Default for Priority {
    fn default() -> Self {
        Self::MEDIUM
    }
}

impl Priority {
    pub const MEDIUM: Self = Self(2);
    // ...
}
```

### 3. IssueType Default (src/model.rs:180)
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    #[default]
    Task,
    // ... other variants
}
```

### 4. Applied in Issue::new() (src/model.rs:617-630)
```rust
pub fn new(id: String, title: String, source_repo: String) -> Self {
    let now = Utc::now();
    Issue {
        id,
        title,
        source_repo: Some(source_repo),
        created_at: now,
        updated_at: now,
        status: Status::default(),        // → Open
        priority: Priority::default(),    // → Priority(2)
        issue_type: IssueType::default(), // → Task
        ..Default::default()
    }
}
```

## Verification

Created test bead `bf-3rvbp0` with `bf create --title "Test default values" --json` and verified:
```json
{
  "id": "bf-3rvbp0",
  "status": "open",
  "priority": 2,
  "issue_type": "task",
  "title": "Test default values"
}
```

All three default values are correctly applied.

## Notes

- The CLI `--type` and `--priority` flags have matching default values (`"task"` and `"2"`) for consistency
- These defaults are overrideable via CLI flags (already implemented in bf-5v6oe)
- No additional code changes needed - the implementation was already complete
