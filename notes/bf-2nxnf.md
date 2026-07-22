# Comprehensive Label Test Bead

**Bead:** bf-2nxnf - Comprehensive label test bead
**Date:** 2026-07-22
**Tool:** `bf` (canonical — `br` is the deprecated alias)
**Status:** ✅ ALL PASS (11/11 scenarios)

## Purpose

`bf-2nxnf` carries the `duplicate-test` label (alongside `frontend`, `phase-1`),
the signature of the comma-joined label artifact seen historically in
`bf-29jlp` (duplicated from `bf-1b0v7`, where three labels were collapsed into
one `duplicate,label,test` row). This bead re-exercises the full `bf label`
surface end-to-end — with special attention to comma handling — and confirms
no such artifact is present here.

## Pre-existing state

Labels stored correctly as **three separate rows** (no comma-joined artifact):

```bash
$ bf labels bf-2nxnf --format json
[
  "duplicate-test",
  "frontend",
  "phase-1"
]

$ sqlite3 .beads/beads.db "SELECT issue_id, label FROM labels WHERE issue_id='bf-2nxnf' ORDER BY label;"
bf-2nxnf|duplicate-test
bf-2nxnf|frontend
bf-2nxnf|phase-1
```

Every operation below used throwaway labels and was verified with **both** the
`bf` CLI and a raw `sqlite3 .beads/beads.db` query on the `labels` table.

## Tests Performed

### A. Add single label — ✅ PASS
`bf label add bf-2nxnf --label comp-single` → "Added label 'comp-single'";
one new row inserted (DB rowcount 1).

### B. Add multiple labels at once — ✅ PASS
`bf label add bf-2nxnf --label comp-a --label comp-b --label comp-c`
→ three separate `Added` confirmations; three distinct rows inserted
(`comp-a`, `comp-b`, `comp-c`, `comp-single` all present, ordered).

### C. Idempotent duplicate add — ✅ PASS
Re-adding an already-present label (`frontend`) reports success but inserts
**no duplicate row** — `PRIMARY KEY (issue_id, label)` enforces uniqueness
(DB rowcount for `frontend` stays 1; JSON shows it exactly once).

### D. Label containing a comma is stored as ONE label — ✅ PASS
`bf label add bf-2nxnf --label "comma,joined"` → stored as the single literal
label `comma,joined` (one row, **not** split into `comma`/`joined`). This is
the direct counterpart to the historical bug: `bf` now inserts one row per
`--label` argument verbatim and does **not** join-with-comma or split-on-comma.
Removed afterward to restore state.

### E. Remove single label — ✅ PASS
`bf label remove bf-2nxnf --label comp-single` → "Removed label ...";
row gone (DB rowcount 0).

### F. Remove multiple labels at once — ✅ PASS
`bf label remove bf-2nxnf --label comp-a --label comp-b --label comp-c`
→ all three removed; zero `comp-*` rows remain. Each label is deleted in its
own `with_immediate_transaction()` (independently atomic), looped over the
`--label` args in `cmd_label`.

### G. Remove non-existent label is idempotent — ✅ PASS
`bf label remove bf-2nxnf --label does-not-exist` → exit 0, no error. The
`DELETE ... WHERE issue_id=? AND label=?` matches zero rows — correctly a
no-op. (Same cosmetic note as `notes/bf-14xur.md`: the CLI prints
"Removed label 'X' …" unconditionally even when no row matched — a misleading
success message, not a correctness bug.)

### H. Add label to non-existent bead fails with FK error — ✅ PASS
`bf label add bf-zzznope --label should-fail`
→ `Error: FOREIGN KEY constraint failed`, exit 1. The `INSERT` into
`labels(issue_id, label)` trips the FK to `issues(id)`, preventing orphan rows.

### I. Labels for non-existent bead returns empty — ✅ PASS
`bf labels bf-zzznope` → empty output, exit 0. JSON form returns `[]`.
Direct `SELECT`, efficient, no error for a missing bead.

### J. Global label hygiene — ✅ PASS
After all add/remove cycles: zero `comp-*` rows and zero `comma,joined` rows
remain **anywhere** in the `labels` table. The transient throwaway labels did
not pollute the global label index.

### K. Bead integrity preserved — ✅ PASS
After all churn, `bf show bf-2nxnf` still shows the bead intact (correct id,
title, status, type, assignee). Label mutation never corrupted the issue row.

## add vs. remove FK asymmetry (confirmed correct, not a bug)

- `bf label add <nonexistent-bead>` → **fails** (FK on INSERT).
- `bf label remove <nonexistent-bead>` → **succeeds silently** (DELETE of zero
  rows matches nothing; no FK check is triggered).

Consistent with idempotent-removal semantics and the prior findings in
`notes/bf-2l0bn.md` / `notes/bf-4p1sr.md`.

## Final State

`bf-2nxnf` restored to its original three-label state:

```bash
$ bf labels bf-2nxnf --format json
[
  "duplicate-test",
  "frontend",
  "phase-1"
]

$ sqlite3 .beads/beads.db "SELECT count(*) FROM labels WHERE issue_id='bf-2nxnf';"
3
```

## Conclusion

All label functionality works correctly across the comprehensive matrix:
add (single + multiple), idempotent duplicate add, comma-containing labels
stored verbatim as single rows, remove (single + multiple), idempotent
non-existent removal, FK validation on add, empty-result for missing beads,
global label hygiene, and bead-integrity preservation. **No comma-joined
artifact is present** on `bf-2nxnf` — the historical `duplicate-test` bug
does not reproduce here. The only finding is the pre-documented cosmetic
"Removed" message in the no-op removal case (Test G), not a defect.
