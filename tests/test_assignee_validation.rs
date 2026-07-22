//! Integration tests for assignee field handling
//!
//! This test file validates that:
//! 1. Empty assignee during bead creation is normalized to unassigned (NULL)
//! 2. Whitespace-only assignee during bead creation is normalized to unassigned
//! 3. Tab-whitespace assignee during bead creation is normalized to unassigned
//! 4. Empty assignee during bead update clears the assignee (NULL)
//! 5. Whitespace-only assignee during bead update clears the assignee (NULL)
//! 6. Valid assignees are accepted during both create and update operations
//! 7. None (no assignee flag) is accepted during both create and update operations

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Create a temporary workspace for testing
/// Resolve the freshly-built bf binary — never the system-installed one.
fn bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

fn setup_test_workspace() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("test-workspace");
    std::fs::create_dir_all(&workspace_dir).unwrap();

    // Initialize bead-forge workspace
    let output = Command::new(bf_binary())
        .arg("init")
        .arg("--prefix")
        .arg("bf")
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf init");

    if !output.status.success() {
        panic!(
            "bf init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    temp_dir
}

/// Run bf create command and return the result
fn run_bf_create(
    workspace_dir: &PathBuf,
    title: &str,
    assignee: Option<&str>,
) -> (bool, String, String) {
    let mut cmd = Command::new(bf_binary());
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
fn run_bf_update(
    workspace_dir: &PathBuf,
    id: &str,
    assignee: Option<&str>,
) -> (bool, String, String) {
    let mut cmd = Command::new(bf_binary());
    cmd.arg("update").arg(id).current_dir(workspace_dir);

    if let Some(assignee_val) = assignee {
        cmd.arg("--assignee").arg(assignee_val);
    }

    let output = cmd.output().expect("Failed to run bf update");
    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (success, stdout, stderr)
}

/// Check via `bf show --json` that the bead's assignee is unset.
///
/// The Issue struct skips serialization when assignee is None, so an unassigned
/// bead has no `assignee` key at all. A set assignee always serializes as a
/// string value (`"assignee":"<name>"`); checking for the absence of that
/// string covers both the omitted-key and a hypothetical null case.
fn assignee_is_unset(workspace_dir: &PathBuf, id: &str) -> bool {
    let output = Command::new(bf_binary())
        .arg("show")
        .arg("--json")
        .arg(id)
        .current_dir(workspace_dir)
        .output()
        .expect("Failed to run bf show --json");
    assert!(
        output.status.success(),
        "bf show --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    !stdout.contains("\"assignee\":\"")
}

#[test]
fn test_create_bead_with_empty_assignee_succeeds_unassigned() {
    let temp_dir = setup_test_workspace();
    let workspace_dir = temp_dir.path().join("test-workspace");

    let (success, stdout, stderr) = run_bf_create(&workspace_dir, "Test bead", Some(""));

    assert!(
        success,
        "bf create with empty assignee should succeed, not fail. stderr: {stderr}"
    );
    let bead_id = stdout.trim();
    assert!(!bead_id.is_empty(), "Should output bead ID");
    assert!(
        assignee_is_unset(&workspace_dir, bead_id),
        "Empty assignee should be normalized to unassigned (NULL)"
    );
}

#[test]
fn test_create_bead_with_whitespace_only_assignee_succeeds_unassigned() {
    let temp_dir = setup_test_workspace();
    let workspace_dir = temp_dir.path().join("test-workspace");

    let (success, stdout, stderr) = run_bf_create(&workspace_dir, "Test bead", Some("   "));

    assert!(
        success,
        "bf create with whitespace-only assignee should succeed, not fail. stderr: {stderr}"
    );
    let bead_id = stdout.trim();
    assert!(!bead_id.is_empty(), "Should output bead ID");
    assert!(
        assignee_is_unset(&workspace_dir, bead_id),
        "Whitespace-only assignee should be normalized to unassigned (NULL)"
    );
}

#[test]
fn test_create_bead_with_tab_whitespace_assignee_succeeds_unassigned() {
    let temp_dir = setup_test_workspace();
    let workspace_dir = temp_dir.path().join("test-workspace");

    let (success, stdout, stderr) = run_bf_create(&workspace_dir, "Test bead", Some("\t\t"));

    assert!(
        success,
        "bf create with tab-whitespace assignee should succeed, not fail. stderr: {stderr}"
    );
    let bead_id = stdout.trim();
    assert!(!bead_id.is_empty(), "Should output bead ID");
    assert!(
        assignee_is_unset(&workspace_dir, bead_id),
        "Tab-whitespace assignee should be normalized to unassigned (NULL)"
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

    assert!(
        success,
        "bf create should succeed with padded assignee (after trim)"
    );
    assert!(!stdout.is_empty(), "Should output bead ID");
}

#[test]
fn test_update_bead_with_empty_assignee_succeeds_clears_assignee() {
    let temp_dir = setup_test_workspace();
    let workspace_dir = temp_dir.path().join("test-workspace");

    // First create a bead WITH an assignee, so clearing is observable.
    let (success, stdout, _stderr) = run_bf_create(&workspace_dir, "Test bead", Some("alice"));
    assert!(success, "Failed to create test bead");
    let bead_id = stdout.trim();

    // Sanity: assignee really is set before we clear it.
    assert!(
        !assignee_is_unset(&workspace_dir, bead_id),
        "Precondition: assignee should be set to alice before clearing"
    );

    // Update with empty assignee — must succeed and clear.
    let (success, _stdout, stderr) = run_bf_update(&workspace_dir, bead_id, Some(""));

    assert!(
        success,
        "bf update with empty assignee should succeed and clear, not fail. stderr: {stderr}"
    );
    assert!(
        assignee_is_unset(&workspace_dir, bead_id),
        "Empty assignee should clear the assignee (set to NULL)"
    );
}

#[test]
fn test_update_bead_with_whitespace_only_assignee_succeeds_clears_assignee() {
    let temp_dir = setup_test_workspace();
    let workspace_dir = temp_dir.path().join("test-workspace");

    // First create a bead WITH an assignee, so clearing is observable.
    let (success, stdout, _stderr) = run_bf_create(&workspace_dir, "Test bead", Some("alice"));
    assert!(success, "Failed to create test bead");
    let bead_id = stdout.trim();

    // Sanity: assignee really is set before we clear it.
    assert!(
        !assignee_is_unset(&workspace_dir, bead_id),
        "Precondition: assignee should be set to alice before clearing"
    );

    // Update with whitespace-only assignee — must succeed and clear.
    let (success, _stdout, stderr) = run_bf_update(&workspace_dir, bead_id, Some("   "));

    assert!(
        success,
        "bf update with whitespace-only assignee should succeed and clear, not fail. stderr: {stderr}"
    );
    assert!(
        assignee_is_unset(&workspace_dir, bead_id),
        "Whitespace-only assignee should clear the assignee (set to NULL)"
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

    assert!(
        success,
        "bf update should succeed without specifying assignee"
    );

    // Verify the assignee is still set (not cleared)
    let output = Command::new(bf_binary())
        .arg("show")
        .arg(bead_id)
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf show");

    let show_output = String::from_utf8_lossy(&output.stdout);
    assert!(
        show_output.contains("alice"),
        "Assignee should still be set to alice"
    );
}

#[test]
fn test_create_bead_with_special_characters_in_assignee() {
    let temp_dir = setup_test_workspace();
    let workspace_dir = temp_dir.path().join("test-workspace");

    // Test with email format
    let (success, stdout, _stderr) = run_bf_create(
        &workspace_dir,
        "Test bead with email",
        Some("alice@example.com"),
    );
    assert!(success, "bf create should succeed with email assignee");

    // Test with hyphens
    let (success, stdout, _stderr) = run_bf_create(
        &workspace_dir,
        "Test bead with hyphens",
        Some("alice-worker-1"),
    );
    assert!(success, "bf create should succeed with hyphenated assignee");

    // Test with underscores
    let (success, stdout, _stderr) = run_bf_create(
        &workspace_dir,
        "Test bead with underscores",
        Some("alice_worker"),
    );
    assert!(
        success,
        "bf create should succeed with underscored assignee"
    );

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

    assert!(
        success,
        "bf update should succeed when changing assignee from alice to bob"
    );

    // Verify the assignee was updated
    let output = Command::new(bf_binary())
        .arg("show")
        .arg(bead_id)
        .current_dir(&workspace_dir)
        .output()
        .expect("Failed to run bf show");

    let show_output = String::from_utf8_lossy(&output.stdout);
    assert!(
        show_output.contains("bob"),
        "Assignee should be updated to bob"
    );
    assert!(
        !show_output.contains("alice"),
        "Old assignee alice should not appear"
    );
}
