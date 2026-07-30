# bf-iyjr — Verify `bf --version` outputs version

## Result: PASS (already complete)

Ran `bf --version` against the freshly built debug binary (`./target/debug/bf`).

### Acceptance criteria

| Criterion | Result |
|-----------|--------|
| `bf --version` (or `bf version`) outputs version information | ✅ — `bf --version` prints `bf 0.3.0` |
| Version format is semantic (e.g., v0.1.0 or similar) | ✅ — `0.3.0` is a semantic version |
| Command returns exit code 0 | ✅ — `EXIT_CODE=0` |

### Captured output

```
$ ./target/debug/bf --version
bf 0.3.0
EXIT_CODE=0
```

Note: `bf --version` satisfies the criterion (the criteria accept `--version` **or** `version`). The `bf version`
subcommand is intentionally not wired — clap rejects it as an unrecognized subcommand, which is expected.

### Wiring

- `Cargo.toml:3` — `version = "0.3.0"`
- `Cargo.toml:60` — `name = "bf"` (binary name)
- `src/cli/mod.rs:24` — `#[command(name = "bf")]`
- `src/cli/mod.rs:25` — `#[command(version = env!("CARGO_PKG_VERSION"))]` drives the `--version` flag

No code changes required — the clap `version` attribute already renders the version derived from `CARGO_PKG_VERSION`.
