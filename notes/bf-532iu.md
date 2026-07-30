# bf-532iu: Epic P4 Lowest Priority

## Implementation Status: ✅ VERIFIED (no code changes needed)

This is a test bead (`type: epic`, `priority: P4`) exercising the **lowest-priority
epic** path through `bf` — the P4 sibling of the many already-closed
"Test epic P0/P1/P2/P3 creation" beads. The feature is already fully implemented
against the installed `bf` binary; this bead confirms it works end-to-end.

`bf-532iu` itself has an intentionally empty body (no description / acceptance
criteria / dependencies), like the other priority-tier test epics. It was created
2026-07-06 by an ad-hoc test run; its title string does not appear in any current
test file (the root-level test scripts that generated these beads were removed in
`bf-3o9`, and accumulated `.beads/` state was flushed/committed in `bf-33zhy`).

Priority model (`src/model.rs`): `P0=CRITICAL`, `P1=HIGH`, `P2=MEDIUM`,
`P3=LOW`, `P4=BACKLOG` — P4 is numerically the highest value and therefore the
**lowest** priority (asserted in `Priority::test_priority_ordering`).

## Verification

Ran an ad-hoc end-to-end test in an isolated temp workspace (`/tmp/bf-532iu-test.*`):

```bash
bf init
ID=$(bf create --type epic --title "Epic P4 lowest priority (bf-532iu)" --priority 4)  # → bf-4kz
```

| # | Check | Command | Result |
|---|-------|---------|--------|
| 1 | `--priority 4` accepted by `bf create --type epic` | `bf create --type epic ... --priority 4` | ✅ created |
| 2 | Text display shows `Priority: P4` | `bf show <id>` | ✅ `Priority: P4`, `Type: epic` |
| 3 | JSON `priority` + `issue_type` | `bf show <id> --json` | ✅ `priority=4`, `issue_type=epic` |
| 4 | Raw DB row | `SELECT priority,issue_type FROM issues` | ✅ `4\|epic` |
| 5 | P4 sorts as **lowest** priority vs P0/P2 | `bf ready` | ✅ order `P0 → P2 → P4` (P4 last) |
| 6 | Survives flush checkpoint (db → JSONL) | `bf sync --flush-only` + re-read | ✅ JSONL `"priority":4` |
| 7 | Epic default priority is P2 (not P4) | `bf create --type epic` (no `--priority`) | ✅ `priority=2` — confirms P4 is explicit |

## Live bead confirmation

Read-only check of `bf-532iu` in the real workspace:

```bash
$ bf show bf-532iu --json | jq '.[0] | {id,title,priority,issue_type,status}'
{ "id": "bf-532iu", "title": "Epic P4 lowest priority",
  "priority": 4, "issue_type": "epic", "status": "in_progress" }

$ sqlite3 .beads/beads.db "SELECT priority,issue_type FROM issues WHERE id='bf-532iu';"
4|epic
```

`bf-532iu` is correctly stored as a P4 epic in both the live SQLite store and the
JSONL checkpoint. No bugs found; no code or test changes needed.
