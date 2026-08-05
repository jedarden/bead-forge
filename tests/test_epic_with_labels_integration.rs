//! Comprehensive integration tests for epic with labels functionality.
//!
//! This test file validates end-to-end epic behavior with labels including:
//! - Creating epics with multiple labels
//! - Filtering epics by labels
//! - Epic children inheriting or having independent labels
//! - Label operations on epics via storage API
//! - JSON roundtrip with labeled epics

use bead_forge::model::{DependencyType, Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;
use std::collections::HashMap;

#[test]
fn test_epic_creation_with_multiple_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with multiple labels
    let epic = Issue {
        id: "epic-multi-labels".to_string(),
        title: "Epic with Multiple Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        labels: vec![
            "phase-1".to_string(),
            "backend".to_string(),
            "critical".to_string(),
        ],
        description: Some("Multi-label epic test".to_string()),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Verify all labels are preserved
    let retrieved = storage.get_issue("epic-multi-labels").unwrap().unwrap();
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.labels.len(), 3);
    assert!(retrieved.labels.contains(&"phase-1".to_string()));
    assert!(retrieved.labels.contains(&"backend".to_string()));
    assert!(retrieved.labels.contains(&"critical".to_string()));
}

#[test]
fn test_epic_label_addition_and_removal() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with initial label
    let epic = Issue {
        id: "epic-label-ops".to_string(),
        title: "Label Operations Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["initial".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Add more labels
    storage.add_label("epic-label-ops", "added-1").unwrap();
    storage.add_label("epic-label-ops", "added-2").unwrap();

    let retrieved = storage.get_issue("epic-label-ops").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 3);
    assert!(retrieved.labels.contains(&"added-1".to_string()));
    assert!(retrieved.labels.contains(&"added-2".to_string()));

    // Remove a label
    storage.remove_label("epic-label-ops", "initial").unwrap();

    let retrieved = storage.get_issue("epic-label-ops").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 2);
    assert!(!retrieved.labels.contains(&"initial".to_string()));
}

#[test]
fn test_epic_with_children_and_independent_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with specific labels
    let epic = Issue {
        id: "parent-epic".to_string(),
        title: "Parent Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["epic-label".to_string(), "parent".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children with their own labels
    for i in 1..=3 {
        let child = Issue {
            id: format!("child-{}", i),
            title: format!("Child {}", i),
            issue_type: IssueType::Task,
            status: Status::Open,
            labels: vec![format!("child-{}-label", i)],
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();
        storage
            .add_dependency(
                "parent-epic",
                &format!("child-{}", i),
                &DependencyType::ParentChild,
                "test",
            )
            .unwrap();
    }

    // Verify epic labels are independent from children
    let epic_retrieved = storage.get_issue("parent-epic").unwrap().unwrap();
    assert_eq!(epic_retrieved.labels.len(), 2);
    assert!(epic_retrieved.labels.contains(&"epic-label".to_string()));
    assert!(epic_retrieved.labels.contains(&"parent".to_string()));

    // Verify each child has its own label
    for i in 1..=3 {
        let child = storage.get_issue(&format!("child-{}", i)).unwrap().unwrap();
        assert_eq!(child.labels.len(), 1);
        assert!(child.labels.contains(&format!("child-{}-label", i)));
    }
}

#[test]
fn test_list_epics_by_label() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple epics with various labels
    let epics: Vec<(&str, Vec<String>)> = vec![
        ("epic-1", vec!["frontend".to_string(), "urgent".to_string()]),
        ("epic-2", vec!["backend".to_string()]),
        (
            "epic-3",
            vec!["frontend".to_string(), "feature".to_string()],
        ),
        ("epic-4", vec!["urgent".to_string()]),
    ];

    for (id, labels) in epics {
        let epic = Issue {
            id: id.to_string(),
            title: format!("Epic {}", id),
            issue_type: IssueType::Epic,
            status: Status::Open,
            labels,
            ..Default::default()
        };
        storage.create_issue(&epic).unwrap();
    }

    // Get all issues and filter by type and label
    let all_issues = storage.list_all_issues().unwrap();
    let frontend_epics: Vec<_> = all_issues
        .iter()
        .filter(|i| i.issue_type == IssueType::Epic && i.labels.contains(&"frontend".to_string()))
        .collect();

    assert_eq!(frontend_epics.len(), 2);
    assert!(frontend_epics.iter().any(|e| e.id == "epic-1"));
    assert!(frontend_epics.iter().any(|e| e.id == "epic-3"));
}

#[test]
fn test_epic_labels_json_serialization() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let epic = Issue {
        id: "epic-json".to_string(),
        title: "JSON Serialization Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["json-test".to_string(), "serialization".to_string()],
        description: Some("Testing epic labels JSON roundtrip".to_string()),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Retrieve and serialize
    let retrieved = storage.get_issue("epic-json").unwrap().unwrap();
    let json = serde_json::to_string_pretty(&retrieved).unwrap();

    // Verify JSON contains epic type and all labels
    // The issue_type field should be serialized as "epic" (snake_case)
    assert!(json.contains("epic"));
    assert!(json.contains("json-test"));
    assert!(json.contains("serialization"));

    // Deserialize and verify
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.labels.len(), 2);
    assert!(deserialized.labels.contains(&"json-test".to_string()));
    assert!(deserialized.labels.contains(&"serialization".to_string()));
}

#[test]
fn test_get_all_labels_in_workspace_with_epics() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epics and regular tasks with various labels
    let issues: Vec<(&str, IssueType, Vec<String>)> = vec![
        (
            "epic-a",
            IssueType::Epic,
            vec!["epic-label".to_string(), "shared".to_string()],
        ),
        (
            "epic-b",
            IssueType::Epic,
            vec!["backend".to_string(), "shared".to_string()],
        ),
        ("task-1", IssueType::Task, vec!["shared".to_string()]),
        (
            "task-2",
            IssueType::Bug,
            vec!["urgent".to_string(), "shared".to_string()],
        ),
    ];

    for (id, issue_type, labels) in issues {
        let issue = Issue {
            id: id.to_string(),
            title: format!("Issue {}", id),
            issue_type,
            status: Status::Open,
            labels,
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();
    }

    // Get all labels with counts
    let all_labels = storage.list_all_labels().unwrap();
    let label_map: HashMap<String, i64> = all_labels.into_iter().collect();

    // Verify label counts
    assert_eq!(label_map.get("shared"), Some(&4)); // All 4 issues have this
    assert_eq!(label_map.get("epic-label"), Some(&1)); // Only epic-a
    assert_eq!(label_map.get("backend"), Some(&1)); // Only epic-b
    assert_eq!(label_map.get("urgent"), Some(&1)); // Only task-2
}

#[test]
fn test_epic_with_empty_label_set() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with no labels
    let epic = Issue {
        id: "epic-no-labels".to_string(),
        title: "Epic Without Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Verify epic has no labels
    let retrieved = storage.get_issue("epic-no-labels").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 0);

    // Verify get_labels returns empty set
    let labels = storage.get_labels("epic-no-labels").unwrap();
    assert_eq!(labels.len(), 0);
}

#[test]
fn test_epic_label_set_semantics() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with a label
    let epic = Issue {
        id: "epic-set-test".to_string(),
        title: "Set Semantics Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["label-1".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Try to add the same label again - should not create duplicate
    storage.add_label("epic-set-test", "label-1").unwrap();

    let retrieved = storage.get_issue("epic-set-test").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 1); // Still only one label
    assert!(retrieved.labels.contains(&"label-1".to_string()));

    // Add a new label
    storage.add_label("epic-set-test", "label-2").unwrap();

    let retrieved = storage.get_issue("epic-set-test").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 2);
}

#[test]
fn test_epic_with_closed_children_and_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with label
    let epic = Issue {
        id: "epic-closed-children".to_string(),
        title: "Epic with Closed Children".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["in-progress".to_string()],
        priority: Priority::HIGH,
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create mix of open and closed children
    for i in 1..=4 {
        let status = if i <= 2 { Status::Closed } else { Status::Open };
        let mut child = Issue {
            id: format!("child-{}", i),
            title: format!("Child {}", i),
            issue_type: IssueType::Task,
            status,
            labels: vec![format!("child-{}-label", i)],
            ..Default::default()
        };
        if i <= 2 {
            child.closed_at = Some(Utc::now());
        }
        storage.create_issue(&child).unwrap();
        storage
            .add_dependency(
                "epic-closed-children",
                &format!("child-{}", i),
                &DependencyType::ParentChild,
                "test",
            )
            .unwrap();
    }

    // Verify epic still has its label regardless of child status
    let epic_retrieved = storage.get_issue("epic-closed-children").unwrap().unwrap();
    assert_eq!(epic_retrieved.labels.len(), 1);
    assert!(epic_retrieved.labels.contains(&"in-progress".to_string()));

    // Count children
    let children = storage.get_dependencies("epic-closed-children").unwrap();
    assert_eq!(children.len(), 4);

    // Verify children have independent labels
    for i in 1..=4 {
        let child = storage.get_issue(&format!("child-{}", i)).unwrap().unwrap();
        assert_eq!(child.labels.len(), 1);
        assert!(child.labels.contains(&format!("child-{}-label", i)));
    }
}

#[test]
fn test_epic_label_persistence_through_update() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with labels
    let epic = Issue {
        id: "epic-update-test".to_string(),
        title: "Update Test Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["label-1".to_string(), "label-2".to_string()],
        priority: Priority::MEDIUM,
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Update epic status
    storage
        .update_issue(
            "epic-update-test",
            &bead_forge::model::IssueChanges {
                status: Some(Status::InProgress),
                ..Default::default()
            },
        )
        .unwrap();

    // Verify labels are preserved through update
    let retrieved = storage.get_issue("epic-update-test").unwrap().unwrap();
    assert_eq!(retrieved.status, Status::InProgress);
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"label-1".to_string()));
    assert!(retrieved.labels.contains(&"label-2".to_string()));
}

#[test]
fn test_multiple_epics_with_common_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epics sharing some labels
    let epics = vec![
        (
            "epic-frontend-1",
            vec!["frontend".to_string(), "feature".to_string()],
        ),
        (
            "epic-frontend-2",
            vec!["frontend".to_string(), "bugfix".to_string()],
        ),
        (
            "epic-backend-1",
            vec!["backend".to_string(), "feature".to_string()],
        ),
        (
            "epic-shared",
            vec![
                "frontend".to_string(),
                "backend".to_string(),
                "feature".to_string(),
            ],
        ),
    ];

    for (id, labels) in epics {
        let epic = Issue {
            id: id.to_string(),
            title: format!("Epic {}", id),
            issue_type: IssueType::Epic,
            status: Status::Open,
            labels,
            ..Default::default()
        };
        storage.create_issue(&epic).unwrap();
    }

    // Verify epics with "frontend" label
    let all_issues = storage.list_all_issues().unwrap();
    let frontend_epics: Vec<_> = all_issues
        .iter()
        .filter(|i| i.issue_type == IssueType::Epic && i.labels.contains(&"frontend".to_string()))
        .collect();
    assert_eq!(frontend_epics.len(), 3);

    // Verify epics with "feature" label
    let feature_epics: Vec<_> = all_issues
        .iter()
        .filter(|i| i.issue_type == IssueType::Epic && i.labels.contains(&"feature".to_string()))
        .collect();
    assert_eq!(feature_epics.len(), 3);
}

#[test]
fn test_epic_label_removal_nonexistent() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with a label
    let epic = Issue {
        id: "epic-remove-test".to_string(),
        title: "Remove Test Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["existing".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Try to remove non-existent label - should succeed but be a no-op
    storage
        .remove_label("epic-remove-test", "does-not-exist")
        .unwrap();

    // Verify existing label is still present
    let retrieved = storage.get_issue("epic-remove-test").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 1);
    assert!(retrieved.labels.contains(&"existing".to_string()));
}
