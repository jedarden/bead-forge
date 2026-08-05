// P0 Epic with Labels Tests
// Tests creating P0 (Critical Priority) epics with labels, verifying storage and serialization

use bead_forge::model::{DependencyType, EpicStatus, Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;

#[test]
fn test_p0_epic_creation_with_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 epic with labels
    let epic = Issue {
        id: "epic-p0-labels".to_string(),
        title: "Critical Priority Epic with Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL, // P0 = 0
        labels: vec![
            "critical".to_string(),
            "urgent".to_string(),
            "security".to_string(),
        ],
        description: Some("This is a critical priority epic with labels".to_string()),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Verify the epic was stored correctly
    let retrieved = storage.get_issue("epic-p0-labels").unwrap().unwrap();

    // Test 1: Verify ID matches
    assert_eq!(retrieved.id, "epic-p0-labels");

    // Test 2: Verify issue type is epic
    assert_eq!(retrieved.issue_type, IssueType::Epic);

    // Test 3: Verify priority is P0 (critical = 0)
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);

    // Test 4: Verify status
    assert_eq!(retrieved.status, Status::Open);

    // Test 5: Verify labels are preserved
    assert_eq!(retrieved.labels.len(), 3);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"urgent".to_string()));
    assert!(retrieved.labels.contains(&"security".to_string()));

    // Test 6: Verify description
    assert_eq!(
        retrieved.description,
        Some("This is a critical priority epic with labels".to_string())
    );
}

#[test]
fn test_p0_epic_with_labels_serialization() {
    // Create P0 epic with labels
    let epic = Issue {
        id: "epic-p0-labels-serialize".to_string(),
        title: "P0 Epic Labels Serialization Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "infrastructure".to_string()],
        ..Default::default()
    };

    // Test JSON serialization
    let json = serde_json::to_string(&epic).unwrap();

    // Verify epic type is serialized correctly
    assert!(json.contains(r#""issue_type":"epic""#));

    // Verify P0 priority is serialized as 0
    assert!(json.contains(r#""priority":0"#));

    // Verify labels are serialized
    assert!(json.contains(r#""labels":["critical","infrastructure"]"#));

    // Test deserialization
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.priority.0, 0);
    assert_eq!(deserialized.labels.len(), 2);
    assert!(deserialized.labels.contains(&"critical".to_string()));
    assert!(deserialized.labels.contains(&"infrastructure".to_string()));
    assert_eq!(deserialized.id, "epic-p0-labels-serialize");
}

#[test]
fn test_p0_epic_children_with_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 epic with labels
    let epic = Issue {
        id: "epic-p0-children".to_string(),
        title: "P0 Epic with Children and Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "feature".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children with different labels
    let child1 = Issue {
        id: "p0-child-1".to_string(),
        title: "Child 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["bug".to_string(), "urgent".to_string()],
        ..Default::default()
    };
    storage.create_issue(&child1).unwrap();
    storage
        .add_dependency(
            "epic-p0-children",
            "p0-child-1",
            &DependencyType::ParentChild,
            "test",
        )
        .unwrap();

    let child2 = Issue {
        id: "p0-child-2".to_string(),
        title: "Child 2".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["frontend".to_string()],
        priority: Priority::CRITICAL, // Child also P0
        ..Default::default()
    };
    storage.create_issue(&child2).unwrap();
    storage
        .add_dependency(
            "epic-p0-children",
            "p0-child-2",
            &DependencyType::ParentChild,
            "test",
        )
        .unwrap();

    // Verify epic and children have their own labels
    let epic_retrieved = storage.get_issue("epic-p0-children").unwrap().unwrap();
    assert_eq!(epic_retrieved.labels.len(), 2);
    assert!(epic_retrieved.labels.contains(&"critical".to_string()));
    assert!(epic_retrieved.labels.contains(&"feature".to_string()));
    assert_eq!(epic_retrieved.priority, Priority::CRITICAL);

    let child1_retrieved = storage.get_issue("p0-child-1").unwrap().unwrap();
    assert_eq!(child1_retrieved.labels.len(), 2);
    assert!(child1_retrieved.labels.contains(&"bug".to_string()));
    assert!(child1_retrieved.labels.contains(&"urgent".to_string()));

    let child2_retrieved = storage.get_issue("p0-child-2").unwrap().unwrap();
    assert_eq!(child2_retrieved.labels.len(), 1);
    assert!(child2_retrieved.labels.contains(&"frontend".to_string()));
    assert_eq!(child2_retrieved.priority, Priority::CRITICAL);
}

#[test]
fn test_p0_epic_with_labels_aggregation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 epic with labels
    let epic = Issue {
        id: "epic-p0-aggregation".to_string(),
        title: "P0 Aggregation Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "feature".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children with overlapping labels
    for i in 1..=3 {
        let child = Issue {
            id: format!("p0-agg-{}", i),
            title: format!("Child {}", i),
            issue_type: IssueType::Task,
            status: Status::Open,
            labels: vec!["bug".to_string(), "urgent".to_string()],
            priority: Priority::CRITICAL,
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();
        storage
            .add_dependency(
                "epic-p0-aggregation",
                &format!("p0-agg-{}", i),
                &DependencyType::ParentChild,
                "test",
            )
            .unwrap();
    }

    // List all labels globally
    let all_labels = storage.list_all_labels().unwrap();

    // Should have: critical (1), feature (1), bug (3), urgent (3)
    assert_eq!(all_labels.len(), 4);

    let label_map: std::collections::HashMap<String, i64> = all_labels.into_iter().collect();
    assert_eq!(label_map.get("critical"), Some(&1));
    assert_eq!(label_map.get("feature"), Some(&1));
    assert_eq!(label_map.get("bug"), Some(&3));
    assert_eq!(label_map.get("urgent"), Some(&3));
}

#[test]
fn test_p0_epic_status_computation_with_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 epic with labels
    let epic = Issue {
        id: "epic-p0-status-labels".to_string(),
        title: "P0 Status Labels Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "in-progress".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children with labels
    for i in 1..=2 {
        let child = Issue {
            id: format!("p0-status-child-{}", i),
            title: format!("Child {}", i),
            issue_type: IssueType::Task,
            status: Status::Open,
            labels: vec!["task".to_string()],
            priority: Priority::CRITICAL,
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();
        storage
            .add_dependency(
                "epic-p0-status-labels",
                &format!("p0-status-child-{}", i),
                &DependencyType::ParentChild,
                "test",
            )
            .unwrap();
    }

    // Compute epic status
    let epic_issue = storage.get_issue("epic-p0-status-labels").unwrap().unwrap();
    let children = storage.get_dependencies("epic-p0-status-labels").unwrap();

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
    assert!(epic_status.epic.labels.contains(&"critical".to_string()));
    assert!(epic_status.epic.labels.contains(&"in-progress".to_string()));

    // Verify P0 priority is preserved
    assert_eq!(epic_status.epic.priority, Priority::CRITICAL);
    assert_eq!(epic_status.epic.priority.0, 0);
}

#[test]
fn test_multiple_p0_epics_with_distinct_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple P0 epics with different labels
    let epics = vec![
        (
            "epic-p0-1",
            vec!["critical".to_string(), "frontend".to_string()],
        ),
        (
            "epic-p0-2",
            vec![
                "critical".to_string(),
                "urgent".to_string(),
                "bug".to_string(),
            ],
        ),
        (
            "epic-p0-3",
            vec![
                "critical".to_string(),
                "refactor".to_string(),
                "tech-debt".to_string(),
            ],
        ),
    ];

    for (id, labels) in &epics {
        let epic = Issue {
            id: id.to_string(),
            title: format!("P0 Epic {}", id),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority: Priority::CRITICAL,
            labels: labels.clone(),
            ..Default::default()
        };
        storage.create_issue(&epic).unwrap();
    }

    // Verify each epic has its own labels and P0 priority
    for (id, labels) in &epics {
        let retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(retrieved.labels.len(), labels.len());
        for label in labels {
            assert!(retrieved.labels.contains(label));
        }
        assert_eq!(retrieved.priority, Priority::CRITICAL);
        assert_eq!(retrieved.priority.0, 0);
    }

    // Verify global label aggregation
    let all_labels = storage.list_all_labels().unwrap();
    // All unique labels: critical (3), frontend (1), urgent (1), bug (1), refactor (1), tech-debt (1)
    assert_eq!(all_labels.len(), 6);
}

#[test]
fn test_p0_epic_with_no_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 epic without labels
    let epic = Issue {
        id: "epic-p0-no-labels".to_string(),
        title: "P0 Epic No Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Verify epic has no labels but has P0 priority
    let retrieved = storage.get_issue("epic-p0-no-labels").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 0);
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
}

#[test]
fn test_p0_epic_labels_update() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 epic with initial labels
    let epic = Issue {
        id: "epic-p0-update-labels".to_string(),
        title: "P0 Update Labels Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Update epic labels
    storage
        .add_label("epic-p0-update-labels", "urgent")
        .unwrap();
    storage
        .add_label("epic-p0-update-labels", "backend")
        .unwrap();

    // Verify labels were added
    let retrieved = storage.get_issue("epic-p0-update-labels").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 3);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"urgent".to_string()));
    assert!(retrieved.labels.contains(&"backend".to_string()));

    // Verify P0 priority is unchanged
    assert_eq!(retrieved.priority, Priority::CRITICAL);

    // Remove a label
    storage
        .remove_label("epic-p0-update-labels", "critical")
        .unwrap();

    // Verify label was removed
    let retrieved = storage.get_issue("epic-p0-update-labels").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 2);
    assert!(!retrieved.labels.contains(&"critical".to_string()));
}

#[test]
fn test_p0_epic_hierarchy_with_label_propagation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 epic with labels
    let epic = Issue {
        id: "epic-p0-hierarchy".to_string(),
        title: "P0 Hierarchy Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "large".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children with their own labels
    let child1 = Issue {
        id: "p0-hierarchy-child-1".to_string(),
        title: "Child 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["frontend".to_string()],
        priority: Priority::CRITICAL,
        ..Default::default()
    };
    storage.create_issue(&child1).unwrap();
    storage
        .add_dependency(
            "epic-p0-hierarchy",
            "p0-hierarchy-child-1",
            &DependencyType::ParentChild,
            "test",
        )
        .unwrap();

    let child2 = Issue {
        id: "p0-hierarchy-child-2".to_string(),
        title: "Child 2".to_string(),
        issue_type: IssueType::Bug,
        status: Status::Open,
        labels: vec!["urgent".to_string(), "bug".to_string()],
        priority: Priority::HIGH, // Different priority
        ..Default::default()
    };
    storage.create_issue(&child2).unwrap();
    storage
        .add_dependency(
            "epic-p0-hierarchy",
            "p0-hierarchy-child-2",
            &DependencyType::ParentChild,
            "test",
        )
        .unwrap();

    // Verify labels don't propagate (each issue has its own labels)
    let epic_retrieved = storage.get_issue("epic-p0-hierarchy").unwrap().unwrap();
    assert_eq!(epic_retrieved.labels.len(), 2);
    assert_eq!(epic_retrieved.priority, Priority::CRITICAL);

    let child1_retrieved = storage.get_issue("p0-hierarchy-child-1").unwrap().unwrap();
    assert_eq!(child1_retrieved.labels.len(), 1);
    assert_eq!(child1_retrieved.priority, Priority::CRITICAL);

    let child2_retrieved = storage.get_issue("p0-hierarchy-child-2").unwrap().unwrap();
    assert_eq!(child2_retrieved.labels.len(), 2);
    assert_eq!(child2_retrieved.priority, Priority::HIGH);

    // Verify global label count includes all labels from all issues
    let all_labels = storage.list_all_labels().unwrap();
    assert_eq!(all_labels.len(), 5); // critical, large, frontend, urgent, bug
}

#[test]
fn test_p0_epic_labels_with_closed_children() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 epic with labels
    let epic = Issue {
        id: "epic-p0-closed-labels".to_string(),
        title: "P0 Closed Labels Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create and close children
    let mut child = Issue {
        id: "p0-closed-child-labels".to_string(),
        title: "Closed Child".to_string(),
        issue_type: IssueType::Task,
        status: Status::Closed,
        labels: vec!["completed".to_string()],
        priority: Priority::CRITICAL,
        ..Default::default()
    };
    child.closed_at = Some(Utc::now());
    storage.create_issue(&child).unwrap();
    storage
        .add_dependency(
            "epic-p0-closed-labels",
            "p0-closed-child-labels",
            &DependencyType::ParentChild,
            "test",
        )
        .unwrap();

    // Verify labels persist on closed issues
    let all_labels = storage.list_all_labels().unwrap();
    assert_eq!(all_labels.len(), 2); // critical, completed

    let label_map: std::collections::HashMap<String, i64> = all_labels.into_iter().collect();
    assert_eq!(label_map.get("critical"), Some(&1));
    assert_eq!(label_map.get("completed"), Some(&1));
}

#[test]
fn test_p0_epic_with_full_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create comprehensive P0 epic with all fields and labels
    let mut epic = Issue {
        id: "epic-p0-full-labels".to_string(),
        title: "Complete P0 Epic with Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        description: Some("Full metadata test with labels".to_string()),
        assignee: Some("test-worker".to_string()),
        labels: vec![
            "critical".to_string(),
            "feature".to_string(),
            "backend".to_string(),
        ],
        ..Default::default()
    };
    epic.created_at = Utc::now();
    epic.updated_at = Utc::now();

    storage.create_issue(&epic).unwrap();

    // Retrieve and verify all fields
    let retrieved = storage.get_issue("epic-p0-full-labels").unwrap().unwrap();

    assert_eq!(retrieved.id, "epic-p0-full-labels");
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(
        retrieved.description,
        Some("Full metadata test with labels".to_string())
    );
    assert_eq!(retrieved.assignee, Some("test-worker".to_string()));
    assert_eq!(retrieved.labels.len(), 3);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"feature".to_string()));
    assert!(retrieved.labels.contains(&"backend".to_string()));
}

#[test]
fn test_p0_epic_display_formatting_with_labels() {
    // Test priority display formatting for P0
    let p0 = Priority::CRITICAL;
    let display = format!("{}", p0);
    assert_eq!(display, "P0");

    // Create P0 epic with labels and test full display
    let epic = Issue {
        id: "epic-p0-display-labels".to_string(),
        title: "P0 Display Labels Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "display-test".to_string()],
        ..Default::default()
    };

    // Verify priority displays as P0
    assert_eq!(format!("{}", epic.priority), "P0");

    // Verify labels are accessible
    assert_eq!(epic.labels.len(), 2);
}

#[test]
fn test_p0_epic_json_roundtrip_with_labels() {
    // Create P0 epic with labels
    let original = Issue {
        id: "epic-p0-roundtrip-labels".to_string(),
        title: "P0 Roundtrip Labels Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        description: Some("Test description with labels".to_string()),
        labels: vec![
            "critical".to_string(),
            "infrastructure".to_string(),
            "database".to_string(),
        ],
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&original).unwrap();

    // Verify JSON contains correct values
    assert!(json.contains("\"epic-p0-roundtrip-labels\""));
    assert!(json.contains("\"issue_type\": \"epic\""));
    assert!(json.contains("\"priority\": 0"));
    assert!(json.contains("Test description with labels"));
    assert!(json.contains("critical"));
    assert!(json.contains("infrastructure"));
    assert!(json.contains("database"));

    // Deserialize back
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    // Verify all fields match
    assert_eq!(deserialized.id, original.id);
    assert_eq!(deserialized.issue_type, original.issue_type);
    assert_eq!(deserialized.priority, original.priority);
    assert_eq!(deserialized.priority.0, 0);
    assert_eq!(deserialized.description, original.description);
    assert_eq!(deserialized.status, original.status);
    assert_eq!(deserialized.labels.len(), original.labels.len());
    for label in &original.labels {
        assert!(deserialized.labels.contains(label));
    }
}

#[test]
fn test_p0_epic_get_labels_with_children() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 epic with labels
    let epic = Issue {
        id: "epic-p0-get-labels".to_string(),
        title: "P0 Get Labels Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string()],
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create child with different labels
    let child = Issue {
        id: "p0-get-labels-child".to_string(),
        title: "Get Labels Child".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["bug".to_string()],
        priority: Priority::HIGH,
        ..Default::default()
    };
    storage.create_issue(&child).unwrap();
    storage
        .add_dependency(
            "epic-p0-get-labels",
            "p0-get-labels-child",
            &DependencyType::ParentChild,
            "test",
        )
        .unwrap();

    // Get labels specifically for epic
    let epic_labels = storage.get_labels("epic-p0-get-labels").unwrap();
    assert_eq!(epic_labels.len(), 1);
    assert!(epic_labels.contains(&"critical".to_string()));

    // Get labels for child
    let child_labels = storage.get_labels("p0-get-labels-child").unwrap();
    assert_eq!(child_labels.len(), 1);
    assert!(child_labels.contains(&"bug".to_string()));

    // Verify epic is P0
    let epic_retrieved = storage.get_issue("epic-p0-get-labels").unwrap().unwrap();
    assert_eq!(epic_retrieved.priority, Priority::CRITICAL);
}

// ===== Tests specifically for epic bf-46xuto =====

#[test]
fn test_bf_46xuto_p0_epic_exact_structure() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic matching exact bf-46xuto structure
    let epic = Issue {
        id: "bf-46xuto".to_string(),
        title: "Test epic with P0 and labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::InProgress,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "epic-p0".to_string()],
        assignee: Some("claude-code-glm-4.7-golf".to_string()),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Verify exact structure matches bf-46xuto
    let retrieved = storage.get_issue("bf-46xuto").unwrap().unwrap();
    assert_eq!(retrieved.id, "bf-46xuto");
    assert_eq!(retrieved.title, "Test epic with P0 and labels");
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.status, Status::InProgress);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(format!("{}", retrieved.priority), "P0");
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"epic-p0".to_string()));
    assert_eq!(
        retrieved.assignee,
        Some("claude-code-glm-4.7-golf".to_string())
    );
}

#[test]
fn test_bf_46xuto_label_variations() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Test different label combinations with P0 priority
    let test_cases = vec![
        vec!["critical"],
        vec!["critical", "epic-p0"],
        vec!["epic-p0"],
        vec!["critical", "epic-p0", "urgent"],
        vec!["critical", "epic-p0", "test", "verification"],
    ];

    for (i, labels) in test_cases.iter().enumerate() {
        let epic = Issue {
            id: format!("bf-46xuto-variant-{}", i),
            title: format!("Test epic variant {}", i),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority: Priority::CRITICAL,
            labels: labels.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };

        storage.create_issue(&epic).unwrap();

        let retrieved = storage
            .get_issue(&format!("bf-46xuto-variant-{}", i))
            .unwrap()
            .unwrap();

        // Verify P0 priority regardless of label combination
        assert_eq!(retrieved.priority, Priority::CRITICAL);
        assert_eq!(retrieved.priority.0, 0);

        // Verify all labels present
        assert_eq!(retrieved.labels.len(), labels.len());
        for label in labels {
            assert!(retrieved.labels.contains(&label.to_string()));
        }
    }
}

#[test]
fn test_bf_46xuto_epic_p0_label_operations() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with initial critical label
    let epic = Issue {
        id: "bf-46xuto-ops-test".to_string(),
        title: "P0 Epic label operations test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string()],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Add epic-p0 label
    storage.add_label("bf-46xuto-ops-test", "epic-p0").unwrap();

    let retrieved = storage.get_issue("bf-46xuto-ops-test").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"epic-p0".to_string()));
    assert_eq!(retrieved.priority, Priority::CRITICAL);

    // Add another label
    storage.add_label("bf-46xuto-ops-test", "urgent").unwrap();

    let updated = storage.get_issue("bf-46xuto-ops-test").unwrap().unwrap();
    assert_eq!(updated.labels.len(), 3);
    assert!(updated.labels.contains(&"urgent".to_string()));
    assert_eq!(updated.priority, Priority::CRITICAL);

    // Remove epic-p0 label
    storage.remove_label("bf-46xuto-ops-test", "epic-p0").unwrap();

    let final_state = storage.get_issue("bf-46xuto-ops-test").unwrap().unwrap();
    assert_eq!(final_state.labels.len(), 2);
    assert!(!final_state.labels.contains(&"epic-p0".to_string()));
    assert!(final_state.labels.contains(&"critical".to_string()));
    assert!(final_state.labels.contains(&"urgent".to_string()));

    // Priority should remain P0 throughout
    assert_eq!(final_state.priority, Priority::CRITICAL);
    assert_eq!(final_state.priority.0, 0);
}

#[test]
fn test_bf_46xuto_json_serialization() {
    // Create epic matching bf-46xuto for JSON testing
    let epic = Issue {
        id: "bf-46xuto-json".to_string(),
        title: "Test epic with P0 and labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::InProgress,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "epic-p0".to_string()],
        assignee: Some("claude-code-glm-4.7-golf".to_string()),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&epic).unwrap();

    // Verify JSON structure
    let parsed = serde_json::from_str::<serde_json::Value>(&json).unwrap();
    assert_eq!(parsed["id"], "bf-46xuto-json");
    assert_eq!(parsed["title"], "Test epic with P0 and labels");
    assert_eq!(parsed["issue_type"], "epic");
    assert_eq!(parsed["status"], "in_progress");
    assert_eq!(parsed["priority"], 0);
    assert_eq!(parsed["assignee"], "claude-code-glm-4.7-golf");

    let label_array = parsed["labels"].as_array().unwrap();
    assert_eq!(label_array.len(), 2);
    assert!(label_array.iter().any(|l| l == "critical"));
    assert!(label_array.iter().any(|l| l == "epic-p0"));

    // Test roundtrip
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, "bf-46xuto-json");
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.priority.0, 0);
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.labels.len(), 2);
    assert!(deserialized.labels.contains(&"critical".to_string()));
    assert!(deserialized.labels.contains(&"epic-p0".to_string()));
}

#[test]
fn test_bf_46xuto_cross_priority_comparison() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 epic
    let p0_epic = Issue {
        id: "bf-46xuto-p0-compare".to_string(),
        title: "P0 Epic with labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "epic-p0".to_string()],
        ..Default::default()
    };

    // Create P1 epic for comparison
    let p1_epic = Issue {
        id: "bf-46xuto-p1-compare".to_string(),
        title: "P1 Epic with labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        labels: vec!["important".to_string(), "epic-p1".to_string()],
        ..Default::default()
    };

    storage.create_issue(&p0_epic).unwrap();
    storage.create_issue(&p1_epic).unwrap();

    let retrieved_p0 = storage
        .get_issue("bf-46xuto-p0-compare")
        .unwrap()
        .unwrap();
    let retrieved_p1 = storage
        .get_issue("bf-46xuto-p1-compare")
        .unwrap()
        .unwrap();

    // Verify P0 is higher priority than P1
    assert!(retrieved_p0.priority < retrieved_p1.priority);
    assert_eq!(retrieved_p0.priority.0, 0);
    assert_eq!(retrieved_p1.priority.0, 1);

    // Verify labels are distinct
    assert!(retrieved_p0.labels.contains(&"epic-p0".to_string()));
    assert!(!retrieved_p1.labels.contains(&"epic-p0".to_string()));
    assert!(retrieved_p1.labels.contains(&"epic-p1".to_string()));
    assert!(!retrieved_p0.labels.contains(&"epic-p1".to_string()));
}

#[test]
fn test_bf_46xuto_comprehensive_validation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with complete bf-46xuto metadata
    let epic = Issue {
        id: "bf-46xuto-comprehensive".to_string(),
        title: "Test epic with P0 and labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::InProgress,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "epic-p0".to_string()],
        assignee: Some("claude-code-glm-4.7-golf".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Comprehensive verification
    let retrieved = storage
        .get_issue("bf-46xuto-comprehensive")
        .unwrap()
        .unwrap();

    // Identity and type
    assert_eq!(retrieved.id, "bf-46xuto-comprehensive");
    assert_eq!(retrieved.title, "Test epic with P0 and labels");
    assert_eq!(retrieved.issue_type, IssueType::Epic);

    // Status and priority
    assert_eq!(retrieved.status, Status::InProgress);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(format!("{}", retrieved.priority), "P0");

    // Labels
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"epic-p0".to_string()));

    // Assignee
    assert_eq!(
        retrieved.assignee,
        Some("claude-code-glm-4.7-golf".to_string())
    );

    // Timestamps - verify they are set (DateTime fields are always set)
    let time_diff = retrieved.updated_at.signed_duration_since(retrieved.created_at);
    assert!(time_diff.num_seconds() >= 0, "updated_at should be after created_at");

    // JSON serialization test
    let json = serde_json::to_string(&retrieved).unwrap();
    assert!(json.contains("\"priority\":0"));
    assert!(json.contains("\"issue_type\":\"epic\""));
    assert!(json.contains("\"status\":\"in_progress\""));
    assert!(json.contains("critical"));
    assert!(json.contains("epic-p0"));

    // Roundtrip test
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.labels.len(), 2);
}

#[test]
fn test_bf_46xuto_display_properties() {
    // Test display formatting for P0 epic
    let epic = Issue {
        id: "bf-46xuto-display".to_string(),
        title: "Test epic with P0 and labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::InProgress,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "epic-p0".to_string()],
        assignee: Some("claude-code-glm-4.7-golf".to_string()),
        ..Default::default()
    };

    // Test priority display
    let priority_display = format!("{}", epic.priority);
    assert_eq!(priority_display, "P0");

    // Test type display
    let type_display = format!("{}", epic.issue_type);
    assert_eq!(type_display, "epic");

    // Verify all properties accessible
    assert_eq!(epic.labels.len(), 2);
    assert_eq!(epic.priority.0, 0);
    assert_eq!(epic.status, Status::InProgress);
}
