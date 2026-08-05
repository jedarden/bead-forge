# Remaining Test Modules Analysis - bf-3ab8sz

## Objective

Identified test modules in the bead-forge codebase that were not covered in the previous test execution bead (bf-5qr973).

## Previous Execution (bf-5qr973)

Executed 13 test modules focusing on format, envelope, and integration tests:

- `test_json_formatter` - JSON formatter unit tests
- `json_formatter_verification` - JSON formatter verification
- `test_jsonl` - JSONL format tests
- `jsonl_compat` - JSONL compatibility tests
- `test_trace_e2e_verification` - End-to-end trace verification
- `test_cargo_test_execution` - Cargo test execution validation
- `test_show_command` - Show command output tests
- `test_special_chars` - Special character handling
- `test_version_display` - Version display tests
- `secret_scanning` - Secret scanning tests
- `velocity_close_integration` - Velocity close integration
- `velocity_seed_integration` - Velocity seed integration
- `test_blocked_cascade` - Blocked cascade tests

## Current Codebase State

- **Total test modules**: 156
- **Modules executed**: 13
- **Remaining modules**: 143

## Test Module Categories

### Remaining modules include:

1. **Autoflush tests (7 modules)**: Batch claim/delete, comprehensive mutations, diagnostics, failure contract, mutation, readonly, wiring
2. **Batch tests (4 modules)**: Atomic, cascade and rotation, mitosis, transaction tests
3. **Claim tests (3 modules)**: Fallback, race, stress
4. **Close/Reopen tests (2 modules)**: Basic close/reopen, cycle tests
5. **Doctor tests (3 modules)**: Reconcile, repair unflushed, safety stack
6. **Epic tests (17 modules)**: Comprehensive epic functionality testing
7. **Label tests (11 modules)**: Label storage, import/export, edge cases
8. **Priority tests (4 modules)**: P0/P1/P2 priority validation
9. **Integration tests (20+ modules)**: Various command integration tests
10. **Unit tests (70+ modules)**: Specific command and feature unit tests

## Output

Created `.beads/traces/bf-4kzs6h-remaining-modules.txt` containing:
- Header with execution statistics
- One module name per line (143 modules)
- Ready for batch execution using `cargo test <module>`

## Notes

- Some modules from bf-5qr973 notes (text_format, toon_format, non_json, claim_stats, mod) are sub-tests within `envelope/mod.rs` rather than standalone modules
- The remaining list includes only standalone `.rs` files from the `tests/` directory
- All module names are derived from filenames without the `.rs` extension

## Next Steps

The remaining 143 modules can be executed in batches to complete full test suite coverage.
