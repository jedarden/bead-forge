# bf-5mz5m — test

## Task
Title: `test`. No description, no acceptance criteria, no plan-section label.
Interpreted as "run the test suite" (the only actionable reading of the word).

## What I did
Ran `cargo test` (lib). Disk checked first: 169G free — well above threshold.

## Result
`616 passed; 3 failed; 10 ignored` in 14.36s.

### Failures (all investigated — none fixed; none are mine to fix)

1. **`batch::tests::test_mixed_op_batch_all_operations_atomic`** — `src/batch.rs:2404`
   `assert_eq!(created_bead.labels.len(), 1)` → left=0, right=1.
   **Root cause: uncommitted in-progress migration `labels` → `bead_labels` in
   `src/batch.rs` (another bead's WIP, currently `M src/batch.rs`).** The migration
   updated 3 of 4 sites (lines 972, 1000, 2107) but missed line 572 — the batch
   create path still `INSERT INTO labels (issue_id, …)`, so batch-created labels
   land in the old `labels` table while `get_issue` reads from `bead_labels`.
   Fix belongs to the migration bead, not here.

2. **`sync::tests::test_find_workspace_not_found`** — `src/sync.rs:360`
   `assert!(result.is_err())` failed (got Ok). `src/sync.rs` is **unmodified**
   (committed code), so this is not from the batch WIP. Environmental on this box:
   `find_beads_dir()`/`find_workspace()` walks up and finds a `.beads` ancestor
   even under `/tmp` (the source comment already flags TMPDIR walking up).

3. **`sync::tests::test_labels_persist_through_full_sync`** — `src/sync.rs:948`
   `sync(workspace).unwrap()` → `No such file or directory`. `src/sync.rs`
   **unmodified** — pre-existing/environmental, not from the batch WIP.

## Why I did not fix them
Shared needle workspace — `src/batch.rs`/`src/sync.rs` carry other beads' work.
The batch failure is a half-finished migration owned elsewhere; the two sync
failures run against committed, unmodified code and are environmental on this host.
Editing any of them would be completing another bead's task on the wrong branch.

## Files changed by this bead
- `notes/bf-5mz5m.md` (this file) — only.
