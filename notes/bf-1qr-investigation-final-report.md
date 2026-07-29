# bf-1qr — --version Flag Behavior Investigation Final Report

## Task
Investigate current `--version` flag behavior and determine why it outputs "Error: bf 0.2.0" instead of "bf 0.2.0".

## Current Behavior (2026-07-28)

### ✅ Version Output Works Correctly
```bash
$ bf --version
bf 0.3.0

$ bf -V
bf 0.3.0
```

- **Output format**: `bf <version>` (clean, no "Error:" prefix)
- **Exit code**: 0 (success)
- **Stream**: stdout
- **Current version**: 0.3.0

## Root Cause Analysis

### Historical Context
The "Error: bf 0.2.0" issue mentioned in the bead description was **already investigated and resolved** in bead **bf-um3e** (closed 2026-07-28).

### Key Finding from bf-um3e
The capital-**`E`** `Error:` prefix is **NOT** from clap and **NOT** from bead-forge code. It is the **Rust standard library's default termination behavior** for programs declared as:

```rust
fn main() -> Result<T, E> where E: Debug
```

When `main` returns `Err(e)`, the Rust std library prints `Error: {e:?}` to **stderr** and exits with code 1.

### Evidence from bf-um3e Investigation

| Invocation | Code Path | Stream | Output | Exit |
|------------|-----------|---------|---------|------|
| `bf --version` / `bf -V` | `main.rs:7-10` manual fast path | **stdout** | `bf 0.3.0` | **0** |
| `bf bogus` (unknown subcommand) | clap `Cli::parse()` | **stderr** | lowercase `error: ...` | **2** |
| `bf show bf-nope` (cmd failure) | `run()` → `Err` → std Termination | **stderr** | `Error: Bead not found...` | **1** |

## Implementation Details

### Current Version Handling (src/main.rs:4-10)
```rust
// Handle version flag before clap parsing to output to stdout
let args: Vec<String> = std::env::args().collect();

if args.len() >= 2 && (args[1] == "--version" || args[1] == "-V") {
    println!("bf {}", env!("CARGO_PKG_VERSION"));
    std::process::exit(0);
}
```

This manual fast path:
1. Intercepts `--version` and `-V` **before** clap parsing
2. Outputs version to **stdout** (not stderr)
3. Exits with code **0** (success)
4. Avoids the Rust std library's error termination behavior

### Alternative Approach (clap-based)
The clap derive also has version configured:
```rust
#[derive(Parser)]
#[command(name = "bf")]
#[command(version = env!("CARGO_PKG_VERSION"))]
```

However, the manual fast path ensures consistent behavior and avoids any potential issues with clap's error handling.

## Testing Coverage

### Comprehensive Test Suite (tests/test_version_display.rs)
- ✅ `test_version_flag_output` - Verifies format starts with "bf "
- ✅ `test_version_matches_cargo_toml` - Ensures version matches Cargo.toml
- ✅ `test_version_short_flag` - Tests `-V` short flag
- ✅ `test_version_exit_code` - Confirms exit code 0

All tests passing, confirming the implementation is correct.

## Classification

✅ **NOT a bug** - Version output works as intended
✅ **NOT a configuration issue** - Manual fast path ensures correct behavior
✅ **IS expected behavior** - Clean version output to stdout with exit code 0

## Historical Timeline

- **bf-saty** (2026-07-03): Implemented version display handler
- **c4b4334** (2026-07-03): Finalized version display handler implementation
- **bf-um3e** (2026-07-28): Root cause analysis of "Error:" prefix behavior
- **bf-1qr** (2026-07-28): Current investigation - confirms issue already resolved

## Conclusion

The `--version` flag behavior is **working correctly**. The historical "Error:" prefix issue was:
1. Thoroughly investigated in bf-um3e
2. Identified as Rust std library behavior (not a bug)
3. Resolved by manual fast path implementation
4. Verified by comprehensive test suite

No further action needed on this bead - the issue was already resolved by the previous investigation.

## References

- Previous investigation: `notes/bf-um3e.md`
- Implementation: `src/main.rs:4-10`
- Tests: `tests/test_version_display.rs`
- Related beads: bf-um3e, bf-saty, bf-5k3l

## Acceptance Criteria Status

- ✅ Document current behavior of --version flag
- ✅ Identify root cause of 'Error:' prefix (from previous investigation)
- ✅ Determine if this is clap's default behavior or a configuration issue

**All acceptance criteria met through existing investigation and documentation.**