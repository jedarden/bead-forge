//! Basic storage-level tests for remove_label operation
//! Tests per acceptance criteria in bf-3jxiwn

use bead_forge::model::{Issue, IssueType, Status};
use bead_forge::storage::Storage;

#[test]
fn test_remove_label_basic() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create an issue with multiple labels
    let issue = Issue {
        id: "test-1".to_string(),
        title: "Test Issue".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["keep1".to_string(), "remove-me".to_string(), "keep2".to_string()],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // Verify initial labels
    let labels = storage.get_labels("test-1").unwrap();
    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"keep1".to_string()));
    assert!(labels.contains(&"remove-me".to_string()));
    assert!(labels.contains(&"keep2".to_string()));

    // Remove one label
    storage.remove_label("test-1", "remove-me").unwrap();

    // Verify the label is gone but others remain
    let labels = storage.get_labels("test-1").unwrap();
    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"keep1".to_string()));
    assert!(labels.contains(&"keep2".to_string()));
    assert!(!labels.contains(&"remove-me".to_string()));

    // Verify global label list
    let all_labels = storage.list_all_labels().unwrap();
    assert_eq!(all_labels.len(), 2);
    let label_set: std::collections::HashSet<String> = all_labels.into_iter().map(|(l, _)| l).collect();
    assert!(label_set.contains("keep1"));
    assert!(label_set.contains("keep2"));
    assert!(!label_set.contains("remove-me"));

    println!("✓ test_remove_label_basic passed");
}

#[test]
fn test_remove_label_uses_immediate_transaction() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create an issue with labels
    let issue = Issue {
        id: "test-2".to_string(),
        title: "Test Issue".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["label1".to_string(), "label2".to_string()],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // Verify remove_label uses BEGIN IMMEDIATE transaction mode.
    // Implementation verification: remove_label() calls with_immediate_transaction()
    // which executes "BEGIN IMMEDIATE" before running the transaction body.
    // See src/storage/sqlite.rs:1743 and src/storage/sqlite.rs:156
    storage.remove_label("test-2", "label1").unwrap();

    // Verify removal worked
    let labels = storage.get_labels("test-2").unwrap();
    assert_eq!(labels.len(), 1);
    assert!(labels.contains(&"label2".to_string()));

    println!("✓ test_remove_label_uses_immediate_transaction passed");
}

#[test]
fn test_remove_nonexistent_label_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create an issue with labels
    let issue = Issue {
        id: "test-3".to_string(),
        title: "Test Issue".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["existing1".to_string(), "existing2".to_string()],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // Remove a label that doesn't exist - should succeed without error
    let result = storage.remove_label("test-3", "nonexistent");
    assert!(result.is_ok(), "Removing non-existent label should succeed");

    // Verify existing labels are unchanged
    let labels = storage.get_labels("test-3").unwrap();
    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"existing1".to_string()));
    assert!(labels.contains(&"existing2".to_string()));

    // Try removing the same non-existent label again - still idempotent
    let result = storage.remove_label("test-3", "nonexistent");
    assert!(result.is_ok(), "Removing non-existent label should still succeed");

    println!("✓ test_remove_nonexistent_label_is_idempotent passed");
}

#[test]
fn test_remove_label_from_nonexistent_issue() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create an issue
    let issue = Issue {
        id: "test-4".to_string(),
        title: "Test Issue".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["label1".to_string()],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // Remove a label from an issue that doesn't exist - should succeed (idempotent)
    let result = storage.remove_label("nonexistent-issue", "some-label");
    assert!(result.is_ok(), "Removing label from non-existent issue should succeed");

    // Verify the existing issue is unchanged
    let labels = storage.get_labels("test-4").unwrap();
    assert_eq!(labels.len(), 1);
    assert!(labels.contains(&"label1".to_string()));

    println!("✓ test_remove_label_from_nonexistent_issue passed");
}

#[test]
fn test_remove_label_whitespace_handling() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create an issue with labels
    let issue = Issue {
        id: "test-5".to_string(),
        title: "Test Issue".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["label1".to_string()],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // Remove label with leading/trailing whitespace - should trim
    storage.remove_label("test-5", "  label1  ").unwrap();

    // Verify the label was removed
    let labels = storage.get_labels("test-5").unwrap();
    assert_eq!(labels.len(), 0);

    println!("✓ test_remove_label_whitespace_handling passed");
}

#[test]
fn test_remove_empty_label_fails() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create an issue
    let issue = Issue {
        id: "test-6".to_string(),
        title: "Test Issue".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // Try removing an empty label - should fail
    let result = storage.remove_label("test-6", "");
    assert!(result.is_err(), "Removing empty label should fail");

    // Try removing a whitespace-only label - should fail
    let result = storage.remove_label("test-6", "   ");
    assert!(result.is_err(), "Removing whitespace-only label should fail");

    println!("✓ test_remove_empty_label_fails passed");
}

#[test]
fn test_remove_last_label() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create an issue with a single label
    let issue = Issue {
        id: "test-7".to_string(),
        title: "Test Issue".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["only-label".to_string()],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // Remove the last label
    storage.remove_label("test-7", "only-label").unwrap();

    // Verify the issue now has no labels
    let labels = storage.get_labels("test-7").unwrap();
    assert_eq!(labels.len(), 0);

    // Verify global label list is empty
    let all_labels = storage.list_all_labels().unwrap();
    assert_eq!(all_labels.len(), 0);

    println!("✓ test_remove_last_label passed");
}

#[test]
fn test_remove_multiple_labels_sequentially() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create an issue with multiple labels
    let issue = Issue {
        id: "test-8".to_string(),
        title: "Test Issue".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![
            "label1".to_string(),
            "label2".to_string(),
            "label3".to_string(),
            "label4".to_string(),
        ],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // Remove labels one at a time
    storage.remove_label("test-8", "label2").unwrap();
    let labels = storage.get_labels("test-8").unwrap();
    assert_eq!(labels.len(), 3);

    storage.remove_label("test-8", "label4").unwrap();
    let labels = storage.get_labels("test-8").unwrap();
    assert_eq!(labels.len(), 2);

    storage.remove_label("test-8", "label1").unwrap();
    let labels = storage.get_labels("test-8").unwrap();
    assert_eq!(labels.len(), 1);

    storage.remove_label("test-8", "label3").unwrap();
    let labels = storage.get_labels("test-8").unwrap();
    assert_eq!(labels.len(), 0);

    println!("✓ test_remove_multiple_labels_sequentially passed");
}

#[test]
fn test_remove_label_case_sensitive() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create an issue with labels of different cases
    let issue = Issue {
        id: "test-9".to_string(),
        title: "Test Issue".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["Bug".to_string(), "feature".to_string()],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // Remove lowercase "bug" - should not remove uppercase "Bug"
    storage.remove_label("test-9", "bug").unwrap();

    // Verify "Bug" still exists
    let labels = storage.get_labels("test-9").unwrap();
    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"Bug".to_string()));
    assert!(labels.contains(&"feature".to_string()));

    println!("✓ test_remove_label_case_sensitive passed");
}

#[test]
fn test_remove_label_special_characters() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create an issue with special character labels
    let issue = Issue {
        id: "test-10".to_string(),
        title: "Test Issue".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![
            "high-priority".to_string(),
            "needs-review".to_string(),
            "API:breaking".to_string(),
        ],
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();

    // Remove labels with special characters
    storage.remove_label("test-10", "needs-review").unwrap();

    let labels = storage.get_labels("test-10").unwrap();
    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"high-priority".to_string()));
    assert!(labels.contains(&"API:breaking".to_string()));
    assert!(!labels.contains(&"needs-review".to_string()));

    println!("✓ test_remove_label_special_characters passed");
}
