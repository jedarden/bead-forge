# bf-5im — Test bead creation and listing

Verified the core `bf` create/list/show lifecycle end-to-end against the built
`bf` 0.3.0 binary (`target/debug/bf`).

## Steps

1. **Create** — `bf create --title 'Test bead'` → created bead `bf-264e2`.
2. **List** — `bf list` output includes the line
   `[bf-264e2] Test bead - open (P2)`. ✅ appears.
3. **Show** — `bf show bf-264e2` prints id/title/status/priority/type. ✅ works.
4. **Cleanup** — closed the test bead:
   `bf close bf-264e2 --reason "..."` → status now `closed`.

## Result

All acceptance criteria pass. No source changes — `bf` already handles
create/list/show correctly. Mutations are db-only (no flush), so `.beads/beads.db`
carries the ephemeral test bead; it is untracked by git, and `.beads/issues.jsonl`
was not touched by this bead.
