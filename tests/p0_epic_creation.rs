// Test P0 (Critical Priority) Epic Creation
// Tests creating epics with P0 (critical) priority, verifying storage and serialization

use bead_forge::model::{Issue, IssueType, Status, Priority};
use bead_forge::storage::Storage;
use chrono::Utc;

#[test]
fn test_p0_epic_creation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with P0 (critical) priority
    let epic = Issue {
        id: "epic-p0-test".to_string(),
        title: "Critical Priority Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL, // P0 = 0
        description: Some("This is a critical priority epic".to_string()),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Verify the epic was stored correctly
    let retrieved = storage.get_issue("epic-p0-test").unwrap().unwrap();

    // Test 1: Verify ID matches
    assert_eq!(retrieved.id, "epic-p0-test");

    // Test 2: Verify issue type is epic
    assert_eq!(retrieved.issue_type, IssueType::Epic);

    // Test 3: Verify priority is P0 (critical = 0)
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);

    // Test 4: Verify status
    assert_eq!(retrieved.status, Status::Open);

    // Test 5: Verify description is preserved
    assert_eq!(retrieved.description, Some("This is a critical priority epic".to_string()));
}

#[test]
fn test_p0_epic_serialization() {
    // Create epic with P0 priority
    let epic = Issue {
        id: "epic-p0-serialize".to_string(),
        title: "P0 Epic Serialization Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        ..Default::default()
    };

    // Test JSON serialization
    let json = serde_json::to_string(&epic).unwrap();

    // Verify epic type is serialized correctly
    assert!(json.contains("\"issue_type\":\"epic\""));

    // Verify P0 priority is serialized as 0
    assert!(json.contains("\"priority\":0"));

    // Test deserialization
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.priority.0, 0);
    assert_eq!(deserialized.id, "epic-p0-serialize");
}

#[test]
fn test_p0_priority_value() {
    // Test that Priority::CRITICAL is indeed P0 (value 0)
    assert_eq!(Priority::CRITICAL.0, 0);

    // Test that it compares correctly
    assert_eq!(Priority::CRITICAL, Priority(0));
    assert_ne!(Priority::CRITICAL, Priority::HIGH);

    // Test that it's the lowest value (highest priority)
    assert!(Priority::CRITICAL < Priority::HIGH);
    assert!(Priority::CRITICAL < Priority::MEDIUM);
    assert!(Priority::CRITICAL < Priority::LOW);
    assert!(Priority::CRITICAL < Priority::BACKLOG);
}

#[test]
fn test_p0_epic_with_full_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create comprehensive P0 epic with all fields
    let mut epic = Issue {
        id: "epic-p0-full".to_string(),
        title: "Complete P0 Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        description: Some("Full metadata test".to_string()),
        assignee: Some("test-worker".to_string()),
        ..Default::default()
    };
    epic.created_at = Utc::now();
    epic.updated_at = Utc::now();

    storage.create_issue(&epic).unwrap();

    // Retrieve and verify all fields
    let retrieved = storage.get_issue("epic-p0-full").unwrap().unwrap();

    assert_eq!(retrieved.id, "epic-p0-full");
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(retrieved.description, Some("Full metadata test".to_string()));
    assert_eq!(retrieved.assignee, Some("test-worker".to_string()));
}

#[test]
fn test_p0_epic_display_formatting() {
    // Test priority display formatting for P0
    let p0 = Priority::CRITICAL;
    let display = format!("{}", p0);
    assert_eq!(display, "P0");

    // Create epic and test full display
    let epic = Issue {
        id: "epic-p0-display".to_string(),
        title: "P0 Display Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        ..Default::default()
    };

    // Verify priority displays as P0
    assert_eq!(format!("{}", epic.priority), "P0");
}

#[test]
fn test_multiple_p0_epics() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple P0 epics
    for i in 1..=3 {
        let epic = Issue {
            id: format!("epic-p0-{}", i),
            title: format!("P0 Epic {}", i),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority: Priority::CRITICAL,
            ..Default::default()
        };
        storage.create_issue(&epic).unwrap();
    }

    // Verify all were stored with correct priority
    let all_issues = storage.list_issues(&Default::default()).unwrap();
    let p0_epics: Vec<_> = all_issues.iter()
        .filter(|i| i.issue_type == IssueType::Epic && i.priority == Priority::CRITICAL)
        .collect();

    assert_eq!(p0_epics.len(), 3);

    // Verify each has priority 0
    for epic in p0_epics {
        assert_eq!(epic.priority.0, 0);
        assert_eq!(epic.issue_type, IssueType::Epic);
    }
}

#[test]
fn test_p0_vs_other_priorities() {
    // Test P0 against other priority levels
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
fn test_p0_epic_json_roundtrip() {
    // Create epic with P0 priority
    let original = Issue {
        id: "epic-p0-roundtrip".to_string(),
        title: "P0 Roundtrip Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        description: Some("Test description".to_string()),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&original).unwrap();

    // Verify JSON contains correct values (note: to_string_pretty adds spaces)
    assert!(json.contains("\"epic-p0-roundtrip\""));
    assert!(json.contains("\"issue_type\": \"epic\""));
    assert!(json.contains("\"priority\": 0"));
    assert!(json.contains("Test description"));

    // Deserialize back
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    // Verify all fields match
    assert_eq!(deserialized.id, original.id);
    assert_eq!(deserialized.issue_type, original.issue_type);
    assert_eq!(deserialized.priority, original.priority);
    assert_eq!(deserialized.priority.0, 0);
    assert_eq!(deserialized.description, original.description);
    assert_eq!(deserialized.status, original.status);
}
