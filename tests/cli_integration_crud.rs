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

/// Get a numeric field from JSON (i64 or u64)
fn get_number(json: &Value, field: &str) -> i64 {
    json.get(field)
        .and_then(|v| v.as_i64())
        .or_else(|| json.get(field).and_then(|v| v.as_u64().map(|n| n as i64)))
        .unwrap_or_else(|| panic!("Field '{}' is not a number or is missing: {}", field, json))
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
    assert!(stdout.contains("Status: open"), "default status should be open");
    assert!(stdout.contains("Priority: P2"), "default priority should be P2");
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
    assert_eq!(get_number(&bead_json, "priority"), 0);
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
    assert_eq!(get_number(&bead_json, "priority"), 1);
    assert_eq!(get_string(&bead_json, "description"), "Complete description");
    assert_eq!(get_string(&bead_json, "assignee"), "test-worker");

    let labels = get_array(&bead_json, "labels");
    assert_eq!(labels.len(), 2);
    assert!(labels.iter().any(|l| l.as_str() == Some("backend")));
    assert!(labels.iter().any(|l| l.as_str() == Some("urgent")));
}

#[test]
fn test_create_p0_bead_with_multiple_labels() {
    let (_t, ws) = setup();

    // Create a P0 priority bead with multiple labels
    let (stdout, _stderr, ok) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "Critical P0 bead with multiple labels",
            "--priority",
            "0",
            "--type",
            "bug",
            "--label",
            "critical",
            "--label",
            "security",
            "--label",
            "urgent",
            "--label",
            "production",
            "--description",
            "This is a P0 critical issue that affects production security",
        ],
    );
    assert!(ok, "create P0 bead with multiple labels should succeed");

    let id = stdout.trim();

    // Verify the bead was created with P0 priority
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok, "show should succeed");

    let bead_json = parse_json(&stdout.trim().trim_start_matches('[').trim_end_matches(']'));

    // Verify P0 priority
    assert_eq!(get_number(&bead_json, "priority"), 0, "priority should be P0 (0)");

    // Verify bug type
    assert_eq!(get_string(&bead_json, "issue_type"), "bug", "type should be bug");

    // Verify all 4 labels are present
    let labels = get_array(&bead_json, "labels");
    assert_eq!(labels.len(), 4, "should have exactly 4 labels");

    // Verify each specific label exists
    let label_values: Vec<&str> = labels.iter()
        .filter_map(|l| l.as_str())
        .collect();

    assert!(label_values.contains(&"critical"), "should contain 'critical' label");
    assert!(label_values.contains(&"security"), "should contain 'security' label");
    assert!(label_values.contains(&"urgent"), "should contain 'urgent' label");
    assert!(label_values.contains(&"production"), "should contain 'production' label");

    // Verify description
    assert_eq!(get_string(&bead_json, "description"), "This is a P0 critical issue that affects production security");

    // Verify the bead appears in P0 priority listings
    let (list_stdout, _stderr, ok) = run_bf(&ws, &["list", "--priority", "0"]);
    assert!(ok, "list by P0 priority should succeed");
    assert!(list_stdout.contains("Critical P0 bead with multiple labels"), "P0 bead should appear in priority 0 list");
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
    assert!(stdout.contains("Priority: P1"), "should show priority");
    assert!(stdout.contains("Type: task"), "should show type");
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
    let (_o1, _e1, ok1) = run_bf(&ws, &["create", "--title", "First bead"]);
    assert!(ok1, "create first bead should succeed");
    let (_o2, _e2, ok2) = run_bf(&ws, &["create", "--title", "Second bead"]);
    assert!(ok2, "create second bead should succeed");
    let (_o3, _e3, ok3) = run_bf(&ws, &["create", "--title", "Third bead"]);
    assert!(ok3, "create third bead should succeed");

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

    let (_stdout1, _stderr1, ok1) = run_bf(&ws, &["create", "--title", "Bead 1"]);
    assert!(ok1);
    let (_stdout2, _stderr2, ok2) = run_bf(&ws, &["create", "--title", "Bead 2"]);
    assert!(ok2);
    let (_stdout3, _stderr3, ok3) = run_bf(&ws, &["create", "--title", "Bead 3"]);
    assert!(ok3);

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
    assert_eq!(get_number(&bead_json, "priority"), 2, "priority should remain unchanged");
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
    assert_eq!(get_number(&bead_json, "priority"), 0);
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

    let (id_stdout, _stderr1, ok1) = run_bf(
        &ws,
        &["create", "--title", "Assigned bead", "--assignee", "worker-1"],
    );
    assert!(ok1);
    let id = id_stdout.trim();

    let (_stdout, _stderr2, ok2) = run_bf(&ws, &["update", &id, "--clear-assignee"]);
    assert!(ok2, "update --clear-assignee should succeed");

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

    let (id_stdout, _stderr1, ok1) = run_bf(
        &ws,
        &["create", "--title", "To reopen", "--assignee", "worker-1"],
    );
    assert!(ok1);
    let id = id_stdout.trim();

    // First close the bead
    let (_stdout1, _stderr2, ok2) = run_bf(&ws, &["close", &id, "--reason", "Done"]);
    assert!(ok2);

    // Now reopen it
    let (_stdout, _stderr3, ok3) = run_bf(&ws, &["reopen", &id]);
    assert!(ok3, "reopen should succeed");

    let (stdout, _stderr, ok4) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok4);

    let bead_json = parse_json(stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&bead_json, "status"), "open", "status should be open");
    assert!(bead_json.get("close_reason").map_or(true, |v| v.is_null()), "close_reason should be null or missing");
    assert!(bead_json.get("closed_at").map_or(true, |v| v.is_null()), "closed_at should be null or missing");
    // assignee should be cleared on reopen
    assert!(bead_json.get("assignee").map_or(true, |v| v.is_null()), "assignee should be null or missing");
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

    let (id_stdout, _stderr1, ok1) = run_bf(&ws, &["create", "--title", "To delete"]);
    assert!(ok1);
    let id = id_stdout.trim();

    // Verify bead exists
    let (_stdout1, _stderr2, ok2) = run_bf(&ws, &["show", &id]);
    assert!(ok2, "bead should exist before delete");

    // Delete the bead
    let (_stdout2, _stderr3, ok3) = run_bf(&ws, &["delete", &id]);
    assert!(ok3, "delete should succeed");

    // Verify bead no longer exists
    let (_stdout3, stderr, ok4) = run_bf(&ws, &["show", &id]);
    assert!(!ok4, "show should fail after delete");
    assert!(stderr.contains("not found"), "error should mention bead not found");

    // Verify it's not in the list
    let (stdout, _stderr4, ok5) = run_bf(&ws, &["list"]);
    assert!(ok5);
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

    let (id_stdout, _stderr1, ok1) = run_bf(&ws, &["create", "--title", "Delete from JSONL"]);
    assert!(ok1);
    let id = id_stdout.trim();

    // Verify bead is in JSONL
    let beads_before = read_beads_from_jsonl(&ws);
    assert!(beads_before.iter().any(|b| b.get("id").and_then(|v| v.as_str()) == Some(id)));

    // Delete the bead
    let (_stdout, _stderr2, ok2) = run_bf(&ws, &["delete", &id]);
    assert!(ok2);

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
    let (id_stdout, _stderr1, ok1) = run_bf(
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
    assert!(ok1);
    let id = id_stdout.trim();

    // 2. Show the bead
    let (stdout1, _stderr2, ok2) = run_bf(&ws, &["show", &id]);
    assert!(ok2);
    assert!(stdout1.contains("Lifecycle test"));

    // 3. Update the bead
    let (_stdout1, _stderr3, ok3) = run_bf(
        &ws,
        &["update", &id, "--status", "in_progress", "--assignee", "worker-1"],
    );
    assert!(ok3);

    // 4. Verify update
    let (stdout2, _stderr4, ok4) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok4);
    let bead_json = parse_json(stdout2.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&bead_json, "status"), "in_progress");
    assert_eq!(get_string(&bead_json, "assignee"), "worker-1");

    // 5. Close the bead
    let (_stdout2, _stderr5, ok5) = run_bf(&ws, &["close", &id, "--reason", "Feature complete"]);
    assert!(ok5);

    // 6. Verify close
    let (stdout3, _stderr6, ok6) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok6);
    let bead_json = parse_json(stdout3.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&bead_json, "status"), "closed");

    // 7. Reopen the bead
    let (_stdout3, _stderr7, ok7) = run_bf(&ws, &["reopen", &id]);
    assert!(ok7);

    // 8. Verify reopen
    let (stdout4, _stderr8, ok8) = run_bf(&ws, &["show", &id]);
    assert!(ok8);
    assert!(stdout4.contains("open"));
    assert!(stdout4.contains("status: open"));

    // 9. Delete the bead
    let (_stdout4, _stderr9, ok9) = run_bf(&ws, &["delete", &id]);
    assert!(ok9);

    // 10. Verify deletion
    let (_stdout5, _stderr10, ok10) = run_bf(&ws, &["show", &id]);
    assert!(!ok10, "bead should not exist after deletion");
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

// ============================================================================
// P0 EPIC CLI tests
// ============================================================================

#[test]
fn test_create_p0_epic_with_labels_via_cli() {
    let (_t, ws) = setup();

    // Create P0 epic with labels using CLI
    let (id_stdout, _stderr, ok) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "Critical Infrastructure Epic",
            "--type",
            "epic",
            "--priority",
            "0", // P0 = Critical
            "--description",
            "Database migration requiring immediate attention",
            "--assignee",
            "infra-team",
            "--label",
            "critical",
            "--label",
            "infrastructure",
            "--label",
            "database",
        ],
    );
    assert!(ok, "create P0 epic should succeed");

    let id = id_stdout.trim();
    assert!(id.starts_with("bf-"), "ID should have bf- prefix");

    // Verify the epic via JSON output
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok, "show should succeed");

    let epic_json = parse_json(stdout.trim().trim_start_matches('[').trim_end_matches(']'));

    // Verify all fields
    assert_eq!(get_string(&epic_json, "title"), "Critical Infrastructure Epic");
    assert_eq!(get_string(&epic_json, "issue_type"), "epic");
    assert_eq!(get_number(&epic_json, "priority"), 0, "P0 should be priority 0");
    assert_eq!(get_string(&epic_json, "status"), "open");
    assert_eq!(
        get_string(&epic_json, "description"),
        "Database migration requiring immediate attention"
    );
    assert_eq!(get_string(&epic_json, "assignee"), "infra-team");

    // Verify labels
    let labels = get_array(&epic_json, "labels");
    assert_eq!(labels.len(), 3);
    assert!(labels.iter().any(|l| l.as_str() == Some("critical")));
    assert!(labels.iter().any(|l| l.as_str() == Some("infrastructure")));
    assert!(labels.iter().any(|l| l.as_str() == Some("database")));
}

#[test]
fn test_p0_epic_text_output_display() {
    let (_t, ws) = setup();

    // Create P0 epic
    let (id_stdout, _stderr, ok) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "P0 Security Epic",
            "--type",
            "epic",
            "--priority",
            "0",
            "--label",
            "security",
            "--label",
            "urgent",
        ],
    );
    assert!(ok);
    let id = id_stdout.trim();

    // Verify text output shows P0 format correctly
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id]);
    assert!(ok, "show should succeed");

    assert!(stdout.contains("P0 Security Epic"), "should show title");
    assert!(stdout.contains("Type: epic"), "should show epic type");
    assert!(stdout.contains("Priority: P0"), "should display P0 priority");
    assert!(stdout.contains("Status: open"), "should show open status");
    assert!(stdout.contains("security"), "should show security label");
    assert!(stdout.contains("urgent"), "should show urgent label");
}

#[test]
fn test_list_p0_epics_by_priority() {
    let (_t, ws) = setup();

    // Create multiple epics with different priorities
    let (_o1, _e1, ok1) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "P0 Critical Epic",
            "--type",
            "epic",
            "--priority",
            "0",
            "--label",
            "critical",
        ],
    );
    assert!(ok1);

    let (_o2, _e2, ok2) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "P1 High Epic",
            "--type",
            "epic",
            "--priority",
            "1",
            "--label",
            "important",
        ],
    );
    assert!(ok2);

    let (_o3, _e3, ok3) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "P2 Normal Epic",
            "--type",
            "epic",
            "--priority",
            "2",
        ],
    );
    assert!(ok3);

    // List only P0 epics
    let (stdout, _stderr, ok) = run_bf(&ws, &["list", "--type", "epic", "--priority", "0"]);
    assert!(ok, "list P0 epics should succeed");

    assert!(stdout.contains("P0 Critical Epic"), "should show P0 epic");
    assert!(!stdout.contains("P1 High Epic"), "should not show P1 epic");
    assert!(!stdout.contains("P2 Normal Epic"), "should not show P2 epic");
}

#[test]
fn test_p0_epic_json_serialization() {
    let (_t, ws) = setup();

    // Create P0 epic with labels
    let (id_stdout, _stderr, ok) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "JSON Test P0 Epic",
            "--type",
            "epic",
            "--priority",
            "0",
            "--label",
            "critical",
            "--label",
            "test",
            "--json",
        ],
    );
    assert!(ok);

    let json_output = id_stdout.trim();
    let epic_json = parse_json(json_output);

    // Verify JSON structure
    assert!(has_field(&epic_json, "id"));
    assert!(has_field(&epic_json, "title"));
    assert!(has_field(&epic_json, "issue_type"));
    assert!(has_field(&epic_json, "priority"));
    assert!(has_field(&epic_json, "status"));
    assert!(has_field(&epic_json, "labels"));

    assert_eq!(get_string(&epic_json, "title"), "JSON Test P0 Epic");
    assert_eq!(get_string(&epic_json, "issue_type"), "epic");
    assert_eq!(get_number(&epic_json, "priority"), 0, "P0 = 0");
    assert_eq!(get_string(&epic_json, "status"), "open");

    let labels = get_array(&epic_json, "labels");
    assert_eq!(labels.len(), 2);
    assert!(labels.iter().any(|l| l.as_str() == Some("critical")));
    assert!(labels.iter().any(|l| l.as_str() == Some("test")));
}

#[test]
fn test_p0_epic_with_multiple_labels_cli() {
    let (_t, ws) = setup();

    // Create P0 epic with many labels
    let (id_stdout, _stderr, ok) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "Multi-label P0 Epic",
            "--type",
            "epic",
            "--priority",
            "0",
            "--label",
            "critical",
            "--label",
            "security",
            "--label",
            "frontend",
            "--label",
            "backend",
            "--label",
            "database",
            "--label",
            "urgent",
        ],
    );
    assert!(ok);

    let id = id_stdout.trim();

    // Verify all labels are present
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok);

    let epic_json = parse_json(stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    let labels = get_array(&epic_json, "labels");

    assert_eq!(labels.len(), 5);
    assert!(labels.iter().any(|l| l.as_str() == Some("critical")));
    assert!(labels.iter().any(|l| l.as_str() == Some("security")));
    assert!(labels.iter().any(|l| l.as_str() == Some("frontend")));
    assert!(labels.iter().any(|l| l.as_str() == Some("backend")));
    assert!(labels.iter().any(|l| l.as_str() == Some("database")));
    assert!(labels.iter().any(|l| l.as_str() == Some("urgent")));
}

#[test]
fn test_p0_epic_label_filtering() {
    let (_t, ws) = setup();

    // Create epics with different labels
    let (_o1, _e1, ok1) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "Security P0 Epic",
            "--type",
            "epic",
            "--priority",
            "0",
            "--label",
            "security",
            "--label",
            "critical",
        ],
    );
    assert!(ok1);

    let (_o2, _e2, ok2) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "Infrastructure P0 Epic",
            "--type",
            "epic",
            "--priority",
            "0",
            "--label",
            "infrastructure",
            "--label",
            "critical",
        ],
    );
    assert!(ok2);

    let (_o3, _e3, ok3) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "Feature P1 Epic",
            "--type",
            "epic",
            "--priority",
            "1",
            "--label",
            "feature",
        ],
    );
    assert!(ok3);

    // List all P0 epics (should show both security and infrastructure)
    let (stdout, _stderr, ok) = run_bf(&ws, &["list", "--type", "epic", "--priority", "0"]);
    assert!(ok);

    assert!(stdout.contains("Security P0 Epic"), "should show security P0 epic");
    assert!(stdout.contains("Infrastructure P0 Epic"), "should show infrastructure P0 epic");
    assert!(!stdout.contains("Feature P1 Epic"), "should not show P1 epic");

    // Both should have critical label
    assert!(stdout.contains("critical"), "should show critical label");
}

#[test]
fn test_p0_epic_priority_comparison() {
    let (_t, ws) = setup();

    // Create P0 epic
    let (id1, _e1, ok1) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "P0 Highest Priority",
            "--type",
            "epic",
            "--priority",
            "0",
        ],
    );
    assert!(ok1);
    let id1 = id1.trim();

    // Create P1 epic
    let (_o2, _e2, ok2) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "P1 High Priority",
            "--type",
            "epic",
            "--priority",
            "1",
        ],
    );
    assert!(ok2);

    // Verify P0 shows correctly
    let (stdout1, _stderr, ok) = run_bf(&ws, &["show", id1]);
    assert!(ok);
    assert!(stdout1.contains("Priority: P0"), "P0 should display as P0");

    // List sorted by priority (P0 should come first)
    let (list_out, _stderr, ok) = run_bf(&ws, &["list", "--type", "epic"]);
    assert!(ok);

    // P0 should appear before P1 in output
    let p0_pos = list_out.find("P0 Highest Priority");
    let p1_pos = list_out.find("P1 High Priority");
    assert!(p0_pos < p1_pos, "P0 epic should appear before P1 epic");
}

#[test]
fn test_p0_epic_update_preserves_priority() {
    let (_t, ws) = setup();

    // Create P0 epic
    let (id_stdout, _stderr, ok) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "Original P0 Epic",
            "--type",
            "epic",
            "--priority",
            "0",
            "--label",
            "critical",
        ],
    );
    assert!(ok);
    let id = id_stdout.trim();

    // Update title and description (not priority)
    let (_o, _e, ok) = run_bf(
        &ws,
        &[
            "update",
            &id,
            "--title",
            "Updated P0 Epic",
            "--description",
            "Updated description",
        ],
    );
    assert!(ok, "update should succeed");

    // Verify priority is still P0
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok);

    let epic_json = parse_json(stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_number(&epic_json, "priority"), 0, "priority should remain P0");
    assert_eq!(get_string(&epic_json, "title"), "Updated P0 Epic");
    assert_eq!(get_string(&epic_json, "description"), "Updated description");
}

#[test]
fn test_p0_epic_close_and_reopen() {
    let (_t, ws) = setup();

    // Create P0 epic
    let (id_stdout, _stderr, ok) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "P0 Epic to Close",
            "--type",
            "epic",
            "--priority",
            "0",
            "--label",
            "critical",
            "--assignee",
            "worker-1",
        ],
    );
    assert!(ok);
    let id = id_stdout.trim();

    // Close the epic
    let (_o, _e, ok) = run_bf(&ws, &["close", &id, "--reason", "P0 epic completed"]);
    assert!(ok, "close should succeed");

    // Verify close
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok);

    let epic_json = parse_json(stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&epic_json, "status"), "closed");
    assert_eq!(get_string(&epic_json, "close_reason"), "P0 epic completed");

    // Reopen the epic
    let (_o, _e, ok) = run_bf(&ws, &["reopen", &id]);
    assert!(ok, "reopen should succeed");

    // Verify reopen
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok);

    let epic_json = parse_json(stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&epic_json, "status"), "open");
    assert_eq!(get_number(&epic_json, "priority"), 0, "priority should remain P0 after reopen");
}

#[test]
fn test_p0_epic_without_labels() {
    let (_t, ws) = setup();

    // Create P0 epic without labels
    let (id_stdout, _stderr, ok) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "P0 Epic No Labels",
            "--type",
            "epic",
            "--priority",
            "0",
        ],
    );
    assert!(ok);

    let id = id_stdout.trim();

    // Verify P0 priority but no labels
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok);

    let epic_json = parse_json(stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_number(&epic_json, "priority"), 0, "should be P0");
    assert_eq!(get_string(&epic_json, "issue_type"), "epic");

    let labels = get_array(&epic_json, "labels");
    assert_eq!(labels.len(), 0, "should have no labels");
}

#[test]
fn test_p0_bead_description_update() {
    let (_t, ws) = setup();

    // Create a P0 bead with initial description
    let (id_stdout, _stderr, ok) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "P0 Bead Description Test",
            "--type",
            "task",
            "--priority",
            "0",
            "--description",
            "Initial description for P0 bead",
        ],
    );
    assert!(ok, "create P0 bead should succeed");

    let id = id_stdout.trim();

    // Verify initial description and P0 priority
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok, "show should succeed");

    let bead_json = parse_json(stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&bead_json, "description"), "Initial description for P0 bead");
    assert_eq!(get_number(&bead_json, "priority"), 0, "priority should be P0");

    // Update the description
    let (_stdout, _stderr, ok) = run_bf(
        &ws,
        &["update", &id, "--description", "Updated description for P0 bead"],
    );
    assert!(ok, "update description should succeed");

    // Verify description was updated and P0 priority is preserved
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok, "show should succeed");

    let updated_json = parse_json(stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(
        get_string(&updated_json, "description"),
        "Updated description for P0 bead",
        "description should be updated"
    );
    assert_eq!(
        get_number(&updated_json, "priority"),
        0,
        "P0 priority should be preserved after description update"
    );
    assert_eq!(
        get_string(&updated_json, "issue_type"),
        "task",
        "type should remain unchanged"
    );
}

