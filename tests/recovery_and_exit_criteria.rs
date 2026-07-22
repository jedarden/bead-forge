//! Capstone exit-criteria tests for Phase 7.1 (parent bf-1wg2v, plan §7.1).
//!
//! This file is the **verification layer** that proves the three §7.1 exit
//! criteria hold end to end, as tempdir-only integration tests:
//!
//! 1. **Named recovery** — `bf sync --flush-only` clears the `dirty_issues` set
//!    *and* rewrites a full checkpoint (every bead, not just the dirty one).
//!    (See [`flush_only_clears_dirty_and_writes_full_checkpoint`].)
//! 2. **Killing a worker between mutation and flush loses nothing** — modeled
//!    with `--no-auto-flush` (the flush never ran). The bead is NOT lost, a
//!    fresh connection still reads it, and once recovered it shows up in
//!    `git diff .beads/`. The default-auto-flush arm proves the same safety in
//!    normal operation: the JSONL artifact already carries the bead the moment
//!    the mutation returns.
//!    (See [`killed_worker_loses_nothing_git_diff_reveals_state`] and
//!    [`default_autoflush_makes_bead_visible_to_git_diff_immediately`].)
//! 3. **Flush failure is visible in machine-readable output** — a wedged flush
//!    carries a `warning` field in the `--json` envelope, and `doctor::check`
//!    surfaces the resulting drift in the structured `DoctorResult`
//!    (`unflushed_count` + `missing_in_jsonl`).
//!    (See [`flush_failure_carries_json_warning`] and
//!    [`doctor_surfaces_unflushed_drift_machine_readable`].)
//!
//! The canonical flush-FAILURE contract (never-fail-mutation, stderr+JSON
//! warning, dirty retention) lives in `tests/autoflush_failure_contract.rs`;
//! these tests are the §7.1-criteria superset that ties recovery + the
//! kill-worker invariant + machine-readable drift together in one place.
//!
//! Per the plan's Test Strategy (Rule 1), every test targets an ephemeral
//! tempdir workspace — never a live `.beads/` database.

use bead_forge::doctor;
use bead_forge::storage::Storage;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn bf() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bf"))
}

/// Run `bf` in `ws`; returns (stdout, stderr, success).
fn run(ws: &Path, args: &[&str]) -> (String, String, bool) {
    let out = bf()
        .current_dir(ws)
        .args(args)
        .output()
        .expect("failed to execute bf");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

/// A fresh `bf init` workspace under a tempdir.
fn init_ws() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let (_o, e, ok) = run(tmp.path(), &["init", "--prefix", "bf"]);
    assert!(ok, "bf init failed: {e}");
    tmp
}

fn jsonl_path(ws: &Path) -> PathBuf {
    ws.join(".beads").join("issues.jsonl")
}

fn storage_for(ws: &Path) -> Storage {
    let metadata = bead_forge::config::load_metadata(&ws.join(".beads")).unwrap();
    let db_path = ws.join(".beads").join(&metadata.database);
    Storage::open(&db_path).unwrap()
}

fn dirty_ids(storage: &Storage) -> Vec<String> {
    storage
        .list_dirty_issues()
        .unwrap()
        .into_iter()
        .map(|i| i.id)
        .collect()
}

fn jsonl_ids(ws: &Path) -> Vec<String> {
    let p = jsonl_path(ws);
    if !p.exists() || p.is_dir() {
        return Vec::new();
    }
    fs::read_to_string(&p)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            serde_json::from_str::<Value>(l).unwrap()["id"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}

/// Run `git` inside `ws` with a throwaway identity (no global config needed).
fn git(ws: &Path, args: &[&str]) -> (bool, String, String) {
    let out = Command::new("git")
        .current_dir(ws)
        .args(["-c", "user.name=bf-test"])
        .args(["-c", "user.email=bf-test@example.com"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .output()
        .expect("failed to run git");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// True when a usable `git` is on PATH (these tests need it for the diff
/// assertions). If absent, a git-dependent test early-returns instead of
/// failing the suite.
fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ===========================================================================
// Exit criterion 1 — named recovery clears dirty AND writes a full checkpoint
// ===========================================================================

/// `bf sync --flush-only` is the **named recovery path** (plan §7.1): after a
/// mutation whose auto-flush did not run (`--no-auto-flush`), the dirty bead
/// must (a) clear the `dirty_issues` set and (b) land in a **full** checkpoint
/// — every bead re-exported, not just the dirty one.
#[test]
fn flush_only_clears_dirty_and_writes_full_checkpoint() {
    let tmp = init_ws();
    let ws = tmp.path();

    // Bead A is created with auto-flush ON (default): it lands in JSONL and is
    // clean (no dirty mark).
    let (out, _e, ok) = run(ws, &["create", "--title", "A flushed"]);
    assert!(ok, "create A failed: {_e}");
    let a = out.trim().to_string();
    let storage = storage_for(ws);
    assert!(jsonl_ids(ws).iter().any(|x| x == &a), "A must auto-flush");
    assert!(
        !dirty_ids(&storage).iter().any(|d| d == &a),
        "A must be clean after auto-flush"
    );

    // Bead B is created with auto-flush DISABLED: it commits to SQLite and is
    // marked dirty, but the JSONL is untouched. This is the exact state a
    // killed worker (or a disabled flush) leaves behind.
    let (out, _e, ok) = run(ws, &["--no-auto-flush", "create", "--title", "B unflushed"]);
    assert!(ok, "create B failed: {_e}");
    let b = out.trim().to_string();

    // Pre-recovery invariants: B is db-only + dirty; JSONL still only has A.
    assert!(storage.get_issue(&b).unwrap().is_some(), "B must exist in db");
    assert!(dirty_ids(&storage).iter().any(|d| d == &b), "B must be dirty");
    let pre = jsonl_ids(ws);
    assert!(pre.iter().any(|x| x == &a));
    assert!(!pre.iter().any(|x| x == &b), "B must NOT yet be in JSONL");

    // Named recovery.
    let (_o, e, ok) = run(ws, &["sync", "--flush-only"]);
    assert!(ok, "sync --flush-only must succeed: {e}");

    // (a) dirty set cleared, (b) FULL checkpoint — both A and B present.
    let storage = storage_for(ws);
    assert!(
        dirty_ids(&storage).is_empty(),
        "recovery flush must clear the entire dirty set, got {:?}",
        dirty_ids(&storage)
    );
    let post = jsonl_ids(ws);
    assert!(
        post.iter().any(|x| x == &a) && post.iter().any(|x| x == &b),
        "full checkpoint must contain BOTH A and B, got {post:?}"
    );
    // Full checkpoint = the whole store, re-sorted by id — not a single-line
    // patch. Two beads in, two lines out.
    assert_eq!(
        post.len(),
        2,
        "full checkpoint must rewrite the complete bead set"
    );
}

// ===========================================================================
// Exit criterion 2 — killing a worker loses nothing git diff can't show
// ===========================================================================

/// Adversarial arm: model a mutation whose flush **never ran** with
/// `--no-auto-flush`. The bead must NOT be lost, must remain readable from a
/// fresh connection (worker restart), and must become visible to
/// `git diff .beads/` once recovered by `bf sync --flush-only`.
#[test]
fn killed_worker_loses_nothing_git_diff_reveals_state() {
    if !git_available() {
        eprintln!("skipping: git not available");
        return;
    }

    // /tmp keeps the tempdir outside any project repo so `git init` is clean.
    let tmp = TempDir::new_in("/tmp").unwrap();
    let ws = tmp.path();
    let (_o, e, ok) = run(ws, &["init", "--prefix", "bf"]);
    assert!(ok, "bf init failed: {e}");

    // Seed one flushed bead so issues.jsonl exists and can be committed.
    let (_seed, _e, ok) = run(ws, &["create", "--title", "seed"]);
    assert!(ok, "seed create failed");

    // Commit the baseline JSONL artifact.
    assert!(git(ws, &["init"]).0, "git init failed");
    assert!(git(ws, &["add", ".beads/issues.jsonl"]).0);
    assert!(git(ws, &["commit", "-m", "baseline"]).0, "git commit failed");

    // A mutation whose flush did not run (worker "killed" before flush, or
    // auto-flush disabled). The db commit + dirty mark land; JSONL is untouched.
    let (out, _e, ok) = run(ws, &["--no-auto-flush", "create", "--title", "killed worker bead"]);
    assert!(ok, "create failed");
    let killed = out.trim().to_string();

    // (1) The bead is NOT lost: it is present in the live store.
    let storage = storage_for(ws);
    assert!(
        storage.get_issue(&killed).unwrap().is_some(),
        "bead must survive a skipped flush"
    );
    assert!(dirty_ids(&storage).iter().any(|d| d == &killed));

    // (2) Recoverable from a fresh connection — models a worker restart after
    // the kill. A brand-new Storage handle still reads the committed bead.
    let reopened = storage_for(ws);
    let bead = reopened
        .get_issue(&killed)
        .expect("reopen query failed")
        .expect("bead must be readable from a fresh connection after the kill");
    assert_eq!(bead.title, "killed worker bead");

    // (3) Before recovery the committed artifact does NOT yet carry the bead —
    // so a plain `git diff` is empty. This is the window the criterion is about:
    // nothing is *lost*, the state is just not yet in git.
    let (ok, diff, _) = git(ws, &["diff", "--name-only", "--", ".beads/issues.jsonl"]);
    assert!(ok);
    assert!(
        diff.trim().is_empty(),
        "pre-recovery diff must be empty (bead not yet in artifact), got: {diff}"
    );

    // (4) Recovery. The named recovery path lands the bead in the artifact and
    // clears the dirty mark.
    let (_o, e, ok) = run(ws, &["sync", "--flush-only"]);
    assert!(ok, "sync --flush-only must succeed: {e}");
    assert!(jsonl_ids(ws).iter().any(|x| x == &killed));
    let storage = storage_for(ws);
    assert!(!dirty_ids(&storage).iter().any(|d| d == &killed));

    // (5) NOW `git diff .beads/` reveals the recoverable state — the criterion.
    let (ok, diff, _) = git(ws, &["diff", "--", ".beads/issues.jsonl"]);
    assert!(ok, "git diff failed");
    assert!(
        diff.contains(&killed),
        "git diff must reveal the recovered bead, got:\n{diff}"
    );
}

/// Normal-operation arm: with auto-flush ON (the default), the JSONL artifact
/// already carries the bead the instant the mutation returns — so killing the
/// worker at any point *after* the command loses nothing `git diff .beads/`
/// can't show.
#[test]
fn default_autoflush_makes_bead_visible_to_git_diff_immediately() {
    if !git_available() {
        eprintln!("skipping: git not available");
        return;
    }

    let tmp = TempDir::new_in("/tmp").unwrap();
    let ws = tmp.path();
    let (_o, e, ok) = run(ws, &["init", "--prefix", "bf"]);
    assert!(ok, "bf init failed: {e}");
    let (_seed, _e, ok) = run(ws, &["create", "--title", "seed"]);
    assert!(ok, "seed create failed");

    assert!(git(ws, &["init"]).0);
    assert!(git(ws, &["add", ".beads/issues.jsonl"]).0);
    assert!(git(ws, &["commit", "-m", "baseline"]).0);

    // Default auto-flush: the bead is in JSONL before the process even exits.
    let (out, _e, ok) = run(ws, &["create", "--title", "live bead"]);
    assert!(ok, "create failed");
    let live = out.trim().to_string();
    assert!(jsonl_ids(ws).iter().any(|x| x == &live));

    // The worker could be killed now and `git diff .beads/` still shows it.
    let (ok, diff, _) = git(ws, &["diff", "--", ".beads/issues.jsonl"]);
    assert!(ok, "git diff failed");
    assert!(
        diff.contains(&live),
        "auto-flushed bead must be visible in git diff immediately, got:\n{diff}"
    );
}

// ===========================================================================
// Exit criterion 3 — flush failure visible in machine-readable output
// ===========================================================================

/// Wedge the auto-flush (make `issues.jsonl` a directory so the atomic
/// temp+rename export cannot replace it). The `--json` envelope must carry a
/// non-null `warning` naming the failure and the recovery command, while the
/// mutation itself still succeeds. (Canonical contract: see
/// `tests/autoflush_failure_contract.rs`; asserted here too so the §7.1
/// criterion is self-contained in the capstone file.)
fn wedge(ws: &Path) {
    let p = jsonl_path(ws);
    if p.exists() {
        fs::remove_file(&p).ok();
    }
    fs::create_dir(&p).unwrap();
}

#[test]
fn flush_failure_carries_json_warning() {
    let tmp = init_ws();
    let ws = tmp.path();
    wedge(ws);

    // The mutation committed, so the command succeeds despite the flush failure.
    let (stdout, _stderr, ok) = run(ws, &["create", "--json", "--title", "wedged"]);
    assert!(ok, "flush failure must NOT fail create");

    let parsed: Value =
        serde_json::from_str(stdout.trim()).expect("--json stdout must be a single JSON object");

    let id = parsed
        .get("id")
        .and_then(|v| v.as_str())
        .expect("--json must carry the created id")
        .to_string();

    // Machine-readable surface: a non-null `warning` string that is actionable.
    let warning = parsed
        .get("warning")
        .and_then(|v| v.as_str())
        .expect("--json must carry a non-null `warning` on flush failure");
    assert!(warning.contains("auto-flush"), "warning: {warning}");
    assert!(
        warning.contains("bf sync --flush-only"),
        "warning must name the recovery command: {warning}"
    );

    // The bead still exists (never lost) and stays dirty for the retry.
    let storage = storage_for(ws);
    assert!(storage.get_issue(&id).unwrap().is_some());
    assert!(dirty_ids(&storage).iter().any(|d| d == &id));
}

/// `doctor::check` is the machine-readable surface for drift today (a future
/// `bf doctor --json`, plan §7.4, will serialize this struct). A db-only bead
/// must show up as both `unflushed_count` and `missing_in_jsonl`, and the
/// `bf doctor` CLI text must surface "Unflushed beads" to a human.
#[test]
fn doctor_surfaces_unflushed_drift_machine_readable() {
    let tmp = init_ws();
    let ws = tmp.path();

    // One flushed bead (clean) + one db-only bead (dirty, not in JSONL).
    run(ws, &["create", "--title", "flushed"]);
    let (out, _e, ok) = run(ws, &["--no-auto-flush", "create", "--title", "db-only"]);
    assert!(ok, "create failed");
    let dbonly = out.trim().to_string();

    // Machine-readable: the structured DoctorResult carries the drift.
    let result = doctor::check(ws).expect("doctor::check failed");
    assert_eq!(
        result.unflushed_count, 1,
        "exactly one dirty bead must be reported unflushed"
    );
    assert!(
        result.missing_in_jsonl.iter().any(|id| id == &dbonly),
        "db-only bead must appear in missing_in_jsonl, got {:?}",
        result.missing_in_jsonl
    );

    // Human surface: the CLI reports the drift too.
    let (stdout, _stderr, ok) = run(ws, &["doctor"]);
    assert!(ok, "bf doctor must exit cleanly on a healthy-but-drifty workspace");
    assert!(
        stdout.contains("Unflushed beads"),
        "bf doctor must surface the unflushed drift in its text output, got:\n{stdout}"
    );
}
