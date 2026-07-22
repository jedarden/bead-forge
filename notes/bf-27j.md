# bf-27j — Test bead update and status changes

Verified the `bf update` title/status lifecycle end-to-end against the built
`bf` 0.3.0 binary (`target/debug/bf`; the PATH `/home/coding/.local/bin/bf`
reports the same version). Tested in an **isolated** workspace
(`~/scratch/bf-27j-test`) so the shared bead-forge `.beads/` was not touched.

## Steps

Created a throwaway bead `bf-34b` in the isolated workspace, then:

1. **Title update** — `bf update bf-34b --title 'New title'` → `Updated bead bf-34b`,
   exit 0. `bf show` now prints `Title: New title`. ✅
2. **Status → in_progress** — `bf update bf-34b --status in_progress` → exit 0.
   `bf show` now prints `Status: in_progress`. ✅ (`in_progress` → `Status::InProgress`.)
3. **Status → done** — `bf update bf-34b --status done` → exit 0.
   `bf show` now prints `Status: done`. ✅ (`done` is not a named status, so
   `Status::from_str` maps it to `Custom("done")`, which persists normally.)
4. **Persistence** — a fresh `bf show bf-34b` process reads back `Title: New title`
   / `Status: done`, and the auto-flushed `issues.jsonl` checkpoint carries
   `title: "New title"`, `status: "done"`. ✅

## Result

All four acceptance criteria pass. No source changes — `bf update` already
handles `--title`, `--status in_progress`, and `--status done` correctly, and the
changes persist across processes and to the JSONL checkpoint.

## Notes

- `Status::from_str` is intentionally permissive (model.rs:131): any unrecognized
  string becomes a `Custom` status rather than erroring, so the parser never
  rejects a status string. This is documented behavior, not a defect, and is
  outside this bead's acceptance criteria.
- Test artifacts live only under `~/scratch/bf-27j-test/.beads/` (untracked,
  outside the repo). The bead-forge repo's `.beads/issues.jsonl` and `beads.db`
  were not modified by this bead.
