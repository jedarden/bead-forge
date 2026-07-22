# bf-4xyoo: Test Epic With Labels

## Implementation Status: ✅ VERIFIED (no code changes needed)

This is a test bead (`type: epic`, labels: `epic-test`, `test`) exercising the epic-with-labels
path through `bf`. The feature is already fully implemented — this bead confirms it works
end-to-end against the installed `bf 0.3.0` binary.

## Verification

Ran an ad-hoc end-to-end test in an isolated temp workspace (`/tmp/bf-4xyoo-test`):

```bash
bf init
bf create --type epic --title "Test epic with labels" \
    --label epic-test --label test --label multi-label   # → bf-4yo
```

| Check | Command | Result |
|-------|---------|--------|
| Multiple `--label` flags accepted | `bf create --type epic ... --label A --label B --label C` | ✅ created |
| Labels persist to storage | `bf show <id> --format json` → `labels` | ✅ `['epic-test', 'multi-label', 'test']` |
| Type stored as `epic` | `bf show <id> --format json` → `issue_type` | ✅ `epic` |
| Text display shows labels | `bf show <id>` | ✅ `Labels: epic-test, multi-label, test` |
| Dedicated label query | `bf labels <id>` | ✅ lists all three |

## Existing test coverage

The repo already has extensive label/epic test coverage (`tests/epic_with_labels.rs`,
`tests/epic_complex_labels.rs`, `tests/test_labels.rs`, `tests/label_storage.rs`,
`tests/p0_epic_labels.rs`, `tests/epic_p0_labels.rs`, and many more). No new test was needed.

## Minor observation (out of scope)

`bf list` has no label-filter flag (`--label`/`--labels` are rejected; only `--all` exists).
Label-based filtering is available via `bf search` and `bf labels <id>` instead. Not part of
this bead's scope; noted only for completeness.
