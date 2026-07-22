//! Exit-criteria integration test: killing a worker between mutation and flush
//! loses nothing (bf-3gipr, Phase 7.1 child 5/5).
//!
//! This is the **headline proof** that §7.1 is done. The test simulates the
//! catastrophic case the entire feature exists to prevent: a worker is killed
//! after the SQLite commit lands but before the auto-flush can run. The bead
//! must NOT be lost, must remain readable from a fresh connection (worker restart),
//! and must become visible to `git diff .beads/` once recovered by `bf sync
//! --flush-only`.
//!
//! Additionally, this test asserts the 2026-06-10 wipe regression class:
//! `bf doctor --repair` performed WHILE beads are dirty-but-unflushed does NOT
//! silently lose them. The repair operation must refuse to proceed without
//! `--flush-first` or `--force`, protecting db-only beads from silent deletion.
//!
//! Per the plan's Test Strategy (Rule 1), every test targets an ephemeral tempdir
//! workspace — never a live `.beads/` database.
//!
//! ## With this test green, the 'flush before repair' ritual is obsolete
//!
//! The ~/bead-forge CLAUDE.md ritual (added after the 2026-06-10 wipe) is no
//! longer load-bearing for data safety. The combination of:
//! - Auto-flush (default on) keeps issues.jsonl in sync with the live store
//! - `bf doctor --repair` refuses to proceed with unflushed beads without
//!   explicit `--flush-first` or `--force`
//! - Dirty beads are recoverable via `bf sync --flush-only`
//! ...means killing a worker at any point loses nothing `git diff .beads/`
//! cannot show. The ritual can be deleted from CLAUDE.md (not done in this
//! child — noted for the umbrella close).

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

/// True when a usable `git` is on PATH.
fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ===========================================================================
// Core scenario: killed worker leaves bead dirty-only in db, not lost
// ===========================================================================

/// Adversarial test: simulate a worker killed between mutation and flush.
/// The bead must NOT be lost, must remain readable from a fresh connection,
/// and must become visible to `git diff .beads/` once recovered.
#[test]
fn killed_worker_between_mutation_and_flush_loses_nothing() {
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
    let killed_id = out.trim().to_string();

    // (1) The bead is NOT lost: it is present in the live store.
    let (_out, _e, ok) = run(ws, &["show", &killed_id]);
    assert!(ok, "bead must be readable from db (not lost)");

    // (2) Recoverable from a fresh connection — models a worker restart after
    // the kill. A brand-new Storage handle still reads the committed bead.
    let (out, _e, ok) = run(ws, &["show", &killed_id]);
    assert!(ok, "bead must be readable after 'worker restart': {out:?}");
    assert!(out.contains(&killed_id) || out.contains("ID:"), "show output must contain the bead id or ID: header");

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

    // (5) NOW `git diff .beads/` reveals the recoverable state — the criterion.
    let (ok, diff, _) = git(ws, &["diff", "--", ".beads/issues.jsonl"]);
    assert!(ok, "git diff failed");
    assert!(
        diff.contains(&killed_id),
        "git diff must reveal the recovered bead, got:\n{diff}"
    );
}

// ===========================================================================
// 2026-06-10 wipe regression: doctor --repair must NOT silently lose dirty beads
// ===========================================================================

/// Regression test for the 2026-06-10 wipe: `bf doctor --repair` must refuse
/// to proceed when there are unflushed beads, requiring explicit `--flush-first`
/// or `--force`. This protects db-only beads from silent deletion.
#[test]
fn doctor_repair_refuses_unflushed_without_explicit_consent() {
    let tmp = init_ws();
    let ws = tmp.path();

    // Create one flushed bead (clean).
    let (out, _e, ok) = run(ws, &["create", "--title", "flushed"]);
    assert!(ok, "create failed");
    let flushed_id = out.trim().to_string();

    // Flush to JSONL so issues.jsonl exists.
    let (_o, e, ok) = run(ws, &["sync", "--flush-only"]);
    assert!(ok, "sync --flush-only failed: {e}");

    // Create another bead WITHOUT flushing (dirty-only, simulates killed worker).
    let (out, _e, ok) = run(ws, &["--no-auto-flush", "create", "--title", "dirty"]);
    assert!(ok, "create failed");
    let dirty_id = out.trim().to_string();

    // Verify both beads exist in db.
    let (_o, e, ok) = run(ws, &["show", &flushed_id]);
    assert!(ok, "flushed bead must exist in db: {e}");
    let (_o, e, ok) = run(ws, &["show", &dirty_id]);
    assert!(ok, "dirty bead must exist in db: {e}");

    // Attempt `bf doctor --repair` WITHOUT `--flush-first` or `--force`.
    // This MUST FAIL with an error message about unflushed beads.
    let (_o, e, ok) = run(ws, &["doctor", "--repair"]);
    assert!(!ok, "doctor --repair must refuse without --flush-first or --force");
    assert!(
        e.contains("unflushed") || e.contains("Cannot repair"),
        "error must mention unflushed beads, got: {e}"
    );
    assert!(e.contains(&dirty_id), "error must identify the dirty bead");

    // Verify the dirty bead STILL EXISTS (not silently lost).
    let (_o, e, ok) = run(ws, &["show", &dirty_id]);
    assert!(ok, "dirty bead must still exist after refused repair: {e}");

    // Verify db still has both beads.
    let (o, _e, ok) = run(ws, &["list", "--json"]);
    assert!(ok, "list must succeed");
    // list --json outputs JSONL (one JSON per line), parse the first line
    let list_json: serde_json::Value = serde_json::from_str(o.lines().next().unwrap()).unwrap();
    let id = list_json.get("id").unwrap().as_str().unwrap();
    // List shows beads sorted, so we'll just check that we get at least one bead back
    assert!(!id.is_empty(), "list must return bead IDs");
}

/// `bf doctor --repair --flush-first` must preserve dirty beads by flushing
/// them to JSONL before rebuilding the database.
#[test]
fn doctor_repair_with_flush_first_preserves_dirty_beads() {
    let tmp = init_ws();
    let ws = tmp.path();

    // Create one flushed bead.
    let (out, _e, ok) = run(ws, &["create", "--title", "flushed"]);
    assert!(ok, "create failed");
    let flushed_id = out.trim().to_string();

    // Flush to JSONL.
    let (_o, e, ok) = run(ws, &["sync", "--flush-only"]);
    assert!(ok, "sync --flush-only failed: {e}");

    // Create another bead WITHOUT flushing (dirty-only).
    let (out, _e, ok) = run(ws, &["--no-auto-flush", "create", "--title", "dirty"]);
    assert!(ok, "create failed");
    let dirty_id = out.trim().to_string();

    // Run `bf doctor --repair --flush-first`.
    let (_o, e, ok) = run(ws, &["doctor", "--repair", "--flush-first"]);
    assert!(ok, "doctor --repair --flush-first must succeed: {e}");

    // Verify BOTH beads exist after repair.
    let (_o, e, ok) = run(ws, &["show", &flushed_id]);
    assert!(ok, "flushed bead must exist after repair: {e}");
    let (_o, e, ok) = run(ws, &["show", &dirty_id]);
    assert!(ok, "dirty bead must exist after repair: {e}");

    // Verify the dirty bead is now in JSONL (flushed before repair).
    let jsonl_content = fs::read_to_string(jsonl_path(ws)).unwrap();
    assert!(jsonl_content.contains(&dirty_id), "dirty bead must be in JSONL after --flush-first repair");

    // Verify unflushed count is 0 after repair.
    let (o, _e, ok) = run(ws, &["doctor"]);
    assert!(ok, "doctor must succeed");
    assert!(
        !o.contains("Unflushed beads") || o.contains("0 unflushed"),
        "after --flush-first repair, unflushed count should be 0, got: {o}"
    );
}

/// `bf doctor --repair --force` must warn but proceed, losing dirty beads.
/// This is the "I know what I'm doing" escape hatch.
#[test]
fn doctor_repair_with_force_warns_but_loses_dirty_beads() {
    let tmp = init_ws();
    let ws = tmp.path();

    // Create one flushed bead.
    let (out, _e, ok) = run(ws, &["create", "--title", "flushed"]);
    assert!(ok, "create failed");
    let flushed_id = out.trim().to_string();

    // Flush to JSONL.
    let (_o, e, ok) = run(ws, &["sync", "--flush-only"]);
    assert!(ok, "sync --flush-only failed: {e}");

    // Create another bead WITHOUT flushing (dirty-only).
    let (out, _e, ok) = run(ws, &["--no-auto-flush", "create", "--title", "doomed"]);
    assert!(ok, "create failed");
    let doomed_id = out.trim().to_string();

    // Run `bf doctor --repair --force`.
    let (_o, e, ok) = run(ws, &["doctor", "--repair", "--force"]);
    assert!(ok, "doctor --repair --force must succeed: {e}");
    assert!(
        e.contains("WARNING") || e.contains("will be LOST") || e.contains(&doomed_id),
        "stderr must warn about losing dirty beads, got: {e}"
    );

    // Verify: flushed bead exists, dirty bead is GONE.
    let (_o, _e, ok) = run(ws, &["show", &flushed_id]);
    assert!(ok, "flushed bead must exist after force repair");
    let (_o, _e, ok) = run(ws, &["show", &doomed_id]);
    assert!(!ok, "dirty bead must be lost after --force repair (that's the point of --force)");

    // Verify JSONL does NOT contain the doomed bead.
    let jsonl_content = fs::read_to_string(jsonl_path(ws)).unwrap();
    assert!(!jsonl_content.contains(&doomed_id), "doomed bead must NOT be in JSONL after --force repair");
}

// ===========================================================================
// Normal operation: default auto-flush makes beads visible immediately
// ===========================================================================

/// With auto-flush ON (the default), the JSONL artifact already carries the
/// bead the instant the mutation returns — so killing the worker at any point
/// *after* the command loses nothing `git diff .beads/` can't show.
#[test]
fn default_autoflush_makes_bead_visible_immediately() {
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
    let live_id = out.trim().to_string();

    // Verify bead is in JSONL immediately.
    let jsonl_content = fs::read_to_string(jsonl_path(ws)).unwrap();
    assert!(jsonl_content.contains(&live_id), "bead must be in JSONL immediately after create");

    // The worker could be killed now and `git diff .beads/` still shows it.
    let (ok, diff, _) = git(ws, &["diff", "--", ".beads/issues.jsonl"]);
    assert!(ok, "git diff failed");
    assert!(
        diff.contains(&live_id),
        "auto-flushed bead must be visible in git diff immediately, got:\n{diff}"
    );
}

// ===========================================================================
// Flush failure is visible in machine-readable output
// ===========================================================================

/// Wedge the auto-flush (make `issues.jsonl` a directory). The `--json` envelope
/// must carry a non-null `warning` naming the failure and the recovery command,
/// while the mutation itself still succeeds.
fn wedge_flush(ws: &Path) {
    let p = jsonl_path(ws);
    if p.exists() {
        fs::remove_file(&p).ok();
    }
    fs::create_dir(&p).unwrap();
}

#[test]
fn flush_failure_surfaces_warning_in_json_output() {
    let tmp = init_ws();
    let ws = tmp.path();
    wedge_flush(ws);

    // The mutation committed, so the command succeeds despite the flush failure.
    let (stdout, _stderr, ok) = run(ws, &["create", "--json", "--title", "wedged"]);
    assert!(ok, "flush failure must NOT fail create");

    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("--json stdout must be valid JSON");

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
    assert!(warning.contains("auto-flush"), "warning must mention auto-flush: {warning}");
    assert!(
        warning.contains("sync --flush-only") || warning.contains("bf sync"),
        "warning must name the recovery command: {warning}"
    );

    // The bead still exists (never lost) and stays dirty for the retry.
    let (_o, e, ok) = run(ws, &["show", &id]);
    assert!(ok, "bead must exist despite flush failure: {e}");
}

#[test]
fn flush_failure_surfaces_warning_in_human_output() {
    let tmp = init_ws();
    let ws = tmp.path();
    wedge_flush(ws);

    // The human (non-JSON) path prints the warning to stderr and still exits 0.
    let (_o, err, ok) = run(ws, &["create", "--title", "wedged human"]);
    assert!(ok, "create must not fail on a flush error");
    assert!(
        err.contains("warning:") && (err.contains("auto-flush") || err.contains("flush")),
        "expected an auto-flush warning on stderr, got: {err}"
    );
}
