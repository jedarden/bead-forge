# bf-3rwg: Verify clap CLI --help support

## Implementation Status: ✅ VERIFIED (no code changes needed)

The clap derive-based CLI in `src/cli/mod.rs` already fully supports `--help` (and `-h`) at
both the top level and every subcommand. This bead confirms it works end-to-end against the
current `bf` build. No source changes were required.

> Note: a prior draft of this note (2026-07-09) referenced line numbers and APIs that have
> since drifted (`Cli::try_parse()` + `clap::error::ErrorKind`, then at line ~694). The
> current source uses `Cli::parse()` — figures below are re-verified against today's tree.

## Acceptance criteria

| # | Criterion | Result |
|---|-----------|--------|
| 1 | `src/cli/mod.rs` has a clap Command structure | ✅ `#[derive(Parser)]` struct `Cli` with `#[command(name = "bf")]` (`src/cli/mod.rs:23-34`) |
| 2 | The CLI app can be instantiated without errors | ✅ `run_cli()` returns `Ok(Cli::parse())` (`src/cli/mod.rs:805-806`); builds clean |
| 3 | clap derives are properly configured for help generation | ✅ `use clap::{Parser, Subcommand}` (line 16); `#[derive(Parser)]` (line 23), `#[derive(Subcommand)]` (line 36); `#[command(version = …)]` / `#[command(about = …, long_about = None)]` |
| 4 | `cargo build` succeeds for the CLI module | ✅ exit 0, no errors |

clap's `Parser` derive injects `-h`/`--help` (and `-V`/`--version`) automatically — there is
nothing extra to wire up. Each `#[derive(Subcommand)]` variant (top-level `Commands` plus the
nested `DepCommands`, `LabelCommands`, `CommentsCommands`, `ConfigCommands`, `AnnotateCommands`)
gets its own `--help` for free.

## Verification

```bash
$ cargo build                                           # exit 0, no errors

$ ./target/debug/bf --help
bead-forge - Drop-in replacement for beads_rust (br)

Usage: bf [OPTIONS] [COMMAND]

Commands:
  create         Create a new bead
  list           List beads
  ...  (show, update, close, reopen, delete, ready, claim, init, sync,
        doctor, merge-jsonl, commit-check, count, batch, mitosis, dep,
        label, labels, comments, search, stats, schema, config, velocity,
        annotate, log, critical-path, rotate, migrate)
$ ./target/debug/bf --help >/dev/null; echo $?          # exit 0

$ ./target/debug/bf claim --help                        # subcommand help also works
Claim a bead (atomic)

Usage: bf claim [OPTIONS] --assignee <ASSIGNEE>
...
  -h, --help            Print help
```

Top-level help, version, and per-subcommand help all render and exit 0. clap also emits the
canonical usage/error banner on bad input (e.g. `bf claim` is `--assignee`-only and rejects a
stray positional ID), confirming the derive pipeline is active end-to-end.

## Build

`cargo build` — clean, no errors.

## Conclusion

All four acceptance criteria pass on the current tree. The clap `Parser`/`Subcommand` derive
configuration is correct, so `--help`/`-h` work at the top level and for every command. No bugs
found, no code changes needed.
