# Bead bf-1af6: Add version field to CLI args

## Task
Add version field to CLI arguments struct in src/cli/mod.rs and ensure Cargo.toml has the version field defined.

## Verification (2026-07-03)

Both requirements were already satisfied:

1. **CLI version attribute** (`src/cli/mod.rs:21`):
   ```rust
   #[command(version = env!("CARGO_PKG_VERSION"))]
   #[command(propagate_version = true)]
   ```
   The `version` attribute pulls from `CARGO_PKG_VERSION` (set in Cargo.toml).
   The `propagate_version = true` ensures version propagates to all subcommands.

2. **Cargo.toml version** (`Cargo.toml:3`):
   ```toml
   version = "0.2.0"
   ```

## Build Status
✅ Compiles cleanly with no errors (`cargo build`)

## Conclusion
No changes were required - the version field was already properly configured in both locations.
