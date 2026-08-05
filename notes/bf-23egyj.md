# Bead bf-23egyj: Fix unused imports in test files

## Investigation

The task description mentions:
- `tests/test_p0_advanced_operations.rs: std::thread, std::time::Duration`

### Findings

1. **`tests/test_p0_advanced_operations.rs` does NOT have these imports**
   - The file only imports: `bead_forge::model::*` and `bead_forge::storage::Storage`
   - No `std::thread` or `std::time::Duration` imports exist

2. **Other test files that DO have these imports are using them correctly**
   - `tests/claim_race.rs` - Has and uses both `std::thread` and `std::time::Duration as StdDuration`
   - `tests/claim_stress.rs` - Has and uses both `std::thread` and `std::time::{Duration, Instant}`
   - `tests/test_trace_e2e_verification.rs` - Has and uses `std::time::Duration`

3. **Cargo compilation confirms no unused import warnings for these**
   - Ran `cargo build --tests` and checked warnings
   - No warnings about `std::thread` or `std::time::Duration` being unused in test files

## Conclusion

The task description appears to be outdated or incorrect. The mentioned unused imports either:
1. Never existed in `tests/test_p0_advanced_operations.rs`
2. Were already removed in a previous commit (see `a826b8f docs(bf-1thegl): Document that unused imports were already removed`)

No changes are needed to fix unused imports in test files.

## Actual Unused Import Warnings

The current unused import warnings from `cargo build --tests` are in `src/cli/tests/` directory, not the main `tests/` directory:
- `src/cli/tests/edge_case_json_tests.rs`: `std::process::Command`, `bf_binary`, `format_detection`, `super::*`
- `src/cli/tests/error_json_tests.rs`: `format_detection`
- `src/cli/tests/json_output.rs`: `PathBuf`, `std::process::Command`
- `src/cli/tests/json_schema_validation.rs`: `std::process::Command`, `bf_binary`, `format_detection`, `super::*`
- `src/cli/tests/list_ready_recent_json_tests.rs`: `super::*`
- `src/cli/tests/search_json_tests.rs`: `std::process::Command`, `super::*`
- `src/cli/tests/show_json_tests.rs`: `std::process::Command`, `bf_binary`, `format_detection`, `super::*`

These are in a different location and were not mentioned in the task description.
