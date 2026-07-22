# bf-10blr: help text for core CRUD bead commands

Task: audit and complete clap help (doc-comment) text for the core
bead-lifecycle subcommands in `src/cli/mod.rs` — `create`, `list`, `show`,
`update`, `close`, `reopen`, `delete`.

## Finding — already complete

The help text the bead asks for is **already present in the tree**. Every one
of the seven variants in the `Commands` enum (`src/cli/mod.rs:43-223`) already
carries:

- a concise one-line `///` short-help, and
- a multi-line long description separated by a blank `///` line, and
- a `///` doc comment on every flag and positional arg.

This was verified two independent ways:

1. **Source inspection** — each variant and each field under it has a `///`
   comment (cross-checked field-by-field against the `match` arms in `run()`
   at `src/cli/mod.rs:1038-1116`).
2. **Rendered output** — clap only emits a flag/arg description when a doc
   comment exists, so a description appearing under `--help` is proof the
   comment is present. All seven render every option with a description.

### Per-variant coverage

| Variant  | Short-help           | Long desc | Flags/args all documented |
|----------|----------------------|-----------|---------------------------|
| create   | "Create a new bead"  | yes       | title, type, priority, description, assignee, label, json |
| list     | "List beads"         | yes       | status, type, assignee, priority, annotation, limit, all, format, json |
| show     | "Show bead details"  | yes       | id, format, json |
| update   | "Update a bead"      | yes       | id, title, status, priority, assignee, clear-assignee, description, acceptance-criteria, notes, design, due-at |
| close    | "Close a bead"       | yes       | id, reason |
| reopen   | "Reopen a bead"      | yes       | id |
| delete   | "Delete a bead"      | yes       | id |

## Verification (this bead)

No source changes were needed, so the work was build + render verification:

- `cargo build` — clean, 0 errors.
- `bf {create,list,show,update,close,reopen,delete} --help` — each exits 0
  and renders a description line for every listed option and positional,
  plus the short-help header and (for each) a multi-line long description.
- The two global flags (`-w/--workspace`, `--no-auto-flush`) also render on
  every subcommand, inherited from the top-level `Cli` struct.

## Files

- new: `notes/bf-10blr.md` (this file)
- no source changes — help text already present.
