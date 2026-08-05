// Comprehensive Test: Epic Labels with All Priority Levels
// Tests epic labels across P0-P4 priority levels and their interactions
// This test file covers the requirements for bead bf-rdnyh

use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;

#[test]
fn test_epic_p0_with_critical_label() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let epic = Issue {
        id: "epic-p0-critical".to_string(),
        title: "Epic P0 with Critical Label".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string()],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    let retrieved = storage.get_issue("epic-p0-critical").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(retrieved.labels.len(), 1);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert_eq!(retrieved.issue_type, IssueType::Epic);
}

#[test]
fn test_epic_p1_with_high_priority_label() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let epic = Issue {
        id: "epic-p1-high".to_string(),
        title: "Epic P1 with High Priority Label".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        labels: vec!["high-priority".to_string()],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    let retrieved = storage.get_issue("epic-p1-high").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::HIGH);
    assert_eq!(retrieved.priority.0, 1);
    assert_eq!(retrieved.labels.len(), 1);
    assert!(retrieved.labels.contains(&"high-priority".to_string()));
}

#[test]
fn test_epic_p2_with_medium_priority_label() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let epic = Issue {
        id: "epic-p2-medium".to_string(),
        title: "Epic P2 with Medium Priority Label".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        labels: vec!["medium-priority".to_string()],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    let retrieved = storage.get_issue("epic-p2-medium").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::MEDIUM);
    assert_eq!(retrieved.priority.0, 2);
    assert_eq!(retrieved.labels.len(), 1);
    assert!(retrieved.labels.contains(&"medium-priority".to_string()));
}

#[test]
fn test_epic_p3_with_low_priority_label() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let epic = Issue {
        id: "epic-p3-low".to_string(),
        title: "Epic P3 with Low Priority Label".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::LOW,
        labels: vec!["low-priority".to_string()],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    let retrieved = storage.get_issue("epic-p3-low").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::LOW);
    assert_eq!(retrieved.priority.0, 3);
    assert_eq!(retrieved.labels.len(), 1);
    assert!(retrieved.labels.contains(&"low-priority".to_string()));
}

#[test]
fn test_epic_p4_with_backlog_label() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let epic = Issue {
        id: "epic-p4-backlog".to_string(),
        title: "Epic P4 with Backlog Label".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::BACKLOG,
        labels: vec!["backlog".to_string()],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    let retrieved = storage.get_issue("epic-p4-backlog").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::BACKLOG);
    assert_eq!(retrieved.priority.0, 4);
    assert_eq!(retrieved.labels.len(), 1);
    assert!(retrieved.labels.contains(&"backlog".to_string()));
}

#[test]
fn test_epic_multiple_labels_across_priorities() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epics with different priorities and multiple labels
    let priorities = vec![
        (Priority::CRITICAL, vec!["critical", "urgent", "security"]),
        (Priority::HIGH, vec!["high-priority", "important", "feature"]),
        (Priority::MEDIUM, vec!["medium-priority", "enhancement"]),
        (Priority::LOW, vec!["low-priority", "nice-to-have"]),
        (Priority::BACKLOG, vec!["backlog", "future"]),
    ];

    for (i, (priority, labels)) in priorities.iter().enumerate() {
        let epic = Issue {
            id: format!("epic-multi-{}", i),
            title: format!("Epic with multiple labels at priority {}", priority.0),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority: *priority,
            labels: labels.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };

        storage.create_issue(&epic).unwrap();

        let retrieved = storage
            .get_issue(&format!("epic-multi-{}", i))
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.priority, *priority);
        assert_eq!(retrieved.labels.len(), labels.len());
        for label in labels {
            assert!(retrieved.labels.contains(&label.to_string()));
        }
    }
}

#[test]
fn test_epic_label_addition_with_priority_update() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with P0 priority
    let epic = Issue {
        id: "epic-label-add-priority".to_string(),
        title: "Epic for label and priority testing".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string()],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Add labels and verify priority is maintained
    storage
        .add_label("epic-label-add-priority", "urgent")
        .unwrap();
    storage
        .add_label("epic-label-add-priority", "security")
        .unwrap();

    let retrieved = storage
        .get_issue("epic-label-add-priority")
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 3);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"urgent".to_string()));
    assert!(retrieved.labels.contains(&"security".to_string()));
}

#[test]
fn test_epic_label_removal_with_priority_maintained() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let epic = Issue {
        id: "epic-label-remove".to_string(),
        title: "Epic for label removal testing".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        labels: vec![
            "high-priority".to_string(),
            "feature".to_string(),
            "backend".to_string(),
        ],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Remove a label and verify priority is maintained
    storage
        .remove_label("epic-label-remove", "backend")
        .unwrap();

    let retrieved = storage.get_issue("epic-label-remove").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::HIGH);
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"high-priority".to_string()));
    assert!(retrieved.labels.contains(&"feature".to_string()));
    assert!(!retrieved.labels.contains(&"backend".to_string()));
}

#[test]
fn test_epic_json_serialization_all_priorities() {
    // Test JSON serialization for epics with labels at all priority levels
    let priorities_and_labels = vec![
        (Priority::CRITICAL, vec!["critical", "urgent"]),
        (Priority::HIGH, vec!["high-priority", "important"]),
        (Priority::MEDIUM, vec!["medium-priority"]),
        (Priority::LOW, vec!["low-priority", "nice-to-have"]),
        (Priority::BACKLOG, vec!["backlog"]),
    ];

    for (priority, labels) in priorities_and_labels {
        let epic = Issue {
            id: format!("epic-json-{}", priority.0),
            title: format!("Epic JSON test for priority {}", priority.0),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority,
            labels: labels.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };

        // Serialize to JSON
        let json = serde_json::to_string(&epic).unwrap();

        // Verify JSON structure
        let parsed = serde_json::from_str::<serde_json::Value>(&json).unwrap();
        assert_eq!(parsed["priority"], priority.0);
        assert_eq!(parsed["issue_type"], "epic");

        let label_array = parsed["labels"].as_array().unwrap();
        assert_eq!(label_array.len(), labels.len());
        for label in labels {
            assert!(label_array.iter().any(|l| l == label));
        }

        // Test roundtrip
        let deserialized: Issue = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.priority, priority);
        assert_eq!(deserialized.labels.len(), labels.len());
    }
}

#[test]
fn test_epic_priority_ordering_with_labels() {
    // Test that priority ordering works correctly with labels
    let priorities = vec![
        Priority::CRITICAL,
        Priority::HIGH,
        Priority::MEDIUM,
        Priority::LOW,
        Priority::BACKLOG,
    ];

    for (i, priority) in priorities.iter().enumerate() {
        let higher_priorities = &priorities[i + 1..];
        for higher_priority in higher_priorities {
            assert!(
                priority < higher_priority,
                "Priority {} should be less than {}",
                priority.0,
                higher_priority.0
            );
        }
    }
}

#[test]
fn test_epic_label_operations_different_priorities() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Test label operations on epics with different priorities
    let test_cases = vec![
        (Priority::CRITICAL, "critical-epic"),
        (Priority::HIGH, "high-epic"),
        (Priority::MEDIUM, "medium-epic"),
        (Priority::LOW, "low-epic"),
        (Priority::BACKLOG, "backlog-epic"),
    ];

    for (priority, id_suffix) in test_cases {
        let id = format!("epic-labels-{}", id_suffix);
        let epic = Issue {
            id: id.clone(),
            title: format!("Epic for label operations at priority {}", priority.0),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority,
            labels: vec![format!("label-{}", priority.0)],
            ..Default::default()
        };

        storage.create_issue(&epic).unwrap();

        // Add label
        storage
            .add_label(&id, &format!("added-{}", priority.0))
            .unwrap();

        let retrieved = storage.get_issue(&id).unwrap().unwrap();
        assert_eq!(retrieved.labels.len(), 2);
        assert_eq!(retrieved.priority, priority);

        // Remove original label
        storage
            .remove_label(&id, &format!("label-{}", priority.0))
            .unwrap();

        let final_state = storage.get_issue(&id).unwrap().unwrap();
        assert_eq!(final_state.labels.len(), 1);
        assert!(final_state.labels.contains(&format!("added-{}", priority.0)));
        assert_eq!(final_state.priority, priority); // Priority should be unchanged
    }
}

#[test]
fn test_epic_priority_label_integration() {
    // Integration test: Create epics with all priority levels and labels,
    // then verify they can be retrieved and filtered correctly
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epics with different priorities
    for priority_value in 0..=4 {
        let epic = Issue {
            id: format!("epic-integration-{}", priority_value),
            title: format!("Integration test epic priority {}", priority_value),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority: Priority(priority_value),
            labels: vec![format!("priority-{}", priority_value)],
            ..Default::default()
        };

        storage.create_issue(&epic).unwrap();
    }

    // Verify all epics can be retrieved
    let all_issues = storage.list_issues(&Default::default()).unwrap();
    let epics: Vec<_> = all_issues
        .into_iter()
        .filter(|i| i.issue_type == IssueType::Epic && i.id.starts_with("epic-integration-"))
        .collect();

    assert_eq!(epics.len(), 5, "Should have 5 test epics");

    // Verify each epic has correct priority and labels
    for epic in epics {
        let priority_value = epic.priority.0;
        assert!(
            epic.labels.contains(&format!("priority-{}", priority_value)),
            "Epic with priority {} should have matching label",
            priority_value
        );
        assert_eq!(
            epic.priority.0, priority_value,
            "Priority value should match"
        );
    }
}

#[test]
fn test_epic_label_priority_display_formatting() {
    // Test that epics with labels display correctly at all priority levels
    let priorities = vec![
        (Priority::CRITICAL, "P0"),
        (Priority::HIGH, "P1"),
        (Priority::MEDIUM, "P2"),
        (Priority::LOW, "P3"),
        (Priority::BACKLOG, "P4"),
    ];

    for (priority, expected_display) in priorities {
        let epic = Issue {
            id: format!("epic-display-{}", priority.0),
            title: format!("Display test epic {}", priority.0),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority,
            labels: vec!["test-label".to_string()],
            ..Default::default()
        };

        let priority_display = format!("{}", epic.priority);
        assert_eq!(
            priority_display, expected_display,
            "Priority {} should display as {}",
            priority.0, expected_display
        );

        assert_eq!(epic.labels.len(), 1);
        assert!(epic.labels.contains(&"test-label".to_string()));
    }
}

#[test]
fn test_epic_priority_comparison_with_labels() {
    // Test that priority comparison works correctly regardless of labels
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epics with same priority but different labels
    let epic1 = Issue {
        id: "epic-compare-1".to_string(),
        title: "Epic 1".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        labels: vec!["feature".to_string(), "backend".to_string()],
        ..Default::default()
    };

    let epic2 = Issue {
        id: "epic-compare-2".to_string(),
        title: "Epic 2".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        labels: vec!["bug".to_string(), "urgent".to_string()],
        ..Default::default()
    };

    storage.create_issue(&epic1).unwrap();
    storage.create_issue(&epic2).unwrap();

    let retrieved1 = storage.get_issue("epic-compare-1").unwrap().unwrap();
    let retrieved2 = storage.get_issue("epic-compare-2").unwrap().unwrap();

    assert_eq!(retrieved1.priority, retrieved2.priority);
    assert_ne!(retrieved1.labels, retrieved2.labels);
}

#[test]
fn test_epic_comprehensive_priority_labels() {
    // Comprehensive test covering all aspects of epic labels and priorities
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic matching bf-rdnyh structure
    let epic = Issue {
        id: "bf-rdnyh-test".to_string(),
        title: "Test Epic Labels Priority".to_string(),
        issue_type: IssueType::Epic,
        status: Status::InProgress,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "priority-test".to_string()],
        assignee: Some("claude-code-glm-4.7-needle1".to_string()),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Verify basic properties
    let retrieved = storage.get_issue("bf-rdnyh-test").unwrap().unwrap();
    assert_eq!(retrieved.id, "bf-rdnyh-test");
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.status, Status::InProgress);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(format!("{}", retrieved.priority), "P0");

    // Verify labels
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"priority-test".to_string()));

    // Verify assignee
    assert_eq!(
        retrieved.assignee,
        Some("claude-code-glm-4.7-needle1".to_string())
    );

    // Add more labels
    storage.add_label("bf-rdnyh-test", "urgent").unwrap();
    storage.add_label("bf-rdnyh-test", "feature").unwrap();

    let updated = storage.get_issue("bf-rdnyh-test").unwrap().unwrap();
    assert_eq!(updated.labels.len(), 4);
    assert!(updated.labels.contains(&"urgent".to_string()));
    assert!(updated.labels.contains(&"feature".to_string()));

    // Verify priority is unchanged
    assert_eq!(updated.priority, Priority::CRITICAL);
    assert_eq!(updated.priority.0, 0);

    // Test JSON serialization
    let json = serde_json::to_string(&updated).unwrap();
    assert!(json.contains("\"priority\":0"));
    assert!(json.contains("\"issue_type\":\"epic\""));
    assert!(json.contains("critical"));
    assert!(json.contains("priority-test"));

    // Test roundtrip
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.labels.len(), updated.labels.len());
    for label in &updated.labels {
        assert!(deserialized.labels.contains(label));
    }
}
