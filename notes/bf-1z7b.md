# `bf --version` Behavior Documentation

## Command Output

### `bf --version`
```
Error: bf 0.2.0
```
**Exit code:** 1

### `br --version`
```
Error: bf 0.2.0
```
**Exit code:** 1

## Analysis

Both `bf` and `br` produce identical output for the `--version` flag:

1. **Output format**: Uses an "Error:" prefix before the version string
2. **Exit code**: Returns exit code 1 (unusual for --version flags, but this is br's behavior)
3. **Version source**: Pulled from `CARGO_PKG_VERSION` in `Cargo.toml` (currently "0.2.0")

## Implementation

In `src/cli/mod.rs`, the version is configured via clap derive macro:

```rust
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(propagate_version = true)]
pub struct Cli {
```

The version is automatically pulled from the `package.version` field in `Cargo.toml`.

## br Compatibility

The `bf` command maintains exact br compatibility for `--version` output, including the non-standard "Error:" prefix and exit code 1. This is intentional behavior to ensure drop-in replacement compatibility.
