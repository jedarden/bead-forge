# bf-1ig29 — Commit & push bf-3o9 orphan-file cleanup (final step, parent bf-3o9)

## Task
Commit the staged orphan-file deletions (23 root-level test shell scripts +
committed `test_version` binary / `test_version.rs`) plus any test ports from
child bf-4waen, then push to origin. Pathspec-scoped only — leave all unrelated
in-flight working-tree changes untouched.

## Findings on claim: the cleanup was ALREADY committed and pushed by a prior run

The premise that "23 orphan-file deletions are already staged" was stale. On
inspection they were already **committed**, not staged:

- Commit `8dc4a79` — `chore(bf-3o9): remove orphaned root-level test shell
  scripts and committed test_version binary, scenarios covered under tests` —
  deletes exactly the 23 orphan paths (test_bf_*.sh, test_epic_*.sh,
  test_p0_*.sh, test_repair_*.sh, test_bead_*.sh, test_version, test_version.rs;
  2959 deletions).
- `git merge-base --is-ancestor 8dc4a79 origin/needle/bf-5wku` → **YES**: the
  cleanup commit is already on `origin` (on branch `needle/bf-5wku`).
- `git ls-files` for all orphan patterns → **empty**: no orphan root files are
  tracked anymore.
- `git diff --cached --name-status` → **empty**: nothing is currently staged.
  No bf-4waen test ports were left staged either.

## Acceptance criteria — all already satisfied

1. "commit pushed to origin referencing bf-3o9" → ✅ `8dc4a79` is on
   `origin/needle/bf-5wku` and references bf-3o9 in its message.
2. "orphaned root files no longer tracked" → ✅ `git ls-files` empty.
3. "all unrelated working-tree changes remain present and untouched" → ✅ left
   as-is (see below); this commit adds only `notes/bf-1ig29.md`.

No source-file change was needed, so per the bead fallback this notes file is
the produced commit.

## Why this commit is pushed to a dedicated branch `needle/bf-1ig29`, not `needle/bf-5wku`

The shared branch `needle/bf-5wku` is divergent at claim time
(`ahead 1, behind 2`), and the divergence is entirely **other beads' in-flight
work**, not bf-3o9/bf-1ig29:

- Local ahead: `5773cf9` (bf-37bzi) — rewrites `src/cli/mod.rs` and adds
  `src/format/*`.
- Remote behind: `aa4cac4` (bf-3iosi, rewrites `src/cli/mod.rs` +
  `tests/autoflush_wiring.rs`) and `6035615` (bf-37bzi, adds `notes/bf-37bzi.md`).

Both sides rewrite `src/cli/mod.rs`. A rebase/merge to land a commit on
`needle/bf-5wku` would force resolving conflicts inside another bead's in-flight
`src/cli/mod.rs` — out of scope for bf-1ig29, and a force-push is forbidden.
So this notes commit is pushed to a fresh `needle/bf-1ig29` branch instead:
no force-push, no disruption to `needle/bf-5wku`, nothing of other beads' work
touched.

## Scope discipline
Committed ONLY `notes/bf-1ig29.md` (explicit pathspec, no `git add -A` /
`git commit -a`). Untouched: `src/cli/mod.rs`, `src/format/*`, `.beads/**`,
`.needle-predispatch-sha`, untracked traces, `tests/autoflush_mutation.rs`, etc.
