//! Integration tests for Phase 7.1 child 4/5 (bead bf-bziwd).
//!
//! Two guarantees that close out the parent (bf-1wg2v):
//!
//! 1. **Diagnostic / read-only surfaces not already pinned by `autoflush_readonly.rs`
//!    (child 3, bf-2rjhk) never rewrite `.beads/issues.jsonl`.** Upstream lesson #326:
//!    a status/doctor-style read that "helpfully" rewrites the JSONL artifact creates
//!    spurious git churn and can mask real drift. Child 3 pinned the broad read-only
//!    sweep (list/show/ready/doctor/labels/…). This file pins the remaining surfaces
//!    named in bf-bziwd's scope:
//!    * `doctor --repair` / `--fix-schema` / `--reclaim-stale` on a **healthy**
//!      workspace — these are the doctor sub-flags that *can* write (to the db), so
//!      they are precisely the ones that must be pinned to never touch JSONL.
//!    * `commit-check` — the pre-commit secret scan.
//!    * `status`, `sync --status`, `doctor --json` — not (currently) bf commands;
//!      the test pins that their invocation is *rejected* by clap **and** leaves
//!      JSONL untouched, so a future addition can't silently regress.
//!
//! 2. **Rotation interplay (plan open question, `docs/plan/plan.md` §Open questions):
//!    incremental auto-flush targets ONLY the active `issues.jsonl`, never rotated
//!    archives.** A workspace with a rotated archive (`issues.jsonl.1`) present is
//!    mutated; the active file must change (the mutation landed) while the archive
//!    stays byte-for-byte identical and its mtime unchanged.
//!
//! Every assertion is the same dual invariant as child 3: byte-identical content
//! (catches drift-masking rewrites) **and** unchanged mtime (catches a rewrite that
//! reproduces identical bytes — wasted I/O / churn visible to anything watching
//! mtimes).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

fn bf() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bf"))
}

/// Run `bf` in `ws`; returns (stdout, stderr, exited-successfully).
fn run(ws: &Path, args: &[&str]) -> (String, String, bool) {
    let out = bf()
        .current_dir(ws)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute bf {args:?}: {e}"));
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

/// Run a `bf` command expected to succeed; returns trimmed stdout.
fn ok(ws: &Path, args: &[&str]) -> String {
    let (out, err, success) = run(ws, args);
    assert!(
        success,
        "bf {args:?} failed: {err}"
    );
    out.trim().to_string()
}

/// Fresh workspace via `bf init` (a real on-disk `.beads/`).
fn init_ws() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let (_o, _e, success) = run(tmp.path(), &["init", "--prefix", "bf"]);
    assert!(success, "bf init failed");
    tmp
}

/// Create a bead (auto-flush on) and return its id.
fn create(ws: &Path, title: &str) -> String {
    ok(ws, &["create", "--title", title])
}

fn jsonl_path(ws: &Path) -> PathBuf {
    ws.join(".beads").join("issues.jsonl")
}

/// (content bytes, mtime) snapshot of a file — the invariant every read-only
/// command must preserve on `issues.jsonl`, and every archive must preserve
/// across an auto-flush.
fn snapshot(path: &Path) -> (Vec<u8>, SystemTime) {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mtime = fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("mtime {}: {e}", path.display()));
    (bytes, mtime)
}

/// Assert `issues.jsonl` is byte-identical AND its mtime is unchanged vs `snap`.
fn assert_jsonl_unchanged(ws: &Path, snap: &(Vec<u8>, SystemTime), label: &str) {
    let (bytes, mtime) = snapshot(&jsonl_path(ws));
    assert_eq!(
        bytes, snap.0,
        "{label}: rewrote issues.jsonl (content drift — the #326 regression)"
    );
    assert_eq!(
        mtime, snap.1,
        "{label}: bumped issues.jsonl mtime (write churn)"
    );
}

// ---------------------------------------------------------------------------
// Part 1 — diagnostic / read-only surfaces never rewrite the active JSONL.
// ---------------------------------------------------------------------------

/// `doctor`'s write-capable sub-flags (`--repair`, `--fix-schema`, `--reclaim-stale`)
/// each CAN mutate the SQLite store — so they are exactly the surfaces that must be
/// pinned to never lay a hand on `issues.jsonl`. On a HEALTHY workspace (no unflushed
/// beads, no corruption, no stale claims) each is effectively a no-op, but the point
/// of the regression is that even the no-op path takes no write handle on the JSONL.
#[test]
fn doctor_write_flags_on_healthy_workspace_leave_jsonl_untouched() {
    let ws = init_ws();
    // Seed a canonical, fully-flushed workspace (each create auto-flushes). No
    // unflushed beads, no corruption → every doctor sub-flag is a clean no-op.
    let _id = create(ws.path(), "Healthy bead");

    let snap = snapshot(&jsonl_path(ws.path()));
    // Sleep past coarse (1s) filesystem mtime granularity so any later write would
    // land in a strictly-later mtime bucket and be detectable.
    std::thread::sleep(Duration::from_millis(1100));

    // Parametric over the doctor write-flags named in scope (--repair is the named
    // one; --fix-schema and --reclaim-stale are its sibling write-flags, pinned for
    // the same reason at no extra cost).
    for args in [
        &["doctor", "--repair"][..],
        &["doctor", "--fix-schema"][..],
        &["doctor", "--reclaim-stale"][..],
    ] {
        let (_out, _err, success) = run(ws.path(), args);
        assert!(
            success,
            "bf {args:?} unexpectedly failed on a healthy workspace; cannot confirm it left JSONL untouched"
        );
        assert_jsonl_unchanged(ws.path(), &snap, &format!("bf {args:?}"));
    }
}

/// `commit-check` is a pre-commit diagnostic: it scans *staged* `.beads/` changes
/// for secrets via `git diff --cached`. It must read only — never rewrite the JSONL
/// artifact it is ostensibly guarding.
#[test]
fn commit_check_never_writes_jsonl() {
    let ws = init_ws();
    let _id = create(ws.path(), "Bead to commit-check");

    // commit-check shells out to git, so give the workspace a repo and stage the
    // artifact so the real scan path (diff --cached) is exercised. No secrets → exit 0.
    let git_ok = Command::new("git")
        .current_dir(ws.path())
        .args(["init"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    assert!(git_ok, "git init failed (is git on PATH?)");
    Command::new("git")
        .current_dir(ws.path())
        .args(["add", ".beads/issues.jsonl"])
        .output()
        .expect("git add failed");

    let snap = snapshot(&jsonl_path(ws.path()));
    std::thread::sleep(Duration::from_millis(1100));

    let (_out, _err, success) = run(ws.path(), &["commit-check"]);
    assert!(success, "bf commit-check failed on a clean workspace");
    assert_jsonl_unchanged(ws.path(), &snap, "bf commit-check");
}

/// `status`, `sync --status`, and `doctor --json` are NOT bf commands (bf has no
/// standalone `status`, `sync --status` flag, or `doctor --json` flag). The contract
/// this pins: such an invocation is *rejected* by clap (non-zero exit, no handler
/// runs) AND leaves `issues.jsonl` untouched — so a future command addition can never
/// silently introduce a read-path that writes JSONL.
#[test]
fn unknown_readonly_invocations_leave_jsonl_untouched() {
    let ws = init_ws();
    let _id = create(ws.path(), "Bead");

    let snap = snapshot(&jsonl_path(ws.path()));
    std::thread::sleep(Duration::from_millis(1100));

    for args in [
        &["status"][..],
        &["sync", "--status"][..],
        &["doctor", "--json"][..],
    ] {
        let (_out, _err, success) = run(ws.path(), args);
        // These are not real bf commands/flags — clap must reject them rather than
        // silently fall through to some handler that could touch JSONL.
        assert!(
            !success,
            "bf {args:?} unexpectedly succeeded; expected clap to reject an unknown command/flag"
        );
        assert_jsonl_unchanged(ws.path(), &snap, &format!("bf {args:?} (unknown)"));
    }
}

// ---------------------------------------------------------------------------
// Part 2 — rotation interplay: auto-flush targets ONLY the active JSONL.
// ---------------------------------------------------------------------------

/// Resolve the plan's open question ("Auto-flush × rotation: incremental flush must
/// target only the active `issues.jsonl`, never archives").
///
/// Setup mimics a workspace that has already rotated: the active `issues.jsonl`
/// holds a live bead, and a rotated archive `issues.jsonl.1` holds an old closed
/// bead. A mutation then fires the incremental auto-flush. The active file MUST
/// change (the mutation landed via [`export_jsonl_merge`]) while the archive stays
/// byte-for-byte identical with an unchanged mtime — proving the flush path never
/// touches rotated archives (which only [`crate::rotate::rotate`] ever writes).
#[test]
fn autoflush_targets_only_active_jsonl_not_archives() {
    let ws = init_ws();
    let active_id = create(ws.path(), "Active bead");

    // Simulate a prior rotation: hand-write a closed bead into issues.jsonl.1.
    // This is a real, parseable JSONL line (same shape `bf`/`rotate` produce) so the
    // archive is a genuine rotation artifact, not a sentinel.
    let archive_path = ws.path().join(".beads").join("issues.jsonl.1");
    fs::write(
        &archive_path,
        concat!(
            r#"{"id":"bf-arch1","title":"Archived closed bead","status":"closed","#,
            r#""priority":2,"type":"task","#,
            r#""created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","#,
            r#""source_repo":"."}"#,
            "\n"
        ),
    )
    .unwrap();

    // Sleep past 1s mtime granularity so the impending active rewrite lands in a
    // strictly-later mtime bucket than the create — detectable as a change.
    std::thread::sleep(Duration::from_millis(1100));

    let active_before = snapshot(&jsonl_path(ws.path()));
    let archive_before = snapshot(&archive_path);

    // Mutation → auto-flush rewrites ONLY the active file (surgical line replace).
    let (_out, err, success) = run(ws.path(), &["update", &active_id, "--status", "in_progress"]);
    assert!(success, "bf update failed: {err}");

    // Active file: the mutation must have landed (status changed) AND the file must
    // have been rewritten (mtime advanced past the snapshot).
    let (active_after_bytes, active_after_mtime) = snapshot(&jsonl_path(ws.path()));
    assert_ne!(
        active_after_bytes, active_before.0,
        "active issues.jsonl should have changed after a mutating auto-flush"
    );
    assert!(
        active_after_mtime > active_before.1,
        "active issues.jsonl mtime should have advanced after the flush"
    );
    assert!(
        String::from_utf8_lossy(&active_after_bytes)
            .contains(r#""status":"in_progress""#),
        "the mutated bead's new status must be present in the active file"
    );

    // Archive: the crux of the open question — byte-for-byte identical AND mtime
    // unchanged. The incremental flush must never touch a rotated archive.
    let (archive_after_bytes, archive_after_mtime) = snapshot(&archive_path);
    assert_eq!(
        archive_after_bytes, archive_before.0,
        "auto-flush must NOT rewrite the rotated archive (issues.jsonl.1)"
    );
    assert_eq!(
        archive_after_mtime, archive_before.1,
        "auto-flush must NOT bump the rotated archive's mtime"
    );

    // The archive still parses and still holds exactly the rotated closed bead —
    // untouched, not duplicated, not merged into.
    let archived_ids: Vec<String> = String::from_utf8_lossy(&archive_after_bytes)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            serde_json::from_str::<serde_json::Value>(l).unwrap()["id"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(archived_ids, vec!["bf-arch1".to_string()]);
    assert!(
        !String::from_utf8_lossy(&archive_after_bytes).contains(&active_id),
        "the active (mutated) bead must not have leaked into the archive"
    );
}
