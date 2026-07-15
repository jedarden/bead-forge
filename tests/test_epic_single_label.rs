// Epic 4: Single Label Tests
// Tests that epics work correctly with exactly one label

use bead_forge::model::{DependencyType, EpicStatus, Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;

#[test]
fn test_epic_creation_single_label() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with single label
    let epic = Issue {
        id: "epic-single-label".to_string(),
        title: "Epic with Single Label".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["feature".to_string()],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Retrieve and verify single label is preserved
    let retrieved = storage.get_issue("epic-single-label").unwrap().unwrap();
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.labels.len(), 1);
    assert_eq!(retrieved.labels[0], "feature");
}

#[test]
fn test_epic_single_label_serialization() {
    // Create epic with single label
    let epic = Issue {
        id: "epic-single-serialize".to_string(),
        title: "Single Label Serialization Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["urgent".to_string()],
        priority: Priority::HIGH,
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&epic).unwrap();

    // Verify epic type and single label are serialized correctly
    assert!(json.contains(r#""issue_type":"epic""#));
    assert!(json.contains(r#""labels":["urgent"]"#));

    // Deserialize and verify
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.labels.len(), 1);
    assert_eq!(deserialized.labels[0], "urgent");
    assert_eq!(deserialized.priority, Priority::HIGH);
}

#[test]
fn test_epic_single_label_with_children() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with single label
    let epic = Issue {
        id: "epic-single-with-children".to_string(),
        title: "Single Label Epic with Children".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["backend".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children - some with labels, some without
    let child1 = Issue {
        id: "child-with-label".to_string(),
        title: "Child with Label".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["api".to_string()],
        ..Default::default()
    };
    storage.create_issue(&child1).unwrap();
    storage
        .add_dependency(
            "epic-single-with-children",
            "child-with-label",
            &DependencyType::ParentChild,
            "test",
        )
        .unwrap();

    let child2 = Issue {
        id: "child-no-label".to_string(),
        title: "Child without Label".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&child2).unwrap();
    storage
        .add_dependency(
            "epic-single-with-children",
            "child-no-label",
            &DependencyType::ParentChild,
            "test",
        )
        .unwrap();

    // Verify epic still has only its single label
    let epic_retrieved = storage
        .get_issue("epic-single-with-children")
        .unwrap()
        .unwrap();
    assert_eq!(epic_retrieved.labels.len(), 1);
    assert_eq!(epic_retrieved.labels[0], "backend");

    // Verify children have their own label state
    let child1_retrieved = storage.get_issue("child-with-label").unwrap().unwrap();
    assert_eq!(child1_retrieved.labels.len(), 1);

    let child2_retrieved = storage.get_issue("child-no-label").unwrap().unwrap();
    assert_eq!(child2_retrieved.labels.len(), 0);
}

#[test]
fn test_epic_single_label_status_computation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with single label
    let epic = Issue {
        id: "epic-single-status".to_string(),
        title: "Single Label Status Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["feature".to_string()],
        priority: Priority::CRITICAL,
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create and close a child
    let mut child = Issue {
        id: "single-status-child".to_string(),
        title: "Single Status Child".to_string(),
        issue_type: IssueType::Task,
        status: Status::Closed,
        labels: vec![],
        ..Default::default()
    };
    child.closed_at = Some(Utc::now());
    storage.create_issue(&child).unwrap();
    storage
        .add_dependency(
            "epic-single-status",
            "single-status-child",
            &DependencyType::ParentChild,
            "test",
        )
        .unwrap();

    // Compute epic status
    let epic_issue = storage.get_issue("epic-single-status").unwrap().unwrap();
    let children = storage.get_dependencies("epic-single-status").unwrap();

    let closed_children = children
        .iter()
        .filter(|d| match storage.get_issue(&d.depends_on_id) {
            Ok(Some(child)) => child.status == Status::Closed,
            _ => false,
        })
        .count();

    let epic_status = EpicStatus {
        epic: epic_issue,
        total_children: children.len(),
        closed_children,
        eligible_for_close: closed_children == children.len() && children.len() > 0,
    };

    // Verify status computation works with single label
    assert_eq!(epic_status.total_children, 1);
    assert_eq!(epic_status.closed_children, 1);
    assert!(epic_status.eligible_for_close);
    assert_eq!(epic_status.epic.labels.len(), 1);
    assert_eq!(epic_status.epic.labels[0], "feature");
    assert_eq!(epic_status.epic.priority, Priority::CRITICAL);
}

#[test]
fn test_epic_single_label_add_and_remove() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with single label
    let epic = Issue {
        id: "epic-single-add-remove".to_string(),
        title: "Single Label Add/Remove Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["initial".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Verify initial single label
    let retrieved = storage
        .get_issue("epic-single-add-remove")
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.labels.len(), 1);
    assert_eq!(retrieved.labels[0], "initial");

    // Add another label
    storage
        .add_label("epic-single-add-remove", "second")
        .unwrap();

    // Verify now has two labels
    let retrieved = storage
        .get_issue("epic-single-add-remove")
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"initial".to_string()));
    assert!(retrieved.labels.contains(&"second".to_string()));

    // Remove the initial label
    storage
        .remove_label("epic-single-add-remove", "initial")
        .unwrap();

    // Verify back to single label
    let retrieved = storage
        .get_issue("epic-single-add-remove")
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.labels.len(), 1);
    assert_eq!(retrieved.labels[0], "second");
}

#[test]
fn test_multiple_epics_different_single_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple epics each with different single label
    let epics = vec![
        ("epic-single-1", "frontend"),
        ("epic-single-2", "backend"),
        ("epic-single-3", "database"),
        ("epic-single-4", "ui"),
    ];

    for (id, label) in &epics {
        let epic = Issue {
            id: id.to_string(),
            title: format!("Single Label Epic {}", id),
            issue_type: IssueType::Epic,
            status: Status::Open,
            labels: vec![label.to_string()],
            ..Default::default()
        };
        storage.create_issue(&epic).unwrap();
    }

    // Verify each epic has exactly one unique label
    for (id, label) in &epics {
        let retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(retrieved.labels.len(), 1);
        assert_eq!(retrieved.labels[0], *label);
    }

    // Verify global label aggregation
    let all_labels = storage.list_all_labels().unwrap();
    assert_eq!(all_labels.len(), 4);

    let label_map: std::collections::HashMap<String, i64> = all_labels.into_iter().collect();
    assert_eq!(label_map.get("frontend"), Some(&1));
    assert_eq!(label_map.get("backend"), Some(&1));
    assert_eq!(label_map.get("database"), Some(&1));
    assert_eq!(label_map.get("ui"), Some(&1));
}

#[test]
fn test_epic_single_label_with_priority() {
    // Create epic with single label and each priority level
    let priorities = vec![
        (Priority::CRITICAL, "critical-feature"),
        (Priority::HIGH, "high-feature"),
        (Priority::MEDIUM, "medium-feature"),
        (Priority::LOW, "low-feature"),
        (Priority::BACKLOG, "backlog-feature"),
    ];

    for (priority, label) in priorities {
        let epic = Issue {
            id: format!("epic-prio-{}", label),
            title: format!("Priority Epic {}", label),
            issue_type: IssueType::Epic,
            status: Status::Open,
            labels: vec![label.to_string()],
            priority,
            ..Default::default()
        };

        // Verify single label and priority coexist correctly
        assert_eq!(epic.labels.len(), 1);
        assert_eq!(epic.labels[0], label);
        assert_eq!(epic.priority, priority);
    }
}

#[test]
fn test_epic_single_label_json_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with single label
    let epic = Issue {
        id: "epic-single-roundtrip".to_string(),
        title: "Single Label Roundtrip Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::InProgress,
        labels: vec!["roundtrip-test".to_string()],
        priority: Priority::HIGH,
        description: Some("Testing single label roundtrip".to_string()),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Retrieve and serialize
    let retrieved = storage.get_issue("epic-single-roundtrip").unwrap().unwrap();
    let json = serde_json::to_string(&retrieved).unwrap();

    // Deserialize
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    // Verify all fields preserved
    assert_eq!(deserialized.id, "epic-single-roundtrip");
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.status, Status::InProgress);
    assert_eq!(deserialized.labels.len(), 1);
    assert_eq!(deserialized.labels[0], "roundtrip-test");
    assert_eq!(deserialized.priority, Priority::HIGH);
    assert_eq!(
        deserialized.description,
        Some("Testing single label roundtrip".to_string())
    );
}

#[test]
fn test_epic_single_label_with_closed_children() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with single label
    let epic = Issue {
        id: "epic-single-closed".to_string(),
        title: "Single Label Closed Children Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["completed".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create and close multiple children
    for i in 1..=3 {
        let mut child = Issue {
            id: format!("closed-child-{}", i),
            title: format!("Closed Child {}", i),
            issue_type: IssueType::Task,
            status: Status::Closed,
            labels: vec![],
            ..Default::default()
        };
        child.closed_at = Some(Utc::now());
        storage.create_issue(&child).unwrap();
        storage
            .add_dependency(
                "epic-single-closed",
                &format!("closed-child-{}", i),
                &DependencyType::ParentChild,
                "test",
            )
            .unwrap();
    }

    // Verify epic still has only single label
    let epic_retrieved = storage.get_issue("epic-single-closed").unwrap().unwrap();
    assert_eq!(epic_retrieved.labels.len(), 1);
    assert_eq!(epic_retrieved.labels[0], "completed");

    // Verify all children are closed
    let children = storage.get_dependencies("epic-single-closed").unwrap();
    assert_eq!(children.len(), 3);

    let closed_count = children
        .iter()
        .filter(|d| match storage.get_issue(&d.depends_on_id) {
            Ok(Some(child)) => child.status == Status::Closed,
            _ => false,
        })
        .count();

    assert_eq!(closed_count, 3);
}

#[test]
fn test_epic_single_label_get_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with single label
    let epic = Issue {
        id: "epic-get-single".to_string(),
        title: "Get Single Label Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["retrieval".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Get labels specifically for epic
    let epic_labels = storage.get_labels("epic-get-single").unwrap();
    assert_eq!(epic_labels.len(), 1);
    assert!(epic_labels.contains(&"retrieval".to_string()));
}

#[test]
fn test_epic_single_label_various_types() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with single label
    let epic = Issue {
        id: "epic-single-various".to_string(),
        title: "Single Label Various Types Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["mixed-types".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children of various types with single labels
    let child_types = vec![
        ("task-child", IssueType::Task, "task-label"),
        ("bug-child", IssueType::Bug, "bug-label"),
        ("feature-child", IssueType::Feature, "feature-label"),
        ("chore-child", IssueType::Chore, "chore-label"),
        ("docs-child", IssueType::Docs, "docs-label"),
    ];

    for (id, issue_type, label) in &child_types {
        let child = Issue {
            id: id.to_string(),
            title: format!("Child {}", id),
            issue_type: issue_type.clone(),
            status: Status::Open,
            labels: vec![label.to_string()],
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();
        storage
            .add_dependency(
                "epic-single-various",
                id,
                &DependencyType::ParentChild,
                "test",
            )
            .unwrap();
    }

    // Verify epic still has only its single label
    let epic_retrieved = storage.get_issue("epic-single-various").unwrap().unwrap();
    assert_eq!(epic_retrieved.labels.len(), 1);
    assert_eq!(epic_retrieved.labels[0], "mixed-types");

    // Verify each child has its single label
    for (id, _, label) in child_types {
        let child_retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(child_retrieved.labels.len(), 1);
        assert_eq!(child_retrieved.labels[0], label);
    }

    // Verify global label count
    let all_labels = storage.list_all_labels().unwrap();
    assert_eq!(all_labels.len(), 6); // mixed-types + 5 child labels
}
