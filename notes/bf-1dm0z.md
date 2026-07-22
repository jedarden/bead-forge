# bf-1dm0z: Test Epic P2

## Implementation Status: ✅ VERIFIED (no code changes needed)

This is a test bead (`type: epic`, `priority: P2`) exercising the **explicit-P2 epic creation**
path through `bf`. The feature is already fully implemented — this bead confirms it works
end-to-end against the installed `bf 0.3.0` binary, on top of existing library-level test coverage.

## What "epic P2" means here

When a user runs `bf create --type epic --title "..." --priority 2`, the resulting epic should
store priority **2 (P2 / Medium)** — the canonical "Medium" priority. This is the *explicit* path,
distinct from `Test epic default priority` (bf-3mvas), which exercises the *omission* path
(`bf create --type epic` with no `--priority`, falling back to clap's default). Both converge on
P2, but they hit different code:

- **Explicit P2 (this bead):** user passes `--priority 2`; `cmd_create` applies it directly via
  `issue.priority = Priority(priority);` (`src/cli/mod.rs:1099`). The value `2` is *not* a fallback
  artifact — it was supplied verbatim.
- **Default (bf-3mvas):** no `--priority`; clap substitutes `default_value = "2"`
  (`src/cli/mod.rs:49`).

At the model level, `Priority::MEDIUM = Self(2)` (`src/model.rs:150`), `Priority::default()` →
`MEDIUM` (`src/model.rs:142-143`), and `format!("{}", Priority::MEDIUM)` == `"P2"`
(`src/model.rs:1604`). `Priority::from_str` accepts both `"2"` and `"P2"` → MEDIUM
(`src/model.rs:1614/1621`).

## Verification

Ran an ad-hoc end-to-end test in an isolated temp workspace (`/tmp/bf-1dm0z-test/`):

| # | Check | Command | Result |
|---|-------|---------|--------|
| 1 | Epic with explicit `--priority 2` stores P2 | `bf create --type epic --title "..." --priority 2` | ✅ `Priority: P2`, `Type: epic` |
| 2 | Raw integer priority (JSON) | `bf show <id> --format json` → `d[0].priority` | ✅ `2` |
| 3 | `issue_type` + `status` stored correctly | `bf show <id> --format json` | ✅ `epic`, `open` |
| 4 | P2 is not a fallback artifact — neighbors differ | `--priority 1` → `1`; `--priority 3` → `3` | ✅ each stored verbatim |
| 5 | Survives flush checkpoint (db → JSONL) | `bf sync --flush-only` then `grep issues.jsonl` | ✅ `"priority":2`, `"issue_type":"epic"` |
| 6 | Update priority P2 → P0 → P2 round-trips | `bf update <id> --priority 0` then `--priority 2` | ✅ `P0` then back to `P2` |
| 7 | `list --type epic` filters to epics | `bf list --type epic --format json` | ✅ returns the epics only |
| 8 | Priority display label mapping (P2 = Medium) | `bf show <id> --format json` → `P{priority}` | ✅ raw `2` → label `P2` |

Output-shape notes (re-confirmed):
- `bf show <id> --format json` returns a **list** (`[ {...} ]`) — parse `d[0]`.
- `bf list --format json` emits **JSONL** (one object per line) — iterate line-by-line.

## Existing library-level test coverage

The repo already has thorough library-level coverage — all passing:

```
cargo test --test epic_default_priority --test test_epic_default_priority --test epic_cli --test epic_type_basic
  test result: ok. 9 passed; 0 failed    (epic_default_priority.rs)
  test result: ok. 7 passed; 0 failed    (test_epic_default_priority.rs)
  test result: ok. 5 passed; 0 failed    (epic_cli.rs)
  test result: ok. 6 passed; 0 failed    (epic_type_basic.rs)
```

These assert `Priority::MEDIUM == Priority(2)`, that `format!("{}", MEDIUM) == "P2"`, that
`from_str("2")`/`from_str("P2")` round-trip, and that the default is MEDIUM (P2). No new test was
needed; this bead adds the **CLI end-to-end** confirmation of the *explicit* `--priority 2` path
(which the default-priority tests do not isolate) on top.

## Build

`cargo build` — clean, no errors or warnings.
