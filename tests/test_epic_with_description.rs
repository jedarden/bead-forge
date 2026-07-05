// Test epic type with description
// This test verifies that epics can be created with descriptions
// and that descriptions are correctly preserved through serialization, storage, and retrieval

#[cfg(test)]
mod tests {
    use bead_forge::model::{Issue, IssueType, Status, Priority};
    use bead_forge::storage::Storage;

    #[test]
    fn test_epic_with_basic_description() {
    // Test creating an epic with a basic description
    let epic = Issue {
        id: "epic-with-desc".to_string(),
        title: "Epic with Description".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        description: Some("This is a test epic with a description".to_string()),
        ..Default::default()
    };

    // Verify description is present
    assert_eq!(epic.description, Some("This is a test epic with a description".to_string()));

    // Test JSON serialization preserves description
    let json = serde_json::to_string(&epic).unwrap();
    assert!(json.contains("This is a test epic with a description"));
}

#[test]
fn test_epic_with_description_serialization_roundtrip() {
    // Test full JSON roundtrip for epic with description
    let epic = Issue {
        id: "epic-desc-serialize".to_string(),
        title: "Epic Description Serialization Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some("Testing epic description serialization".to_string()),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&epic).unwrap();

    // Verify JSON contains description
    assert!(json.contains("Testing epic description serialization"));

    // Deserialize back
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, "epic-desc-serialize");
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.description, Some("Testing epic description serialization".to_string()));
}

#[test]
fn test_epic_with_description_storage_and_retrieval() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with description
    let epic = Issue {
        id: "epic-desc-storage".to_string(),
        title: "Epic Description Storage Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        description: Some("Testing epic description storage and retrieval".to_string()),
        ..Default::default()
    };

    // Store the epic
    storage.create_issue(&epic).unwrap();

    // Retrieve the epic
    let retrieved = storage.get_issue("epic-desc-storage").unwrap().unwrap();

    // Verify all fields match including description
    assert_eq!(retrieved.id, "epic-desc-storage");
    assert_eq!(retrieved.title, "Epic Description Storage Test");
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.description, Some("Testing epic description storage and retrieval".to_string()));
}

#[test]
fn test_epic_with_various_description_formats() {
    // Test epic with various description formats
    let test_cases = vec![
        ("Empty description", Some("".to_string())),
        ("Short description", Some("Short".to_string())),
        ("Medium description", Some("This is a medium-length description that provides some context about the epic.".to_string())),
        ("Long description", Some("This is a longer description that provides detailed information about the epic. It may include multiple sentences and paragraphs to fully describe the scope and objectives of the epic.".to_string())),
        ("None description", None),
    ];

    for (title, description) in test_cases {
        let epic = Issue {
            id: format!("epic-{}", title.replace(' ', "-").to_lowercase()),
            title: title.to_string(),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority: Priority::MEDIUM,
            description: description.clone(),
            ..Default::default()
        };

        // Verify description matches
        assert_eq!(epic.description, description);

        // Verify JSON serialization
        let json = serde_json::to_string(&epic).unwrap();
        let deserialized: Issue = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.description, description);
    }
}

#[test]
fn test_epic_with_markdown_description() {
    // Test epic with markdown-formatted description
    let markdown_description = r#"# Epic Overview

This epic implements a new feature.

## Objectives
- Objective 1
- Objective 2
- Objective 3

## Technical Details

Code changes will be made to the following modules:
- Module A
- Module B

## Acceptance Criteria

1. Criteria 1
2. Criteria 2
3. Criteria 3"#;

    let epic = Issue {
        id: "epic-markdown-desc".to_string(),
        title: "Epic with Markdown Description".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some(markdown_description.to_string()),
        ..Default::default()
    };

    // Verify description is preserved
    assert_eq!(epic.description, Some(markdown_description.to_string()));

    // Verify JSON serialization
    let json = serde_json::to_string(&epic).unwrap();
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.description, Some(markdown_description.to_string()));
}

#[test]
fn test_epic_with_multiline_description() {
    // Test epic with multiline description
    let multiline_desc = "First line\nSecond line\nThird line\nFourth line";

    let epic = Issue {
        id: "epic-multiline-desc".to_string(),
        title: "Epic with Multiline Description".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        description: Some(multiline_desc.to_string()),
        ..Default::default()
    };

    // Verify description preserves newlines
    assert_eq!(epic.description, Some(multiline_desc.to_string()));

    // Verify JSON serialization
    let json = serde_json::to_string(&epic).unwrap();
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.description, Some(multiline_desc.to_string()));
}

#[test]
fn test_epic_with_special_characters_in_description() {
    // Test epic with special characters in description
    let special_desc = "Description with special characters: <>&\"'\\/@#$%^*()_+-=[]{}|;:,.<>?/~`";

    let epic = Issue {
        id: "epic-special-desc".to_string(),
        title: "Epic with Special Characters in Description".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        description: Some(special_desc.to_string()),
        ..Default::default()
    };

    // Verify description preserves special characters
    assert_eq!(epic.description, Some(special_desc.to_string()));

    // Verify JSON serialization
    let json = serde_json::to_string(&epic).unwrap();
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.description, Some(special_desc.to_string()));
}

#[test]
fn test_epic_with_unicode_in_description() {
    // Test epic with unicode characters in description
    let unicode_desc = "Epic with unicode: 你好 🚀 test café 日精emoji 👍";

    let epic = Issue {
        id: "epic-unicode-desc".to_string(),
        title: "Epic with Unicode Description".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        description: Some(unicode_desc.to_string()),
        ..Default::default()
    };

    // Verify description preserves unicode
    assert_eq!(epic.description, Some(unicode_desc.to_string()));

    // Verify JSON serialization
    let json = serde_json::to_string(&epic).unwrap();
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.description, Some(unicode_desc.to_string()));
}

#[test]
fn test_epic_with_description_and_children() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with description and children
    let epic = Issue {
        id: "epic-desc-children".to_string(),
        title: "Epic with Description and Children".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some("Parent epic with description and child tasks".to_string()),
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children tasks with descriptions
    let children = vec![
        ("child-1", "First child task"),
        ("child-2", "Second child task"),
        ("child-3", "Third child task"),
    ];

    for (id, desc) in &children {
        let child = Issue {
            id: id.to_string(),
            title: desc.to_string(),
            issue_type: IssueType::Task,
            status: Status::Open,
            priority: Priority::MEDIUM,
            description: Some(format!("{}: Child task description", desc)),
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();

        use bead_forge::model::DependencyType;
        storage.add_dependency("epic-desc-children", id, &DependencyType::ParentChild, "test").unwrap();
    }

    // Verify epic description is preserved
    let retrieved_epic = storage.get_issue("epic-desc-children").unwrap().unwrap();
    assert_eq!(retrieved_epic.description, Some("Parent epic with description and child tasks".to_string()));

    // Verify epic has all children
    let epic_children = storage.get_dependencies("epic-desc-children").unwrap();
    assert_eq!(epic_children.len(), 3);

    // Verify children descriptions are preserved
    for (id, desc) in &children {
        let child = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(child.description, Some(format!("{}: Child task description", desc)));
    }
}

#[test]
fn test_epic_description_persistence_with_update() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with initial description
    let epic = Issue {
        id: "epic-desc-update".to_string(),
        title: "Epic Description Update Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        description: Some("Initial description".to_string()),
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Verify initial description
    let retrieved = storage.get_issue("epic-desc-update").unwrap().unwrap();
    assert_eq!(retrieved.description, Some("Initial description".to_string()));

    // Update epic with new description using IssueChanges
    use bead_forge::model::IssueChanges;
    let changes = IssueChanges {
        description: Some("Updated description with more details".to_string()),
        status: Some(Status::InProgress),
        priority: Some(Priority::HIGH.0),
        ..Default::default()
    };
    storage.update_issue("epic-desc-update", &changes).unwrap();

    // Verify updated description
    let updated_retrieved = storage.get_issue("epic-desc-update").unwrap().unwrap();
    assert_eq!(updated_retrieved.description, Some("Updated description with more details".to_string()));
}

#[test]
fn test_epic_description_with_all_priorities() {
    // Test epic description works with all priority levels
    let priorities = vec![
        (Priority::CRITICAL, "P0"),
        (Priority::HIGH, "P1"),
        (Priority::MEDIUM, "P2"),
        (Priority::LOW, "P3"),
        (Priority::BACKLOG, "P4"),
    ];

    for (priority, expected_display) in priorities {
        let epic = Issue {
            id: format!("epic-{}-desc", expected_display),
            title: format!("Epic {} with description", expected_display),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority,
            description: Some(format!("Description for {} epic", expected_display)),
            ..Default::default()
        };

        // Verify both priority and description
        assert_eq!(format!("{}", epic.priority), expected_display);
        assert_eq!(epic.description, Some(format!("Description for {} epic", expected_display)));

        // Verify JSON serialization
        let json = serde_json::to_string(&epic).unwrap();
        assert!(json.contains(&format!("\"priority\":{}", priority.0)));
        assert!(json.contains(&format!("Description for {} epic", expected_display)));
    }
}

#[test]
fn test_epic_description_length_limits() {
    // Test epic with very long description
    let long_description = "A".repeat(10000); // 10k character description

    let epic = Issue {
        id: "epic-long-desc".to_string(),
        title: "Epic with Long Description".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        description: Some(long_description.clone()),
        ..Default::default()
    };

    // Verify long description is preserved
    assert_eq!(epic.description, Some(long_description.clone()));

    // Verify JSON serialization
    let json = serde_json::to_string(&epic).unwrap();
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.description, Some(long_description));
}

#[test]
fn test_epic_description_with_newlines_and_tabs() {
    // Test epic with newlines and tabs in description
    let desc_with_whitespace = "Line 1\n\tIndented line\nLine 3\n\nDouble newline";

    let epic = Issue {
        id: "epic-whitespace-desc".to_string(),
        title: "Epic with Whitespace in Description".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        description: Some(desc_with_whitespace.to_string()),
        ..Default::default()
    };

    // Verify whitespace is preserved
    assert_eq!(epic.description, Some(desc_with_whitespace.to_string()));

    // Verify JSON serialization
    let json = serde_json::to_string(&epic).unwrap();
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.description, Some(desc_with_whitespace.to_string()));
}}
