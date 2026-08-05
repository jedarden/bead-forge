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
    assert!(
        git(ws, &["commit", "-m", "baseline"]).0,
        "git commit failed"
    );

    // A mutation whose flush did not run (worker "killed" before flush, or
    // auto-flush disabled). The db commit + dirty mark land; JSONL is untouched.
    let (out, _e, ok) = run(
        ws,
        &["--no-auto-flush", "create", "--title", "killed worker bead"],
    );
    assert!(ok, "create failed");
    let killed_id = out.trim().to_string();

    // (1) The bead is NOT lost: it is present in the live store.
    let (_out, _e, ok) = run(ws, &["show", &killed_id]);
    assert!(ok, "bead must be readable from db (not lost)");

    // (2) Recoverable from a fresh connection — models a worker restart after
    // the kill. A brand-new Storage handle still reads the committed bead.
    let (out, _e, ok) = run(ws, &["show", &killed_id]);
    assert!(ok, "bead must be readable after 'worker restart': {out:?}");
    assert!(
        out.contains(&killed_id) || out.contains("ID:"),
        "show output must contain the bead id or ID: header"
    );

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

/// Regression test for the 2026-06-10 wipe, updated for the Phase 7.2 doctor
/// safety stack: on a workspace whose only "problem" is unflushed (db-only)
/// beads, `bf doctor --repair` is a **healthy no-op**. Unflushed beads are a
/// flush concern, not corruption, so the JSONL rebuild is never reached (layer 1)
/// and the dirty bead cannot be lost. This is strictly safer than the old
/// refuse-or-force guard it replaces.
#[test]
fn doctor_repair_on_unflushed_only_is_a_safe_noop() {
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

    // `bf doctor --repair` on a healthy-but-dirty workspace SUCCEEDS as a no-op:
    // it does NOT rebuild, and it does NOT lose the dirty bead.
    let (o, _e, ok) = run(ws, &["doctor", "--repair"]);
    assert!(
        ok,
        "doctor --repair must succeed (safe no-op) on an unflushed-only workspace"
    );
    assert!(
        o.contains("healthy") && o.contains("no JSONL rebuild"),
        "repair must report a healthy no-op, got: {o}"
    );

    // Verify the dirty bead STILL EXISTS (not silently lost).
    let (_o, e, ok) = run(ws, &["show", &dirty_id]);
    assert!(ok, "dirty bead must still exist after no-op repair: {e}");

    // And it is still db-only (repair did not flush it — that stays a `bf sync`
    // concern), so no data was silently checkpointed either.
    let jsonl_content = fs::read_to_string(jsonl_path(ws)).unwrap();
    assert!(
        !jsonl_content.contains(&dirty_id),
        "no-op repair must not flush the dirty bead"
    );
}

/// `bf doctor --repair --flush-first` on a *healthy* workspace (bf-ku8hv):
/// `--flush-first` is scoped to protecting beads immediately before a genuine
/// JSONL rebuild. When health-check finds no rebuild is needed, that scope
/// never opens, so repair must NOT perform a side-effecting flush on its own
/// (that would violate the "a repair that repairs nothing must not write"
/// contract) — instead it reports the dirty count and points at the
/// canonical checkpoint command. The dirty bead must still be safe (present
/// in the db, not lost), just not yet promoted to the JSONL authority.
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
    let (o, e, ok) = run(ws, &["doctor", "--repair", "--flush-first"]);
    assert!(ok, "doctor --repair --flush-first must succeed: {e}");
    assert!(
        o.contains("healthy") && o.contains("no JSONL rebuild"),
        "healthy workspace must report a no-op repair, got: {o}"
    );

    // Verify BOTH beads exist after repair (nothing lost).
    let (_o, e, ok) = run(ws, &["show", &flushed_id]);
    assert!(ok, "flushed bead must exist after repair: {e}");
    let (_o, e, ok) = run(ws, &["show", &dirty_id]);
    assert!(ok, "dirty bead must exist after repair: {e}");

    // On a healthy workspace, `--flush-first` must NOT perform a side-effecting
    // flush of its own (bf-ku8hv) — the dirty bead stays db-only until an
    // explicit `bf sync --flush-only`, and repair must say so.
    let jsonl_content = fs::read_to_string(jsonl_path(ws)).unwrap();
    assert!(
        !jsonl_content.contains(&dirty_id),
        "healthy-path --flush-first must not silently write the dirty bead to JSONL"
    );
    assert!(
        o.contains("unflushed") && o.contains("sync --flush-only"),
        "must point at the canonical checkpoint command, got: {o}"
    );

    // The dirty bead remains reported as unflushed until explicitly checkpointed.
    let (o, _e, ok) = run(ws, &["doctor"]);
    assert!(ok, "doctor must succeed");
    assert!(
        o.contains("Unflushed beads: 1"),
        "dirty bead should still be reported unflushed (not lost, not silently flushed), got: {o}"
    );

    // Explicit checkpoint is the documented recovery path, and it works.
    let (_o, e, ok) = run(ws, &["sync", "--flush-only"]);
    assert!(ok, "sync --flush-only failed: {e}");
    let jsonl_content = fs::read_to_string(jsonl_path(ws)).unwrap();
    assert!(
        jsonl_content.contains(&dirty_id),
        "dirty bead must be in JSONL after explicit flush"
    );
}

/// Phase 7.2 safety improvement: `bf doctor --repair --force` on a *healthy*
/// workspace does NOT lose dirty beads, because the JSONL rebuild is unreachable
/// from a healthy state (layer 1) — `--force` only opts out of dirty-bead
/// preservation *when a rebuild is actually triggered* by corruption/divergence.
/// The old contract, where `--force` unconditionally rebuilt and dropped db-only
/// beads even on a clean workspace, was itself the 2026-06-10 footgun.
#[test]
fn doctor_repair_force_on_healthy_workspace_does_not_lose_dirty() {
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
    let (out, _e, ok) = run(ws, &["--no-auto-flush", "create", "--title", "kept"]);
    assert!(ok, "create failed");
    let kept_id = out.trim().to_string();

    // Run `bf doctor --repair --force`. No corruption/divergence exists, so no
    // rebuild happens and --force has nothing to discard.
    let (o, _e, ok) = run(ws, &["doctor", "--repair", "--force"]);
    assert!(
        ok,
        "doctor --repair --force must succeed on a healthy workspace"
    );
    assert!(
        o.contains("healthy") && o.contains("no JSONL rebuild"),
        "healthy workspace must not rebuild even with --force, got: {o}"
    );

    // Both beads survive — --force did NOT nuke the dirty bead on a healthy tree.
    let (_o, _e, ok) = run(ws, &["show", &flushed_id]);
    assert!(ok, "flushed bead must exist after force repair");
    let (_o, e, ok) = run(ws, &["show", &kept_id]);
    assert!(
        ok,
        "dirty bead must survive --force on a healthy workspace: {e}"
    );
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
    assert!(
        jsonl_content.contains(&live_id),
        "bead must be in JSONL immediately after create"
    );

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

    // Mutation commands always wrap `--json` output in the standard envelope
    // ({version, kind, data, warning?}) so a warning can ride alongside the
    // result — the id lives under `data`, not at the envelope's top level.
    let id = parsed
        .get("data")
        .and_then(|d| d.get("id"))
        .and_then(|v| v.as_str())
        .expect("--json must carry the created id under data.id")
        .to_string();

    // Machine-readable surface: a non-null `warning` string that is actionable.
    let warning = parsed
        .get("warning")
        .and_then(|v| v.as_str())
        .expect("--json must carry a non-null `warning` on flush failure");
    assert!(
        warning.contains("auto-flush"),
        "warning must mention auto-flush: {warning}"
    );
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
