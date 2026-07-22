# bf-ivkz7: Test Epic Default Priority

## Implementation Status: ✅ VERIFIED (no code changes needed)

This is a test bead (`type: epic`) exercising the epic-default-priority path through `bf`.
The feature is already fully implemented — this bead confirms it works end-to-end against
the installed `bf 0.3.0` binary, on top of existing library-level test coverage.

## What "epic default priority" means here

When a user runs `bf create --type epic --title "..."` **without** `--priority`, the resulting
epic should get the default priority **P2 (Medium)**, the same default as every other type.
Epics are not special-cased — they inherit the same default as task/bug/feature/etc.

The default comes from clap's `--priority` arg declaration (`src/cli/mod.rs:48-50`):

```rust
/// Priority (0=Critical, 4=Backlog)
#[arg(long, default_value = "2")]
priority: i32,
```

…which `cmd_create` applies directly via `issue.priority = Priority(priority);` (`src/cli/mod.rs:1099`).
At the model level, `Priority::default()` and `Priority::MEDIUM` are both `2` (`src/model.rs:141-152`),
so every code path that falls back to the default converges on P2.

## Verification

Ran an ad-hoc end-to-end test in an isolated temp workspace (`/tmp/bf-ivkz7-test.*`):

| Check | Command | Result |
|-------|---------|--------|
| Epic without `--priority` defaults to P2 | `bf create --type epic --title "..."` | ✅ `priority: 2`, displays `Priority: P2` |
| Explicit `--priority 0` honored | `bf create --type epic --priority 0 ...` | ✅ `priority: 0` |
| Default persists to storage (JSON) | `bf show <id> --format json` → `priority` | ✅ `2` |
| Type stored as `epic` | `bf show <id> --format json` → `issue_type` | ✅ `epic` |
| Text display shows `Priority: P2` | `bf show <id>` | ✅ `Priority: P2` |
| Survives flush checkpoint (db → JSONL) | `bf sync --flush-only` then `bf show` + `grep issues.jsonl` | ✅ `"priority":2` in both |
| Multiple epics, no flag, all default | 3× `bf create --type epic`, then `bf list --format json` | ✅ all default epics `priority: 2` |

Output-shape notes reused from bf-4l98s and re-confirmed:
- `bf show <id> --format json` returns a **list** (`[ {...} ]`) — parse `d[0]`.
- `bf list --format json` emits **JSONL** (one object per line) — iterate line-by-line.

## Finding: config `default_priority` is NOT wired into `bf create`

`config.yaml` has a `default_priority` key (default `2`, set by `cmd_init`, `src/cli/mod.rs:1053`),
read by `bf config get/set` (`src/cli/mod.rs:2405/2438`) and consumed by batch create
(`src/batch.rs:19,647`). However, `cmd_create` (`src/cli/mod.rs:1080-1104`) loads the config but
**never reads `config.default_priority`** — it uses clap's hardcoded `default_value = "2"` instead.

Demonstrated empirically: setting `default_priority: 4` in `.beads/config.yaml` and running
`bf create --type epic` (no `--priority`) still produced `priority: 2`, not `4`.

So `default_priority` is effectively decorative for the interactive `bf create` command — a
user who changes it in config will be surprised that new epics/beads still come out as P2.
This is out of scope for this test bead (no spec requires the config value to drive create),
but it's worth tracking as a potential follow-up if config-driven defaults are ever desired.

## Existing test coverage

The repo already has thorough library-level coverage in
`tests/epic_default_priority.rs` (7 test fns, all passing) — `Priority::default()` is P2,
`Issue { issue_type: Epic, ..Default::default() }` is P2, storage roundtrips preserve P2,
serialization emits `"priority":2`, `Issue::new()` defaults to P2, and default-vs-explicit
priorities across P0–P4. No new test was needed; this bead adds the missing
**CLI end-to-end** confirmation plus the config-decoupling finding on top of it.
