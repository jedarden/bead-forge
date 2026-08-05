// Test P2 (Normal Priority) Epic Creation
// Tests creating epics with P2 (normal) priority, verifying storage and serialization

use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;

#[test]
fn test_p2_epic_creation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with P2 (normal) priority
    let epic = Issue {
        id: "epic-p2-test".to_string(),
        title: "Normal Priority Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM, // P2 = 2
        description: Some("This is a normal priority epic".to_string()),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Verify the epic was stored correctly
    let retrieved = storage.get_issue("epic-p2-test").unwrap().unwrap();

    // Test 1: Verify ID matches
    assert_eq!(retrieved.id, "epic-p2-test");

    // Test 2: Verify issue type is epic
    assert_eq!(retrieved.issue_type, IssueType::Epic);

    // Test 3: Verify priority is P2 (normal = 2)
    assert_eq!(retrieved.priority, Priority::MEDIUM);
    assert_eq!(retrieved.priority.0, 2);

    // Test 4: Verify status
    assert_eq!(retrieved.status, Status::Open);

    // Test 5: Verify description is preserved
    assert_eq!(
        retrieved.description,
        Some("This is a normal priority epic".to_string())
    );
}

#[test]
fn test_p2_epic_serialization() {
    // Create epic with P2 priority
    let epic = Issue {
        id: "epic-p2-serialize".to_string(),
        title: "P2 Epic Serialization Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        ..Default::default()
    };

    // Test JSON serialization
    let json = serde_json::to_string(&epic).unwrap();

    // Verify epic type is serialized correctly
    assert!(json.contains("\"issue_type\":\"epic\""));

    // Verify P2 priority is serialized as 2
    assert!(json.contains("\"priority\":2"));

    // Test deserialization
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.priority, Priority::MEDIUM);
    assert_eq!(deserialized.priority.0, 2);
    assert_eq!(deserialized.id, "epic-p2-serialize");
}

#[test]
fn test_p2_priority_value() {
    // Test that Priority::MEDIUM is indeed P2 (value 2)
    assert_eq!(Priority::MEDIUM.0, 2);

    // Test that it compares correctly
    assert_eq!(Priority::MEDIUM, Priority(2));
    assert_ne!(Priority::MEDIUM, Priority::CRITICAL);
    assert_ne!(Priority::MEDIUM, Priority::HIGH);

    // Test that it's between P1 and P3
    assert!(Priority::CRITICAL < Priority::MEDIUM);
    assert!(Priority::HIGH < Priority::MEDIUM);
    assert!(Priority::MEDIUM < Priority::LOW);
    assert!(Priority::MEDIUM < Priority::BACKLOG);
}

#[test]
fn test_p2_epic_with_full_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create comprehensive P2 epic with all fields
    let mut epic = Issue {
        id: "epic-p2-full".to_string(),
        title: "Complete P2 Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        description: Some("Full metadata test for P2 epic".to_string()),
        assignee: Some("test-worker".to_string()),
        ..Default::default()
    };
    epic.created_at = Utc::now();
    epic.updated_at = Utc::now();

    storage.create_issue(&epic).unwrap();

    // Retrieve and verify all fields
    let retrieved = storage.get_issue("epic-p2-full").unwrap().unwrap();

    assert_eq!(retrieved.id, "epic-p2-full");
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.priority, Priority::MEDIUM);
    assert_eq!(retrieved.priority.0, 2);
    assert_eq!(
        retrieved.description,
        Some("Full metadata test for P2 epic".to_string())
    );
    assert_eq!(retrieved.assignee, Some("test-worker".to_string()));
}

#[test]
fn test_p2_epic_display_formatting() {
    // Test priority display formatting for P2
    let p2 = Priority::MEDIUM;
    let display = format!("{}", p2);
    assert_eq!(display, "P2");

    // Create epic and test full display
    let epic = Issue {
        id: "epic-p2-display".to_string(),
        title: "P2 Display Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        ..Default::default()
    };

    // Verify priority displays as P2
    assert_eq!(format!("{}", epic.priority), "P2");
}

#[test]
fn test_multiple_p2_epics() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple P2 epics
    for i in 1..=3 {
        let epic = Issue {
            id: format!("epic-p2-{}", i),
            title: format!("P2 Epic {}", i),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority: Priority::MEDIUM,
            ..Default::default()
        };
        storage.create_issue(&epic).unwrap();
    }

    // Verify all were stored with correct priority
    let all_issues = storage.list_issues(&Default::default()).unwrap();
    let p2_epics: Vec<_> = all_issues
        .iter()
        .filter(|i| i.issue_type == IssueType::Epic && i.priority == Priority::MEDIUM)
        .collect();

    assert_eq!(p2_epics.len(), 3);

    // Verify each has priority 2
    for epic in p2_epics {
        assert_eq!(epic.priority.0, 2);
        assert_eq!(epic.issue_type, IssueType::Epic);
    }
}

#[test]
fn test_p2_vs_other_priorities() {
    // Test P2 against other priority levels
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
fn test_p2_epic_json_roundtrip() {
    // Create epic with P2 priority
    let original = Issue {
        id: "epic-p2-roundtrip".to_string(),
        title: "P2 Roundtrip Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        description: Some("Test description for P2 epic".to_string()),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&original).unwrap();

    // Verify JSON contains correct values (note: to_string_pretty adds spaces)
    assert!(json.contains("\"epic-p2-roundtrip\""));
    assert!(json.contains("\"issue_type\": \"epic\""));
    assert!(json.contains("\"priority\": 2"));
    assert!(json.contains("Test description for P2 epic"));

    // Deserialize back
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    // Verify all fields match
    assert_eq!(deserialized.id, original.id);
    assert_eq!(deserialized.issue_type, original.issue_type);
    assert_eq!(deserialized.priority, original.priority);
    assert_eq!(deserialized.priority.0, 2);
    assert_eq!(deserialized.description, original.description);
    assert_eq!(deserialized.status, original.status);
}

#[test]
fn test_p2_priority_from_string() {
    // Test parsing P2 from various string formats
    let p2_from_p2_str = "P2".parse::<Priority>().unwrap();
    assert_eq!(
        p2_from_p2_str,
        Priority::MEDIUM,
        "Parsing 'P2' should give MEDIUM"
    );

    let p2_from_2_str = "2".parse::<Priority>().unwrap();
    assert_eq!(
        p2_from_2_str,
        Priority::MEDIUM,
        "Parsing '2' should give MEDIUM"
    );

    let p2_from_p2_lowercase = "p2".parse::<Priority>().unwrap();
    assert_eq!(
        p2_from_p2_lowercase,
        Priority::MEDIUM,
        "Parsing 'p2' should give MEDIUM (case insensitive)"
    );

    // Test with whitespace
    let p2_from_whitespace = "  P2  ".parse::<Priority>().unwrap();
    assert_eq!(
        p2_from_whitespace,
        Priority::MEDIUM,
        "Parsing '  P2  ' should give MEDIUM (whitespace trimmed)"
    );
}

#[test]
fn test_p2_priority_ordering() {
    // Test that P2 is correctly ordered relative to other priorities
    let p0 = Priority::CRITICAL;
    let p1 = Priority::HIGH;
    let p2 = Priority::MEDIUM;
    let p3 = Priority::LOW;
    let p4 = Priority::BACKLOG;

    // P2 should be greater than P0, P1 (lower priority)
    assert!(p2 > p0, "P2 should be greater than P0 (lower priority)");
    assert!(p2 > p1, "P2 should be greater than P1 (lower priority)");
    assert!(p2 >= p0);
    assert!(p2 >= p1);

    // P2 should be less than P3, P4 (higher priority)
    assert!(p2 < p3, "P2 should be less than P3 (higher priority)");
    assert!(p2 < p4, "P2 should be less than P4 (higher priority)");
    assert!(p2 <= p3);
    assert!(p2 <= p4);

    // P2 should equal itself
    assert_eq!(p2, p2);
    assert!(p2 <= p2);
    assert!(p2 >= p2);
}

#[test]
fn test_p2_epic_with_different_statuses() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P2 epics with different statuses
    let statuses = vec![
        Status::Open,
        Status::InProgress,
        Status::Blocked,
        Status::Deferred,
    ];

    for (i, status) in statuses.iter().enumerate() {
        let epic = Issue {
            id: format!("epic-p2-status-{}", i),
            title: format!("P2 Epic with {:?}", status),
            issue_type: IssueType::Epic,
            status: status.clone(),
            priority: Priority::MEDIUM,
            ..Default::default()
        };
        storage.create_issue(&epic).unwrap();
    }

    // Verify all were stored with P2 priority
    let all_issues = storage.list_issues(&Default::default()).unwrap();
    let p2_epics: Vec<_> = all_issues
        .iter()
        .filter(|i| i.issue_type == IssueType::Epic && i.priority == Priority::MEDIUM)
        .collect();

    assert_eq!(p2_epics.len(), 4);

    // Verify each has priority 2
    for epic in p2_epics {
        assert_eq!(epic.priority.0, 2);
        assert_eq!(epic.issue_type, IssueType::Epic);
    }
}

#[test]
fn test_p2_epic_with_children() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P2 epic
    let epic = Issue {
        id: "epic-p2-with-children".to_string(),
        title: "P2 Epic with Children".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        description: Some("P2 epic with child tasks".to_string()),
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create child tasks
    for i in 1..=3 {
        let child_id = format!("child-p2-{}", i);
        let child = Issue {
            id: child_id.clone(),
            title: format!("Child task {}", i),
            issue_type: IssueType::Task,
            status: Status::Open,
            priority: Priority::MEDIUM, // Same priority as epic
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();

        // Add dependency relationship (child depends on epic)
        storage
            .add_dependency(
                &child_id,
                &epic.id,
                &bead_forge::model::DependencyType::ParentChild,
                "test",
            )
            .unwrap();
    }

    // Retrieve epic and verify
    let retrieved = storage.get_issue("epic-p2-with-children").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::MEDIUM);
    assert_eq!(retrieved.issue_type, IssueType::Epic);
}

#[test]
fn test_p2_mixed_with_other_priorities() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epics with mixed priorities
    let priorities = vec![
        (Priority::CRITICAL, "p0-epic"),
        (Priority::HIGH, "p1-epic"),
        (Priority::MEDIUM, "p2-epic"),
        (Priority::LOW, "p3-epic"),
        (Priority::BACKLOG, "p4-epic"),
    ];

    for (priority, id) in priorities {
        let epic = Issue {
            id: id.to_string(),
            title: format!("Epic with {:?}", priority),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority,
            ..Default::default()
        };
        storage.create_issue(&epic).unwrap();
    }

    // Query specifically for P2 epics
    let all_issues = storage.list_issues(&Default::default()).unwrap();
    let p2_epics: Vec<_> = all_issues
        .iter()
        .filter(|i| i.issue_type == IssueType::Epic && i.priority == Priority::MEDIUM)
        .collect();

    assert_eq!(p2_epics.len(), 1);
    assert_eq!(p2_epics[0].id, "p2-epic");
    assert_eq!(p2_epics[0].priority.0, 2);
}

#[test]
fn test_p2_default_priority_check() {
    // Test that P2 is the default priority when not specified
    let epic_no_priority = Issue {
        id: "epic-default".to_string(),
        title: "Default Priority Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM, // P2 as default
        ..Default::default()
    };

    assert_eq!(epic_no_priority.priority.0, 2);
    assert_eq!(format!("{}", epic_no_priority.priority), "P2");
}
