# Epic Default Priority Test (bf-3tv27)

## Summary
Verified that epics created without specifying a priority default to P2 (Medium), confirming the expected behavior.

## What Was Tested

Created `tests/epic_default_priority.rs` with 7 comprehensive tests:

1. **test_epic_default_priority_is_p2**: Verifies that `Issue::default()` for epic type assigns P2 (Medium) priority
2. **test_epic_default_priority_storage**: Tests that epic created with default priority stores and retrieves correctly as P2
3. **test_epic_default_priority_serialization**: Confirms JSON serialization/deserialization preserves P2 default
4. **test_priority_default_impl_returns_p2**: Validates `Priority::default()` returns P2 (Medium)
5. **test_multiple_epics_with_default_priority**: Batch test ensuring multiple epics all get P2 by default
6. **test_epic_default_vs_explicit_priorities**: Comparison test showing default alongside P0-P4 priorities
7. **test_issue_new_default_priority**: Verifies `Issue::new()` constructor also applies P2 default

## Implementation Details

The default priority mechanism works as follows:

1. **CLI Layer**: `src/cli/mod.rs` line 49 defines `#[arg(long, default_value = "2")]` for the `--priority` argument
2. **Model Layer**: `src/model.rs` lines 118-122 implement `Default for Priority` returning `Priority::MEDIUM` (value 2)
3. **Create Command**: `cmd_create()` function (line 1066) sets `issue.priority = Priority(priority)` using the CLI default

## Test Results

All 7 tests pass:
```
running 7 tests
test test_epic_default_priority_is_p2 ... ok
test test_epic_default_priority_serialization ... ok
test test_epic_default_vs_explicit_priorities ... ok
test test_issue_new_default_priority ... ok
test test_epic_default_priority_storage ... ok
test test_priority_default_impl_returns_p2 ... ok
test test_multiple_epics_with_default_priority ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Verification

The test confirms that:
- Epic-type beads without explicit priority assignment default to P2 (Medium)
- This default is consistent across creation, storage, and serialization
- The default matches both the CLI argument default and the Rust `Default` trait implementation
