# bf-lnm3i: Test Feature Default Priority

## Implementation Status: ✅ VERIFIED (no code changes needed)

This is a test bead (`type: feature`, `priority: P2`) exercising the feature-default-priority
path through `bf`. The feature is already fully implemented — this bead confirms it works
end-to-end against the installed `bf 0.3.0` binary, on top of existing library-level test
coverage.

## What "feature default priority" means here

When a user runs `bf create --type feature --title "..."` **without** `--priority`, the resulting
feature should get the default priority **P2 (Medium)** — the same default as every other issue
type. Features are not special-cased; they inherit the same default as task/bug/epic/etc.

The default comes from clap's `--priority` arg declaration (`src/cli/mod.rs:49`):

```rust
/// Priority (0=Critical, 4=Backlog)
#[arg(long, default_value = "2")]
priority: i32,
```

…which `cmd_create` applies directly via `issue.priority = Priority(priority);`
(`src/cli/mod.rs:1099`). At the model level, `impl Default for Priority` returns `Self::MEDIUM`
(`src/model.rs:141-145`), and `Priority::MEDIUM = Self(2)` (`src/model.rs:149`), so every code
path that falls back to the default converges on P2.

This is the **omission** path (no `--priority`, clap substitutes the default) — distinct from the
explicit path (`bf create --type feature --priority 2`), though both converge on P2.

## Verification

Ran an ad-hoc end-to-end test in an isolated temp workspace (`/tmp/bf-lnm3i-test/`):

| # | Check | Command | Result |
|---|-------|---------|--------|
| 1 | Feature without `--priority` defaults to P2 | `bf create --type feature --title "..."` | ✅ `Priority: P2`, `Type: feature` |
| 2 | Raw integer priority (JSON) | `bf show <id> --format json` → `d[0].priority` | ✅ `2` |
| 3 | `issue_type` stored correctly | `bf show <id> --format json` → `d[0].issue_type` | ✅ `feature` |
| 4 | Explicit `--priority 0` honored (P0) | `bf create --type feature --priority 0 ...` | ✅ `Priority: P0` |
| 5 | Explicit `--priority 4` honored (P4) | `bf create --type feature --priority 4 ...` | ✅ `Priority: P4` |
| 6 | Task type shares the same default (no type-specific default) | `bf create --type task` | ✅ `Priority: P2` |
| 7 | Multiple features, no flag, all default | 3× `bf create --type feature`, then `bf list --type feature --format json` | ✅ all `priority: 2`, distinct `[2]` |
| 8 | Survives flush checkpoint (db → JSONL) | `bf sync --flush-only` then parse `issues.jsonl` | ✅ features with `priority==2` present |
| 9 | Output shape re-confirmed | `bf show --format json` / `bf list --format json` | ✅ `show` = list (`[{…}]`); `list` = JSONL |
| 10 | Config `default_priority` is decorative for create | set `default_priority: 4`, omit `--priority` | ✅ still `Priority: P2` (see finding) |

Output-shape notes (re-confirmed):
- `bf show <id> --format json` returns a **list** (`[ {...} ]`) — parse `d[0]`.
- `bf list --format json` emits **JSONL** (one object per line) — iterate line-by-line.

## Existing library-level test coverage

The repo already has thorough library-level coverage — all passing:

```
cargo test --test feature_default_priority
  test result: ok. 10 passed; 0 failed    (feature_default_priority.rs)
```

These assert `Issue { ..Default::default() }` yields `Priority::MEDIUM == Priority(2)` for
features, that explicit priorities override it, storage roundtrips, JSON serialization emits
`"priority":2` / `"issue_type":"feature"`, every priority level (P0–P4) is available for
features, and that features/bugs share the same default. No new test was needed; this bead adds
the **CLI end-to-end** confirmation (the omission path that the library tests do not isolate) on
top.

## Finding: config `default_priority` is NOT wired into `bf create` (same as epic case)

`config.yaml` has a `default_priority` key (default `2`, written by `bf init`). A
`grep -rn '\.default_priority' src/` shows it is read only by the `bf config get/set`
subcommand (`src/cli/mod.rs:2395/2405/2439`) — **never by `cmd_create`**. The create command's
default comes entirely from clap's hardcoded `default_value = "2"`.

Demonstrated empirically in the temp workspace:

```bash
sed -i 's/^default_priority:.*/default_priority: 4/' .beads/config.yaml
bf create --title "Config Override Feature" --type feature   # → still Priority: P2, not P4
```

This is the identical latent inconsistency already documented for epics in
[notes/bf-3mvas.md](bf-3mvas.md): a user who changes `default_priority` in config (or via
`bf config set default_priority 4`) and then omits `--priority` will be surprised that new
features still come out as P2. Pre-existing, not a regression, and out of scope for this
verification bead — flagging for awareness.

## Build

`cargo build` — clean, no errors or warnings.
