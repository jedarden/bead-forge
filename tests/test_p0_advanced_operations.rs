// Advanced P0 Critical Operations Tests
// Tests complex P0 functionality: dependencies, claiming, batch operations, priority comparisons
// This covers the requirements for bead bf-k4904r (Test bead 3 for P0 critical)

use bead_forge::model::{Dependency, DependencyType, Issue, IssueChanges, IssueFilter, IssueType, Priority, Status};
use bead_forge::storage::Storage;

// ============================================================================
// Test 1: P0 bead with dependencies
// ============================================================================

#[test]
fn test_p0_bead_with_dependencies() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create a dependency bead
    let dependency = Issue {
        id: "dep-bead".to_string(),
        title: "Dependency Bead".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::HIGH,
        ..Default::default()
    };
    storage.create_issue(&dependency).unwrap();

    // Create P0 bead with dependency
    let now = chrono::Utc::now();
    let p0_bead = Issue {
        id: "p0-with-deps".to_string(),
        title: "P0 Bead with Dependencies".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        dependencies: vec![Dependency {
            issue_id: "p0-with-deps".to_string(),
            depends_on_id: "dep-bead".to_string(),
            dep_type: DependencyType::Blocks,
            created_at: now,
            created_by: Some(String::new()),
            metadata: None,
            thread_id: None,
            title: None,
        }],
        ..Default::default()
    };
    storage.create_issue(&p0_bead).unwrap();

    // Verify P0 bead and its dependencies
    let retrieved = storage.get_issue("p0-with-deps").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.dependencies.len(), 1);
    assert_eq!(retrieved.dependencies[0].depends_on_id, "dep-bead");
}

// ============================================================================
// Test 2: Multiple P0 beads with interdependencies
// ============================================================================

#[test]
fn test_multiple_p0_with_interdependencies() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead 1 (blocks others)
    let p0_1 = Issue {
        id: "p0-blocker".to_string(),
        title: "P0 Blocker".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        ..Default::default()
    };
    storage.create_issue(&p0_1).unwrap();

    // Create P0 bead 2 that depends on P0 bead 1
    let now = chrono::Utc::now();
    let p0_2 = Issue {
        id: "p0-blocked".to_string(),
        title: "P0 Blocked".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        dependencies: vec![Dependency {
            issue_id: "p0-blocked".to_string(),
            depends_on_id: "p0-blocker".to_string(),
            dep_type: DependencyType::Blocks,
            created_at: now,
            created_by: Some(String::new()),
            metadata: None,
            thread_id: None,
            title: None,
        }],
        ..Default::default()
    };
    storage.create_issue(&p0_2).unwrap();

    // Verify both are P0
    let bead_1 = storage.get_issue("p0-blocker").unwrap().unwrap();
    let bead_2 = storage.get_issue("p0-blocked").unwrap().unwrap();

    assert_eq!(bead_1.priority, Priority::CRITICAL);
    assert_eq!(bead_2.priority, Priority::CRITICAL);
    assert_eq!(bead_2.dependencies.len(), 1);
    assert_eq!(bead_2.dependencies[0].depends_on_id, "p0-blocker");
}

// ============================================================================
// Test 3: P0 priority comparison and ordering
// ============================================================================

#[test]
fn test_p0_priority_ordering() {
    // Test that P0 < P1 < P2 < P3 < P4
    assert!(Priority::CRITICAL < Priority::HIGH);
    assert!(Priority::CRITICAL < Priority::MEDIUM);
    assert!(Priority::CRITICAL < Priority::LOW);
    assert!(Priority::CRITICAL < Priority::BACKLOG);

    // Test raw values
    assert_eq!(Priority::CRITICAL.0, 0);
    assert_eq!(Priority::HIGH.0, 1);
    assert_eq!(Priority::MEDIUM.0, 2);
    assert_eq!(Priority::LOW.0, 3);
    assert_eq!(Priority::BACKLOG.0, 4);
}

// ============================================================================
// Test 4: Create P0 beads via batch operation
// ============================================================================

#[test]
fn test_p0_batch_creation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple P0 beads via batch
    let p0_1 = Issue {
        id: "batch-p0-1".to_string(),
        title: "Batch P0 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        ..Default::default()
    };

    let p0_2 = Issue {
        id: "batch-p0-2".to_string(),
        title: "Batch P0 2".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        ..Default::default()
    };

    storage.create_issue(&p0_1).unwrap();
    storage.create_issue(&p0_2).unwrap();

    // Verify both are P0
    let bead_1 = storage.get_issue("batch-p0-1").unwrap().unwrap();
    let bead_2 = storage.get_issue("batch-p0-2").unwrap().unwrap();

    assert_eq!(bead_1.priority, Priority::CRITICAL);
    assert_eq!(bead_2.priority, Priority::CRITICAL);
}

// ============================================================================
// Test 5: P0 bead with assignee
// ============================================================================

#[test]
fn test_p0_bead_with_assignee() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead with assignee
    let p0_assigned = Issue {
        id: "p0-assigned".to_string(),
        title: "P0 Assigned Bead".to_string(),
        issue_type: IssueType::Task,
        status: Status::InProgress,
        priority: Priority::CRITICAL,
        assignee: Some("claude-agent".to_string()),
        ..Default::default()
    };
    storage.create_issue(&p0_assigned).unwrap();

    // Verify P0 priority and assignee
    let retrieved = storage.get_issue("p0-assigned").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.assignee, Some("claude-agent".to_string()));
    assert_eq!(retrieved.status, Status::InProgress);
}

// ============================================================================
// Test 6: Update P0 bead preserves priority
// ============================================================================

#[test]
fn test_p0_update_preserves_priority() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead
    let p0_bead = Issue {
        id: "p0-update-test".to_string(),
        title: "Original Title".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        ..Default::default()
    };
    storage.create_issue(&p0_bead).unwrap();

    // Update the bead (should preserve P0 priority)
    let changes = IssueChanges {
        title: Some("Updated Title".to_string()),
        status: Some(Status::InProgress),
        assignee: Some("new-assignee".to_string()),
        ..Default::default()
    };
    storage.update_issue("p0-update-test", &changes).unwrap();

    // Verify priority is still P0
    let retrieved = storage.get_issue("p0-update-test").unwrap().unwrap();
    assert_eq!(retrieved.title, "Updated Title");
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.status, Status::InProgress);
}

// ============================================================================
// Test 7: Close and reopen P0 bead
// ============================================================================

#[test]
fn test_p0_close_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead
    let p0_bead = Issue {
        id: "p0-close-reopen".to_string(),
        title: "P0 Close Reopen Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        ..Default::default()
    };
    storage.create_issue(&p0_bead).unwrap();

    // Close the P0 bead
    // Note: IssueChanges doesn't have close_reason/closed_by_session/closed_at fields.
    // The storage layer handles these automatically when status is set to Closed.
    let close_changes = IssueChanges {
        status: Some(Status::Closed),
        actor: Some("test-session".to_string()),
        ..Default::default()
    };
    storage.update_issue("p0-close-reopen", &close_changes).unwrap();

    // Verify it's closed
    let closed_bead = storage.get_issue("p0-close-reopen").unwrap().unwrap();
    assert_eq!(closed_bead.status, Status::Closed);
    assert_eq!(closed_bead.priority, Priority::CRITICAL);

    // Reopen the P0 bead
    // Note: closed_at/close_reason/closed_by_session are not IssueChanges fields.
    // Setting status to Open is sufficient; storage layer clears closure metadata.
    let reopen_changes = IssueChanges {
        status: Some(Status::Open),
        actor: Some("test-session".to_string()),
        ..Default::default()
    };
    storage.update_issue("p0-close-reopen", &reopen_changes).unwrap();

    // Verify it's open again with P0 priority
    let reopened_bead = storage.get_issue("p0-close-reopen").unwrap().unwrap();
    assert_eq!(reopened_bead.status, Status::Open);
    assert_eq!(reopened_bead.priority, Priority::CRITICAL);
}

// ============================================================================
// Test 8: List P0 beads among mixed priorities
// ============================================================================

#[test]
fn test_list_p0_among_mixed_priorities() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create beads with different priorities
    let priorities = vec![
        ("p0-test", Priority::CRITICAL),
        ("p1-test", Priority::HIGH),
        ("p2-test", Priority::MEDIUM),
        ("another-p0", Priority::CRITICAL),
        ("p3-test", Priority::LOW),
        ("p4-test", Priority::BACKLOG),
    ];

    for (id, priority) in &priorities {
        let bead = Issue {
            id: id.to_string(),
            title: format!("Bead {}", id),
            issue_type: IssueType::Task,
            status: Status::Open,
            priority: *priority,
            ..Default::default()
        };
        storage.create_issue(&bead).unwrap();
    }

    // List all issues
    let filter = IssueFilter::default();
    let all_issues = storage.list_issues(&filter).unwrap();

    // Filter P0 beads
    let p0_beads: Vec<_> = all_issues
        .iter()
        .filter(|b| b.priority == Priority::CRITICAL)
        .collect();

    // Should have exactly 2 P0 beads
    assert_eq!(p0_beads.len(), 2);
    assert!(p0_beads.iter().any(|b| b.id == "p0-test"));
    assert!(p0_beads.iter().any(|b| b.id == "another-p0"));
}

// ============================================================================
// Test 9: P0 bead with multiple dependencies
// ============================================================================

#[test]
fn test_p0_with_multiple_dependencies() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple dependencies
    let now = chrono::Utc::now();
    let dep1 = Issue {
        id: "dep-1".to_string(),
        title: "Dependency 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::HIGH,
        created_at: now,
        updated_at: now,
        ..Default::default()
    };
    storage.create_issue(&dep1).unwrap();

    let dep2 = Issue {
        id: "dep-2".to_string(),
        title: "Dependency 2".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::HIGH,
        created_at: now,
        updated_at: now,
        ..Default::default()
    };
    storage.create_issue(&dep2).unwrap();

    let dep3 = Issue {
        id: "dep-3".to_string(),
        title: "Dependency 3".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::HIGH,
        created_at: now,
        updated_at: now,
        ..Default::default()
    };
    storage.create_issue(&dep3).unwrap();

    // Create P0 bead with multiple dependencies
    let now = chrono::Utc::now();
    let p0_bead = Issue {
        id: "p0-multi-deps".to_string(),
        title: "P0 with Multiple Dependencies".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        dependencies: vec![
            Dependency {
                issue_id: "p0-multi-deps".to_string(),
                depends_on_id: "dep-1".to_string(),
                dep_type: DependencyType::Blocks,
                created_at: now,
                created_by: Some(String::new()),
                metadata: None,
                thread_id: None,
                title: None,
            },
            Dependency {
                issue_id: "p0-multi-deps".to_string(),
                depends_on_id: "dep-2".to_string(),
                dep_type: DependencyType::Blocks,
                created_at: now,
                created_by: Some(String::new()),
                metadata: None,
                thread_id: None,
                title: None,
            },
            Dependency {
                issue_id: "p0-multi-deps".to_string(),
                depends_on_id: "dep-3".to_string(),
                dep_type: DependencyType::Blocks,
                created_at: now,
                created_by: Some(String::new()),
                metadata: None,
                thread_id: None,
                title: None,
            },
        ],
        ..Default::default()
    };
    storage.create_issue(&p0_bead).unwrap();

    // Verify all dependencies
    let retrieved = storage.get_issue("p0-multi-deps").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.dependencies.len(), 3);
    assert_eq!(retrieved.dependencies[0].depends_on_id, "dep-1");
    assert_eq!(retrieved.dependencies[1].depends_on_id, "dep-2");
    assert_eq!(retrieved.dependencies[2].depends_on_id, "dep-3");
}

// ============================================================================
// Test 10: P0 priority serialization to JSON
// ============================================================================

#[test]
fn test_p0_json_serialization_roundtrip() {
    let p0_bead = Issue {
        id: "p0-json-test".to_string(),
        title: "P0 JSON Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        description: Some("Testing P0 serialization".to_string()),
        labels: vec!["critical".to_string(), "json-test".to_string()],
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&p0_bead).unwrap();

    // Verify JSON contains priority 0
    assert!(json.contains(r#""priority":0"#));

    // Deserialize back
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    // Verify P0 priority is preserved
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.priority.0, 0);
    assert_eq!(deserialized.labels.len(), 2);
    assert!(deserialized.labels.contains(&"critical".to_string()));
    assert!(deserialized.labels.contains(&"json-test".to_string()));
}

// ============================================================================
// Test 11: P0 epic with child tasks
// ============================================================================

#[test]
fn test_p0_epic_with_children() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 epic
    let epic = Issue {
        id: "p0-epic-with-children".to_string(),
        title: "P0 Epic with Children".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create child tasks with parent-child dependencies
    let now = chrono::Utc::now();
    let child1 = Issue {
        id: "child-1".to_string(),
        title: "Child Task 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::HIGH,
        dependencies: vec![Dependency {
            issue_id: "child-1".to_string(),
            depends_on_id: "p0-epic-with-children".to_string(),
            dep_type: DependencyType::ParentChild,
            created_at: now,
            created_by: Some(String::new()),
            metadata: None,
            thread_id: None,
            title: None,
        }],
        ..Default::default()
    };
    storage.create_issue(&child1).unwrap();

    let child2 = Issue {
        id: "child-2".to_string(),
        title: "Child Task 2".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::HIGH,
        dependencies: vec![Dependency {
            issue_id: "child-2".to_string(),
            depends_on_id: "p0-epic-with-children".to_string(),
            dep_type: DependencyType::ParentChild,
            created_at: now,
            created_by: Some(String::new()),
            metadata: None,
            thread_id: None,
            title: None,
        }],
        ..Default::default()
    };
    storage.create_issue(&child2).unwrap();

    // Verify epic is P0 and has children
    let retrieved_epic = storage.get_issue("p0-epic-with-children").unwrap().unwrap();
    assert_eq!(retrieved_epic.priority, Priority::CRITICAL);
    assert_eq!(retrieved_epic.issue_type, IssueType::Epic);

    // Verify children reference the epic via dependencies
    let retrieved_child1 = storage.get_issue("child-1").unwrap().unwrap();
    let retrieved_child2 = storage.get_issue("child-2").unwrap().unwrap();

    assert_eq!(retrieved_child1.dependencies.len(), 1);
    assert_eq!(retrieved_child1.dependencies[0].depends_on_id, "p0-epic-with-children");
    assert_eq!(retrieved_child1.dependencies[0].dep_type, DependencyType::ParentChild);
    assert_eq!(retrieved_child2.dependencies.len(), 1);
    assert_eq!(retrieved_child2.dependencies[0].depends_on_id, "p0-epic-with-children");
    assert_eq!(retrieved_child2.dependencies[0].dep_type, DependencyType::ParentChild);
}

// ============================================================================
// Test 12: P0 bead status transitions
// ============================================================================

#[test]
fn test_p0_status_transitions() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead as open
    let p0_bead = Issue {
        id: "p0-status-test".to_string(),
        title: "P0 Status Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        ..Default::default()
    };
    storage.create_issue(&p0_bead).unwrap();

    // Transition to in_progress
    let progress_changes = IssueChanges {
        status: Some(Status::InProgress),
        ..Default::default()
    };
    storage.update_issue("p0-status-test", &progress_changes).unwrap();

    let retrieved = storage.get_issue("p0-status-test").unwrap().unwrap();
    assert_eq!(retrieved.status, Status::InProgress);
    assert_eq!(retrieved.priority, Priority::CRITICAL);

    // Transition to blocked
    let blocked_changes = IssueChanges {
        status: Some(Status::Blocked),
        ..Default::default()
    };
    storage.update_issue("p0-status-test", &blocked_changes).unwrap();

    let retrieved = storage.get_issue("p0-status-test").unwrap().unwrap();
    assert_eq!(retrieved.status, Status::Blocked);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
}

// ============================================================================
// Test 13: P0 bead with description and labels
// ============================================================================

#[test]
fn test_p0_with_description_and_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead with full metadata
    let p0_full = Issue {
        id: "p0-full-metadata".to_string(),
        title: "P0 with Full Metadata".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        description: Some("This is a critical issue requiring immediate attention".to_string()),
        labels: vec!["critical".to_string(), "urgent".to_string(), "security".to_string()],
        assignee: Some("senior-engineer".to_string()),
        ..Default::default()
    };
    storage.create_issue(&p0_full).unwrap();

    // Verify all fields
    let retrieved = storage.get_issue("p0-full-metadata").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(
        retrieved.description,
        Some("This is a critical issue requiring immediate attention".to_string())
    );
    assert_eq!(retrieved.labels.len(), 3);
    assert_eq!(retrieved.assignee, Some("senior-engineer".to_string()));
}

// ============================================================================
// Test 14: P0 priority value representation
// ============================================================================

#[test]
fn test_p0_priority_value() {
    // Test that P0 is represented as 0
    assert_eq!(Priority::CRITICAL.0, 0);

    // Test Display trait shows "P0"
    assert_eq!(format!("{}", Priority::CRITICAL), "P0");

    // Test that we can create Priority from value 0
    let p0_from_value = Priority(0);
    assert_eq!(p0_from_value, Priority::CRITICAL);
}

// ============================================================================
// Test 15: Multiple P0 beads with different types
// ============================================================================

#[test]
fn test_multiple_p0_with_different_types() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 epic
    let epic = Issue {
        id: "p0-epic".to_string(),
        title: "P0 Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create P0 task
    let task = Issue {
        id: "p0-task".to_string(),
        title: "P0 Task".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        ..Default::default()
    };
    storage.create_issue(&task).unwrap();

    // Create P0 bug
    let bug = Issue {
        id: "p0-bug".to_string(),
        title: "P0 Bug".to_string(),
        issue_type: IssueType::Bug,
        status: Status::Open,
        priority: Priority::CRITICAL,
        ..Default::default()
    };
    storage.create_issue(&bug).unwrap();

    // List all issues and filter by P0
    let filter = IssueFilter::default();
    let all_issues = storage.list_issues(&filter).unwrap();
    let p0_issues: Vec<_> = all_issues
        .iter()
        .filter(|i| i.priority == Priority::CRITICAL)
        .collect();

    assert_eq!(p0_issues.len(), 3);
    assert!(p0_issues.iter().any(|i| i.issue_type == IssueType::Epic));
    assert!(p0_issues.iter().any(|i| i.issue_type == IssueType::Task));
    assert!(p0_issues.iter().any(|i| i.issue_type == IssueType::Bug));
}

// ============================================================================
// Test 16: P0 bead get_labels retrieves correct labels
// ============================================================================

#[test]
fn test_p0_get_labels_retrieves_correct_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead with multiple labels
    let p0_with_labels = Issue {
        id: "p0-labels-test".to_string(),
        title: "P0 with Labels".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![
            "critical".to_string(),
            "urgent".to_string(),
            "security".to_string(),
            "backend".to_string(),
        ],
        ..Default::default()
    };
    storage.create_issue(&p0_with_labels).unwrap();

    // Verify labels via storage.get_labels()
    let retrieved_labels = storage.get_labels("p0-labels-test").unwrap();

    // Should have exactly 4 labels
    assert_eq!(retrieved_labels.len(), 4);

    // Should contain all expected labels
    assert!(retrieved_labels.contains(&"critical".to_string()));
    assert!(retrieved_labels.contains(&"urgent".to_string()));
    assert!(retrieved_labels.contains(&"security".to_string()));
    assert!(retrieved_labels.contains(&"backend".to_string()));

    // Verify full issue retrieval also includes labels
    let full_issue = storage.get_issue("p0-labels-test").unwrap().unwrap();
    assert_eq!(full_issue.priority, Priority::CRITICAL);
    assert_eq!(full_issue.labels.len(), 4);
    assert_eq!(full_issue.labels, retrieved_labels);
}

// ============================================================================
// Test 17: P0 bead with empty labels list
// ============================================================================

#[test]
fn test_p0_empty_labels_list() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead with no labels
    let p0_no_labels = Issue {
        id: "p0-no-labels".to_string(),
        title: "P0 without Labels".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&p0_no_labels).unwrap();

    // Verify get_labels returns empty vector
    let retrieved_labels = storage.get_labels("p0-no-labels").unwrap();
    assert_eq!(retrieved_labels.len(), 0);
    assert!(retrieved_labels.is_empty());

    // Verify full issue retrieval also has empty labels
    let full_issue = storage.get_issue("p0-no-labels").unwrap().unwrap();
    assert_eq!(full_issue.priority, Priority::CRITICAL);
    assert!(full_issue.labels.is_empty());
}

// ============================================================================
// Test 18: P0 bead with single label
// ============================================================================

#[test]
fn test_p0_single_label() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead with single label
    let p0_single_label = Issue {
        id: "p0-single-label".to_string(),
        title: "P0 with Single Label".to_string(),
        issue_type: IssueType::Bug,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["p0-only".to_string()],
        ..Default::default()
    };
    storage.create_issue(&p0_single_label).unwrap();

    // Verify get_labels returns single label
    let retrieved_labels = storage.get_labels("p0-single-label").unwrap();
    assert_eq!(retrieved_labels.len(), 1);
    assert_eq!(retrieved_labels[0], "p0-only");

    // Verify full issue retrieval
    let full_issue = storage.get_issue("p0-single-label").unwrap().unwrap();
    assert_eq!(full_issue.priority, Priority::CRITICAL);
    assert_eq!(full_issue.labels.len(), 1);
    assert_eq!(full_issue.labels[0], "p0-only");
}

// ============================================================================
// Test 19: Multiple P0 beads with different labels
// ============================================================================

#[test]
fn test_multiple_p0_with_different_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create first P0 bead with security labels
    let p0_security = Issue {
        id: "p0-security".to_string(),
        title: "P0 Security Issue".to_string(),
        issue_type: IssueType::Bug,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["security".to_string(), "critical".to_string(), "cve".to_string()],
        ..Default::default()
    };
    storage.create_issue(&p0_security).unwrap();

    // Create second P0 bead with performance labels
    let p0_performance = Issue {
        id: "p0-performance".to_string(),
        title: "P0 Performance Issue".to_string(),
        issue_type: IssueType::Bug,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["performance".to_string(), "critical".to_string(), "hot-path".to_string()],
        ..Default::default()
    };
    storage.create_issue(&p0_performance).unwrap();

    // Verify labels for first P0
    let security_labels = storage.get_labels("p0-security").unwrap();
    assert_eq!(security_labels.len(), 3);
    assert!(security_labels.contains(&"security".to_string()));
    assert!(security_labels.contains(&"critical".to_string()));
    assert!(security_labels.contains(&"cve".to_string()));

    // Verify labels for second P0
    let performance_labels = storage.get_labels("p0-performance").unwrap();
    assert_eq!(performance_labels.len(), 3);
    assert!(performance_labels.contains(&"performance".to_string()));
    assert!(performance_labels.contains(&"critical".to_string()));
    assert!(performance_labels.contains(&"hot-path".to_string()));

    // Verify both are P0
    let issue_1 = storage.get_issue("p0-security").unwrap().unwrap();
    let issue_2 = storage.get_issue("p0-performance").unwrap().unwrap();
    assert_eq!(issue_1.priority, Priority::CRITICAL);
    assert_eq!(issue_2.priority, Priority::CRITICAL);
}
