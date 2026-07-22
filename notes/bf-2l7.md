# bf-2l7 — "Add --help flag to bf CLI"

**Outcome: No code change needed — the feature already exists, fully, via clap.**

The task asked to "Add a --help flag to src/cli/mod.rs" so that `bf --help`
shows usage information for all commands. Investigation against the live binary
proves **all three acceptance criteria are already satisfied** by clap's
`#[derive(Parser)]` on `Cli` (`src/cli/mod.rs:23`). No edit to `src/cli/mod.rs`
was made — and the obvious edit (a manual `--help` arg) would be actively
harmful, since it conflicts with clap's auto-generated flag.

## Acceptance criteria — all met, verified against `./target/debug/bf`

1. **"Add a --help flag to src/cli/mod.rs"** — present. clap's derive macro on
   `Cli` (`#[derive(Parser)]` at `src/cli/mod.rs:23`, parsed via
   `Cli::parse()` in `run_cli()` at `:992`) auto-derives `-h`/`--help`. A manual
   arg would duplicate the built-in and is rejected by clap, so the correct
   "implementation" is to let clap provide it — which it does.
2. **"Should show usage information for all commands"** — yes. Top-level
   `--help` lists all 32 subcommands with their one-line descriptions plus the
   global options (`-w/--workspace`, `--no-auto-flush`, `-h/--help`,
   `-V/--version`).
3. **"Test with: ./target/debug/bf --help"** — passes, exit code 0.

## Verified help surfaces (all exit 0)

- `bf --help` and `bf -h` → top-level usage + all commands + options.
- `bf <cmd> --help` (e.g. `bf create --help`) → full per-command options with
  the long doc-comment descriptions from the enum variants.
- `bf help <cmd>` (e.g. `bf help list`) → the auto-generated `help` subcommand.
- `bf <group> <sub> --help` (e.g. `bf dep add --help`) → nested subcommand help.

The existing code even leans on this: `src/cli/mod.rs:1006` carries the comment
"clap handles --help automatically, exiting before this point", and the
no-command error message (`:1010`) directs users to `bf --help`.

## Why no code change

- Adding a `#[arg(long)] help: bool` field to `Cli` would collide with clap's
  reserved `help` flag and fail to compile / error at runtime.
- There is nothing else to implement: the flag exists, the listing of all
  commands exists, per-command help exists, and the `version` flag comes along
  for free (`#[command(version = ...)]` at `:25`).

Closed as already-complete; this note is the only artifact.
