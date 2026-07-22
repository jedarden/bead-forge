# bf-rdnyh: Test Epic Labels Priority — epic + labels + P0

## Implementation Status: ✅ VERIFIED (no code changes needed)

This is a test bead (`type: epic`, `priority: P0`, labels: `critical`, `priority-test`) exercising
the **combined** path through `bf`: an epic that carries both labels *and* an explicit P0 (Critical)
priority at the same time. The feature is already fully implemented — this bead confirms it works
end-to-end against the installed `bf 0.3.0` binary.

Sibling of `bf-2jewa` (epic + `critical`/`high-priority` + P0); this one runs the same intersection
with the `critical` + `priority-test` label pair.

## What "epic + labels + P0" means here

When a user runs
`bf create --type epic --title "..." --priority 0 --label critical --label priority-test`, the
resulting epic must store **all three** attributes together: `issue_type=epic`, `priority=0 (P0)`,
and one row per label in the `labels` table (no comma-join, no comma-split).

- **Explicit P0:** `--priority 0` is supplied verbatim, not a fallback. `Priority::CRITICAL ==
  Priority(0)` and `format!("{}", CRITICAL) == "P0"` (`src/model.rs`).
- **Labels:** each `--label` inserts one distinct row into `labels(issue_id, label)`.

`bf show --format json` returns a **list** (`[ {...} ]`); the parsed object is `d[0]`.

## Live bead confirmation

Read-only check of `bf-rdnyh` itself in the real workspace:

```
$ bf show bf-rdnyh --format json  →  d[0]
priority=0  type=epic  status=in_progress  labels=['critical','priority-test']
```

Both labels present as distinct entries — no comma-joined artifact.

## Verification (fresh e2e in an isolated temp workspace)

```bash
bf init --prefix bf
EPIC=$(bf create --type epic --title "Epic P0+labels e2e (bf-rdnyh)" \
    --priority 0 --label critical --label priority-test)   # → bf-66h
```

| # | Check | Command | Result |
|---|-------|---------|--------|
| 1 | Epic with labels + explicit `--priority 0` created | `bf create --type epic ... --priority 0 --label ...` | ✅ created `bf-66h` |
| 2 | Text display shows all three attributes | `bf show bf-66h` | ✅ `Priority: P0`, `Type: epic`, `Labels: critical, priority-test` |
| 3 | JSON stores priority/type/status/labels together | `bf show bf-66h --format json` → `d[0]` | ✅ `priority=0 type=epic status=open labels=['critical','priority-test']` |
| 4 | Survives flush checkpoint (db → JSONL) with all attrs | `bf sync --flush-only` then grep `issues.jsonl` | ✅ `"priority":0 "issue_type":"epic" "labels":["critical","priority-test"]` |

All four checks pass. The combined epic + labels + P0 path stores every attribute verbatim and
round-trips through the JSONL flush intact.

## Existing test coverage

The library-level path is covered by dedicated integration tests, including
`tests/epic_p0_labels.rs` and `tests/p0_epic_labels.rs` (creation, JSON round-trip, serialization,
label update, multiple distinct P0 epics, hierarchy label propagation, full-metadata storage).

> Note: `cargo test` could **not** be run this session — the working tree carries an unrelated,
> in-progress change from concurrent fleet work that leaves the lib un-buildable
> (`error[E0425]: cannot find value scored_bead_to_issue` at `src/cli/mod.rs:1454`; the symbol was
> never defined in git history). That break is outside this bead's scope, so it was left untouched;
> this note commits only `notes/bf-rdnyh.md`. The end-to-end verification above ran against the
> already-installed `bf 0.3.0` binary, which is unaffected.

## Conclusion

The combined path works correctly end-to-end: an epic carrying explicit P0 (Critical) priority
together with the `critical` and `priority-test` labels stores and round-trips all three attributes
faithfully. No source changes were required.
