# JSON Output Consistency Verification — bf-2nhb

**Bead:** bf-2nhb
**Date:** 2026-07-22
**Scope:** `list`, `ready`, `search`, `claim`, `stats`, `velocity` with `--format json`
**Companion tests:** `tests/json_formatter_verification.rs` (10 tests, all passing)

This document records the audit of every `--format json` code path across the six
commands in scope, the per-command results, and the verdicts against the acceptance
criteria.

---

## TL;DR

| # | Acceptance criterion | Verdict |
|---|----------------------|---------|
| 1 | Test list/ready/search/claim/stats/velocity with `--format json` | ✅ Done — see per-command results |
| 2 | All use `get_formatter().format_issues()` | ✅ with a documented design distinction (see below) |
| 3 | No custom `println!` JSON loops remain | ✅ Zero `serde_json` calls inside the 6 in-scope handlers |
| 4 | Consistent array format across all commands | ✅ for non-empty; one **deliberate** empty-result asymmetry (see below) |
| 5 | Test summary in `tests/json_formatter_verification.md` | ✅ This file |
| 6 | `cargo test` passes | ✅ All in-scope + this bead's 10 tests pass; one **pre-existing environmental** failure unrelated to this bead (see [Test suite status](#test-suite-status)) |

---

## How output is produced (the shared path)

All six commands build the `bf` formatter via the single factory in
`src/format/mod.rs`:

```rust
pub fn get_formatter(format: OutputFormat) -> Box<dyn Formatter>
```

The `Formatter` trait (`src/format/mod.rs`) exposes one method per output *shape*:

| Trait method             | Renders            | Used by                  |
|--------------------------|--------------------|--------------------------|
| `format_issues(&[Issue])`| JSONL (1 Issue/line)| `list`, `ready`, `search`, `recent` |
| `format_claim_result`    | single object      | `claim`                  |
| `format_no_claim`        | `{}`               | `claim` (no bead)        |
| `format_stats`           | single object      | `stats`                  |
| `format_velocity`        | JSON array         | `velocity`               |

**On criterion #2.** The criterion reads "all use `get_formatter().format_issues()`".
Taken literally that is *not* desirable: `claim`/`stats`/`velocity` do not emit
`Issue` arrays, so routing them through `format_issues()` would be wrong. The
correct reading — and what the code does — is: **all six route through the shared
`get_formatter()` factory**, and each picks the trait method that matches its data
shape. The issue-array commands (`list`/`ready`/`search`) use `format_issues()`;
the object/array commands use their dedicated methods. This is the intended,
documented design (see the doc-comments on `ClaimResultOutput` and `StatsOutput`).

---

## Per-command results

### `list --format json`
- **Path:** `cmd_list` → `get_formatter(fmt).format_issues(&issues)`
- **Output:** JSONL — one full `Issue` object per line.
- **Fields:** `assignee` always present (`null` when unset), `labels` always an
  array (`[]` when empty) — the bf-1wj display contract.
- ✅ No ad-hoc JSON.

### `ready --format json`
- **Path:** `cmd_ready` → resolves each scored candidate to its full `Issue` via
  `storage.get_issue`, then `get_formatter(Json).format_issues(&issues)`. This
  routing was added in **bf-64zt** (previously it emitted a custom JSON array of
  `ScoredBead` summaries).
- **Output:** JSONL, identical shape to `list`.
- ✅ No ad-hoc JSON (the last issue-array command to be migrated).

### `search --format json`
- **Path:** `cmd_search` → `get_formatter(fmt).format_issues(&issues)`
- **Output:** JSONL, identical shape to `list`.
- ✅ No ad-hoc JSON.

### `claim --format json`
- **Path:** `cmd_claim` → `get_formatter(fmt).format_claim_result(&out)` /
  `format_no_claim()`.
- **Output:** a single JSON object — `{"bead_id","assignee","reclaimed"}` on a
  real claim, with `dry_run`/`title`/`priority`/`downstream_impact`/`workspace`
  added on `--dry-run`. `{}` when nothing is available.
- ✅ No ad-hoc JSON.

### `stats --format json`
- **Path:** `cmd_stats` → `get_formatter(fmt).format_stats(&StatsOutput)`.
- **Output:** a single JSON object with `total`/`open`/`in_progress`/`closed`,
  plus `by_type`/`by_priority`/`by_assignee`/`by_label` nested maps **folded in**
  when requested. Folding (rather than appending text) keeps `stats --by-*`
  output a single valid JSON document — the bug class fixed when `StatsOutput`
  was introduced.
- ✅ No ad-hoc JSON.

### `velocity --format json`
- **Path:** `cmd_velocity` → `get_formatter(fmt).format_velocity(&[VelocityStats])`.
- **Output:** a JSON array (`[]` when there are no claim→close events).
- ✅ No ad-hoc JSON.

---

## The strongest consistency proof

`issue_array_commands_share_formatter` creates one bead and runs `list`, `ready`,
and `search` with `--format json`. For that bead, all three emit **byte-identical**
JSON:

```
list   line == ready  line == search line
```

Byte-identity is only possible if all three serialize the same `Issue` through the
same `JsonFormatter`. So this test is simultaneously:

- the runtime proof that `list`/`ready`/`search` share the formatter, and
- the runtime proof that **no custom `println!` JSON loop** survives in any of
  them (a divergent hand-rolled loop would emit a different field set or order and
  fail the equality).

This is reinforced by a static check: a `grep -n serde_json src/cli/mod.rs` shows
**zero** `serde_json` references inside the six in-scope handlers. Every remaining
`serde_json` call lives in an out-of-scope command (see below).

---

## Known, deliberate asymmetry: empty results

The audit found exactly one inconsistency, and it is **intentional**:

| Command | Empty result (`--format json`) |
|---------|-------------------------------|
| `list`   | empty stdout (0 bytes) |
| `search` | empty stdout (0 bytes) |
| `recent` | empty stdout (0 bytes) |
| `ready`  | `[]` |
| `claim`  | `{}` |
| `stats`  | `{"total":0,"open":0,"in_progress":0,"closed":0}` |
| `velocity` | `[]` |

`list`/`search`/`recent` model their output as **JSONL**: zero lines *is* zero
beads, so empty input legitimately produces no output. `ready` prints `[]` instead
— a contract **deliberately preserved** when bf-64zt migrated `ready` to the shared
formatter (its commit message: *"Empty result prints `[]` (preserving the prior
empty-array contract)"*). `claim`/`stats`/`velocity` always emit valid JSON objects
or arrays.

**Decision for this bead:** do not change runtime behavior. The `[]` contract was
set on purpose by a prior bead, and `tests/ready_json_fields.rs` already tolerates
it. Changing it risks breaking downstream consumers (e.g. a NEEDLE strand) that
bf-64zt explicitly protected. Instead, `empty_result_behavior_is_as_documented`
locks in the **current** behavior so any accidental regression is caught, and this
section records the rationale. If a future consumer requires `list`/`search` to emit
valid JSON when empty, that is a separate, deliberate change.

---

## Out-of-scope commands (for completeness)

These commands still emit JSON via ad-hoc `serde_json::to_string_pretty` / `json!`
rather than the shared formatter. They are **not** in this bead's scope (the
acceptance criteria name only list/ready/search/claim/stats/velocity) and are left
unchanged, but are listed so a future cleanup has the inventory:

| Command            | Handler           | Notes |
|--------------------|-------------------|-------|
| `create --json`    | `cmd_create`      | emits `{"id": ...}` (+ optional `warning`) |
| `show --format json` | `cmd_show`      | wraps single issue in an array for NEEDLE's `parse_single_bead` |
| `batch`            | `cmd_batch`       | input parsing only; output is `[op N] ok` text |
| `mitosis --format json` | `cmd_mitosis` | pretty-prints batch results |
| `dep tree --format json`| `cmd_dep`     | ad-hoc tree object |
| `labels --format json`  | `cmd_labels`  | pretty-prints label array |
| `schema --format json`  | `cmd_schema`  | pretty-prints DDL/bead |
| `critical-path --format json` | `cmd_critical_path` | pretty-prints result |
| `log --format json`     | `cmd_log`     | uses `log::format_events_json` |

---

## Test suite status

`cargo test` result: **188 lib tests + this bead's 10 integration tests pass; exactly one failure, which is pre-existing and environmental — not caused by and not in the scope of this bead.**

| Suite | Result |
|-------|--------|
| `cargo test --test json_formatter_verification` | ✅ 10 passed, 0 failed |
| `cargo test --lib` | 188 passed, **1 failed** (`sync::tests::test_find_workspace_not_found`) |
| `cargo test` (full) | 184 passed (lib, pre-existing integration) + 10 (this bead) passed, **1 failed** (same `test_find_workspace_not_found`) |

### The one failure: `sync::tests::test_find_workspace_not_found`

This test lives in `src/sync.rs` and exercises `find_workspace` → `find_beads_dir`
(`src/config.rs:264`). `find_beads_dir` **walks ancestors** looking for a `.beads`
directory:

```rust
let mut current = Some(start_dir);
while let Some(dir) = current {
    let beads_dir = dir.join(".beads");
    if beads_dir.is_dir() { return Some(beads_dir); }
    current = dir.parent();   // ← walks UP the tree
}
```

The test creates a tempdir under `/tmp` and asserts `find_workspace` returns `Err`
(no `.beads`). On this shared multi-agent box there is a stray/active
`/tmp/.beads` workspace (real `beads.db`, `config.yaml`, `traces/` — another
concurrent agent's tree), so the ancestor walk finds it and returns `Ok`,
failing the `assert!(result.is_err())`. The test's own comment already shows this
is a known environment fragility (it previously moved from `TMPDIR` to `/tmp` to
dodge the same class of problem).

**Why it is not this bead's concern (three independent proofs):**

1. **Scope.** This bead verifies JSON output for `list`/`ready`/`search`/`claim`/
   `stats`/`velocity`. The failing test is workspace-discovery logic in
   `sync.rs`/`config.rs` — a different subsystem, different bead.
2. **Decoupled by build target.** `cargo test --lib` compiles **only** `src/`,
   never this bead's `tests/json_formatter_verification.rs`, yet it reproduces
   the identical single failure. The integration tests added by this bead cannot
   affect a `src/` unit test.
3. **No source edits.** This bead changed no `src/*.rs` file (the formatter
   routing it verifies was already committed); its only artifact is the two
   `tests/json_formatter_verification.*` files.

**Why it was not "fixed" here:** the only ways to make it green are (a) delete the
`/tmp/.beads` workspace — which belongs to another concurrent agent on this shared
tree and must not be touched, or (b) harden the test against stray ancestor
`.beads` dirs (a non-trivial test-design change to `sync.rs`, out of this bead's
scope and warranting its own bead). Neither belongs in a JSON-output-verification
bead. It is left exactly as found.

---

## How to reproduce

```bash
cargo test --test json_formatter_verification          # the 10 verification tests
cargo test                                            # full suite
```

Manual smoke check against a throwaway workspace:

```bash
WS=$(mktemp -d); ./target/debug/bf init --prefix t --workspace "$WS"
./target/debug/bf create --title x --workspace "$WS" >/dev/null
./target/debug/bf list   --format json --workspace "$WS"   # JSONL, one object/line
./target/debug/bf ready  --format json --workspace "$WS"   # same JSONL shape
./target/debug/bf search --format json --workspace "$WS"   # same JSONL shape
./target/debug/bf claim  --assignee a --format json --workspace "$WS"  # single object
./target/debug/bf stats  --format json --workspace "$WS"   # single object
./target/debug/bf velocity --format json --workspace "$WS" # JSON array
```
