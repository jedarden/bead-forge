//! Basic P0 Label Add Test Case
//!
//! Tests adding P0 label to issues using the LabelTestWorkspace fixtures.
//! Covers happy path and edge cases (duplicate add handling).

// Test fixtures are in the same tests directory, need to declare them as a module
mod label_test_fixtures;

use label_test_fixtures::{LabelTestWorkspace, assert_labels_eq, assert_has_label, assert_label_count};

#[test]
fn test_p0_label_add_happy_path() {
    // Create workspace and P0 epic without labels
    let ws = LabelTestWorkspace::new().unwrap();
    ws.create_p0_epic_with_labels("bf-p0-001", "Critical Epic", &[]).unwrap();

    // Add P0 label to the epic
    ws.add_label("bf-p0-001", "P0").unwrap();

    // Verify the label was added successfully
    assert_has_label("bf-p0-001", "P0", &ws).unwrap();
    assert_label_count("bf-p0-001", 1, &ws).unwrap();
}

#[test]
fn test_p0_label_add_duplicate_handling() {
    // Create workspace and P0 epic with P0 label already present
    let ws = LabelTestWorkspace::new().unwrap();
    ws.create_p0_epic_with_labels("bf-p0-002", "Critical Epic Duplicate", &["P0"]).unwrap();

    // Attempt to add P0 label again (should not create duplicate)
    let result = ws.add_label("bf-p0-002", "P0");
    assert!(result.is_ok(), "Adding duplicate label should succeed without error");

    // Verify only one P0 label exists (no duplicate)
    let labels = ws.get_labels("bf-p0-002").unwrap();
    let p0_count = labels.iter().filter(|l| *l == "P0").count();
    assert_eq!(p0_count, 1, "P0 label should appear exactly once");
    assert_label_count("bf-p0-002", 1, &ws).unwrap();
}

#[test]
fn test_p0_label_add_to_existing_labeled_bead() {
    // Create workspace and P0 epic with other labels
    let ws = LabelTestWorkspace::new().unwrap();
    ws.create_p0_epic_with_labels("bf-p0-003", "Critical Epic Mixed", &["urgent", "critical"]).unwrap();

    // Add P0 label to bead that already has other labels
    ws.add_label("bf-p0-003", "P0").unwrap();

    // Verify all labels are present
    assert_labels_eq("bf-p0-003", &["urgent", "critical", "P0"], &ws).unwrap();
    assert_label_count("bf-p0-003", 3, &ws).unwrap();
}

#[test]
fn test_p0_label_add_basic_task() {
    // Create workspace and basic task (not epic) with P0 priority
    let ws = LabelTestWorkspace::new().unwrap();
    let task = label_test_fixtures::LabelTestBeadBuilder::new("bf-task-001", "Critical Task")
        .with_priority(bead_forge::Priority::CRITICAL)
        .build();
    ws.storage().unwrap().create_issue(&task).unwrap();

    // Add P0 label to the task
    ws.add_label("bf-task-001", "P0").unwrap();

    // Verify the label was added
    assert_has_label("bf-task-001", "P0", &ws).unwrap();
    assert_label_count("bf-task-001", 1, &ws).unwrap();

    // Verify priority is still P0
    let retrieved = ws.get_bead("bf-task-001").unwrap().unwrap();
    assert_eq!(retrieved.priority, bead_forge::Priority::CRITICAL);
}

#[test]
fn test_p0_label_add_multiple_times_different_beads() {
    // Create workspace with multiple P0 beads
    let ws = LabelTestWorkspace::new().unwrap();
    ws.create_p0_epic_with_labels("bf-p0-multi-1", "Epic 1", &[]).unwrap();
    ws.create_p0_epic_with_labels("bf-p0-multi-2", "Epic 2", &[]).unwrap();
    ws.create_p0_epic_with_labels("bf-p0-multi-3", "Epic 3", &[]).unwrap();

    // Add P0 label to each bead
    ws.add_label("bf-p0-multi-1", "P0").unwrap();
    ws.add_label("bf-p0-multi-2", "P0").unwrap();
    ws.add_label("bf-p0-multi-3", "P0").unwrap();

    // Verify each bead has the P0 label
    assert_has_label("bf-p0-multi-1", "P0", &ws).unwrap();
    assert_has_label("bf-p0-multi-2", "P0", &ws).unwrap();
    assert_has_label("bf-p0-multi-3", "P0", &ws).unwrap();
}
