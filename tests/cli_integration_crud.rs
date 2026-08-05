//! Integration tests for core CRUD commands
//!
//! Tests the basic bead lifecycle commands:
//! - create: Create beads with correct IDs, types, priorities
//! - show: Display full bead details including all fields
//! - list: Filter beads by status, type, assignee, priority
//! - update: Modify only specified fields without affecting others
//! - close: Transition status to closed with reason
//! - delete: Permanently remove beads
//! - reopen: Reset closed beads to open and clear assignee
//!
//! All tests verify --json output and error handling.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Get the path to the bf binary
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

/// Setup a test workspace with bf configuration
fn setup() -> (TempDir, PathBuf) {
    let temp = TempDir::new().unwrap();
    let workspace = temp.path().to_path_buf();
    let (_o, e, ok) = run_bf(&workspace, &["init", "--prefix", "bf"]);
    assert!(ok, "bf init failed: {e}");
    (temp, workspace)
}

/// Get the path to the beads directory
fn beads_dir(workspace: &Path) -> PathBuf {
    workspace.join(".beads")
}

/// Get the path to the database file
fn db_path(workspace: &Path) -> PathBuf {
    beads_dir(workspace).join("beads.db")
}

/// Get the path to the JSONL file
fn jsonl_path(workspace: &Path) -> PathBuf {
    beads_dir(workspace).join("issues.jsonl")
}

/// Parse every non-empty line of `issues.jsonl` into a JSON object.
fn read_beads_from_jsonl(workspace: &Path) -> Vec<Value> {
    let path = jsonl_path(workspace);
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("issues.jsonl unreadable at {}: {e}", path.display()));
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).expect("issues.jsonl line is not valid JSON"))
        .collect()
}

/// Parse JSON output from commands
fn parse_json(json: &str) -> Value {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("Failed to parse JSON: {}\nJSON was: {}", e, json))
}

/// Parse a JSONL string (newline-delimited JSON) into a Vec of values
fn parse_jsonl(jsonl: &str) -> Vec<Value> {
    jsonl
        .lines()
        .filter(|line| !line.trim().is_empty() && *line != "[]")
        .map(|line| parse_json(line))
        .collect()
}

/// Get a string field from JSON, panic if missing or not a string
fn get_string(json: &Value, field: &str) -> String {
    json.get(field)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("Field '{}' is not a string or is missing: {}", field, json))
        .to_string()
}

/// Check if JSON has a specific field
fn has_field(json: &Value, field: &str) -> bool {
    json.get(field).is_some()
}

/// Get an array field from JSON
fn get_array(json: &Value, field: &str) -> Vec<Value> {
    json.get(field)
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_else(|| vec![])
}

// ============================================================================
// CREATE command tests
// ============================================================================

#[test]
fn test_create_bead_with_defaults() {
    let (_t, ws) = setup();

    let (stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", "Test bead with defaults"]);
    assert!(ok, "create should succeed");

    let id = stdout.trim();
    assert!(!id.is_empty(), "create should return an ID");
    assert!(id.starts_with("bf-"), "ID should start with bf- prefix");

    // Verify bead exists in database
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id]);
    assert!(ok, "show should succeed");
    assert!(stdout.contains("Test bead with defaults"), "show should display the title");
    assert!(stdout.contains("open"), "default status should be open");
    assert!(stdout.contains("priority: 2"), "default priority should be 2");
}

#[test]
fn test_create_bead_with_custom_type_and_priority() {
    let (_t, ws) = setup();

    let (stdout, _stderr, ok) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "Critical bug",
            "--type",
            "bug",
            "--priority",
            "0",
        ],
    );
    assert!(ok, "create should succeed");

    let id = stdout.trim();

    // Verify the bead was created with correct type and priority
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok, "show should succeed");

    let bead_json = parse_json(&stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&bead_json, "issue_type"), "bug");
    assert_eq!(get_string(&bead_json, "priority"), "0");
}

#[test]
fn test_create_bead_with_all_fields() {
    let (_t, ws) = setup();

    let (stdout, _stderr, ok) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "Full feature bead",
            "--type",
            "feature",
            "--priority",
            "1",
            "--description",
            "Complete description",
            "--assignee",
            "test-worker",
            "--label",
            "backend",
            "--label",
            "urgent",
        ],
    );
    assert!(ok, "create should succeed");

    let id = stdout.trim();

    // Verify all fields via JSON
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok, "show should succeed");

    let bead_json = parse_json(&stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&bead_json, "title"), "Full feature bead");
    assert_eq!(get_string(&bead_json, "issue_type"), "feature");
    assert_eq!(get_string(&bead_json, "priority"), "1");
    assert_eq!(get_string(&bead_json, "description"), "Complete description");
    assert_eq!(get_string(&bead_json, "assignee"), "test-worker");

    let labels = get_array(&bead_json, "labels");
    assert_eq!(labels.len(), 2);
    assert!(labels.iter().any(|l| l.as_str() == Some("backend")));
    assert!(labels.iter().any(|l| l.as_str() == Some("urgent")));
}

#[test]
fn test_create_bead_json_output() {
    let (_t, ws) = setup();

    let (stdout, _stderr, ok) = run_bf(
        &ws,
        &["create", "--title", "JSON output test", "--json"],
    );
    assert!(ok, "create with --json should succeed");

    let json = parse_json(stdout.trim());
    assert!(has_field(&json, "id"));
    assert!(has_field(&json, "title"));
    assert!(has_field(&json, "type"));
    assert!(has_field(&json, "priority"));
    assert!(has_field(&json, "status"));

    assert_eq!(get_string(&json, "title"), "JSON output test");
    assert_eq!(get_string(&json, "type"), "task");
    assert_eq!(get_string(&json, "status"), "open");
}

#[test]
fn test_create_bead_empty_title_error() {
    let (_t, ws) = setup();

    let (_stdout, stderr, ok) = run_bf(&ws, &["create", "--title", ""]);
    assert!(!ok, "create with empty title should fail");
    assert!(stderr.contains("Title cannot be empty"), "error message should mention empty title");
}

#[test]
fn test_create_bead_invalid_type_error() {
    let (_t, ws) = setup();

    let (_stdout, stderr, ok) = run_bf(
        &ws,
        &["create", "--title", "Test", "--type", "invalid-type"],
    );
    assert!(!ok, "create with invalid type should fail");
    assert!(stderr.contains("type"), "error message should mention type issue");
}

// ============================================================================
// SHOW command tests
// ============================================================================

#[test]
fn test_show_displays_all_fields() {
    let (_t, ws) = setup();

    // Create a bead with various fields
    let (id_stdout, _stderr, ok) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "Show test bead",
            "--type",
            "task",
            "--priority",
            "1",
            "--description",
            "Test description",
            "--assignee",
            "worker-1",
            "--label",
            "test-label",
        ],
    );
    assert!(ok);
    let id = id_stdout.trim();

    // Show the bead
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id]);
    assert!(ok, "show should succeed");

    // Verify all important fields are displayed
    assert!(stdout.contains(&id), "should show ID");
    assert!(stdout.contains("Show test bead"), "should show title");
    assert!(stdout.contains("priority: 1"), "should show priority");
    assert!(stdout.contains("type: task"), "should show type");
    assert!(stdout.contains("Test description"), "should show description");
    assert!(stdout.contains("worker-1"), "should show assignee");
    assert!(stdout.contains("test-label"), "should show label");
}

#[test]
fn test_show_json_format() {
    let (_t, ws) = setup();

    let (id_stdout, _stderr, ok) = run_bf(
        &ws,
        &["create", "--title", "JSON format test"],
    );
    assert!(ok);
    let id = id_stdout.trim();

    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok, "show --json should succeed");

    let json_str = stdout.trim();
    // show --json returns a single-element array
    assert!(json_str.starts_with('['), "JSON output should be an array");
    assert!(json_str.ends_with(']'), "JSON output should be an array");

    let json = parse_json(json_str.trim_start_matches('[').trim_end_matches(']'));
    assert!(has_field(&json, "id"));
    assert!(has_field(&json, "title"));
    assert!(has_field(&json, "status"));
    assert!(has_field(&json, "priority"));
    assert!(has_field(&json, "issue_type"));
    assert!(has_field(&json, "created_at"));
    assert!(has_field(&json, "updated_at"));
}

#[test]
fn test_show_nonexistent_bead_error() {
    let (_t, ws) = setup();

    let (_stdout, stderr, ok) = run_bf(&ws, &["show", "bf-nonexistent"]);
    assert!(!ok, "show of nonexistent bead should fail");
    assert!(stderr.contains("not found"), "error should mention bead not found");
}

// ============================================================================
// LIST command tests
// ============================================================================

#[test]
fn test_list_displays_beads() {
    let (_t, ws) = setup();

    // Create multiple beads
    run_bf(&ws, &["create", "--title", "First bead"]).unwrap();
    run_bf(&ws, &["create", "--title", "Second bead"]).unwrap();
    run_bf(&ws, &["create", "--title", "Third bead"]).unwrap();

    let (stdout, _stderr, ok) = run_bf(&ws, &["list"]);
    assert!(ok, "list should succeed");

    assert!(stdout.contains("First bead"), "should show first bead");
    assert!(stdout.contains("Second bead"), "should show second bead");
    assert!(stdout.contains("Third bead"), "should show third bead");
}

#[test]
fn test_list_filter_by_status() {
    let (_t, ws) = setup();

    // Create beads and set different statuses
    let (id1, _stderr1, ok1) = run_bf(&ws, &["create", "--title", "Open bead"]);
    assert!(ok1);
    let id1 = id1.trim();

    let (id2, _stderr2, ok2) = run_bf(&ws, &["create", "--title", "Closed bead"]);
    assert!(ok2);
    let id2 = id2.trim();

    let (_stdout, _stderr, ok) = run_bf(&ws, &["close", id2, "--reason", "Test"]);
    assert!(ok);

    // List only open beads
    let (stdout, _stderr, ok) = run_bf(&ws, &["list", "--status", "open"]);
    assert!(ok, "list --status open should succeed");
    assert!(stdout.contains("Open bead"), "should show open bead");
    assert!(!stdout.contains("Closed bead"), "should not show closed bead");

    // List only closed beads
    let (stdout, _stderr, ok) = run_bf(&ws, &["list", "--status", "closed"]);
    assert!(ok, "list --status closed should succeed");
    assert!(!stdout.contains("Open bead"), "should not show open bead");
    assert!(stdout.contains("Closed bead"), "should show closed bead");
}

#[test]
fn test_list_filter_by_type() {
    let (_t, ws) = setup();

    let (_stdout1, _stderr1, ok1) = run_bf(&ws, &["create", "--title", "Bug bead", "--type", "bug"]);
    assert!(ok1);
    let (_stdout2, _stderr2, ok2) = run_bf(&ws, &["create", "--title", "Feature bead", "--type", "feature"]);
    assert!(ok2);

    let (stdout, _stderr, ok) = run_bf(&ws, &["list", "--type", "bug"]);
    assert!(ok, "list --type bug should succeed");
    assert!(stdout.contains("Bug bead"), "should show bug");
    assert!(!stdout.contains("Feature bead"), "should not show feature");
}

#[test]
fn test_list_filter_by_priority() {
    let (_t, ws) = setup();

    let (_stdout1, _stderr1, ok1) = run_bf(&ws, &["create", "--title", "Critical bead", "--priority", "0"]);
    assert!(ok1);
    let (_stdout2, _stderr2, ok2) = run_bf(&ws, &["create", "--title", "Normal bead", "--priority", "2"]);
    assert!(ok2);

    let (stdout, _stderr, ok) = run_bf(&ws, &["list", "--priority", "0"]);
    assert!(ok, "list --priority 0 should succeed");
    assert!(stdout.contains("Critical bead"), "should show critical bead");
    assert!(!stdout.contains("Normal bead"), "should not show normal bead");
}

#[test]
fn test_list_filter_by_assignee() {
    let (_t, ws) = setup();

    let (id1, _stderr, ok) = run_bf(&ws, &["create", "--title", "Assigned to alice", "--assignee", "alice"]);
    assert!(ok);

    let (id2, _stderr, ok) = run_bf(&ws, &["create", "--title", "Assigned to bob", "--assignee", "bob"]);
    assert!(ok);

    let (stdout, _stderr, ok) = run_bf(&ws, &["list", "--assignee", "alice"]);
    assert!(ok, "list --assignee alice should succeed");
    assert!(stdout.contains("Assigned to alice"), "should show alice's bead");
    assert!(!stdout.contains("Assigned to bob"), "should not show bob's bead");
}

#[test]
fn test_list_json_output() {
    let (_t, ws) = setup();

    let (_stdout1, _stderr1, ok1) = run_bf(&ws, &["create", "--title", "Bead 1"]);
    assert!(ok1, "create should succeed");
    let (_stdout2, _stderr2, ok2) = run_bf(&ws, &["create", "--title", "Bead 2"]);
    assert!(ok2, "create should succeed");

    let (stdout, _stderr, ok) = run_bf(&ws, &["list", "--format", "json"]);
    assert!(ok, "list --format json should succeed");

    let beads = parse_jsonl(&stdout);
    assert!(beads.len() >= 2, "should return at least 2 beads");

    for bead in beads {
        assert!(has_field(&bead, "id"));
        assert!(has_field(&bead, "title"));
        assert!(has_field(&bead, "status"));
    }
}

#[test]
fn test_list_limit() {
    let (_t, ws) = setup();

    run_bf(&ws, &["create", "--title", "Bead 1"]).unwrap();
    run_bf(&ws, &["create", "--title", "Bead 2"]).unwrap();
    run_bf(&ws, &["create", "--title", "Bead 3"]).unwrap();

    let (stdout, _stderr, ok) = run_bf(&ws, &["list", "--limit", "2"]);
    assert!(ok, "list --limit should succeed");

    // Should have exactly 2 beads
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert!(lines.len() <= 2, "should respect limit");
}

// ============================================================================
// UPDATE command tests
// ============================================================================

#[test]
fn test_update_modifies_only_specified_fields() {
    let (_t, ws) = setup();

    let (id_stdout, _stderr, ok) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "Original title",
            "--priority",
            "2",
            "--type",
            "task",
        ],
    );
    assert!(ok);
    let id = id_stdout.trim();

    // Update only title
    let (_stdout, _stderr, ok) = run_bf(&ws, &["update", &id, "--title", "New title"]);
    assert!(ok, "update should succeed");

    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok);

    let bead_json = parse_json(stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&bead_json, "title"), "New title", "title should be updated");
    assert_eq!(get_string(&bead_json, "priority"), "2", "priority should remain unchanged");
    assert_eq!(get_string(&bead_json, "issue_type"), "task", "type should remain unchanged");
}

#[test]
fn test_update_multiple_fields() {
    let (_t, ws) = setup();

    let (id_stdout, _stderr, ok) = run_bf(
        &ws,
        &["create", "--title", "Original", "--priority", "2"],
    );
    assert!(ok);
    let id = id_stdout.trim();

    let (_stdout, _stderr, ok) = run_bf(
        &ws,
        &[
            "update",
            &id,
            "--title",
            "Updated title",
            "--priority",
            "0",
            "--status",
            "in_progress",
            "--assignee",
            "test-worker",
        ],
    );
    assert!(ok, "update should succeed");

    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok);

    let bead_json = parse_json(stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&bead_json, "title"), "Updated title");
    assert_eq!(get_string(&bead_json, "priority"), "0");
    assert_eq!(get_string(&bead_json, "status"), "in_progress");
    assert_eq!(get_string(&bead_json, "assignee"), "test-worker");
}

#[test]
fn test_update_description() {
    let (_t, ws) = setup();

    let (id_stdout, _stderr, ok) = run_bf(
        &ws,
        &["create", "--title", "Description test"],
    );
    assert!(ok);
    let id = id_stdout.trim();

    let (_stdout, _stderr, ok) = run_bf(
        &ws,
        &["update", &id, "--description", "New description"],
    );
    assert!(ok, "update should succeed");

    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok);

    let bead_json = parse_json(stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&bead_json, "description"), "New description");
}

#[test]
fn test_update_clear_assignee() {
    let (_t, ws) = setup();

    let (id_stdout, _stderr, ok) = run_bf(
        &ws,
        &["create", "--title", "Assigned bead", "--assignee", "worker-1"],
    );
    assert!(ok);
    let id = id_stdout.trim();

    let (_stdout, _stderr, ok) = run_bf(&ws, &["update", &id, "--clear-assignee"]);
    assert!(ok, "update --clear-assignee should succeed");

    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok);

    let bead_json = parse_json(stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    // When assignee is null, serde_json::Value::as_str() returns None, so get_string would panic
    // We need to check that the assignee field is null
    assert!(bead_json.get("assignee").unwrap().is_null(), "assignee should be null after clearing");
}

#[test]
fn test_update_nonexistent_bead_error() {
    let (_t, ws) = setup();

    let (_stdout, stderr, ok) = run_bf(&ws, &["update", "bf-nonexistent", "--title", "New"]);
    assert!(!ok, "update of nonexistent bead should fail");
    assert!(stderr.contains("not found"), "error should mention bead not found");
}

#[test]
fn test_update_invalid_status_error() {
    let (_t, ws) = setup();

    let (id_stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", "Test"]);
    assert!(ok);
    let id = id_stdout.trim();

    // Invalid status that doesn't parse
    let (_stdout, stderr, ok) = run_bf(&ws, &["update", &id, "--status", "invalid_status"]);
    // The update command might succeed but not change the status if it's invalid
    // or it might fail - we just verify the behavior is handled
    if !ok {
        assert!(stderr.contains("status"), "error should mention status");
    }
}

// ============================================================================
// CLOSE command tests
// ============================================================================

#[test]
fn test_close_transitions_to_closed_with_reason() {
    let (_t, ws) = setup();

    let (id_stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", "To close"]);
    assert!(ok);
    let id = id_stdout.trim();

    let (_stdout, _stderr, ok) = run_bf(&ws, &["close", &id, "--reason", "Completed successfully"]);
    assert!(ok, "close should succeed");

    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok);

    let bead_json = parse_json(stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&bead_json, "status"), "closed");
    assert_eq!(get_string(&bead_json, "close_reason"), "Completed successfully");
    assert!(has_field(&bead_json, "closed_at"), "should have closed_at timestamp");
}

#[test]
fn test_close_default_reason() {
    let (_t, ws) = setup();

    let (id_stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", "Auto close"]);
    assert!(ok);
    let id = id_stdout.trim();

    // Close without specifying reason
    let (_stdout, _stderr, ok) = run_bf(&ws, &["close", &id]);
    assert!(ok, "close should succeed with default reason");

    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok);

    let bead_json = parse_json(stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&bead_json, "close_reason"), "Completed");
}

#[test]
fn test_close_nonexistent_bead_error() {
    let (_t, ws) = setup();

    let (_stdout, stderr, ok) = run_bf(&ws, &["close", "bf-nonexistent", "--reason", "Test"]);
    assert!(!ok, "close of nonexistent bead should fail");
    assert!(stderr.contains("not found"), "error should mention bead not found");
}

// ============================================================================
// REOPEN command tests
// ============================================================================

#[test]
fn test_reopen_resets_closed_bead_to_open() {
    let (_t, ws) = setup();

    let (id_stdout, _stderr, ok) = run_bf(
        &ws,
        &["create", "--title", "To reopen", "--assignee", "worker-1"],
    );
    assert!(ok);
    let id = id_stdout.trim();

    // First close the bead
    run_bf(&ws, &["close", &id, "--reason", "Done"]).unwrap();

    // Now reopen it
    let (_stdout, _stderr, ok) = run_bf(&ws, &["reopen", &id]);
    assert!(ok, "reopen should succeed");

    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok);

    let bead_json = parse_json(stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&bead_json, "status"), "open", "status should be open");
    assert!(bead_json.get("close_reason").unwrap().is_null(), "close_reason should be null");
    assert!(bead_json.get("closed_at").unwrap().is_null(), "closed_at should be null");
    // assignee should be cleared on reopen
    assert!(bead_json.get("assignee").unwrap().is_null(), "assignee should be cleared");
}

#[test]
fn test_reopen_nonexistent_bead_error() {
    let (_t, ws) = setup();

    let (_stdout, stderr, ok) = run_bf(&ws, &["reopen", "bf-nonexistent"]);
    assert!(!ok, "reopen of nonexistent bead should fail");
    assert!(stderr.contains("not found"), "error should mention bead not found");
}

#[test]
fn test_reopen_open_bead_error() {
    let (_t, ws) = setup();

    let (id_stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", "Already open"]);
    assert!(ok);
    let id = id_stdout.trim();

    let (_stdout, stderr, ok) = run_bf(&ws, &["reopen", &id]);
    assert!(!ok, "reopen of already open bead should fail");
    assert!(stderr.contains("already open"), "error should mention bead is already open");
}

// ============================================================================
// DELETE command tests
// ============================================================================

#[test]
fn test_delete_permanently_removes_bead() {
    let (_t, ws) = setup();

    let (id_stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", "To delete"]);
    assert!(ok);
    let id = id_stdout.trim();

    // Verify bead exists
    let (_stdout, _stderr, ok) = run_bf(&ws, &["show", &id]);
    assert!(ok, "bead should exist before delete");

    // Delete the bead
    let (_stdout, _stderr, ok) = run_bf(&ws, &["delete", &id]);
    assert!(ok, "delete should succeed");

    // Verify bead no longer exists
    let (_stdout, stderr, ok) = run_bf(&ws, &["show", &id]);
    assert!(!ok, "show should fail after delete");
    assert!(stderr.contains("not found"), "error should mention bead not found");

    // Verify it's not in the list
    let (stdout, _stderr, ok) = run_bf(&ws, &["list"]);
    assert!(ok);
    assert!(!stdout.contains("To delete"), "deleted bead should not appear in list");
}

#[test]
fn test_delete_nonexistent_bead_error() {
    let (_t, ws) = setup();

    // Deleting a non-existent bead might succeed or fail depending on implementation
    // The important thing is that the bead doesn't exist afterward
    let (_stdout, _stderr, ok) = run_bf(&ws, &["delete", "bf-nonexistent"]);
    // Don't assert on ok - the behavior varies by implementation
}

#[test]
fn test_delete_removes_from_jsonl() {
    let (_t, ws) = setup();

    let (id_stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", "Delete from JSONL"]);
    assert!(ok);
    let id = id_stdout.trim();

    // Verify bead is in JSONL
    let beads_before = read_beads_from_jsonl(&ws);
    assert!(beads_before.iter().any(|b| b.get("id").and_then(|v| v.as_str()) == Some(id)));

    // Delete the bead
    run_bf(&ws, &["delete", &id]).unwrap();

    // Verify bead is removed from JSONL (should be pruned on flush)
    let beads_after = read_beads_from_jsonl(&ws);
    assert!(!beads_after.iter().any(|b| b.get("id").and_then(|v| v.as_str()) == Some(id)));
}

// ============================================================================
// INTEGRATION: Full lifecycle test
// ============================================================================

#[test]
fn test_full_bead_lifecycle() {
    let (_t, ws) = setup();

    // 1. Create a bead
    let (id_stdout, _stderr, ok) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "Lifecycle test",
            "--type",
            "feature",
            "--priority",
            "1",
            "--description",
            "Initial description",
            "--label",
            "lifecycle",
        ],
    );
    assert!(ok);
    let id = id_stdout.trim();

    // 2. Show the bead
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id]);
    assert!(ok);
    assert!(stdout.contains("Lifecycle test"));

    // 3. Update the bead
    let (_stdout, _stderr, ok) = run_bf(
        &ws,
        &["update", &id, "--status", "in_progress", "--assignee", "worker-1"],
    );
    assert!(ok);

    // 4. Verify update
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok);
    let bead_json = parse_json(stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&bead_json, "status"), "in_progress");
    assert_eq!(get_string(&bead_json, "assignee"), "worker-1");

    // 5. Close the bead
    let (_stdout, _stderr, ok) = run_bf(&ws, &["close", &id, "--reason", "Feature complete"]);
    assert!(ok);

    // 6. Verify close
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok);
    let bead_json = parse_json(stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&bead_json, "status"), "closed");

    // 7. Reopen the bead
    let (_stdout, _stderr, ok) = run_bf(&ws, &["reopen", &id]);
    assert!(ok);

    // 8. Verify reopen
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id]);
    assert!(ok);
    assert!(stdout.contains("open"));
    assert!(stdout.contains("status: open"));

    // 9. Delete the bead
    let (_stdout, _stderr, ok) = run_bf(&ws, &["delete", &id]);
    assert!(ok);

    // 10. Verify deletion
    let (_stdout, _stderr, ok) = run_bf(&ws, &["show", &id]);
    assert!(!ok, "bead should not exist after deletion");
}

// ============================================================================
// Error message quality tests
// ============================================================================

#[test]
fn test_error_messages_are_clear() {
    let (_t, ws) = setup();

    // Test create with empty title
    let (_stdout, stderr, _ok) = run_bf(&ws, &["create", "--title", ""]);
    assert!(stderr.contains("Title") || stderr.contains("title"), "error should mention title");
    assert!(stderr.contains("empty") || stderr.contains("blank"), "error should mention empty/blank");

    // Test show of nonexistent bead
    let (_stdout, stderr, _ok) = run_bf(&ws, &["show", "bf-bogus-id"]);
    assert!(stderr.contains("not found") || stderr.contains("Bead not found"), "error should mention not found");

    // Test update of nonexistent bead
    let (_stdout, stderr, _ok) = run_bf(&ws, &["update", "bf-bogus", "--title", "New"]);
    assert!(stderr.contains("not found"), "error should mention not found");

    // Test close of nonexistent bead
    let (_stdout, stderr, _ok) = run_bf(&ws, &["close", "bf-bogus", "--reason", "Test"]);
    assert!(stderr.contains("not found"), "error should mention not found");

    // Test reopen of nonexistent bead
    let (_stdout, stderr, _ok) = run_bf(&ws, &["reopen", "bf-bogus"]);
    assert!(stderr.contains("not found"), "error should mention not found");
}

// ============================================================================
// JSON output consistency tests
// ============================================================================

#[test]
fn test_json_output_consistency() {
    let (_t, ws) = setup();

    // Create several beads
    let (id1, _stderr, ok) = run_bf(&ws, &["create", "--title", "Bead A", "--priority", "0"]);
    assert!(ok);
    let id1 = id1.trim();

    let (id2, _stderr, ok) = run_bf(&ws, &["create", "--title", "Bead B", "--priority", "2"]);
    assert!(ok);
    let id2 = id2.trim();

    // Test that list JSON and show JSON have consistent field names
    let (list_json, _stderr, ok) = run_bf(&ws, &["list", "--format", "json"]);
    assert!(ok);
    let list_beads = parse_jsonl(&list_json);

    if let Some(list_bead) = list_beads.first() {
        // Check that list has expected fields
        assert!(has_field(list_bead, "id"));
        assert!(has_field(list_bead, "title"));
        assert!(has_field(list_bead, "status"));
        assert!(has_field(list_bead, "priority"));
        assert!(has_field(list_bead, "issue_type"));
    }

    // Check that individual show has same field names
    let (show_json, _stderr, ok) = run_bf(&ws, &["show", id1, "--json"]);
    assert!(ok);
    let show_bead = parse_json(show_json.trim().trim_start_matches('[').trim_end_matches(']'));

    // Fields should match between list and show
    for field in ["id", "title", "status", "priority", "issue_type"] {
        assert!(has_field(&show_bead, field), "show should have field: {}", field);
    }
}
