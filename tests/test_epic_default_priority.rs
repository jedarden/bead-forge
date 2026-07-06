// Test Epic Default Priority
// Tests that when creating an epic without specifying priority, it uses the default

use bead_forge::model::{Issue, IssueType, Status, Priority};

#[test]
fn test_epic_default_priority() {
    // Create an epic without explicitly setting priority
    let epic = Issue {
        id: "epic-default-test".to_string(),
        title: "Epic with Default Priority".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        // Note: priority field not set, should use Default::default()
        description: Some("Testing default priority for epic".to_string()),
        ..Default::default()
    };

    // Verify that the default priority is P2 (MEDIUM)
    assert_eq!(epic.priority, Priority::MEDIUM);
    assert_eq!(epic.priority.0, 2);

    // Verify epic type is set correctly
    assert_eq!(epic.issue_type, IssueType::Epic);

    // Verify other defaults
    assert_eq!(epic.status, Status::Open);
}

#[test]
fn test_epic_default_vs_explicit_priority() {
    // Compare epic with default priority vs explicit priority
    let epic_default = Issue {
        id: "epic-default".to_string(),
        title: "Epic Default Priority".to_string(),
        issue_type: IssueType::Epic,
        ..Default::default()
    };

    let epic_p1 = Issue {
        id: "epic-p1".to_string(),
        title: "Epic P1 Priority".to_string(),
        issue_type: IssueType::Epic,
        priority: Priority::HIGH,
        ..Default::default()
    };

    // Default should be P2, not P1
    assert_eq!(epic_default.priority, Priority::MEDIUM);
    assert_eq!(epic_default.priority.0, 2);

    // Explicit P1 should be P1
    assert_eq!(epic_p1.priority, Priority::HIGH);
    assert_eq!(epic_p1.priority.0, 1);

    // They should be different
    assert_ne!(epic_default.priority, epic_p1.priority);
}

#[test]
fn test_default_priority_is_medium() {
    // Test that Priority::default() is MEDIUM (P2)
    let default_priority = Priority::default();
    assert_eq!(default_priority, Priority::MEDIUM);
    assert_eq!(default_priority.0, 2);
}

#[test]
fn test_default_issue_type_is_task_not_epic() {
    // Test that IssueType::default() is Task, not Epic
    let default_type = IssueType::default();
    assert_eq!(default_type, IssueType::Task);
    assert_ne!(default_type, IssueType::Epic);
}

#[test]
fn test_epic_serialization_with_default_priority() {
    // Create epic with default priority
    let epic = Issue {
        id: "epic-serialize-default".to_string(),
        title: "Epic Default Priority Serialization".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        description: Some("Testing default priority serialization".to_string()),
        ..Default::default()
    };

    // Verify default priority is P2
    assert_eq!(epic.priority, Priority::MEDIUM);
    assert_eq!(epic.priority.0, 2);

    // Serialize to JSON
    let json = serde_json::to_string(&epic).unwrap();

    // Verify epic type is serialized
    assert!(json.contains("\"issue_type\":\"epic\""));

    // Verify default priority (2) is serialized
    assert!(json.contains("\"priority\":2"));

    // Deserialize back
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.priority, Priority::MEDIUM);
    assert_eq!(deserialized.priority.0, 2);
}

#[test]
fn test_all_priorities_exist_for_epics() {
    // Test that epic can have any priority level
    let priorities = vec![
        (Priority::CRITICAL, 0, "P0"),
        (Priority::HIGH, 1, "P1"),
        (Priority::MEDIUM, 2, "P2"),
        (Priority::LOW, 3, "P3"),
        (Priority::BACKLOG, 4, "P4"),
    ];

    for (i, (priority, value, display)) in priorities.iter().enumerate() {
        let epic = Issue {
            id: format!("epic-p{}-test", value),
            title: format!("Epic with {} priority", display),
            issue_type: IssueType::Epic,
            priority: *priority,
            ..Default::default()
        };

        // Verify priority is set correctly
        assert_eq!(epic.priority.0, *value);
        assert_eq!(format!("{}", epic.priority), *display);
        assert_eq!(epic.issue_type, IssueType::Epic);

        // Verify serialization
        let json = serde_json::to_string(&epic).unwrap();
        assert!(json.contains(&format!("\"priority\":{}", value)));
        assert!(json.contains("\"issue_type\":\"epic\""));

        // Verify roundtrip
        let deserialized: Issue = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.priority, *priority);
        assert_eq!(deserialized.issue_type, IssueType::Epic);

        println!("Test {} for {}: Epic has priority {} (display: {})", i + 1, display, value, format!("{}", epic.priority));
    }
}
