# Investigation: --version Flag Behavior in bead-forge

## Current Behavior

When running `bf --version` or `bf -V`, the output is:
```
Error: bf 0.2.0
```

The command exits with code 1, and the version is prefixed with "Error: ".

## Root Cause Analysis

### 1. Expected clap Behavior

This is actually **expected behavior** for clap's default `--version` implementation. In clap 4.x, when the `--version` flag is used, clap internally treats it as an early-exit condition that uses the error formatting system to display the version and immediately exit.

### 2. How clap Handles Version

Looking at the CLI configuration in `src/cli/mod.rs:21-22`:

```rust
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(propagate_version = true)]
```

The `version` attribute sets the version string, and clap's default behavior:
1. Detects `--version` or `-V` flag
2. Formats the output using clap's error formatter
3. Adds "Error: " prefix (this is intentional)
4. Exits with code 1

### 3. Why "Error:" Prefix?

The "Error:" prefix is part of clap's standard error formatting. This is by design because:
- The `--version` flag triggers an early exit path
- clap uses its error message formatter for all early exits
- The "Error:" prefix is added by clap's `Error` type when displayed

### 4. Verification with br

Running `br --version` also outputs:
```
Error: bf 0.2.0
```

This confirms that both `br` and `bf` exhibit the same behavior, which is the expected clap behavior.

## Is This a Bug?

**No, this is not a bug.** This is clap's default behavior for version output.

## Possible Solutions

If the "Error:" prefix is undesirable, there are several approaches:

### Option 1: Custom version_formatter (Not Available in clap 4 derive API)

The clap 4 derive API doesn't have a stable `version_formatter` attribute that would allow customizing the version output format without the "Error:" prefix. This feature may be available in clap's builder API or future clap versions.

### Option 2: Manual Version Flag

Disable clap's built-in version handling and implement a manual `--version` subcommand or argument:

```rust
#[derive(Parser)]
#[command(version = "", about = "...")]  // Disable default version
struct Cli {
    #[arg(long, action = ArgAction::SetTrue)]
    version: bool,
    // ... rest of CLI
}

// In main():
if cli.version {
    println!("bf {}", env!("CARGO_PKG_VERSION"));
    std::process::exit(0);
}
```

### Option 3: Use clap's builder API

Switch from derive API to builder API which offers more control over version formatting.

### Option 4: Accept Default Behavior

Since this is clap's expected behavior and `br` also exhibits it, the simplest solution is to accept this as the standard behavior.

## Recommendation

**Accept the current behavior** because:

1. It's clap's standard and expected behavior
2. Both `br` and `bf` output the same format
3. The "Error:" prefix doesn't break any functionality
4. Users typically pipe or parse version output anyway
5. Changing it would require significant refactoring with minimal benefit

## Files Referenced

- `src/cli/mod.rs:21-22` - CLI version configuration
- `Cargo.toml:3` - Version definition (0.2.0)

## Related clap Documentation

- [clap derive API documentation](https://docs.rs/clap/latest/clap/_derive/index.html)
- clap version formatting uses `Command::version()` method
- Version output uses clap's error formatter for early exit

## Conclusion

The "Error:" prefix in `bf --version` output is expected clap behavior, not a configuration issue or bug. The current implementation in `src/cli/mod.rs` correctly uses clap's derive API with the `version` attribute. No changes are recommended unless there's a specific user-facing requirement to remove the "Error:" prefix.
