# bf-5ovqz — Audit: read-only/diagnostic commands never reach a JSONL write path

Pure static audit (child 1/4 of bf-bziwd). No code or test changes — every
command is CLEAN, so nothing to fix (child 2 owns any leak, but none found).

## What "JSONL write path" means here

The audit-listed write entry points and where they are *actually* invoked:

| Entry point | Reached only from |
|---|---|
| `autoflush::after_mutation_with_config` | local helper `autoflush_after_mutation` (src/cli/mod.rs:1355) |
| `autoflush::after_delete` | local helper `autoflush_after_delete` (src/cli/mod.rs:1369) |
| `autoflush::enabled` | the two helpers above + `cmd_claim` (src/cli/mod.rs:1812) |
| `autoflush::after_mutation` | `cmd_claim` flush closure (src/cli/mod.rs:1814) |
| `export_jsonl` / `export_jsonl_dirty` / `export_jsonl_merge` | `sync::flush` / `sync::flush_dirty` / `sync::flush_after_delete` (src/sync.rs:56,115,154) |
| `sync_to_jsonl` (storage method) | `doctor::repair` flush-first branch (src/doctor.rs:647) |

grep of every call site (`after_mutation_with_config`, `after_mutation`,
`after_delete`, `enabled`, the two `autoflush_*` helpers) lands **only** in the
mutating handlers: create, update, close, reopen, delete, claim, batch, mitosis,
dep add/remove, label add/remove, comments add, annotate set/remove/clear. None
of the read-only handlers below appear in that list.

## Per-command verdicts

### Read-only query commands — all CLEAN (pure SELECT / read)
- `list` (cmd_list, :1464) — `list_issues` / `list_all_with_archives`. **CLEAN**
- `show` (cmd_show, :1546) — `get_issue` + archive fallback. **CLEAN**
- `ready` (cmd_ready, :1736) — `get_ready_candidates` (SELECT scoring). Uses
  `with_immediate_transaction` only as a consistent read snapshot — **no write,
  no flush**. **CLEAN**
- `critical-path` (cmd_critical_path, :3127) — `compute_epic_critical_path`
  (read-only graph traversal under a read tx). **CLEAN**
- `velocity` (cmd_velocity, :2928) — `get_velocity_stats` (read query). **CLEAN**
- `labels` (cmd_labels, :2578) — `get_labels`. **CLEAN** (also `label list`,
  cmd_label :2556 — `get_labels`/`list_all_labels`, read-only)
- `comments list` (cmd_comments List, :2605) — `list_comments`. **CLEAN**
  (note: `comments add` :2602 DOES flush — out of scope, mutating)
- `count` (cmd_count, :2207) — `list_issues`/`count_issues`. **CLEAN**
- `search` (cmd_search, :2622) — `search_issues`. **CLEAN**
- `stats` (cmd_stats, :2665) — `get_stats`/`get_stats_by_*`/`list_all_labels`. **CLEAN**
- `log` (cmd_log, :2994) — `query_events`; `--git` adds
  `reconstruct_events_from_git` which **reads** JSONL via git history, no write. **CLEAN**
- `recent` (cmd_recent, :3292) — `list_issues`. **CLEAN**
- `schema` (cmd_schema, :2717) — prints `SCHEMA_SQL` constant, or for a bead ID
  `get_issue`/`get_annotations` (read). **CLEAN**

### `commit-check` — CLEAN
- `cmd_commit_check` (:2194) → `scan_staged_beads` (reads staged git files,
  scans for secrets) → prints/exits. commit_check.rs references JSONL only as a
  *file being scanned*; never `sync`/`flush`/`autoflush`/`export_jsonl`. **CLEAN**

### `doctor` — CLEAN on a healthy workspace (the stated scope)
- default health check → `doctor::check` (:65): PRAGMA integrity, `stream_issues`
  (reads JSONL), `check_consistency_with_hash` (reads both), `count_unflushed`.
  Pure read. **CLEAN**
- `--fix-schema` → `doctor::fix_null_not_null` (:344): raw SQLite `Connection`,
  in-place `UPDATE … WHERE typeof(col)='null'`. Writes SQLite only; **no JSONL
  write, no autoflush, no export**. **CLEAN**
- `--reclaim-stale` → `doctor::reclaim_stale` (:748): single in-place
  `UPDATE issues SET status='open', assignee=NULL …` under a tx. Writes SQLite
  only; **no JSONL write, no autoflush**. **CLEAN**
- `--repair` → `doctor::repair` (:593): on a healthy workspace `get_unflushed_ids`
  returns empty, so the guarded flush block (:638-685) is skipped and
  `storage.sync_to_jsonl` (:647) is **never reached**; repair then backs up the
  db, drops it, and rebuilds via `import_jsonl` (reads JSONL → writes SQLite). **CLEAN**

  Flagged for completeness (NOT a leak, outside the healthy-workspace scope):
  `doctor --repair --flush-first` with unflushed beads present reaches
  `storage.sync_to_jsonl` (:647). This is an explicit, user-requested,
  error-guarded flush (it refuses to run against a corrupt db), not an autoflush
  leak — correct by design. It is the only JSONL-write-reachable line anywhere in
  a diagnostic command, and every other `export_jsonl`/`sync::flush` reference in
  doctor.rs is `#[cfg(test)]` only.

### Not `bf` subcommands (clap rejects — confirmed against the `Commands` enum)
- `bf status` — no `Status` variant exists. **rejected**
- `bf sync --status` — `Sync` has only `--flush-only`/`--import-only`. **rejected**
- `bf doctor --json` — `Doctor` has no `--json` flag (only repair, flush_first,
  force, reclaim_stale, ttl, fix_schema). **rejected**

## Conclusion
Every read-only/diagnostic command is CLEAN: none reach
`autoflush::after_mutation_with_config`, `autoflush::after_delete`,
`autoflush::enabled`, `export_jsonl`, `export_jsonl_dirty`, or `sync_to_jsonl`
on a healthy workspace. The expected outcome holds — no fix needed (child 2 has
nothing to do from this audit).
