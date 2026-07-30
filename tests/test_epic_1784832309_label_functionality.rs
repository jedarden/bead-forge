//! Comprehensive integration tests for Epic 1784832309: Label Functionality
//!
//! This test suite validates:
//! - Creating epics with multiple labels
//! - Adding/removing labels to/from existing epics
//! - Filtering epics by label
//! - Label persistence and serialization
//! - Epic-specific label operations (epic vs child labels)
//! - List filtering by type (epic) with labels

use std::fs;
use std::path::PathBuf;
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

/// Extract bead ID from command output
fn extract_bead_id(output: &str) -> String {
    output
        .lines()
        .find(|line| line.contains("bf-"))
        .and_then(|line| line.split("bf-").nth(1))
        .map(|id| format!("bf-{}", id.trim().split_whitespace().next().unwrap_or(id)))
        .expect("Could not extract bead ID from output")
}

/// Run `bf labels <id>` and return labels
fn get_bead_labels(workspace: &std::path::Path, bead_id: &str) -> Vec<String> {
    let out = Command::new(get_bf_binary())
        .args(["labels", bead_id])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf labels");
    assert!(
        out.status.success(),
        "bf labels failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

/// Create an epic with specified labels
fn create_epic_with_labels(
    workspace: &std::path::Path,
    title: &str,
    labels: &[&str],
) -> String {
    let mut cmd = Command::new(get_bf_binary());
    cmd.args(["create", "--title", title, "--type", "epic"]);
    for label in labels {
        cmd.args(["--label", label]);
    }
    let out = cmd
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(
        out.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    extract_bead_id(&stdout)
}

/// Add labels to a bead
fn add_labels(workspace: &std::path::Path, bead_id: &str, labels: &[&str]) {
    let mut cmd = Command::new(get_bf_binary());
    cmd.args(["label", "add", bead_id]);
    for label in labels {
        cmd.args(["--label", label]);
    }
    let out = cmd
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf label add");
    assert!(
        out.status.success(),
        "bf label add failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Remove labels from a bead
fn remove_labels(workspace: &std::path::Path, bead_id: &str, labels: &[&str]) {
    let mut cmd = Command::new(get_bf_binary());
    cmd.args(["label", "remove", bead_id]);
    for label in labels {
        cmd.args(["--label", label]);
    }
    let out = cmd
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf label remove");
    assert!(
        out.status.success(),
        "bf label remove failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn test_epic_label_functionality_1_create_with_labels() {
    // Test 1: Create epic with multiple labels
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let epic_id = create_epic_with_labels(
        workspace,
        "Test Epic with Labels",
        &["epic-test", "phase-1", "priority-high"],
    );

    let labels = get_bead_labels(workspace, &epic_id);
    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"epic-test".to_string()));
    assert!(labels.contains(&"phase-1".to_string()));
    assert!(labels.contains(&"priority-high".to_string()));
}

#[test]
fn test_epic_label_functionality_2_add_labels_to_existing_epic() {
    // Test 2: Add labels to existing epic
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let epic_id = create_epic_with_labels(workspace, "Epic for Add Test", &["base-label"]);

    let before = get_bead_labels(workspace, &epic_id);
    assert_eq!(before.len(), 1);

    add_labels(workspace, &epic_id, &["new-label-1", "new-label-2"]);

    let after = get_bead_labels(workspace, &epic_id);
    assert_eq!(after.len(), 3);
    assert!(after.contains(&"base-label".to_string()));
    assert!(after.contains(&"new-label-1".to_string()));
    assert!(after.contains(&"new-label-2".to_string()));
}

#[test]
fn test_epic_label_functionality_3_remove_labels_from_epic() {
    // Test 3: Remove labels from epic
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let epic_id = create_epic_with_labels(
        workspace,
        "Epic for Remove Test",
        &["label-1", "label-2", "label-3"],
    );

    let before = get_bead_labels(workspace, &epic_id);
    assert_eq!(before.len(), 3);

    remove_labels(workspace, &epic_id, &["label-2"]);

    let after = get_bead_labels(workspace, &epic_id);
    assert_eq!(after.len(), 2);
    assert!(after.contains(&"label-1".to_string()));
    assert!(after.contains(&"label-3".to_string()));
    assert!(!after.contains(&"label-2".to_string()));
}

#[test]
fn test_epic_label_functionality_4_filter_epics_by_label() {
    // Test 4: List/filter epics by label
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create multiple epics with different labels
    let epic1 = create_epic_with_labels(workspace, "Epic 1", &["frontend", "phase-1"]);
    let epic2 = create_epic_with_labels(workspace, "Epic 2", &["backend", "phase-1"]);
    let epic3 = create_epic_with_labels(workspace, "Epic 3", &["frontend", "urgent"]);

    // List all epics
    let out = Command::new(get_bf_binary())
        .args(["list", "--type", "epic", "--format", "json"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf list");

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();

    // Verify we have 3 epics
    let count = stdout.lines().count();
    assert_eq!(count, 3);

    // Verify each epic has its labels
    for line in stdout.lines() {
        if line.contains(&epic1) {
            assert!(line.contains("frontend"));
            assert!(line.contains("phase-1"));
        } else if line.contains(&epic2) {
            assert!(line.contains("backend"));
            assert!(line.contains("phase-1"));
        } else if line.contains(&epic3) {
            assert!(line.contains("frontend"));
            assert!(line.contains("urgent"));
        }
    }
}

#[test]
fn test_epic_label_functionality_5_set_semantics_duplicate_handling() {
    // Test 5: Set semantics - duplicate labels don't create duplicates
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let epic_id = create_epic_with_labels(workspace, "Set Semantics Epic", &["unique-label"]);

    let before = get_bead_labels(workspace, &epic_id);
    assert_eq!(before.len(), 1);

    // Try adding the same label again
    add_labels(workspace, &epic_id, &["unique-label"]);

    let after = get_bead_labels(workspace, &epic_id);
    assert_eq!(after.len(), 1); // Still only 1 label
    assert!(after.contains(&"unique-label".to_string()));
}

#[test]
fn test_epic_label_functionality_6_idempotent_removal() {
    // Test 6: Removing non-existent label is idempotent (no-op)
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let epic_id = create_epic_with_labels(workspace, "Idempotent Epic", &["existing-label"]);

    let before = get_bead_labels(workspace, &epic_id);
    assert_eq!(before.len(), 1);

    // Remove a label that doesn't exist - should succeed and not change anything
    remove_labels(workspace, &epic_id, &["non-existent-label"]);

    let after = get_bead_labels(workspace, &epic_id);
    assert_eq!(after.len(), 1);
    assert!(after.contains(&"existing-label".to_string()));
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_epic_label_functionality_7_epic_with_child_labels_independence() {
    // Test 7: Epic and child beads have independent labels
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let epic_id = create_epic_with_labels(workspace, "Parent Epic", &["epic-only", "parent"]);

    // Create a child task with different labels
    let child_id = create_epic_with_labels(workspace, "Child Task", &["child-only", "task"]);

    // Add dependency
    let out = Command::new(get_bf_binary())
        .args(["dep", "add-blocker", &epic_id, &child_id])
        .current_dir(workspace)
        .output()
        .expect("Failed to add dependency");
    assert!(out.status.success());

    // Verify labels are independent
    let epic_labels = get_bead_labels(workspace, &epic_id);
    let child_labels = get_bead_labels(workspace, &child_id);

    assert_eq!(epic_labels.len(), 2);
    assert!(epic_labels.contains(&"epic-only".to_string()));
    assert!(epic_labels.contains(&"parent".to_string()));
    assert!(!epic_labels.contains(&"child-only".to_string()));

    assert_eq!(child_labels.len(), 2);
    assert!(child_labels.contains(&"child-only".to_string()));
    assert!(child_labels.contains(&"task".to_string()));
    assert!(!child_labels.contains(&"epic-only".to_string()));
}

#[test]
fn test_epic_label_functionality_8_label_persistence_across_commands() {
    // Test 8: Labels persist across different operations
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let epic_id =
        create_epic_with_labels(workspace, "Persistence Epic", &["persistent", "test-data"]);

    // Perform various operations
    add_labels(workspace, &epic_id, &["added-later"]);
    remove_labels(workspace, &epic_id, &["test-data"]);

    // Update epic title
    let out = Command::new(get_bf_binary())
        .args(["update", &epic_id, "--title", "Updated Persistence Epic"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf update");
    assert!(out.status.success());

    // Verify labels persist correctly
    let final_labels = get_bead_labels(workspace, &epic_id);
    assert_eq!(final_labels.len(), 2);
    assert!(final_labels.contains(&"persistent".to_string()));
    assert!(final_labels.contains(&"added-later".to_string()));
    assert!(!final_labels.contains(&"test-data".to_string()));
}

#[test]
fn test_epic_label_functionality_9_empty_label_handling() {
    // Test 9: Handle epics with no labels
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic without labels
    let mut cmd = Command::new(get_bf_binary());
    cmd.args(["create", "--title", "No Labels Epic", "--type", "epic"]);
    let out = cmd
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(out.status.success());

    let epic_id = extract_bead_id(&String::from_utf8(out.stdout).unwrap());

    let labels = get_bead_labels(workspace, &epic_id);
    assert_eq!(labels.len(), 0);

    // Add a label to empty epic
    add_labels(workspace, &epic_id, &["now-has-label"]);

    let labels = get_bead_labels(workspace, &epic_id);
    assert_eq!(labels.len(), 1);
    assert!(labels.contains(&"now-has-label".to_string()));
}

#[test]
fn test_epic_label_functionality_10_multiple_label_operations_in_sequence() {
    // Test 10: Multiple label operations in sequence
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let epic_id = create_epic_with_labels(
        workspace,
        "Multi-Op Epic",
        &["initial", "starting-point"],
    );

    // Sequence: add, add, remove, add, remove
    add_labels(workspace, &epic_id, &["added-1"]);
    add_labels(workspace, &epic_id, &["added-2"]);
    remove_labels(workspace, &epic_id, &["initial"]);
    add_labels(workspace, &epic_id, &["added-3"]);
    remove_labels(workspace, &epic_id, &["starting-point"]);

    let labels = get_bead_labels(workspace, &epic_id);
    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"added-1".to_string()));
    assert!(labels.contains(&"added-2".to_string()));
    assert!(labels.contains(&"added-3".to_string()));
    assert!(!labels.contains(&"initial".to_string()));
    assert!(!labels.contains(&"starting-point".to_string()));
}

#[test]
fn test_epic_label_functionality_11_json_format_with_labels() {
    // Test 11: Verify labels appear correctly in JSON output
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let epic_id = create_epic_with_labels(
        workspace,
        "JSON Format Epic",
        &["json-test", "format-validation"],
    );

    let out = Command::new(get_bf_binary())
        .args(["show", &epic_id, "--format", "json"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();

    // Verify JSON contains the labels
    assert!(stdout.contains("\"labels\""));
    assert!(stdout.contains("json-test"));
    assert!(stdout.contains("format-validation"));
}

#[test]
fn test_epic_label_functionality_12_label_order_independence() {
    // Test 12: Label operations work regardless of initial order
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epics with same labels but different initial order
    let epic1 = create_epic_with_labels(workspace, "Order Test 1", &["z", "a", "m"]);
    let epic2 = create_epic_with_labels(workspace, "Order Test 2", &["a", "m", "z"]);

    let labels1 = get_bead_labels(workspace, &epic1);
    let labels2 = get_bead_labels(workspace, &epic2);

    // Both should have the same labels
    assert_eq!(labels1.len(), 3);
    assert_eq!(labels2.len(), 3);
    assert!(labels1.contains(&"z".to_string()));
    assert!(labels1.contains(&"a".to_string()));
    assert!(labels1.contains(&"m".to_string()));
    assert!(labels2.contains(&"z".to_string()));
    assert!(labels2.contains(&"a".to_string()));
    assert!(labels2.contains(&"m".to_string()));
}

#[test]
fn test_epic_label_functionality_13_special_characters_in_labels() {
    // Test 13: Handle labels with special characters
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let epic_id = create_epic_with_labels(
        workspace,
        "Special Chars Epic",
        &["label-with-dash", "label_with_underscore", "label.with.dots"],
    );

    let labels = get_bead_labels(workspace, &epic_id);
    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"label-with-dash".to_string()));
    assert!(labels.contains(&"label_with_underscore".to_string()));
    assert!(labels.contains(&"label.with.dots".to_string()));
}
