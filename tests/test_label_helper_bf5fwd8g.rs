//! Test the P0 label bead creation helper function
//!
//! This test verifies that the `create_bead_with_labels` helper function
//! in tests/common.rs works correctly for creating beads with custom labels.

mod common;
use common::TempWorkspace;

#[test]
fn test_create_bead_with_single_label() {
    let ws = TempWorkspace::new().unwrap();

    // Create a bead with a single label
    ws.create_bead_with_labels("bf-single-label", "Test bead with single label", &["critical"])
        .expect("Failed to create bead with label");

    // Verify the bead was created with the correct label
    let bead = ws.get_bead("bf-single-label").unwrap().unwrap();
    assert_eq!(bead.id, "bf-single-label");
    assert_eq!(bead.title, "Test bead with single label");
    assert_eq!(bead.labels, vec!["critical".to_string()]);
}

#[test]
fn test_create_bead_with_multiple_labels() {
    let ws = TempWorkspace::new().unwrap();

    // Create a bead with multiple labels
    ws.create_bead_with_labels(
        "bf-multi-label",
        "Test bead with multiple labels",
        &["bug", "critical", "backend"]
    )
    .expect("Failed to create bead with multiple labels");

    // Verify the bead was created with all labels
    let bead = ws.get_bead("bf-multi-label").unwrap().unwrap();
    assert_eq!(bead.id, "bf-multi-label");
    assert_eq!(bead.labels.len(), 3);
    assert!(bead.labels.contains(&"bug".to_string()));
    assert!(bead.labels.contains(&"critical".to_string()));
    assert!(bead.labels.contains(&"backend".to_string()));
}

#[test]
fn test_create_bead_with_empty_labels() {
    let ws = TempWorkspace::new().unwrap();

    // Create a bead with no labels
    ws.create_bead_with_labels("bf-no-labels", "Test bead with no labels", &[])
        .expect("Failed to create bead without labels");

    // Verify the bead was created with no labels
    let bead = ws.get_bead("bf-no-labels").unwrap().unwrap();
    assert_eq!(bead.id, "bf-no-labels");
    assert_eq!(bead.labels.len(), 0);
}

#[test]
fn test_create_multiple_beads_with_different_labels() {
    let ws = TempWorkspace::new().unwrap();

    // Create multiple beads with different label combinations
    ws.create_bead_with_labels("bf-bug-001", "Critical bug", &["bug", "p0"])
        .expect("Failed to create first bead");

    ws.create_bead_with_labels("bf-feature-001", "New feature", &["feature", "enhancement"])
        .expect("Failed to create second bead");

    ws.create_bead_with_labels("bf-task-001", "Documentation task", &["docs", "low-priority"])
        .expect("Failed to create third bead");

    // Verify all beads were created with correct labels
    let bug_bead = ws.get_bead("bf-bug-001").unwrap().unwrap();
    assert_eq!(bug_bead.labels, vec!["bug".to_string(), "p0".to_string()]);

    let feature_bead = ws.get_bead("bf-feature-001").unwrap().unwrap();
    assert_eq!(feature_bead.labels, vec!["feature".to_string(), "enhancement".to_string()]);

    let task_bead = ws.get_bead("bf-task-001").unwrap().unwrap();
    assert_eq!(task_bead.labels, vec!["docs".to_string(), "low-priority".to_string()]);
}

#[test]
fn test_label_helper_creates_default_task_type() {
    let ws = TempWorkspace::new().unwrap();

    // Create a bead with labels - should default to Task type
    ws.create_bead_with_labels("bf-default-task", "Default task bead", &["test"])
        .expect("Failed to create bead");

    // Verify it's a task type
    let bead = ws.get_bead("bf-default-task").unwrap().unwrap();
    assert_eq!(bead.issue_type, bead_forge::IssueType::Task);
    assert_eq!(bead.labels, vec!["test".to_string()]);
}
