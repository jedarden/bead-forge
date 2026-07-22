# bf-n6u — NEEDLE mitosis made atomic (plan Phase 5.3, Race 3)

**Status:** done. The code change lives in the separate **NEEDLE** repo
(`https://git.ardenone.com/jedarden/NEEDLE.git`), committed as `1a84f9a`
(`fix(needle-bf-n6u): make bead-splitting mitosis atomic via bf batch (Race 3)`)
and pushed to `main`. This note records the work for the bead-forge tracker.

## Problem (Race 3, the last live one)

NEEDLE split a bead into children with N+N separate subprocess calls and no
shared transaction: `bead_store::create_bead` (`br create`) then a separate
`bead_store::add_dependency` (`br dep add`), per child. A kill between those
calls (SIGKILL / OOM / pod eviction) left an orphaned child with no dependency
link and a parent that never unblocks. Races 1 & 2 were already fixed via
`run_bf_claim` (`bf claim`); Race 3 was still live.

Note: NEEDLE's actual split logic does **not** close the parent (a split parent
stays open/blocked while children are worked), so the fix uses `bf batch`
(create + dep_add_blocker, **no** close op) rather than `bf mitosis` (which
closes the parent). The literal string "mitosis" wasn't in the shell-out path —
the sites are `src/mitosis/mod.rs::create_children` (primary) and the `unravel`
strand.

## Fix (in NEEDLE)

- New trait method `BeadStore::split_bead(parent, &[NewChild])`:
  - **Default impl** = historical sequential `create_bead` + `add_dependency`
    loop. Covers all mock stores automatically and serves as the fallback.
  - **`BrCliBeadStore` override** (the production store) runs a single
    `bf batch --json` transaction: N `create` ops then N `dep_add_blocker` ops
    (`id` = parent/blocked, `blocker` = `@idx` child), no `close`. bf executes
    the array inside one SQLite `BEGIN IMMEDIATE`, so a crash or a failing op
    rolls the whole split back — no orphaned children.
  - Degrades gracefully: on bf-missing / non-zero exit / serialize error it
    logs and falls back to the sequential path (mirrors `run_bf_claim`). On a
    successful commit it does **not** fall back (that would double-create).
- Migrated both split sites (`mitosis::create_children`, `unravel`) to
  `split_bead`.

## Tests (NEEDLE `tests/real_br_integration_tests.rs`, real `bf`)

- `split_bead_creates_children_and_links_them_atomically` — children created,
  carry parent-tracking labels, parent blocked / children ready.
- `failed_mitosis_batch_leaves_no_orphaned_children` — a batch whose dep op
  references a nonexistent parent fails and leaves **zero** orphans (mirrors the
  crash-safety verification in `docs/needle-mitosis-migration.md`).

## Gate

`cargo fmt`; `cargo clippy --lib -- -D warnings` clean; the four touched files
are clippy-clean; existing mitosis (27) + unravel (30) unit tests pass; both new
integration tests pass. Pre-existing baseline clippy errors in unrelated
test/example files were left untouched.

## References

- Migration guide followed: `bead-forge/docs/needle-mitosis-migration.md`
- bf implementation relied on: `bead-forge/src/batch.rs`
  (`execute_batch` / `with_immediate_transaction`).
