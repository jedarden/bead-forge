// Test epic P1 (high priority) creation
// This test verifies that epics can be created with P1 (high) priority
// and that the priority is correctly preserved through serialization, storage, and retrieval

use bead_forge::model::{Issue, IssueType, Status, Priority};
use bead_forge::storage::Storage;

#[test]
fn test_epic_p1_high_creation() {
    // Test creating an epic with P1 (high) priority
    let epic = Issue {
        id: "epic-p1-test".to_string(),
        title: "P1 High Priority Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH, // P1 = high priority
        ..Default::default()
    };

    // Verify priority is P1
    assert_eq!(epic.priority, Priority::HIGH);
    assert_eq!(epic.priority.0, 1);

    // Test JSON serialization preserves P1 priority
    let json = serde_json::to_string(&epic).unwrap();
    assert!(json.contains("\"priority\":1"));

    // Verify Display shows P1
    let priority_display = format!("{}", epic.priority);
    assert_eq!(priority_display, "P1");
}

#[test]
fn test_epic_p1_serialization_roundtrip() {
    // Test full JSON roundtrip for epic with P1 priority
    let epic = Issue {
        id: "epic-p1-serialize".to_string(),
        title: "P1 Epic Serialization Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some("This is a high priority epic".to_string()),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&epic).unwrap();

    // Verify JSON contains expected values
    assert!(json.contains("\"issue_type\":\"epic\""));
    assert!(json.contains("\"priority\":1"));
    assert!(json.contains("high"));

    // Deserialize back
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, "epic-p1-serialize");
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.priority, Priority::HIGH);
    assert_eq!(deserialized.priority.0, 1);
    assert_eq!(deserialized.status, Status::Open);
}

#[test]
fn test_epic_p1_storage_and_retrieval() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with P1 priority
    let epic = Issue {
        id: "epic-p1-storage".to_string(),
        title: "P1 Epic Storage Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some("High priority epic for storage test".to_string()),
        ..Default::default()
    };

    // Store the epic
    storage.create_issue(&epic).unwrap();

    // Retrieve the epic
    let retrieved = storage.get_issue("epic-p1-storage").unwrap().unwrap();

    // Verify all fields match
    assert_eq!(retrieved.id, "epic-p1-storage");
    assert_eq!(retrieved.title, "P1 Epic Storage Test");
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.status, Status::Open);
    assert_eq!(retrieved.priority, Priority::HIGH);
    assert_eq!(retrieved.priority.0, 1);
    assert_eq!(retrieved.description, epic.description);
}

#[test]
fn test_epic_p1_with_children() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P1 epic with children
    let epic = Issue {
        id: "epic-p1-children".to_string(),
        title: "P1 Epic with Children".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some("High priority epic with child tasks".to_string()),
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children tasks (also with various priorities)
    let child_priorities = vec![
        ("child-1", Priority::CRITICAL, "Critical priority child"),
        ("child-2", Priority::HIGH, "High priority child"),
        ("child-3", Priority::MEDIUM, "Medium priority child"),
    ];

    for (id, priority, title) in &child_priorities {
        let child = Issue {
            id: id.to_string(),
            title: title.to_string(),
            issue_type: IssueType::Task,
            status: Status::Open,
            priority: *priority,
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();

        use bead_forge::model::DependencyType;
        storage.add_dependency("epic-p1-children", id, &DependencyType::ParentChild, "test").unwrap();
    }

    // Verify epic has all children
    let children = storage.get_dependencies("epic-p1-children").unwrap();
    assert_eq!(children.len(), 3);

    // Verify epic priority remains P1 regardless of children's priorities
    let retrieved_epic = storage.get_issue("epic-p1-children").unwrap().unwrap();
    assert_eq!(retrieved_epic.priority, Priority::HIGH);
    assert_eq!(retrieved_epic.priority.0, 1);
}

#[test]
fn test_epic_p1_priority_ordering() {
    // Test that P1 epics have correct ordering relative to other priorities
    // P0 (CRITICAL=0) < P1 (HIGH=1) < P2 (MEDIUM=2) < P3 (LOW=3) < P4 (BACKLOG=4)

    // Verify P1 is greater than P0 (less urgent)
    assert!(Priority::HIGH > Priority::CRITICAL);

    // Verify P1 is less than P2 (more urgent)
    assert!(Priority::HIGH < Priority::MEDIUM);

    // Verify P1 is less than all lower priorities
    assert!(Priority::HIGH < Priority::LOW);
    assert!(Priority::HIGH < Priority::BACKLOG);

    // Specifically verify P1 has numerical value 1
    assert_eq!(Priority::HIGH.0, 1);
}

#[test]
fn test_epic_p1_from_str_parsing() {
    // Test parsing "P1" and "1" strings to Priority::HIGH
    use std::str::FromStr;

    let p1_from_str = Priority::from_str("P1").unwrap();
    assert_eq!(p1_from_str, Priority::HIGH);
    assert_eq!(p1_from_str.0, 1);

    let one_from_str = Priority::from_str("1").unwrap();
    assert_eq!(one_from_str, Priority::HIGH);

    // Test case insensitive
    let lowercase = Priority::from_str("p1").unwrap();
    assert_eq!(lowercase, Priority::HIGH);
}

#[test]
fn test_epic_p1_json_serialization_format() {
    // Test that P1 epic serializes to expected JSON format
    let epic = Issue {
        id: "epic-p1-json".to_string(),
        title: "P1 JSON Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        ..Default::default()
    };

    let json = serde_json::to_string_pretty(&epic).unwrap();

    // Verify JSON structure
    assert!(json.contains("\"id\": \"epic-p1-json\""));
    assert!(json.contains("\"issue_type\": \"epic\""));
    assert!(json.contains("\"priority\": 1"));
    assert!(json.contains("\"status\": \"open\""));

    // Verify no unexpected fields or values
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["priority"], 1);
    assert_eq!(parsed["issue_type"], "epic");
}

#[test]
fn test_epic_p1_with_multiple_statuses() {
    // Test that P1 epic can be created with various statuses
    let statuses = vec![
        (Status::Open, "open"),
        (Status::InProgress, "in_progress"),
        (Status::Blocked, "blocked"),
    ];

    for (status, status_str) in statuses {
        let epic = Issue {
            id: format!("epic-p1-{}", status_str),
            title: format!("P1 Epic with {} status", status_str),
            issue_type: IssueType::Epic,
            status,
            priority: Priority::HIGH,
            ..Default::default()
        };

        // Verify priority is P1 regardless of status
        assert_eq!(epic.priority, Priority::HIGH);
        assert_eq!(epic.priority.0, 1);

        // Verify JSON serialization preserves both status and priority
        let json = serde_json::to_string(&epic).unwrap();
        assert!(json.contains("\"priority\":1"));
        assert!(json.contains(&format!("\"status\":\"{}\"", status_str)));
    }
}

#[test]
fn test_epic_p1_compared_to_p0() {
    // Test that P1 and P0 are distinct and ordered correctly
    let p0_epic = Issue {
        id: "epic-p0".to_string(),
        title: "P0 Critical Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        ..Default::default()
    };

    let p1_epic = Issue {
        id: "epic-p1".to_string(),
        title: "P1 High Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        ..Default::default()
    };

    // Verify priorities are different
    assert_ne!(p0_epic.priority, p1_epic.priority);

    // Verify P0 is numerically less than P1 (more urgent)
    assert!(p0_epic.priority < p1_epic.priority);
    assert_eq!(p0_epic.priority.0, 0);
    assert_eq!(p1_epic.priority.0, 1);

    // Verify Display formats
    assert_eq!(format!("{}", p0_epic.priority), "P0");
    assert_eq!(format!("{}", p1_epic.priority), "P1");
}

#[test]
fn test_epic_p1_sync_equals() {
    // Test that P1 epics can be compared using sync_equals
    let mut epic1 = Issue {
        id: "epic-p1-sync".to_string(),
        title: "P1 Sync Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        ..Default::default()
    };

    let mut epic2 = epic1.clone();

    // Modify timestamps (should be ignored by sync_equals)
    epic2.created_at = chrono::Utc::now() + chrono::Duration::seconds(100);
    epic2.updated_at = chrono::Utc::now() + chrono::Duration::seconds(200);

    // Should still be equal according to sync_equals
    assert!(epic1.sync_equals(&epic2));

    // Change priority - should not be equal
    epic2.priority = Priority::MEDIUM;
    assert!(!epic1.sync_equals(&epic2));
}
