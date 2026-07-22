# bf-4p4v5 — Cross-check: every required read-only command is pinned (no gap, no leak)

Child 2/4 of bf-bziwd (depends on child 1, bf-5ovqz). Task: cross-check the
REQUIRED read-only command list against what the two existing snapshot tests
actually assert, and fill any gap / fix any leak child 1 flagged.

## Outcome: no gap to fill, no leak to fix

I independently re-verified the CLI and tests (did not take child 1's audit or
the prior notes on faith). **Every** command in the required list is already
asserted by a passing test with the dual invariant (byte-identical content **and**
unchanged mtime). Child 1's static audit found every read-only/diagnostic handler
CLEAN — none reach `autoflush::after_mutation_with_config`, `autoflush::after_delete`,
`autoflush::enabled`, `export_jsonl`, `export_jsonl_dirty`, or `sync_to_jsonl` on a
healthy workspace — and I confirmed that independently below. So there is no code
change to make. Adding redundant test cases would exercise clap arg-parsing, not
the JSONL-write invariant — noise, not coverage. This is the expected result for
this bead.

## Why "rejected" is the correct pin (not a missing-feature bug)

The required list includes `status`, `sync --status`, `doctor --json`. These are
**not currently `bf` commands** — but that is intentional, not a gap. Verified two
ways:

1. **CLI reality** (`src/cli/mod.rs`, read directly):
   - No `Status` variant in the `Commands` enum → `bf status` is clap-rejected.
   - `Sync` (mod.rs:309) exposes only `--flush-only` / `--import-only` → `bf sync
     --status` rejected.
   - `Doctor` (mod.rs:325) exposes only `--repair`, `--flush-first`, `--force`,
     `--reclaim-stale`, `--ttl`, `--fix-schema` → `bf doctor --json` rejected.

2. **Plan intent** (`docs/plan/plan.md` §7.4, P1 — not yet built):
   - `bf doctor --json` is where the planned `AnomalyClass` enum gets surfaced.
   - `bf sync --status` is the planned "In sync" verdict.

So the test that asserts these are *rejected* AND leave JSONL untouched
(`unknown_readonly_invocations_leave_jsonl_untouched`) is a deliberate forcing
function: when §7.4 implements them, that test will fail and the implementer must
flip it to assert success + JSONL-still-untouched. Pinning current behavior here
is the correct regression guard against a future read-path silently writing JSONL.

## The `--json` qualifier — fully covered, same code path

`--json` is documented in the struct as "alias for --format json" (mod.rs `List`
variant), and every read-only handler folds it identically:

```rust
let format = if json { "json".to_string() } else { format };
```

at mod.rs:1069, 1075, 1122, 1137, 1244, 1289, 2460. So `list --json` and `list
--format json` resolve to the same `format` value and the same output branch. The
existing test exercises the JSON path via `--format json` (autoflush_readonly.rs:144,
:146); a separate `--json` case would only re-test clap's alias parsing, not the
write invariant. `doctor --json` specifically is pinned as rejected (above).

## Coverage matrix (required list → exact assertion)

Dual invariant = `assert_unchanged` / `assert_jsonl_unchanged`, both of which
assert byte-identical content AND identical mtime vs the pre-command snapshot.

### Query commands — `readonly_commands_never_write_jsonl` (autoflush_readonly.rs)

Seeded workspace (2 beads, dep, label, comment, annotation; each mutation
auto-flushed → canonical JSONL). One snapshot, then the parametric loop runs each
command, asserts it succeeded, and asserts the invariant.

| Required cmd | Case (file:line) | Invariant |
|---|---|---|
| `list`            | autoflush_readonly.rs:143 | assert_unchanged:176 |
| `show`            | autoflush_readonly.rs:145 | assert_unchanged:176 |
| `ready`           | autoflush_readonly.rs:147 | assert_unchanged:176 |
| `critical-path`   | autoflush_readonly.rs:148 | assert_unchanged:176 |
| `velocity`        | autoflush_readonly.rs:149 | assert_unchanged:176 |
| `doctor` (default)| autoflush_readonly.rs:150 | assert_unchanged:176 |
| `labels`          | autoflush_readonly.rs:151 | assert_unchanged:176 |
| `comments list`   | autoflush_readonly.rs:164 | assert_unchanged:176 |

`doctor` is *additionally* pinned in the sharp case
`doctor_does_not_flush_even_with_unflushed_beads` (autoflush_readonly.rs:185):
seeds a db-only "ghost" bead and proves `doctor` leaves JSONL untouched AND does
not silently flush the ghost.

### Doctor write-flags + commit-check — `autoflush_diagnostics_and_rotation.rs`

| Required cmd | Case (file:line) | Invariant |
|---|---|---|
| `doctor --repair` (healthy)        | :130 | assert_jsonl_unchanged:139 |
| `doctor --fix-schema` (sibling)    | :131 | assert_jsonl_unchanged:139 |
| `doctor --reclaim-stale` (sibling) | :132 | assert_jsonl_unchanged:139 |
| `commit-check`                     | :169 | assert_jsonl_unchanged:171 |

### Rejected-unknown commands — `unknown_readonly_invocations_leave_jsonl_untouched`

| Required cmd | Case (file:line) | Invariant |
|---|---|---|
| `status`        | :188 | `!success` + assert_jsonl_unchanged:199 |
| `sync --status` | :189 | `!success` + assert_jsonl_unchanged:199 |
| `doctor --json` | :190 | `!success` + assert_jsonl_unchanged:199 |

These assert clap **rejects** the invocation (non-zero exit, no handler runs)
**and** leaves JSONL untouched — pinning the contract so a future command
addition can never silently introduce a read-path that writes JSONL.

## Independent leak re-check (not relying on child 1)

grep of every autoflush/export entry point call site lands only in mutating
handlers (create, update, close, reopen, delete, claim, batch, mitosis, dep
add/remove, label add/remove, comments add, annotate set/remove/clear). No
read-only handler from the required list appears. The one JSONL-write-reachable
line anywhere in a diagnostic — `doctor --repair --flush-first` reaching
`storage.sync_to_jsonl` (doctor.rs:647) — is an explicit, user-requested,
error-guarded flush outside the healthy-workspace scope; correct by design, and
not in this bead's scope.

## Validation

```
cargo build  → exit 0, no errors

cargo test --test autoflush_readonly --test autoflush_diagnostics_and_rotation
  autoflush_diagnostics_and_rotation: 4 passed; 0 failed
  autoflush_readonly:                 2 passed; 0 failed
  → 6 passed; 0 failed
```

All acceptance criteria met:
- Every command in the required read-only list is asserted by a passing
  byte-identical + mtime-unchanged test case. ✓
- No leak to fix (child 1 found none; re-confirmed here against the CLI). ✓
- Both test files pass; cargo build clean. ✓
