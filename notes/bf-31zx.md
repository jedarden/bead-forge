# bf-31zx — Research: clap `--version` flag defaults

**Task:** Investigate how clap handles `--version` flags by default.
**Scope:** clap v4 (bead-forge pins `clap = { version = "4", features = ["derive"] }` in `Cargo.toml:11`; latest at time of writing is 4.6.x).

## TL;DR (answers to the acceptance criteria)

1. **Does clap add an `Error:` prefix to `--version` output by default?** → **NO.** The `error:` prefix is reserved for *parse failures* (missing required args, invalid values, conflicts) and those go to **stderr** with exit code **2**. Version output is plain `name version` to **stdout** with exit code **0**.
2. **Default output format:** `<bin_name> <version>\n` — e.g. `bf 0.3.0`, `MyApp 1.0`. Single line, trailing newline, no decoration.
3. **How the flag gets enabled:** clap auto-adds `-V, --version` whenever a version is set on the `Command` — via `#[command(version = "...")]`, `#[command(version)]` (reads `Cargo.toml`), or `.version(...)`. If no version is set, the flag is absent.
4. **Common patterns:** `#[command(version)]` (Cargo.toml-derived, recommended), `propagate_version = true` to expose it on subcommands, `disable_version_flag = true` to suppress it and roll your own.

## 1. How clap handles `--version` by default

clap auto-generates a `-V, --version` flag the moment the `Command` has a version string. There is no manual `#[arg]` field needed. Three ways to supply the version (derive API):

```rust
// (a) Explicit literal
#[command(version = "1.0")]

// (b) From Cargo.toml — recommended; expands to crate_version!() / CARGO_PKG_VERSION
#[command(version)]

// (c) From an env var (what bead-forge does)
#[command(version = env!("CARGO_PKG_VERSION"))]
```

This is exactly what bead-forge uses (`src/cli/mod.rs:25`):

```rust
#[command(name = "bf")]
#[command(version = env!("CARGO_PKG_VERSION"))]
```

So clap *would* handle `bf --version` natively — printing `bf 0.3.0` to stdout and exiting 0.

## 2. Version output format — examples

Straight from the official clap derive tutorial (`docs.rs/clap/4.6.x`):

```
$ 02_apps_derive --version
MyApp 1.0

$ 02_crate_derive --version      # #[command(version)] → reads Cargo.toml
clap [..]
```

Format is always `<name> <version>\n`. The `<name>` is the `#[command(name = ...)]` value (or the crate name / binary name if unset); the version is the supplied string. **No banner, no author, no license, no color** by default.

## 3. The `Error:` prefix question — definitively NO

This is the crux of the investigation. Observed clap behavior:

| Output | Stream | Exit code | Prefixed? |
|---|---|---|---|
| `--version` | **stdout** | **0** | no — plain `name version` |
| `--help` | **stdout** | **0** | no |
| parse error (missing arg, bad value, conflict) | **stderr** | **2** | yes — `error: <message>` |

From the tutorial, error output looks like:

```
$ 03_03_positional_derive
? 2
error: the following required arguments were not provided:
  <NAME>

Usage: 03_03_positional_derive[EXE] <NAME>

For more information, try '--help'.
```

The lowercase `error:` prefix (note: clap uses lowercase `error:`, not `Error:`) appears **only** on `DisplayHelp`/`DisplayVersion`-rejected paths that represent genuine parse failures. Version output is a *successful, intentional* emission — it never touches the error formatter.

**Empirical confirmation in this repo** (bead-forge):
```
$ cargo run -- --version
bf 0.3.0           # ← stdout, no prefix
$ echo $? 
0
```
Routing `--version` to stdout-only vs stderr-only showed stdout = `bf 0.3.0`, stderr = empty — consistent with clap's documented behavior.

> ⚠️ Note: that output currently comes from a **manual** handler in `src/main.rs:7-10`, not directly from clap (see §5). But the manual handler prints exactly what clap would print natively.

## 4. Common patterns for version output

- **Read from `Cargo.toml`** (most common, zero-maintenance):
  ```rust
  #[derive(Parser)]
  #[command(version, about, long_about = None)]  // version + about pulled from Cargo.toml
  struct Cli { ... }
  ```
- **Propagate to subcommands** so `bf create --version` also works:
  ```rust
  #[command(version, propagate_version = true)]
  ```
  Without this, only the top-level command gets `--version`.
- **Custom/multi-line version** (e.g. you want build hash + harness version): disable the built-in flag and add your own:
  ```rust
  #[command(disable_version_flag = true)]
  struct Cli {
      #[arg(short = 'V', long, action = clap::ArgAction::Version)]
      version: bool,
  }
  // then handle it yourself, or use Cli::command().render_version()
  ```
- **`Command::render_version()`** — the builder method that produces the `name version\n` string if you want to print it yourself (useful when wiring it into a richer version subcommand).
- **Rich/colored output** — third-party crates like `clap-version-flag` exist precisely because the default is plain text; only reach for these if you want color/banner formatting.

## 5. Relevance to bead-forge

`src/main.rs:4-10` intercepts `--version`/`-V` *before* clap and prints it manually:

```rust
// Handle version flag before clap parsing to output to stdout
if args.len() >= 2 && (args[1] == "--version" || args[1] == "-V") {
    println!("bf {}", env!("CARGO_PKG_VERSION"));
    std::process::exit(0);
}
let cli = bead_forge::cli::run_cli()?;
```

The inline comment implies this was added to *force stdout*. But per this research, **clap already routes `--version` to stdout (exit 0) by default** — it does not go to stderr and does not get an `error:` prefix. So if the manual handler was added to fix a "version printed to stderr" or "Error: prefix" symptom, that symptom was **not** clap's default behavior. Likely causes of such a symptom elsewhere would be: a typo'd version flag that clap didn't recognize as the version flag (so it became an *unknown argument* parse error → `error:` prefix → stderr), or the version being emitted via an `anyhow`/error path rather than clap's version path.

Because `#[command(version = env!("CARGO_PKG_VERSION"))]` is already set on `Cli`, **removing the manual handler would let clap produce identical `bf 0.3.0` / stdout / exit-0 output natively.** The manual handler is redundant *unless* there's a known reason it's needed (e.g. a subcommand-only parser, or wanting to bypass clap's arg setup cost). Any follow-up work to simplify this should verify the exact historical motivation before deleting it — flag as a separate task, not done here.

## Sources

- Official clap derive tutorial (v4.6.x): https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html
- clap crate docs: https://docs.rs/clap
- clap-version-flag (colored override, shows what the default is *not*): https://crates.io/crates/clap-version-flag
