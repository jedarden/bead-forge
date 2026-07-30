# bf-59bn0: Run P0 epic creation unit tests

## Summary

Verification-only bead. `tests/p0_epic_creation.rs` already exists and is tracked in
git (last touched by `9b71cc3f`). No source or test changes were required — the task
was to confirm the suite passes.

## Command

```
cargo test --test p0_epic_creation
```

## Result

```
running 8 tests
test test_p0_epic_creation ... ok
test test_p0_epic_display_formatting ... ok
test test_p0_epic_json_roundtrip ... ok
test test_p0_epic_serialization ... ok
test test_multiple_p0_epics ... ok
test test_p0_priority_value ... ok
test test_p0_vs_other_priorities ... ok
test test_p0_epic_with_full_metadata ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
```

## Acceptance criteria

All satisfied — every named test passes:

- [x] `test_p0_epic_creation` — round-trips a `Priority::CRITICAL` epic through
      `Storage::create_issue` / `get_issue`, asserting id, type, priority (`.0 == 0`),
      status, and description.
- [x] `test_p0_epic_serialization` — serde JSON emits `"issue_type":"epic"` and
      `"priority":0`; deserialization preserves both.
- [x] `test_p0_priority_value` — `Priority::CRITICAL == Priority(0)` and orders
      strictly below HIGH / MEDIUM / LOW / BACKLOG.
- [x] `test_p0_epic_with_full_metadata` — assignee and timestamps survive storage.
- [x] `test_p0_epic_display_formatting` — `Display` renders `P0`.
- [x] `test_multiple_p0_epics` — three P0 epics persist and are recovered via
      `list_issues` with the default filter.
- [x] `test_p0_vs_other_priorities` — P0..P4 map to values 0..4 with matching display
      strings and monotonic ordering.
- [x] `test_p0_epic_json_roundtrip` — pretty-printed JSON round-trip preserves all fields.
- [x] All 8 tests pass with `cargo test`.

## Notes

Nothing flaky observed; the suite is pure unit-level (tempdir-backed SQLite plus serde),
with no shared-state or timing dependencies.
