# bf-2szxv: Child Task For Epic Labels Test

## Implementation Status: ✅ VERIFIED (no code changes needed)

This is a test bead (`type: task`, labels: `bug`, `child-label`) — the **child
task** half of an epic-labels scenario. The feature set is already fully
implemented; this bead confirms that a labeled child task attached as a
dependency to a labeled epic works end-to-end against the installed `bf 0.3.0`
binary.

Complements the epic-side verifications recorded in `bf-2ocnb`, `bf-110ct`
(and earlier `bf-yitu4`, `bf-ayjwy`, `bf-4xyoo`): those proved a labeled epic
works on its own; this one proves the **child-task-with-labels blocking a
labeled epic** path.

## Verification

Ran a fresh ad-hoc end-to-end test in an isolated temp workspace
(`/tmp/bf-2szxv-test.*`):

```bash
bf init --prefix bf
EPIC=$(bf create --type epic --title "Epic labels test (bf-2szxv)" \
    --label epic-test --label test)                              # → bf-460
CHILD=$(bf create --type task --title "Child task for epic labels test" \
    --label bug --label child-label)                             # → bf-bqa
bf dep add "$CHILD" --blocks "$EPIC"                             # child blocks epic
```

| # | Check | Command | Result |
|---|-------|---------|--------|
| 1 | Labeled child task created (multiple `--label`) | `bf create --type task ... --label bug --label child-label` | ✅ created `bf-bqa` |
| 2 | Child labels via text/JSON | `bf labels bf-bqa --format json` | ✅ `["bug","child-label"]` |
| 3 | Child blocks epic (`dep add`) | `bf dep add bf-bqa --blocks bf-460` | ✅ `bf-460 depends on bf-bqa (blocks)` |
| 4 | Dependency listed from epic | `bf dep list bf-460` | ✅ shows the blocks edge |
| 5 | Dependency tree from epic | `bf dep tree bf-460` | ✅ renders child node |
| 6 | Epic NOT ready while child open | `bf ready` (grep `bf-460`) | ✅ 0 matches (correctly blocked) |
| 7 | Labels stored as separate DB rows (child) | `SELECT label FROM labels WHERE issue_id='bf-bqa'` | ✅ 2 rows, no comma-join |
| 8 | Labels stored as separate DB rows (epic) | `SELECT label FROM labels WHERE issue_id='bf-460'` | ✅ 2 rows |
| 9 | Dependency row in DB | `SELECT * FROM dependencies` | ✅ `bf-460|bf-bqa|blocks` |
| 10 | Survives flush checkpoint (db → JSONL) | `bf sync --flush-only` then `bf dep list` + `bf labels` | ✅ dep + all labels intact |
| 11 | Closing child → epic becomes ready | `bf close bf-bqa` then `bf ready` | ✅ `bf-460` now ready |

Note on the `dependencies` schema: columns are `issue_id` (the blocked bead) and
`depends_on_id` (the blocker), with `type` defaulting to `blocks`. So the row
`bf-460|bf-bqa|blocks` reads "epic `bf-460` depends on child `bf-bqa`".

## Live bead confirmation

Read-only check of `bf-2szxv` itself in the real workspace:

```bash
$ bf labels bf-2szxv --format json
[
  "bug",
  "child-label"
]

$ sqlite3 .beads/beads.db "SELECT issue_id, label FROM labels WHERE issue_id='bf-2szxv' ORDER BY label;"
bf-2szxv|bug
bf-2szxv|child-label
```

Labels stored as separate rows — no comma-joined artifacts.

## Existing test coverage

The repo already has extensive label/dependency test coverage
(`tests/epic_with_labels.rs`, `tests/label_storage.rs`, dependency/claim tests,
etc.). No new test was needed; this bead adds the **CLI end-to-end** confirmation
of the child-task-with-labels-blocking-a-labeled-epic path on top of it.

## Conclusion

A labeled child task blocking a labeled epic works correctly end-to-end: child
creation with multiple labels, label text/JSON queries, per-row DB storage, the
`dep add`/`dep list`/`dep tree` relationship, correct readiness gating (epic
blocked while child open, ready once child closed), and flush survival for both
labels and the dependency edge. No bugs found.
