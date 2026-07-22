# bf-3mvas: Test Epic Default Priority

## Implementation Status: ✅ VERIFIED (no code changes needed)

This is a test bead (`type: epic`) exercising the epic-default-priority path through `bf`.
The feature is already fully implemented — this bead confirms it works end-to-end against the
installed `bf 0.3.0` binary, on top of existing library-level test coverage.

## What "epic default priority" means here

When a user runs `bf create --type epic --title "..."` **without** `--priority`, the resulting
epic should get the default priority **P2 (Medium)** — the same default as every other issue
type. Epics are not special-cased; they inherit the same default as task/bug/feature/etc.

The default comes from clap's `--priority` arg declaration (`src/cli/mod.rs:49`):

```rust
/// Priority (0=Critical, 4=Backlog)
#[arg(long, default_value = "2")]
priority: i32,
```

…which `cmd_create` applies directly via `issue.priority = Priority(priority);`
(`src/cli/mod.rs:1099`). At the model level, `Priority::default()` returns `Self::MEDIUM`
(`src/model.rs:142-143`), and `Priority::MEDIUM = Self(2)` (`src/model.rs:150`), so every code
path that falls back to the default converges on P2.

## Verification

Ran an ad-hoc end-to-end test in an isolated temp workspace (`/tmp/bf-3mvas-test/`):

| # | Check | Command | Result |
|---|-------|---------|--------|
| 1 | Epic without `--priority` defaults to P2 | `bf create --type epic --title "..."` | ✅ `priority: 2`, displays `Priority: P2` |
| 2 | Raw integer priority (JSON) | `bf show <id> --format json` → `d[0].priority` | ✅ `2` |
| 3 | `issue_type` stored correctly | `bf show <id> --format json` → `d[0].issue_type` | ✅ `epic` |
| 4 | Explicit `--priority 0` honored | `bf create --type epic --priority 0 ...` | ✅ `Priority: P0` |
| 5 | Explicit `--priority 4` honored | `bf create --type epic --priority 4 ...` | ✅ `Priority: P4` |
| 6 | Task type shares the same default (no type-specific default) | `bf create --type task` | ✅ `Priority: P2` |
| 7 | Multiple epics, no flag, all default | 3× `bf create --type epic`, then `bf list --type epic --format json` | ✅ all `priority: 2` |
| 8 | Survives flush checkpoint (db → JSONL) | `bf sync --flush-only` then `grep issues.jsonl` | ✅ `"priority":2` present |

Output-shape notes (re-confirmed):
- `bf show <id> --format json` returns a **list** (`[ {...} ]`) — parse `d[0]`.
- `bf list --format json` emits **JSONL** (one object per line) — iterate line-by-line.

## Existing library-level test coverage

The repo already has thorough library-level coverage — all passing:

```
cargo test --test epic_default_priority --test test_epic_default_priority
  test result: ok. 7 passed; 0 failed    (epic_default_priority.rs)
  test result: ok. 6 passed; 0 failed    (test_epic_default_priority.rs)
```

These assert `Priority::default() == MEDIUM` (2), that epics inherit it, explicit priorities
override it, serialization roundtrips, and every priority level (P0–P4) is available for
epics. No new test was needed; this bead adds the **CLI end-to-end** confirmation on top.

## Finding: config `default_priority` is NOT wired into `bf create`

`config.yaml` has a `default_priority` key (default `2`, written by `bf init`). A
`grep -rn '\.default_priority' src/` shows it is read only by the `bf config get/set`
subcommand (`src/cli/mod.rs:2395/2405/2439`) — **never by `cmd_create`**. The create command's
default comes entirely from clap's hardcoded `default_value = "2"`.

Demonstrated empirically in the temp workspace:

```bash
sed -i 's/^default_priority:.*/default_priority: 4/' .beads/config.yaml
bf create --title "Config Override Epic" --type epic   # → still Priority: P2, not P4
```

So `default_priority` is effectively decorative for the interactive `bf create` command — a
user who changes it in config (or via `bf config set default_priority 4`) and then omits
`--priority` will be surprised that new epics still come out as P2. This is a pre-existing
latent inconsistency, not a regression, and out of scope for this verification bead (no spec
requires the config value to drive create) — flagging for awareness.
