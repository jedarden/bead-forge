# bf-5k3l — Locate version output code

**Task:** Find the specific code that generates the version output, and locate where (if anywhere) the `Error:` prefix is added to it.

## TL;DR

- **Version message is constructed in `src/main.rs:7-10`** — a fast path that intercepts `--version`/`-V` *before* clap parses, prints `bf <version>` to **stdout**, and `exit(0)`.
- **The `Error:` prefix is NOT added to version output.** Empirically `bf --version` prints `bf 0.3.0` to stdout, exit 0, with **nothing on stderr**.
- The `Error:` prefix string lives only in the **error-rendering** formatters (`src/format/text.rs:41`, `src/format/toon.rs:40`), which are never reached by the version path.

## Empirical verification

```
$ bf --version
bf 0.3.0          # stdout
                  # stderr: empty
$ echo $?
0
```

Confirmed via `cargo run --quiet -- --version`: identical bytes on stdout, empty stderr, exit 0.

## Where the version message is constructed

| Location | Role |
|----------|------|
| `src/main.rs:7-10` | **Runtime-active path.** Intercepts `--version`/`-V` before clap: `println!("bf {}", env!("CARGO_PKG_VERSION"))` → `std::process::exit(0)`. This is what actually fires. |
| `src/cli/mod.rs:21` | `pub const VERSION: &str = env!("CARGO_PKG_VERSION")` — exported constant (library-use). |
| `src/cli/mod.rs:25` | clap derive `#[command(version = env!("CARGO_PKG_VERSION"))]` — clap's *own* `--version` handler. **Never fires** because `main.rs` intercepts the flag first. Kept for help-text consistency. |
| `Cargo.toml:3` | `version = "0.3.0"` — the source feeding `CARGO_PKG_VERSION`. |

The active code, verbatim (`src/main.rs:3-14`):

```rust
fn main() -> Result<()> {
    // Handle version flag before clap parsing to output to stdout
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 2 && (args[1] == "--version" || args[1] == "-V") {
        println!("bf {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    let cli = bead_forge::cli::run_cli()?;
    bead_forge::cli::run(cli)
}
```

Because this `println!`s directly to stdout and returns/exits before any formatter runs, **no `Error:` prefix can ever be prepended to the version line.**

## Where the `Error:` prefix actually lives

It exists only as the error-message renderer for the text/toon output formatters — used when a *command* fails, not for `--version`:

| File:line | Code |
|-----------|------|
| `src/format/text.rs:41` | `format!("Error: {}\n", message)` — impl of `Formatter::format_error` for `TextFormatter` |
| `src/format/toon.rs:40` | `format!("Error: {}\n", message)` — impl of `Formatter::format_error` for `ToonFormatter` |
| `src/format/mod.rs` (trait) | declares `fn format_error(&self, message: &str) -> String;` |

These are invoked only through `get_formatter(...).format_error(...)` on a command error — a code path entirely disjoint from the `--version` fast path in `main.rs`.

(Note: clap itself prints `error:` (lowercase) to **stderr**, exit **2**, for *parse failures* like unknown flags — again, not version. See bf-31zx for the clap details.)

## Conclusion

If `Error:` was observed prefixing version output, it did **not** come from bead-forge's code. Possible external causes worth checking: a wrapper/shell alias, an outdated pre-`main.rs`-intercept binary on `$PATH` (this fast path was added to route version to stdout — see commits `aa5d885`/`f5e7856`), or stderr/stdout interleaving in a capturing harness. The current tree, built and run, produces a clean `bf 0.3.0` on stdout with exit 0.
