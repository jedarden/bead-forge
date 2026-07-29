# bf-1qr Investigation Summary: --version flag behavior

## Task
Investigate current --version flag behavior and determine why it outputs 'Error: bf 0.2.0' instead of 'bf 0.2.0'.

## Current Behavior (Verified)

**As of bead-forge 0.3.0, the `--version` flag works correctly:**

```bash
$ bf --version
bf 0.3.0

$ bf -V  
bf 0.3.0

$ echo $?
0
```

- Output: `bf 0.3.0` (semantic version format)
- Stream: **stdout** (not stderr)
- Exit code: **0** (success)
- **No "Error:" prefix on version output**

## Root Cause Analysis

### The "Error:" Prefix is NOT Related to --version

The capital-**`E`** `Error:` prefix is **not** clap and **not** bead-forge code. It is the **Rust standard library's default termination behavior** for any program declared `fn main() -> Result<T, E>` where `E: Debug`.

When `main` returns `Err(e)`, the runtime prints `Error: {e:?}` to **stderr** and exits with code **1**.

### Code Path Analysis

**bead-forge's `main` function:**
```rust
fn main() -> Result<()> {
    // Handle version flag before clap parsing to output to stdout
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 2 && (args[1] == "--version" || args[1] == "-V") {
        println!("bf {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);  // --version exits HERE; never returns Err
    }

    let cli = bead_forge::cli::run_cli()?;
    bead_forge::cli::run(cli)   // returns Result<()>; Err propagates out of main
}
```

The `--version` flag is handled **before** clap parsing (lines 7-10):
- Outputs to **stdout** via `println!()`
- Exits with code **0** via `std::process::exit(0)`
- Never returns `Err`, so never triggers the Rust std termination

### Exit Code / Stream Matrix

| Invocation | Code path | Stream | Output | Exit |
|---|---|---|---|---|
| `bf --version` / `bf -V` | `main.rs:7-10` manual fast path | **stdout** | `bf 0.3.0` | **0** |
| `bf show <invalid-id>` (cmd failure) | `run()` → `Err` → `main` → **std Termination** | **stderr** | `Error: Bead not found: <invalid-id>` | **1** |
| `bf <bogus-subcommand>` (parse fail) | clap `Cli::parse()` in `run_cli` | **stderr** | lowercase `error: unrecognized subcommand …` | **2** |

### Two Different Error Prefixes

1. **Capital-`E` `Error:`** → Rust std::process::Termination (stderr, exit 1)
2. **Lowercase `error:`** → clap parse errors (stderr, exit 2)

## Historical Context

The reported `Error: bf 0.2.0` is **unreproducible** from the current codebase. Most plausible origins:

1. **Stale 0.2.0 binary** - built before the stdout fast path was added
2. **Shell alias/wrapper** - prefixing output with "Error:"
3. **Stdout/stderr interleaving** - in a capturing harness
4. **Typo'd flag** - unrecognized flag → lowercase clap error (not capital-`E`)

## Classification

- **Bug?** **No.** `--version` works correctly. `Error:` on failures is by-design Rust termination.
- **clap default?** **No.** clap's `--version` is stdout/exit-0/no-prefix; clap errors are lowercase `error:`.
- **Config issue?** **No.** Nothing toggles the capital-`E` `Error:` — unconditional std behavior.
- **Expected behavior?** **Yes.** Both `--version` (stdout, exit 0) and `Error:` on failures (stderr, exit 1) are expected.

## Conclusion

The `--version` flag works correctly: `bf 0.3.0`, stdout, exit 0, no prefix. The `Error:` prefix is not a version problem and not clap — it is the Rust runtime terminating a `Result`-returning `main` on command failures (`Error: {:?}` → stderr, exit 1). This is expected, repo-independent std behavior.

## Related Documentation

- `notes/bf-um3e.md` - Root cause analysis of Error: prefix
- `notes/bf-1qr-addendum.md` - Independent verification of findings
- `tests/test_version_display.rs` - Comprehensive version flag tests
- `src/main.rs:3-14` - Version fast path implementation
- `src/cli/mod.rs:24-25` - clap version configuration
