// Test labels functionality in bead-forge

use std::process::Command;

fn bf() -> Command {
    let mut cmd = Command::new("bf");
    cmd.arg("-w").arg(".beads").current_dir(".");
    cmd
}

fn create_test_bead(title: &str) -> String {
    let output = bf()
        .arg("create")
        .arg("--title")
        .arg(title)
        .arg("--type")
        .arg("task")
        .arg("--priority")
        .arg("2")
        .output()
        .expect("Failed to create bead");

    assert!(output.status.success(), "Failed to create bead: {}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    // Extract bead ID from output (format: "bf-xxxx")
    let id = stdout.trim().to_string();
    id
}

#[test]
fn test_label_add_and_list() {
    // Create a test bead
    let bead_id = create_test_bead("Test label bead");

    // Add labels
    let output = bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .arg("--label")
        .arg("backend")
        .arg("--label")
        .arg("phase-1")
        .output()
        .expect("Failed to add labels");

    assert!(output.status.success(), "Failed to add labels: {}", String::from_utf8_lossy(&output.stderr));

    // List labels for the bead
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    assert!(output.status.success(), "Failed to list labels: {}", String::from_utf8_lossy(&output.stderr));

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

    assert_eq!(labels.len(), 3, "Expected 3 labels, got {}", labels.len());
    assert!(labels.contains(&"urgent".to_string()), "Missing 'urgent' label");
    assert!(labels.contains(&"backend".to_string()), "Missing 'backend' label");
    assert!(labels.contains(&"phase-1".to_string()), "Missing 'phase-1' label");

    // Clean up
    bf().arg("close").arg(&bead_id).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead");
}

#[test]
fn test_label_remove() {
    // Create a test bead
    let bead_id = create_test_bead("Test label removal bead");

    // Add labels
    bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .arg("--label")
        .arg("backend")
        .arg("--label")
        .arg("bug")
        .output()
        .expect("Failed to add labels");

    // Remove one label
    let output = bf()
        .arg("label")
        .arg("remove")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .output()
        .expect("Failed to remove label");

    assert!(output.status.success(), "Failed to remove label: {}", String::from_utf8_lossy(&output.stderr));

    // Verify the label was removed
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    assert!(output.status.success(), "Failed to list labels: {}", String::from_utf8_lossy(&output.stderr));

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

    assert_eq!(labels.len(), 2, "Expected 2 labels after removal, got {}", labels.len());
    assert!(!labels.contains(&"urgent".to_string()), "'urgent' label should have been removed");
    assert!(labels.contains(&"backend".to_string()), "Missing 'backend' label");
    assert!(labels.contains(&"bug".to_string()), "Missing 'bug' label");

    // Clean up
    bf().arg("close").arg(&bead_id).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead");
}

#[test]
fn test_label_all_unique() {
    // Create multiple test beads with overlapping labels
    let bead1 = create_test_bead("Label list test bead 1");
    let bead2 = create_test_bead("Label list test bead 2");

    // Add different labels to each bead
    bf()
        .arg("label")
        .arg("add")
        .arg(&bead1)
        .arg("--label")
        .arg("urgent")
        .arg("--label")
        .arg("backend")
        .output()
        .expect("Failed to add labels to bead 1");

    bf()
        .arg("label")
        .arg("add")
        .arg(&bead2)
        .arg("--label")
        .arg("urgent")
        .arg("--label")
        .arg("frontend")
        .output()
        .expect("Failed to add labels to bead 2");

    // List all unique labels (no bead ID specified)
    let output = bf()
        .arg("label")
        .arg("list")
        .output()
        .expect("Failed to list all unique labels");

    assert!(output.status.success(), "Failed to list all labels: {}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    // Output format is "label (count)" per line
    let labels: Vec<String> = stdout
        .lines()
        .filter(|line| !line.is_empty() && !line.contains("All labels:"))
        .map(|line| line.trim().split('(').next().unwrap().trim().to_string())
        .collect();

    // Should have 3 unique labels: urgent, backend, frontend
    assert!(labels.len() >= 3, "Expected at least 3 unique labels, got {:?}: {}", labels, stdout);
    assert!(labels.contains(&"urgent".to_string()), "Missing 'urgent' label in {:?}", labels);
    assert!(labels.contains(&"backend".to_string()), "Missing 'backend' label in {:?}", labels);
    assert!(labels.contains(&"frontend".to_string()), "Missing 'frontend' label in {:?}", labels);

    // Clean up
    bf().arg("close").arg(&bead1).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead 1");
    bf().arg("close").arg(&bead2).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead 2");
}

#[test]
fn test_label_empty_bead() {
    // Create a test bead with no labels
    let bead_id = create_test_bead("Empty label test bead");

    // List labels for a bead with no labels
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    assert!(output.status.success(), "Failed to list labels: {}", String::from_utf8_lossy(&output.stderr));

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

    assert_eq!(labels.len(), 0, "Expected 0 labels for new bead, got {}", labels.len());

    // Clean up
    bf().arg("close").arg(&bead_id).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead");
}

#[test]
fn test_label_duplicate_handling() {
    // Create a test bead
    let bead_id = create_test_bead("Duplicate label test bead");

    // Add the same label twice
    bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .arg("--label")
        .arg("urgent")
        .output()
        .expect("Failed to add labels");

    // Verify only one instance of the label exists
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    assert!(output.status.success(), "Failed to list labels: {}", String::from_utf8_lossy(&output.stderr));

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

    assert_eq!(labels.len(), 1, "Expected 1 label after duplicate add, got {}", labels.len());
    assert!(labels.contains(&"urgent".to_string()), "Missing 'urgent' label");

    // Clean up
    bf().arg("close").arg(&bead_id).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead");
}

#[test]
fn test_label_remove_multiple() {
    // Create a test bead
    let bead_id = create_test_bead("Test multiple label removal bead");

    // Add multiple labels
    bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .arg("--label")
        .arg("backend")
        .arg("--label")
        .arg("bug")
        .arg("--label")
        .arg("phase-1")
        .output()
        .expect("Failed to add labels");

    // Remove multiple labels at once
    let output = bf()
        .arg("label")
        .arg("remove")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .arg("--label")
        .arg("bug")
        .output()
        .expect("Failed to remove labels");

    assert!(output.status.success(), "Failed to remove labels: {}", String::from_utf8_lossy(&output.stderr));

    // Verify the labels were removed
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    assert!(output.status.success(), "Failed to list labels: {}", String::from_utf8_lossy(&output.stderr));

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

    assert_eq!(labels.len(), 2, "Expected 2 labels after removal, got {}", labels.len());
    assert!(!labels.contains(&"urgent".to_string()), "'urgent' label should have been removed");
    assert!(!labels.contains(&"bug".to_string()), "'bug' label should have been removed");
    assert!(labels.contains(&"backend".to_string()), "Missing 'backend' label");
    assert!(labels.contains(&"phase-1".to_string()), "Missing 'phase-1' label");

    // Clean up
    bf().arg("close").arg(&bead_id).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead");
}

#[test]
fn test_label_remove_nonexistent() {
    // Create a test bead with labels
    let bead_id = create_test_bead("Test nonexistent label removal bead");

    // Add one label
    bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("backend")
        .output()
        .expect("Failed to add label");

    // Try to remove a label that doesn't exist (should be idempotent)
    let output = bf()
        .arg("label")
        .arg("remove")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .output()
        .expect("Failed to attempt removal");

    // Should succeed even if label doesn't exist
    assert!(output.status.success(), "Removing nonexistent label should succeed: {}", String::from_utf8_lossy(&output.stderr));

    // Verify the original label is still there
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

    assert_eq!(labels.len(), 1, "Expected 1 label, got {}", labels.len());
    assert!(labels.contains(&"backend".to_string()), "Missing 'backend' label");

    // Clean up
    bf().arg("close").arg(&bead_id).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead");
}

#[test]
fn test_label_remove_all_labels() {
    // Create a test bead
    let bead_id = create_test_bead("Test remove all labels bead");

    // Add a single label
    bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .output()
        .expect("Failed to add label");

    // Remove the last label
    let output = bf()
        .arg("label")
        .arg("remove")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .output()
        .expect("Failed to remove label");

    assert!(output.status.success(), "Failed to remove label: {}", String::from_utf8_lossy(&output.stderr));

    // Verify no labels remain
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

    assert_eq!(labels.len(), 0, "Expected 0 labels after removing all, got {}", labels.len());

    // Clean up
    bf().arg("close").arg(&bead_id).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead");
}

#[test]
fn test_label_remove_idempotent() {
    // Create a test bead
    let bead_id = create_test_bead("Test idempotent label removal bead");

    // Add a label
    bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .output()
        .expect("Failed to add label");

    // Remove the label twice (should be idempotent)
    let output1 = bf()
        .arg("label")
        .arg("remove")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .output()
        .expect("Failed to remove label first time");

    assert!(output1.status.success(), "First removal failed: {}", String::from_utf8_lossy(&output1.stderr));

    let output2 = bf()
        .arg("label")
        .arg("remove")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .output()
        .expect("Failed to remove label second time");

    // Second removal should also succeed (idempotent)
    assert!(output2.status.success(), "Second removal should succeed (idempotent): {}", String::from_utf8_lossy(&output2.stderr));

    // Verify no labels remain
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

    assert_eq!(labels.len(), 0, "Expected 0 labels, got {}", labels.len());

    // Clean up
    bf().arg("close").arg(&bead_id).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead");
}

#[test]
fn test_label_remove_empty_label_list() {
    // Create a test bead
    let bead_id = create_test_bead("Test remove from empty list bead");

    // Try to remove a label from a bead with no labels
    let output = bf()
        .arg("label")
        .arg("remove")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .output()
        .expect("Failed to attempt removal");

    // Should succeed (idempotent)
    assert!(output.status.success(), "Removing from empty label list should succeed: {}", String::from_utf8_lossy(&output.stderr));

    // Verify still no labels
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

    assert_eq!(labels.len(), 0, "Expected 0 labels, got {}", labels.len());

    // Clean up
    bf().arg("close").arg(&bead_id).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead");
}
