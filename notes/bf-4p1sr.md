# Label Functionality Test Results

## Bead ID: bf-4p1sr
## Title: Label test bead
## Test Date: 2026-07-22
## Tool: `bf` (canonical — `br` is the deprecated alias)

This bead (`bf-4p1sr`) is a label test bead seeded with labels `test1` and
`test2`. It re-verifies the CLI label functionality end-to-end. All operations
were exercised against `bf-4p1sr` itself, using throwaway labels, and the
label state was restored to the original (`test1`, `test2`) afterward.

## Pre-existing state

- Labels: `test1`, `test2` — two separate rows in the `labels` table
  (confirmed via `SELECT ... FROM labels WHERE issue_id='bf-4p1sr'`).
- No comma-joined label artifacts — each label is stored as its own row.

## Tests Performed

### 1. List labels for a specific bead (text) — ✅ PASS
`bf label list bf-4p1sr` prints a header and one label per line.

### 2. List labels via `bf labels <ID>` alias — ✅ PASS
`bf labels bf-4p1sr` prints just the label names (no header).

### 3. List ALL unique labels (no ID) — ✅ PASS
`bf label list` prints every label across the workspace with its usage count,
sorted by count descending.

### 4. JSON output — ✅ PASS
`bf labels bf-4p1sr --format json` returns a valid JSON array of label strings.

### 5. Add single label — ✅ PASS
`bf label add bf-4p1sr --label verify-single` → "Added label 'verify-single'";
verified present, then removed.

### 6. Add multiple labels at once — ✅ PASS
`bf label add bf-4p1sr --label multi-a --label multi-b --label multi-c` —
all three added, one confirmation each.

### 7. Duplicate add is idempotent — ✅ PASS
Re-adding an already-present label (`test1`) reports success but produces no
duplicate row (PRIMARY KEY `(issue_id, label)` enforces it; JSON shows `test1`
exactly once).

### 8. Remove single label — ✅ PASS
`bf label remove bf-4p1sr --label verify-single` → "Removed label ...";
confirmed gone.

### 9. Remove multiple labels at once — ✅ PASS
`bf label remove bf-4p1sr --label multi-a --label multi-b --label multi-c` —
all three removed.

### 10. Remove non-existent label is idempotent — ✅ PASS
`bf label remove bf-4p1sr --label does-not-exist` reports removed, exit code 0,
no error.

### 11. Add label to non-existent bead fails with FK error — ✅ PASS
`bf label add bf-zzznope --label should-fail` →
"Error: FOREIGN KEY constraint failed", exit code 1. Prevents orphan rows.

### 12. Labels for non-existent bead returns empty — ✅ PASS
`bf labels bf-zzznope` → empty output, clean exit 0.

## Final State

Labels on `bf-4p1sr` restored to original: `test1`, `test2`.

```
$ bf labels bf-4p1sr --format json
[
  "test1",
  "test2"
]

$ sqlite3 .beads/beads.db "SELECT issue_id, label FROM labels WHERE issue_id='bf-4p1sr' ORDER BY label;"
bf-4p1sr|test1
bf-4p1sr|test2
```

## Conclusion

All label functionality is working as expected with the `bf` CLI:
add/remove (single + multiple), list (all + per-bead, two alias forms),
JSON output, idempotent duplicate/non-existent handling, and FK validation on
the bead. No bugs found on this bead — unlike the earlier `bf-29jlp` /
`bf-42i8k` cases, no comma-joined label artifacts were present here.
