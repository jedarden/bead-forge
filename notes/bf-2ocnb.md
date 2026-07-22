# bf-2ocnb: Test Epic With Labels

## Implementation Status: ✅ VERIFIED (no code changes needed)

This is a test bead (`type: epic`, labels: `epic-test`, `test`) exercising the
epic-with-labels path through `bf`. The feature is already fully implemented — this
bead confirms it works end-to-end against the installed `bf 0.3.0` binary.

Re-confirms the same path previously verified in `bf-yitu4`, `bf-ayjwy`, and `bf-4xyoo`.

## Verification

Ran a fresh ad-hoc end-to-end test in an isolated temp workspace (`/tmp/bf-2ocnb-test`):

```bash
bf init
bf create --type epic --title "Test epic with labels (bf-2ocnb)" \
    --label epic-test --label test --label multi-label   # → bf-4q4
```

| # | Check | Command | Result |
|---|-------|---------|--------|
| 1 | Multiple `--label` flags accepted | `bf create --type epic ... --label A --label B --label C` | ✅ created `bf-4q4` |
| 2 | Text display shows labels | `bf show bf-4q4` | ✅ `Labels: epic-test, multi-label, test` |
| 3 | Labels persist to storage (JSON) | `bf show bf-4q4 --format json` → `labels` | ✅ `['epic-test', 'multi-label', 'test']` |
| 4 | Type stored as `epic` | `bf show bf-4q4 --format json` → `issue_type` | ✅ `epic` |
| 5 | Dedicated label query (text) | `bf labels bf-4q4` | ✅ lists all three |
| 6 | Label query JSON output | `bf labels bf-4q4 --format json` | ✅ valid JSON array |
| 7 | Each label = its own DB row (no comma-join) | `SELECT label FROM labels WHERE issue_id='bf-4q4'` | ✅ 3 rows |
| 8 | Survives flush checkpoint (db → JSONL) | `bf sync --flush-only` then `bf labels bf-4q4` | ✅ labels intact |
| 9 | Add label to epic | `bf label add bf-4q4 --label added-later` | ✅ added |
| 10 | Remove label from epic | `bf label remove bf-4q4 --label multi-label` | ✅ removed |
| 11 | Epic with NO labels works | `bf create --type epic --title "..."` | ✅ created (0 rows in `labels` table, as expected) |

Note: `bf show --format json` returns a JSON **array** of beads (not a single object);
labels and `issue_type` are read from the first element.

## Live bead confirmation

Read-only check of `bf-2ocnb` itself in the real workspace:

```bash
$ bf labels bf-2ocnb --format json
[
  "epic-test",
  "test"
]

$ sqlite3 .beads/beads.db "SELECT issue_id, label FROM labels WHERE issue_id='bf-2ocnb' ORDER BY label;"
bf-2ocnb|epic-test
bf-2ocnb|test
```

Labels stored as separate rows — no comma-joined artifacts.

## Existing test coverage

The repo already has extensive label/epic test coverage (`tests/epic_with_labels.rs`,
`tests/epic_complex_labels.rs`, `tests/test_labels.rs`, `tests/label_storage.rs`,
`tests/p0_epic_labels.rs`, `tests/epic_p0_labels.rs`, and more). No new test was needed;
this bead adds the **CLI end-to-end** confirmation on top of it.

## Conclusion

Epic-with-labels works correctly end-to-end: create (single + multiple labels), text
and JSON display, dedicated label queries, per-row DB storage, flush survival, and
add/remove roundtrips on an epic. No bugs found.
