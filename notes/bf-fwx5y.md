# bf-fwx5y — Verify `test_version` binary/source is safe to remove

**Parent:** bf-3o9 (repo-hygiene cleanup) · **Depends on child:** bf-4waen
**Type:** chore · **Scope:** read-only verification — no index/tree changes, unrelated
working-tree changes untouched.

## Task

Confirm the root-level `test_version` (4.2 MB compiled binary) and its source
`test_version.rs` are safe to delete **before** child bf-4waen commits the deletion.
ACCEPTANCE: comment documents (1) non-release and (2) version-display scenario covered.

## Verdict: SAFE TO REMOVE ✅

### (1) `test_version` is NOT the bf release artifact

- Release process = the Argo `bead-forge-build` WorkflowTemplate, in
  `declarative-config/k8s/iad-ci/argo-workflows/bead-forge-build-workflowtemplate.yml`:
  - `cargo build --release` → `target/release/bf`
  - `cp ./target/release/bf ./target/release/bf-linux-x86_64`
  - `sha256sum bf-linux-x86_64 > SHA256SUMS`
  - `gh release create "v${VERSION}" ... bf-linux-x86_64 SHA256SUMS`
- `docs/deployment.md:37` confirms: *"The `bead-forge-build` workflow builds the
  binary and creates a GitHub release with the `bf-linux-x86_64` asset."*
- `Cargo.toml` `[[bin]] name = "bf" path = "src/main.rs"` — the only binary.
  **`test_version.rs` is not a cargo target** (no `[[bin]]`/`[[test]]`/`[[example]]`/`[[bench]]`
  references it), so it is never compiled by `cargo build`/`cargo test`.
- The committed `test_version` binary is the compiled ELF of the standalone scratch
  `test_version.rs` (`strings`: rustc 1.95.0, contains `bf 0.2.0` + `test_version.rs` —
  per `notes/bf-5wz0l.md`).

### No references to the root-level files

`git grep test_version` across the tracked tree hits **only**:
- `.beads/issues.jsonl` — bead metadata (parent bf-1ig29 description)
- `notes/bf-3hu5.md` — references the *test function names* (live in `tests/test_version_display.rs`)
- `notes/bf-5wz0l.md` — prior audit confirming the binary is the compiled scratch ELF

Zero references in `scripts/`, `deploy/`, `systemd/`, `docs/`, `Cargo.toml`, or any CI
config (`.github`/`.argo`/`ci` all absent; GH Actions disabled per CLAUDE.md, CI = Argo).

### (2) Version-display scenario is covered by the committed test

`tests/test_version_display.rs` (registered `[[test]] name = "test_version_display"` in
`Cargo.toml`) runs the **real** `bf` binary via `CARGO_BIN_EXE_bf`:

| Test | Checks |
|------|--------|
| `test_version_flag_output` | `bf --version` → starts with `bf `, valid semver |
| `test_version_matches_cargo_toml` | version matches `Cargo.toml` |
| `test_version_short_flag` | `bf -V` works |
| `test_version_exit_code` | exits 0 |

The scratch `test_version.rs` only hardcodes `println!("bf 0.2.0")` for `--version`/`-V`.
The committed test is strictly more rigorous (real binary, semver validation, Cargo.toml
cross-check). Scenario fully covered.

## Conclusion

→ child **bf-4waen** may proceed with deleting `test_version` and `test_version.rs`.

Deliverable: acceptance comment (id 23) added to bf-fwx5y documenting both checks.
This notes file is the commit artifact (source work was read-only).
