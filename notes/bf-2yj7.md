# bf-2yj7: doctor --repair destroys unflushed db-only beads — FIXED

## Summary

This bead is already **COMPLETE**. All acceptance criteria have been verified:

1. ✅ **Integration test**: `test_doctor_repair_refuses_unflushed_beads` in `tests/doctor_repair_unflushed.rs` verifies that `doctor --repair` refuses when unflushed beads exist, db unchanged, bead still present.

2. ✅ **Integration test**: `test_doctor_repair_flush_first_preserves_unflushed` verifies that `doctor --repair --flush-first` preserves beads in both db and issues.jsonl.

3. ✅ **Integration test**: `test_doctor_repair_force_loses_unflushed_beads` verifies that `doctor --repair --force` warns and proceeds (with data loss).

4. ✅ **Unit test**: `test_doctor_reports_unflushed_count` in `src/doctor.rs` verifies that `doctor` reports unflushed-bead count as a health line.

5. ✅ **Documentation**: README.md line 266 and plan.md lines 245-259 document the authority inversion and flush-before-repair rule.

6. ✅ **Tests green**: All 96 unit tests pass, all 9 integration tests pass.

## What Was Implemented

The fix was already in place:

- **`src/doctor.rs`**: 
  - `count_unflushed()` and `get_unflushed_ids()` functions detect dirty beads
  - `check()` reports unflushed count in health check output
  - `repair()` has `flush_first` and `force` parameters for safe/forced repair
  - Refuses repair with unflushed beads unless `--flush-first` or `--force` is passed

- **`src/cli/mod.rs`**: CLI exposes `--flush-first` and `--force` flags for `doctor` command

- **`tests/doctor_repair_unflushed.rs`**: Comprehensive integration tests covering all scenarios

- **`src/sync.rs`**: Clears dirty marks after flush (small fix)

## Verification Results

```bash
# All unit tests pass
cargo test --lib
test result: ok. 96 passed; 0 failed; 0 ignored

# Integration tests pass
cargo test --test doctor_repair_unflushed
test result: ok. 9 passed; 0 failed

# Build succeeds
cargo build
BUILD SUCCESS
```

## Historical Context

On 2026-06-10, seven independent agents across seven workspaces (ARMOR, NEEDLE, AgentScribe, kalshi-weather, jedarden.com, vibe-coding-discovery, face/pose/sun repos) each lost their entire first batch of freshly created beads by running `doctor --repair` after bulk creates. Four db-only beads in ARMOR (bf-4rm7/5zxa/tojg/tr44) were permanently lost.

This fix implements the flush-before-repair protection that would have prevented this data loss.
