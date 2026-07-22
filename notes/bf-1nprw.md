# bf-1nprw — Investigate `bf ready` returning zero results despite open beads

**Status: NO BUG FOUND.** The reported symptom does not reproduce; the
`get_ready_candidates` query is correct. A regression test was added to lock in
the invariant.

## The report

Observed 2026-07-20: `bf list` showed 54 open beads, but `bf ready --limit 500
--json` returned an empty array. Suspicion was a correctness bug in the
ready-set query (possibly tied to the multi-workspace claim path in commit
`99f68beb`).

## Investigation

**Code path.** `bf ready` → `cmd_ready` (`src/cli/mod.rs:1397`) →
`get_ready_candidates` (`src/claim.rs:414`) → one SQL statement. The `--json`
branch is a straight `serde_json::to_string(&candidates)` pass-through — it does
**not** re-filter, so an empty array can only come from the query itself
returning nothing.

**Reproduction (live `.beads/beads.db`, this repo, 2026-07-22).**

| Metric | Count |
|---|---|
| open beads total | 66 |
| open beads passing pre-block filters (ephemeral/pinned/template/deleted) | 66 |
| `bf ready --limit 500 --json` returns | **12** |
| hand-computed unblocked open set (independent SQLite query) | **12** |

The 12 returned ids are **exactly** the 12 truly-unblocked open beads. The
remaining 54 each genuinely have ≥1 unclosed blocker (verified by direct
`dependencies ⋈ issues` query). Notably `bf-127ow` ("Test Epic 1") — cited in the
report as a suspicious standalone bead — is in fact blocked by `bf-ncms2`
(status=`blocked`, a non-terminal status), so excluding it is correct.

**Data sanity.** No self-dependency loops; no dangling `depends_on_id` rows.

**Query sanity.** The `NOT EXISTS (… INNER JOIN issues blocker … AND blocker.status
NOT IN ('closed','tombstone','done','completed'))` blocker clause is identical
across all 6 ready/claim SQL fragments in `claim.rs`. Dangling blockers are
dropped by the INNER JOIN, so they cannot cause false blocking. Logic is sound.

**The referenced commit.** `99f68beb` is `fix(claim): claim_any prefers the
primary workspace deterministically` — it touches the **multi-workspace
`claim_any`** path, not `bf ready` (which is single-workspace and unaffected).

## Why it likely *looked* like zero on 2026-07-20

Several of the 12 currently-ready beads (e.g. `bf-3fkja`, `bf-1dcws`,
`bf-48pw0`, `bf-33zhy`) were created on/after 2026-07-20. At the moment of the
report it is plausible that, transiently, every open bead genuinely had an
unclosed blocker — i.e. the empty result was correct, not a bug. There is no
evidence in the code or current data of a query defect.

## Change made

Added `test_ready_includes_zero_dependency_open_beads_bf_1nprw` in
`src/claim.rs`. It builds a mixed workspace — standalone zero-dependency open
beads, a bead blocked by an open blocker, and a bead blocked transitively by a
`status=blocked` blocker (the `bf-127ow → bf-ncms2` shape) — and pins the exact
ready-set membership. This is the regression guard requested by the bead: an
open bead with zero dependencies must appear in `bf ready` output.

`cargo test --lib claim::` → 10 passed, 0 failed.
