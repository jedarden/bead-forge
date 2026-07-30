# bf-5vz3z6 — Epic Test 2 (distinguishing-label search)

**Type:** `epic` | **Labels:** `epic-test`, `high-priority` | **Status:** in_progress → closed
**Task:** "Another test epic" — verification-only, no code task.

## What this bead is

The sibling of bf-63v50t ("Epic Test 1"). The two were created to exercise
multi-epic label search together:

| Bead | Labels |
|------|--------|
| bf-63v50t (Epic Test 1) | `backend`, `epic-test` |
| bf-5vz3z6 (Epic Test 2) | `epic-test`, `high-priority` |

They **share `epic-test`** but **differ on the second label** (`backend` vs
`high-priority`). That makes bf-5vz3z6 the vehicle to verify the one thing a
single epic cannot prove: that label search **distinguishes** two epics that
share one label but differ on another, and that multi-label search **unions**
them correctly. Like the other recent label/epic test beads
(bf-63v50t, bf-5voa30, bf-2spaoy…), there is no implementation work — only
verification.

## Verification (all pass)

| Check | Result |
|-------|--------|
| `bf labels bf-5vz3z6` | `epic-test`, `high-priority` ✓ |
| br-compat `labels` table rows | `bf-5vz3z6\|epic-test`, `bf-5vz3z6\|high-priority` ✓ |
| `bead_labels` (bf-specific) table | empty — labels live in br-compat `labels` table |
| JSONL checkpoint (git-tracked) | `"labels":["epic-test","high-priority"]` persisted ✓ |
| `bf search -l high-priority --format json` | finds bf-5vz3z6, **not** bf-63v50t ✓ |
| `bf search -l backend --format json --limit 1000` | finds bf-63v50t, **not** bf-5vz3z6 ✓ |
| `bf search -l epic-test -t epic --format json` (shared label) | finds **both** ✓ |
| `bf search -l backend -l high-priority` (multi-label OR) | finds **both** ✓ |
| `sqlite3 .beads/beads.db "PRAGMA integrity_check;"` | `ok` ✓ |

## Notes

- The key result for this bead: **single-label search correctly distinguishes**
  the two epics by their differing label (`high-priority` → only bf-5vz3z6;
  `backend` → only bf-63v50t), while **shared-label search and multi-label OR**
  correctly return both. This is the dimension bf-63v50t could not test alone.
- Label data is stored in the **br-compatible `labels` table** (`issue_id`,
  `label`), not the bf-specific `bead_labels` table — consistent with bf-63v50t.
- JSONL checkpoint shows `status:"open"` while the live store shows
  `in_progress` — expected: the checkpoint lags the live store between
  auto-flushes. The label array itself is consistent across both.
- `bf search` uses `--format json` (not `--json`); default `--limit` is 50.
