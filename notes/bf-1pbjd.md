# bf-1pbjd: Epic with Labels + P3 Priority

## Implementation Status: ✅ VERIFIED (no code changes needed)

This is a test bead (`type: epic`, `priority: P3`, labels: `priority-3`, `test-epic`) exercising
the **combined** path through `bf`: an epic that carries both labels *and* an explicit `--priority 3`
at the same time. The feature is already fully implemented — this bead confirms it works end-to-end
against the installed `bf 0.3.0` binary.

Re-confirms and *combines* two paths previously verified independently:
- explicit epic P3 priority — `bf-3twnr` (and `bf-1dm0z`, `bf-3mvas`)
- epic-with-labels — `bf-31l74` (and `bf-110ct`, `bf-2ocnb`, `bf-2nxnf`)

This bead runs the **intersection** — epic + labels + explicit P3 together — independently on the
current binary.

## What "epic + labels + P3" means here

When a user runs `bf create --type epic --title "..." --priority 3 --label A --label B`, the
resulting epic must store **all three** attributes together: `issue_type=epic`, `priority=3 (P3)`,
and one row per label in the `labels` table. The two code paths converge in `cmd_create`:

- **Explicit P3:** `--priority 3` → `issue.priority = Priority(priority)` (`src/cli/mod.rs:1099`).
  Not a fallback — supplied verbatim.
- **Labels:** each `--label` inserts one row into `labels(issue_id, label)` verbatim; **no**
  comma-join or comma-split (the historical `duplicate,label,test` artifact from `bf-29jlp`).

At the model level `Priority::LOW == Priority(3)` (`src/model.rs:151`) and `format!("{}", LOW) ==
"P3"` (`src/model.rs:1605`). `bf show --format json` returns a **list** (`[ {...} ]`); the parsed
object is `d[0]`. `bf list --format json` emits **JSONL** (one object per line).

## Live bead confirmation

Read-only check of `bf-1pbjd` itself in the real workspace:

```bash
$ bf show bf-1pbjd --format json  →  d[0]
priority=3  type=epic  status=in_progress  labels=['priority-3','test-epic']

$ sqlite3 .beads/beads.db "SELECT label FROM labels WHERE issue_id='bf-1pbjd' ORDER BY label;"
bf-1pbjd|priority-3
bf-1pbjd|test-epic
```

Two separate rows — no comma-joined artifact.

## Verification

Ran a fresh ad-hoc end-to-end test in an isolated temp workspace (`/tmp/bf-1pbjd-test.*/`):

```bash
bf init --prefix bf
EPIC=$(bf create --type epic --title "Epic labels+P3 combo (bf-1pbjd)" \
    --priority 3 --label priority-3 --label test-epic --label extra-tag)   # → bf-3l0
```

| # | Check | Command | Result |
|---|-------|---------|--------|
| 1 | Epic with labels + explicit `--priority 3` created | `bf create --type epic ... --priority 3 --label ...` | ✅ created `bf-3l0` |
| 2 | Text display shows all three attributes | `bf show bf-3l0` | ✅ `Priority: P3`, `Type: epic`, `Labels: ...` |
| 3 | JSON stores priority/type/status/labels together | `bf show bf-3l0 --format json` → `d[0]` | ✅ `priority=3 type=epic status=open labels=[...]` |
| 4 | Each label = its own DB row (no comma-join) | `SELECT label FROM labels WHERE issue_id='bf-3l0'` | ✅ 3 rows |
| 5 | Survives flush checkpoint (db → JSONL) with all attrs | `bf sync --flush-only` then re-parse issues.jsonl | ✅ `priority=3 type=epic labels=[...]` |
| 6 | P3 is not a fallback artifact — neighbors differ | `--priority 2` → `2`; `--priority 4` → `4` | ✅ each stored verbatim alongside `3` |
| 7 | `list --type epic` filters to epics | `bf list --type epic --format json` | ✅ epic only, all rows `issue_type=epic` |
| 8 | Priority update round-trips P3 → P0 → P3 | `bf update <id> --priority 0` then `--priority 3` | ✅ `0` then back to `3` |
| 9 | Label add/remove round-trips on an epic | `bf label add/remove <id> --label temp-label` | ✅ 3 → 4 → 3 rows |

Concrete captured output:

```
# Check 2
Status: open  Priority: P3  Type: epic  Labels: extra-tag, priority-3, test-epic

# Check 3
priority= 3 type= epic status= open labels= ['extra-tag', 'priority-3', 'test-epic']

# Check 4
extra-tag / priority-3 / test-epic   (label count: 3)

# Check 5 (post-flush)
flushed: priority= 3 type= epic labels= ['extra-tag', 'priority-3', 'test-epic']

# Check 6
P2 neighbor priority= 2
P4 neighbor priority= 4

# Check 8
after P0: 0
after P3: 3

# Check 9
after add: 4 rows
after remove: 3 rows
```

## Existing test coverage

The repo already has extensive library-level coverage for both epic/label and epic/priority paths
(`tests/epic_with_labels.rs`, `tests/epic_complex_labels.rs`, `tests/test_comprehensive_labels.rs`,
`tests/label_storage.rs`, `tests/p0_epic_labels.rs`, `tests/epic_cli.rs`, `tests/epic_type_basic.rs`,
`tests/epic_default_priority.rs`, `test_epic_default_priority.rs`, and more). No new test was needed;
this bead adds the **CLI end-to-end** confirmation of the *combined* epic+labels+P3 path on top.

## Build

`cargo build` — clean, no errors or warnings.

## Conclusion

The combined path works correctly end-to-end: an epic carrying explicit P3 priority **and**
multiple labels is created, displayed (text + JSON), stored as separate per-label DB rows with the
correct priority/type, survives the flush checkpoint, filters under `list --type epic`, and supports
priority and label round-trips. P3 is supplied verbatim (not a default). No bugs found.
