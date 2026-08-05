// P0 Bead Creation with Labels Test Suite
// Tests comprehensive scenarios for creating Priority 0 (Critical) beads with labels

use bead_forge::model::{Issue, IssueChanges, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;

#[test]
fn test_p0_task_creation_with_single_label() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let task = Issue {
        id: "task-p0-single".to_string(),
        title: "P0 Task with Single Label".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["urgent".to_string()],
        ..Default::default()
    };

    storage.create_issue(&task).unwrap();

    let retrieved = storage.get_issue("task-p0-single").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(retrieved.labels.len(), 1);
    assert!(retrieved.labels.contains(&"urgent".to_string()));
    assert_eq!(retrieved.issue_type, IssueType::Task);
}

#[test]
fn test_p0_task_creation_with_multiple_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let task = Issue {
        id: "task-p0-multiple".to_string(),
        title: "P0 Task with Multiple Labels".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![
            "urgent".to_string(),
            "security".to_string(),
            "critical".to_string(),
        ],
        ..Default::default()
    };

    storage.create_issue(&task).unwrap();

    let retrieved = storage.get_issue("task-p0-multiple").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 3);
    assert!(retrieved.labels.contains(&"urgent".to_string()));
    assert!(retrieved.labels.contains(&"security".to_string()));
    assert!(retrieved.labels.contains(&"critical".to_string()));
}

#[test]
fn test_p0_bug_creation_with_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bug = Issue {
        id: "bug-p0-critical".to_string(),
        title: "Critical Bug with Labels".to_string(),
        issue_type: IssueType::Bug,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["bug".to_string(), "production".to_string()],
        ..Default::default()
    };

    storage.create_issue(&bug).unwrap();

    let retrieved = storage.get_issue("bug-p0-critical").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.issue_type, IssueType::Bug);
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"bug".to_string()));
    assert!(retrieved.labels.contains(&"production".to_string()));
}

#[test]
fn test_p0_feature_creation_with_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let feature = Issue {
        id: "feature-p0-important".to_string(),
        title: "Critical Feature with Labels".to_string(),
        issue_type: IssueType::Feature,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["feature".to_string(), "urgent".to_string()],
        ..Default::default()
    };

    storage.create_issue(&feature).unwrap();

    let retrieved = storage.get_issue("feature-p0-important").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.issue_type, IssueType::Feature);
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"feature".to_string()));
    assert!(retrieved.labels.contains(&"urgent".to_string()));
}

#[test]
fn test_p0_epic_creation_with_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let epic = Issue {
        id: "epic-p0-critical".to_string(),
        title: "Critical Epic with Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["epic".to_string(), "critical-path".to_string()],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    let retrieved = storage.get_issue("epic-p0-critical").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"epic".to_string()));
    assert!(retrieved.labels.contains(&"critical-path".to_string()));
}

#[test]
fn test_p0_label_addition_after_creation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 task without labels
    let task = Issue {
        id: "task-p0-add-labels".to_string(),
        title: "P0 Task for Label Addition".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![],
        ..Default::default()
    };

    storage.create_issue(&task).unwrap();

    // Add labels to P0 task
    storage.add_label("task-p0-add-labels", "urgent").unwrap();
    storage.add_label("task-p0-add-labels", "critical").unwrap();

    let retrieved = storage.get_issue("task-p0-add-labels").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"urgent".to_string()));
    assert!(retrieved.labels.contains(&"critical".to_string()));
}

#[test]
fn test_p0_label_removal() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let task = Issue {
        id: "task-p0-remove-labels".to_string(),
        title: "P0 Task for Label Removal".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["urgent".to_string(), "security".to_string(), "bug".to_string()],
        ..Default::default()
    };

    storage.create_issue(&task).unwrap();

    // Remove a label
    storage.remove_label("task-p0-remove-labels", "bug").unwrap();

    let retrieved = storage.get_issue("task-p0-remove-labels").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"urgent".to_string()));
    assert!(retrieved.labels.contains(&"security".to_string()));
    assert!(!retrieved.labels.contains(&"bug".to_string()));
}

#[test]
fn test_p0_label_update_via_changes() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let task = Issue {
        id: "task-p0-update-labels".to_string(),
        title: "P0 Task for Label Update".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["old-label".to_string()],
        ..Default::default()
    };

    storage.create_issue(&task).unwrap();

    // Update labels
    let changes = IssueChanges {
        labels: Some(vec!["new-label".to_string(), "updated".to_string()]),
        ..Default::default()
    };
    storage.update_issue("task-p0-update-labels", &changes).unwrap();

    let retrieved = storage.get_issue("task-p0-update-labels").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"new-label".to_string()));
    assert!(retrieved.labels.contains(&"updated".to_string()));
    assert!(!retrieved.labels.contains(&"old-label".to_string()));
}

#[test]
fn test_p0_json_serialization_with_labels() {
    let task = Issue {
        id: "task-p0-json".to_string(),
        title: "P0 Task for JSON Testing".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["json-test".to_string(), "critical".to_string()],
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&task).unwrap();

    // Verify JSON contains priority 0 and labels
    assert!(json.contains("\"priority\":0"));
    assert!(json.contains("\"issue_type\":\"task\""));
    assert!(json.contains("json-test"));
    assert!(json.contains("critical"));

    // Deserialize and verify
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.priority.0, 0);
    assert_eq!(deserialized.labels.len(), 2);
    assert!(deserialized.labels.contains(&"json-test".to_string()));
    assert!(deserialized.labels.contains(&"critical".to_string()));
}

#[test]
fn test_p0_multiple_beads_with_different_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple P0 beads with different labels
    let p0_beads = vec![
        ("p0-1", "Critical Security Bug", vec!["security", "urgent"]),
        ("p0-2", "Performance Issue", vec!["performance", "critical"]),
        ("p0-3", "Data Loss Bug", vec!["data-loss", "blocking"]),
        ("p0-4", "API Outage", vec!["api", "production", "urgent"]),
    ];

    for (id, title, labels) in &p0_beads {
        let task = Issue {
            id: id.to_string(),
            title: title.to_string(),
            issue_type: IssueType::Bug,
            status: Status::Open,
            priority: Priority::CRITICAL,
            labels: labels.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        storage.create_issue(&task).unwrap();
    }

    // Verify all P0 beads were created with correct labels
    for (id, _, expected_labels) in p0_beads {
        let retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(retrieved.priority, Priority::CRITICAL);
        assert_eq!(retrieved.labels.len(), expected_labels.len());
        for label in expected_labels {
            assert!(retrieved.labels.contains(&label.to_string()));
        }
    }

    // Verify global label list
    let all_labels = storage.list_all_labels().unwrap();
    assert!(all_labels.len() > 0);
}

#[test]
fn test_p0_priority_maintained_with_label_operations() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let task = Issue {
        id: "task-p0-priority-maintained".to_string(),
        title: "P0 Task Priority Maintenance".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["initial".to_string()],
        ..Default::default()
    };

    storage.create_issue(&task).unwrap();

    // Perform various label operations
    storage.add_label("task-p0-priority-maintained", "second").unwrap();
    storage.add_label("task-p0-priority-maintained", "third").unwrap();
    storage.remove_label("task-p0-priority-maintained", "initial").unwrap();

    // Update with new label set
    let changes = IssueChanges {
        labels: Some(vec!["updated".to_string(), "final".to_string()]),
        ..Default::default()
    };
    storage.update_issue("task-p0-priority-maintained", &changes).unwrap();

    let retrieved = storage.get_issue("task-p0-priority-maintained").unwrap().unwrap();
    // Priority should remain P0 throughout all label operations
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(retrieved.labels.len(), 2);
}

#[test]
fn test_p0_with_empty_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let task = Issue {
        id: "task-p0-no-labels".to_string(),
        title: "P0 Task without Labels".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![],
        ..Default::default()
    };

    storage.create_issue(&task).unwrap();

    let retrieved = storage.get_issue("task-p0-no-labels").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 0);
}

#[test]
fn test_p0_with_special_character_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let task = Issue {
        id: "task-p0-special-chars".to_string(),
        title: "P0 Task with Special Character Labels".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![
            "high-priority".to_string(),
            "needs-review".to_string(),
            "API:breaking".to_string(),
            "bug:security".to_string(),
        ],
        ..Default::default()
    };

    storage.create_issue(&task).unwrap();

    let retrieved = storage.get_issue("task-p0-special-chars").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 4);
    assert!(retrieved.labels.contains(&"high-priority".to_string()));
    assert!(retrieved.labels.contains(&"needs-review".to_string()));
    assert!(retrieved.labels.contains(&"API:breaking".to_string()));
    assert!(retrieved.labels.contains(&"bug:security".to_string()));
}

#[test]
fn test_p0_with_unicode_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let task = Issue {
        id: "task-p0-unicode".to_string(),
        title: "P0 Task with Unicode Labels".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![
            "🐛-critical".to_string(),
            "高优先级".to_string(),
            "critique".to_string(),
        ],
        ..Default::default()
    };

    storage.create_issue(&task).unwrap();

    let retrieved = storage.get_issue("task-p0-unicode").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 3);
    assert!(retrieved.labels.contains(&"🐛-critical".to_string()));
    assert!(retrieved.labels.contains(&"高优先级".to_string()));
    assert!(retrieved.labels.contains(&"critique".to_string()));
}

#[test]
fn test_p0_closed_bead_retains_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let mut task = Issue {
        id: "task-p0-closed".to_string(),
        title: "P0 Task to be Closed".to_string(),
        issue_type: IssueType::Task,
        status: Status::Closed,
        priority: Priority::CRITICAL,
        labels: vec!["completed".to_string(), "verified".to_string()],
        ..Default::default()
    };
    task.closed_at = Some(Utc::now());

    storage.create_issue(&task).unwrap();

    let retrieved = storage.get_issue("task-p0-closed").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.status, Status::Closed);
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"completed".to_string()));
    assert!(retrieved.labels.contains(&"verified".to_string()));
}

#[test]
fn test_p0_label_aggregation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple P0 beads with overlapping labels
    let p0_tasks = vec![
        ("p0-aggr-1", vec!["critical", "urgent"]),
        ("p0-aggr-2", vec!["critical", "security"]),
        ("p0-aggr-3", vec!["urgent", "security"]),
        ("p0-aggr-4", vec!["critical"]),
    ];

    for (id, labels) in p0_tasks {
        let task = Issue {
            id: id.to_string(),
            title: format!("P0 Aggregation Test {}", id),
            issue_type: IssueType::Task,
            status: Status::Open,
            priority: Priority::CRITICAL,
            labels: labels.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        storage.create_issue(&task).unwrap();
    }

    // Verify label aggregation
    let all_labels = storage.list_all_labels().unwrap();
    let label_map: std::collections::HashMap<String, i64> = all_labels.into_iter().collect();

    // "critical" appears in 3 tasks
    assert_eq!(label_map.get("critical"), Some(&3));
    // "urgent" appears in 2 tasks
    assert_eq!(label_map.get("urgent"), Some(&2));
    // "security" appears in 2 tasks
    assert_eq!(label_map.get("security"), Some(&2));
}

#[test]
fn test_p0_comprehensive_integration() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create a comprehensive P0 scenario with multiple issue types and labels
    let p0_epic = Issue {
        id: "epic-p0-integration".to_string(),
        title: "P0 Integration Test Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["epic".to_string(), "integration".to_string()],
        ..Default::default()
    };
    storage.create_issue(&p0_epic).unwrap();

    // Create various child beads with different labels
    let child_tasks = vec![
        ("child-p0-1", "Security Fix", vec!["security", "urgent"]),
        ("child-p0-2", "Performance Fix", vec!["performance", "critical"]),
        ("child-p0-3", "Data Recovery", vec!["data-loss", "blocking"]),
    ];

    for (id, title, labels) in &child_tasks {
        let task = Issue {
            id: id.to_string(),
            title: title.to_string(),
            issue_type: IssueType::Task,
            status: Status::Open,
            priority: Priority::CRITICAL,
            labels: labels.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        storage.create_issue(&task).unwrap();
    }

    // Verify epic
    let epic_retrieved = storage.get_issue("epic-p0-integration").unwrap().unwrap();
    assert_eq!(epic_retrieved.priority, Priority::CRITICAL);
    assert_eq!(epic_retrieved.issue_type, IssueType::Epic);
    assert_eq!(epic_retrieved.labels.len(), 2);

    // Verify all children are P0 with correct labels
    for (id, _, expected_labels) in &child_tasks {
        let child = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(child.priority, Priority::CRITICAL);
        assert_eq!(child.labels.len(), expected_labels.len());
        for label in expected_labels {
            assert!(child.labels.contains(&label.to_string()));
        }
    }

    // Verify global label state
    let all_labels = storage.list_all_labels().unwrap();
    assert!(all_labels.len() >= 7); // epic (2) + children (6+ unique labels)
}
