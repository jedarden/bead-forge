//! Test label add/remove operations on individual beads
//! Tests basic label add and remove operations per acceptance criteria in bf-1n8dfu

use tempfile::TempDir;
use std::path::PathBuf;
use std::process::Command;
use std::str;

/// Helper to run bf commands in a test workspace
fn bf_command(workspace_dir: &PathBuf, args: &[&str]) -> String {
    let output = Command::new("cargo")
        .arg("run")
        .arg("--")
        .args(args)
        .current_dir(workspace_dir)
        .env("BF_DB_PATH", workspace_dir.join(".beads/beads.db").to_str().unwrap())
        .output()
        .expect("Failed to run bf command");

    let stdout = str::from_utf8(&output.stdout).unwrap_or("").to_string();
    let stderr = str::from_utf8(&output.stderr).unwrap_or("").to_string();

    if !output.status.success() {
        eprintln!("Command failed: bf {}", args.join(" "));
        eprintln!("stdout: {}", stdout);
        eprintln!("stderr: {}", stderr);
        panic!("bf command failed with exit code: {:?}", output.status.code());
    }

    format!("{}\n{}", stdout, stderr)
}

/// Helper to create a test workspace
fn create_test_workspace() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let beads_dir = dir.path().join(".beads");
    std::fs::create_dir(&beads_dir).expect("Failed to create .beads dir");

    // Initialize the workspace
    bf_command(&dir.path().to_path_buf(), &["init", "--prefix", "test"]);

    dir
}

#[test]
fn test_add_single_label() {
    let workspace = create_test_workspace();
    let ws_path = workspace.path().to_path_buf();

    // Create a test bead
    let output = bf_command(&ws_path, &[
        "create", "--title", "Test bead for labels", "--type", "task"
    ]);
    assert!(output.contains("test-"));

    // Extract the bead ID
    let bead_id = output
        .lines()
        .find(|line| line.starts_with("test-"))
        .unwrap()
        .trim()
        .to_string();

    // Add a single label
    let output = bf_command(&ws_path, &["label", "add", &bead_id, "--label", "test-label"]);
    assert!(output.contains("test-label") || output.contains("Added") || output.contains("success"));

    // Verify label appears in show output
    let show_output = bf_command(&ws_path, &["show", &bead_id]);
    assert!(show_output.contains("test-label"));

    println!("✓ test_add_single_label passed");
}

#[test]
fn test_add_multiple_labels_at_once() {
    let workspace = create_test_workspace();
    let ws_path = workspace.path().to_path_buf();

    // Create a test bead
    let output = bf_command(&ws_path, &[
        "create", "--title", "Test multiple labels", "--type", "task"
    ]);
    let bead_id = output
        .lines()
        .find(|line| line.starts_with("test-"))
        .unwrap()
        .trim()
        .to_string();

    // Add multiple labels at once
    let output = bf_command(&ws_path, &[
        "label", "add", &bead_id,
        "--label", "label1",
        "--label", "label2",
        "--label", "label3"
    ]);
    assert!(output.contains("success") || output.contains("Added") || output.contains("label"));

    // Verify all labels appear in show output
    let show_output = bf_command(&ws_path, &["show", &bead_id]);
    assert!(show_output.contains("label1"));
    assert!(show_output.contains("label2"));
    assert!(show_output.contains("label3"));

    println!("✓ test_add_multiple_labels_at_once passed");
}

#[test]
fn test_duplicate_labels_idempotent() {
    let workspace = create_test_workspace();
    let ws_path = workspace.path().to_path_buf();

    // Create a test bead
    let output = bf_command(&ws_path, &[
        "create", "--title", "Test duplicate labels", "--type", "task"
    ]);
    let bead_id = output
        .lines()
        .find(|line| line.starts_with("test-"))
        .unwrap()
        .trim()
        .to_string();

    // Add a label
    bf_command(&ws_path, &["label", "add", &bead_id, "--label", "unique-label"]);

    // Try adding the same label again - should not error or duplicate
    let output = bf_command(&ws_path, &["label", "add", &bead_id, "--label", "unique-label"]);
    assert!(!output.to_lowercase().contains("error"));

    // Verify label appears only once
    let show_output = bf_command(&ws_path, &["show", &bead_id, "--json"]);
    let label_count = show_output.matches("unique-label").count();
    assert_eq!(label_count, 1, "Label should appear exactly once, not duplicated");

    println!("✓ test_duplicate_labels_idempotent passed");
}

#[test]
fn test_remove_single_label() {
    let workspace = create_test_workspace();
    let ws_path = workspace.path().to_path_buf();

    // Create a test bead with labels
    let output = bf_command(&ws_path, &[
        "create", "--title", "Test remove label", "--type", "task"
    ]);
    let bead_id = output
        .lines()
        .find(|line| line.starts_with("test-"))
        .unwrap()
        .trim()
        .to_string();

    // Add labels
    bf_command(&ws_path, &[
        "label", "add", &bead_id,
        "--label", "keep1",
        "--label", "remove-me",
        "--label", "keep2"
    ]);

    // Remove a single label
    let output = bf_command(&ws_path, &["label", "remove", &bead_id, "--label", "remove-me"]);
    assert!(output.contains("success") || output.contains("Removed") || output.contains("remove-me"));

    // Verify label was removed but others remain
    let show_output = bf_command(&ws_path, &["show", &bead_id]);
    assert!(show_output.contains("keep1"));
    assert!(show_output.contains("keep2"));
    assert!(!show_output.contains("remove-me"));

    println!("✓ test_remove_single_label passed");
}

#[test]
fn test_remove_multiple_labels_at_once() {
    let workspace = create_test_workspace();
    let ws_path = workspace.path().to_path_buf();

    // Create a test bead with labels
    let output = bf_command(&ws_path, &[
        "create", "--title", "Test remove multiple", "--type", "task"
    ]);
    let bead_id = output
        .lines()
        .find(|line| line.starts_with("test-"))
        .unwrap()
        .trim()
        .to_string();

    // Add labels
    bf_command(&ws_path, &[
        "label", "add", &bead_id,
        "--label", "keep",
        "--label", "remove1",
        "--label", "remove2",
        "--label", "remove3"
    ]);

    // Remove multiple labels at once
    let output = bf_command(&ws_path, &[
        "label", "remove", &bead_id,
        "--label", "remove1",
        "--label", "remove2",
        "--label", "remove3"
    ]);
    assert!(output.contains("success") || output.contains("Removed"));

    // Verify only the kept label remains
    let show_output = bf_command(&ws_path, &["show", &bead_id]);
    assert!(show_output.contains("keep"));
    assert!(!show_output.contains("remove1"));
    assert!(!show_output.contains("remove2"));
    assert!(!show_output.contains("remove3"));

    println!("✓ test_remove_multiple_labels_at_once passed");
}

#[test]
fn test_remove_nonexistent_label_graceful() {
    let workspace = create_test_workspace();
    let ws_path = workspace.path().to_path_buf();

    // Create a test bead
    let output = bf_command(&ws_path, &[
        "create", "--title", "Test nonexistent remove", "--type", "task"
    ]);
    let bead_id = output
        .lines()
        .find(|line| line.starts_with("test-"))
        .unwrap()
        .trim()
        .to_string();

    // Try removing a label that doesn't exist - should not error
    let output = bf_command(&ws_path, &["label", "remove", &bead_id, "--label", "nonexistent"]);
    assert!(!output.to_lowercase().contains("error"), "Should handle gracefully without error");

    println!("✓ test_remove_nonexistent_label_graceful passed");
}

#[test]
fn test_labels_visible_in_show() {
    let workspace = create_test_workspace();
    let ws_path = workspace.path().to_path_buf();

    // Create a test bead
    let output = bf_command(&ws_path, &[
        "create", "--title", "Test show visibility", "--type", "task"
    ]);
    let bead_id = output
        .lines()
        .find(|line| line.starts_with("test-"))
        .unwrap()
        .trim()
        .to_string();

    // Add labels
    bf_command(&ws_path, &[
        "label", "add", &bead_id,
        "--label", "priority",
        "--label", "backend"
    ]);

    // Verify labels appear in both text and JSON output
    let show_output = bf_command(&ws_path, &["show", &bead_id]);
    assert!(show_output.contains("priority"));
    assert!(show_output.contains("backend"));

    let show_json = bf_command(&ws_path, &["show", &bead_id, "--json"]);
    assert!(show_json.contains("priority"));
    assert!(show_json.contains("backend"));

    println!("✓ test_labels_visible_in_show passed");
}

#[test]
fn test_labels_persist_in_database() {
    let workspace = create_test_workspace();
    let ws_path = workspace.path().to_path_buf();

    // Create a test bead
    let output = bf_command(&ws_path, &[
        "create", "--title", "Test persistence", "--type", "task"
    ]);
    let bead_id = output
        .lines()
        .find(|line| line.starts_with("test-"))
        .unwrap()
        .trim()
        .to_string();

    // Add labels
    bf_command(&ws_path, &[
        "label", "add", &bead_id,
        "--label", "persistent1",
        "--label", "persistent2"
    ]);

    // Verify labels are in the database by querying via show
    let show_output = bf_command(&ws_path, &["show", &bead_id, "--json"]);
    assert!(show_output.contains("persistent1"));
    assert!(show_output.contains("persistent2"));

    println!("✓ test_labels_persist_in_database passed");
}
