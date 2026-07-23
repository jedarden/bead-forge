# bf-3a2: Fix version output format

## Summary
Fixed the `--version` flag to output clean version information without an 'Error:' prefix.

## Implementation
The fix was implemented in commit `5d09073a60701615bcf0e7f042eda0705b66cb47`.

### Changes Made
1. **src/cli/mod.rs**: Added `#[command(disable_version_flag = true)]` to disable clap's built-in version flag
2. **src/main.rs**: Already had manual version handling that outputs `println!("bf {}", env!("CARGO_PKG_VERSION"))` with exit code 0

### Verification
```bash
$ ./target/debug/bf --version
bf 0.3.0

$ ./target/debug/bf --version; echo "Exit code: $?"
bf 0.3.0
Exit code: 0
```

## Acceptance Criteria
- ✅ Modify clap configuration to output version without 'Error:' prefix
- ✅ Version output should be just 'bf <version>' on stdout
- ✅ Exit code should be 0 (success)
