# bf-1vch24 — Test (verification)

**Task:** `Test` (label `test`). Interpreted as: run the bead-forge test suite and
report the state of the build + tests. Verification bead — documents findings only,
no production code changed.

## Method

1. `cargo build` — clean, no errors.
2. `cargo test` (lib) — 616 passed, **3 failed**, 10 ignored.
3. To attribute root cause, the 3 failures were re-run in an isolated `git worktree`
   checked out at committed `HEAD` (i.e. *without* the uncommitted `src/batch.rs` WIP
   that was sitting in the shared working tree). All 3 failed identically there, so
   **all 3 failures are pre-existing** — none are introduced by the uncommitted WIP.

## Build

`cargo build` → clean (exit 0), no errors.

## Test result: 616 / 619 passing (99.5%)

Three failures, all pre-existing and deterministic (reproduce identically at committed
HEAD, independent of the uncommitted `batch.rs` change):

### 1. `batch::tests::test_mixed_op_batch_all_operations_atomic` — REAL BUG (pre-existing)

Panics at `src/batch.rs:2404`: `assert_eq!(created_bead.labels.len(), 1)` → left `0`,
right `1`. A bead created via a batch `Create` op (with `labels: ["batch-created"]`)
reads back with **0 labels**.

**Root cause — writer/reader table mismatch:**

| Path | Table used |
|------|-----------|
| `execute_create` writes Create-op labels — `src/batch.rs:569-572` | legacy `labels` |
| `execute_label_add` / `execute_label_remove` (LabelAdd/LabelRemove ops) | `bead_labels` |
| `load_labels_conn` reads labels for `get_issue` — `src/storage/sqlite.rs:1040` | `bead_labels` |

The schema declares **both** `labels` (`schema.rs:116`) and `bead_labels`
(`schema.rs:270`). `execute_create` still inserts into the legacy `labels` table, but
`get_issue` reads from `bead_labels` → the just-created labels are invisible.

**Note on the uncommitted WIP:** the `src/batch.rs` change in the working tree (belonging
to **bf-5wku**, not this bead) is an *incomplete fix attempt* for this exact bug — it
migrated `execute_label_add`/`execute_label_remove` to `bead_labels` but did **not**
touch `execute_create` (line 569-572), which still writes to `labels`. The test stays red
with the WIP applied. **One-line fix to finish it:** route `execute_create`'s label insert
to `bead_labels` (`INSERT INTO bead_labels (bead_id, label) …`). Not applied here —
`batch.rs` is another agent's uncommitted territory in this shared workspace.

### 2. `sync::tests::test_find_workspace_not_found` — environment-specific

Panics at `src/sync.rs:360`: `assert!(result.is_err())` fails — `find_workspace` returns
`Ok` from a temp dir created under `/tmp`. The test's own comment flags this as
environment-sensitive (it already swapped from `TMPDIR` to `/tmp` to dodge
`/home/coding/.beads` being found by the upward walk). Still trips on this box —
`find_workspace` walks up from the temp dir and resolves a `.beads` ancestor. Pre-existing,
not related to the `batch.rs` change.

### 3. `sync::tests::test_labels_persist_through_full_sync` — pre-existing test failure (+ DB leak)

Panics at `src/sync.rs:948`: `sync(workspace).unwrap()` → `Err(No such file or directory)`.
Deterministic, reproduces in isolation (not test-ordering pollution). Additionally emits a
warning that a bead `bf-sync-labels` is unflushed in the **real** workspace
`.beads/beads.db` — i.e. the test leaks a bead into the live store rather than isolating to
its temp workspace (test-isolation defect). The failure itself (`sync()` returning
`NotFound`) is a separate pre-existing issue.

## Conclusion / recommendation

- Build is green; 99.5% of tests pass.
- The only *code* defect surfaced is **#1** (batch Create-op labels lost to the legacy
  `labels` table). It is already being addressed by bf-5wku's in-flight `labels →
  bead_labels` migration; that work just needs to also cover `execute_create`
  (`src/batch.rs:569-572`). Pointed out here, not fixed, to avoid editing another agent's
  uncommitted file in the shared tree.
- #2 and #3 are pre-existing test/environment issues, independent of the WIP.

No source changes made by this bead — verification only.
