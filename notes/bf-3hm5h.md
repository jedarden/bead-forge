# bf-3hm5h — NULL-datetime & schema hardening (verification)

Spec: `docs/plan/plan.md` Phase 7.3 (P0, small).

## Status

Implementation already landed in commit `9d7f519`
(*fix(bf-3hm5h): NULL-datetime tolerance + NOT NULL schema doctor/fixer*),
which is an ancestor of the current HEAD. This retry dispatch re-verified the
acceptance criteria and found them satisfied; this note records that verification.

## Acceptance criteria — verified

1. **`parse_datetime` no longer crashes on NULL datetime columns.**
   `parse_required_datetime(Option<String>)` (`src/storage/sqlite.rs`) maps
   `None` / empty / whitespace-only to `DateTime::<Utc>::UNIX_EPOCH` instead of
   letting `row.get::<String>()` raise the fatal `InvalidColumnType` that aborted
   the whole list/flush. Genuinely malformed non-empty values still error (a
   distinct corruption class).

2. **All datetime column reads audited for NULL tolerance.**
   - `row_to_issue_conn` and the other `row_to_*` readers route required columns
     (`created_at`, `updated_at`) through `parse_required_datetime` and optional
     columns (`closed_at`, `due_at`, `defer_until`, `deleted_at`, `compacted_at`)
     through the `parse_opt_dt` closure, which already maps NULL/empty → `None`.
   - `velocity.rs` reads `claimed_at` behind a `claimed_at IS NOT NULL AND
     claimed_at != ''` filter and degrades gracefully (`return Ok(false)`) on any
     remaining unparseable value rather than erroring the close path.

3. **Doctor detector + repair fixer for NULL-in-NOT-NULL rows.**
   `doctor::check_null_not_null` surfaces `NullNotNullViolation`s in `bf doctor`
   (and `--json`); `doctor::fix_null_not_null` repairs them in place, wired to the
   `bf doctor --fix-schema` flag in `src/cli/mod.rs`.

4. **Regression tests with hand-crafted NULL rows in tempdir DBs.**
   `doctor::tests::{test_check_reports_null_not_null,
   test_fix_null_not_null_repairs_in_place, test_check_no_null_not_null_on_clean_db}`
   plus `parse_datetime_tests::{accepts_rfc3339_and_sqlite_native_formats,
   required_datetime_tolerates_null_and_empty}` and the `velocity` datetime tests.

## Test run

Verified against a clean HEAD checkout in an isolated worktree (the shared working
tree currently carries another in-flight bead's uncommitted `src/format/` changes,
unrelated to this bead):

```
running 9 tests ... test result: ok. 9 passed; 0 failed
```
