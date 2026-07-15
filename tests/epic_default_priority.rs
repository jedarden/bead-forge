// Test Epic Default Priority
// Tests that epics created without specifying a priority get the default P2 (Medium) priority

use bead_forge::model::{Issue, IssueType, Priority};
use bead_forge::storage::Storage;
use chrono::Utc;

#[test]
fn test_epic_default_priority_is_p2() {
    // Create an epic using Default::default() which should apply default priority
    let epic = Issue {
        id: "epic-default-test".to_string(),
        title: "Test Epic Default Priority".to_string(),
        issue_type: IssueType::Epic,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // Verify the default priority is P2 (Medium)
    assert_eq!(
        epic.priority,
        Priority::MEDIUM,
        "Epic should have P2 (Medium) priority by default"
    );
    assert_eq!(epic.priority.0, 2, "Epic priority value should be 2");
}

#[test]
fn test_epic_default_priority_storage() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create and store an epic with default priority
    let epic = Issue {
        id: "epic-storage-default".to_string(),
        title: "Epic Storage Default Priority".to_string(),
        issue_type: IssueType::Epic,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Retrieve and verify the default priority was preserved
    let retrieved = storage.get_issue("epic-storage-default").unwrap().unwrap();

    assert_eq!(
        retrieved.issue_type,
        IssueType::Epic,
        "Issue type should be Epic"
    );
    assert_eq!(
        retrieved.priority,
        Priority::MEDIUM,
        "Retrieved epic should have P2 priority"
    );
    assert_eq!(
        retrieved.priority.0, 2,
        "Retrieved epic priority value should be 2"
    );
}

#[test]
fn test_epic_default_priority_serialization() {
    // Create an epic with default priority
    let epic = Issue {
        id: "epic-serialize-default".to_string(),
        title: "Epic Serialization Default Priority".to_string(),
        issue_type: IssueType::Epic,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&epic).unwrap();

    // Verify epic type is serialized correctly
    assert!(
        json.contains(r#""issue_type":"epic""#),
        "JSON should contain epic type"
    );

    // Verify default P2 priority is serialized as 2
    assert!(
        json.contains(r#""priority":2"#),
        "JSON should contain priority: 2"
    );

    // Deserialize and verify
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.priority, Priority::MEDIUM);
    assert_eq!(deserialized.priority.0, 2);
}

#[test]
fn test_priority_default_impl_returns_p2() {
    // Test that Priority::default() returns P2 (Medium)
    let default_priority = Priority::default();

    assert_eq!(
        default_priority,
        Priority::MEDIUM,
        "Priority::default() should return MEDIUM"
    );
    assert_eq!(default_priority.0, 2, "Default priority value should be 2");
    assert_eq!(
        format!("{}", default_priority),
        "P2",
        "Default priority should display as P2"
    );
}

#[test]
fn test_multiple_epics_with_default_priority() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple epics using default priority
    for i in 1..=5 {
        let epic = Issue {
            id: format!("epic-default-{}", i),
            title: format!("Default Priority Epic {}", i),
            issue_type: IssueType::Epic,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };
        storage.create_issue(&epic).unwrap();
    }

    // Retrieve all epics and verify they all have P2 priority
    let all_issues = storage.list_issues(&Default::default()).unwrap();
    let epics: Vec<_> = all_issues
        .iter()
        .filter(|i| i.issue_type == IssueType::Epic)
        .collect();

    assert_eq!(epics.len(), 5, "Should have 5 epics");

    // Verify each epic has P2 (Medium) priority
    for epic in epics {
        assert_eq!(
            epic.priority,
            Priority::MEDIUM,
            "Epic {} should have P2 priority",
            epic.id
        );
        assert_eq!(
            epic.priority.0, 2,
            "Epic {} priority value should be 2",
            epic.id
        );
    }
}

#[test]
fn test_epic_default_vs_explicit_priorities() {
    // Create epics with different priorities including default
    let epics = vec![
        (Priority::CRITICAL, 0, "P0"),
        (Priority::HIGH, 1, "P1"),
        (Priority::MEDIUM, 2, "P2"), // This is the default
        (Priority::LOW, 3, "P3"),
        (Priority::BACKLOG, 4, "P4"),
    ];

    for (priority, value, label) in epics {
        let epic = Issue {
            id: format!("epic-{}-{}", label, value),
            title: format!("{} Epic", label),
            issue_type: IssueType::Epic,
            priority,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };

        assert_eq!(
            epic.priority.0, value,
            "{} epic should have priority value {}",
            label, value
        );
        assert_eq!(
            format!("{}", epic.priority),
            label,
            "{} epic should display as {}",
            label,
            label
        );
    }
}

#[test]
fn test_issue_new_default_priority() {
    // Test Issue::new() which uses Default::default() for priority
    let epic = Issue::new(
        "epic-new-test".to_string(),
        "Test Issue New".to_string(),
        ".".to_string(),
    );

    // Set the issue type to epic
    let mut epic = epic;
    epic.issue_type = IssueType::Epic;

    // Verify priority is P2 (the default)
    assert_eq!(
        epic.priority,
        Priority::MEDIUM,
        "Issue::new() should have P2 default priority"
    );
    assert_eq!(epic.priority.0, 2, "Default priority value should be 2");
}
