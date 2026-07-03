# bf --version Behavior Documentation

## Task: bf-1z7b

Documented on: 2026-07-03

## Current Behavior

### bf --version
```bash
$ bf --version
Error: bf 0.2.0
$ echo $?
1
```

### br --version
```bash
$ br --version
Error: bf 0.2.0
$ echo $?
1
```

## Root Cause Analysis

The "Error:" prefix and exit code 1 are caused by how clap handles version requests in a `Result<()>` context:

### Error Propagation Chain

1. **clap returns an error for version display**: When `--version` is passed, clap returns `Err(clap::Error::new(ErrorKind::DisplayVersion))` containing the version string.

2. **Error propagated via `?` operator**: In `src/main.rs`:
   ```rust
   fn main() -> Result<()> {
       let cli = bead_forge::cli::run_cli()?;  // ? propagates the clap error
       bead_forge::cli::run(cli)
   }
   ```

3. **anyhow formats the error**: Since clap's error is treated as a `Result::Err`, anyhow's error formatter prints it with "Error:" prefix.

### Key Findings

1. **Binary Relationship**: `/home/coding/.local/bin/br` is a symlink to `/home/coding/.local/bin/bf`
   - Both commands invoke the same `bf` binary (50MB, modified 2026-07-02 11:06)
   - This is intentional: bead-forge is a drop-in replacement for beads_rust

2. **Version Source**: Version comes from `Cargo.toml`:
   - `version = "0.2.0"`
   - Read via `env!("CARGO_PKG_VERSION")` in `src/cli/mod.rs:21`

3. **CLI Configuration** (from `src/cli/mod.rs:18-26`):
   ```rust
   #[derive(Parser)]
   #[command(name = "bf")]
   #[command(about = "bead-forge - Drop-in replacement for beads_rust (br)", long_about = None)]
   #[command(version = env!("CARGO_PKG_VERSION"))]
   #[command(propagate_version = true)]
   pub struct Cli {
       #[command(subcommand)]
       pub command: Commands,
   ```

4. **This is NOT standard clap behavior**: clap normally prints `--version` with exit code 0 and no "Error:" prefix. The current behavior is specific to how clap errors are propagated through the `Result<()>` return type.

## Comparison with Standard CLI Behavior

| Aspect | bead-forge current | Standard CLI (expected) |
|--------|-------------------|------------------------|
| Output | `Error: bf 0.2.0` | `bf 0.2.0` |
| Exit code | 1 | 0 |
| Prefix | `Error: ` | None |

## Implications

1. **Scripting**: Scripts checking version with `bf --version` will fail on exit code checks
2. **Error Parsing**: The "Error:" prefix confuses automated tooling expecting clean version strings
3. **User Experience**: "Error:" suggests something went wrong when version display is successful

## Recommended Fix

Handle clap's special error kinds (`DisplayVersion`, `DisplayHelp`) before they reach anyhow's error formatter. This requires modifying `src/main.rs` to catch these errors and exit cleanly.

## Test Commands

```bash
# Test version output and exit code
bf --version 2>&1; echo "Exit code: $?"

# Compare with br (symlink to bf)
br --version 2>&1; echo "Exit code: $?"

# Verify symlink
ls -la ~/.local/bin/br

# Check binary details
file ~/.local/bin/bf
ls -lh ~/.local/bin/bf
```

## Notes

- Documentation captures the current state as of bead-forge 0.2.0 (2026-07-03)
- This is a known issue that should be addressed in a future bead
- The fix requires restructuring how clap errors are handled in `src/main.rs`
