# bf-12luu — Rebase local main onto origin/main and publish catch-up

**Task:** Reconcile local `main` with `origin/main` (`git pull --rebase`), resolving
any `.beads/issues.jsonl` conflicts per the flush-before-repair rule, then push the
result and confirm parity.

## Starting state (2026-07-22)

The bead's premise (local main "9 behind", dated 2026-07-20) was stale. Actual divergence:

- Local `main` was **ahead 2, behind 4** vs `origin/main`.
- The 2 local-ahead commits:
  - `ad877b0` notes(bf-3cu1k) — retry re-verification of checkpoint script AC
  - `317b796` notes(bf-doiq) — verify list/ready JSON already routes through shared formatter
- The 4 origin-ahead commits:
  - `531e415` fix(bf-1w20y) — match SHA256SUMS entry by filename in bf-update
  - `3b24ea2` notes(bf-4gkg5) — verify bf-checkpoint.sh config-gating + workspace slice
  - `5e6754a` notes(bf-3cu1k) — same subject as local `ad877b0`
  - `12f5d64` notes(bf-doiq) — same subject as local `317b796`

## Diagnosis

The two local-ahead commits were **content-identical twins** of two origin commits —
same bead IDs, same subjects, same files, same line counts. Confirmed via `git patch-id`:
both pairs share patch-id `2987931cac7c8460c266d6afd5d84f92e45f5491`. A full
`git diff origin/main main` showed local main differed from origin only by *lacking*
origin's `bf-1w20y` fix and `bf-4gkg5` note — its own two notes were already upstream
byte-for-byte. No `.beads/issues.jsonl` divergence existed to conflict.

## Work performed

- Operated in an **isolated git worktree** (`/tmp/bf-12luu-rebase`) on `main`, not in the
  shared needle working tree. This is required because the shared tree was checked out on
  `needle/bf-5wku` with another agent's uncommitted `.beads/` work in flight — switching its
  HEAD would have disrupted concurrent agents.
- `git rebase origin/main` — git detected both local commits as "previously applied"
  (cherry-picks already present upstream) and **skipped** them. No conflicts arose.

## Outcome

- Local `main` advanced from `ad877b0` to `531e415` == `origin/main`.
- `main...origin/main` ahead/behind = **0 / 0** (identical).
- This `notes(bf-12luu)` commit is the published catch-up record on `origin/main`.
- Temporary worktree removed after push.
