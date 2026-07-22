# bf-1dcws: 7.9 Multi-box & fleet concurrency hardening

Spec: `docs/plan/plan.md` §7.9 (P2). All three parts implemented in one loop.

## 1. Merge anchor + three-way JSONL merge

**New module `src/merge.rs`.** A plain git *text* merge of `issues.jsonl` is
hazardous — each line is a whole bead, so a conflict marker corrupts JSON and a
"take theirs" silently drops beads created on the other box (the recurring
lab/ex44 divergence). `merge_maps()` merges **per-bead, id-keyed** against a
common ancestor:

- one-sided edit → take the changed side
- identical edit on both sides → take it, no conflict
- both sides diverged → deterministic last-writer-wins by `updated_at`, ties
  broken by content hash (so the winner is independent of side order / which
  box runs the merge — covered by `merge_is_order_independent`)
- delete racing a concurrent modify → **keep the modification** (never silently
  lose in-flight work — `merge_delete_vs_modify_keeps_modification`)
- missing base → safe union (nothing deleted)

Output is sorted by id (stable diffs) and written atomically (temp + rename).

**CLI: `bf merge-jsonl --ours %A --theirs %B [--base %O] [--output %A]`** —
handled before the `.beads`-directory requirement so it works as a git merge
driver. Base defaults to the local merge anchor. Always exits 0 (no markers are
ever emitted; every divergence is auto-resolved). Git driver wiring is
documented in `docs/README.md`.

**Merge anchor `.beads/beads.base.jsonl`** — `merge::update_base_anchor()` is
called from `sync::flush()` and `sync::import()` so the anchor always mirrors
the last state this box agreed on with the artifact. It is the fallback 3-way
base when git doesn't supply `%O` (out-of-band cross-checkout merges).
Local-only, git-ignored.

## 2. Concurrent-writer hardening

**New test `tests/fleet_concurrency.rs`** spawns N concurrent `bf` *processes*
(not threads sharing one `Storage`) and asserts the upstream #109/#191 bug
classes stay dead:

- `fleet_concurrent_creates_no_silent_loss` — 12 workers × 4 creates; `count`
  must equal 48 (no parallel-write loss, no ID collide-and-overwrite).
- `fleet_creates_survive_flush_and_reimport` — fleet writes, flush to JSONL,
  wipe `beads.db`, `sync --import-only`; every bead must round-trip. Also
  asserts the merge anchor gets created.
- `fleet_concurrent_claims_no_double_claim` — 20 workers herd 15 beads; no bead
  claimed twice, exactly 15 claimed, surplus workers get nothing.

All three pass. The existing `BEGIN IMMEDIATE` + `SQLITE_BUSY` backoff held up
under process-level contention — no code fix was needed, the tests lock the
guarantee in.

## 3. Pre-export history backups

**New module `src/history.rs`.** `backup_before_export()` copies the outgoing
`issues.jsonl` into `.beads/.bf_history/issues-<ts>-<ns>.jsonl` before the new
one atomically replaces it, then prunes to the newest `max_backups` snapshots.
Wired into `sync::flush()` as best-effort (a failed backup logs a warning but
never aborts the flush it protects). New `HistoryConfig` in `config.rs`:
`enabled` (default true), `max_backups` (default 20; 0 = unbounded).
Local-only, git-ignored.

## Files

- new: `src/merge.rs`, `src/history.rs`, `tests/fleet_concurrency.rs`
- edited: `src/sync.rs` (backup + anchor hooks), `src/config.rs` (HistoryConfig),
  `src/cli/mod.rs` (`merge-jsonl` subcommand + handler), `src/lib.rs` (exports),
  `.gitignore` (`.bf_history/`, `beads.base.jsonl`), `docs/README.md`.

## Validation

- `cargo build` — clean, 0 warnings.
- New tests: 9 (merge) + 7 (history) + 3 (fleet) all pass.
- `cargo test --lib` — 153 pass. The single failure
  (`sync::tests::test_find_workspace_not_found`) is pre-existing and
  environmental: a leaked `/tmp/.beads` from earlier test runs makes
  `find_workspace` walk up and succeed. It fails identically on HEAD and is
  unrelated to this change.
