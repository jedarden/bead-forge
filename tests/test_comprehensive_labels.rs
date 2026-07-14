// Comprehensive label functionality tests
// Tests all label operations including add, remove, list, and edge cases

use std::process::Command;
use std::collections::HashSet;

/// Resolve the freshly-built bf binary — never the system-installed one.
fn bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

fn bf() -> Command {
    let mut cmd = Command::new(bf_binary());
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
    stdout.trim().to_string()
}

fn get_labels(bead_id: &str) -> Vec<String> {
    let output = bf()
        .arg("labels")
        .arg(bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    assert!(output.status.success(), "Failed to list labels: {}", String::from_utf8_lossy(&output.stderr));

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    serde_json::from_str::<Vec<String>>(&json_output).expect("Failed to parse labels JSON")
}

#[test]
fn test_label_lifecycle() {
    // Test complete label lifecycle: add -> list -> remove -> verify
    let bead_id = create_test_bead("Label lifecycle test");

    // Initial state: no labels
    let labels = get_labels(&bead_id);
    assert_eq!(labels.len(), 0, "New bead should have no labels");

    // Add labels
    let add_output = bf()
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

    assert!(add_output.status.success(), "Failed to add labels: {}",
        String::from_utf8_lossy(&add_output.stderr));

    // Verify labels were added
    let labels = get_labels(&bead_id);
    assert_eq!(labels.len(), 3, "Should have 3 labels after adding");

    let label_set: HashSet<_> = labels.into_iter().collect();
    assert!(label_set.contains("urgent"), "Missing 'urgent' label");
    assert!(label_set.contains("backend"), "Missing 'backend' label");
    assert!(label_set.contains("bug"), "Missing 'bug' label");

    // Remove one label
    let remove_output = bf()
        .arg("label")
        .arg("remove")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .output()
        .expect("Failed to remove label");

    assert!(remove_output.status.success(), "Failed to remove label: {}",
        String::from_utf8_lossy(&remove_output.stderr));

    // Verify label was removed
    let labels = get_labels(&bead_id);
    assert_eq!(labels.len(), 2, "Should have 2 labels after removal");

    let label_set: HashSet<_> = labels.into_iter().collect();
    assert!(!label_set.contains("urgent"), "'urgent' label should be removed");
    assert!(label_set.contains("backend"), "'backend' label should still exist");
    assert!(label_set.contains("bug"), "'bug' label should still exist");

    // Clean up
    bf().arg("close").arg(&bead_id).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead");
}

#[test]
fn test_label_duplicates_prevented() {
    // Test that duplicate labels are automatically prevented
    let bead_id = create_test_bead("Duplicate prevention test");

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

    // Verify only one instance exists
    let labels = get_labels(&bead_id);
    assert_eq!(labels.len(), 1, "Duplicate labels should be prevented");
    assert_eq!(labels[0], "urgent", "Only 'urgent' label should exist");

    // Clean up
    bf().arg("close").arg(&bead_id).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead");
}

#[test]
fn test_label_multiple_operations() {
    // Test multiple add and remove operations in sequence
    let bead_id = create_test_bead("Multiple operations test");

    // Add labels in batches
    bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("bug")
        .arg("--label")
        .arg("urgent")
        .output()
        .expect("Failed to add first batch");

    let labels = get_labels(&bead_id);
    assert_eq!(labels.len(), 2, "Should have 2 labels after first batch");

    // Add more labels
    bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("backend")
        .arg("--label")
        .arg("frontend")
        .output()
        .expect("Failed to add second batch");

    let labels = get_labels(&bead_id);
    assert_eq!(labels.len(), 4, "Should have 4 labels after second batch");

    // Remove multiple labels at once
    bf()
        .arg("label")
        .arg("remove")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .arg("--label")
        .arg("frontend")
        .output()
        .expect("Failed to remove labels");

    let labels = get_labels(&bead_id);
    assert_eq!(labels.len(), 2, "Should have 2 labels after removal");

    let label_set: HashSet<_> = labels.into_iter().collect();
    assert!(label_set.contains("bug"), "'bug' label should remain");
    assert!(label_set.contains("backend"), "'backend' label should remain");
    assert!(!label_set.contains("urgent"), "'urgent' label should be removed");
    assert!(!label_set.contains("frontend"), "'frontend' label should be removed");

    // Clean up
    bf().arg("close").arg(&bead_id).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead");
}

#[test]
fn test_label_idempotent_removal() {
    // Test that removing a non-existent label is idempotent
    let bead_id = create_test_bead("Idempotent removal test");

    // Add a label
    bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("backend")
        .output()
        .expect("Failed to add label");

    // Try to remove a label that doesn't exist
    let output = bf()
        .arg("label")
        .arg("remove")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .output()
        .expect("Failed to attempt removal");

    assert!(output.status.success(), "Removing non-existent label should succeed");

    // Verify the original label is still there
    let labels = get_labels(&bead_id);
    assert_eq!(labels.len(), 1, "Should still have 1 label");
    assert_eq!(labels[0], "backend", "Original label should remain");

    // Remove the same non-existent label again
    let output = bf()
        .arg("label")
        .arg("remove")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .output()
        .expect("Failed to attempt second removal");

    assert!(output.status.success(), "Second removal should also succeed");

    // Clean up
    bf().arg("close").arg(&bead_id).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead");
}

#[test]
fn test_label_special_characters() {
    // Test labels with special characters
    let bead_id = create_test_bead("Special characters test");

    // Add labels with various special characters
    bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("high-priority")
        .arg("--label")
        .arg("needs-review")
        .arg("--label")
        .arg("API:breaking")
        .output()
        .expect("Failed to add labels with special characters");

    // Verify all labels were stored correctly
    let labels = get_labels(&bead_id);
    assert_eq!(labels.len(), 3, "Should have 3 labels with special characters");

    let label_set: HashSet<_> = labels.into_iter().collect();
    assert!(label_set.contains("high-priority"), "Missing 'high-priority' label");
    assert!(label_set.contains("needs-review"), "Missing 'needs-review' label");
    assert!(label_set.contains("API:breaking"), "Missing 'API:breaking' label");

    // Clean up
    bf().arg("close").arg(&bead_id).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead");
}

#[test]
fn test_label_phase_labels() {
    // Test phase labels that map to plan sections
    let bead_id = create_test_bead("Phase labels test");

    // Add various phase labels
    bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("phase-1")
        .arg("--label")
        .arg("phase-2")
        .arg("--label")
        .arg("phase-3")
        .output()
        .expect("Failed to add phase labels");

    // Verify phase labels
    let labels = get_labels(&bead_id);
    assert_eq!(labels.len(), 3, "Should have 3 phase labels");

    let label_set: HashSet<_> = labels.into_iter().collect();
    assert!(label_set.contains("phase-1"), "Missing 'phase-1' label");
    assert!(label_set.contains("phase-2"), "Missing 'phase-2' label");
    assert!(label_set.contains("phase-3"), "Missing 'phase-3' label");

    // Clean up
    bf().arg("close").arg(&bead_id).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead");
}

#[test]
fn test_label_all_unique() {
    // Test that label list shows unique labels across all beads
    let bead1 = create_test_bead("Label uniqueness test 1");
    let bead2 = create_test_bead("Label uniqueness test 2");

    // Add overlapping labels to different beads
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

    // List all unique labels
    let output = bf()
        .arg("label")
        .arg("list")
        .output()
        .expect("Failed to list all labels");

    assert!(output.status.success(), "Failed to list all labels: {}",
        String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Parse label names from output
    let labels: Vec<String> = stdout
        .lines()
        .filter(|line| !line.is_empty() && !line.contains("All labels:"))
        .map(|line| line.trim().split('(').next().unwrap().trim().to_string())
        .collect();

    // Verify we have the expected unique labels
    assert!(labels.len() >= 3, "Should have at least 3 unique labels: {:?}", labels);
    assert!(labels.contains(&"urgent".to_string()), "Missing 'urgent' label");
    assert!(labels.contains(&"backend".to_string()), "Missing 'backend' label");
    assert!(labels.contains(&"frontend".to_string()), "Missing 'frontend' label");

    // Clean up
    bf().arg("close").arg(&bead1).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead 1");
    bf().arg("close").arg(&bead2).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead 2");
}

#[test]
fn test_label_unicode() {
    // Test labels with unicode characters
    let bead_id = create_test_bead("Unicode labels test");

    // Add labels with unicode characters
    bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("🐛-bug")
        .arg("--label")
        .arg("高优先级")
        .arg("--label")
        .arg("critical")
        .output()
        .expect("Failed to add unicode labels");

    // Verify unicode labels were stored correctly
    let labels = get_labels(&bead_id);
    assert_eq!(labels.len(), 3, "Should have 3 unicode labels");

    let label_set: HashSet<_> = labels.into_iter().collect();
    assert!(label_set.contains("🐛-bug"), "Missing emoji label");
    assert!(label_set.contains("高优先级"), "Missing chinese label");
    assert!(label_set.contains("critical"), "Missing 'critical' label");

    // Clean up
    bf().arg("close").arg(&bead_id).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead");
}

#[test]
fn test_label_empty_operations() {
    // Test operations on beads with no labels
    let bead_id = create_test_bead("Empty operations test");

    // Try to list labels when none exist
    let labels = get_labels(&bead_id);
    assert_eq!(labels.len(), 0, "New bead should have no labels");

    // Try to remove a label when none exist
    let output = bf()
        .arg("label")
        .arg("remove")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .output()
        .expect("Failed to attempt removal");

    assert!(output.status.success(), "Removal from empty label list should succeed");

    // Verify still no labels
    let labels = get_labels(&bead_id);
    assert_eq!(labels.len(), 0, "Should still have no labels");

    // Clean up
    bf().arg("close").arg(&bead_id).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead");
}

#[test]
fn test_label_persistence() {
    // Test that labels persist across bead updates
    let bead_id = create_test_bead("Label persistence test");

    // Add labels
    bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .arg("--label")
        .arg("backend")
        .output()
        .expect("Failed to add labels");

    // Update bead title
    let update_output = bf()
        .arg("update")
        .arg(&bead_id)
        .arg("--title")
        .arg("Updated title")
        .output()
        .expect("Failed to update bead");

    assert!(update_output.status.success(), "Failed to update bead: {}",
        String::from_utf8_lossy(&update_output.stderr));

    // Verify labels still exist after update
    let labels = get_labels(&bead_id);
    assert_eq!(labels.len(), 2, "Labels should persist after update");

    let label_set: HashSet<_> = labels.into_iter().collect();
    assert!(label_set.contains("urgent"), "'urgent' label should persist");
    assert!(label_set.contains("backend"), "'backend' label should persist");

    // Clean up
    bf().arg("close").arg(&bead_id).arg("--reason").arg("Test cleanup")
        .output().expect("Failed to close bead");
}
