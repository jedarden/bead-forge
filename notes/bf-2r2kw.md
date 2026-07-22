# bf-2r2kw — Test Epic 7: Priority P0 with labels

**Type:** epic · **Priority:** P0 · **Labels:** critical, high-priority

## Task

Verify that an epic can carry P0 (Critical) priority together with labels,
end-to-end through storage, serialization, and CLI display.

## Verification

The bead itself is a live example of the feature under test. `br show bf-2r2kw`
reports:

```
Type: epic
Priority: P0
Labels: critical, high-priority
```

confirming an epic accepts P0 priority alongside multiple labels via the real
CLI/storage path.

Existing automated coverage was run and passes:

- `cargo test --test epic_p0_labels` — 12 passed
  (creation, single/multiple labels, serialization, children, closed status,
  priority display, label add/remove, filtering, JSON roundtrip, priority ordering)
- `cargo test --test p0_epic_labels` — 14 passed
  (metadata, hierarchy/label propagation, aggregation, status computation,
  closed children, no-labels, distinct-label multi-epic, JSON roundtrip)

Total: 26 tests, 0 failures.

## Conclusion

Epic + P0 priority + labels is correct and fully covered by existing tests; no
source or test changes were required. Adding another test would duplicate
`tests/epic_p0_labels.rs` / `tests/p0_epic_labels.rs`.
