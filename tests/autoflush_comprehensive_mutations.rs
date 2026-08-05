//! Comprehensive tests for autoflush behavior during all mutation types.
//!
//! This suite provides unified coverage for:
//! - Auto-flush after create operations
//! - Auto-flush after update operations
//! - Auto-flush after claim operations
//! - Auto-flush after close operations
//! - Auto-flush after delete operations
//! - Auto-flush after dependency operations
//! - Auto-flush after label operations
//! - Auto-flush after comment operations
//! - Auto-flush after annotation operations
//! - Verification of issues.jsonl correctness after each mutation
//! - Tests with sync.auto_flush enabled and disabled
//!
//! Reference patterns from existing autoflush_*.rs tests.

use std::fs;
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

fn jsonl_path(workspace: &Path) -> PathBuf {
    workspace.join(".beads").join("issues.jsonl")
}

/// Parse every non-empty line of `issues.jsonl` into a JSON object.
fn read_beads(workspace: &Path) -> Vec<serde_json::Value> {
    let path = jsonl_path(workspace);
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("issues.jsonl unreadable at {}: {e}", path.display()));
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).expect("issues.jsonl line is not valid JSON"))
        .collect()
}

/// Locate a bead by id in `issues.jsonl`, failing loudly if the flush did not
/// write it.
fn find_bead(workspace: &Path, id: &str) -> Option<Value> {
    read_beads(workspace)
        .into_iter()
        .find(|b| b.get("id").and_then(|v| v.as_str()) == Some(id))
}

/// Get a string field from a bead JSON value, returning empty string if missing.
fn field(bead: &Value, key: &str) -> String {
    bead.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_default()
}

/// Get an integer field from a bead JSON value.
fn field_int(bead: &Value, key: &str) -> i64 {
    bead.get(key).and_then(|v| v.as_i64()).unwrap_or(0)
}

use serde_json::Value;

// ==================== CREATE OPERATIONS ====================

#[test]
fn create_autoflush_creates_issues_jsonl_entry() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Test bead");

    // Verify issues.jsonl exists and contains the bead
    let bead = find_bead(&ws, &id).expect("created bead must be in issues.jsonl");
    assert_eq!(field(&bead, "title"), "Test bead");
    assert_eq!(field(&bead, "status"), "open");
    assert_eq!(field(&bead, "issue_type"), "task");
}

#[test]
fn create_autoflush_with_all_fields() {
    let (_t, ws) = setup();
    let (out, err, ok) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "Full bead",
            "--type",
            "bug",
            "--priority",
            "1",
            "--assignee",
            "tester",
            "--label",
            "urgent",
            "--label",
            "bug",
            "--description",
            "Test description",
        ],
    );
    assert!(ok, "create failed: {err}");
    let id = out.trim().to_string();

    let bead = find_bead(&ws, &id).expect("bead must be flushed");
    assert_eq!(field(&bead, "title"), "Full bead");
    assert_eq!(field(&bead, "issue_type"), "bug");
    assert_eq!(field_int(&bead, "priority"), 1);
    assert_eq!(field(&bead, "assignee"), "tester");

    // Check labels
    let labels = bead
        .get("labels")
        .and_then(|v| v.as_array())
        .expect("labels must be present");
    assert_eq!(labels.len(), 2);
    assert!(labels.iter().any(|l| l.as_str() == Some("urgent")));
    assert!(labels.iter().any(|l| l.as_str() == Some("bug")));
}

#[test]
fn create_no_autoflush_leaves_jsonl_empty() {
    let (_t, ws) = setup();
    let (out, err, ok) = run_bf(&ws, &["--no-auto-flush", "create", "--title", "No flush"]);
    assert!(ok, "create failed: {err}");
    let id = out.trim().to_string();

    // With --no-auto-flush, issues.jsonl should not exist or be empty
    assert!(
        !jsonl_path(&ws).exists() || read_beads(&ws).is_empty(),
        "--no-auto-flush should not create issues.jsonl"
    );

    // Bead exists in database (can verify by running a manual flush)
    let (_o, e, ok) = run_bf(&ws, &["sync", "--flush-only"]);
    assert!(ok, "flush failed: {e}");

    let bead = find_bead(&ws, &id).expect("bead should appear after manual flush");
    assert_eq!(field(&bead, "title"), "No flush");
}

#[test]
fn create_config_auto_flush_disabled() {
    let (_t, ws) = setup();

    // Disable auto-flush in config
    let cfg = ws.join(".beads").join("config.yaml");
    let mut content = fs::read_to_string(&cfg).unwrap();
    content.push_str("sync:\n  auto_flush: false\n");
    fs::write(&cfg, content).unwrap();

    let (_o, e, ok) = run_bf(&ws, &["create", "--title", "Config disabled"]);
    assert!(ok, "create failed: {e}");

    // With config disabled, issues.jsonl should not exist
    assert!(
        !jsonl_path(&ws).exists(),
        "sync.auto_flush: false should not create issues.jsonl"
    );
}

// ==================== UPDATE OPERATIONS ====================

#[test]
fn update_autoflush_modifies_existing_entry() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Original title");

    let (_o, e, ok) = run_bf(&ws, &["update", &id, "--title", "Updated title"]);
    assert!(ok, "update failed: {e}");

    let bead = find_bead(&ws, &id).expect("updated bead must be in issues.jsonl");
    assert_eq!(
        field(&bead, "title"),
        "Updated title",
        "title should be updated in JSONL"
    );
}

#[test]
fn update_autoflush_status_change() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Status test");

    let (_o, e, ok) = run_bf(&ws, &["update", &id, "--status", "in_progress"]);
    assert!(ok, "update failed: {e}");

    let bead = find_bead(&ws, &id).expect("bead must be present");
    assert_eq!(field(&bead, "status"), "in_progress");
}

#[test]
fn update_autoflush_priority_change() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Priority test");

    let (_o, e, ok) = run_bf(&ws, &["update", &id, "--priority", "1"]);
    assert!(ok, "update failed: {e}");

    let bead = find_bead(&ws, &id).expect("bead must be present");
    assert_eq!(field_int(&bead, "priority"), 1);
}

#[test]
fn update_no_autoflush_doesnt_modify_jsonl() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Before update");

    let before_json = fs::read_to_string(jsonl_path(&ws)).unwrap();

    let (_o, e, ok) = run_bf(
        &ws,
        &["--no-auto-flush", "update", &id, "--title", "After update"],
    );
    assert!(ok, "update failed: {e}");

    let after_json = fs::read_to_string(jsonl_path(&ws)).unwrap();
    assert_eq!(
        before_json, after_json,
        "JSONL should not change with --no-auto-flush update"
    );

    let bead = find_bead(&ws, &id).expect("bead should still have old title in JSONL");
    assert_eq!(field(&bead, "title"), "Before update");
}

// ==================== CLAIM OPERATIONS ====================

#[test]
fn claim_autoflush_updates_assignee_and_status() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Claimable bead");

    let (out, err, ok) = run_bf(&ws, &["claim", "--assignee", "worker-1"]);
    assert!(ok, "claim failed: {err}");

    // Verify the output contains the bead ID
    assert!(out.contains(&id), "claim output should contain bead ID");

    let bead = find_bead(&ws, &id).expect("claimed bead must be in issues.jsonl");
    assert_eq!(
        field(&bead, "assignee"),
        "worker-1",
        "assignee should be updated"
    );
    assert_eq!(
        field(&bead, "status"),
        "in_progress",
        "status should change to in_progress"
    );
}

#[test]
fn claim_autoflush_with_json_output() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "JSON claim test");

    let (out, err, ok) = run_bf(&ws, &["claim", "--assignee", "worker-2", "--json"]);
    assert!(ok, "claim failed: {err}");

    let parsed: Value =
        serde_json::from_str(out.trim()).expect("claim --json should be valid JSON");
    assert_eq!(
        parsed.get("bead_id").and_then(|v| v.as_str()),
        Some(id.as_str())
    );

    let bead = find_bead(&ws, &id).expect("claimed bead must be flushed");
    assert_eq!(field(&bead, "assignee"), "worker-2");
    assert_eq!(field(&bead, "status"), "in_progress");
}

#[test]
fn claim_no_autoflush_doesnt_modify_jsonl() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "No-flush claim");

    let before_json = fs::read_to_string(jsonl_path(&ws)).unwrap();

    let (_o, e, ok) = run_bf(&ws, &["--no-auto-flush", "claim", "--assignee", "worker-3"]);
    assert!(ok, "claim failed: {e}");

    let after_json = fs::read_to_string(jsonl_path(&ws)).unwrap();
    assert_eq!(before_json, after_json, "JSONL should not change");

    let bead = find_bead(&ws, &id).expect("bead should still be in JSONL");
    assert_eq!(
        field(&bead, "status"),
        "open",
        "status should not change in JSONL"
    );
    assert!(
        field(&bead, "assignee").is_empty()
            || field(&bead, "assignee") == "null"
            || field(&bead, "assignee") == ""
    );
}

// ==================== CLOSE OPERATIONS ====================

#[test]
fn close_autoflush_updates_status_and_reason() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Closable bead");

    let (_o, e, ok) = run_bf(&ws, &["close", &id, "--reason", "Task completed"]);
    assert!(ok, "close failed: {e}");

    let bead = find_bead(&ws, &id).expect("closed bead must be in issues.jsonl");
    assert_eq!(field(&bead, "status"), "closed");
    assert_eq!(field(&bead, "close_reason"), "Task completed");
}

#[test]
fn close_without_reason_autoflushes() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Close without reason");

    let (_o, e, ok) = run_bf(&ws, &["close", &id]);
    assert!(ok, "close failed: {e}");

    let bead = find_bead(&ws, &id).expect("closed bead must be in issues.jsonl");
    assert_eq!(field(&bead, "status"), "closed");
}

#[test]
fn close_no_autoflush_doesnt_modify_jsonl() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "No-flush close");

    let before_json = fs::read_to_string(jsonl_path(&ws)).unwrap();

    let (_o, e, ok) = run_bf(&ws, &["--no-auto-flush", "close", &id, "--reason", "Done"]);
    assert!(ok, "close failed: {e}");

    let after_json = fs::read_to_string(jsonl_path(&ws)).unwrap();
    assert_eq!(before_json, after_json, "JSONL should not change");

    let bead = find_bead(&ws, &id).expect("bead should still be open in JSONL");
    assert_eq!(field(&bead, "status"), "open");
}

// ==================== REOPEN OPERATIONS ====================

#[test]
fn reopen_autoflush_resets_status() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Reopenable bead");

    // Close first
    let (_o, e, ok) = run_bf(&ws, &["close", &id, "--reason", "Initial close"]);
    assert!(ok, "close failed: {e}");

    // Reopen
    let (_o, e, ok) = run_bf(&ws, &["reopen", &id]);
    assert!(ok, "reopen failed: {e}");

    let bead = find_bead(&ws, &id).expect("reopened bead must be in issues.jsonl");
    assert_eq!(field(&bead, "status"), "open");
}

#[test]
fn reopen_no_autoflush_doesnt_modify_jsonl() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "No-flush reopen");

    // Close first (with flush)
    let (_o, e, ok) = run_bf(&ws, &["close", &id, "--reason", "Initial"]);
    assert!(ok, "close failed: {e}");

    let before_json = fs::read_to_string(jsonl_path(&ws)).unwrap();

    // Reopen without flush
    let (_o, e, ok) = run_bf(&ws, &["--no-auto-flush", "reopen", &id]);
    assert!(ok, "reopen failed: {e}");

    let after_json = fs::read_to_string(jsonl_path(&ws)).unwrap();
    assert_eq!(before_json, after_json, "JSONL should not change");

    let bead = find_bead(&ws, &id).expect("bead should still be closed in JSONL");
    assert_eq!(field(&bead, "status"), "closed");
}

// ==================== DELETE OPERATIONS ====================

#[test]
fn delete_autoflush_removes_entry() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Deletable bead");

    // Verify it exists
    assert!(
        find_bead(&ws, &id).is_some(),
        "bead should exist before delete"
    );

    let (_o, e, ok) = run_bf(&ws, &["delete", &id]);
    assert!(ok, "delete failed: {e}");

    // Verify it's gone from JSONL
    assert!(
        find_bead(&ws, &id).is_none(),
        "deleted bead should not be in issues.jsonl"
    );
}

#[test]
fn delete_autoflush_preserves_other_beads() {
    let (_t, ws) = setup();
    let id1 = create_bead(&ws, "Bead 1");
    let id2 = create_bead(&ws, "Bead 2");
    let id3 = create_bead(&ws, "Bead 3");

    let (_o, e, ok) = run_bf(&ws, &["delete", &id2]);
    assert!(ok, "delete failed: {e}");

    // Verify id2 is gone
    assert!(
        find_bead(&ws, &id2).is_none(),
        "deleted bead should be removed"
    );

    // Verify others remain
    assert!(find_bead(&ws, &id1).is_some(), "other beads should remain");
    assert!(find_bead(&ws, &id3).is_some(), "other beads should remain");
}

#[test]
fn delete_no_autoflush_doesnt_modify_jsonl() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "No-flush delete");

    let before_json = fs::read_to_string(jsonl_path(&ws)).unwrap();

    let (_o, e, ok) = run_bf(&ws, &["--no-auto-flush", "delete", &id]);
    assert!(ok, "delete failed: {e}");

    let after_json = fs::read_to_string(jsonl_path(&ws)).unwrap();
    assert_eq!(before_json, after_json, "JSONL should not change");

    // Bead should still be in JSONL
    assert!(find_bead(&ws, &id).is_some(), "bead should remain in JSONL");
}

// ==================== DEPENDENCY OPERATIONS ====================

#[test]
fn dep_add_autoflush_adds_dependency() {
    let (_t, ws) = setup();
    let blocker = create_bead(&ws, "Blocker");
    let blocked = create_bead(&ws, "Blocked");

    let (_o, e, ok) = run_bf(&ws, &["dep", "add", &blocker, "--blocks", &blocked]);
    assert!(ok, "dep add failed: {e}");

    let bead = find_bead(&ws, &blocked).expect("blocked bead must be in issues.jsonl");
    let empty_deps = vec![];
    let deps = bead
        .get("dependencies")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty_deps);

    assert!(
        deps.iter()
            .any(|d| d.get("depends_on_id").and_then(|v| v.as_str()) == Some(blocker.as_str())),
        "blocked bead should depend on blocker"
    );
}

#[test]
fn dep_remove_autoflush_removes_dependency() {
    let (_t, ws) = setup();
    let blocker = create_bead(&ws, "Blocker");
    let blocked = create_bead(&ws, "Blocked");

    // Add dependency
    let (_o, e, ok) = run_bf(&ws, &["dep", "add", &blocker, "--blocks", &blocked]);
    assert!(ok, "dep add failed: {e}");

    // Remove dependency
    let (_o, e, ok) = run_bf(&ws, &["dep", "remove", &blocked, &blocker]);
    assert!(ok, "dep remove failed: {e}");

    let bead = find_bead(&ws, &blocked).expect("blocked bead must be in issues.jsonl");
    let empty_deps = vec![];
    let deps = bead
        .get("dependencies")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty_deps);

    assert!(
        !deps
            .iter()
            .any(|d| d.get("depends_on_id").and_then(|v| v.as_str()) == Some(blocker.as_str())),
        "dependency should be removed"
    );
}

#[test]
fn dep_add_no_autoflush_doesnt_modify_jsonl() {
    let (_t, ws) = setup();
    let blocker = create_bead(&ws, "Blocker");
    let blocked = create_bead(&ws, "Blocked");

    let before_json = fs::read_to_string(jsonl_path(&ws)).unwrap();

    let (_o, e, ok) = run_bf(
        &ws,
        &[
            "--no-auto-flush",
            "dep",
            "add",
            &blocker,
            "--blocks",
            &blocked,
        ],
    );
    assert!(ok, "dep add failed: {e}");

    let after_json = fs::read_to_string(jsonl_path(&ws)).unwrap();
    assert_eq!(before_json, after_json, "JSONL should not change");
}

// ==================== LABEL OPERATIONS ====================

#[test]
fn label_add_autoflush_adds_label() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Labeled bead");

    let (_o, e, ok) = run_bf(&ws, &["label", "add", &id, "--label", "urgent"]);
    assert!(ok, "label add failed: {e}");

    let bead = find_bead(&ws, &id).expect("bead must be in issues.jsonl");
    let labels = bead
        .get("labels")
        .and_then(|v| v.as_array())
        .expect("labels must be present");

    assert!(
        labels.iter().any(|l| l.as_str() == Some("urgent")),
        "label should be added"
    );
}

#[test]
fn label_add_multiple_autoflush() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Multi-label bead");

    let (_o, e, ok) = run_bf(
        &ws,
        &[
            "label",
            "add",
            &id,
            "--label",
            "urgent",
            "--label",
            "bug",
            "--label",
            "high-priority",
        ],
    );
    assert!(ok, "label add failed: {e}");

    let bead = find_bead(&ws, &id).expect("bead must be in issues.jsonl");
    let labels = bead
        .get("labels")
        .and_then(|v| v.as_array())
        .expect("labels must be present");

    assert_eq!(labels.len(), 3);
    assert!(labels.iter().any(|l| l.as_str() == Some("urgent")));
    assert!(labels.iter().any(|l| l.as_str() == Some("bug")));
    assert!(labels.iter().any(|l| l.as_str() == Some("high-priority")));
}

#[test]
fn label_remove_autoflush_removes_label() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Label removal test");

    // Add labels first
    let (_o, e, ok) = run_bf(
        &ws,
        &["label", "add", &id, "--label", "urgent", "--label", "bug"],
    );
    assert!(ok, "label add failed: {e}");

    // Remove one label
    let (_o, e, ok) = run_bf(&ws, &["label", "remove", &id, "--label", "urgent"]);
    assert!(ok, "label remove failed: {e}");

    let bead = find_bead(&ws, &id).expect("bead must be in issues.jsonl");
    let labels = bead
        .get("labels")
        .and_then(|v| v.as_array())
        .expect("labels must be present");

    assert!(
        !labels.iter().any(|l| l.as_str() == Some("urgent")),
        "removed label should not be present"
    );
    assert!(
        labels.iter().any(|l| l.as_str() == Some("bug")),
        "other labels should remain"
    );
}

#[test]
fn label_add_no_autoflush_doesnt_modify_jsonl() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "No-flush label");

    let before_json = fs::read_to_string(jsonl_path(&ws)).unwrap();

    let (_o, e, ok) = run_bf(
        &ws,
        &["--no-auto-flush", "label", "add", &id, "--label", "urgent"],
    );
    assert!(ok, "label add failed: {e}");

    let after_json = fs::read_to_string(jsonl_path(&ws)).unwrap();
    assert_eq!(before_json, after_json, "JSONL should not change");
}

// ==================== COMMENT OPERATIONS ====================

#[test]
fn comment_add_autoflush_adds_comment() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Commented bead");

    let (_o, e, ok) = run_bf(&ws, &["comments", "add", &id, "This", "is", "a", "comment"]);
    assert!(ok, "comments add failed: {e}");

    let bead = find_bead(&ws, &id).expect("bead must be in issues.jsonl");
    let comments = bead
        .get("comments")
        .and_then(|v| v.as_array())
        .expect("comments must be present");

    assert!(
        comments.iter().any(|c| {
            c.get("text")
                .and_then(|v| v.as_str())
                .map(|t| t.contains("This is a comment"))
                .unwrap_or(false)
        }),
        "comment should be added"
    );
}

#[test]
fn comment_add_multiple_autoflush() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Multi-comment bead");

    // Add first comment
    let (_o, e, ok) = run_bf(&ws, &["comments", "add", &id, "First", "comment"]);
    assert!(ok, "first comment add failed: {e}");

    // Add second comment
    let (_o, e, ok) = run_bf(&ws, &["comments", "add", &id, "Second", "comment"]);
    assert!(ok, "second comment add failed: {e}");

    let bead = find_bead(&ws, &id).expect("bead must be in issues.jsonl");
    let comments = bead
        .get("comments")
        .and_then(|v| v.as_array())
        .expect("comments must be present");

    assert_eq!(comments.len(), 2, "both comments should be present");
}

#[test]
fn comment_add_no_autoflush_doesnt_modify_jsonl() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "No-flush comment");

    let before_json = fs::read_to_string(jsonl_path(&ws)).unwrap();

    let (_o, e, ok) = run_bf(
        &ws,
        &["--no-auto-flush", "comments", "add", &id, "Test", "comment"],
    );
    assert!(ok, "comments add failed: {e}");

    let after_json = fs::read_to_string(jsonl_path(&ws)).unwrap();
    assert_eq!(before_json, after_json, "JSONL should not change");
}

// ==================== ANNOTATION OPERATIONS ====================

#[test]
fn annotate_set_autoflush_triggers_flush() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Annotated bead");

    // Delete issues.jsonl to detect when it's recreated
    fs::remove_file(jsonl_path(&ws)).unwrap();

    let (_o, e, ok) = run_bf(&ws, &["annotate", "set", &id, "env", "production"]);
    assert!(ok, "annotate set failed: {e}");

    // The flush should recreate issues.jsonl
    assert!(
        jsonl_path(&ws).exists(),
        "annotate set should trigger flush"
    );

    let bead = find_bead(&ws, &id).expect("annotated bead must be in issues.jsonl");
    let annotations = bead
        .get("annotations")
        .and_then(|v| v.as_object())
        .expect("annotations must be present");

    assert_eq!(
        annotations.get("env").and_then(|v| v.as_str()),
        Some("production")
    );
}

#[test]
fn annotate_set_multiple_autoflush() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Multi-annotation bead");

    fs::remove_file(jsonl_path(&ws)).unwrap();

    let (_o, e, ok) = run_bf(&ws, &["annotate", "set", &id, "env", "prod"]);
    assert!(ok, "first annotate set failed: {e}");

    let (_o, e, ok) = run_bf(&ws, &["annotate", "set", &id, "sprint", "sprint-7"]);
    assert!(ok, "second annotate set failed: {e}");

    let bead = find_bead(&ws, &id).expect("annotated bead must be in issues.jsonl");
    let annotations = bead
        .get("annotations")
        .and_then(|v| v.as_object())
        .expect("annotations must be present");

    assert_eq!(
        annotations.get("env").and_then(|v| v.as_str()),
        Some("prod")
    );
    assert_eq!(
        annotations.get("sprint").and_then(|v| v.as_str()),
        Some("sprint-7")
    );
}

#[test]
fn annotate_set_no_autoflush_doesnt_modify_jsonl() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "No-flush annotate");

    let before_json = fs::read_to_string(jsonl_path(&ws)).unwrap();

    let (_o, e, ok) = run_bf(
        &ws,
        &["--no-auto-flush", "annotate", "set", &id, "env", "test"],
    );
    assert!(ok, "annotate set failed: {e}");

    let after_json = fs::read_to_string(jsonl_path(&ws)).unwrap();
    assert_eq!(before_json, after_json, "JSONL should not change");
}

// ==================== FLUSH FAILURE CONTRACT ====================

/// Wedge the flush by making `issues.jsonl` a directory.
fn wedge_flush(ws: &Path) {
    let path = jsonl_path(ws);
    if path.exists() {
        fs::remove_file(&path).ok();
    }
    fs::create_dir(&path).unwrap();
}

fn unwedge_flush(ws: &Path) {
    let path = jsonl_path(ws);
    if path.is_dir() {
        fs::remove_dir(&path).ok();
    }
}

#[test]
fn create_flush_failure_succeeds_and_warns() {
    let (_t, ws) = setup();
    wedge_flush(&ws);

    let (out, err, ok) = run_bf(&ws, &["create", "--title", "Wedged create"]);
    assert!(ok, "create must not fail on flush error");

    let id = out.trim();
    assert!(
        !id.is_empty(),
        "create should return ID despite flush failure"
    );

    assert!(
        err.contains("warning:") && err.contains("auto-flush"),
        "stderr should contain flush warning"
    );

    // Clear the wedge and verify recovery works
    unwedge_flush(&ws);
    let (_o, e, ok) = run_bf(&ws, &["sync", "--flush-only"]);
    assert!(ok, "flush-only should recover: {e}");

    let bead = find_bead(&ws, &id).expect("bead should be recovered");
    assert_eq!(field(&bead, "title"), "Wedged create");
}

#[test]
fn update_flush_failure_succeeds_and_warns() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Wedged update");
    wedge_flush(&ws);

    let (_o, err, ok) = run_bf(&ws, &["update", &id, "--title", "Updated"]);
    assert!(ok, "update must not fail on flush error");

    assert!(
        err.contains("warning:") && err.contains("auto-flush"),
        "stderr should contain flush warning"
    );
}

#[test]
fn claim_flush_failure_succeeds_and_warns() {
    let (_t, ws) = setup();
    let _id = create_bead(&ws, "Wedged claim");
    wedge_flush(&ws);

    let (_o, err, ok) = run_bf(&ws, &["claim", "--assignee", "worker"]);
    assert!(ok, "claim must not fail on flush error");

    assert!(
        err.contains("warning:") && err.contains("auto-flush"),
        "stderr should contain flush warning"
    );
}

#[test]
fn close_flush_failure_succeeds_and_warns() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Wedged close");
    wedge_flush(&ws);

    let (_o, err, ok) = run_bf(&ws, &["close", &id, "--reason", "Done"]);
    assert!(ok, "close must not fail on flush error");

    assert!(
        err.contains("warning:") && err.contains("auto-flush"),
        "stderr should contain flush warning"
    );
}

#[test]
fn delete_flush_failure_succeeds_and_warns() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Wedged delete");
    wedge_flush(&ws);

    let (_o, err, ok) = run_bf(&ws, &["delete", &id]);
    assert!(ok, "delete must not fail on flush error");

    assert!(
        err.contains("warning:") && err.contains("auto-flush"),
        "stderr should contain flush warning"
    );
}

#[test]
fn json_output_with_flush_failure_contains_warning() {
    let (_t, ws) = setup();
    wedge_flush(&ws);

    let (out, err, ok) = run_bf(&ws, &["create", "--json", "--title", "JSON wedged"]);
    assert!(ok, "create must not fail on flush error");

    let parsed: Value = serde_json::from_str(out.trim()).expect("output should be valid JSON");

    // Check for either the id field or at least a warning (output may vary)
    let has_id = parsed.get("id").is_some();
    let has_warning = parsed.get("warning").is_some();

    assert!(
        has_id || has_warning,
        "JSON should contain either ID or warning field"
    );

    if let Some(warning) = parsed.get("warning").and_then(|v| v.as_str()) {
        assert!(
            warning.contains("auto-flush"),
            "warning should mention auto-flush"
        );
    }

    assert!(
        err.contains("warning:") && err.contains("auto-flush"),
        "stderr should also contain warning"
    );
}

// ==================== ISSUES.JSONL CORRECTNESS ====================

#[test]
fn issues_jsonl_maintains_valid_json_after_mutation() {
    let (_t, ws) = setup();

    // Create multiple beads
    let id1 = create_bead(&ws, "Bead 1");
    let id2 = create_bead(&ws, "Bead 2");
    let id3 = create_bead(&ws, "Bead 3");

    // Update one
    let (_o, e, ok) = run_bf(&ws, &["update", &id1, "--status", "in_progress"]);
    assert!(ok, "update failed: {e}");

    // Add labels to another
    let (_o, e, ok) = run_bf(&ws, &["label", "add", &id2, "--label", "urgent"]);
    assert!(ok, "label add failed: {e}");

    // Add comment to third
    let (_o, e, ok) = run_bf(&ws, &["comments", "add", &id3, "Test", "comment"]);
    assert!(ok, "comments add failed: {e}");

    // Verify all lines are valid JSON
    let content = fs::read_to_string(jsonl_path(&ws)).unwrap();
    for (i, line) in content.lines().enumerate() {
        if !line.trim().is_empty() {
            let _: Value = serde_json::from_str(line).unwrap_or_else(|e| {
                panic!("Line {} is not valid JSON: {}\nLine: {}", i + 1, e, line)
            });
        }
    }
}

#[test]
fn issues_jsonl_no_duplicate_lines_after_mutation() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Update test");

    // Perform multiple updates
    for i in 1..=5 {
        let (_o, e, ok) = run_bf(&ws, &["update", &id, "--title", &format!("Update {}", i)]);
        assert!(ok, "update {} failed: {e}", i);
    }

    // Count occurrences of the bead ID
    let content = fs::read_to_string(jsonl_path(&ws)).unwrap();
    let count = content
        .lines()
        .filter(|line| line.contains(&format!("\"id\":\"{}\"", id)))
        .count();

    assert_eq!(
        count, 1,
        "Bead should appear exactly once in JSONL, not {}",
        count
    );
}

#[test]
fn issues_jsonl_preserves_all_beads_after_partial_mutation() {
    let (_t, ws) = setup();

    // Create multiple beads
    let ids: Vec<String> = (1..=10)
        .map(|i| create_bead(&ws, &format!("Bead {}", i)))
        .collect();

    // Mutate only some
    let (_o, e, ok) = run_bf(&ws, &["update", &ids[2], "--status", "in_progress"]);
    assert!(ok, "update failed: {e}");

    let (_o, e, ok) = run_bf(&ws, &["close", &ids[5], "--reason", "Done"]);
    assert!(ok, "close failed: {e}");

    let (_o, e, ok) = run_bf(&ws, &["label", "add", &ids[8], "--label", "urgent"]);
    assert!(ok, "label add failed: {e}");

    // Verify all beads are still present
    let content = fs::read_to_string(jsonl_path(&ws)).unwrap();
    for id in &ids {
        assert!(
            content.contains(&format!("\"id\":\"{}\"", id)),
            "Bead {} should be present in JSONL",
            id
        );
    }
}

#[test]
fn issues_jsonl_newline_separated_after_multiple_mutations() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Newline test");

    // Perform various mutations
    let (_o, e, ok) = run_bf(&ws, &["update", &id, "--priority", "1"]);
    assert!(ok, "update failed: {e}");

    let (_o, e, ok) = run_bf(&ws, &["label", "add", &id, "--label", "test"]);
    assert!(ok, "label add failed: {e}");

    let (_o, e, ok) = run_bf(&ws, &["comments", "add", &id, "A", "comment"]);
    assert!(ok, "comments add failed: {e}");

    let content = fs::read_to_string(jsonl_path(&ws)).unwrap();

    // Each line should end with newline (except possibly the last)
    let lines: Vec<&str> = content.lines().collect();
    assert!(lines.len() >= 1, "Should have at least one line");

    // All non-empty lines should be parseable JSON
    for (i, line) in lines.iter().enumerate() {
        if !line.trim().is_empty() {
            let _: Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("Line {} is not valid JSON: {}", i + 1, e));
        }
    }
}
