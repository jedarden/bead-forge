//! Test: Negative Priority Validation
//!
//! Test that `bf create` with negative priority fails gracefully.
//! This validates bead bf-4hjupr.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Create a temporary workspace for testing
fn setup_test_workspace() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("test-workspace");
    fs::create_dir_all(&workspace_dir).unwrap();
    let beads_dir = workspace_dir.join(".beads");
    fs::create_dir_all(&beads_dir).unwrap();

    let config_path = beads_dir.join("config.yaml");
    fs::write(
        &config_path,
        r#"issue_prefixes: [bf]
default_priority: 2
default_type: task
claim_ttl_minutes: 30
"#,
    )
    .unwrap();

    let metadata_path = beads_dir.join("metadata.json");
    fs::write(
        &metadata_path,
        r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
    )
    .unwrap();

    let db_path = beads_dir.join("beads.db");
    bead_forge::storage::Storage::open(&db_path).unwrap();

    (temp_dir, beads_dir)
}

/// Get the path to the bf binary
fn get_bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Run a bf command and return the output
fn run_bf_command(workspace: &Path, args: &[&str]) -> (String, String, bool) {
    let out = Command::new(get_bf_binary())
        .args(args)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf command");
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    let success = out.status.success();
    (stdout, stderr, success)
}

/// Test 1: Epic with negative priority -1 should fail
#[test]
fn test_epic_negative_priority_fails() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Attempt to create epic with negative priority
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Bad Epic",
            "--type",
            "epic",
            "--priority",
            "-1",
        ],
    );

    // Should fail
    assert!(!success, "bf create with negative priority should fail");

    // Error message should indicate negative priority is invalid
    let error_output = if !stderr.is_empty() {
        stderr
    } else {
        stdout
    };

    assert!(
        error_output.contains("priority") || error_output.contains("Priority"),
        "Error message should mention 'priority': {}",
        error_output
    );
    assert!(
        error_output.contains("invalid") || error_output.contains("must") || error_output.contains("0.*4"),
        "Error message should indicate priority is invalid: {}",
        error_output
    );
}

/// Test 2: Epic with priority less than -10 should fail
#[test]
fn test_epic_large_negative_priority_fails() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Attempt to create epic with large negative priority
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Very Bad Epic",
            "--type",
            "epic",
            "--priority",
            "-10",
        ],
    );

    // Should fail
    assert!(!success, "bf create with large negative priority should fail");

    // Error message should mention priority
    let error_output = if !stderr.is_empty() {
        stderr
    } else {
        stdout
    };

    assert!(
        error_output.contains("priority") || error_output.contains("Priority"),
        "Error message should mention 'priority': {}",
        error_output
    );
}

/// Test 3: Valid priority boundaries (0 and 4) should work
#[test]
fn test_epic_valid_priority_boundaries() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic with priority 0 (Critical) - should succeed
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Valid Critical Epic",
            "--type",
            "epic",
            "--priority",
            "0",
        ],
    );

    assert!(success, "Priority 0 (Critical) should be valid: stderr={}", stderr);
    assert!(!stdout.is_empty(), "Should output bead ID for priority 0");

    // Create epic with priority 4 (Backlog) - should succeed
    let (stdout2, stderr2, success2) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Valid Backlog Epic",
            "--type",
            "epic",
            "--priority",
            "4",
        ],
    );

    assert!(success2, "Priority 4 (Backlog) should be valid: stderr={}", stderr2);
    assert!(!stdout2.is_empty(), "Should output bead ID for priority 4");
}

/// Test 4: Priority greater than 4 should fail
#[test]
fn test_epic_priority_above_maximum_fails() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Attempt to create epic with priority 5 (above maximum)
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Bad Epic High Priority",
            "--type",
            "epic",
            "--priority",
            "5",
        ],
    );

    // Should fail
    assert!(!success, "bf create with priority > 4 should fail");

    // Error message should mention priority
    let error_output = if !stderr.is_empty() {
        stderr
    } else {
        stdout
    };

    assert!(
        error_output.contains("priority") || error_output.contains("Priority"),
        "Error message should mention 'priority': {}",
        error_output
    );
}

/// Test 5: Task with negative priority should also fail
#[test]
fn test_task_negative_priority_fails() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Attempt to create task with negative priority
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Bad Task",
            "--type",
            "task",
            "--priority",
            "-1",
        ],
    );

    // Should fail
    assert!(!success, "bf create with negative priority should fail for tasks too");

    // Error message should mention priority
    let error_output = if !stderr.is_empty() {
        stderr
    } else {
        stdout
    };

    assert!(
        error_output.contains("priority") || error_output.contains("Priority"),
        "Error message should mention 'priority': {}",
        error_output
    );
}

/// Test 6: Verify valid priorities work correctly for epics
#[test]
fn test_epic_valid_priorities() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let priorities = vec![0, 1, 2, 3, 4];
    let titles = vec![
        "Critical Epic",
        "High Epic",
        "Medium Epic",
        "Low Epic",
        "Backlog Epic",
    ];

    for (priority, title) in priorities.iter().zip(titles.iter()) {
        let (stdout, stderr, success) = run_bf_command(
            workspace,
            &[
                "create",
                "--title",
                title,
                "--type",
                "epic",
                "--priority",
                &priority.to_string(),
            ],
        );

        assert!(
            success,
            "Priority {} should be valid for epic: stderr={}",
            priority,
            stderr
        );
        assert!(
            !stdout.is_empty(),
            "Should output bead ID for priority {}",
            priority
        );
    }
}
