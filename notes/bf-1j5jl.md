# Bead bf-1j5jl: Epic Default Priority Tests

## Summary
Created comprehensive tests verifying epic default priority behavior. All 6 tests passing.

## Test File Created
`tests/test_epic_default_priority.rs` - 6 tests

## Tests Verified

### 1. `test_epic_default_priority`
- **What:** Creates an epic without explicitly setting priority
- **Verifies:** Epic uses default priority of P2 (MEDIUM, value 2)
- **Expected:** `Priority::MEDIUM` (not P0, P1, or any other value)

### 2. `test_epic_default_vs_explicit_priority`
- **What:** Compares epic with default priority vs explicit P1 priority
- **Verifies:** Default (P2) differs from explicit P1
- **Confirms:** Default priority is not mistakenly set to P1

### 3. `test_default_priority_is_medium`
- **What:** Tests `Priority::default()` directly
- **Verifies:** Default priority is `Priority::MEDIUM` (value 2)
- **Confirms:** Correct Rust Default impl for Priority type

### 4. `test_default_issue_type_is_task_not_epic`
- **What:** Tests `IssueType::default()` directly
- **Verifies:** Default issue type is `Task`, not `Epic`
- **Important:** Epics require explicit `--type epic` flag

### 5. `test_epic_serialization_with_default_priority`
- **What:** Creates epic with default priority and serializes to JSON
- **Verifies:** Serialization preserves priority value 2
- **Roundtrip:** JSON → Rust → JSON maintains P2 priority

### 6. `test_all_priorities_exist_for_epics`
- **What:** Tests epics can have any priority level (P0-P4)
- **Verifies:** Each priority level (0-4) works with epic type
- **Serialization:** Each level roundtrips correctly through JSON

## Key Findings

### Default Priority for All Issue Types
- **Default:** `Priority::MEDIUM` (P2, value 2)
- **Applies to:** All issue types (Task, Bug, Feature, **Epic**, Chore, Docs, Question)
- **Reason:** `Issue::default()` uses `Priority::default()` which is `MEDIUM`

### Epic Type is Not Default
- **Default IssueType:** `Task` (not Epic)
- **To create epic:** Must pass `--type epic` to `bf create`
- **Priority behavior:** Epic uses same default priority (P2) as other types

### CLI Create Command
```rust
// From src/cli/mod.rs
Create {
    priority: i32,     // default_value = "2" (MEDIUM)
    type_: String,     // default_value = "task"
    ...
}
```

When creating an epic:
```bash
bf create --title "My Epic" --type epic
# Creates epic with priority 2 (MEDIUM) - default

bf create --title "My Epic" --type epic --priority 1
# Creates epic with priority 1 (HIGH) - explicit
```

## Test Execution
```bash
cargo test --test test_epic_default_priority
# Result: 6 passed
```

## Related Tests
- `tests/p0_epic_creation.rs` - Tests explicit P0 epic creation
- `tests/p1_epic_creation.rs` - Tests explicit P1 epic creation
- `tests/epic_comprehensive.rs` - Comprehensive epic functionality tests
