# bf-2l2v5: help text for metadata subcommand groups

Task: audit and complete clap help (doc-comment) text for the metadata
subcommand groups in `src/cli/mod.rs` — `DepCommands` (`dep` add/remove/list/tree),
`LabelCommands`, `CommentsCommands`, `AnnotateCommands`, and `ConfigCommands`.
Then run a full-CLI verification pass (`bf --help` + the five group `--help`s).

## Finding — already complete

The help text the bead asks for is **already present in the tree**. Every one
of the five group enums (`src/cli/mod.rs:775-990`) already carries:

- a concise one-line `///` short-help on the parent group and on each child
  subcommand, plus a multi-line long description separated by a blank `///`
  line (where useful), and
- a `///` doc comment on every flag and positional arg in every child.

This is the same outcome as the sibling audits `bf-10blr` (core CRUD commands),
`bf-5d4ze` (workflow & maintenance commands), and `bf-2jgd8` (query & reporting
commands). Together those four audits plus this one cover the entire `Commands`
enum and every nested subcommand enum.

This was verified two independent ways:

1. **Source inspection** — each group, each child variant, and each field under
   them has a `///` comment (cross-checked field-by-field against the `match`
   arms in `cmd_dep` / `cmd_label` / `cmd_comments` / `cmd_annotate` /
   `cmd_config`).
2. **Rendered output** — clap only emits a flag/arg description when a doc
   comment exists, so a description appearing under `--help` is proof the
   comment is present. Every child renders every option and positional with a
   description.

### Per-group coverage

| Group / child  | Short-help                                        | Long desc | Flags/args all documented                |
|----------------|---------------------------------------------------|-----------|------------------------------------------|
| **dep**        | "Manage dependencies"                             | yes       | (group)                                  |
| └ add          | "Add a dependency"                                | yes       | blocks, blocker, type                    |
| └ remove       | "Remove a dependency"                             | yes       | issue, depends-on                        |
| └ list         | "List dependencies of an issue"                   | yes       | id                                       |
| └ tree         | "Show dependency tree rooted at issue"            | yes       | id, direction, max-depth, format, json   |
| **label**      | "Manage labels"                                   | yes       | (group)                                  |
| └ add          | "Add label(s) to an issue"                        | yes       | label, id                                |
| └ remove       | "Remove label(s) from an issue"                   | yes       | label, id                                |
| └ list         | "List labels for an issue or all unique labels"   | yes       | id (optional)                            |
| **comments**   | "Manage comments"                                 | yes       | (group)                                  |
| └ add          | "Add a comment"                                   | yes       | id, text                                 |
| └ list         | "List comments for an issue"                      | yes       | id                                       |
| **annotate**   | "Manage annotations"                              | yes       | (group)                                  |
| └ set          | "Set an annotation"                               | yes       | id, key, value                           |
| └ get          | "Get an annotation"                               | yes       | id, key                                  |
| └ remove       | "Remove an annotation"                            | yes       | id, key                                  |
| └ list         | "List all annotations for an issue"               | yes       | id                                       |
| └ clear        | "Clear all annotations for an issue"              | yes       | id                                       |
| **config**     | "Configuration management"                        | yes       | (group)                                  |
| └ list         | "List all config values"                          | yes       | (none)                                   |
| └ get          | "Get a specific config value"                     | yes       | key                                      |
| └ set          | "Set a config value"                              | yes       | key, value                               |
| └ path         | "Show config file path"                           | yes       | (none)                                   |

## Verification (this bead)

No source changes were needed, so the work was build + render verification:

- `cargo build` — clean, 0 errors.
- `bf --help` — lists every top-level command (all 32 + auto `help`) with a
  one-line description.
- `bf {dep,label,comments,annotate,config} --help` — each exits 0 and renders
  the parent group's short-help header plus a multi-line long description, with
  a one-line description for every child subcommand.
- Leaf spot-checks `bf {dep add, dep tree, label add, comments add, annotate set,
  config set} --help` — each exits 0 and renders a description for every listed
  positional and option. `dep tree` (the most flag-heavy child, 5 options) was
  inspected in full; every flag carries a description, confirming no bare
  `--flag` is missing its `///` comment.
- The two global flags (`-w/--workspace`, `--no-auto-flush`) also render on
  every subcommand, inherited from the top-level `Cli` struct.

## Files

- new: `notes/bf-2l2v5.md` (this file)
- no source changes — help text already present.
