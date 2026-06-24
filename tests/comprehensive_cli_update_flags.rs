//! Comprehensive CLI integration tests for ALL `bf update` command flags
//!
//! This test file validates that every single update flag works correctly via CLI:
//! --title, --status, --priority, --assignee, --description, --acceptance-criteria,
//! --notes, --design, --due-at
//!
//! It also tests edge cases, error scenarios, and combinations.

use std::process::Command;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper function to get the bf binary path
fn bf_path() -> PathBuf {
    std::env::var("CARGO_BIN_EXE_bf")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./target/debug/bf"))
}

/// Helper function to initialize a test workspace
fn init_test_workspace() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();

    let bf = bf_path();
    let init_result = Command::new(&bf)
        .arg("init")
        .arg("--prefix")
        .arg("test")
        .current_dir(workspace)
        .output()
        .expect("Failed to initialize workspace");

    assert!(init_result.status.success(), "bf init failed: {}",
            String::from_utf8_lossy(&init_result.stderr));

    temp_dir
}

/// Helper function to create a test bead
fn create_test_bead(workspace: impl AsRef<std::path::Path>, title: &str) -> String {
    let bf = bf_path();
    let create_result = Command::new(&bf)
        .arg("create")
        .arg("--title")
        .arg(title)
        .arg("--type")
        .arg("task")
        .arg("--priority")
        .arg("2")
        .current_dir(workspace)
        .output()
        .expect("Failed to create bead");

    assert!(create_result.status.success(), "bf create failed: {}",
            String::from_utf8_lossy(&create_result.stderr));

    String::from_utf8(create_result.stdout).unwrap().trim().to_string()
}

/// Helper function to update a bead and verify success
fn update_bead(workspace: impl AsRef<std::path::Path>, bead_id: &str, args: &[&str]) {
    let bf = bf_path();
    let mut cmd = Command::new(&bf);
    cmd.arg("update")
       .arg(bead_id)
       .args(args)
       .current_dir(workspace);

    let result = cmd.output().expect("Failed to run update");
    assert!(result.status.success(), "bf update failed: {}",
            String::from_utf8_lossy(&result.stderr));
}

/// Helper function to get bead details as JSON
fn get_bead_json(workspace: impl AsRef<std::path::Path>, bead_id: &str) -> serde_json::Value {
    let bf = bf_path();
    let show_result = Command::new(&bf)
        .arg("show")
        .arg(bead_id)
        .arg("--format")
        .arg("json")
        .current_dir(workspace)
        .output()
        .expect("Failed to show bead");

    assert!(show_result.status.success(), "bf show failed: {}",
            String::from_utf8_lossy(&show_result.stderr));

    let output = String::from_utf8(show_result.stdout).unwrap();
    let beads: Vec<serde_json::Value> = serde_json::from_str(&output)
        .expect("Failed to parse JSON");
    beads.into_iter().next().expect("No bead found")
}

// ==================== TITLE FLAG TESTS ====================

#[test]
fn test_cli_update_title_flag_basic() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Original Title");

    update_bead(workspace, &bead_id, &["--title", "Updated Title"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["title"], "Updated Title");
}

#[test]
fn test_cli_update_title_flag_with_special_characters() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Simple Title");

    let special_title = "Title with émojis 🎉 and spëcial çharacters! @#$%";
    update_bead(workspace, &bead_id, &["--title", special_title]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["title"], special_title);
}

#[test]
fn test_cli_update_title_flag_empty() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Original Title");

    update_bead(workspace, &bead_id, &["--title", ""]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["title"], "");
}

// ==================== STATUS FLAG TESTS ====================

#[test]
fn test_cli_update_status_flag_to_open() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Status");

    // First set to blocked
    update_bead(workspace, &bead_id, &["--status", "blocked"]);

    // Then update to open
    update_bead(workspace, &bead_id, &["--status", "open"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["status"], "open");
}

#[test]
fn test_cli_update_status_flag_to_in_progress() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Status");

    update_bead(workspace, &bead_id, &["--status", "in_progress"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["status"], "in_progress");
}

#[test]
fn test_cli_update_status_flag_to_blocked() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Status");

    update_bead(workspace, &bead_id, &["--status", "blocked"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["status"], "blocked");
}

#[test]
fn test_cli_update_status_flag_to_deferred() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Status");

    update_bead(workspace, &bead_id, &["--status", "deferred"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["status"], "deferred");
}

#[test]
fn test_cli_update_status_flag_invalid() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Status");

    let bf = bf_path();
    let result = Command::new(&bf)
        .arg("update")
        .arg(&bead_id)
        .arg("--status")
        .arg("invalid_status")
        .current_dir(workspace)
        .output()
        .expect("Failed to run update");

    // CLI accepts invalid status values (stored as-is in TEXT field)
    // The status is stored as a string without validation at the CLI level
    assert!(result.status.success(), "bf update should succeed even with invalid status");

    // Verify the invalid status was actually stored
    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["status"], "invalid_status");
}

// ==================== PRIORITY FLAG TESTS ====================

#[test]
fn test_cli_update_priority_flag_critical() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Priority");

    update_bead(workspace, &bead_id, &["--priority", "0"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["priority"], 0);
}

#[test]
fn test_cli_update_priority_flag_high() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Priority");

    update_bead(workspace, &bead_id, &["--priority", "1"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["priority"], 1);
}

#[test]
fn test_cli_update_priority_flag_medium() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Priority");

    update_bead(workspace, &bead_id, &["--priority", "2"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["priority"], 2);
}

#[test]
fn test_cli_update_priority_flag_low() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Priority");

    update_bead(workspace, &bead_id, &["--priority", "3"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["priority"], 3);
}

#[test]
fn test_cli_update_priority_flag_backlog() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Priority");

    update_bead(workspace, &bead_id, &["--priority", "4"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["priority"], 4);
}

// ==================== ASSIGNEE FLAG TESTS ====================

#[test]
fn test_cli_update_assignee_flag_basic() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Assignee");

    update_bead(workspace, &bead_id, &["--assignee", "worker-1"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["assignee"], "worker-1");
}

#[test]
fn test_cli_update_assignee_flag_reassignment() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Assignee");

    update_bead(workspace, &bead_id, &["--assignee", "worker-1"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["assignee"], "worker-1");

    update_bead(workspace, &bead_id, &["--assignee", "worker-2"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["assignee"], "worker-2");
}

#[test]
fn test_cli_update_assignee_flag_clear() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Assignee");

    update_bead(workspace, &bead_id, &["--assignee", "worker-1"]);

    update_bead(workspace, &bead_id, &["--assignee", ""]);

    let bead = get_bead_json(workspace, &bead_id);
    // Empty string behavior - verify it's cleared
    assert!(bead["assignee"].as_str().map_or(false, |s| s.is_empty()));
}

// ==================== DESCRIPTION FLAG TESTS ====================

#[test]
fn test_cli_update_description_flag_basic() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Description");

    update_bead(workspace, &bead_id, &["--description", "New description"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["description"], "New description");
}

#[test]
fn test_cli_update_description_flag_multiline() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Description");

    let multiline = "Line 1\nLine 2\nLine 3";
    update_bead(workspace, &bead_id, &["--description", multiline]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["description"], multiline);
}

#[test]
fn test_cli_update_description_flag_unicode() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Description");

    let unicode = "Description with émojis 🎉 and spëcial çharacters";
    update_bead(workspace, &bead_id, &["--description", unicode]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["description"], unicode);
}

// ==================== ACCEPTANCE CRITERIA FLAG TESTS ====================

#[test]
fn test_cli_update_acceptance_criteria_flag_basic() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test AC");

    update_bead(workspace, &bead_id, &["--acceptance-criteria", "Must pass all tests"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["acceptance_criteria"], "Must pass all tests");
}

#[test]
fn test_cli_update_acceptance_criteria_flag_multiline() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test AC");

    let multiline = "AC 1: Feature works\nAC 2: Tests pass\nAC 3: Docs updated";
    update_bead(workspace, &bead_id, &["--acceptance-criteria", multiline]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["acceptance_criteria"], multiline);
}

// ==================== NOTES FLAG TESTS ====================

#[test]
fn test_cli_update_notes_flag_basic() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Notes");

    update_bead(workspace, &bead_id, &["--notes", "Implementation notes"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["notes"], "Implementation notes");
}

#[test]
fn test_cli_update_notes_flag_multiline() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Notes");

    let multiline = "Note 1: First step\nNote 2: Second step\nNote 3: Third step";
    update_bead(workspace, &bead_id, &["--notes", multiline]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["notes"], multiline);
}

// ==================== DESIGN FLAG TESTS ====================

#[test]
fn test_cli_update_design_flag_basic() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Design");

    update_bead(workspace, &bead_id, &["--design", "Technical design approach"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["design"], "Technical design approach");
}

#[test]
fn test_cli_update_design_flag_multiline() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Design");

    let multiline = "Design:\n1. Use X library\n2. Implement Y pattern\n3. Test with Z";
    update_bead(workspace, &bead_id, &["--design", multiline]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["design"], multiline);
}

// ==================== DUE_AT FLAG TESTS ====================

#[test]
fn test_cli_update_due_at_flag_rfc3339() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Due");

    update_bead(workspace, &bead_id, &["--due-at", "2025-12-31T23:59:59Z"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert!(bead["due_at"].is_string());
    let due_str = bead["due_at"].as_str().unwrap();
    assert!(due_str.starts_with("2025-12-31"));
}

#[test]
fn test_cli_update_due_at_flag_invalid_format() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Due");

    let bf = bf_path();
    let result = Command::new(&bf)
        .arg("update")
        .arg(&bead_id)
        .arg("--due-at")
        .arg("invalid-date-format")
        .current_dir(workspace)
        .output()
        .expect("Failed to run update");

    // Should fail with invalid date format
    assert!(!result.status.success(), "bf update should fail with invalid date format");
}

// ==================== COMBINATION UPDATE TESTS ====================

#[test]
fn test_cli_update_all_flags_together() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test All Flags");

    update_bead(workspace, &bead_id, &[
        "--title", "Completely Updated Title",
        "--status", "in_progress",
        "--priority", "1",
        "--assignee", "super-worker",
        "--description", "Updated description",
        "--acceptance-criteria", "Updated AC",
        "--notes", "Updated notes",
        "--design", "Updated design",
        "--due-at", "2025-12-31T23:59:59Z"
    ]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["title"], "Completely Updated Title");
    assert_eq!(bead["status"], "in_progress");
    assert_eq!(bead["priority"], 1);
    assert_eq!(bead["assignee"], "super-worker");
    assert_eq!(bead["description"], "Updated description");
    assert_eq!(bead["acceptance_criteria"], "Updated AC");
    assert_eq!(bead["notes"], "Updated notes");
    assert_eq!(bead["design"], "Updated design");
    assert!(bead["due_at"].is_string());
}

#[test]
fn test_cli_update_preserves_unspecified_fields() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Preserve");

    // Set all fields first
    update_bead(workspace, &bead_id, &[
        "--status", "in_progress",
        "--priority", "1",
        "--assignee", "worker",
        "--description", "Description",
        "--acceptance-criteria", "AC",
        "--notes", "Notes",
        "--design", "Design",
        "--due-at", "2025-01-01T00:00:00Z"
    ]);

    // Update only title
    update_bead(workspace, &bead_id, &["--title", "New Title Only"]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["title"], "New Title Only");
    // All other fields should be preserved
    assert_eq!(bead["status"], "in_progress");
    assert_eq!(bead["priority"], 1);
    assert_eq!(bead["assignee"], "worker");
    assert_eq!(bead["description"], "Description");
    assert_eq!(bead["acceptance_criteria"], "AC");
    assert_eq!(bead["notes"], "Notes");
    assert_eq!(bead["design"], "Design");
    assert!(bead["due_at"].is_string());
}

#[test]
fn test_cli_update_status_priority_combination() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test Combo");

    update_bead(workspace, &bead_id, &[
        "--status", "in_progress",
        "--priority", "0"
    ]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["status"], "in_progress");
    assert_eq!(bead["priority"], 0);
}

#[test]
fn test_cli_update_title_assignee_combination() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Old Title");

    update_bead(workspace, &bead_id, &[
        "--title", "New Title with Assignee",
        "--assignee", "new-worker"
    ]);

    let bead = get_bead_json(workspace, &bead_id);
    assert_eq!(bead["title"], "New Title with Assignee");
    assert_eq!(bead["assignee"], "new-worker");
}

// ==================== ERROR SCENARIO TESTS ====================

#[test]
fn test_cli_update_nonexistent_bead() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();

    let bf = bf_path();
    let result = Command::new(&bf)
        .arg("update")
        .arg("test-nonexistent")
        .arg("--title")
        .arg("New Title")
        .current_dir(workspace)
        .output()
        .expect("Failed to run update");

    // Should fail with non-existent bead
    assert!(!result.status.success(), "bf update should fail with non-existent bead");
}

#[test]
fn test_cli_update_without_changes() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_test_bead(workspace, "Test No Changes");

    // Update with no actual changes (just the bead ID)
    let bf = bf_path();
    let result = Command::new(&bf)
        .arg("update")
        .arg(&bead_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run update");

    // Should still succeed (no-op is allowed)
    assert!(result.status.success(), "bf update with no changes should succeed");
}