// Epic 6: Complex Labels Tests
// Tests epics with many labels (4+), complex label interactions, and edge cases

use bead_forge::model::{Issue, IssueType, Status, DependencyType, EpicStatus, Priority, IssueChanges};
use bead_forge::storage::Storage;
use chrono::Utc;

#[test]
fn test_epic_with_four_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with 4 labels (matching the bead's scenario)
    let epic = Issue {
        id: "epic-four-labels".to_string(),
        title: "Epic with Four Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec![
            "api".to_string(),
            "breaking-change".to_string(),
            "critical".to_string(),
            "feature".to_string(),
        ],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Retrieve and verify all 4 labels are preserved
    let retrieved = storage.get_issue("epic-four-labels").unwrap().unwrap();
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.labels.len(), 4);
    assert!(retrieved.labels.contains(&"api".to_string()));
    assert!(retrieved.labels.contains(&"breaking-change".to_string()));
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"feature".to_string()));
}

#[test]
fn test_epic_with_many_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with 10 labels
    let many_labels = vec![
        "api".to_string(),
        "backend".to_string(),
        "breaking-change".to_string(),
        "critical".to_string(),
        "feature".to_string(),
        "high-priority".to_string(),
        "performance".to_string(),
        "security".to_string(),
        "tech-debt".to_string(),
        "urgent".to_string(),
    ];

    let epic = Issue {
        id: "epic-many-labels".to_string(),
        title: "Epic with Many Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: many_labels.clone(),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Retrieve and verify all labels are preserved
    let retrieved = storage.get_issue("epic-many-labels").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 10);
    for label in &many_labels {
        assert!(retrieved.labels.contains(label));
    }
}

#[test]
fn test_epic_complex_label_serialization() {
    // Create epic with 4 labels
    let epic = Issue {
        id: "epic-complex-serialize".to_string(),
        title: "Complex Label Serialization Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec![
            "api".to_string(),
            "breaking-change".to_string(),
            "critical".to_string(),
            "feature".to_string(),
        ],
        priority: Priority::CRITICAL,
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&epic).unwrap();

    // Verify epic type and all labels are serialized
    assert!(json.contains(r#""issue_type":"epic""#));
    assert!(json.contains(r#""labels":["api","breaking-change","critical","feature"]"#));

    // Deserialize and verify
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.labels.len(), 4);
    assert_eq!(deserialized.priority, Priority::CRITICAL);
}

#[test]
fn test_epic_complex_labels_with_children() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with 4 labels
    let epic = Issue {
        id: "epic-complex-children".to_string(),
        title: "Complex Labels with Children".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec![
            "api".to_string(),
            "breaking-change".to_string(),
            "critical".to_string(),
            "feature".to_string(),
        ],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children with different label combinations
    let child1 = Issue {
        id: "child-complex-1".to_string(),
        title: "Child 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["urgent".to_string(), "bug".to_string()],
        ..Default::default()
    };
    storage.create_issue(&child1).unwrap();
    storage.add_dependency("epic-complex-children", "child-complex-1", &DependencyType::ParentChild, "test").unwrap();

    let child2 = Issue {
        id: "child-complex-2".to_string(),
        title: "Child 2".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["docs".to_string()],
        ..Default::default()
    };
    storage.create_issue(&child2).unwrap();
    storage.add_dependency("epic-complex-children", "child-complex-2", &DependencyType::ParentChild, "test").unwrap();

    // Verify epic still has all 4 labels
    let epic_retrieved = storage.get_issue("epic-complex-children").unwrap().unwrap();
    assert_eq!(epic_retrieved.labels.len(), 4);

    // Verify children have their own labels
    let child1_retrieved = storage.get_issue("child-complex-1").unwrap().unwrap();
    assert_eq!(child1_retrieved.labels.len(), 2);

    let child2_retrieved = storage.get_issue("child-complex-2").unwrap().unwrap();
    assert_eq!(child2_retrieved.labels.len(), 1);

    // Verify global label aggregation
    let all_labels = storage.list_all_labels().unwrap();
    assert_eq!(all_labels.len(), 7); // 4 epic labels + 2 child1 labels + 1 child2 label
}

#[test]
fn test_epic_complex_labels_aggregation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with 4 labels
    let epic = Issue {
        id: "epic-complex-aggregation".to_string(),
        title: "Complex Aggregation Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec![
            "api".to_string(),
            "breaking-change".to_string(),
            "critical".to_string(),
            "feature".to_string(),
        ],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children with overlapping labels
    for i in 1..=5 {
        let child = Issue {
            id: format!("agg-child-{}", i),
            title: format!("Agg Child {}", i),
            issue_type: IssueType::Task,
            status: Status::Open,
            labels: vec!["api".to_string(), "bug".to_string()],
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();
        storage.add_dependency("epic-complex-aggregation", &format!("agg-child-{}", i), &DependencyType::ParentChild, "test").unwrap();
    }

    // List all labels globally
    let all_labels = storage.list_all_labels().unwrap();

    // Should have: api (6), breaking-change (1), critical (1), feature (1), bug (5)
    assert_eq!(all_labels.len(), 5);

    let label_map: std::collections::HashMap<String, i64> = all_labels.into_iter().collect();
    assert_eq!(label_map.get("api"), Some(&6));    // 1 epic + 5 children
    assert_eq!(label_map.get("breaking-change"), Some(&1));
    assert_eq!(label_map.get("critical"), Some(&1));
    assert_eq!(label_map.get("feature"), Some(&1));
    assert_eq!(label_map.get("bug"), Some(&5));    // 5 children
}

#[test]
fn test_epic_complex_labels_status_computation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with 4 labels
    let epic = Issue {
        id: "epic-complex-status".to_string(),
        title: "Complex Status Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec![
            "api".to_string(),
            "breaking-change".to_string(),
            "critical".to_string(),
            "feature".to_string(),
        ],
        priority: Priority::CRITICAL,
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create and close some children
    for i in 1..=3 {
        let mut child = Issue {
            id: format!("complex-status-child-{}", i),
            title: format!("Child {}", i),
            issue_type: IssueType::Task,
            status: Status::Closed,
            labels: vec![],
            ..Default::default()
        };
        child.closed_at = Some(Utc::now());
        storage.create_issue(&child).unwrap();
        storage.add_dependency("epic-complex-status", &format!("complex-status-child-{}", i), &DependencyType::ParentChild, "test").unwrap();
    }

    // Compute epic status
    let epic_issue = storage.get_issue("epic-complex-status").unwrap().unwrap();
    let children = storage.get_dependencies("epic-complex-status").unwrap();

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

    // Verify status computation works with complex labels
    assert_eq!(epic_status.total_children, 3);
    assert_eq!(epic_status.closed_children, 3);
    assert!(epic_status.eligible_for_close);
    assert_eq!(epic_status.epic.labels.len(), 4);
    assert_eq!(epic_status.epic.priority, Priority::CRITICAL);
}

#[test]
fn test_epic_complex_labels_add_and_remove() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with 4 labels
    let epic = Issue {
        id: "epic-complex-add-remove".to_string(),
        title: "Complex Add/Remove Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec![
            "api".to_string(),
            "critical".to_string(),
            "feature".to_string(),
            "urgent".to_string(),
        ],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Verify initial state
    let retrieved = storage.get_issue("epic-complex-add-remove").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 4);

    // Add more labels
    storage.add_label("epic-complex-add-remove", "breaking-change").unwrap();
    storage.add_label("epic-complex-add-remove", "security").unwrap();

    // Verify now has 6 labels
    let retrieved = storage.get_issue("epic-complex-add-remove").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 6);

    // Remove some labels
    storage.remove_label("epic-complex-add-remove", "urgent").unwrap();
    storage.remove_label("epic-complex-add-remove", "feature").unwrap();

    // Verify back to 4 labels
    let retrieved = storage.get_issue("epic-complex-add-remove").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 4);
    assert!(retrieved.labels.contains(&"api".to_string()));
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"breaking-change".to_string()));
    assert!(retrieved.labels.contains(&"security".to_string()));
}

#[test]
fn test_multiple_epics_with_complex_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple epics each with different complex label sets
    let epics = vec![
        ("epic-complex-1", vec!["api", "backend", "critical", "feature"]),
        ("epic-complex-2", vec!["bug", "urgent", "frontend", "performance"]),
        ("epic-complex-3", vec!["docs", "low-priority", "refactor", "tech-debt"]),
    ];

    for (id, labels) in &epics {
        let epic = Issue {
            id: id.to_string(),
            title: format!("Complex Labels Epic {}", id),
            issue_type: IssueType::Epic,
            status: Status::Open,
            labels: labels.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        storage.create_issue(&epic).unwrap();
    }

    // Verify each epic has all its labels
    for (id, labels) in &epics {
        let retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(retrieved.labels.len(), labels.len());
        for label in labels {
            assert!(retrieved.labels.contains(&label.to_string()));
        }
    }

    // Verify global label aggregation
    let all_labels = storage.list_all_labels().unwrap();
    assert_eq!(all_labels.len(), 12); // All unique labels
}

#[test]
fn test_epic_complex_labels_with_special_characters() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with labels containing special characters
    let epic = Issue {
        id: "epic-special-labels".to_string(),
        title: "Special Character Labels Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec![
            "high-priority".to_string(),
            "needs-review".to_string(),
            "API:breaking".to_string(),
            "bug-fix-2.0".to_string(),
        ],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Retrieve and verify all special character labels are preserved
    let retrieved = storage.get_issue("epic-special-labels").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 4);
    assert!(retrieved.labels.contains(&"high-priority".to_string()));
    assert!(retrieved.labels.contains(&"needs-review".to_string()));
    assert!(retrieved.labels.contains(&"API:breaking".to_string()));
    assert!(retrieved.labels.contains(&"bug-fix-2.0".to_string()));
}

#[test]
fn test_epic_complex_labels_json_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with 4 labels
    let epic = Issue {
        id: "epic-complex-roundtrip".to_string(),
        title: "Complex Labels Roundtrip Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::InProgress,
        labels: vec![
            "api".to_string(),
            "breaking-change".to_string(),
            "critical".to_string(),
            "feature".to_string(),
        ],
        priority: Priority::HIGH,
        description: Some("Testing complex label roundtrip".to_string()),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Retrieve and serialize
    let retrieved = storage.get_issue("epic-complex-roundtrip").unwrap().unwrap();
    let json = serde_json::to_string(&retrieved).unwrap();

    // Deserialize
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    // Verify all fields preserved including all labels
    assert_eq!(deserialized.id, "epic-complex-roundtrip");
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.status, Status::InProgress);
    assert_eq!(deserialized.labels.len(), 4);
    assert_eq!(deserialized.priority, Priority::HIGH);
    assert_eq!(deserialized.description, Some("Testing complex label roundtrip".to_string()));
}

#[test]
fn test_epic_complex_labels_get_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with 4 labels
    let epic = Issue {
        id: "epic-get-complex".to_string(),
        title: "Get Complex Labels Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec![
            "api".to_string(),
            "breaking-change".to_string(),
            "critical".to_string(),
            "feature".to_string(),
        ],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Get labels specifically for epic
    let epic_labels = storage.get_labels("epic-get-complex").unwrap();
    assert_eq!(epic_labels.len(), 4);
    assert!(epic_labels.contains(&"api".to_string()));
    assert!(epic_labels.contains(&"breaking-change".to_string()));
    assert!(epic_labels.contains(&"critical".to_string()));
    assert!(epic_labels.contains(&"feature".to_string()));
}

#[test]
fn test_epic_complex_labels_update_via_changes() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with 4 labels
    let epic = Issue {
        id: "epic-complex-update".to_string(),
        title: "Complex Labels Update Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec![
            "api".to_string(),
            "breaking-change".to_string(),
            "critical".to_string(),
            "feature".to_string(),
        ],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Verify initial state
    let retrieved = storage.get_issue("epic-complex-update").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 4);

    // Update labels via IssueChanges (replace entire list)
    let new_labels = vec![
        "urgent".to_string(),
        "bug".to_string(),
        "security".to_string(),
    ];
    let changes = IssueChanges {
        labels: Some(new_labels.clone()),
        ..Default::default()
    };
    storage.update_issue("epic-complex-update", &changes).unwrap();

    // Verify labels were replaced
    let retrieved = storage.get_issue("epic-complex-update").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 3);
    for label in &new_labels {
        assert!(retrieved.labels.contains(label));
    }
    assert!(!retrieved.labels.contains(&"api".to_string()));
    assert!(!retrieved.labels.contains(&"breaking-change".to_string()));
}

#[test]
fn test_epic_complex_labels_with_closed_children() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with 4 labels
    let epic = Issue {
        id: "epic-complex-closed".to_string(),
        title: "Complex Labels Closed Children Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec![
            "api".to_string(),
            "breaking-change".to_string(),
            "critical".to_string(),
            "feature".to_string(),
        ],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create and close multiple children
    for i in 1..=5 {
        let mut child = Issue {
            id: format!("complex-closed-child-{}", i),
            title: format!("Closed Child {}", i),
            issue_type: IssueType::Task,
            status: Status::Closed,
            labels: vec!["completed".to_string()],
            ..Default::default()
        };
        child.closed_at = Some(Utc::now());
        storage.create_issue(&child).unwrap();
        storage.add_dependency("epic-complex-closed", &format!("complex-closed-child-{}", i), &DependencyType::ParentChild, "test").unwrap();
    }

    // Verify epic still has all 4 labels
    let epic_retrieved = storage.get_issue("epic-complex-closed").unwrap().unwrap();
    assert_eq!(epic_retrieved.labels.len(), 4);

    // Verify all children are closed
    let children = storage.get_dependencies("epic-complex-closed").unwrap();
    assert_eq!(children.len(), 5);

    let closed_count = children.iter().filter(|d| {
        match storage.get_issue(&d.depends_on_id) {
            Ok(Some(child)) => child.status == Status::Closed,
            _ => false,
        }
    }).count();

    assert_eq!(closed_count, 5);

    // Verify global label aggregation includes closed children
    let all_labels = storage.list_all_labels().unwrap();
    assert_eq!(all_labels.len(), 5); // 4 epic labels + 1 child label
}

#[test]
fn test_epic_complex_labels_with_various_types() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with 4 labels
    let epic = Issue {
        id: "epic-complex-types".to_string(),
        title: "Complex Labels Various Types Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec![
            "api".to_string(),
            "breaking-change".to_string(),
            "critical".to_string(),
            "feature".to_string(),
        ],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children of various types with complex labels
    let child_types = vec![
        ("task-child", IssueType::Task, vec!["urgent", "bug", "performance"]),
        ("bug-child", IssueType::Bug, vec!["critical", "security", "api"]),
        ("feature-child", IssueType::Feature, vec!["feature", "frontend", "high-priority"]),
        ("chore-child", IssueType::Chore, vec!["low-priority", "docs"]),
        ("docs-child", IssueType::Docs, vec!["documentation", "review-needed"]),
    ];

    for (id, issue_type, labels) in &child_types {
        let child = Issue {
            id: id.to_string(),
            title: format!("Child {}", id),
            issue_type: issue_type.clone(),
            status: Status::Open,
            labels: labels.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();
        storage.add_dependency("epic-complex-types", id, &DependencyType::ParentChild, "test").unwrap();
    }

    // Verify epic still has all 4 labels
    let epic_retrieved = storage.get_issue("epic-complex-types").unwrap().unwrap();
    assert_eq!(epic_retrieved.labels.len(), 4);

    // Verify each child has its labels
    for (id, _, labels) in &child_types {
        let child_retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(child_retrieved.labels.len(), labels.len());
    }

    // Verify global label count includes all labels (with overlaps counted once)
    let all_labels = storage.list_all_labels().unwrap();
    // Epic: api, breaking-change, critical, feature (4)
    // Task: urgent, bug, performance (3 new)
    // Bug: security (1 new - api, critical already exist)
    // Feature: frontend, high-priority (2 new - feature already exists)
    // Chore: low-priority, docs (2 new)
    // Docs: documentation, review-needed (2 new)
    // Total: 4 + 3 + 1 + 2 + 2 + 2 = 14 unique labels
    assert_eq!(all_labels.len(), 14);
}

#[test]
fn test_epic_label_edge_case_duplicate_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with labels
    let epic = Issue {
        id: "epic-duplicate-test".to_string(),
        title: "Duplicate Label Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec!["feature".to_string(), "urgent".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Try to add a duplicate label
    let result = storage.add_label("epic-duplicate-test", "feature");

    // Should either succeed (no-op) or fail - verify idempotency
    if result.is_ok() {
        let retrieved = storage.get_issue("epic-duplicate-test").unwrap().unwrap();
        // Should still have only 2 unique labels (no duplicate)
        let unique_labels: std::collections::HashSet<_> = retrieved.labels.iter().collect();
        assert_eq!(unique_labels.len(), 2);
    }
}

#[test]
fn test_epic_label_edge_case_empty_label_removal() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with 4 labels
    let epic = Issue {
        id: "epic-empty-removal".to_string(),
        title: "Empty Label Removal Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec![
            "api".to_string(),
            "breaking-change".to_string(),
            "critical".to_string(),
            "feature".to_string(),
        ],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Try to remove a non-existent label
    let result = storage.remove_label("epic-empty-removal", "nonexistent");

    // Should either succeed (no-op) or fail gracefully
    // Verify original labels are unchanged
    let retrieved = storage.get_issue("epic-empty-removal").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 4);
}

#[test]
fn test_epic_complex_labels_ordering_preservation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with labels in specific order
    let labels_in_order = vec![
        "api".to_string(),
        "breaking-change".to_string(),
        "critical".to_string(),
        "feature".to_string(),
    ];

    let epic = Issue {
        id: "epic-order-test".to_string(),
        title: "Label Ordering Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: labels_in_order.clone(),
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Retrieve and verify
    let retrieved = storage.get_issue("epic-order-test").unwrap().unwrap();
    assert_eq!(retrieved.labels, labels_in_order);
}
