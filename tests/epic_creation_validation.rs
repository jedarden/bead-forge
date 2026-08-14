// Epic Creation Validation Tests
//
// This module tests the validation of epic creation operations, ensuring that:
// - Epic issues can be created with valid attributes
// - Epic type is correctly set and persisted
// - Epic priority validation works correctly
// - Epic creation through CLI and storage both work as expected
// - Edge cases and error conditions are properly handled

use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;

/// Helper function to create a basic epic for testing
fn create_test_epic(id: &str, title: &str) -> Issue {
    Issue {
        id: id.to_string(),
        title: title.to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        description: Some(format!("Test epic: {}", title)),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    }
}

#[test]
fn test_epic_creation_basic() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let epic = create_test_epic("test-epic-basic", "Basic Epic Creation");
    storage.create_issue(&epic).unwrap();

    let retrieved = storage.get_issue("test-epic-basic").unwrap().unwrap();
    assert_eq!(retrieved.id, "test-epic-basic");
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.title, "Basic Epic Creation");
}

#[test]
fn test_epic_creation_with_priority() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Test creating epics with different priority levels
    let priorities = vec![
        (Priority::CRITICAL, "epic-p0"),
        (Priority::HIGH, "epic-p1"),
        (Priority::MEDIUM, "epic-p2"),
        (Priority::LOW, "epic-p3"),
        (Priority::BACKLOG, "epic-p4"),
    ];

    for (priority, id_suffix) in priorities {
        let mut epic = create_test_epic(
            &format!("test-{}", id_suffix),
            &format!("Priority Epic {:?}", priority),
        );
        epic.priority = priority;
        storage.create_issue(&epic).unwrap();

        let retrieved = storage
            .get_issue(&format!("test-{}", id_suffix))
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.priority, priority);
        assert_eq!(retrieved.issue_type, IssueType::Epic);
    }
}

#[test]
fn test_epic_creation_with_description() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let mut epic = create_test_epic("test-epic-desc", "Epic with Description");
    epic.description = Some("This is a detailed epic description for testing".to_string());
    storage.create_issue(&epic).unwrap();

    let retrieved = storage.get_issue("test-epic-desc").unwrap().unwrap();
    assert_eq!(
        retrieved.description,
        Some("This is a detailed epic description for testing".to_string())
    );
}

#[test]
fn test_epic_creation_with_assignee() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let mut epic = create_test_epic("test-epic-assignee", "Epic with Assignee");
    epic.assignee = Some("test-worker".to_string());
    storage.create_issue(&epic).unwrap();

    let retrieved = storage.get_issue("test-epic-assignee").unwrap().unwrap();
    assert_eq!(retrieved.assignee, Some("test-worker".to_string()));
}

#[test]
fn test_epic_creation_serialization() {
    let epic = create_test_epic("test-epic-serialize", "Serialization Test Epic");

    let json = serde_json::to_string(&epic).unwrap();
    assert!(json.contains("\"issue_type\":\"epic\""));

    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.id, "test-epic-serialize");
}

#[test]
fn test_multiple_epic_creation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple epics
    for i in 1..=5 {
        let epic = create_test_epic(&format!("multi-epic-{}", i), &format!("Multi Epic {}", i));
        storage.create_issue(&epic).unwrap();
    }

    // Verify all were created
    let all_issues = storage.list_issues(&Default::default()).unwrap();
    let epic_count = all_issues
        .iter()
        .filter(|i| i.issue_type == IssueType::Epic)
        .count();
    assert_eq!(epic_count, 5);
}

#[test]
fn test_epic_creation_id_uniqueness() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let epic1 = create_test_epic("duplicate-id", "First Epic");
    storage.create_issue(&epic1).unwrap();

    // Creating another epic with same ID should fail or replace
    let epic2 = create_test_epic("duplicate-id", "Second Epic");
    let result = storage.create_issue(&epic2);

    // The behavior depends on the storage implementation
    // Either it should fail with an error or replace the existing epic
    match result {
        Ok(_) => {
            // If it succeeded, verify it was replaced
            let retrieved = storage.get_issue("duplicate-id").unwrap().unwrap();
            assert_eq!(retrieved.title, "Second Epic");
        }
        Err(_) => {
            // If it failed, verify original epic still exists
            let retrieved = storage.get_issue("duplicate-id").unwrap().unwrap();
            assert_eq!(retrieved.title, "First Epic");
        }
    }
}

#[test]
fn test_epic_type_preservation() {
    // Test that epic type is preserved through serialization and storage operations
    let epic = create_test_epic("type-preservation", "Type Preservation Epic");

    // Serialize and deserialize
    let json = serde_json::to_string(&epic).unwrap();
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Epic);

    // Store and retrieve
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();
    storage.create_issue(&epic).unwrap();

    let retrieved = storage.get_issue("type-preservation").unwrap().unwrap();
    assert_eq!(retrieved.issue_type, IssueType::Epic);
}

#[test]
fn test_negative_priority_rejected() {
    // Test that creating an epic with negative priority (-1) is rejected
    // This validates that the priority validation function correctly rejects
    // invalid priority values outside the valid range of 0-4

    use bead_forge::validation::validate_priority;

    // Test that negative priority is rejected
    let result = validate_priority(-1);
    assert!(result.is_invalid(), "Priority -1 should be rejected as invalid");

    // Verify the error message contains useful information
    let error_msg = result.to_string();
    assert!(error_msg.contains("Invalid priority"), "Error should mention 'Invalid priority'");
    assert!(error_msg.contains("-1"), "Error should include the invalid value");

    // Test that the validation result can be converted to a Result error
    let result_as_result = result.to_result();
    assert!(result_as_result.is_err(), "to_result() should return Err for invalid priority");

    // Additional verification: test that the entire negative range is rejected
    let negative_values = vec![-1, -2, -10, -100];
    for priority in negative_values {
        let result = validate_priority(priority);
        assert!(
            result.is_invalid(),
            "Priority {} should be rejected as invalid",
            priority
        );
    }
}
