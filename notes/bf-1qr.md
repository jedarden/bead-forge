# bf-1qr — Current --version Flag Behavior Investigation

**Task:** Research how clap handles --version and why it outputs 'Error: bf 0.2.0' instead of 'bf 0.2.0'.

**Status:** ✅ **RESOLVED** - Issue was fixed; current behavior is correct

## Current Behavior (Verified as of 2026-07-28)

```bash
$ bf --version
bf 0.3.0
$ echo $?
0
```

- **Output:** `bf 0.3.0` (clean, no prefix)
- **Stream:** stdout (not stderr)
- **Exit code:** 0
- **Status:** Working correctly ✅

## Historical Context (Why This Task Existed)

The task description references an issue where `--version` supposedly output `Error: bf 0.2.0`. This was actually investigated and resolved in bead **bf-um3e** (closed 2026-07-02). The investigation revealed:

### 1. The Real Source of "Error:" Prefix

The capital-**E** `Error:` prefix that appears on **command failures** is **NOT** from clap and **NOT** from bead-forge code. It is the **Rust standard library's default termination behavior** for programs declared `fn main() -> Result<T, E>`.

When `main` returns `Err(e)`, the Rust runtime prints `Error: {e:?}` to **stderr** and exits with code 1.

### 2. The Fix Applied

A manual fast path was added to `src/main.rs:7-10` to handle `--version` before clap parsing:

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

This ensures `--version`:
- Outputs to stdout (not stderr)
- Exits with code 0 (not 1)
- Never reaches the `Result` error path
- Produces clean output without "Error:" prefix

### 3. Why the Previous Investigation Was Wrong

The original content of this file (preserved below) incorrectly concluded that the "Error:" prefix was clap's expected behavior. This was disproven by bf-um3e's empirical analysis, which showed:

- clap's native `--version` outputs to stdout, exits 0, **without** "Error:" prefix
- The capital-**E** `Error:` only appears when `main` returns `Err()`
- This is Rust std behavior, not clap behavior

## clap's Actual Default Behavior

According to prior investigation in bf-31zx:

- clap's native `--version` routes to **stdout**, exits **0**, **no prefix**
- clap's lowercase `error:` prefix (exit 2) is reserved for **parse failures** only
- clap never emits a capital-**E** `Error:`

## Classification

- **Bug?** ❌ No - fixed in prior work
- **Configuration issue?** ❌ No - working correctly
- **Expected behavior?** ✅ Yes - current implementation is correct

## References

- **bf-um3e**: Root cause analysis - 'Error:' prefix is Rust std behavior (closed 2026-07-02)
- **bf-31zx**: clap version flag research
- **bf-5k3l**: Version output code location analysis
- **Commit db5e205**: "close(bf-um3e): Root cause analysis complete - 'Error:' prefix is Rust std behavior"

---

## 📚 ARCHIVED: Incorrect Original Analysis

The following content was the original analysis in this file, which was **proven incorrect** by bf-um3e's investigation. It is preserved for historical reference only.

> **Original (incorrect) conclusion:** "The 'Error:' prefix is part of clap's standard error formatting. This is by design..."
>
> **Reality:** The "Error:" prefix comes from Rust's std `Termination` trait for `Result<()>`, not from clap. The manual fast path in main.rs now correctly bypasses this for `--version`.
