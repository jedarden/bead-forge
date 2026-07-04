//! Integration tests for assignee field validation
//!
//! This test file validates that:
//! 1. Empty assignee is rejected during bead creation
//! 2. Whitespace-only assignee is rejected during bead creation
//! 3. Empty assignee is rejected during bead updates
//! 4. Whitespace-only assignee is rejected during bead updates
//! 5. Valid assignees are accepted during both create and update operations
//! 6. None (no assignee) is accepted during both create and update operations

use std::process::Command;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a temporary workspace for testing
fn setup_test_workspace() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("test-workspace");
    std::fs::create_dir_all(&workspace_dir).unwrap();

    // Initialize bead-forge workspace
    let output = Command::new("bf")
        .arg("init")
        .arg("--prefix")
        .arg("bf")
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf init");

    if !output.status.success() {
        panic!("bf init failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    temp_dir
}

/// Run bf create command and return the result
fn run_bf_create(workspace_dir: &PathBuf, title: &str, assignee: Option<&str>) -> (bool, String, String) {
    let mut cmd = Command::new("bf");
    cmd.arg("create")
        .arg("--title")
        .arg(title)
        .current_dir(workspace_dir);

    if let Some(assignee_val) = assignee {
        cmd.arg("--assignee").arg(assignee_val);
    }

    let output = cmd.output().expect("Failed to run bf create");
    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (success, stdout, stderr)
}

/// Run bf update command and return the result
fn run_bf_update(workspace_dir: &PathBuf, id: &str, assignee: Option<&str>) -> (bool, String, String) {
    let mut cmd = Command::new("bf");
    cmd.arg("update")
        .arg(id)
        .current_dir(workspace_dir);

    if let Some(assignee_val) = assignee {
        cmd.arg("--assignee").arg(assignee_val);
    }

    let output = cmd.output().expect("Failed to run bf update");
    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (success, stdout, stderr)
}

#[test]
fn test_create_bead_with_empty_assignee_rejected() {
    let temp_dir = setup_test_workspace();
    let workspace_dir = temp_dir.path().join("test-workspace");

    let (success, stdout, stderr) = run_bf_create(&workspace_dir, "Test bead", Some(""));

    assert!(!success, "bf create should fail with empty assignee");
    assert!(
        stderr.contains("Assignee cannot be empty or whitespace-only"),
        "Error message should mention empty assignee"
    );
}

#[test]
fn test_create_bead_with_whitespace_only_assignee_rejected() {
    let temp_dir = setup_test_workspace();
    let workspace_dir = temp_dir.path().join("test-workspace");

    let (success, stdout, stderr) = run_bf_create(&workspace_dir, "Test bead", Some("   "));

    assert!(!success, "bf create should fail with whitespace-only assignee");
    assert!(
        stderr.contains("Assignee cannot be empty or whitespace-only"),
        "Error message should mention empty assignee"
    );
}

#[test]
fn test_create_bead_with_tab_whitespace_assignee_rejected() {
    let temp_dir = setup_test_workspace();
    let workspace_dir = temp_dir.path().join("test-workspace");

    let (success, stdout, stderr) = run_bf_create(&workspace_dir, "Test bead", Some("\t\t"));

    assert!(!success, "bf create should fail with tab whitespace-only assignee");
    assert!(
        stderr.contains("Assignee cannot be empty or whitespace-only"),
        "Error message should mention empty assignee"
    );
}

#[test]
fn test_create_bead_with_valid_assignee_accepted() {
    let temp_dir = setup_test_workspace();
    let workspace_dir = temp_dir.path().join("test-workspace");

    let (success, stdout, _stderr) = run_bf_create(&workspace_dir, "Test bead", Some("alice"));

    assert!(success, "bf create should succeed with valid assignee");
    assert!(!stdout.is_empty(), "Should output bead ID");
}

#[test]
fn test_create_bead_without_assignee_accepted() {
    let temp_dir = setup_test_workspace();
    let workspace_dir = temp_dir.path().join("test-workspace");

    let (success, stdout, _stderr) = run_bf_create(&workspace_dir, "Test bead", None);

    assert!(success, "bf create should succeed without assignee");
    assert!(!stdout.is_empty(), "Should output bead ID");
}

#[test]
fn test_create_bead_with_padded_whitespace_assignee_accepted() {
    let temp_dir = setup_test_workspace();
    let workspace_dir = temp_dir.path().join("test-workspace");

    let (success, stdout, _stderr) = run_bf_create(&workspace_dir, "Test bead", Some("  alice  "));

    assert!(success, "bf create should succeed with padded assignee (after trim)");
    assert!(!stdout.is_empty(), "Should output bead ID");
}

#[test]
fn test_update_bead_with_empty_assignee_rejected() {
    let temp_dir = setup_test_workspace();
    let workspace_dir = temp_dir.path().join("test-workspace");

    // First create a valid bead
    let (success, stdout, _) = run_bf_create(&workspace_dir, "Test bead", None);
    assert!(success, "Failed to create test bead");
    let bead_id = stdout.trim();

    // Try to update with empty assignee
    let (success, _stdout, stderr) = run_bf_update(&workspace_dir, bead_id, Some(""));

    assert!(!success, "bf update should fail with empty assignee");
    assert!(
        stderr.contains("Assignee cannot be empty or whitespace-only"),
        "Error message should mention empty assignee"
    );
}

#[test]
fn test_update_bead_with_whitespace_only_assignee_rejected() {
    let temp_dir = setup_test_workspace();
    let workspace_dir = temp_dir.path().join("test-workspace");

    // First create a valid bead
    let (success, stdout, _) = run_bf_create(&workspace_dir, "Test bead", None);
    assert!(success, "Failed to create test bead");
    let bead_id = stdout.trim();

    // Try to update with whitespace-only assignee
    let (success, _stdout, stderr) = run_bf_update(&workspace_dir, bead_id, Some("   "));

    assert!(!success, "bf update should fail with whitespace-only assignee");
    assert!(
        stderr.contains("Assignee cannot be empty or whitespace-only"),
        "Error message should mention empty assignee"
    );
}

#[test]
fn test_update_bead_with_valid_assignee_accepted() {
    let temp_dir = setup_test_workspace();
    let workspace_dir = temp_dir.path().join("test-workspace");

    // First create a valid bead
    let (success, stdout, _) = run_bf_create(&workspace_dir, "Test bead", None);
    assert!(success, "Failed to create test bead");
    let bead_id = stdout.trim();

    // Update with valid assignee
    let (success, _stdout, _stderr) = run_bf_update(&workspace_dir, bead_id, Some("bob"));

    assert!(success, "bf update should succeed with valid assignee");
}

#[test]
fn test_update_bead_without_assignee_accepted() {
    let temp_dir = setup_test_workspace();
    let workspace_dir = temp_dir.path().join("test-workspace");

    // First create a valid bead with assignee
    let (success, stdout, _) = run_bf_create(&workspace_dir, "Test bead", Some("alice"));
    assert!(success, "Failed to create test bead");
    let bead_id = stdout.trim();

    // Update without specifying assignee (should not clear the existing assignee)
    let (success, _stdout, _stderr) = run_bf_update(&workspace_dir, bead_id, None);

    assert!(success, "bf update should succeed without specifying assignee");

    // Verify the assignee is still set (not cleared)
    let output = Command::new("bf")
        .arg("show")
        .arg(bead_id)
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf show");

    let show_output = String::from_utf8_lossy(&output.stdout);
    assert!(show_output.contains("alice"), "Assignee should still be set to alice");
}

#[test]
fn test_create_bead_with_special_characters_in_assignee() {
    let temp_dir = setup_test_workspace();
    let workspace_dir = temp_dir.path().join("test-workspace");

    // Test with email format
    let (success, stdout, _stderr) = run_bf_create(&workspace_dir, "Test bead with email", Some("alice@example.com"));
    assert!(success, "bf create should succeed with email assignee");

    // Test with hyphens
    let (success, stdout, _stderr) = run_bf_create(&workspace_dir, "Test bead with hyphens", Some("alice-worker-1"));
    assert!(success, "bf create should succeed with hyphenated assignee");

    // Test with underscores
    let (success, stdout, _stderr) = run_bf_create(&workspace_dir, "Test bead with underscores", Some("alice_worker"));
    assert!(success, "bf create should succeed with underscored assignee");

    assert!(!stdout.is_empty(), "Should output bead ID");
}

#[test]
fn test_update_assignee_from_valid_to_valid() {
    let temp_dir = setup_test_workspace();
    let workspace_dir = temp_dir.path().join("test-workspace");

    // First create a valid bead with assignee
    let (success, stdout, _) = run_bf_create(&workspace_dir, "Test bead", Some("alice"));
    assert!(success, "Failed to create test bead");
    let bead_id = stdout.trim();

    // Update to a different valid assignee
    let (success, _stdout, _stderr) = run_bf_update(&workspace_dir, bead_id, Some("bob"));

    assert!(success, "bf update should succeed when changing assignee from alice to bob");

    // Verify the assignee was updated
    let output = Command::new("bf")
        .arg("show")
        .arg(bead_id)
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf show");

    let show_output = String::from_utf8_lossy(&output.stdout);
    assert!(show_output.contains("bob"), "Assignee should be updated to bob");
    assert!(!show_output.contains("alice"), "Old assignee alice should not appear");
}
