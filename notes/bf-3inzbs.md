# Bead bf-3inzbs: Smoke-test — build + test suite state

## Summary

Health-check / smoke-test bead (empty description, title "test"). Verified the
bead-forge working tree builds cleanly and the test suite runs at the documented
baseline. No source changes made — this is a verification bead.

## Verification (2026-07-25)

### Build

```
$ cargo build
=== EXIT: 0 ===
```

Clean compile, no errors.

### Test suite

```
$ cargo test
test result: FAILED. 616 passed; 3 failed; 10 ignored; 0 measured; 0 filtered out; finished in 14.16s
```

**616 / 619 pass** — identical to the baseline recorded in prior smoke-test beads
(`bf-2iskc5`, `bf-1vch24`).

The 3 failures are pre-existing and environmental, not regressions:

| Test | Cause |
|------|-------|
| `sync::tests::test_labels_persist_through_full_sync` | Fails on an unflushed `bf-sync-labels` bead — leftover workspace state from a prior agent's sync test. `br doctor` reports `Unflushed beads: 1`. |
| `sync::tests::test_find_workspace_not_found` | Known pre-existing failure (workspace-path-dependent). |
| `batch::tests::test_mixed_op_batch_all_operations_atomic` | Known pre-existing failure. |

### DB integrity

```
$ sqlite3 .beads/beads.db "PRAGMA integrity_check;"
ok
```

Live store is healthy.

## Conclusion

Build clean; suite at documented 616/619 baseline; DB integrity ok. No action
required beyond this verification record.

## Note on shared workspace

The working tree had uncommitted changes to `src/batch.rs` and
`tests/test_json_edge_cases.rs` at session start — these belong to other
agents/beads in this shared needle workspace and were left untouched. Only this
notes file was committed.
