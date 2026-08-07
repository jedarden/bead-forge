//! P0 Critical Bug Testing
//! Tests for Priority 0 (Critical) bug creation, operations, and persistence
//! This covers the bug-specific scenario for P0 critical beads

use bead_forge::model::{Dependency, DependencyType, Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use std::collections::BTreeMap;
use tempfile::TempDir;

fn create_p0_bug(id: &str, title: &str, labels: Vec<&str>) -> Issue {
    let now = chrono::Utc::now();
    Issue {
        id: id.to_string(),
        content_hash: None,
        title: title.to_string(),
        description: Some(format!("Critical bug: {}", title)),
        design: None,
        acceptance_criteria: None,
        notes: None,
        status: Status::Open,
        priority: Priority::CRITICAL, // P0 = Critical
        issue_type: IssueType::Bug,
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
        labels: labels.iter().map(|s| s.to_string()).collect(),
        dependencies: vec![],
        comments: vec![],
        events: vec![],
        annotations: BTreeMap::new(),
    }
}

#[test]
fn test_p0_bug_creation_basic() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let bug = create_p0_bug("bf-crit-1", "Security vulnerability in auth", vec!["security"]);
    storage.create_issue(&bug).unwrap();

    let retrieved = storage.get_issue("bf-crit-1").unwrap().unwrap();
    assert_eq!(retrieved.id, "bf-crit-1");
    assert_eq!(retrieved.issue_type, IssueType::Bug);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(retrieved.status, Status::Open);
    assert_eq!(retrieved.labels.len(), 1);
    assert!(retrieved.labels.contains(&"security".to_string()));
}

#[test]
fn test_p0_bug_with_multiple_labels() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let bug = create_p0_bug(
        "bf-crit-2",
        "Production data loss bug",
        vec!["production", "data-loss", "urgent", "blocking"],
    );
    storage.create_issue(&bug).unwrap();

    let retrieved = storage.get_issue("bf-crit-2").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.issue_type, IssueType::Bug);
    assert_eq!(retrieved.labels.len(), 4);
    assert!(retrieved.labels.contains(&"production".to_string()));
    assert!(retrieved.labels.contains(&"data-loss".to_string()));
    assert!(retrieved.labels.contains(&"urgent".to_string()));
    assert!(retrieved.labels.contains(&"blocking".to_string()));
}

#[test]
fn test_p0_bug_serialization() {
    let bug = create_p0_bug(
        "bf-crit-json",
        "Critical bug for JSON testing",
        vec!["json-test", "serialization", "critical"],
    );

    // Serialize to JSON
    let json = serde_json::to_string(&bug).unwrap();

    // Verify JSON structure
    assert!(json.contains("\"priority\":0"));
    assert!(json.contains("\"issue_type\":\"bug\""));
    assert!(json.contains("json-test"));
    assert!(json.contains("serialization"));
    assert!(json.contains("critical"));

    // Deserialize and verify
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.priority.0, 0);
    assert_eq!(deserialized.issue_type, IssueType::Bug);
    assert_eq!(deserialized.labels.len(), 3);
}

#[test]
fn test_p0_bug_with_assignee() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let mut bug = create_p0_bug("bf-crit-assigned", "Critical bug with assignee", vec!["assigned"]);
    bug.assignee = Some("claude-code-glm-4.7-delta".to_string());
    storage.create_issue(&bug).unwrap();

    let retrieved = storage.get_issue("bf-crit-assigned").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.issue_type, IssueType::Bug);
    assert_eq!(retrieved.assignee, Some("claude-code-glm-4.7-delta".to_string()));
    assert!(retrieved.labels.contains(&"assigned".to_string()));
}

#[test]
fn test_p0_bug_clear_assignee() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create bug with assignee
    let mut bug = create_p0_bug("bf-crit-clear", "Critical bug for assignee clear", vec![]);
    bug.assignee = Some("stale-assignee".to_string());
    storage.create_issue(&bug).unwrap();

    // Verify initial state
    let initial = storage.get_issue("bf-crit-clear").unwrap().unwrap();
    assert_eq!(initial.assignee, Some("stale-assignee".to_string()));

    // Clear assignee using empty string pattern
    let changes = bug.clear_assignee("test-actor".to_string());
    storage.update_issue("bf-crit-clear", &changes).unwrap();

    // Verify assignee is cleared (NULL)
    let cleared = storage.get_issue("bf-crit-clear").unwrap().unwrap();
    assert_eq!(cleared.assignee, None);
    // P0 priority should remain unchanged
    assert_eq!(cleared.priority, Priority::CRITICAL);
}

#[test]
fn test_p0_bug_label_operations() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let bug = create_p0_bug("bf-crit-labels", "Critical bug for label ops", vec!["initial"]);
    storage.create_issue(&bug).unwrap();

    // Add labels
    storage.add_label("bf-crit-labels", "investigating").unwrap();
    storage.add_label("bf-crit-labels", "needs-hotfix").unwrap();

    let after_add = storage.get_issue("bf-crit-labels").unwrap().unwrap();
    assert_eq!(after_add.labels.len(), 3);
    assert_eq!(after_add.priority, Priority::CRITICAL);

    // Remove a label
    storage.remove_label("bf-crit-labels", "initial").unwrap();

    let after_remove = storage.get_issue("bf-crit-labels").unwrap().unwrap();
    assert_eq!(after_remove.labels.len(), 2);
    assert!(after_remove.labels.contains(&"investigating".to_string()));
    assert!(after_remove.labels.contains(&"needs-hotfix".to_string()));
    assert!(!after_remove.labels.contains(&"initial".to_string()));
    // Priority should remain P0
    assert_eq!(after_remove.priority, Priority::CRITICAL);
}

#[test]
fn test_p0_bug_closed_and_reopened() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let mut bug = create_p0_bug("bf-crit-reopen", "Critical bug for close/reopen", vec!["reopened"]);
    bug.assignee = Some("fixer".to_string());
    storage.create_issue(&bug).unwrap();

    // Close the bug using update_issue to avoid worker_sessions table dependency
    use bead_forge::model::IssueChanges;
    let close_changes = IssueChanges {
        status: Some(Status::Closed),
        assignee: Some(String::new()), // Clear assignee on close
        ..Default::default()
    };
    storage.update_issue("bf-crit-reopen", &close_changes).unwrap();

    let closed = storage.get_issue("bf-crit-reopen").unwrap().unwrap();
    assert_eq!(closed.status, Status::Closed);
    assert_eq!(closed.priority, Priority::CRITICAL);
    assert_eq!(closed.labels.len(), 1);

    // Reopen the bug using Issue::reopen() and update_issue
    let bug_to_reopen = storage.get_issue("bf-crit-reopen").unwrap().unwrap();
    let mut reopen_changes = bug_to_reopen.reopen("fixer".to_string());
    reopen_changes.assignee = Some(String::new()); // Clear assignee on reopen
    storage.update_issue("bf-crit-reopen", &reopen_changes).unwrap();

    let reopened = storage.get_issue("bf-crit-reopen").unwrap().unwrap();
    assert_eq!(reopened.status, Status::Open);
    assert_eq!(reopened.assignee, None); // Assignee cleared on reopen
    assert_eq!(reopened.priority, Priority::CRITICAL); // Priority preserved
}

#[test]
fn test_p0_bug_with_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create dependent bug (P1)
    let mut dep_bug = create_p0_bug("bf-dep-1", "Lower priority dependency", vec!["dependency"]);
    dep_bug.priority = Priority::HIGH;
    dep_bug.id = "bf-dep-1".to_string();
    storage.create_issue(&dep_bug).unwrap();

    // Create P0 critical bug with dependency
    let mut bug = create_p0_bug("bf-crit-deps", "Critical bug with dependencies", vec!["with-deps"]);
    let dep = Dependency {
        issue_id: "bf-crit-deps".to_string(),
        depends_on_id: "bf-dep-1".to_string(),
        dep_type: DependencyType::Blocks,
        created_at: chrono::Utc::now(),
        created_by: Some("test".to_string()),
        ..Default::default()
    };
    bug.dependencies = vec![dep];
    storage.create_issue(&bug).unwrap();

    let retrieved = storage.get_issue("bf-crit-deps").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.issue_type, IssueType::Bug);
    assert_eq!(retrieved.dependencies.len(), 1);
    assert_eq!(retrieved.dependencies[0].depends_on_id, "bf-dep-1");
}

#[test]
fn test_p0_bug_persistence_across_reopen() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create and persist
    {
        let storage = Storage::open(&db_path).unwrap();
        let bug = create_p0_bug(
            "bf-crit-persist",
            "Critical bug for persistence test",
            vec!["persistent", "critical"],
        );
        storage.create_issue(&bug).unwrap();
    }

    // Reopen and verify
    let storage = Storage::open(&db_path).unwrap();
    let retrieved = storage.get_issue("bf-crit-persist").unwrap().unwrap();

    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(retrieved.issue_type, IssueType::Bug);
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"persistent".to_string()));
    assert!(retrieved.labels.contains(&"critical".to_string()));
}

#[test]
fn test_multiple_p0_bugs_different_categories() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create multiple P0 bugs in different categories
    let bugs = vec![
        (
            "bf-security-1",
            "Authentication bypass",
            vec!["security", "auth", "blocking"],
        ),
        (
            "bf-perf-1",
            "Database deadlock causing outage",
            vec!["performance", "outage", "blocking"],
        ),
        (
            "bf-data-1",
            "Customer data corruption",
            vec!["data-loss", "customer-impact", "urgent"],
        ),
    ];

    for (id, title, labels) in &bugs {
        let bug = create_p0_bug(id, title, labels.clone());
        storage.create_issue(&bug).unwrap();
    }

    // Verify all bugs stored correctly
    for (id, _, expected_labels) in &bugs {
        let retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(retrieved.priority, Priority::CRITICAL);
        assert_eq!(retrieved.priority.0, 0);
        assert_eq!(retrieved.issue_type, IssueType::Bug);
        assert_eq!(retrieved.labels.len(), expected_labels.len());
        for label in expected_labels.iter() {
            assert!(retrieved.labels.contains(&label.to_string()));
        }
    }
}

#[test]
fn test_p0_bug_with_special_labels() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let bug = create_p0_bug(
        "bf-crit-special",
        "Critical bug with special label formats",
        vec!["priority:p0", "severity:critical", "area:auth", "hotfix-required"],
    );
    storage.create_issue(&bug).unwrap();

    let retrieved = storage.get_issue("bf-crit-special").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 4);
    assert!(retrieved.labels.contains(&"priority:p0".to_string()));
    assert!(retrieved.labels.contains(&"severity:critical".to_string()));
    assert!(retrieved.labels.contains(&"area:auth".to_string()));
    assert!(retrieved.labels.contains(&"hotfix-required".to_string()));
}

#[test]
fn test_p0_bug_unicode_labels() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let bug = create_p0_bug(
        "bf-crit-unicode",
        "Critical bug with unicode labels",
        vec!["🔥-critical", "🐛-bug", "🚨-urgent"],
    );
    storage.create_issue(&bug).unwrap();

    let retrieved = storage.get_issue("bf-crit-unicode").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 3);
    assert!(retrieved.labels.contains(&"🔥-critical".to_string()));
    assert!(retrieved.labels.contains(&"🐛-bug".to_string()));
    assert!(retrieved.labels.contains(&"🚨-urgent".to_string()));
}

#[test]
fn test_p0_bug_label_idempotency() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let bug = create_p0_bug("bf-crit-idemp", "Critical bug for idempotency", vec!["test"]);
    storage.create_issue(&bug).unwrap();

    // Add same label twice - should only appear once
    storage.add_label("bf-crit-idemp", "duplicate").unwrap();
    storage.add_label("bf-crit-idemp", "duplicate").unwrap();

    let retrieved = storage.get_issue("bf-crit-idemp").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 2); // "test" + "duplicate" (not 3)

    // Remove non-existent label - should succeed (no-op)
    storage.remove_label("bf-crit-idemp", "nonexistent").unwrap();

    let after_nonexist = storage.get_issue("bf-crit-idemp").unwrap().unwrap();
    assert_eq!(after_nonexist.labels.len(), 2); // Still 2

    // Remove same label twice - second should be no-op
    storage.remove_label("bf-crit-idemp", "duplicate").unwrap();
    storage.remove_label("bf-crit-idemp", "duplicate").unwrap();

    let after_remove = storage.get_issue("bf-crit-idemp").unwrap().unwrap();
    assert_eq!(after_remove.labels.len(), 1); // Only "test" remains
    assert_eq!(after_remove.priority, Priority::CRITICAL); // Priority unchanged
}
