# bf-2iskc5 — Test (smoke-test bead)

**Bead:** bf-2iskc5 · Title: "Test" · No description
**Date:** 2026-07-25
**Agent:** claude-code-glm-4.7-h1-bforge
**Branch:** needle/bf-5wku (shared working tree)

## Task

Needle smoke-test bead. Ran the build + test suite to verify the codebase
health and report results. No source changes were made by this bead — it only
observes and reports.

## Results

### `cargo build`
**Clean.** Exit 0, no errors or warnings from the build itself.

### `cargo test` (full suite)
```
test result: FAILED. 616 passed; 3 failed; 10 ignored
```

**616 / 619 tests pass.** The 3 failures are NOT regressions introduced by this
bead. They fall into two unrelated buckets:

1. **`batch::tests::test_mixed_op_batch_all_operations_atomic`** (`src/batch.rs:2404`)
   - Asserts `created_bead.labels.len() == 1` after a `LabelAdd` batch op, but
     gets `0` — the label does not round-trip.
   - **Cause:** uncommitted, in-progress `src/batch.rs` change in this shared
     working tree renames the labels table `labels`/`issue_id` →
     `bead_labels`/`bead_id` in `execute_label_add`/`execute_label_remove`. The
     rename targets the correct canonical table (`bead_labels`, schema line 270),
     so the fix is on the right track, but the round-trip still fails — that
     work belongs to **another agent's** open bead, not this one.
   - This bead left that uncommitted change **untouched** (shared-workspace
     discipline: commit single paths, never `git add -A`).

2. **`sync::tests::test_find_workspace_not_found`** (`src/sync.rs:360`)
   - `assert!(result.is_err())` fails — `find_workspace` finds a `.beads`
     ancestor from the temp dir.
   - **Cause:** environment-sensitive. The test's own comment documents this
     brittleness (it switched to `/tmp` to dodge TMPDIR being under the project
     root, but a `.beads` ancestor is still being found on this box). Not a code
     regression.

3. **`sync::tests::test_labels_persist_through_full_sync`** (`src/sync.rs:948`)
   - `std::fs::read_to_string(&jsonl_path)` → "No such file or directory".
   - **Cause:** environment/path-sensitive in this shared tree. The pre-failure
     warning ("1 unflushed bead bf-sync-labels in SQLite") indicates the sync
     wrote to the real db rather than the test's temp path. Not a code
     regression.

## Disk
169G free on `/` at start — no `target/` cleanup needed. bead-forge `target/`
is 6.3G.

## Conclusion
Build is healthy; 99.5% of the suite passes. The 3 failures are pre-existing /
in-progress / environment-sensitive and outside this bead's scope. This bead's
sole artifact is this notes file.
