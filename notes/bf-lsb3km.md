# Self-Blocking Prevention Implementation (bf-lsb3km)

## Implementation Summary

### 1. Added Self-Blocking Check to Batch Operations

**File**: `src/batch.rs`

Added validation in `execute_dep_add_blocker()` function to prevent self-blocking:

```rust
// Prevent self-blocking: a bead cannot block itself
if id == blocker {
    return Err(anyhow!(
        "Cannot add self-blocking dependency: bead '{}' cannot block itself",
        id
    ));
}
```

This matches the validation already present in `src/storage/sqlite.rs::add_dependency()`.

### 2. Comprehensive Test Coverage

Created two test files:

#### Unit Tests: `tests/test_self_blocking_prevention.rs`
- Tests storage layer rejects self-blocking
- Tests batch operations reject self-blocking
- Tests error message quality
- Tests edge cases (different dependency types, case sensitivity)
- Tests that valid dependencies (different beads) still work

#### CLI Integration Tests: `tests/test_self_blocking_cli.rs`
- Tests `bf dep add` command rejects self-blocking with same ID
- Tests `bf dep add` allows valid dependencies between different beads
- Tests `bf batch` operations reject self-blocking
- Tests `bf batch` allows valid dependencies
- Tests error message quality

## Acceptance Criteria Verification

✅ **Test that a bead cannot block itself**
   - Implemented in both storage layer and batch operations

✅ **Verify `bf dep add <bead-id> --blocks <same-id>` fails with clear error**
   - Storage layer returns: "Cannot add self-blocking dependency: bead 'X' cannot block itself"
   - Batch operations return: "Cannot add self-blocking dependency: bead 'X' cannot block itself"

✅ **Test error message is informative**
   - Error clearly states what went wrong ("Cannot add self-blocking dependency")
   - Error includes the problematic bead ID
   - Error explains why ("cannot block itself")

✅ **Ensure this works for both `bf dep add` and batch operations**
   - `bf dep add` uses storage layer (already had check)
   - Batch operations use `execute_dep_add_blocker()` (now has check)

## Code Changes

### Modified Files
1. `src/batch.rs` - Added self-blocking check to `execute_dep_add_blocker()`

### New Files
1. `tests/test_self_blocking_prevention.rs` - Unit tests
2. `tests/test_self_blocking_cli.rs` - CLI integration tests

## Verification

The implementation prevents the following scenarios:

1. **Direct self-blocking**: `bf dep add bf-abc --blocks bf-abc` ❌
2. **Batch self-blocking**: `[{"op":"dep_add_blocker","id":"bf-abc","blocker":"bf-abc"}]` ❌
3. **Placeholder self-blocking**: Create bead then `[{"op":"dep_add_blocker","id":"@0","blocker":"@0"}]` ❌

While still allowing:

1. **Valid blocking**: `bf dep add bf-abc --blocks bf-def` ✅
2. **Batch valid blocking**: `[{"op":"dep_add_blocker","id":"bf-abc","blocker":"bf-def"}]` ✅
3. **Non-blocking self-reference**: `relates_to` type dependencies (if needed) ✅

## Notes

- The storage layer already had this validation in `add_dependency()`
- The batch operation path bypassed the storage layer, so it needed its own check
- Error messages are consistent between both code paths
- The validation only applies to blocking dependency types (Blocks, ParentChild, etc.)
- Non-blocking types like RelatesTo may allow self-reference if needed
