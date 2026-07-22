# bf-um3e — Root cause: why the `Error:` prefix appears

**Task:** Correlate all findings (bf-31zx, bf-3392, bf-5k3l) to determine why the
`Error:` prefix appears. Compare clap defaults with the actual implementation, and
classify it as bug / config / expected.

## TL;DR (the answer)

The capital-**`E`** `Error:` prefix is **not** clap and **not** bead-forge code. It is
the **Rust standard library's default termination behavior** for any program declared
`fn main() -> Result<T, E>` where `E: Debug`. When `main` returns `Err(e)`, the runtime
prints `Error: {e:?}` to **stderr** and exits with `ExitCode::FAILURE` (**1**).

bead-forge's `main` is exactly that shape (`src/main.rs:3`):

```rust
fn main() -> Result<()> {
    ...
    let cli = bead_forge::cli::run_cli()?;
    bead_forge::cli::run(cli)   // returns Result<()>; Err propagates out of main
}
```

So **every command failure** surfaces as `Error: <anyhow Debug>` on stderr, exit 1. This
is **expected Rust behavior**, not a bug and not a configuration issue.

This is also **why the string couldn't be found in the repo**: the relevant code is in
the Rust std `Termination` impl (`std::process::Termination for Result<T, E>`), which is
not part of this source tree. Grepping `src/` for `"Error:"` finds only the two unrelated
`format_error` formatters (bf-5k3l) — those govern a *different* error path and are not
the source of the observed prefix.

## Evidence — empirical exit-code / stream matrix

Captured against the current built binary (`target/debug/bf`, `bf 0.3.0`):

| Invocation | Code path | Stream | Output | Exit |
|---|---|---|---|---|
| `bf --version` / `bf -V` | `main.rs:7-10` manual fast path | **stdout** | `bf 0.3.0` | **0** |
| `bf bogus` (unknown subcommand) | clap `Cli::parse()` (`run_cli`) | **stderr** | lowercase `error: unrecognized subcommand …` | **2** |
| `bf show bf-nope` (cmd failure) | `run()` → `Err` → `main` → **std Termination** | **stderr** | `Error: Bead not found: bf-nope` | **1** |
| `bf` (no subcommand) | `run()` → `Err` → `main` → **std Termination** | **stderr** | `Error: No command provided. Use 'bf --help' …` | **1** |

Byte-exact capture of the failing-command output (xxxd stderr):

```
00000000: 4572 726f 723a 2042 6561 6420 6e6f 7420  Error: Bead not
00000010: 666f 756e 643a 2062 662d 6e6f 7065 0a    found: bf-nope.
```

Stdout for that invocation is empty — the message is stderr-only, as the std runtime does.

## Reconciling with the prior beads

| Bead | Established | How it fits |
|---|---|---|
| bf-3392 | `bf --version` → `bf 0.3.0`, exit 0. | Correct. `--version` is the stdout fast path and **never** reaches the `Error:` path. The `Error:` symptom is unrelated to version output. |
| bf-31zx | clap's default routes `--version` to stdout (exit 0, no prefix); the `error:` prefix (lowercase) is reserved for **parse failures** (stderr, exit 2). | Correct, and matches the `bf bogus` row above. clap never emits a capital-`E` `Error:`. |
| bf-5k3l | The `Error:` literal in `src/` lives only in `format_error` (`text.rs:41`, `toon.rs:40`); the version path can't reach it. | Correct that those are the only in-repo `Error:` strings — but they are **not** the source of the observed prefix. The real source (std `Termination`) is outside the repo, which is exactly what made it hard to locate. |

The earlier hypothesis in bf-5k3l ("wrapper/alias, stale binary, or harness interleaving")
was looking for an *external* cause. The actual cause is internal but out-of-tree: the
Rust runtime. No external factor is required to reproduce it — `bf show <any-missing-id>`
produces `Error: …` from a clean current build.

## clap defaults vs. actual implementation

bf-31zx already showed clap would natively print `bf 0.3.0` to stdout (exit 0) for
`--version`. bead-forge's `main.rs:7-10` replicates that by hand. The two points worth
recording for the correlation:

1. **`--version` fast path is redundant but harmless.** `Cli::parse()` already does the
   right thing (stdout, exit 0). The manual intercept duplicates it. Not a bug — just
   dead-equivalent code. (Tracked as a possible simplification; not in scope here.)
2. **The `Error:` prefix has nothing to do with clap.** clap's contribution is the
   lowercase `error:` (exit 2) on parse failures. The capital-`E` `Error:` (exit 1) is
   purely the Rust runtime terminating a `Result`-returning `main`.

## Classification

- **Bug?** No. `Error: {:?}` to stderr / exit 1 is the canonical Rust behavior for
  `fn main() -> Result<()>`. It is by-design termination.
- **Configuration issue?** No. No flag/env toggles it; it is unconditional std behavior.
- **Expected behavior?** **Yes**, for command failures (bead not found, no command, DB /
  IO errors, etc.).

### Minor design note (not the bug under investigation)
Because the failure path returns through `main`'s `Result`, the message is shaped by
anyhow's `Debug` (`{:?}`), **not** by the selected `--format` formatter. So a
`--format json` run that fails still emits the plain-text `Error: …` instead of a JSON
error document:

```
$ bf show bf-nope --format json
Error: Bead not found: bf-nope      # stderr, plain text — JSON formatter bypassed
```

The `format_error` formatters (bf-5k3l) only shape errors that bead-forge routes through
`get_formatter(...).format_error(...)` explicitly; errors propagated via `?` skip them
entirely. That is a UX inconsistency worth a future task, but it is distinct from the
question of *why* the `Error:` prefix exists — which is answered above.

## Conclusion

The `Error:` prefix appears because `main` returns `Result<()>` and lets errors
propagate via `?`; the Rust runtime then prints `Error: {:?}` and exits 1. It is
expected, repo-independent std behavior — not a bug, not a config issue, and unrelated
to `--version` (which exits 0 on the stdout fast path). Filed as a comment on the
predecessor bead bf-5k3l for traceability.
