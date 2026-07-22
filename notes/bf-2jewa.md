# bf-2jewa: Epic 7 — Priority P0 with labels

## Implementation Status: ✅ VERIFIED (no code changes needed)

This is a test bead (`type: epic`, `priority: P0`, labels: `critical`, `high-priority`) exercising
the **combined** path through `bf`: an epic that carries both labels *and* an explicit
`--priority 0` (Critical) at the same time. The feature is already fully implemented — this bead
confirms it works end-to-end against the installed `bf 0.3.0` binary.

Companion to `bf-1pbjd` (epic + labels + **P3**); this one runs the same intersection at the
**P0 / Critical** end of the priority range.

## What "epic + labels + P0" means here

When a user runs `bf create --type epic --title "..." --priority 0 --label critical --label high-priority`,
the resulting epic must store **all three** attributes together: `issue_type=epic`, `priority=0 (P0)`,
and one row per label in the `labels` table. The two code paths converge in `cmd_create`:

- **Explicit P0:** `--priority 0` → `issue.priority = Priority(priority)` (`src/cli/mod.rs`). Supplied
  verbatim, not a fallback. At the model level `Priority::CRITICAL == Priority(0)` and
  `format!("{}", CRITICAL) == "P0"` (`src/model.rs`).
- **Labels:** each `--label` inserts one row into `labels(issue_id, label)` verbatim; **no**
  comma-join or comma-split.

`bf show --format json` returns a **list** (`[ {...} ]`); the parsed object is `d[0]`.
`bf list --format json` emits **JSONL** (one object per line). Note: `bf create` prints the bare
bead ID to stdout and does **not** accept `--format json`.

## Live bead confirmation

Read-only check of `bf-2jewa` itself in the real workspace:

```
$ bf show bf-2jewa --format json  →  d[0]
priority=0  type=epic  status=in_progress  labels=['critical','high-priority']
```

Both labels present as distinct entries — no comma-joined artifact.

## Verification

Ran a fresh ad-hoc end-to-end test in an isolated temp workspace (`/tmp/bf-2jewa-test.*/`):

```bash
bf init --prefix bf
EPIC=$(bf create --type epic --title "Epic P0+labels e2e (bf-2jewa)" \
    --priority 0 --label critical --label high-priority)   # → bf-12g
```

| # | Check | Command | Result |
|---|-------|---------|--------|
| 1 | Epic with labels + explicit `--priority 0` created | `bf create --type epic ... --priority 0 --label ...` | ✅ created `bf-12g` |
| 2 | Text display shows all three attributes | `bf show bf-12g` | ✅ `Priority: P0`, `Type: epic`, `Labels: critical, high-priority` |
| 3 | JSON stores priority/type/status/labels together | `bf show bf-12g --format json` → `d[0]` | ✅ `priority=0 type=epic status=open labels=['critical','high-priority']` |
| 4 | Survives flush checkpoint (db → JSONL) with all attrs | `bf sync --flush-only` then re-parse issues.jsonl | ✅ `priority=0 type=epic labels=['critical','high-priority']` |
| 5 | P0 is not a fallback artifact — neighbors differ | `--priority 2` → `2`; `--priority 4` → `4` | ✅ each stored verbatim alongside `0` |
| 6 | `list --type epic` filters to epics | `bf list --type epic --format json` | ✅ all rows `issue_type=epic` |
| 7 | Label add/remove round-trips on an epic | `bf label add/remove <id> --label temp-label` | ✅ 2 → 3 → 2 labels |

## Existing test coverage

`tests/p0_epic_labels.rs` covers the P0-epic-with-labels path at the library level — 14 tests,
all passing:

```
cargo test --test p0_epic_labels
test result: ok. 14 passed; 0 failed; 0 ignored
```

Covers creation, JSON round-trip, serialization, label update, multiple distinct P0 epics,
child/hierarchy label propagation, status computation, and full-metadata storage. No new test was
needed; this bead adds the **CLI end-to-end** confirmation of the combined epic+labels+P0 path.

## Build

`cargo test --test p0_epic_labels` — clean, no errors or warnings.

## Conclusion

The combined path works correctly end-to-end: an epic carrying explicit P0 (Critical) priority
**and** multiple labels is created, displayed (text + JSON), stored with the correct priority/type
and both labels, survives the flush checkpoint, filters under `list --type epic`, and supports
label round-trips. P0 is supplied verbatim (not a default). No bugs found.
