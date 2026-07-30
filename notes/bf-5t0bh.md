# bf-5t0bh — Record rotation interplay resolution on parent bf-1wg2v

Auto-split child 3/4 of bf-bziwd. No code change — this bead existed only because one
acceptance criterion of bf-bziwd was never met: the plan §7.1 Open Question about
rotation interplay was never recorded as a comment on the parent bf-1wg2v (its comments
listed only the child-1-of-5 dirty-tracking note, [19]).

## What I did

Verified the incremental auto-flush code path scopes writes to the active `issues.jsonl`
only and never opens or writes rotated archives, then posted the resolution as a comment.

### Code path (single resolution site, by construction)

- `src/autoflush.rs::after_mutation` → `run()` → `crate::sync::flush_dirty` (autoflush.rs:71-73).
  Hard-delete variant: `after_delete` → `flush_after_delete`.
- `src/sync.rs::flush_dirty` (sync.rs:98) resolves the export target in exactly ONE place:
  `let jsonl_path = beads_dir.join(&metadata.jsonl_export);` (sync.rs:104) → the active
  `issues.jsonl`. It hands that path to `export_jsonl_dirty`.
- `src/jsonl.rs::export_jsonl_dirty` (jsonl.rs:188) → `export_jsonl_merge(path, &issues, &[])`
  (jsonl.rs:202). `export_jsonl_merge` (jsonl.rs:104) reads/writes only the `path` it is
  given, via atomic temp+rename, and never enumerates or opens numbered archives. Archives
  are written EXCLUSIVELY by `crate::rotate::rotate`.

Because the active path is the sole resolution on this path, an archive path is never
constructed here — the guarantee holds by construction; no runtime guard needed. Doc
comments on `flush_dirty` and `export_jsonl_dirty` already mark this RESOLVED.

### Confirming tests (both pass)

- `tests/autoflush_diagnostics_and_rotation.rs::autoflush_targets_only_active_jsonl_not_archives`
  (line 217): workspace with a hand-written `issues.jsonl.1` archive mutated via `bf update`
  (auto-flush); asserts the active file changes in content AND mtime while the archive stays
  byte-for-byte identical with unchanged mtime, and the mutated bead does not leak into the
  archive.
- `tests/batch_cascade_and_rotation.rs::incremental_flush_targets_only_active_jsonl_not_archive`
  (line 182): same invariant from the batch path.

## Deliverable

Posted comment **[25]** on `bf-1wg2v` documenting the resolution, citing the code path and
both confirming tests. Closes plan §7.1 Open question (`docs/plan/plan.md:1389-1390`).
