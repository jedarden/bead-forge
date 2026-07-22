# Duplicate Label Test Bead

**Bead:** bf-29jlp - Duplicate label test bead
**Duplicated from:** bf-1b0v7
**Date:** 2026-07-22
**Status:** ✅ COMPLETED

## What Was Done

Tested label handling on a duplicate bead and corrected a malformed label state:

1. Inspected labels on `bf-29jlp` via `bf labels bf-29jlp --format json`
2. Discovered the labels were stored incorrectly (see bug below)
3. Removed the malformed single label, then added the three intended labels:
   `bf label add bf-29jlp --label duplicate --label test --label label`
4. Verified via DB and CLI: three separate rows now exist — `duplicate`, `label`, `test`

## Bug Found: Duplication joins labels into a single comma-string

When `bf-29jlp` was duplicated from `bf-1b0v7`, its three labels were collapsed into
**one** `labels` table row containing the literal string `duplicate,label,test`, instead
of three separate rows.

Comparison of raw `labels` table rows:

| Bead | Labels rows (correct) |
|------|-----------------------|
| `bf-1b0v7` (source) | `duplicate` / `label` / `test` (3 rows) |
| `bf-29jlp` (duplicate, before fix) | `duplicate,label,test` (1 malformed row) |

This produced a wrong JSON output before the fix:

```json
["duplicate,label,test"]   // WRONG — single label with commas
```

After the fix, output matches the source:

```json
["duplicate", "label", "test"]   // CORRECT — three labels
```

### Likely cause

The duplication path appears to have joined the source's labels with `,` and stored
the resulting string as a single label, rather than iterating and inserting one row
per label. Worth tracing in the duplicate/clone logic (search for where label copy
happens during duplication).

## Verification

```bash
$ bf labels bf-29jlp --format json
[
  "duplicate",
  "label",
  "test"
]

$ sqlite3 .beads/beads.db "SELECT issue_id, label FROM labels WHERE issue_id='bf-29jlp' ORDER BY label;"
bf-29jlp|duplicate
bf-29jlp|label
bf-29jlp|test
```
