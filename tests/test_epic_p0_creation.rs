// Test epic P0 (critical priority) creation
// This test verifies that epics can be created with P0 (critical) priority
// and that the priority is correctly preserved through serialization, storage, and retrieval

use bead_forge::model::{Issue, IssueType, Status, Priority};
use bead_forge::storage::Storage;

#[test]
fn test_epic_p0_critical_creation() {
    // Test creating an epic with P0 (critical) priority
    let epic = Issue {
        id: "epic-p0-test".to_string(),
        title: "P0 Critical Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL, // P0 = critical priority
        ..Default::default()
    };

    // Verify priority is P0
    assert_eq!(epic.priority, Priority::CRITICAL);
    assert_eq!(epic.priority.0, 0);

    // Test JSON serialization preserves P0 priority
    let json = serde_json::to_string(&epic).unwrap();
    assert!(json.contains("\"priority\":0"));

    // Verify Display shows P0
    let priority_display = format!("{}", epic.priority);
    assert_eq!(priority_display, "P0");
}

#[test]
fn test_epic_p0_serialization_roundtrip() {
    // Test full JSON roundtrip for epic with P0 priority
    let epic = Issue {
        id: "epic-p0-serialize".to_string(),
        title: "P0 Epic Serialization Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        description: Some("This is a critical epic".to_string()),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&epic).unwrap();

    // Verify JSON contains expected values
    assert!(json.contains("\"issue_type\":\"epic\""));
    assert!(json.contains("\"priority\":0"));
    assert!(json.contains("critical"));

    // Deserialize back
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, "epic-p0-serialize");
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.priority.0, 0);
    assert_eq!(deserialized.status, Status::Open);
}

#[test]
fn test_epic_p0_storage_and_retrieval() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with P0 priority
    let epic = Issue {
        id: "epic-p0-storage".to_string(),
        title: "P0 Epic Storage Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        description: Some("Critical priority epic for storage test".to_string()),
        ..Default::default()
    };

    // Store the epic
    storage.create_issue(&epic).unwrap();

    // Retrieve the epic
    let retrieved = storage.get_issue("epic-p0-storage").unwrap().unwrap();

    // Verify all fields match
    assert_eq!(retrieved.id, "epic-p0-storage");
    assert_eq!(retrieved.title, "P0 Epic Storage Test");
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.status, Status::Open);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(retrieved.description, epic.description);
}

#[test]
fn test_epic_all_priority_levels() {
    // Test that epic can be created with all priority levels
    let priorities = vec![
        (Priority::CRITICAL, "P0"),
        (Priority::HIGH, "P1"),
        (Priority::MEDIUM, "P2"),
        (Priority::LOW, "P3"),
        (Priority::BACKLOG, "P4"),
    ];

    for (priority, expected_display) in priorities {
        let epic = Issue {
            id: format!("epic-{}", expected_display),
            title: format!("Epic with {} priority", expected_display),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority,
            ..Default::default()
        };

        // Verify Display format
        assert_eq!(format!("{}", epic.priority), expected_display);

        // Verify JSON serialization
        let json = serde_json::to_string(&epic).unwrap();
        assert!(json.contains(&format!("\"priority\":{}", priority.0)));

        // Verify roundtrip
        let deserialized: Issue = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.priority, priority);
    }
}

#[test]
fn test_epic_p0_with_children() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 epic with children
    let epic = Issue {
        id: "epic-p0-children".to_string(),
        title: "P0 Epic with Children".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        description: Some("Critical epic with child tasks".to_string()),
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children tasks (also with various priorities)
    let child_priorities = vec![
        ("child-1", Priority::HIGH, "High priority child"),
        ("child-2", Priority::MEDIUM, "Medium priority child"),
        ("child-3", Priority::CRITICAL, "Critical priority child"),
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
        storage.add_dependency("epic-p0-children", id, &DependencyType::ParentChild, "test").unwrap();
    }

    // Verify epic has all children
    let children = storage.get_dependencies("epic-p0-children").unwrap();
    assert_eq!(children.len(), 3);

    // Verify epic priority remains P0 regardless of children's priorities
    let retrieved_epic = storage.get_issue("epic-p0-children").unwrap().unwrap();
    assert_eq!(retrieved_epic.priority, Priority::CRITICAL);
    assert_eq!(retrieved_epic.priority.0, 0);
}

#[test]
fn test_epic_p0_priority_ordering() {
    // Test that P0 epics sort correctly by priority
    // Note: Priority uses numerical ordering where 0 < 1 < 2...
    // So CRITICAL (0) < HIGH (1) < MEDIUM (2) < LOW (3) < BACKLOG (4)
    let priorities = vec![
        Priority::CRITICAL,
        Priority::HIGH,
        Priority::MEDIUM,
        Priority::LOW,
        Priority::BACKLOG,
    ];

    // Verify numerical ordering: CRITICAL (0) < HIGH (1) < MEDIUM (2) < LOW (3) < BACKLOG (4)
    for i in 1..priorities.len() {
        assert!(priorities[i - 1] < priorities[i],
            "Priority ordering failed: {:?} should be < {:?}",
            priorities[i - 1],
            priorities[i]);
    }

    // Specifically verify P0 has lowest numerical value (highest priority)
    assert_eq!(Priority::CRITICAL.0, 0);
    assert!(Priority::CRITICAL < Priority::HIGH);
    assert!(Priority::CRITICAL < Priority::MEDIUM);
    assert!(Priority::CRITICAL < Priority::LOW);
    assert!(Priority::CRITICAL < Priority::BACKLOG);
}

#[test]
fn test_epic_p0_from_str_parsing() {
    // Test parsing "P0" and "0" strings to Priority::CRITICAL
    use std::str::FromStr;

    let p0_from_str = Priority::from_str("P0").unwrap();
    assert_eq!(p0_from_str, Priority::CRITICAL);
    assert_eq!(p0_from_str.0, 0);

    let zero_from_str = Priority::from_str("0").unwrap();
    assert_eq!(zero_from_str, Priority::CRITICAL);

    // Test case insensitive
    let lowercase = Priority::from_str("p0").unwrap();
    assert_eq!(lowercase, Priority::CRITICAL);
}

#[test]
fn test_epic_p0_json_serialization_format() {
    // Test that P0 epic serializes to expected JSON format
    let epic = Issue {
        id: "epic-p0-json".to_string(),
        title: "P0 JSON Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        ..Default::default()
    };

    let json = serde_json::to_string_pretty(&epic).unwrap();

    // Verify JSON structure
    assert!(json.contains("\"id\": \"epic-p0-json\""));
    assert!(json.contains("\"issue_type\": \"epic\""));
    assert!(json.contains("\"priority\": 0"));
    assert!(json.contains("\"status\": \"open\""));

    // Verify no unexpected fields or values
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["priority"], 0);
    assert_eq!(parsed["issue_type"], "epic");
}
