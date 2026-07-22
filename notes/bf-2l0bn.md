# Label Removal Test Results

**Bead:** bf-2l0bn - Label removal test target
**Prior target bead:** bf-2wd56 (closed, "Test cleanup - label removal tests complete")
**Date:** 2026-07-22
**Tool:** `bf` 0.3.0 (canonical — `br` is the deprecated alias)
**Status:** ✅ ALL PASS

## Purpose

`bf-2l0bn` is a **target bead** for exercising the `bf label remove` path
end-to-end. The bead started with no labels; setup labels were added only to
give `remove` something to act on, then removed again. Every removal variant
was run against this bead and the final state was restored to empty.

## Tests Performed

All operations were driven against `bf-2l0bn` and verified with both
`bf labels <id> --format json` and a raw `sqlite3 .beads/beads.db` query on the
`labels` table.

### 1. Initial state — ✅ PASS
`bf labels bf-2l0bn --format json` → `[]` (empty).

### 2. Setup: add three labels — ✅ PASS
`bf label add bf-2l0bn --label remove-target --label alpha --label beta`
→ three separate rows: `alpha`, `beta`, `remove-target`.

### 3. Remove a SINGLE label — ✅ PASS
`bf label remove bf-2l0bn --label alpha` → "Removed label 'alpha' …";
remaining: `beta`, `remove-target`. The other two rows are untouched.

### 4. Remove MULTIPLE labels at once — ✅ PASS
`bf label remove bf-2l0bn --label beta --label remove-target`
→ both removed; bead now empty (`[]`).

### 5. Remove NON-EXISTENT label — ✅ PASS (idempotent)
`bf label remove bf-2l0bn --label does-not-exist` → exit 0, no error,
no spurious row created. Removing a label that isn't there is a no-op.

### 6. Duplicate removal is idempotent — ✅ PASS
Added `gamma`, then `bf label remove … --label gamma --label gamma`
→ exit 0; the same label passed twice removes cleanly with no error.

### 7. Raw DB agrees with CLI — ✅ PASS
After all removals: `SELECT issue_id, label FROM labels WHERE issue_id='bf-2l0bn';`
returns zero rows — no orphaned label rows left behind.

### 8. Global label list stays clean — ✅ PASS
None of the transient labels (`alpha`/`beta`/`gamma`/`remove-target`/
`does-not-exist`) remain in `bf label list`; the target bead did not pollute
the global label index.

### 9. Target bead integrity preserved — ✅ PASS
After every add/remove cycle, `bf show bf-2l0bn` still shows the bead intact
(correct id, title, status, type). Label churn never corrupted the issue row.

## Noteworthy behavior: add vs. remove FK asymmetry

`bf label remove <nonexistent-bead> --label x` **succeeds silently** (exit 0,
no row inserted), whereas `bf label add <nonexistent-bead>` fails with
`FOREIGN KEY constraint failed` (documented in `notes/bf-42i8k.md`).

This asymmetry is **correct**, not a bug:
- `add` is an `INSERT` into `labels(issue_id, label)`, so a missing `issue_id`
  trips the FK to `issues(id)` → error.
- `remove` is a `DELETE`; with no matching row there is nothing to delete and
  no FK check is triggered → silent no-op, consistent with idempotent-removal
  semantics.

Verified directly: after `bf label remove bf-zzznope --label x`, the DB holds
zero rows for `bf-zzznope` — no orphan is ever created.

## Final State

`bf-2l0bn` restored to empty label set, matching the prior target bead's
cleanup outcome:

```bash
$ bf labels bf-2l0bn --format json
[]

$ sqlite3 .beads/beads.db "SELECT issue_id, label FROM labels WHERE issue_id='bf-2l0bn';"
(no rows)
```

## Conclusion

All label-removal functionality works as expected: single remove, multi-remove,
remove-all (by removing every label), idempotent non-existent removal, and
idempotent duplicate removal — each leaving the `labels` table and the target
bead itself in a consistent state.
