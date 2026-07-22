# Label Functionality Test Results (duplicate of bf-5pta4)

## Bead ID: bf-42i8k
## Test Date: 2026-07-22
## Tool: `bf` (canonical — `br` is the deprecated alias)

This bead is a duplicate of the closed `bf-5pta4`. It re-verifies the CLI label
functionality end-to-end. All operations were exercised against bead `bf-42i8k`
itself and the label state was restored to its original afterward.

## Pre-existing state note

The bead carries one label that is a single comma-containing string,
`duplicate,label,test`, plus the separate labels `integration-test` and
`test-label`. This is pre-existing data (a comma-in-label test artifact), not
introduced by this work — and it confirms labels are stored verbatim, not
split on commas.

## Tests Performed

### 1. List all unique labels — ✅ PASS
`bf label list` (no ID) prints every label with its usage count, sorted.
Cleans up the comma-containing label as one entry.

### 2. List labels for a specific bead — ✅ PASS
`bf label list bf-42i8k` / `bf labels bf-42i8k` shows the bead's labels.

### 3. JSON output — ✅ PASS
`bf labels bf-42i8k --format json` returns a valid JSON array of label strings.

### 4. Add single label — ✅ PASS
`bf label add bf-42i8k --label verify-single` → "Added label 'verify-single'";
verified present, then removed.

### 5. Add multiple labels at once — ✅ PASS
`bf label add bf-42i8k --label multi-a --label multi-b --label multi-c` —
all three added, one confirmation each.

### 6. Remove single label — ✅ PASS
`bf label remove bf-42i8k --label verify-single` → "Removed label ...";
confirmed gone.

### 7. Remove multiple labels at once — ✅ PASS
`bf label remove bf-42i8k --label multi-a --label multi-b --label multi-c` —
all three removed.

### 8. Duplicate add is idempotent — ✅ PASS
Adding an already-present label (`test-label`) succeeds with no duplicate row
(PRIMARY KEY `(issue_id, label)` enforces it; JSON shows the label exactly once).

### 9. Remove non-existent label is idempotent — ✅ PASS
`bf label remove bf-42i8k --label does-not-exist` reports removed, no error.

### 10. Add label to non-existent bead fails with FK error — ✅ PASS
`bf label add bf-zzznope --label should-fail` →
"Error: FOREIGN KEY constraint failed", exit code 1. Prevents orphan rows.

### 11. Labels for non-existent bead returns empty — ✅ PASS
`bf labels bf-zzznope` → empty output, clean exit 0.

## Final State

Labels on `bf-42i8k` restored to original:
- `duplicate,label,test`
- `integration-test`
- `test-label`

## Conclusion

All label functionality is working as expected with the `bf` CLI:
add/remove (single + multiple), list (all + per-bead), JSON output,
idempotent duplicate/non-existent handling, and FK validation on the bead.
