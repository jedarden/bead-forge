//! Guard: no test may run `bf` against a real bead workspace.
//!
//! ## Why this exists
//!
//! `bf` resolves its workspace from `--workspace`, defaulting to the process
//! cwd (`src/cli/mod.rs`: `cli.workspace.unwrap_or_else(|| PathBuf::from("."))`).
//! Under `cargo test` the cwd is the package root, so **any `bf` invocation that
//! does not pass `--workspace`/`-w` and does not set `current_dir` operates on
//! bead-forge's own live `.beads/` store.**
//!
//! This is not hypothetical. On 2026-08-05 the live store was destroyed and
//! `.beads/issues.jsonl` was truncated to 0 bytes, losing 2302 beads until they
//! were recovered from git. Two things fed it: an orphaned `test_mark_dirty`
//! binary (its parent `cargo test` had died) that kept running for 16h with cwd
//! set to the repo, and `tests/test_epic_error_handling.rs`, which created real
//! beads ("Critical Epic", "Backlog Epic", "Test Task") in the live store on
//! every single test run.
//!
//! The same class of bug had already been found and fixed once, in
//! `tests/test_comprehensive_labels.rs` — see the comment there — and then
//! reappeared in three other files because nothing enforced it. Hence this test.
//!
//! ## What to do if this test fails
//!
//! Scope the offending file's `bf` invocations to a throwaway workspace:
//!
//! ```ignore
//! let ws = TempDir::new().unwrap();
//! Command::new(env!("CARGO_BIN_EXE_bf"))
//!     .arg("--workspace").arg(ws.path())
//!     .args(["init", "--prefix", "bf"])
//!     .output().unwrap();
//! ```
//!
//! For invocations that genuinely cannot open a store (`--help`, `--version`),
//! anchoring the child to an empty temp cwd is enough:
//!
//! ```ignore
//! Command::new(bf_binary()).current_dir(isolated_cwd())
//! ```
//!
//! There is deliberately **no allowlist**. A rule with no exceptions cannot be
//! eroded one "harmless" exception at a time, which is how this recurred.

use std::fs;
use std::path::{Path, PathBuf};

/// Does this source file spawn the `bf` binary?
fn spawns_bf(src: &str) -> bool {
    src.contains("CARGO_BIN_EXE_bf")
        || src.contains("target/debug/bf")
        || src.contains("bf_binary()")
}

/// Does this source file scope its `bf` invocations to an explicit workspace?
///
/// Deliberately permissive — any of these markers means the author was aware of
/// workspace scoping. The guard's job is to catch files with *no* scoping at
/// all, which is the shape that caused the 2026-08-05 loss.
fn has_workspace_scoping(src: &str) -> bool {
    src.contains("--workspace")
        || src.contains("\"-w\"")
        || src.contains("current_dir")
        || src.contains("BEADS_DIR")
}

/// Recursively collect `.rs` files under `dir`.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

#[test]
fn no_test_runs_bf_against_a_real_workspace() {
    let this_file = file_name_of(Path::new(file!()));
    let tests_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");

    let mut files = Vec::new();
    rust_files(&tests_dir, &mut files);
    assert!(
        !files.is_empty(),
        "found no test sources under {} — the guard would silently pass",
        tests_dir.display()
    );

    let mut offenders: Vec<String> = files
        .iter()
        .filter(|path| file_name_of(path) != this_file)
        .filter_map(|path| {
            let src = fs::read_to_string(path).ok()?;
            (spawns_bf(&src) && !has_workspace_scoping(&src)).then(|| file_name_of(path))
        })
        .collect();

    offenders.sort();

    assert!(
        offenders.is_empty(),
        "These test files spawn `bf` without scoping it to a workspace, so they run \
         against bead-forge's OWN live .beads/ store (cargo sets cwd to the package \
         root):\n  {}\n\nFix: pass `--workspace <tempdir>`, or set `current_dir` to an \
         empty temp dir for store-free flags. See this file's module docs.",
        offenders.join("\n  ")
    );
}
