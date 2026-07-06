// Comprehensive epic P1 priority and description test
// This test comprehensively verifies that epics can be created with P1 (high) priority
// and descriptions, and that both are correctly preserved through serialization,
// storage, retrieval, and all operations

use bead_forge::model::{Issue, IssueType, Status, Priority, DependencyType, EpicStatus, IssueChanges};
use bead_forge::storage::Storage;
use chrono::{Utc, Duration};

#[test]
fn test_epic_p1_with_basic_description() {
    // Test creating an epic with P1 priority and basic description
    let epic = Issue {
        id: "epic-p1-desc-basic".to_string(),
        title: "P1 Epic with Description".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH, // P1
        description: Some("This is a P1 epic with a description".to_string()),
        ..Default::default()
    };

    // Verify P1 priority and description
    assert_eq!(epic.priority, Priority::HIGH);
    assert_eq!(epic.priority.0, 1);
    assert_eq!(epic.description, Some("This is a P1 epic with a description".to_string()));

    // Test JSON serialization preserves both
    let json = serde_json::to_string(&epic).unwrap();
    assert!(json.contains("\"priority\":1"));
    assert!(json.contains("This is a P1 epic with a description"));
    assert!(json.contains("\"issue_type\":\"epic\""));
}

#[test]
fn test_epic_p1_with_description_serialization_roundtrip() {
    // Test full JSON roundtrip for epic with P1 priority and description
    let epic = Issue {
        id: "epic-p1-desc-serialize".to_string(),
        title: "P1 Epic Description Serialization Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some("Testing P1 epic with description serialization".to_string()),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&epic).unwrap();

    // Verify JSON contains all expected values
    assert!(json.contains("\"issue_type\":\"epic\""));
    assert!(json.contains("\"priority\":1"));
    assert!(json.contains("Testing P1 epic with description serialization"));

    // Deserialize back
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, "epic-p1-desc-serialize");
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.priority, Priority::HIGH);
    assert_eq!(deserialized.priority.0, 1);
    assert_eq!(deserialized.description, Some("Testing P1 epic with description serialization".to_string()));
}

#[test]
fn test_epic_p1_with_description_storage_and_retrieval() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with P1 priority and description
    let epic = Issue {
        id: "epic-p1-desc-storage".to_string(),
        title: "P1 Epic Description Storage Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some("P1 epic with description for storage test".to_string()),
        ..Default::default()
    };

    // Store the epic
    storage.create_issue(&epic).unwrap();

    // Retrieve the epic
    let retrieved = storage.get_issue("epic-p1-desc-storage").unwrap().unwrap();

    // Verify all fields match
    assert_eq!(retrieved.id, "epic-p1-desc-storage");
    assert_eq!(retrieved.title, "P1 Epic Description Storage Test");
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.status, Status::Open);
    assert_eq!(retrieved.priority, Priority::HIGH);
    assert_eq!(retrieved.priority.0, 1);
    assert_eq!(retrieved.description, Some("P1 epic with description for storage test".to_string()));
}

#[test]
fn test_epic_p1_with_markdown_description() {
    // Test P1 epic with markdown-formatted description
    let markdown_description = r#"# P1 Epic Overview

This is a high priority epic implementing a critical feature.

## Objectives
- Implement core functionality
- Add comprehensive tests
- Update documentation

## Acceptance Criteria

1. All tests pass
2. Documentation is complete
3. Code is reviewed"#;

    let epic = Issue {
        id: "epic-p1-markdown-desc".to_string(),
        title: "P1 Epic with Markdown Description".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some(markdown_description.to_string()),
        ..Default::default()
    };

    // Verify both P1 priority and markdown description are preserved
    assert_eq!(epic.priority, Priority::HIGH);
    assert_eq!(epic.priority.0, 1);
    assert_eq!(epic.description, Some(markdown_description.to_string()));

    // Verify JSON serialization
    let json = serde_json::to_string(&epic).unwrap();
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::HIGH);
    assert_eq!(deserialized.description, Some(markdown_description.to_string()));
}

#[test]
fn test_epic_p1_with_description_and_children() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P1 epic with description and children
    let epic = Issue {
        id: "epic-p1-desc-children".to_string(),
        title: "P1 Epic with Description and Children".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some("High priority epic with description and child tasks".to_string()),
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children tasks with various priorities and descriptions
    let children = vec![
        ("child-1", Priority::CRITICAL, "Critical child task"),
        ("child-2", Priority::HIGH, "High priority child task"),
        ("child-3", Priority::MEDIUM, "Medium priority child task"),
    ];

    for (id, priority, title) in &children {
        let child = Issue {
            id: id.to_string(),
            title: title.to_string(),
            issue_type: IssueType::Task,
            status: Status::Open,
            priority: *priority,
            description: Some(format!("{}: Child task description", title)),
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();
        storage.add_dependency("epic-p1-desc-children", id, &DependencyType::ParentChild, "test").unwrap();
    }

    // Verify epic P1 priority and description are preserved
    let retrieved_epic = storage.get_issue("epic-p1-desc-children").unwrap().unwrap();
    assert_eq!(retrieved_epic.priority, Priority::HIGH);
    assert_eq!(retrieved_epic.priority.0, 1);
    assert_eq!(retrieved_epic.description, Some("High priority epic with description and child tasks".to_string()));

    // Verify epic has all children
    let epic_children = storage.get_dependencies("epic-p1-desc-children").unwrap();
    assert_eq!(epic_children.len(), 3);

    // Verify children descriptions are preserved
    for (id, _priority, title) in &children {
        let child = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(child.description, Some(format!("{}: Child task description", title)));
    }
}

#[test]
fn test_epic_p1_description_persistence_with_update() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with P1 priority and initial description
    let epic = Issue {
        id: "epic-p1-desc-update".to_string(),
        title: "P1 Epic Description Update Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some("Initial P1 epic description".to_string()),
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Verify initial P1 priority and description
    let retrieved = storage.get_issue("epic-p1-desc-update").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::HIGH);
    assert_eq!(retrieved.description, Some("Initial P1 epic description".to_string()));

    // Update epic with new description using IssueChanges
    let changes = IssueChanges {
        description: Some("Updated P1 epic description with more details".to_string()),
        status: Some(Status::InProgress),
        ..Default::default()
    };
    storage.update_issue("epic-p1-desc-update", &changes).unwrap();

    // Verify updated description while P1 priority is preserved
    let updated_retrieved = storage.get_issue("epic-p1-desc-update").unwrap().unwrap();
    assert_eq!(updated_retrieved.priority, Priority::HIGH); // Priority unchanged
    assert_eq!(updated_retrieved.status, Status::InProgress);
    assert_eq!(updated_retrieved.description, Some("Updated P1 epic description with more details".to_string()));
}

#[test]
fn test_epic_p1_with_long_description() {
    // Test P1 epic with very long description
    let long_description = "This is a very long description for a P1 epic. "
        .repeat(100); // ~5000 character description

    let epic = Issue {
        id: "epic-p1-long-desc".to_string(),
        title: "P1 Epic with Long Description".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some(long_description.clone()),
        ..Default::default()
    };

    // Verify both P1 priority and long description are preserved
    assert_eq!(epic.priority, Priority::HIGH);
    assert_eq!(epic.priority.0, 1);
    assert_eq!(epic.description, Some(long_description.clone()));

    // Verify JSON serialization
    let json = serde_json::to_string(&epic).unwrap();
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::HIGH);
    assert_eq!(deserialized.description, Some(long_description));
}

#[test]
fn test_epic_p1_with_special_characters_in_description() {
    // Test P1 epic with special characters in description
    let special_desc = "P1 epic with special characters: <>&\"'\\/@#$%^*()_+-=[]{}|;:,.<>?/~`";

    let epic = Issue {
        id: "epic-p1-special-desc".to_string(),
        title: "P1 Epic with Special Characters".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some(special_desc.to_string()),
        ..Default::default()
    };

    // Verify both P1 priority and special characters are preserved
    assert_eq!(epic.priority, Priority::HIGH);
    assert_eq!(epic.description, Some(special_desc.to_string()));

    // Verify JSON serialization
    let json = serde_json::to_string(&epic).unwrap();
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::HIGH);
    assert_eq!(deserialized.description, Some(special_desc.to_string()));
}

#[test]
fn test_epic_p1_with_unicode_in_description() {
    // Test P1 epic with unicode characters in description
    let unicode_desc = "P1 epic with unicode: 你好 🚀 test café 日精 emoji 👍 高优先级";

    let epic = Issue {
        id: "epic-p1-unicode-desc".to_string(),
        title: "P1 Epic with Unicode Description".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some(unicode_desc.to_string()),
        ..Default::default()
    };

    // Verify both P1 priority and unicode are preserved
    assert_eq!(epic.priority, Priority::HIGH);
    assert_eq!(epic.description, Some(unicode_desc.to_string()));

    // Verify JSON serialization
    let json = serde_json::to_string(&epic).unwrap();
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::HIGH);
    assert_eq!(deserialized.description, Some(unicode_desc.to_string()));
}

#[test]
fn test_epic_p1_with_multiline_description() {
    // Test P1 epic with multiline description
    let multiline_desc = "P1 epic details:\nLine 1: Critical feature\nLine 2: High priority\nLine 3: Important work";

    let epic = Issue {
        id: "epic-p1-multiline-desc".to_string(),
        title: "P1 Epic with Multiline Description".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some(multiline_desc.to_string()),
        ..Default::default()
    };

    // Verify both P1 priority and multiline description are preserved
    assert_eq!(epic.priority, Priority::HIGH);
    assert_eq!(epic.description, Some(multiline_desc.to_string()));

    // Verify JSON serialization
    let json = serde_json::to_string(&epic).unwrap();
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::HIGH);
    assert_eq!(deserialized.description, Some(multiline_desc.to_string()));
}

#[test]
fn test_epic_p1_status_computation_with_description() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P1 epic with description and children
    let epic = Issue {
        id: "epic-p1-status-desc".to_string(),
        title: "P1 Epic Status Test with Description".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some("P1 epic for testing status computation with description".to_string()),
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create 3 children, close 2 of them
    for i in 1..=3 {
        let is_closed = i <= 2;
        let status = if is_closed { Status::Closed } else { Status::Open };
        let mut child = Issue {
            id: format!("child-p1-status-{}", i),
            title: format!("Child {}", i),
            issue_type: IssueType::Task,
            status,
            description: Some(format!("Child {} description", i)),
            ..Default::default()
        };
        if is_closed {
            child.closed_at = Some(Utc::now());
        }
        storage.create_issue(&child).unwrap();
        storage.add_dependency("epic-p1-status-desc", &child.id, &DependencyType::ParentChild, "test").unwrap();
    }

    // Verify P1 priority and description are preserved
    let epic_issue = storage.get_issue("epic-p1-status-desc").unwrap().unwrap();
    assert_eq!(epic_issue.priority, Priority::HIGH);
    assert_eq!(epic_issue.description, Some("P1 epic for testing status computation with description".to_string()));

    // Verify epic status computation
    let children = storage.get_dependencies("epic-p1-status-desc").unwrap();
    let closed_children = children.iter().filter(|d| {
        match storage.get_issue(&d.depends_on_id) {
            Ok(Some(child)) => child.status == Status::Closed,
            _ => false,
        }
    }).count();

    let epic_status = EpicStatus {
        epic: epic_issue,
        total_children: children.len(),
        closed_children,
        eligible_for_close: closed_children == children.len() && children.len() > 0,
    };

    assert_eq!(epic_status.total_children, 3);
    assert_eq!(epic_status.closed_children, 2);
    assert!(!epic_status.eligible_for_close); // Not all closed
}

#[test]
fn test_epic_p1_with_empty_description() {
    // Test P1 epic with empty description
    let epic = Issue {
        id: "epic-p1-empty-desc".to_string(),
        title: "P1 Epic with Empty Description".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some("".to_string()),
        ..Default::default()
    };

    // Verify P1 priority and empty description
    assert_eq!(epic.priority, Priority::HIGH);
    assert_eq!(epic.description, Some("".to_string()));

    // Verify JSON serialization
    let json = serde_json::to_string(&epic).unwrap();
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::HIGH);
    assert_eq!(deserialized.description, Some("".to_string()));
}

#[test]
fn test_epic_p1_with_none_description() {
    // Test P1 epic with None description
    let epic = Issue {
        id: "epic-p1-none-desc".to_string(),
        title: "P1 Epic with None Description".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: None,
        ..Default::default()
    };

    // Verify P1 priority and None description
    assert_eq!(epic.priority, Priority::HIGH);
    assert_eq!(epic.description, None);

    // Verify JSON serialization (description should be skipped)
    let json = serde_json::to_string(&epic).unwrap();
    assert!(!json.contains("\"description\"")); // None should skip serialization
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::HIGH);
    assert_eq!(deserialized.description, None);
}

#[test]
fn test_epic_p1_with_all_statuses_and_description() {
    // Test that P1 epic with description works with various statuses
    let statuses = vec![
        (Status::Open, "open"),
        (Status::InProgress, "in_progress"),
        (Status::Blocked, "blocked"),
        (Status::Deferred, "deferred"),
        (Status::Draft, "draft"),
    ];

    for (status, status_str) in statuses {
        let epic = Issue {
            id: format!("epic-p1-{}-desc", status_str),
            title: format!("P1 Epic with {} status and description", status_str),
            issue_type: IssueType::Epic,
            status,
            priority: Priority::HIGH,
            description: Some(format!("Description for {} status P1 epic", status_str)),
            ..Default::default()
        };

        // Verify P1 priority and description regardless of status
        assert_eq!(epic.priority, Priority::HIGH);
        assert_eq!(epic.description, Some(format!("Description for {} status P1 epic", status_str)));

        // Verify JSON serialization preserves all three
        let json = serde_json::to_string(&epic).unwrap();
        assert!(json.contains("\"priority\":1"));
        assert!(json.contains(&format!("\"status\":\"{}\"", status_str)));
        assert!(json.contains(&format!("Description for {} status P1 epic", status_str)));
    }
}

#[test]
fn test_epic_p1_sync_equals_with_description() {
    // Test that P1 epics with description can be compared using sync_equals
    let epic1 = Issue {
        id: "epic-p1-sync-desc".to_string(),
        title: "P1 Sync Test with Description".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some("Original description".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    let mut epic2 = epic1.clone();

    // Modify timestamps (should be ignored by sync_equals)
    epic2.created_at = Utc::now() + Duration::seconds(100);
    epic2.updated_at = Utc::now() + Duration::seconds(200);

    // Should still be equal according to sync_equals
    assert!(epic1.sync_equals(&epic2));

    // Change description - should not be equal
    epic2.description = Some("Modified description".to_string());
    assert!(!epic1.sync_equals(&epic2));

    // Change priority - should not be equal
    epic2.description = epic1.description.clone(); // Restore description
    epic2.priority = Priority::MEDIUM;
    assert!(!epic1.sync_equals(&epic2));
}
