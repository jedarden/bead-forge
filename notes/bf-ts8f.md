# E2E Tests Already Implemented (bf-ts8f)

## Finding

The bead description states "tests/test_jsonl.rs is empty (0 tests run)" but this is **outdated**. The E2E tests were already implemented in previous commits:

- `f2c7b6a` - test(bf-ts8f): add E2E bf vs br output parity test
- `12bb810` - test(bf-ts8f): add E2E bf vs br output parity test
- `7952b68` - test(bf-ts8f): add actual br command execution E2E test
- `626cb92` - test(e2e): run actual bf CLI command in E2E br parity test

## Current State

**tests/test_jsonl.rs** contains 8 E2E tests (all passing):

1. `test_e2e_br_vs_bf_list_output_parity` - **The key E2E test**: Runs both `bf list --format json --all` and `br list --format json --all` on the same workspace and compares outputs bead-by-bead
2. `test_e2e_bf_vs_br_output_parity_forge_snapshot` - Tests with forge-snapshot.jsonl fixture
3. `test_e2e_bf_vs_br_output_parity_needle_snapshot` - Tests with needle-snapshot.jsonl fixture
4. `test_e2e_bf_vs_br_output_parity_simple_bead` - Tests with simple_bead.jsonl fixture
5. `test_e2e_jsonl_round_trip_output_parity` - Tests export after import produces identical JSONL

## Test Results

```
running 8 tests
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured
```

The tests verify:
- bf can import br-generated JSONL files
- bf list produces identical JSON output to br list
- All critical fields match (id, title, status, priority, type, timestamps, etc.)
- Dependencies and comments are properly stripped for br compatibility
- Round-trip JSONL export/import preserves data integrity
