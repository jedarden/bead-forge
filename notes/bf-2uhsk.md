# bf-2uhsk: `bf reopen` clears the assignee field

## Implementation Status: ✅ COMPLETE (already committed)

The fix and its regression test were already committed in prior commits
reachable from `main`. This pass re-verified the behavior rather than
re-implementing it, and confirms the acceptance criteria are met.

## What the fix is

`cmd_reopen` (`src/cli/mod.rs:1581`) builds an `IssueChanges` that sets both
`status: Some(Status::Open)` **and** `assignee: Some(String::new())`. The empty
string is the three-valued "clear to NULL" signal: `update_issue`'s storage
layer (`src/storage/sqlite.rs:545`) maps a whitespace-empty assignee to
`assignee = NULL`, never persisting a literal empty string that would read back
as "assigned" and hide the bead from claiming. Defaulting `assignee` to `None`
would mean "leave unchanged", leaving a stale foreign assignee on a now-open
bead — the NEEDLE fleet-audit root cause this bead addresses.

## Where it landed (both reachable from HEAD)

- **Fix:** `fe50a54 feat(bf-1dcws)` introduced `assignee: Some(String::new())`
  in `cmd_reopen` (verified via `git log -S`).
- **Regression test:** `16ab5b4 test(bf-2uhsk): pin that bf reopen clears the
  assignee` added two tests to `tests/test_close_reopen_integration.rs`:
  - `test_reopen_clears_assignee` — assign a worker, close, reopen, assert
    `bf show` reports a null/absent assignee.
  - `test_reopen_without_assignee_is_noop` — reopen an unassigned bead, assert
    no error and assignee remains unset.
- **Merge:** `81e4227` pulled the sibling fix beads (bf-gj673, bf-4mj7l,
  bf-2uhsk) together.

## Re-verification (this pass)

- `cargo build` — clean (no errors).
- `cargo test --test test_close_reopen_integration` — 7 passed, 0 failed,
  including both `test_reopen_clears_assignee` and
  `test_reopen_without_assignee_is_noop`.
- `cargo fmt --check` — the bead's own files (`src/cli/mod.rs`,
  `tests/test_close_reopen_integration.rs`) are fmt-clean. Pre-existing fmt
  drift exists in unrelated files (batch.rs, claim.rs, doctor.rs, merge.rs,
  etc.); reformatting those is out of scope for this bead.
- `cargo clippy -D warnings` — not gated for this bead: the codebase carries a
  known ~134-error clippy baseline (see memory: clippy-baseline-not-a-gate).
  None of this bead's changes introduce new clippy regressions.
