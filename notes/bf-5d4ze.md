# bf-5d4ze: help text for workflow & maintenance commands

Task: audit and complete clap help (doc-comment) text for the workflow and
maintenance subcommands in `src/cli/mod.rs` — `init`, `claim`, `ready`,
`batch`, `mitosis`, `sync`, `doctor`, `rotate`, `migrate`, `merge-jsonl`,
`commit-check`.

## Finding — already complete

The help text the bead asks for is **already present in the tree**. Every one
of the eleven variants in the `Commands` enum (`src/cli/mod.rs:43-773`) already
carries:

- a concise one-line `///` short-help, and
- a multi-line long description separated by a blank `///` line (where useful —
  the flagless `commit-check` still has both), and
- a `///` doc comment on every flag and positional arg.

This is the same outcome as the sibling audit `bf-10blr` (core CRUD commands).

This was verified two independent ways:

1. **Source inspection** — each variant and each field under it has a `///`
   comment (cross-checked field-by-field against the `match` arms in `run()`).
2. **Rendered output** — clap only emits a flag/arg description when a doc
   comment exists, so a description appearing under `--help` is proof the
   comment is present. All eleven render every option with a description.

### Per-variant coverage

| Variant      | Short-help                                        | Long desc | Flags/args all documented                       |
|--------------|---------------------------------------------------|-----------|-------------------------------------------------|
| init         | "Initialize a new workspace"                      | yes       | prefix                                          |
| claim        | "Claim a bead (atomic)"                           | yes       | assignee, model, harness, harness-version, any, fallback, workspace-paths, dry-run, format, json |
| ready        | "Show ready (unblocked) beads"                    | yes       | limit, format, json                             |
| batch        | "Batch operations (atomic)"                       | yes       | file, json, stdin                               |
| mitosis      | "Mitosis: split a bead into children atomically"  | yes       | id, children, reason, format                    |
| sync         | "Sync (flush to JSONL or import from JSONL)"      | yes       | flush-only, import-only                         |
| doctor       | "Doctor - check and repair"                       | yes       | repair, flush-first, force, reclaim-stale, ttl, fix-schema |
| rotate       | "Rotate (archive) closed beads older than threshold" | yes    | days, dry-run                                   |
| migrate      | "Migrate workspace from br to bf"                 | yes       | workspace, from-jsonl, seed-velocity, dry-run, skip-verify |
| merge-jsonl  | "Three-way merge of JSONL bead files..."          | yes       | base, ours, theirs, output                      |
| commit-check | "Commit check - scan staged .beads/ changes..."   | yes       | (none)                                          |

## Verification (this bead)

No source changes were needed, so the work was build + render verification:

- `cargo build` — clean, 0 errors.
- `bf {init,claim,ready,batch,mitosis,sync,doctor,rotate,migrate,merge-jsonl,commit-check} --help`
  — each exits 0 and renders the short-help header plus a multi-line long
  description, with a description line for every listed option and positional.
  `claim` (47 lines) and `doctor` (33 lines) — the two most flag-heavy — were
  inspected in full; every flag carries a description, confirming no bare
  `--flag` is missing its `///` comment.
- The two global flags (`-w/--workspace`, `--no-auto-flush`) also render on
  every subcommand, inherited from the top-level `Cli` struct.

## Files

- new: `notes/bf-5d4ze.md` (this file)
- no source changes — help text already present.
