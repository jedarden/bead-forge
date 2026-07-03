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

## Key Findings

1. **Binary Relationship**: `/home/coding/.local/bin/br` is a symlink to `/home/coding/.local/bin/bf`
   - Both commands invoke the same `bf` binary
   - This is intentional: bead-forge is a drop-in replacement for beads_rust

2. **Version Source**: Version comes from `Cargo.toml`:
   - `bead-forge = "0.2.0"`
   - Read via `env!("CARGO_PKG_VERSION")` in `src/cli/mod.rs`

3. **Error Prefix and Exit Code**:
   - clap outputs version with "Error:" prefix by default
   - Exit code is 1 (not 0)
   - This is standard clap behavior for --version

4. **CLI Configuration** (from `src/cli/mod.rs`):
   ```rust
   #[command(name = "bf")]
   #[command(about = "bead-forge - Drop-in replacement for beads_rust (br)", long_about = None)]
   #[command(version = env!("CARGO_PKG_VERSION"))]
   #[command(propagate_version = true)]
   pub struct Cli {
   ```

## Comparison with Original br

The original `br` (beads_rust) likely had similar behavior since:
- bead-forge aims to be a drop-in replacement
- clap's default --version behavior includes the "Error:" prefix
- Exit code 1 is clap's default for --version

## Notes

- This documentation captures the current state as of bead-forge 0.2.0
- The "Error:" prefix is misleading for a successful version display
- Exit code 1 could cause issues in scripts expecting 0 for successful version query
- Future enhancement could add custom version handling for cleaner output
