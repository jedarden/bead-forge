# Triage Report: bf-e5ge — doctor --repair refuse exit code

## Issue Description
`bf doctor --repair` appears to print Error messages but exit with code 0 instead of non-zero when repair refuses to proceed.

## Code Analysis

### Repair Refuse Branches (src/doctor.rs)

The `repair()` function has three "refuse" conditions that **already return `Err(...)`**:

1. **Line 310-314**: JSONL file not found
   ```rust
   if !jsonl_path.exists() {
       return Err(anyhow!(
           "Cannot repair: JSONL file not found at {}",
           jsonl_path.display()
       ));
   }
   ```

2. **Line 327-332**: Cannot flush from corrupt DB when --flush-first is set
   ```rust
   if flush_first {
       return Err(anyhow!(
           "Cannot flush: database is corrupted and unreadable.\n\
            Flushing from a corrupt DB would poison the JSONL checkpoint.\n\
            Unflushed beads cannot be recovered.\n\
            Remove --flush-first to proceed with repair only."
       ));
   }
   ```

3. **Line 356-373**: Unflushed beads exist without --flush-first or --force
   ```rust
   } else if !force {
       return Err(anyhow!(
           "Cannot repair: {} unflushed bead(s) exist ({}).\n\
            Run 'bf doctor --repair --flush-first' to flush before repair,\n\
            or 'bf doctor --repair --force' to proceed (these beads will be LOST)."
       ));
   }
   ```

### Error Propagation Path

**src/cli/mod.rs (cmd_doctor):**
```rust
fn cmd_doctor(...) -> Result<()> {
    if repair {
        let imported = crate::doctor::repair(workspace_dir, flush_first, force)?;
        // ...
    }
    Ok(())
}
```

The `?` operator propagates `Err(...)` correctly.

**src/main.rs:**
```rust
fn main() -> Result<()> {
    let cli = bead_forge::cli::run_cli()?;
    bead_forge::cli::run(cli)
}
```

### Expected Exit Code Behavior

When `main()` returns `Err<E>`, the Rust standard library:
1. Calls `E::Display` to format the error message
2. Prints it to stderr
3. **Exits with code 1** (non-zero)

This is the standard behavior for `fn main() -> Result<()>`.

## Current Exit Code Status

**✓ The code IS CORRECT** - all refuse branches return `Err(...)`, which properly propagates to `main()` and exits with code 1.

### Why Exit Code 1 Works Here

The chain is:
- `repair()` returns `Err(anyhow!(...))`
- `cmd_doctor()` propagates with `?`
- `run()` propagates with `?`
- `main()` receives `Err(...)`
- Rust's termination handler prints error and exits 1

### Possible Confusion Points

1. **Misleading `eprintln!` before `return Err`**: Some branches have `eprintln!("WARNING: ...")` messages that print to stderr before returning `Err`, but these don't affect exit code.

2. **No "Error:" prefix**: anyhow's default error format doesn't prefix with "Error:", it just prints the message. This may make errors look like warnings.

3. **Non-refuse branches use `eprintln!`**: Lines 336-340, 351-354, 376-389 print warnings with `eprintln!` but then proceed (return `Ok(())`). These are NOT refuse cases.

## Fix Plan

### Current State: NO FIX NEEDED

The exit code behavior is already correct. All refuse branches properly return `Err(...)` which maps to exit code 1.

### If Enhanced Error Formatting is Desired

OPTIONAL: To make errors more visually distinct from warnings, consider:

1. **Add explicit error handling in main.rs**:
   ```rust
   fn main() -> Result<()> {
       if let Err(e) = try_main() {
           eprintln!("Error: {}", e);
           std::process::exit(1);
       }
       Ok(())
   }

   fn try_main() -> Result<()> {
       let cli = bead_forge::cli::run_cli()?;
       bead_forge::cli::run(cli)
   }
   ```
   This adds an explicit "Error:" prefix.

2. **Status: Not required** - The current behavior is correct. Exit code 1 is already being returned for all refuse cases.

## Verification

To verify the fix (if changes were made):
```bash
# Test JSONL not found exit code
cd /tmp/test_wsp
bf init
rm .beads/issues.jsonl
bf doctor --repair
echo $?  # Should be 1
```

## Conclusion

**Triage Result**: FALSE ALARM - The exit code behavior is already correct.

All `repair()` refuse branches properly return `Err(...)`, which causes `main()` to exit with code 1. The bug report appears to be based on a misunderstanding or testing with an old version of the code.

**Recommendation**: Close bead bf-e5ge as "Working as designed - exit code 1 already returned for all refuse cases."
