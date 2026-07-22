# bf-4waen — Port uncovered orphan-file scenarios into tests/*.rs

Depends on child bf-5wz0l (audit). Parent bf-3o9 (repo hygiene).

## What the audit flagged as NOT COVERED

Child bf-5wz0l audited all 23 orphan files and found 21/23 already covered by
committed `tests/*.rs`. Two gaps were flagged for this bead:

1. **`bf search`** subcommand (exercised by `test_bead_b_operations.sh` Test 8).
   No committed test invoked `bf search` — `grep` for it across `tests/`
   found only prose mentions.
2. **`bf comments add` / `comments list`** CLI round-trip (exercised by
   `test_bf_test3.sh`). Storage layer (`add_comment`/`list_comments`) and the
   `comments` table schema were covered, but no committed test drove the
   `comments add` → `comments list` CLI path end-to-end.

## STATE-NOTE discrepancy (same pattern as the child)

This bead's STATE NOTE claimed two UNTRACKED test files already existed from a
prior run: `tests/comments_cli.rs` and `tests/count_command.rs`. **They do not
exist** — `git status --porcelain -- tests/` is empty, and `find` finds neither
file (outside `target/`). This mirrors the child bead's STATE-NOTE discrepancy
(it claimed staged deletions existed; they did not). So there was nothing to
"confirm and leave staged" — both ports had to be written fresh.

(`count_command.rs`, had it existed, would have covered `test_bf_count.sh`,
which the audit already marked COVERED (#9, `fleet_concurrency.rs:52`) — so it
was not one of the two gaps and would have been a redundant port anyway.)

## Ports written (2 new files, 9 tests, all passing)

### tests/comments_cli.rs — ports `test_bf_test3.sh` (gap #2)

End-to-end `comments add` → `comments list` CLI round-trip via real `bf`
process invocations (pattern borrowed from `fleet_concurrency.rs` /
`test_basic_workflow.rs`):

- `comments_add_and_list_round_trip` — empty-list reports "No comments"; add
  confirms "Added comment"; list surfaces the body text.
- `comments_list_preserves_insertion_order` — three comments listed in
  insertion order.
- `comments_add_joins_multiple_text_args` — multiple text args joined with
  spaces (quoting optional) round-trip correctly.

### tests/search_command.rs — ports `test_bead_b_operations.sh` Test 8 (gap #1)

Exercises `cmd_search` (`src/cli/mod.rs:2482`) → `storage.search_issues`
(`src/storage/sqlite.rs:1441`) end-to-end:

- `search_matches_title_and_description` — title-only AND description-only
  `LIKE %q%` matching; confirms non-matches are excluded.
- `search_filters_by_type` — `--type epic` isolates epics from tasks.
- `search_filters_by_status` — `--status open` vs `--status closed` after a
  `bf close`.
- `search_filters_by_priority_range` — `--priority-min/--priority-max`
  (0=Critical, 4=Backlog) isolates the critical bead.
- `search_filters_by_label` — `--label urgent`.
- `search_limit_caps_results` — `--limit N` caps the result rows.

## Verification

```
$ cargo build                           # clean, exit 0, no errors
$ cargo test --test comments_cli --test search_command
test result: ok. 3 passed; 0 failed   (comments_cli)
test result: ok. 6 passed; 0 failed   (search_command)
```

## Cross-cutting constraint honored

Only the two new `tests/*.rs` files and this notes file are touched. The
unrelated in-flight changes (src, docs, deploy, bf-checkpoint, `.beads/`
trace files) are left untouched, and the orphan script *deletions* are left
for child 4 of bf-3o9 (deletions are NOT part of this bead).

## Files added

- `tests/comments_cli.rs` (new)
- `tests/search_command.rs` (new)
- `notes/bf-4waen.md` (this file)
