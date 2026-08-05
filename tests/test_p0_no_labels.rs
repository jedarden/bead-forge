//! P0 No Labels Testing
//! Tests for Priority 0 (Critical) beads without any labels
//! Verifies that P0 functionality works correctly when labels are absent

use bead_forge::model::{Dependency, DependencyType, Issue, IssueChanges, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use std::collections::BTreeMap;
use tempfile::TempDir;

// Helper function to create P0 beads without labels
fn create_p0_no_labels(id: &str, title: &str, issue_type: IssueType) -> Issue {
    let now = chrono::Utc::now();
    Issue {
        id: id.to_string(),
        content_hash: None,
        title: title.to_string(),
        description: Some(format!("Critical issue: {}", title)),
        design: None,
        acceptance_criteria: None,
        notes: None,
        status: Status::Open,
        priority: Priority::CRITICAL, // P0 = Critical
        issue_type,
        assignee: None,
        owner: None,
        estimated_minutes: None,
        created_at: now,
        created_by: Some("test".to_string()),
        updated_at: now,
        closed_at: None,
        close_reason: None,
        closed_by_session: None,
        due_at: None,
        defer_until: None,
        external_ref: None,
        source_system: None,
        source_repo: Some(".".to_string()),
        deleted_at: None,
        deleted_by: None,
        delete_reason: None,
        original_type: None,
        compaction_level: None,
        compacted_at: None,
        compacted_at_commit: None,
        original_size: None,
        sender: None,
        ephemeral: false,
        pinned: false,
        is_template: false,
        labels: vec![], // Empty labels - NO LABELS
        dependencies: vec![],
        comments: vec![],
        annotations: BTreeMap::new(),
    }
}

#[test]
fn test_p0_bug_creation_without_labels() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let bug = create_p0_no_labels("bf-nolabel-1", "Critical bug without labels", IssueType::Bug);
    storage.create_issue(&bug).unwrap();

    let retrieved = storage.get_issue("bf-nolabel-1").unwrap().unwrap();
    assert_eq!(retrieved.id, "bf-nolabel-1");
    assert_eq!(retrieved.issue_type, IssueType::Bug);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(retrieved.status, Status::Open);
    assert_eq!(retrieved.labels.len(), 0); // NO LABELS
    assert!(retrieved.labels.is_empty());
}

#[test]
fn test_p0_task_creation_without_labels() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let task = create_p0_no_labels("bf-nolabel-task-1", "Critical task without labels", IssueType::Task);
    storage.create_issue(&task).unwrap();

    let retrieved = storage.get_issue("bf-nolabel-task-1").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.issue_type, IssueType::Task);
    assert_eq!(retrieved.labels.len(), 0);
}

#[test]
fn test_p0_epic_creation_without_labels() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let epic = create_p0_no_labels("bf-nolabel-epic-1", "Critical epic without labels", IssueType::Epic);
    storage.create_issue(&epic).unwrap();

    let retrieved = storage.get_issue("bf-nolabel-epic-1").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.labels.len(), 0);
}

#[test]
fn test_p0_serialization_without_labels() {
    let bug = create_p0_no_labels(
        "bf-nolabel-json",
        "Critical bug for JSON testing without labels",
        IssueType::Bug,
    );

    // Serialize to JSON
    let json = serde_json::to_string(&bug).unwrap();

    // Verify JSON structure
    assert!(json.contains(r#""priority":0"#));
    assert!(json.contains(r#""issue_type":"bug""#));
    assert!(json.contains(r#""labels":[]"#)); // Empty labels array

    // Deserialize and verify
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.priority.0, 0);
    assert_eq!(deserialized.issue_type, IssueType::Bug);
    assert_eq!(deserialized.labels.len(), 0);
}

#[test]
fn test_p0_with_assignee_without_labels() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let mut bug = create_p0_no_labels("bf-nolabel-assigned", "Critical bug with assignee but no labels", IssueType::Bug);
    bug.assignee = Some("claude-code-glm-4.7-delta".to_string());
    storage.create_issue(&bug).unwrap();

    let retrieved = storage.get_issue("bf-nolabel-assigned").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.issue_type, IssueType::Bug);
    assert_eq!(retrieved.assignee, Some("claude-code-glm-4.7-delta".to_string()));
    assert_eq!(retrieved.labels.len(), 0);
}

#[test]
fn test_p0_clear_assignee_without_labels() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create bug with assignee but no labels
    let mut bug = create_p0_no_labels("bf-nolabel-clear", "Critical bug for assignee clear without labels", IssueType::Bug);
    bug.assignee = Some("stale-assignee".to_string());
    storage.create_issue(&bug).unwrap();

    // Verify initial state
    let initial = storage.get_issue("bf-nolabel-clear").unwrap().unwrap();
    assert_eq!(initial.assignee, Some("stale-assignee".to_string()));
    assert_eq!(initial.labels.len(), 0);

    // Clear assignee using empty string pattern
    let changes = IssueChanges {
        assignee: Some(String::new()), // Empty string clears assignee
        actor: Some("test-actor".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-nolabel-clear", &changes).unwrap();

    // Verify assignee is cleared (NULL) and still no labels
    let cleared = storage.get_issue("bf-nolabel-clear").unwrap().unwrap();
    assert_eq!(cleared.assignee, None);
    assert_eq!(cleared.priority, Priority::CRITICAL);
    assert_eq!(cleared.labels.len(), 0);
}

#[test]
fn test_p0_label_operations_without_initial_labels() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let bug = create_p0_no_labels("bf-nolabel-add", "Critical bug starting without labels", IssueType::Bug);
    storage.create_issue(&bug).unwrap();

    // Verify initial state has no labels
    let initial = storage.get_issue("bf-nolabel-add").unwrap().unwrap();
    assert_eq!(initial.labels.len(), 0);

    // Add labels dynamically
    storage.add_label("bf-nolabel-add", "investigating").unwrap();
    storage.add_label("bf-nolabel-add", "needs-hotfix").unwrap();

    let after_add = storage.get_issue("bf-nolabel-add").unwrap().unwrap();
    assert_eq!(after_add.labels.len(), 2);
    assert_eq!(after_add.priority, Priority::CRITICAL);

    // Remove labels
    storage.remove_label("bf-nolabel-add", "investigating").unwrap();
    storage.remove_label("bf-nolabel-add", "needs-hotfix").unwrap();

    let after_remove = storage.get_issue("bf-nolabel-add").unwrap().unwrap();
    assert_eq!(after_remove.labels.len(), 0); // Back to no labels
    assert_eq!(after_remove.priority, Priority::CRITICAL);
}

#[test]
fn test_p0_closed_and_reopened_without_labels() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let mut bug = create_p0_no_labels("bf-nolabel-reopen", "Critical bug for close/reopen without labels", IssueType::Bug);
    bug.assignee = Some("fixer".to_string());
    storage.create_issue(&bug).unwrap();

    // Verify initial state
    let initial = storage.get_issue("bf-nolabel-reopen").unwrap().unwrap();
    assert_eq!(initial.labels.len(), 0);

    // Close the bug
    let close_changes = IssueChanges {
        status: Some(Status::Closed),
        close_reason: Some("Fixed in production".to_string()),
        closed_by_session: Some("test-session".to_string()),
        actor: Some("fixer".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-nolabel-reopen", &close_changes).unwrap();

    let closed = storage.get_issue("bf-nolabel-reopen").unwrap().unwrap();
    assert_eq!(closed.status, Status::Closed);
    assert_eq!(closed.priority, Priority::CRITICAL);
    assert_eq!(closed.labels.len(), 0);

    // Reopen the bug (clears assignee)
    let reopen_changes = IssueChanges {
        status: Some(Status::Open),
        assignee: Some(String::new()), // Clear assignee on reopen
        actor: Some("test-actor".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-nolabel-reopen", &reopen_changes).unwrap();

    let reopened = storage.get_issue("bf-nolabel-reopen").unwrap().unwrap();
    assert_eq!(reopened.status, Status::Open);
    assert_eq!(reopened.assignee, None);
    assert_eq!(reopened.priority, Priority::CRITICAL);
    assert_eq!(reopened.labels.len(), 0);
}

#[test]
fn test_p0_with_dependencies_without_labels() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create dependent bug (P1)
    let mut dep_bug = create_p0_no_labels("bf-dep-nolabel-1", "Dependency without labels", IssueType::Bug);
    dep_bug.priority = Priority::HIGH;
    dep_bug.id = "bf-dep-nolabel-1".to_string();
    storage.create_issue(&dep_bug).unwrap();

    // Create P0 critical bug with dependency but no labels
    let mut bug = create_p0_no_labels("bf-nolabel-deps", "Critical bug with dependencies but no labels", IssueType::Bug);
    bug.dependencies = vec![Dependency {
        issue_id: "bf-nolabel-deps".to_string(),
        depends_on_id: "bf-dep-nolabel-1".to_string(),
        dep_type: DependencyType::Blocks,
        created_at: chrono::Utc::now(),
        created_by: Some("test".to_string()),
        ..Default::default()
    }];
    storage.create_issue(&bug).unwrap();

    let retrieved = storage.get_issue("bf-nolabel-deps").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.issue_type, IssueType::Bug);
    assert_eq!(retrieved.dependencies.len(), 1);
    assert_eq!(retrieved.dependencies[0].depends_on_id, "bf-dep-nolabel-1");
    assert_eq!(retrieved.labels.len(), 0);
}

#[test]
fn test_p0_persistence_across_reopen_without_labels() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create and persist
    {
        let storage = Storage::open(&db_path).unwrap();
        let bug = create_p0_no_labels(
            "bf-nolabel-persist",
            "Critical bug for persistence test without labels",
            IssueType::Bug,
        );
        storage.create_issue(&bug).unwrap();
    }

    // Reopen and verify
    let storage = Storage::open(&db_path).unwrap();
    let retrieved = storage.get_issue("bf-nolabel-persist").unwrap().unwrap();

    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(retrieved.issue_type, IssueType::Bug);
    assert_eq!(retrieved.labels.len(), 0);
}

#[test]
fn test_multiple_p0_bugs_different_categories_without_labels() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create multiple P0 bugs in different categories, all without labels
    let bugs = vec![
        ("bf-security-nolabel-1", "Authentication bypass", IssueType::Bug),
        ("bf-perf-nolabel-1", "Database deadlock causing outage", IssueType::Bug),
        ("bf-data-nolabel-1", "Customer data corruption", IssueType::Bug),
        ("bf-task-nolabel-1", "Critical hotfix deployment", IssueType::Task),
        ("bf-epic-nolabel-1", "Critical infrastructure migration", IssueType::Epic),
    ];

    for (id, title, issue_type) in &bugs {
        let bug = create_p0_no_labels(id, title, *issue_type);
        storage.create_issue(&bug).unwrap();
    }

    // Verify all bugs stored correctly without labels
    for (id, _, issue_type) in &bugs {
        let retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(retrieved.priority, Priority::CRITICAL);
        assert_eq!(retrieved.priority.0, 0);
        assert_eq!(retrieved.issue_type, *issue_type);
        assert_eq!(retrieved.labels.len(), 0);
    }
}

#[test]
fn test_p0_update_preserves_priority_without_labels() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create P0 bug without labels
    let p0_bead = create_p0_no_labels("bf-nolabel-update", "Original Title", IssueType::Task);
    storage.create_issue(&p0_bead).unwrap();

    // Update the bead (should preserve P0 priority and no labels)
    let updated = Issue {
        id: "bf-nolabel-update".to_string(),
        title: "Updated Title".to_string(),
        issue_type: IssueType::Task,
        status: Status::InProgress,
        priority: Priority::CRITICAL,
        labels: vec![], // Still no labels
        assignee: Some("new-assignee".to_string()),
        ..Default::default()
    };
    storage.update_issue(&updated).unwrap();

    // Verify priority is still P0 and still no labels
    let retrieved = storage.get_issue("bf-nolabel-update").unwrap().unwrap();
    assert_eq!(retrieved.title, "Updated Title");
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.status, Status::InProgress);
    assert_eq!(retrieved.labels.len(), 0);
}

#[test]
fn test_p0_with_description_without_labels() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create P0 bug with description but no labels
    let mut bug = create_p0_no_labels("bf-nolabel-desc", "Critical bug with description", IssueType::Bug);
    bug.description = Some("This is a critical security vulnerability requiring immediate patch".to_string());
    storage.create_issue(&bug).unwrap();

    // Verify all fields
    let retrieved = storage.get_issue("bf-nolabel-desc").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(
        retrieved.description,
        Some("This is a critical security vulnerability requiring immediate patch".to_string())
    );
    assert_eq!(retrieved.labels.len(), 0);
}

#[test]
fn test_p0_json_format_without_labels() {
    let bug = create_p0_no_labels(
        "bf-nolabel-json-fmt",
        "Critical bug for JSON format test",
        IssueType::Bug,
    );

    // Serialize to JSON
    let json = serde_json::to_string(&bug).unwrap();

    // Parse JSON to verify structure
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["priority"], 0);
    assert_eq!(parsed["issue_type"], "bug");
    assert_eq!(parsed["labels"].as_array().unwrap().len(), 0);
}

#[test]
fn test_p0_list_without_labels() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create multiple P0 bugs without labels
    for i in 1..=5 {
        let bug = create_p0_no_labels(
            &format!("bf-nolabel-list-{}", i),
            &format!("Critical bug {} for list test", i),
            IssueType::Bug,
        );
        storage.create_issue(&bug).unwrap();
    }

    // List all issues
    let all_issues = storage.list_issues().unwrap();

    // Filter P0 bugs
    let p0_bugs: Vec<_> = all_issues
        .iter()
        .filter(|b| b.priority == Priority::CRITICAL)
        .collect();

    // Should have exactly 5 P0 bugs, all without labels
    assert_eq!(p0_bugs.len(), 5);
    for bug in p0_bugs {
        assert_eq!(bug.labels.len(), 0);
        assert_eq!(bug.priority, Priority::CRITICAL);
    }
}

#[test]
fn test_p0_priority_value_without_labels() {
    let bug = create_p0_no_labels("bf-nolabel-prio", "Test P0 priority value", IssueType::Bug);

    // Test that P0 is represented as 0
    assert_eq!(bug.priority.0, 0);

    // Test Display trait shows "P0"
    assert_eq!(format!("{}", bug.priority), "P0");

    // Test that we can create Priority from value 0
    let p0_from_value = Priority(0);
    assert_eq!(p0_from_value, Priority::CRITICAL);
}

#[test]
fn test_p0_all_issue_types_without_labels() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Test all issue types as P0 without labels
    let issue_types = vec![
        ("bf-nolabel-bug", IssueType::Bug),
        ("bf-nolabel-task", IssueType::Task),
        ("bf-nolabel-story", IssueType::Story),
        ("bf-nolabel-epic", IssueType::Epic),
    ];

    for (id, issue_type) in &issue_types {
        let issue = create_p0_no_labels(id, &format!("P0 {:?}", issue_type), *issue_type);
        storage.create_issue(&issue).unwrap();
    }

    // Verify all were created correctly
    for (id, issue_type) in &issue_types {
        let retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(retrieved.priority, Priority::CRITICAL);
        assert_eq!(retrieved.issue_type, *issue_type);
        assert_eq!(retrieved.labels.len(), 0);
    }
}
