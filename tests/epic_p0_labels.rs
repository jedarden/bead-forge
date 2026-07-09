// Epic P0 Priority with Labels Tests
// Tests that epics can have P0 (Critical) priority and labels together

use bead_forge::model::{Issue, IssueType, Status, DependencyType, Priority};
use bead_forge::storage::Storage;
use chrono::Utc;

#[test]
fn test_epic_p0_creation_with_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with P0 priority and labels
    let epic = Issue {
        id: "epic-p0-labels".to_string(),
        title: "P0 Epic with Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "high-priority".to_string(), "security".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Retrieve and verify P0 priority and labels are preserved
    let retrieved = storage.get_issue("epic-p0-labels").unwrap().unwrap();
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(retrieved.labels.len(), 3);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"high-priority".to_string()));
    assert!(retrieved.labels.contains(&"security".to_string()));
}

#[test]
fn test_epic_p0_single_label() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with P0 priority and single label
    let epic = Issue {
        id: "epic-p0-single".to_string(),
        title: "P0 Epic Single Label".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["urgent".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    let retrieved = storage.get_issue("epic-p0-single").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 1);
    assert!(retrieved.labels.contains(&"urgent".to_string()));
}

#[test]
fn test_epic_p0_multiple_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with P0 priority and multiple labels
    let epic = Issue {
        id: "epic-p0-multi".to_string(),
        title: "P0 Epic Multiple Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![
            "critical".to_string(),
            "security".to_string(),
            "hotfix".to_string(),
            "production".to_string(),
            "blocking".to_string(),
        ],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    let retrieved = storage.get_issue("epic-p0-multi").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 5);
}

#[test]
fn test_epic_p0_labels_serialization() {
    // Create epic with P0 priority and labels
    let epic = Issue {
        id: "epic-p0-serialize".to_string(),
        title: "P0 Serialization Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "hotfix".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&epic).unwrap();

    // Verify P0 priority and labels are serialized correctly
    assert!(json.contains(r#""issue_type":"epic""#));
    assert!(json.contains(r#""priority":0"#));
    assert!(json.contains(r#""labels":["critical","hotfix"]"#));

    // Deserialize and verify
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.priority.0, 0);
    assert_eq!(deserialized.labels.len(), 2);
    assert!(deserialized.labels.contains(&"critical".to_string()));
    assert!(deserialized.labels.contains(&"hotfix".to_string()));
}

#[test]
fn test_epic_p0_with_children_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with P0 priority and labels
    let epic = Issue {
        id: "epic-p0-children".to_string(),
        title: "P0 Epic with Children".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "security".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children with different labels and priorities
    let child1 = Issue {
        id: "p0-child-1".to_string(),
        title: "Child 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["urgent".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };
    storage.create_issue(&child1).unwrap();
    storage.add_dependency("epic-p0-children", "p0-child-1", &DependencyType::ParentChild, "test").unwrap();

    let child2 = Issue {
        id: "p0-child-2".to_string(),
        title: "Child 2".to_string(),
        issue_type: IssueType::Bug,
        status: Status::Open,
        priority: Priority::HIGH,
        labels: vec!["bug".to_string(), "frontend".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };
    storage.create_issue(&child2).unwrap();
    storage.add_dependency("epic-p0-children", "p0-child-2", &DependencyType::ParentChild, "test").unwrap();

    // Verify epic has P0 priority and correct labels
    let epic_retrieved = storage.get_issue("epic-p0-children").unwrap().unwrap();
    assert_eq!(epic_retrieved.priority, Priority::CRITICAL);
    assert_eq!(epic_retrieved.labels.len(), 2);
    assert!(epic_retrieved.labels.contains(&"critical".to_string()));
    assert!(epic_retrieved.labels.contains(&"security".to_string()));

    // Verify children have their own priorities and labels
    let child1_retrieved = storage.get_issue("p0-child-1").unwrap().unwrap();
    assert_eq!(child1_retrieved.priority, Priority::CRITICAL);
    assert_eq!(child1_retrieved.labels.len(), 1);

    let child2_retrieved = storage.get_issue("p0-child-2").unwrap().unwrap();
    assert_eq!(child2_retrieved.priority, Priority::HIGH);
    assert_eq!(child2_retrieved.labels.len(), 2);
}

#[test]
fn test_multiple_epics_p0_with_different_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple P0 epics with different labels
    let epics = vec![
        ("epic-p0-1", vec!["critical".to_string(), "security".to_string()]),
        ("epic-p0-2", vec!["hotfix".to_string(), "production".to_string()]),
        ("epic-p0-3", vec!["blocking".to_string(), "data-loss".to_string()]),
    ];

    for (id, labels) in &epics {
        let epic = Issue {
            id: id.to_string(),
            title: format!("P0 Epic {}", id),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority: Priority::CRITICAL,
            labels: labels.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };
        storage.create_issue(&epic).unwrap();
    }

    // Verify each epic has P0 priority and correct labels
    for (id, labels) in &epics {
        let retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(retrieved.priority, Priority::CRITICAL);
        assert_eq!(retrieved.priority.0, 0);
        assert_eq!(retrieved.labels.len(), labels.len());
        for label in labels {
            assert!(retrieved.labels.contains(label));
        }
    }
}

#[test]
fn test_epic_p0_labels_with_closed_status() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with P0 priority, labels, and closed status
    let mut epic = Issue {
        id: "epic-p0-closed".to_string(),
        title: "Closed P0 Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Closed,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "completed".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };
    epic.closed_at = Some(Utc::now());

    storage.create_issue(&epic).unwrap();

    // Verify closed P0 epic with labels is preserved
    let retrieved = storage.get_issue("epic-p0-closed").unwrap().unwrap();
    assert_eq!(retrieved.status, Status::Closed);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.closed_at.is_some());
}

#[test]
fn test_epic_p0_priority_display() {
    // Create epic with P0 priority and labels
    let epic = Issue {
        id: "epic-p0-display".to_string(),
        title: "P0 Display Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // Verify priority displays as P0
    assert_eq!(format!("{}", epic.priority), "P0");
    assert_eq!(epic.priority.0, 0);
    assert_eq!(epic.priority, Priority::CRITICAL);
}

#[test]
fn test_epic_p0_labels_update() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with P0 priority and initial labels
    let epic = Issue {
        id: "epic-p0-update".to_string(),
        title: "P0 Update Labels Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Add more labels
    storage.add_label("epic-p0-update", "hotfix").unwrap();
    storage.add_label("epic-p0-update", "production").unwrap();

    // Verify P0 priority is preserved and labels were added
    let retrieved = storage.get_issue("epic-p0-update").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 3);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"hotfix".to_string()));
    assert!(retrieved.labels.contains(&"production".to_string()));

    // Remove a label
    storage.remove_label("epic-p0-update", "critical").unwrap();

    // Verify label was removed but P0 priority is preserved
    let retrieved = storage.get_issue("epic-p0-update").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 2);
    assert!(!retrieved.labels.contains(&"critical".to_string()));
}

#[test]
fn test_epic_p0_filtering_by_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 epic with specific labels
    let epic = Issue {
        id: "epic-p0-filter".to_string(),
        title: "P0 Filter Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "security".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create other issues with different priorities and labels
    let other_epic = Issue {
        id: "epic-p1-filter".to_string(),
        title: "P1 Filter Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        labels: vec!["feature".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };
    storage.create_issue(&other_epic).unwrap();

    // List all issues and verify filtering
    let all_issues = storage.list_issues(&Default::default()).unwrap();
    let p0_epics: Vec<_> = all_issues
        .iter()
        .filter(|i| i.issue_type == IssueType::Epic && i.priority == Priority::CRITICAL)
        .collect();

    assert_eq!(p0_epics.len(), 1);
    assert_eq!(p0_epics[0].id, "epic-p0-filter");
    assert_eq!(p0_epics[0].labels.len(), 2);
}

#[test]
fn test_epic_p0_json_roundtrip() {
    // Create epic with P0 priority and labels
    let epic = Issue {
        id: "epic-p0-roundtrip".to_string(),
        title: "P0 Roundtrip Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "hotfix".to_string(), "blocking".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&epic).unwrap();

    // Deserialize back
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    // Verify all fields match
    assert_eq!(deserialized.id, epic.id);
    assert_eq!(deserialized.title, epic.title);
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.status, Status::Open);
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.priority.0, 0);
    assert_eq!(deserialized.labels, epic.labels);
    assert_eq!(deserialized.labels.len(), 3);
}

#[test]
fn test_epic_p0_default_comparison_with_other_priorities() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epics with all priority levels
    let priorities = vec![
        (Priority::CRITICAL, 0, "epic-p0"),
        (Priority::HIGH, 1, "epic-p1"),
        (Priority::MEDIUM, 2, "epic-p2"),
        (Priority::LOW, 3, "epic-p3"),
        (Priority::BACKLOG, 4, "epic-p4"),
    ];

    for (priority, value, id) in &priorities {
        let epic = Issue {
            id: id.to_string(),
            title: format!("Epic {}", id),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority: *priority,
            labels: vec!["test".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };
        storage.create_issue(&epic).unwrap();
    }

    // Verify P0 epic has lowest priority value
    let p0_epic = storage.get_issue("epic-p0").unwrap().unwrap();
    assert_eq!(p0_epic.priority.0, 0);
    assert_eq!(p0_epic.priority, Priority::CRITICAL);

    // Verify P0 < P1 < P2 < P3 < P4 in terms of priority value
    let all_issues = storage.list_issues(&Default::default()).unwrap();
    let mut epics: Vec<_> = all_issues
        .iter()
        .filter(|i| i.issue_type == IssueType::Epic && i.id.starts_with("epic-p"))
        .collect();
    epics.sort_by_key(|e| e.priority.0);

    assert_eq!(epics[0].id, "epic-p0");
    assert_eq!(epics[0].priority.0, 0);
    assert_eq!(epics[1].id, "epic-p1");
    assert_eq!(epics[1].priority.0, 1);
    assert_eq!(epics[2].id, "epic-p2");
    assert_eq!(epics[2].priority.0, 2);
    assert_eq!(epics[3].id, "epic-p3");
    assert_eq!(epics[3].priority.0, 3);
    assert_eq!(epics[4].id, "epic-p4");
    assert_eq!(epics[4].priority.0, 4);
}
