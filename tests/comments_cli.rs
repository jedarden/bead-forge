//! Integration test for the `bf comments add` / `comments list` CLI round-trip.
//!
//! Ports the scenario exercised by the orphaned root-level script
//! `test_bf_test3.sh` (removed in the bf-3o9 repo-hygiene cleanup) into a real
//! cargo integration test. The storage layer (`add_comment` / `list_comments`)
//! is already exercised by `dirty_tracking.rs` and the `comments` table schema
//! is checked by `br_isolation.rs`, but no committed test drove the
//! `comments add` -> `comments list` CLI path end-to-end until now (audit
//! bf-5wz0l flagged this as one of two coverage gaps).

use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn bf() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bf"))
}

/// Run `bf` with args in `workspace`, returning (stdout, stderr, success).
fn run_bf(workspace: &Path, args: &[&str]) -> (String, String, bool) {
    let output = bf()
        .current_dir(workspace)
        .args(args)
        .output()
        .expect("failed to execute bf");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.success(),
    )
}

fn setup() -> (TempDir, PathBuf) {
    let temp = TempDir::new().unwrap();
    let workspace = temp.path().to_path_buf();
    let (_o, e, ok) = run_bf(&workspace, &["init", "--prefix", "bf"]);
    assert!(ok, "bf init failed: {e}");
    (temp, workspace)
}

/// Create a bead and return its bare ID (create prints the bare ID on stdout).
fn create_bead(workspace: &Path, title: &str) -> String {
    let (out, err, ok) = run_bf(workspace, &["create", "--title", title]);
    assert!(ok, "bf create failed: {err}");
    let id = out.trim().to_string();
    assert!(!id.is_empty(), "create produced no id: {out}");
    id
}

#[test]
fn comments_add_and_list_round_trip() {
    let (_temp, ws) = setup();
    let id = create_bead(&ws, "Test bead for comments round-trip");

    // A bead with no comments reports so.
    let (out, err, ok) = run_bf(&ws, &["comments", "list", &id]);
    assert!(ok, "comments list failed: {err}");
    assert!(out.contains("No comments"), "unexpected empty-list output: {out}");

    // Add a single comment.
    let (out, err, ok) = run_bf(&ws, &["comments", "add", &id, "This is a test comment"]);
    assert!(ok, "comments add failed: {err}");
    assert!(out.contains("Added comment"), "add did not confirm: {out}");

    // Listing now surfaces the comment body.
    let (out, err, ok) = run_bf(&ws, &["comments", "list", &id]);
    assert!(ok, "comments list failed: {err}");
    assert!(
        out.contains("This is a test comment"),
        "comment body missing from list: {out}"
    );
}

#[test]
fn comments_list_preserves_insertion_order() {
    let (_temp, ws) = setup();
    let id = create_bead(&ws, "Multi-comment bead");

    let bodies = ["First comment", "Second comment", "Third comment"];
    for body in &bodies {
        let (_o, e, ok) = run_bf(&ws, &["comments", "add", &id, body]);
        assert!(ok, "comments add failed: {e}");
    }

    let (out, err, ok) = run_bf(&ws, &["comments", "list", &id]);
    assert!(ok, "comments list failed: {err}");

    // All three present, in the order they were added.
    let first = out
        .find("First comment")
        .expect("First comment missing");
    let second = out
        .find("Second comment")
        .expect("Second comment missing");
    let third = out
        .find("Third comment")
        .expect("Third comment missing");
    assert!(first < second && second < third, "order not preserved: {out}");
}

#[test]
fn comments_add_joins_multiple_text_args() {
    // The CLI joins multiple text args with spaces, so quoting is optional.
    let (_temp, ws) = setup();
    let id = create_bead(&ws, "Join-args bead");

    let (out, err, ok) = run_bf(&ws, &["comments", "add", &id, "multi", "word", "comment"]);
    assert!(ok, "comments add failed: {err}");
    assert!(out.contains("Added comment"), "add did not confirm: {out}");

    let (out, err, ok) = run_bf(&ws, &["comments", "list", &id]);
    assert!(ok, "comments list failed: {err}");
    assert!(
        out.contains("multi word comment"),
        "joined text not round-tripped: {out}"
    );
}
