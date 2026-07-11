# bf-4uf: Remove Stray Debug Scripts

## Summary

Removed 7 ad hoc debug scripts that were committed at repo root instead of tests/ or scripts/. All scenarios were verified to be covered by proper tests in tests/.

## Files Deleted

1. **test_limit_zero.sh** → `tests/limit_zero.rs`
   - Tested `bf ready --limit 0` returning all beads
   - Coverage: limit_zero.rs tests limit=0, limit=5, and default limit behavior

2. **test_repair_bug.sh** → `tests/doctor_repair_unflushed.rs`
   - Reproduced dirty_issues over-reporting after doctor --repair
   - Coverage: doctor_repair_unflushed.rs tests repair with unflushed beads, dirty_issues tracking, and export_hashes table

3. **test_reproduce.sh** → `tests/test_bf_2hqt.rs`
   - Tested flush → doctor --repair → count_unflushed cycle
   - Coverage: test_bf_2hqt.rs tests repair cycle with count_unflushed verification

4. **test_reproduce_v2.sh** → `tests/test_bf_2hqt.rs`
   - Tested sync --import → count_unflushed cycle
   - Coverage: test_bf_2hqt.rs tests import cycle

5. **test_reproduce_debug.sh** → `tests/test_dirty_repair.rs`
   - Tested dirty_issues table after repair cycle
   - Coverage: test_dirty_repair.rs tests dirty bead tracking after repair

6. **tmp_fix_worker_sessions.py** → NOT a test script
   - Emergency manual repair script for malformed worker_sessions data
   - This was a one-off fix for production data, not a test scenario

7. **tmp_fix_worker_sessions.rs** → NOT a test script
   - Companion Rust script for the same emergency repair
   - Not a test script - one-off data fix tool

## Verification

All scenarios exercised by the shell scripts are covered by proper unit tests in tests/:
- `limit_zero.rs` - limit parameter behavior
- `doctor_repair_unflushed.rs` - repair with unflushed bead protection
- `test_bf_2hqt.rs` - flush/repair/import cycles
- `test_dirty_repair.rs` - dirty_issues table tracking

The tmp_fix_worker_sessions.* files were emergency repair tools for fixing malformed production data, not test scripts. The proper schema validation exists in `tests/velocity_close_integration.rs` which tests worker_sessions with proper RFC3339 timestamps.

## Acceptance Criteria Met

✅ For each script, verified the scenario is covered by an existing test file
✅ No scenarios needed to be ported (all already covered)
✅ Deleted all 7 files from repo root
✅ Repo root no longer contains these ad hoc test_*.sh or tmp_*.py/rs files

## Date

2026-07-11
