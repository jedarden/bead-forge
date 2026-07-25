# bf-4eiwi3 — Regular Bug (label-vs-type independence)

**Type:** `bug` | **Labels:** `epic-test` | **Priority:** P2 | **Status:** in_progress → closed
**Task:** "Bug with epic-test label" — verification-only, no code task.

## What this bead is

A test bead carrying the `epic-test` label but typed `bug` (not `epic`). It is
the latest in the label/epic verification series (bf-63v50t, bf-5vz3z6,
bf-5voa30, bf-2spaoy…). There is no implementation work — only verification.

The dimension this bead adds that the **epic-typed** siblings could not prove:
**the label name `epic-test` is independent of issue `type`.** A bug can carry a
label whose name starts with "epic" and the type filter still discriminates
correctly. The epic-typed beads (bf-63v50t, bf-5vz3z6) collapse two axes into
one — for them, `-l epic-test -t epic` always matches — so they cannot show
that a label match and a type match are filtered separately.

## Verification (all pass)

| Check | Result |
|-------|--------|
| `bf labels bf-4eiwi3` | `epic-test` ✓ |
| br-compatible `labels` table row | `bf-4eiwi3\|epic-test` ✓ |
| `bead_labels` (bf-specific) table | empty — label lives in br-compat `labels` table |
| JSONL checkpoint (git-tracked) | `"labels":["epic-test"]` persisted ✓ (`issue_type:"bug"`) |
| `bf search -l epic-test --format json --limit 1000` | finds bf-4eiwi3 ✓ (label match) |
| `bf search -l epic-test -t epic` | **excludes** bf-4eiwi3 ✓ (correct: `bug` ≠ `epic`) |
| `bf search -l epic-test -t bug` | finds bf-4eiwi3 ✓ (label + type both match) |
| `sqlite3 .beads/beads.db "PRAGMA integrity_check;"` | `ok` ✓ |

## Key result

Label-name matching and type filtering are **independent**: the same `epic-test`
label query returns bf-4eiwi3 when unconstrained or filtered to `-t bug`, and
correctly drops it under `-t epic`. This is exactly the discrimination the
epic-typed test beads could not exercise.

## Notes

- Label data stored in the br-compatible `labels` table (`issue_id`, `label`),
  consistent with the sibling beads.
- JSONL checkpoint shows `status:"open"` while live store shows `in_progress` —
  expected: checkpoint lags between auto-flushes; the label array is consistent
  across both.

## Workspace state observed

- Dispatched at HEAD `6d469cd`; `.needle-predispatch-sha` matches HEAD — clean
  dispatch.
- **Shared workspace**: branch is `needle/bf-5wku` with other beads' uncommitted
  work already in the tree (`src/batch.rs`, `tests/test_json_edge_cases.rs`,
  `.beads/issues.jsonl`, `.needle-predispatch-sha`, untracked `.beads/traces/`).
- Per the shared-workspace rule, **only this notes file was committed**; the
  other beads' changes were left untouched (no `git add -A` / `git commit -a`).
