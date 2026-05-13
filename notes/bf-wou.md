# BF-WOU: Batch Placeholder References for Mitosis Pattern

## Summary

Verified that batch placeholder references (@0, @1, @2, etc.) work correctly for the NEEDLE mitosis pattern.

## Implementation Status

✅ **Fully implemented** in `src/batch.rs`:
- `resolve_reference()` function (lines 142-153) handles placeholder resolution
- `execute_batch()` uses `resolve_reference()` for all operations that reference IDs
- `mitosis()` and `mitosis_ex()` functions generate @-references for dependencies

## Test Coverage

✅ **Comprehensive tests** in `tests/batch_mitosis.rs`:

1. **`test_mitosis_atomic_batch`**: Tests basic mitosis with @-references
2. **`test_batch_rollback_on_error`**: Tests atomicity and rollback
3. **`test_mitosis_helper_produces_at_references`**: Tests @-reference generation
4. **`test_cli_batch_json_at_references`**: **End-to-end CLI test** verifying `bf batch --json` with @-references

## Manual Verification

Tested mitosis pattern via CLI:

```bash
# Create parent bead
bf create --title "Parent task to split"
# Parent ID: bf-2g8

# Execute mitosis with @-references
bf batch --json '[
  {"op": "create", "title": "Child 1", "type_": "task", "priority": 2},
  {"op": "create", "title": "Child 2", "type_": "bug", "priority": 0},
  {"op": "dep_add_blocker", "parent": "@0", "child": "bf-2g8"},
  {"op": "dep_add_blocker", "parent": "@1", "child": "bf-2g8"},
  {"op": "close", "id": "bf-2g8", "reason": "Split via CLI"}
]'

# Output:
# [op 0] ok: bf-2zz  (@0 resolved to bf-2zz)
# [op 1] ok: bf-28g  (@1 resolved to bf-28g)
# [op 2] ok
# [op 3] ok
# [op 4] ok
```

**Results:**
- ✅ Parent `bf-2g8` is closed
- ✅ Children `bf-2zz` and `bf-28g` are created and open
- ✅ Dependencies created: parent depends on both children (children block parent)
- ✅ @-references resolved correctly to created IDs

## Mitosis Pattern

The mitosis pattern atomically splits a parent bead into children:

1. **Create N child beads** (operations 0 to N-1)
2. **Add dependencies** using @0, @1, ..., @(N-1) to reference created children
3. **Close the parent** bead

All operations run in a single `BEGIN IMMEDIATE` transaction, ensuring atomicity.

## Resolution Logic

From `src/batch.rs:142-153`:

```rust
fn resolve_reference(reference: &str, created_ids: &[String]) -> String {
    if let Some(rest) = reference.strip_prefix('@') {
        if let Ok(idx) = rest.parse::<usize>() {
            if idx < created_ids.len() {
                return created_ids[idx].clone();
            }
        }
    }
    reference.to_string()  // passthrough for literal IDs
}
```

**Key behaviors:**
- @0, @1, @2, etc. resolve to IDs of beads created at positions 0, 1, 2, ...
- Literal IDs (e.g., "bf-123") pass through unchanged
- Out-of-bounds @-refs return as-is (will fail on validation)

## Test Results

All tests pass:
- ✅ `test_mitosis_atomic_batch`
- ✅ `test_batch_rollback_on_error`
- ✅ `test_mitosis_helper_produces_at_references`
- ✅ `test_cli_batch_json_at_references`

```
cargo test --test batch_mitosis
# test result: ok. 4 passed; 0 failed; 0 ignored
```

## Additional Testing

Also verified the `bf mitosis` command (high-level wrapper):

```bash
bf mitosis --children '[{"title": "Child A", "type_": "task", "priority": 0}]' bf-parent
```

**Results:**
- ✅ Creates child beads atomically
- ✅ Establishes dependencies using @-references internally
- ✅ Closes parent bead
- ✅ JSON output format works correctly

## Conclusion

Batch placeholder references (@0, @1, etc.) are **fully implemented and tested** for the NEEDLE mitosis pattern. The feature works correctly through:
- Direct API: `execute_batch()` with `BatchOp` enum
- CLI command: `bf batch --json` with manual @-references
- High-level command: `bf mitosis` (uses @-references internally)

All operations are atomic via `BEGIN IMMEDIATE` transactions, ensuring no partial state on failure.
