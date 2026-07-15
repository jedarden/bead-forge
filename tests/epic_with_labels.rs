// Epic with Labels Tests
// Tests that epics can have labels and labels work correctly with epic functionality

use bead_forge::model::{DependencyType, EpicStatus, Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;

#[test]
fn test_epic_creation_with_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with labels
    let epic = Issue {
        id: "epic-labels".to_string(),
        title: "Epic with Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec![
            "feature".to_string(),
            "frontend".to_string(),
            "high-priority".to_string(),
        ],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Retrieve and verify labels are preserved
    let retrieved = storage.get_issue("epic-labels").unwrap().unwrap();
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.labels.len(), 3);
    assert!(retrieved.labels.contains(&"feature".to_string()));
    assert!(retrieved.labels.contains(&"frontend".to_string()));
    assert!(retrieved.labels.contains(&"high-priority".to_string()));
}

#[test]
fn test_epic_children_with_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with labels
    let epic = Issue {
        id: "epic-children".to_string(),
        title: "Epic with Children".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["feature".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children with different labels
    let child1 = Issue {
        id: "child-1".to_string(),
        title: "Child 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["bug".to_string(), "urgent".to_string()],
        ..Default::default()
    };
    storage.create_issue(&child1).unwrap();
    storage
        .add_dependency(
            "epic-children",
            "child-1",
            &DependencyType::ParentChild,
            "test",
        )
        .unwrap();

    let child2 = Issue {
        id: "child-2".to_string(),
        title: "Child 2".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["frontend".to_string()],
        ..Default::default()
    };
    storage.create_issue(&child2).unwrap();
    storage
        .add_dependency(
            "epic-children",
            "child-2",
            &DependencyType::ParentChild,
            "test",
        )
        .unwrap();

    // Verify epic and children have their own labels
    let epic_retrieved = storage.get_issue("epic-children").unwrap().unwrap();
    assert_eq!(epic_retrieved.labels.len(), 1);
    assert!(epic_retrieved.labels.contains(&"feature".to_string()));

    let child1_retrieved = storage.get_issue("child-1").unwrap().unwrap();
    assert_eq!(child1_retrieved.labels.len(), 2);
    assert!(child1_retrieved.labels.contains(&"bug".to_string()));
    assert!(child1_retrieved.labels.contains(&"urgent".to_string()));

    let child2_retrieved = storage.get_issue("child-2").unwrap().unwrap();
    assert_eq!(child2_retrieved.labels.len(), 1);
    assert!(child2_retrieved.labels.contains(&"frontend".to_string()));
}

#[test]
fn test_epic_labels_serialization() {
    // Create epic with labels
    let epic = Issue {
        id: "epic-serialize-labels".to_string(),
        title: "Serialization Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["feature".to_string(), "backend".to_string()],
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&epic).unwrap();

    // Verify epic type and labels are serialized
    assert!(json.contains(r#""issue_type":"epic""#));
    assert!(json.contains(r#""labels":["feature","backend"]"#));

    // Deserialize and verify
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.labels.len(), 2);
    assert!(deserialized.labels.contains(&"feature".to_string()));
    assert!(deserialized.labels.contains(&"backend".to_string()));
}

#[test]
fn test_epic_with_labels_aggregation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with labels
    let epic = Issue {
        id: "epic-aggregation".to_string(),
        title: "Aggregation Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["feature".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children with overlapping labels
    for i in 1..=3 {
        let child = Issue {
            id: format!("agg-{}", i),
            title: format!("Child {}", i),
            issue_type: IssueType::Task,
            status: Status::Open,
            labels: vec!["bug".to_string(), "urgent".to_string()],
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();
        storage
            .add_dependency(
                "epic-aggregation",
                &format!("agg-{}", i),
                &DependencyType::ParentChild,
                "test",
            )
            .unwrap();
    }

    // List all labels globally
    let all_labels = storage.list_all_labels().unwrap();

    // Should have: feature (1), bug (3), urgent (3)
    assert_eq!(all_labels.len(), 3);

    let label_map: std::collections::HashMap<String, i64> = all_labels.into_iter().collect();
    assert_eq!(label_map.get("feature"), Some(&1));
    assert_eq!(label_map.get("bug"), Some(&3));
    assert_eq!(label_map.get("urgent"), Some(&3));
}

#[test]
fn test_epic_status_computation_with_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with labels
    let epic = Issue {
        id: "epic-status-labels".to_string(),
        title: "Status Labels Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["feature".to_string(), "in-progress".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children with labels
    for i in 1..=2 {
        let child = Issue {
            id: format!("status-child-{}", i),
            title: format!("Child {}", i),
            issue_type: IssueType::Task,
            status: Status::Open,
            labels: vec!["task".to_string()],
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();
        storage
            .add_dependency(
                "epic-status-labels",
                &format!("status-child-{}", i),
                &DependencyType::ParentChild,
                "test",
            )
            .unwrap();
    }

    // Compute epic status
    let epic_issue = storage.get_issue("epic-status-labels").unwrap().unwrap();
    let children = storage.get_dependencies("epic-status-labels").unwrap();

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

    // Verify status computation works regardless of labels
    assert_eq!(epic_status.total_children, 2);
    assert_eq!(epic_status.closed_children, 0);
    assert!(!epic_status.eligible_for_close);

    // Verify epic labels are preserved in status
    assert_eq!(epic_status.epic.labels.len(), 2);
    assert!(epic_status.epic.labels.contains(&"feature".to_string()));
    assert!(epic_status.epic.labels.contains(&"in-progress".to_string()));
}

#[test]
fn test_multiple_epics_with_distinct_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple epics with different labels
    let epics = vec![
        (
            "epic-1",
            vec!["feature".to_string(), "frontend".to_string()],
        ),
        ("epic-2", vec!["bug".to_string(), "urgent".to_string()]),
        (
            "epic-3",
            vec!["refactor".to_string(), "tech-debt".to_string()],
        ),
    ];

    for (id, labels) in &epics {
        let epic = Issue {
            id: id.to_string(),
            title: format!("Epic {}", id),
            issue_type: IssueType::Epic,
            status: Status::Open,
            labels: labels.clone(),
            ..Default::default()
        };
        storage.create_issue(&epic).unwrap();
    }

    // Verify each epic has its own labels
    for (id, labels) in &epics {
        let retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(retrieved.labels.len(), labels.len());
        for label in labels {
            assert!(retrieved.labels.contains(label));
        }
    }

    // Verify global label aggregation
    let all_labels = storage.list_all_labels().unwrap();
    assert_eq!(all_labels.len(), 6); // All unique labels
}

#[test]
fn test_epic_with_no_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic without labels
    let epic = Issue {
        id: "epic-no-labels".to_string(),
        title: "Epic No Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Verify epic has no labels
    let retrieved = storage.get_issue("epic-no-labels").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 0);
    assert_eq!(retrieved.issue_type, IssueType::Epic);
}

#[test]
fn test_epic_labels_update() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with initial labels
    let epic = Issue {
        id: "epic-update-labels".to_string(),
        title: "Update Labels Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["feature".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Update epic labels
    storage.add_label("epic-update-labels", "urgent").unwrap();
    storage.add_label("epic-update-labels", "backend").unwrap();

    // Verify labels were added
    let retrieved = storage.get_issue("epic-update-labels").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 3);
    assert!(retrieved.labels.contains(&"feature".to_string()));
    assert!(retrieved.labels.contains(&"urgent".to_string()));
    assert!(retrieved.labels.contains(&"backend".to_string()));

    // Remove a label
    storage
        .remove_label("epic-update-labels", "feature")
        .unwrap();

    // Verify label was removed
    let retrieved = storage.get_issue("epic-update-labels").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 2);
    assert!(!retrieved.labels.contains(&"feature".to_string()));
}

#[test]
fn test_epic_hierarchy_with_label_propagation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with labels
    let epic = Issue {
        id: "epic-hierarchy".to_string(),
        title: "Hierarchy Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["feature".to_string(), "large".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children with their own labels
    let child1 = Issue {
        id: "hierarchy-child-1".to_string(),
        title: "Child 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["frontend".to_string()],
        ..Default::default()
    };
    storage.create_issue(&child1).unwrap();
    storage
        .add_dependency(
            "epic-hierarchy",
            "hierarchy-child-1",
            &DependencyType::ParentChild,
            "test",
        )
        .unwrap();

    let child2 = Issue {
        id: "hierarchy-child-2".to_string(),
        title: "Child 2".to_string(),
        issue_type: IssueType::Bug,
        status: Status::Open,
        labels: vec!["urgent".to_string(), "bug".to_string()],
        ..Default::default()
    };
    storage.create_issue(&child2).unwrap();
    storage
        .add_dependency(
            "epic-hierarchy",
            "hierarchy-child-2",
            &DependencyType::ParentChild,
            "test",
        )
        .unwrap();

    // Verify labels don't propagate (each issue has its own labels)
    let epic_retrieved = storage.get_issue("epic-hierarchy").unwrap().unwrap();
    assert_eq!(epic_retrieved.labels.len(), 2);

    let child1_retrieved = storage.get_issue("hierarchy-child-1").unwrap().unwrap();
    assert_eq!(child1_retrieved.labels.len(), 1);

    let child2_retrieved = storage.get_issue("hierarchy-child-2").unwrap().unwrap();
    assert_eq!(child2_retrieved.labels.len(), 2);

    // Verify global label count includes all labels from all issues
    let all_labels = storage.list_all_labels().unwrap();
    assert_eq!(all_labels.len(), 5); // feature, large, frontend, urgent, bug
}

#[test]
fn test_epic_labels_with_closed_children() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with labels
    let epic = Issue {
        id: "epic-closed-labels".to_string(),
        title: "Closed Labels Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["feature".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create and close children
    let mut child = Issue {
        id: "closed-child-labels".to_string(),
        title: "Closed Child".to_string(),
        issue_type: IssueType::Task,
        status: Status::Closed,
        labels: vec!["completed".to_string()],
        ..Default::default()
    };
    child.closed_at = Some(Utc::now());
    storage.create_issue(&child).unwrap();
    storage
        .add_dependency(
            "epic-closed-labels",
            "closed-child-labels",
            &DependencyType::ParentChild,
            "test",
        )
        .unwrap();

    // Verify labels persist on closed issues
    let all_labels = storage.list_all_labels().unwrap();
    assert_eq!(all_labels.len(), 2); // feature, completed

    let label_map: std::collections::HashMap<String, i64> = all_labels.into_iter().collect();
    assert_eq!(label_map.get("feature"), Some(&1));
    assert_eq!(label_map.get("completed"), Some(&1));
}

#[test]
fn test_epic_default_priority_with_labels() {
    // Create epic with labels and default priority
    let epic = Issue {
        id: "epic-priority-labels".to_string(),
        title: "Priority Labels Epic".to_string(),
        issue_type: IssueType::Epic,
        labels: vec!["feature".to_string(), "high-priority".to_string()],
        ..Default::default()
    };

    // Verify default priority and labels coexist
    assert_eq!(epic.priority, Priority::MEDIUM);
    assert_eq!(epic.labels.len(), 2);
    assert!(epic.labels.contains(&"feature".to_string()));
    assert!(epic.labels.contains(&"high-priority".to_string()));
}

#[test]
fn test_epic_get_labels_with_children() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with labels
    let epic = Issue {
        id: "epic-get-labels".to_string(),
        title: "Get Labels Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["feature".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create child with different labels
    let child = Issue {
        id: "get-labels-child".to_string(),
        title: "Get Labels Child".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["bug".to_string()],
        ..Default::default()
    };
    storage.create_issue(&child).unwrap();
    storage
        .add_dependency(
            "epic-get-labels",
            "get-labels-child",
            &DependencyType::ParentChild,
            "test",
        )
        .unwrap();

    // Get labels specifically for epic
    let epic_labels = storage.get_labels("epic-get-labels").unwrap();
    assert_eq!(epic_labels.len(), 1);
    assert!(epic_labels.contains(&"feature".to_string()));

    // Get labels for child
    let child_labels = storage.get_labels("get-labels-child").unwrap();
    assert_eq!(child_labels.len(), 1);
    assert!(child_labels.contains(&"bug".to_string()));
}
