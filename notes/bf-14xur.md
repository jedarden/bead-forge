# Test Label Removal Bead

**Bead:** bf-14xur - Test label removal bead
**Date:** 2026-07-22
**Status:** ✅ COMPLETED

## What Was Done

Exercised `bf label remove` end-to-end against `bf-14xur` (original labels:
`phase-1`, `test3`, `urgent`) and verified behavior in both the CLI output and the
underlying `labels` table.

## Scenarios Tested

### TEST 1 — remove a single label

```bash
$ bf label remove bf-14xur --label test3
Removed label 'test3' from bf-14xur

$ bf labels bf-14xur --format json
[
  "phase-1",
  "urgent"
]

$ sqlite3 .beads/beads.db "SELECT label FROM labels WHERE issue_id='bf-14xur' ORDER BY label;"
bf-14xur|phase-1
bf-14xur|urgent
```

**Result:** ✅ The `test3` row was deleted from `labels`; CLI + DB agree.

### TEST 2 — remove a non-existent label (no-op)

```bash
$ bf label remove bf-14xur --label nonexistent
Removed label 'nonexistent' from bf-14xur     # exit 0
```

**Result:** ✅ No error, exit code 0. The `DELETE ... WHERE issue_id=? AND label=?`
(`src/storage/sqlite.rs:1275`) simply matched zero rows. Labels unchanged.

⚠️ **Minor UX note:** the CLI prints `Removed label 'X' from <id>` unconditionally
(`src/cli/mod.rs:2166-2169`) even when no row matched. It would be friendlier to
inspect the `execute()` row-count and only print "Removed" when a row was actually
deleted. Not a correctness bug — the delete is correctly idempotent — just a
misleading success message. Worth a follow-up if polishing.

### TEST 3 — remove multiple labels in one invocation

```bash
$ bf label remove bf-14xur --label phase-1 --label urgent
Removed label 'phase-1' from bf-14xur
Removed label 'urgent' from bf-14xur

$ bf labels bf-14xur
# (no output — bead has zero labels)

$ sqlite3 .beads/beads.db "SELECT label FROM labels WHERE issue_id='bf-14xur';"
# (empty)
```

**Result:** ✅ Both labels removed; bead correctly reports zero labels. Each label
is deleted in its own `with_immediate_transaction()` (so each is independently atomic),
looped over the `--label` args in `cmd_label` (`src/cli/mod.rs:2162-2170`).

## Restoration

After testing, the original three labels were re-added so the bead is left in its
original state:

```bash
$ bf label add bf-14xur --label phase-1 --label test3 --label urgent
$ bf labels bf-14xur --format json
[
  "phase-1",
  "test3",
  "urgent"
]
```

## Implementation Reviewed

`remove_label` (`src/storage/sqlite.rs:1275-1283`):

```rust
pub fn remove_label(&self, issue_id: &str, label: &str) -> Result<()> {
    self.with_immediate_transaction(|tx| {
        tx.execute(
            "DELETE FROM labels WHERE issue_id = ?1 AND label = ?2",
            params![issue_id, label],
        )?;
        Ok(())
    })
}
```

- Wrapped in `BEGIN IMMEDIATE` via `with_immediate_transaction` ✅
- Parameterized query — no injection ✅
- Correctly scoped to both `issue_id` AND `label` (no cross-bead label deletion) ✅
- Idempotent (no error when the label is absent) ✅

## Conclusion

Label removal works correctly across single-label, multi-label, and no-op cases.
The only finding is a cosmetic message in the no-op case (Test 2) — not a defect.
