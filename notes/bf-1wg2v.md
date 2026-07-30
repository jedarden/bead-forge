# bf-1wg2v — 7.1 Incremental auto-flush + dirty_issues tracking

## Outcome: already implemented and verified (no source changes needed)

This bead (umbrella, Phase 7.1) was re-dispatched by needle on a stale
failure-count. The implementation was already landed and committed (see
`6a8a589 test(bf-1wg2v): verify Phase 7.1 auto-flush implementation complete`).
This pass re-verified the full spec against the code and the test suite.

## Verification (2026-07-22)

`cargo build` — clean. Auto-flush test suites, all green:

| Suite | Result |
|---|---|
| autoflush_wiring | 8 passed |
| kill_worker_preserves_beads | 4 passed |
| recovery_and_exit_criteria | 3 passed |
| autoflush_failure_contract | 12 passed |
| autoflush_readonly | 2 passed |
| autoflush_diagnostics_and_rotation | 11 passed |
| autoflush_batch_claim_delete | 24 passed |
| autoflush_mutation | 7 passed |
| doctor_repair_unflushed | 5 passed |

Total: 76 passed, 0 failed.

## Spec → implementation map

- **dirty_issues table (issue_id, marked_at)** — `src/storage/schema.rs:173`
  (`CREATE TABLE IF NOT EXISTS dirty_issues`, plus `idx_dirty_issues_marked_at`).
- **Every mutation marks dirty + best-effort incremental export of dirty rows
  only** — `src/autoflush.rs` (`after_mutation`, `after_delete`,
  `after_mutation_with_config`); surgical JSONL line replacement in `src/jsonl.rs`
  (not a full rewrite). Covered by `autoflush_mutation`, `autoflush_wiring`.
- **Failure → warn on stderr + `warning` field in --json envelope, keep dirty
  marks, never fail the mutation** — `FlushOutcome` (`warning()`, `is_failure()`)
  + `src/format/warning.rs`. Covered by `autoflush_failure_contract`.
- **Recovery: `bf sync --flush-only` clears dirty set** — `src/sync.rs`.
  Covered by `recovery_and_exit_criteria`.
- **Config `sync.auto_flush` default true; `--no-auto-flush` override** —
  `src/config.rs` + `autoflush::enabled()`. Covered by `autoflush_wiring`.
- **`bf batch` flushes ONCE at transaction end** — `src/batch.rs`. Covered by
  `autoflush_batch_claim_delete`.
- **Read-only/diagnostic commands never write JSONL** — covered by
  `autoflush_readonly`.
- **Exit criterion: kill worker between mutation and flush, lose nothing** —
  covered by `kill_worker_preserves_beads`.
- **Rotation interplay (Open Question): incremental flush targets only the
  active issues.jsonl, never archives** — covered by
  `autoflush_diagnostics_and_rotation`.
- **flush-before-repair ritual now unnecessary** — `doctor_repair_unflushed`
  confirms doctor handles unflushed state.

All Phase 7.1 exit criteria are met. No code change required this pass.
