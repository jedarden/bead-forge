// Test P1 (High Priority) Epic Creation
// Tests creating epics with P1 (high) priority, verifying storage and serialization

use bead_forge::model::{Issue, IssueType, Status, Priority};
use bead_forge::storage::Storage;
use chrono::Utc;

#[test]
fn test_p1_epic_creation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with P1 (high) priority
    let epic = Issue {
        id: "epic-p1-test".to_string(),
        title: "High Priority Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH, // P1 = 1
        description: Some("This is a high priority epic".to_string()),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Verify the epic was stored correctly
    let retrieved = storage.get_issue("epic-p1-test").unwrap().unwrap();

    // Test 1: Verify ID matches
    assert_eq!(retrieved.id, "epic-p1-test");

    // Test 2: Verify issue type is epic
    assert_eq!(retrieved.issue_type, IssueType::Epic);

    // Test 3: Verify priority is P1 (high = 1)
    assert_eq!(retrieved.priority, Priority::HIGH);
    assert_eq!(retrieved.priority.0, 1);

    // Test 4: Verify status
    assert_eq!(retrieved.status, Status::Open);

    // Test 5: Verify description is preserved
    assert_eq!(retrieved.description, Some("This is a high priority epic".to_string()));
}

#[test]
fn test_p1_epic_serialization() {
    // Create epic with P1 priority
    let epic = Issue {
        id: "epic-p1-serialize".to_string(),
        title: "P1 Epic Serialization Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        ..Default::default()
    };

    // Test JSON serialization
    let json = serde_json::to_string(&epic).unwrap();

    // Verify epic type is serialized correctly
    assert!(json.contains("\"issue_type\":\"epic\""));

    // Verify P1 priority is serialized as 1
    assert!(json.contains("\"priority\":1"));

    // Test deserialization
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.priority, Priority::HIGH);
    assert_eq!(deserialized.priority.0, 1);
    assert_eq!(deserialized.id, "epic-p1-serialize");
}

#[test]
fn test_p1_priority_value() {
    // Test that Priority::HIGH is indeed P1 (value 1)
    assert_eq!(Priority::HIGH.0, 1);

    // Test that it compares correctly
    assert_eq!(Priority::HIGH, Priority(1));
    assert_ne!(Priority::HIGH, Priority::CRITICAL);
    assert_ne!(Priority::HIGH, Priority::MEDIUM);

    // Test that it's between P0 and P2
    assert!(Priority::CRITICAL < Priority::HIGH);
    assert!(Priority::HIGH < Priority::MEDIUM);
    assert!(Priority::HIGH < Priority::LOW);
    assert!(Priority::HIGH < Priority::BACKLOG);
}

#[test]
fn test_p1_epic_with_full_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create comprehensive P1 epic with all fields
    let mut epic = Issue {
        id: "epic-p1-full".to_string(),
        title: "Complete P1 Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some("Full metadata test".to_string()),
        assignee: Some("test-worker".to_string()),
        ..Default::default()
    };
    epic.created_at = Utc::now();
    epic.updated_at = Utc::now();

    storage.create_issue(&epic).unwrap();

    // Retrieve and verify all fields
    let retrieved = storage.get_issue("epic-p1-full").unwrap().unwrap();

    assert_eq!(retrieved.id, "epic-p1-full");
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.priority, Priority::HIGH);
    assert_eq!(retrieved.priority.0, 1);
    assert_eq!(retrieved.description, Some("Full metadata test".to_string()));
    assert_eq!(retrieved.assignee, Some("test-worker".to_string()));
}

#[test]
fn test_p1_epic_display_formatting() {
    // Test priority display formatting for P1
    let p1 = Priority::HIGH;
    let display = format!("{}", p1);
    assert_eq!(display, "P1");

    // Create epic and test full display
    let epic = Issue {
        id: "epic-p1-display".to_string(),
        title: "P1 Display Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        ..Default::default()
    };

    // Verify priority displays as P1
    assert_eq!(format!("{}", epic.priority), "P1");
}

#[test]
fn test_multiple_p1_epics() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple P1 epics
    for i in 1..=3 {
        let epic = Issue {
            id: format!("epic-p1-{}", i),
            title: format!("P1 Epic {}", i),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority: Priority::HIGH,
            ..Default::default()
        };
        storage.create_issue(&epic).unwrap();
    }

    // Verify all were stored with correct priority
    let all_issues = storage.list_issues(&Default::default()).unwrap();
    let p1_epics: Vec<_> = all_issues.iter()
        .filter(|i| i.issue_type == IssueType::Epic && i.priority == Priority::HIGH)
        .collect();

    assert_eq!(p1_epics.len(), 3);

    // Verify each has priority 1
    for epic in p1_epics {
        assert_eq!(epic.priority.0, 1);
        assert_eq!(epic.issue_type, IssueType::Epic);
    }
}

#[test]
fn test_p1_vs_other_priorities() {
    // Test P1 against other priority levels
    let priorities = vec![
        (Priority::CRITICAL, 0, "P0"),
        (Priority::HIGH, 1, "P1"),
        (Priority::MEDIUM, 2, "P2"),
        (Priority::LOW, 3, "P3"),
        (Priority::BACKLOG, 4, "P4"),
    ];

    for (priority, value, expected_display) in priorities {
        assert_eq!(priority.0, value);
        assert_eq!(format!("{}", priority), expected_display);
    }

    // Verify ordering: P0 < P1 < P2 < P3 < P4
    assert!(Priority::CRITICAL < Priority::HIGH);
    assert!(Priority::HIGH < Priority::MEDIUM);
    assert!(Priority::MEDIUM < Priority::LOW);
    assert!(Priority::LOW < Priority::BACKLOG);
}

#[test]
fn test_p1_epic_json_roundtrip() {
    // Create epic with P1 priority
    let original = Issue {
        id: "epic-p1-roundtrip".to_string(),
        title: "P1 Roundtrip Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some("Test description".to_string()),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&original).unwrap();

    // Verify JSON contains correct values (note: to_string_pretty adds spaces)
    assert!(json.contains("\"epic-p1-roundtrip\""));
    assert!(json.contains("\"issue_type\": \"epic\""));
    assert!(json.contains("\"priority\": 1"));
    assert!(json.contains("Test description"));

    // Deserialize back
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    // Verify all fields match
    assert_eq!(deserialized.id, original.id);
    assert_eq!(deserialized.issue_type, original.issue_type);
    assert_eq!(deserialized.priority, original.priority);
    assert_eq!(deserialized.priority.0, 1);
    assert_eq!(deserialized.description, original.description);
    assert_eq!(deserialized.status, original.status);
}

#[test]
fn test_p1_priority_from_string() {
    // Test parsing P1 from various string formats
    let p1_from_p1_str = "P1".parse::<Priority>().unwrap();
    assert_eq!(p1_from_p1_str, Priority::HIGH, "Parsing 'P1' should give HIGH");

    let p1_from_1_str = "1".parse::<Priority>().unwrap();
    assert_eq!(p1_from_1_str, Priority::HIGH, "Parsing '1' should give HIGH");

    let p1_from_p1_lowercase = "p1".parse::<Priority>().unwrap();
    assert_eq!(p1_from_p1_lowercase, Priority::HIGH, "Parsing 'p1' should give HIGH (case insensitive)");

    // Test with whitespace
    let p1_from_whitespace = "  P1  ".parse::<Priority>().unwrap();
    assert_eq!(p1_from_whitespace, Priority::HIGH, "Parsing '  P1  ' should give HIGH (whitespace trimmed)");
}

#[test]
fn test_p1_priority_ordering() {
    // Test that P1 is correctly ordered relative to other priorities
    let p0 = Priority::CRITICAL;
    let p1 = Priority::HIGH;
    let p2 = Priority::MEDIUM;
    let p3 = Priority::LOW;
    let p4 = Priority::BACKLOG;

    // P1 should be greater than P0 (lower priority)
    assert!(p1 > p0, "P1 should be greater than P0 (lower priority)");
    assert!(p1 >= p0);

    // P1 should be less than P2, P3, P4 (higher priority)
    assert!(p1 < p2, "P1 should be less than P2 (higher priority)");
    assert!(p1 < p3, "P1 should be less than P3 (higher priority)");
    assert!(p1 < p4, "P1 should be less than P4 (higher priority)");
    assert!(p1 <= p2);
    assert!(p1 <= p3);
    assert!(p1 <= p4);

    // P1 should equal itself
    assert_eq!(p1, p1);
    assert!(p1 <= p1);
    assert!(p1 >= p1);
}

#[test]
fn test_p1_epic_with_different_statuses() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P1 epics with different statuses
    let statuses = vec![
        Status::Open,
        Status::InProgress,
        Status::Blocked,
        Status::Deferred,
    ];

    for (i, status) in statuses.iter().enumerate() {
        let epic = Issue {
            id: format!("epic-p1-status-{}", i),
            title: format!("P1 Epic with {:?}", status),
            issue_type: IssueType::Epic,
            status: status.clone(),
            priority: Priority::HIGH,
            ..Default::default()
        };
        storage.create_issue(&epic).unwrap();
    }

    // Verify all were stored with P1 priority
    let all_issues = storage.list_issues(&Default::default()).unwrap();
    let p1_epics: Vec<_> = all_issues.iter()
        .filter(|i| i.issue_type == IssueType::Epic && i.priority == Priority::HIGH)
        .collect();

    assert_eq!(p1_epics.len(), 4);

    // Verify each has priority 1
    for epic in p1_epics {
        assert_eq!(epic.priority.0, 1);
        assert_eq!(epic.issue_type, IssueType::Epic);
    }
}

#[test]
fn test_p1_epic_with_children() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P1 epic
    let epic = Issue {
        id: "epic-p1-with-children".to_string(),
        title: "P1 Epic with Children".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some("P1 epic with child tasks".to_string()),
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create child tasks
    for i in 1..=3 {
        let child_id = format!("child-{}", i);
        let child = Issue {
            id: child_id.clone(),
            title: format!("Child task {}", i),
            issue_type: IssueType::Task,
            status: Status::Open,
            priority: Priority::HIGH, // Same priority as epic
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();

        // Add dependency relationship (child depends on epic)
        storage.add_dependency(
            &child_id,
            &epic.id,
            &bead_forge::model::DependencyType::ParentChild,
            "test"
        ).unwrap();
    }

    // Retrieve epic and verify
    let retrieved = storage.get_issue("epic-p1-with-children").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::HIGH);
    assert_eq!(retrieved.issue_type, IssueType::Epic);
}
