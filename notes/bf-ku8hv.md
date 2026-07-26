# bf-ku8hv: doctor --repair --flush-first must not write JSONL on a healthy workspace

Task: a read-only diagnostic operation (`bf doctor --repair --flush-first`
that repairs nothing) was performing a write to the JSONL checkpoint,
violating the read-only contract. Fix the write path and add a regression
test.

## Fix

`src/doctor.rs`, `repair_stack()` healthy early-return branch (was lines
~1007-1023). The old code, when `--flush-first` was set and unflushed beads
were present on a healthy workspace, called `storage.sync_to_jsonl(...)` —
a write to the checkpoint even though **no rebuild was needed**.

The branch now:

- sets `report.healthy = true` and clears any stale repeat-failure marker,
- never calls `sync_to_jsonl` — the `--flush-first` flag is scoped to the
  rebuild ("flush unflushed beads *before* repair"); with no rebuild pending
  there is nothing to protect,
- when unflushed beads are present, points the user at the canonical
  checkpoint command (`bf sync --flush-only`) instead of silently writing.

The only remaining `flush_first → sync_to_jsonl` write is at the rebuild
path (`src/doctor.rs:1029`), which is unreachable when the workspace is
healthy (the early return fires first). So `--flush-first` still does its
job when a rebuild is genuinely needed.

## Regression test

`tests/doctor_safety_stack.rs::healthy_repair_with_flush_first_does_not_write_jsonl`:

- builds a healthy workspace carrying one unflushed bead,
- runs `repair_stack` with `flush_first: true`,
- asserts the JSONL checkpoint is byte-identical before/after,
- asserts the unflushed bead is still dirty (nothing was flushed for it),
- asserts the report advises `bf sync --flush-only` and does NOT report a
  flush.

## Audit — read-only commands have no JSONL write paths

Acceptance criterion: "all read-only commands have been audited and
verified to have no write paths."

`sync_to_jsonl` is the only JSONL write primitive. A full scan of `src/`
finds exactly **two** call sites, both on the doctor rebuild path:

| Call site               | Function          | Reachable from CLI? | Read-only? |
|-------------------------|-------------------|---------------------|------------|
| `src/doctor.rs:649`     | legacy `repair()` | NO — tests only     | n/a        |
| `src/doctor.rs:1029`    | `repair_stack()`  | yes, rebuild path   | no (write) |

Neither is on a read-only path. `doctor.rs:1029` is only reached when a
rebuild is genuinely needed (after the healthy early return). The legacy
`repair()` at `:649` is not wired to any CLI command — `bf doctor --repair`
dispatches through `repair_stack` (`src/cli/mod.rs:2306`); the legacy fn is
exercised only by `tests/doctor_repair_unflushed.rs` and `test_bf_2hqt.rs`.

`cmd_doctor` branch audit (`src/cli/mod.rs:2238`):

| Branch            | Handler                     | Writes JSONL? |
|-------------------|-----------------------------|---------------|
| `bf doctor` (no flag) | `doctor::check()`       | NO (read-only)|
| `bf doctor --runs`    | `recovery::list_runs()` | NO (read-only)|
| `bf doctor --repair`  | `doctor::repair_stack()`| NO when healthy (this fix) |
| `--restore` / `--fix-schema` / `--reclaim-stale` | … | intentionally mutating, gated behind their own flags — not read-only diagnostics |

The pure read-query commands — `list`, `show`, `ready`, `count`,
`search`, `stats`, `schema`, `velocity`, `labels`, `recent`, `log`,
`critical-path`, `commit-check` — never call `sync_to_jsonl` (no call
site outside doctor.rs exists), so they cannot write the JSONL checkpoint.

Note: the healthy path does call `clear_repair_failed_marker`
(`src/doctor.rs:1012`), a best-effort delete of a stale marker file. On a
truly healthy workspace that marker does not exist, so it is a no-op; it is
not a JSONL checkpoint write and is excluded from the read-only contract by
the byte-identity assertion in the regression test.

## Verification

- `cargo build` — clean, 0 errors.
- `cargo test --test doctor_safety_stack` — 24 passed, 0 failed (incl. the
  new `healthy_repair_with_flush_first_does_not_write_jsonl`).

## Files

- `src/doctor.rs` — removed `sync_to_jsonl` write from the healthy path.
- `tests/doctor_safety_stack.rs` — regression test for bf-ku8hv.
- `notes/bf-ku8hv.md` — this file (audit record).
