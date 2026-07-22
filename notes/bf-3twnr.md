# bf-3twnr: Test Epic P3

## Implementation Status: ✅ VERIFIED (no code changes needed)

This is a test bead (`type: epic`, `priority: P3`) exercising the **explicit-P3 epic creation**
path through `bf`. The feature is already fully implemented — this bead confirms it works
end-to-end against the installed `bf 0.3.0` binary, on top of existing library-level test coverage.

## What "epic P3" means here

When a user runs `bf create --type epic --title "..." --priority 3`, the resulting epic should
store priority **3 (P3 / Low)**. This is the *explicit* path, distinct from the default/omission
path (`bf create --type epic` with no `--priority`, which falls back to clap's `default_value = "2"`
→ P2). The two paths converge on the same `cmd_create` code but supply different values:

- **Explicit P3 (this bead):** user passes `--priority 3`; `cmd_create` applies it directly via
  `issue.priority = Priority(priority);` (`src/cli/mod.rs:1099`). The value `3` is *not* a
  fallback artifact — it was supplied verbatim.
- **Default (bf-3mvas / bf-1dm0z-style):** no `--priority`; clap substitutes `default_value = "2"`
  (`src/cli/mod.rs:49`) → P2.

At the model level, `Priority::LOW = Self(3)` (`src/model.rs:151`), and
`format!("{}", Priority::LOW) == "P3"` (`src/model.rs:1605`). `Priority::from_str` accepts both
`"3"` and `"P3"` → LOW (`src/model.rs:1622` / `src/model.rs:1615`). So every code path that lands
on 3 maps cleanly to the `P3`/Low label.

`bf create` prints the new bead ID to stdout; `bf show <id> --format json` returns a **list**
(`[ {...} ]`) so the parsed object is `d[0]`, and `bf list --format json` emits **JSONL** (one
object per line).

## Verification

Ran an ad-hoc end-to-end test in an isolated temp workspace (`/tmp/bf-3twnr-test/`):

| # | Check | Command | Result |
|---|-------|---------|--------|
| 1 | Epic with explicit `--priority 3` stores P3 | `bf create --type epic --title "..." --priority 3` | ✅ `Priority: P3`, `Type: epic` |
| 2 | Raw integer priority (JSON) | `bf show <id> --format json` → `d[0].priority` | ✅ `3`, `issue_type=epic`, `status=open` |
| 3 | `issue_type` + `status` stored correctly | (same as #2) | ✅ `epic`, `open` |
| 4 | P3 is not a default/fallback artifact — neighbors differ | `--priority 2` → `2`; `--priority 4` → `4` | ✅ each stored verbatim alongside `3` |
| 5 | Survives flush checkpoint (db → JSONL) | `bf sync --flush-only` then re-parse issues.jsonl | ✅ `priority=3`, `issue_type=epic` |
| 6 | Update priority P3 → P0 → P3 round-trips | `bf update <id> --priority 0` then `--priority 3` | ✅ `0` then back to `3` |
| 7 | `list --type epic` filters to epics | `bf list --type epic --format json` | ✅ returns epics only |
| 8 | Priority display label mapping (raw 3 → label P3) | `bf show <id>` → `Priority:` line | ✅ raw `3` → label `P3` |

Concrete captured output:

```
# Check 1
ID: bf-2z9  Title: Explicit P3 Epic  Status: open  Priority: P3  Type: epic

# Check 2
priority=3 issue_type=epic status=open

# Check 4
create --priority P2 -> stored priority 2
create --priority P3 -> stored priority 3
create --priority P4 -> stored priority 4

# Check 6
after update --priority 0: 0
after update --priority 3: 3

# Check 5 (post-flush)
flushed: priority=3 issue_type=epic status=open
```

## Existing library-level test coverage

The repo already has thorough library-level coverage — all passing:

```
cargo test --lib model::tests
  test result: ok. 43 passed; 0 failed      (incl. LOW==3, format=="P3", from_str "3"/"P3")

cargo test --test epic_default_priority --test test_epic_default_priority --test epic_cli --test epic_type_basic
  test result: ok. 7 passed; 0 failed    (epic_default_priority.rs)
  test result: ok. 6 passed; 0 failed    (test_epic_default_priority.rs)
  test result: ok. 9 passed; 0 failed    (epic_cli.rs)
  test result: ok. 5 passed; 0 failed    (epic_type_basic.rs)
```

These assert `Priority::LOW == Priority(3)`, `format!("{}", LOW) == "P3"`, that
`from_str("3")`/`from_str("P3")` round-trip to LOW, and the full priority ordering. No new test
was needed; this bead adds the **CLI end-to-end** confirmation of the *explicit* `--priority 3`
path (which the default-priority tests do not isolate) on top.

## Build

`cargo build` — clean, no errors or warnings.

## Note

`config.yaml`'s `default_priority` key is decorative for `bf create` (the create default comes
entirely from clap's hardcoded `default_value = "2"`) — but that is irrelevant here, since this
bead tests the *explicit* `--priority 3` path, which bypasses any default entirely. The
config-decorative finding is already tracked in `notes/bf-3mvas.md` / `notes/bf-9543h.md`.
