# Bead bf-3pdfse: Epic → Child-Task Label Propagation

## Summary

Verification bead in the harness-test epic series. Asks whether **a child task
created under an epic inherits the epic's labels** — i.e. does splitting an epic
(via `mitosis`) or wiring a blocker dependency (`dep add-blocker`) copy the
parent epic's labels onto the child bead?

**Finding: No.** Child tasks never inherit the parent epic's labels. Labels are
**opt-in per child**: a child gets labels only when they are stated explicitly in
the mitosis `--children` JSON definition. There is no propagation, inheritance,
or copy of parent→child labels anywhere in `src/`. No code changes were
required; this note is the commit artifact.

## Source: how children get their labels

Child beads are produced in two places in `src/batch.rs`, and neither reads the
parent's labels:

### 1. `mitosis()` — plain split (src/batch.rs:1074-1083)

Each child is created with a hard-coded empty label vector:

```rust
// src/batch.rs:1074-1083
for (title, type_, priority) in &children {
    ops.push(BatchOp::Create {
        title: title.clone(),
        type_: type_.clone(),
        priority: *priority,
        description: None,
        assignee: None,
        labels: Vec::new(),   // <-- always empty
    });
}
```

### 2. `mitosis_ex()` — extended split (src/batch.rs:1113-1122)

Labels come solely from the explicit child definition (`child.labels`), never
from the parent:

```rust
// src/batch.rs:1113-1122
for child in &children {
    ops.push(BatchOp::Create {
        title: child.title.clone(),
        type_: child.type_.clone(),
        priority: child.priority,
        description: child.description.clone(),
        assignee: child.assignee.clone(),
        labels: child.labels.clone(),   // <-- explicit per-child only
    });
}
```

`MitosisChild` (src/batch.rs:1141-1153) carries `labels: Vec<String>` with
`#[serde(default)]`, so omitted labels deserialize to empty.

### 3. `dep add-blocker` — dependency wiring only

The other "child" path (`BatchOp::DepAddBlocker`, src/batch.rs:242) only inserts
a row into the `dependencies` table linking two **already-existing** beads. It
creates no bead and never touches labels.

### No propagation code exists

A repo-wide search for any parent→child label logic returned nothing:

```
$ grep -rniE "label.*propagat|propagat.*label|inherit.*label|label.*inherit|parent.*label|copy.*label" src/
(none found)
```

## Empirical verification

Created a throwaway epic carrying the `test-child-task` label, then split it via
`mitosis` into two children — one with no explicit labels, one with an explicit
`inherited-check` label — and inspected both. All probes were deleted afterward
so the shared live store (`issues.jsonl`) was not polluted (shared workspace —
races with other needle agents).

```
$ br create --title "TEMP epic-label-propagation probe (bf-3pdfse)" --type epic --label "test-child-task"
bf-4kvmno

$ br show bf-4kvmno --json   # (labels trimmed for brevity)
[{"id":"bf-4kvmno", ..., "issue_type":"epic", ..., "labels":["test-child-task"]}]

$ br mitosis bf-4kvmno --children '[
    {"title":"TEMP child no-labels probe (bf-3pdfse)","type":"task","priority":2},
    {"title":"TEMP child with-labels probe (bf-3pdfse)","type":"task","priority":2,"labels":["inherited-check"]}
  ]' --reason "bf-3pdfse verification split" --format json
[
  {"op":0,"status":"ok","id":"bf-2eh837","message":"Created bead bf-2eh837"},
  {"op":1,"status":"ok","id":"bf-3wdqzi","message":"Created bead bf-3wdqzi"},
  {"op":2,"status":"ok","message":"ok: bf-4kvmno blocked by bf-2eh837"},
  {"op":3,"status":"ok","message":"ok: bf-4kvmno blocked by bf-3wdqzi"},
  {"op":4,"status":"ok","message":"Closed bead bf-4kvmno"}
]

$ br show bf-2eh837 --json   # child created with NO explicit labels
[{"id":"bf-2eh837", ..., "issue_type":"task", ..., "compaction_level":0, ...}]
                                  # ^^^ no "labels" key at all -> 0 labels

$ br show bf-3wdqzi --json   # child created WITH explicit label
[{"id":"bf-3wdqzi", ..., "issue_type":"task", ..., "labels":["inherited-check"]}]
                                  # ^^^ only its own label; epic's not inherited
```

- Child **without** explicit labels (`bf-2eh837`): JSON omits `labels` entirely →
  0 labels. Epic's `test-child-task` **not** inherited. ✓
- Child **with** explicit label (`bf-3wdqzi`): `labels` = `["inherited-check"]`
  only. Epic's `test-child-task` **not** inherited. ✓
- Parent epic (`bf-4kvmno`): correctly `closed` by the split, retained its own
  `["test-child-task"]`. ✓

Cleanup:

```
$ br delete bf-2eh837 && br delete bf-3wdqzi && br delete bf-4kvmno
Deleted bead bf-2eh837
Deleted bead bf-3wdqzi
Deleted bead bf-4kvmno

$ grep -cE 'bf-2eh837|bf-3wdqzi|bf-4kvmno' .beads/issues.jsonl
0
```

All three probes removed; `issues.jsonl` is clean.

## Bead State

| Field | Value |
|-------|-------|
| ID | `bf-3pdfse` |
| Title | Child task verification epic |
| Type | `epic` |
| Status | `in_progress` |
| Priority | `P2` (2) |
| Description | Verifying epic labels child task functionality |
| Assignee | `claude-code-glm-4.7-h1-bforge` |
| Labels | `another-test-label`, `bug-verification`, `test-child-task` |

The bead itself is an `epic` that carries the `test-child-task` label — exactly
the scenario probed (an epic with a label that a naive caller might expect to
flow onto its children). It does not.

## Conclusion

Epic labels do **not** propagate to child tasks. A child bead's labels come only
from the explicit `labels` field of its mitosis `--children` definition
(`mitosis_ex`, src/batch.rs:1120) or — for the plain `mitosis()` path — are
hard-coded empty (src/batch.rs:1081). `dep add-blocker` only wires edges and
never sets labels. There is no parent→child label inheritance code anywhere in
`src/`. **Works as expected (opt-in per child).**
