//! End-to-end tests for auto-flush wiring on single-issue mutations
//! (Phase 7.1 child 2/5, bf-3iosi).
//!
//! Every single-issue mutation handler (`create`, `update`, `close`, `reopen`,
//! `dep add/remove`, `label add/remove`, `comments add`, `annotate set/…`) runs
//! a best-effort SQLite→JSONL flush after its storage write commits, honoring
//! the effective auto-flush switch (`config.sync.auto_flush && !--no-auto-flush`)
//! and surfacing a flush failure through the warning channel WITHOUT failing the
//! mutation. These tests drive the real `bf` binary against a tempdir workspace
//! and assert on `issues.jsonl` (the flush target), matching the CLI-spawn shape
//! already used by `comments_cli.rs`.

use serde_json::Value;
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
fn read_beads(workspace: &Path) -> Vec<Value> {
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
/// write it. This is the core assertion shared by every "auto-flush on" test:
/// the changed bead must be present immediately after the mutation returns.
fn find_bead(workspace: &Path, id: &str) -> Value {
    read_beads(workspace)
        .into_iter()
        .find(|b| b.get("id").and_then(|v| v.as_str()) == Some(id))
        .unwrap_or_else(|| panic!("bead {id} not found in issues.jsonl after mutation"))
}

fn field<'a>(bead: &'a Value, key: &str) -> &'a str {
    bead.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("bead missing string field '{key}': {bead}"))
}

// --- Auto-flush ON (the default): each mutation lands in issues.jsonl. ---

#[test]
fn create_flushes_new_bead() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Created bead");
    let bead = find_bead(&ws, &id);
    assert_eq!(field(&bead, "title"), "Created bead");
    assert_eq!(field(&bead, "status"), "open");
}

#[test]
fn update_flushes_changed_status() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "To update");
    let (_o, e, ok) = run_bf(&ws, &["update", &id, "--status", "in_progress"]);
    assert!(ok, "update failed: {e}");
    assert_eq!(field(&find_bead(&ws, &id), "status"), "in_progress");
}

#[test]
fn close_flushes_closed_status_and_reason() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "To close");
    let (_o, e, ok) = run_bf(&ws, &["close", &id, "--reason", "all done"]);
    assert!(ok, "close failed: {e}");
    let bead = find_bead(&ws, &id);
    assert_eq!(field(&bead, "status"), "closed");
    assert_eq!(field(&bead, "close_reason"), "all done");
}

#[test]
fn reopen_flushes_reopened_status() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "To reopen");
    let (_o, e, ok) = run_bf(&ws, &["close", &id, "--reason", "done"]);
    assert!(ok, "close failed: {e}");
    let (_o, e, ok) = run_bf(&ws, &["reopen", &id]);
    assert!(ok, "reopen failed: {e}");
    assert_eq!(field(&find_bead(&ws, &id), "status"), "open");
}

#[test]
fn dep_add_flushes_dependency() {
    let (_t, ws) = setup();
    let blocker = create_bead(&ws, "Blocker");
    let blocked = create_bead(&ws, "Blocked");
    // `dep add <blocker> --blocks <blocked>`: the blocked bead gains a dependency
    // on the blocker, so its flushed line carries the depends_on edge.
    let (_o, e, ok) = run_bf(&ws, &["dep", "add", &blocker, "--blocks", &blocked]);
    assert!(ok, "dep add failed: {e}");
    let bead = find_bead(&ws, &blocked);
    let deps = bead
        .get("dependencies")
        .and_then(|v| v.as_array())
        .expect("dependencies array missing");
    assert!(
        deps.iter()
            .any(|d| d.get("depends_on_id").and_then(|v| v.as_str()) == Some(blocker.as_str())),
        "blocked bead does not depend on blocker in issues.jsonl: {bead}"
    );
}

#[test]
fn label_add_flushes_label() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "To label");
    let (_o, e, ok) = run_bf(&ws, &["label", "add", &id, "--label", "urgent"]);
    assert!(ok, "label add failed: {e}");
    let bead = find_bead(&ws, &id);
    let labels = bead
        .get("labels")
        .and_then(|v| v.as_array())
        .expect("labels array missing");
    assert!(
        labels.iter().any(|l| l.as_str() == Some("urgent")),
        "label 'urgent' not flushed to issues.jsonl: {bead}"
    );
}

#[test]
fn comment_add_flushes_comment() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "To comment");
    let (_o, e, ok) = run_bf(&ws, &["comments", "add", &id, "a", "flushed", "comment"]);
    assert!(ok, "comments add failed: {e}");
    let bead = find_bead(&ws, &id);
    let comments = bead
        .get("comments")
        .and_then(|v| v.as_array())
        .expect("comments array missing");
    assert!(
        comments.iter().any(|c| c
            .get("text")
            .and_then(|v| v.as_str())
            .map(|t| t.contains("a flushed comment"))
            .unwrap_or(false)),
        "comment body not flushed to issues.jsonl: {bead}"
    );
}

#[test]
fn annotate_set_triggers_flush() {
    // Annotations are not serialized as a top-level JSONL field, so we assert
    // the flush *happened* rather than its content: create clears the dirty
    // marks and writes issues.jsonl, we delete it, and `annotate set` must
    // re-flush the (now-dirty-again) bead, recreating the file.
    let (_t, ws) = setup();
    let id = create_bead(&ws, "To annotate");
    fs::remove_file(jsonl_path(&ws)).unwrap();
    let (_o, e, ok) = run_bf(&ws, &["annotate", "set", &id, "env", "prod"]);
    assert!(ok, "annotate set failed: {e}");
    assert!(
        jsonl_path(&ws).exists(),
        "annotate set did not re-flush issues.jsonl"
    );
    // The re-flushed file must contain the annotated bead.
    find_bead(&ws, &id);
}

#[test]
fn claim_flushes_status_and_assignee() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "To claim");
    // Claim the bead - this changes status to in_progress and sets assignee
    let (_o, e, ok) = run_bf(&ws, &["claim", "--assignee", "test-worker"]);
    assert!(ok, "claim failed: {e}");
    let bead = find_bead(&ws, &id);
    assert_eq!(field(&bead, "status"), "in_progress");
    assert_eq!(field(&bead, "assignee"), "test-worker");
}

#[test]
fn claim_with_any_flag_flushes_claimed_bead() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "To claim with any");
    // Claim with --any flag - should still flush to the workspace where bead was claimed
    let (_o, e, ok) = run_bf(&ws, &["claim", "--any", "--assignee", "another-worker"]);
    assert!(ok, "claim --any failed: {e}");
    let bead = find_bead(&ws, &id);
    assert_eq!(field(&bead, "status"), "in_progress");
    assert_eq!(field(&bead, "assignee"), "another-worker");
}

#[test]
fn claim_with_no_auto_flush_leaves_jsonl_untouched() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Bead to claim without flush");
    // First create and flush to get the bead into the system
    let _bead = find_bead(&ws, &id);
    // Delete issues.jsonl to test that claim doesn't recreate it
    fs::remove_file(jsonl_path(&ws)).unwrap();

    // Claim with --no-auto-flush - should not write issues.jsonl
    let (_o, e, ok) = run_bf(
        &ws,
        &["--no-auto-flush", "claim", "--assignee", "worker-no-flush"],
    );
    assert!(ok, "claim with --no-auto-flush failed: {e}");
    assert!(
        !jsonl_path(&ws).exists(),
        "--no-auto-flush claim must not write issues.jsonl"
    );
}

#[test]
fn claim_with_config_auto_flush_disabled_leaves_jsonl_untouched() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "Bead to claim with config disabled");
    // First create and flush to get the bead into the system
    let _bead = find_bead(&ws, &id);
    // Delete issues.jsonl to test that claim doesn't recreate it
    fs::remove_file(jsonl_path(&ws)).unwrap();

    // Persist the config master switch off
    let cfg = ws.join(".beads").join("config.yaml");
    let mut content = fs::read_to_string(&cfg).unwrap();
    content.push_str("sync:\n  auto_flush: false\n");
    fs::write(&cfg, content).unwrap();

    // Claim with config auto_flush disabled - should not write issues.jsonl
    let (_o, e, ok) = run_bf(&ws, &["claim", "--assignee", "worker-config-off"]);
    assert!(ok, "claim with auto_flush=false config failed: {e}");
    assert!(
        !jsonl_path(&ws).exists(),
        "claim with sync.auto_flush:false must not write issues.jsonl"
    );
}

#[test]
fn reclaim_flushes_reclaimed_status() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "To reclaim");
    // First claim the bead
    let (_o, e, ok) = run_bf(&ws, &["claim", "--assignee", "original-worker"]);
    assert!(ok, "initial claim failed: {e}");

    // Create another bead that's open, so there's something to claim
    let open_id = create_bead(&ws, "Open bead");

    // Claim a second bead - both claimed beads should be flushed
    let (_o, e, ok) = run_bf(&ws, &["claim", "--assignee", "new-worker"]);
    assert!(ok, "second claim failed: {e}");

    // Both beads should be flushed to issues.jsonl
    let first_bead = find_bead(&ws, &id);
    assert_eq!(
        field(&first_bead, "status"),
        "in_progress",
        "first bead should be in_progress"
    );
    assert_eq!(
        field(&first_bead, "assignee"),
        "original-worker",
        "first bead should have original assignee"
    );

    let second_bead = find_bead(&ws, &open_id);
    assert_eq!(
        field(&second_bead, "status"),
        "in_progress",
        "second bead should be in_progress"
    );
    assert_eq!(
        field(&second_bead, "assignee"),
        "new-worker",
        "second bead should have assignee"
    );
}

#[test]
fn claim_flush_failure_warns_and_retains_dirty() {
    let (_t, ws) = setup();
    let id = create_bead(&ws, "To claim with wedged flush");
    // First flush normally to make sure the bead is in the database
    let _bead = find_bead(&ws, &id);

    // Wedge the flush by making issues.jsonl a directory
    wedge_flush(&ws);

    // Claim should succeed (exit 0) despite the wedged flush
    let (_o, err, ok) = run_bf(&ws, &["claim", "--assignee", "worker-wedge"]);
    assert!(ok, "claim must not fail on a flush error");
    assert!(
        err.contains("warning:") && err.contains("auto-flush"),
        "expected an auto-flush warning on stderr, got: {err}"
    );

    // Clear the wedge and verify dirty mark is retained
    fs::remove_dir(jsonl_path(&ws)).unwrap();
    let (_o, e, ok) = run_bf(&ws, &["sync", "--flush-only"]);
    assert!(ok, "sync --flush-only failed: {e}");

    // The claimed bead should now appear in issues.jsonl
    let bead = find_bead(&ws, &id);
    assert_eq!(field(&bead, "status"), "in_progress");
    assert_eq!(field(&bead, "assignee"), "worker-wedge");
}

// --- Auto-flush OFF: the switch suppresses the flush. ---

#[test]
fn no_auto_flush_flag_leaves_jsonl_untouched() {
    let (_t, ws) = setup();
    // Create with the flag: no flush, so no issues.jsonl is produced.
    let (out, e, ok) = run_bf(&ws, &["--no-auto-flush", "create", "--title", "No flush"]);
    assert!(ok, "create failed: {e}");
    let id = out.trim().to_string();
    assert!(
        !jsonl_path(&ws).exists(),
        "--no-auto-flush create must not write issues.jsonl"
    );
    // A subsequent flagged mutation must likewise leave it absent.
    let (_o, e, ok) = run_bf(
        &ws,
        &["--no-auto-flush", "update", &id, "--status", "in_progress"],
    );
    assert!(ok, "update failed: {e}");
    assert!(
        !jsonl_path(&ws).exists(),
        "--no-auto-flush update must not write issues.jsonl"
    );
}

#[test]
fn config_auto_flush_false_leaves_jsonl_untouched() {
    let (_t, ws) = setup();
    // Persist the config master switch off; no CLI flag needed.
    let cfg = ws.join(".beads").join("config.yaml");
    let mut content = fs::read_to_string(&cfg).unwrap();
    content.push_str("sync:\n  auto_flush: false\n");
    fs::write(&cfg, content).unwrap();

    let (_o, e, ok) = run_bf(&ws, &["create", "--title", "Config off"]);
    assert!(ok, "create failed: {e}");
    assert!(
        !jsonl_path(&ws).exists(),
        "sync.auto_flush:false must not write issues.jsonl"
    );
}

// --- Flush failure is non-fatal and surfaces a warning. ---

/// Wedge the flush by making `issues.jsonl` a directory: the atomic temp+rename
/// export cannot overwrite it, so the flush fails while the mutation itself
/// already committed.
fn wedge_flush(ws: &Path) {
    let path = jsonl_path(ws);
    if path.exists() {
        fs::remove_file(&path).ok();
    }
    fs::create_dir(&path).unwrap();
}

#[test]
fn flush_failure_nonfatal_json_warning_and_dirty_retained() {
    let (_t, ws) = setup();
    wedge_flush(&ws);

    // Mutation succeeds (exit 0) despite the wedged flush; --json carries a
    // top-level "warning" alongside the "data" envelope.
    let (out, _e, ok) = run_bf(&ws, &["create", "--json", "--title", "Wedged"]);
    assert!(ok, "create must not fail on a flush error");
    let parsed: Value =
        serde_json::from_str(out.trim()).expect("create --json emitted invalid JSON");
    let id = parsed
        .get("data")
        .and_then(|d| d.get("id"))
        .and_then(|v| v.as_str())
        .expect("create --json missing data.id")
        .to_string();
    assert!(
        parsed
            .get("warning")
            .and_then(|v| v.as_str())
            .map(|w| w.contains("auto-flush"))
            .unwrap_or(false),
        "create --json must carry an auto-flush warning: {parsed}"
    );

    // The dirty mark is retained: once the wedge is cleared, an explicit
    // flush-only sync recovers the bead into issues.jsonl.
    fs::remove_dir(jsonl_path(&ws)).unwrap();
    let (_o, e, ok) = run_bf(&ws, &["sync", "--flush-only"]);
    assert!(ok, "sync --flush-only failed: {e}");
    assert_eq!(
        field(&find_bead(&ws, &id), "title"),
        "Wedged",
        "retained-dirty bead did not recover on the next flush"
    );
}

#[test]
fn flush_failure_warns_on_stderr_for_human_output() {
    let (_t, ws) = setup();
    wedge_flush(&ws);

    // The human (non-JSON) path prints the warning to stderr and still exits 0.
    let (_o, err, ok) = run_bf(&ws, &["create", "--title", "Wedged human"]);
    assert!(ok, "create must not fail on a flush error");
    assert!(
        err.contains("warning:") && err.contains("auto-flush"),
        "expected an auto-flush warning on stderr, got: {err}"
    );
}
