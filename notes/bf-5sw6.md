# bf-5sw6: limit=0 behavior verification

## Summary
The bug for `bf ready --limit 0` returning no results was **already fixed** in previous commits. All acceptance criteria are met.

## What was verified

### 1. Functional behavior ✅
- `bf ready --limit 0` correctly returns all ready beads (unlimited behavior)
- `bf ready --limit 1` returns 1 bead
- Tested with 4 ready beads in workspace: limit=0 returned all 4

### 2. Implementation ✅
- `get_ready_candidates()` in `src/claim.rs` handles `limit=0` by omitting the LIMIT clause entirely
- SQL LIMIT 0 would return 0 rows (verified in test), so the implementation correctly special-cases limit=0
- Code at line 419: `let unlimited = limit == 0;`

### 3. Tests ✅
- Unit test: `claim::tests::test_get_ready_candidates_limit_zero_returns_all` passes
- Integration tests in `tests/limit_zero.rs`:
  - `test_ready_limit_zero_returns_all`: Tests limit=0 returns all 15 beads
  - `test_ready_limit_zero_direct_sql_check`: Verifies SQL LIMIT 0 behavior and application handling

### 4. Help text ✅
- `bf ready --help` shows: `--limit <LIMIT>  Limit results (0 = unlimited) [default: 10]`

## Fix history
The bug was fixed in two commits:
1. **26d3067** (2026-06-24 07:59): "fix(bf-5sw6): correct misleading test for limit=0 behavior"
   - Fixed test to actually test limit=0 instead of i64::MAX
   - Added comprehensive integration tests in tests/limit_zero.rs
   - Verified SQL LIMIT 0 returns 0 rows

2. **f67f7a9** (2026-06-24 08:04): "fix(bf-5sw6): complete limit=0 behavior fix"
   - Removed workaround in cmd_ready that used i64::MAX
   - Ensured get_ready_candidates handles limit=0 correctly

## Acceptance criteria status
- ✅ `bf ready --limit 0` returns all unblocked ready beads
- ✅ Regression test covering limit=0 behavior
- ✅ `bf ready --help` clarifies semantics of --limit 0

All criteria met. No further work required.

## Re-verification (2026-06-24)
Verified that the fix remains in place and working correctly:

### Code inspection confirmed:
1. `src/claim.rs:418-419`: Correctly detects `unlimited = limit == 0`
2. `src/cli/mod.rs:1039`: For `bf list`, converts `limit=0` to `None` (unlimited)
3. Both use different SQL queries when unlimited vs limited

### Runtime verification confirmed:
```bash
$ ./target/debug/bf ready --limit 0
[bf-6mca] Test update flags (priority=2, impact=0, float=1000)
[bf-5me7] Test bead for update flags (priority=2, impact=0, float=1000)
[bf-1qq1] Test bead (priority=2, impact=0, float=1000)
[bf-2j9e] Another test bead (priority=2, impact=0, float=1000)
```
Returns all 4 ready beads as expected.

### Tests pass:
```bash
$ cargo test test_get_ready_candidates_limit_zero_returns_all --lib
test claim::tests::test_get_ready_candidates_limit_zero_returns_all ... ok
```
