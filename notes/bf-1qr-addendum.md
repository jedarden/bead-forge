# bf-1qr — Addendum: corrected `--version` / `Error:` findings (verified)

> This file is an **addendum** to `notes/bf-1qr.md`. It records the independently
> verified root-cause findings for the `--version` flag and the `Error:` prefix. It is
> kept separate so the investigation results are committed and traceable.

## Task

Research how clap handles `--version`; explain why `Error: bf 0.2.0` was reportedly
observed instead of `bf 0.2.0`; classify as bug / config / expected.

## TL;DR (acceptance criteria)

1. **Current behavior of `--version`:** `bf --version` and `bf -V` print
   `bf 0.3.0` to **stdout** and exit **0**. There is **no** `Error:` prefix on the
   version line.
2. **Root cause of the `Error:` prefix:** It is **unrelated to `--version`**. It appears
   only on the *command-failure* path, emitted by the **Rust standard library's
   `Termination` impl** for `fn main() -> Result<T, E>` where `E: Debug`: when `main`
   returns `Err(e)`, the runtime prints `Error: {e:?}` to **stderr** and exits **1**.
3. **clap default vs. config issue?** **Neither.** clap's native `--version` is
   stdout / exit-0 / no-prefix (verified). clap's own errors are lowercase `error:` on
   stderr, exit **2**, for parse failures. The capital-`E` `Error:` is unconditional
   Rust-runtime termination — not a bug, not a config issue.

## Evidence — empirical exit-code / stream matrix

Captured against the installed binary `/home/coding/.local/bin/bf` (`bf 0.3.0`),
separating stdout from stderr with shell redirection:

| Invocation | Code path | Stream | Output | Exit |
|---|---|---|---|---|
| `bf --version` | `main.rs:7-10` fast path | **stdout** | `bf 0.3.0` | **0** |
| `bf -V` | `main.rs:7-10` fast path | **stdout** | `bf 0.3.0` | **0** |
| `bf show bf-doesnotexist` (cmd fail) | `run()`→`Err`→`main`→**std Termination** | **stderr** | `Error: Bead not found: bf-doesnotexist` | **1** |
| `bf bogus-subcommand` (parse fail) | clap `Cli::parse()` in `run_cli` | **stderr** | lowercase `error: unrecognized subcommand …` | **2** |
| `bf --bogus` (unknown flag) | clap `Cli::parse()` in `run_cli` | **stderr** | lowercase `error: unexpected argument '--bogus' found` | **2** |

Byte-exact checks:
- `bf --version 2>&1 1>/dev/null` → **empty** (stderr has nothing; line is stdout-only).
- `bf --version 2>/dev/null` → `bf 0.3.0` on stdout, exit 0.
- `bf show bf-doesnotexist 2>/dev/null` → empty stdout; `Error:` message is stderr-only.

The version line and the `Error:` line are on disjoint code paths *and* disjoint
streams; they can never co-occur.

## Root cause — reproduced independently

bead-forge's `main` returns `Result` and propagates errors via `?` (`src/main.rs`):

```rust
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && (args[1] == "--version" || args[1] == "-V") {
        println!("bf {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);               // --version exits HERE; never returns Err
    }
    let cli = bead_forge::cli::run_cli()?;   // run_cli() -> Result<Cli>  (cli/mod.rs:975)
    bead_forge::cli::run(cli)                // run(cli)  -> Result<()>  (cli/mod.rs:979)
}                                            // Err propagates out of main
```

Any `Err` from `run` propagates out of `main`; the Rust runtime then prints
`Error: {e:?}` to stderr and exits 1.

**Independent reproduction** (scratch program mirroring `main`'s shape, no clap, no bf
code):

```rust
#[derive(Debug)]
struct MyErr(String);
fn main() -> Result<(), MyErr> {
    let _cli: () = parse()?;
    Ok(())
}
fn parse() -> Result<(), MyErr> {
    Err(MyErr("simulated command failure".into()))
}
```

Observed: `Error: MyErr("simulated command failure")` on **stderr**, **0 bytes** on
stdout, exit **1**. The exact `Error: {Debug}` / stderr / exit-1 signature reproduces
with zero involvement from clap or bead-forge — confirming the prefix is std-runtime
behavior, not application code. (`grep "Error:"` in `src/` finds only two unrelated
`format_error` formatters in `src/format/text.rs` and `src/format/toon.rs`; the
observed prefix does not originate in this tree — it lives in `std::process::Termination`.)

## Why `--version` never gets the prefix

`--version` is intercepted before clap parsing (`main.rs:7-10`): it `println!`s to
stdout and calls `std::process::exit(0)`, so it never reaches `run()`/`run_cli()`,
never returns `Err`, and exits 0. No prefix can attach.

(The intercept is redundant: clap already wires `--version` natively via
`#[command(version = env!("CARGO_PKG_VERSION"))]` at `cli/mod.rs:25`, which also prints
`bf 0.3.0` to stdout / exit 0 — see bf-31zx's clean clap-only control. Candidate for
future simplification; out of scope here.)

## Reconciling the reported `Error: bf 0.2.0`

That exact string is unreproducible from the current tree (`--version` is stdout/exit-0;
version is now 0.3.0). Most plausible origins:
- A **stale 0.2.0 binary** built before the `main.rs` stdout fast path (pre-commits
  `aa5d885` / `f5e7856`), where version output once went through an error path.
- A **shell alias/wrapper** prefixing output, or **stdout/stderr interleaving** in a
  capturing harness.
- A typo'd flag clap didn't recognize as `--version` → unknown-argument parse error →
  lowercase `error:` / stderr / exit 2 (lowercase, not capital `Error:`).

## Classification

- **Bug?** No. `--version` is correct; `Error:` on failures is by-design Rust termination.
- **clap default?** No. clap's `--version` is stdout/exit-0/no-prefix; clap errors are
  lowercase `error:` / stderr / exit 2.
- **Config issue?** No. Nothing toggles the capital-`E` `Error:` — unconditional std behavior.

## Conclusion

`bf --version` is clean: `bf 0.3.0`, stdout, exit 0, no prefix. The `Error:` prefix is
not a version problem and not clap — it is the Rust runtime terminating a
`Result`-returning `main` on command failures (`Error: {:?}` → stderr, exit 1). Expected,
repo-independent std behavior.

## Files referenced

- `src/main.rs:3-14` — `fn main() -> Result<()>`; `--version` fast path and `?` propagation.
- `src/cli/mod.rs:975` — `pub fn run_cli() -> Result<Cli>`.
- `src/cli/mod.rs:979` — `pub fn run(cli: Cli) -> Result<()>`.
- `src/cli/mod.rs:24-25` — `#[command(name = "bf")]`, `#[command(version = env!("CARGO_PKG_VERSION"))]`.
- `Cargo.toml:3` — `version = "0.3.0"`.

## Related beads

- bf-3392, bf-31zx, bf-5k3l, bf-um3e — corroborate these findings (um3e is the root-cause correlation).
