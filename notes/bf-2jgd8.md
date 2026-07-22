# bf-2jgd8: help text for query & reporting commands

Task: audit and complete clap help (doc-comment) text for the query and
reporting subcommands in `src/cli/mod.rs` — `search`, `stats`, `count`,
`recent`, `log`, `velocity`, `critical-path`, `labels`, `schema`.

## Finding — already complete

The help text the bead asks for is **already present in the tree**. Every one
of the nine variants in the `Commands` enum (`src/cli/mod.rs:43-773`) already
carries:

- a concise one-line `///` short-help, and
- a multi-line long description separated by a blank `///` line (where useful —
  even the lighter ones like `count` and `labels` have both), and
- a `///` doc comment on every flag and positional arg.

This is the same outcome as the sibling audits `bf-10blr` (core CRUD commands)
and `bf-5d4ze` (workflow & maintenance commands).

This was verified two independent ways:

1. **Source inspection** — each variant and each field under it has a `///`
   comment (cross-checked field-by-field against the `match` arms in `run()`).
2. **Rendered output** — clap only emits a flag/arg description when a doc
   comment exists, so a description appearing under `--help` is proof the
   comment is present. All nine render every option with a description.

### Per-variant coverage

| Variant        | Short-help                                                       | Long desc | Flags/args all documented                                      |
|----------------|------------------------------------------------------------------|-----------|----------------------------------------------------------------|
| search         | "Search beads"                                                   | yes       | query, status, type, assignee, label, priority-min, priority-max, limit, format |
| stats          | "Show statistics"                                                | yes       | by-type, by-priority, by-assignee, by-label, format            |
| count          | "Count beads"                                                    | yes       | status                                                         |
| recent         | "Show recently modified beads"                                   | yes       | status, type, assignee, priority, since, before, time-period, limit, format, json |
| log            | "Show event log for a bead"                                      | yes       | id, limit, since, actor, status-changes, diff, git, format, json |
| velocity       | "Show velocity stats (bead-forge specific)"                      | yes       | model, harness, format                                         |
| critical-path  | "Show critical path (longest chain of blocking dependencies)"    | yes       | id, max-depth, format                                          |
| labels         | "List labels for a specific issue (direct SELECT, efficient)"    | yes       | id, format                                                     |
| schema         | "Emit JSON Schema"                                               | yes       | target, format                                                 |

## Verification (this bead)

No source changes were needed, so the work was build + render verification:

- `cargo build` — clean, 0 errors.
- `bf {search,stats,count,recent,log,velocity,critical-path,labels,schema} --help`
  — each exits 0 and renders the short-help header plus a multi-line long
  description, with a description line for every listed option and positional.
  `log` (27 lines) and `recent` (31 lines) — the two most flag-heavy — were
  inspected in full; every flag carries a description, confirming no bare
  `--flag` is missing its `///` comment.
- The two global flags (`-w/--workspace`, `--no-auto-flush`) also render on
  every subcommand, inherited from the top-level `Cli` struct.

## Files

- new: `notes/bf-2jgd8.md` (this file)
- no source changes — help text already present.
