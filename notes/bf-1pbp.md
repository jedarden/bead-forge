# bf-1pbp: help text for available bf commands

Task: ensure every available `bf` command has `about`/`long_about` help text
and that every flag/arg carries help text in `src/cli/mod.rs`, then verify
`cargo build` succeeds.

## Finding — already complete

The help text the bead asks for is **already present in the tree**. In clap
derive, a `///` doc comment on an enum variant becomes its `about` (first
paragraph) and `long_about` (full multi-paragraph text); a `///` comment on a
field becomes that flag/arg's help. The entire `Commands` enum
(`src/cli/mod.rs:43-773`) and every nested subcommand enum
(`DepCommands`, `LabelCommands`, `CommentsCommands`, `ConfigCommands`,
`AnnotateCommands`, lines 775-990) already carries:

- a concise one-line `///` short-help plus a multi-line long description
  (blank `///` separator) on every command and subcommand, and
- a `///` doc comment on every flag and positional arg in every command.

This is the umbrella audit covering the full command surface; the per-range
sibling audits — `bf-10blr` (core CRUD), `bf-5d4ze` (workflow & maintenance),
`bf-2jgd8` (query & reporting), `bf-2jgd8`'s note, and `bf-2l2v5` (metadata
subcommand groups) — cover the same surface in slices. Together they confirm
the whole CLI is documented.

## Verification

This was verified three independent ways:

1. **Structural scan of the source** — a Python pass over the enum-definition
   region (`src/cli/mod.rs:42-991`) confirms **0** command/subcommand
   variants lack a preceding `///` comment, and **0** `#[arg]`/`#[command]`
   field attributes lack one. (The only attributes without a preceding `///`
   are the four structural `#[command(...)]`/`#[command(subcommand)]`
   attributes on the `Cli` struct itself at lines 24-38, which correctly take
   their `about` from the explicit attribute and never carry doc comments.)
2. **Rendered help sweep** — clap only emits a description when a doc comment
   exists, so a description under `--help` is proof the comment is present.
   `bf <cmd> --help` was run for all **32** top-level commands and all **18**
   nested subcommands (across `dep`/`label`/`comments`/`config`/`annotate`);
   every one exits 0 and renders a non-empty description plus documented
   options.
3. **`cargo build`** — exits 0 with no errors (`target/debug/bf` produced and
   used for the sweep above).

## Result

No source changes required. The CLI's help text is complete across the entire
command surface; `cargo build` is clean.
