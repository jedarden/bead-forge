// Test epic type creation
// This test verifies that epics can be created with the correct type
// and that the epic type is correctly preserved through serialization, storage, and retrieval

use bead_forge::model::{Issue, IssueType, Status, Priority};
use bead_forge::storage::Storage;

#[test]
fn test_epic_type_basic_creation() {
    // Test creating an epic with the correct type
    let epic = Issue {
        id: "epic-basic-test".to_string(),
        title: "Basic Epic Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        ..Default::default()
    };

    // Verify issue type is Epic
    assert_eq!(epic.issue_type, IssueType::Epic);

    // Verify it's not Task (the default)
    assert_ne!(epic.issue_type, IssueType::Task);

    // Verify the epic type string representation
    assert_eq!(epic.issue_type.as_str(), "epic");
}

#[test]
fn test_epic_type_serialization() {
    // Test that epic type serializes correctly to JSON
    let epic = Issue {
        id: "epic-serialize-test".to_string(),
        title: "Epic Serialization Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&epic).unwrap();

    // Verify JSON contains epic type as a string value
    assert!(json.contains(r#""issue_type":"epic""#));

    // Verify it's not serialized as a number (e.g., "issue_type":0)
    assert!(!json.contains(r#""issue_type":"0"#));
    assert!(!json.contains(r#""issue_type":1"#));
    assert!(!json.contains(r#""issue_type":2"#));
}

#[test]
fn test_epic_type_deserialization() {
    // Test that epic type can be deserialized from JSON
    let json = r#"{
        "id": "epic-deserialize-test",
        "title": "Epic Deserialization Test",
        "issue_type": "epic",
        "status": "open",
        "priority": 2,
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z"
    }"#;

    // Deserialize from JSON
    let epic: Issue = serde_json::from_str(json).unwrap();

    // Verify issue type is Epic
    assert_eq!(epic.issue_type, IssueType::Epic);
    assert_eq!(epic.id, "epic-deserialize-test");
    assert_eq!(epic.title, "Epic Deserialization Test");
}

#[test]
fn test_epic_type_roundtrip() {
    // Test full serialization roundtrip for epic type
    let original = Issue {
        id: "epic-roundtrip-test".to_string(),
        title: "Epic Roundtrip Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::LOW,
        description: Some("Testing epic type roundtrip".to_string()),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&original).unwrap();

    // Deserialize back
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    // Verify all fields match
    assert_eq!(deserialized.id, original.id);
    assert_eq!(deserialized.title, original.title);
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.status, original.status);
    assert_eq!(deserialized.priority, original.priority);
    assert_eq!(deserialized.description, original.description);
}

#[test]
fn test_epic_type_storage() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic
    let epic = Issue {
        id: "epic-storage-test".to_string(),
        title: "Epic Storage Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        ..Default::default()
    };

    // Store the epic
    storage.create_issue(&epic).unwrap();

    // Retrieve the epic
    let retrieved = storage.get_issue("epic-storage-test").unwrap().unwrap();

    // Verify issue type is preserved
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.id, "epic-storage-test");
    assert_eq!(retrieved.title, "Epic Storage Test");
}

#[test]
fn test_epic_type_vs_other_types() {
    // Test that epic type is distinct from other issue types
    let epic = Issue {
        id: "epic-vs-others".to_string(),
        title: "Epic vs Others".to_string(),
        issue_type: IssueType::Epic,
        ..Default::default()
    };

    assert_eq!(epic.issue_type, IssueType::Epic);
    assert_ne!(epic.issue_type, IssueType::Task);
    assert_ne!(epic.issue_type, IssueType::Bug);
    assert_ne!(epic.issue_type, IssueType::Feature);
    assert_ne!(epic.issue_type, IssueType::Chore);
    assert_ne!(epic.issue_type, IssueType::Docs);
    assert_ne!(epic.issue_type, IssueType::Question);
}

#[test]
fn test_epic_type_with_all_statuses() {
    // Test that epic type works with all possible statuses
    let statuses = vec![
        Status::Open,
        Status::InProgress,
        Status::Blocked,
        Status::Deferred,
        Status::Closed,
    ];

    for status in &statuses {
        let epic = Issue {
            id: format!("epic-status-{:?}", status),
            title: format!("Epic with {:?}", status),
            issue_type: IssueType::Epic,
            status: status.clone(),
            ..Default::default()
        };

        assert_eq!(epic.issue_type, IssueType::Epic);
        assert_eq!(epic.status, *status);
    }
}

#[test]
fn test_epic_type_with_all_priorities() {
    // Test that epic type works with all priority levels
    let priorities = vec![
        (Priority::CRITICAL, 0, "P0"),
        (Priority::HIGH, 1, "P1"),
        (Priority::MEDIUM, 2, "P2"),
        (Priority::LOW, 3, "P3"),
        (Priority::BACKLOG, 4, "P4"),
    ];

    for (priority, numeric_value, display_name) in priorities {
        let epic = Issue {
            id: format!("epic-priority-{}", display_name),
            title: format!("Epic with {}", display_name),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority,
            ..Default::default()
        };

        assert_eq!(epic.issue_type, IssueType::Epic);
        assert_eq!(epic.priority.0, numeric_value);
        assert_eq!(format!("{}", epic.priority), display_name);
    }
}

#[test]
fn test_multiple_epics_creation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple epics
    for i in 1..=5 {
        let epic = Issue {
            id: format!("epic-multi-{}", i),
            title: format!("Multi Epic {}", i),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority: Priority::MEDIUM,
            ..Default::default()
        };

        storage.create_issue(&epic).unwrap();
    }

    // Verify all epics have correct type
    for i in 1..=5 {
        let retrieved = storage.get_issue(&format!("epic-multi-{}", i)).unwrap().unwrap();
        assert_eq!(retrieved.issue_type, IssueType::Epic);
        assert_eq!(retrieved.title, format!("Multi Epic {}", i));
    }
}

#[test]
fn test_epic_type_equality() {
    // Test epic type equality and comparison
    let epic1 = Issue {
        id: "epic-equality-1".to_string(),
        title: "Epic Equality Test 1".to_string(),
        issue_type: IssueType::Epic,
        ..Default::default()
    };

    let epic2 = Issue {
        id: "epic-equality-2".to_string(),
        title: "Epic Equality Test 2".to_string(),
        issue_type: IssueType::Epic,
        ..Default::default()
    };

    // Both should have Epic type
    assert_eq!(epic1.issue_type, epic2.issue_type);
    assert_eq!(epic1.issue_type, IssueType::Epic);
    assert_eq!(epic2.issue_type, IssueType::Epic);
}

#[test]
fn test_epic_type_string_conversion() {
    // Test epic type string conversion methods
    let epic_type = IssueType::Epic;

    // Test as_str() method
    assert_eq!(epic_type.as_str(), "epic");

    // Test Display trait
    assert_eq!(format!("{}", epic_type), "epic");

    // Test serialization
    let serialized = serde_json::to_string(&epic_type).unwrap();
    assert_eq!(serialized, "\"epic\"");

    // Test deserialization
    let deserialized: IssueType = serde_json::from_str("\"epic\"").unwrap();
    assert_eq!(deserialized, IssueType::Epic);
}
