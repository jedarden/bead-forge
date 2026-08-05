//! P0 Single Label Assignment Tests
//! Tests Priority 0 (Critical) beads with exactly one label
//! This covers the requirements for bead bf-60leiq (P0 Single Label Test)

use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use std::collections::BTreeMap;

fn create_p0_bead(id: &str, title: &str, label: &str) -> Issue {
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
        issue_type: IssueType::Task,
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
        labels: vec![label.to_string()], // Single label
        dependencies: vec![],
        comments: vec![],
        annotations: BTreeMap::new(),
    }
}

#[test]
fn test_p0_single_label_creation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let p0_bead = create_p0_bead("bf-p0-single-1", "Critical security bug", "security");
    storage.create_issue(&p0_bead).unwrap();

    let retrieved = storage.get_issue("bf-p0-single-1").unwrap().unwrap();
    assert_eq!(retrieved.id, "bf-p0-single-1");
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(retrieved.status, Status::Open);
    assert_eq!(retrieved.labels.len(), 1); // Exactly one label
    assert_eq!(retrieved.labels[0], "security");
}

#[test]
fn test_p0_single_label_serialization() {
    let p0_bead = create_p0_bead("bf-p0-json-single", "Critical bug for JSON test", "urgent");

    // Serialize to JSON
    let json = serde_json::to_string(&p0_bead).unwrap();

    // Verify JSON structure
    assert!(json.contains("\"priority\":0"));
    assert!(json.contains("\"labels\":[\"urgent\"]")); // Single label in array

    // Deserialize and verify
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.priority.0, 0);
    assert_eq!(deserialized.labels.len(), 1);
    assert_eq!(deserialized.labels[0], "urgent");
}

#[test]
fn test_p0_single_label_add_second_label() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let p0_bead = create_p0_bead("bf-p0-add-label", "Critical bug", "initial");
    storage.create_issue(&p0_bead).unwrap();

    // Verify initial single label
    let initial = storage.get_issue("bf-p0-add-label").unwrap().unwrap();
    assert_eq!(initial.labels.len(), 1);
    assert_eq!(initial.labels[0], "initial");

    // Add a second label
    storage.add_label("bf-p0-add-label", "investigating").unwrap();

    // Verify now has two labels
    let after_add = storage.get_issue("bf-p0-add-label").unwrap().unwrap();
    assert_eq!(after_add.labels.len(), 2);
    assert!(after_add.labels.contains(&"initial".to_string()));
    assert!(after_add.labels.contains(&"investigating".to_string()));
    assert_eq!(after_add.priority, Priority::CRITICAL); // P0 priority preserved
}

#[test]
fn test_p0_single_label_remove_to_empty() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let p0_bead = create_p0_bead("bf-p0-remove-label", "Critical bug", "temporary");
    storage.create_issue(&p0_bead).unwrap();

    // Verify initial single label
    let initial = storage.get_issue("bf-p0-remove-label").unwrap().unwrap();
    assert_eq!(initial.labels.len(), 1);
    assert_eq!(initial.labels[0], "temporary");

    // Remove the only label
    storage.remove_label("bf-p0-remove-label", "temporary").unwrap();

    // Verify now has zero labels
    let after_remove = storage.get_issue("bf-p0-remove-label").unwrap().unwrap();
    assert_eq!(after_remove.labels.len(), 0);
    assert_eq!(after_remove.priority, Priority::CRITICAL); // P0 priority preserved
}

#[test]
fn test_p0_single_label_persistence() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create and persist P0 bead with single label
    {
        let storage = Storage::open(&db_path).unwrap();
        let p0_bead = create_p0_bead("bf-p0-persist-single", "Persistent critical bug", "persistent");
        storage.create_issue(&p0_bead).unwrap();
    }

    // Reopen storage and verify
    let storage = Storage::open(&db_path).unwrap();
    let retrieved = storage.get_issue("bf-p0-persist-single").unwrap().unwrap();

    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(retrieved.labels.len(), 1);
    assert_eq!(retrieved.labels[0], "persistent");
}

#[test]
fn test_p0_single_label_with_update() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let p0_bead = create_p0_bead("bf-p0-update-single", "Critical bug", "before-update");
    storage.create_issue(&p0_bead).unwrap();

    // Update the bead using IssueChanges
    let changes = bead_forge::model::IssueChanges {
        status: Some(bead_forge::model::Status::InProgress),
        assignee: Some("fixer".to_string()),
        actor: Some("test-actor".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-p0-update-single", &changes).unwrap();

    // Verify single label and P0 priority preserved through update
    let updated = storage.get_issue("bf-p0-update-single").unwrap().unwrap();
    assert_eq!(updated.labels.len(), 1);
    assert_eq!(updated.labels[0], "before-update");
    assert_eq!(updated.priority, Priority::CRITICAL);
    assert_eq!(updated.status, Status::InProgress);
    assert_eq!(updated.assignee, Some("fixer".to_string()));
}

#[test]
fn test_multiple_p0_beads_different_single_labels() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create multiple P0 beads each with different single label
    let p0_beads = vec![
        ("bf-p0-single-1", "Security bug", "security"),
        ("bf-p0-single-2", "Performance bug", "performance"),
        ("bf-p0-single-3", "Data loss bug", "data-loss"),
        ("bf-p0-single-4", "UI blocking bug", "ui"),
    ];

    for (id, title, label) in &p0_beads {
        let bead = create_p0_bead(id, title, label);
        storage.create_issue(&bead).unwrap();
    }

    // Verify each P0 bead has exactly one unique label
    for (id, _, expected_label) in &p0_beads {
        let retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(retrieved.priority, Priority::CRITICAL);
        assert_eq!(retrieved.labels.len(), 1);
        assert_eq!(retrieved.labels[0], *expected_label);
    }

    // Verify global label aggregation
    let all_labels = storage.list_all_labels().unwrap();
    assert_eq!(all_labels.len(), 4); // 4 unique labels

    let label_map: std::collections::HashMap<String, i64> = all_labels.into_iter().collect();
    assert_eq!(label_map.get("security"), Some(&1));
    assert_eq!(label_map.get("performance"), Some(&1));
    assert_eq!(label_map.get("data-loss"), Some(&1));
    assert_eq!(label_map.get("ui"), Some(&1));
}

#[test]
fn test_p0_single_label_get_labels() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let p0_bead = create_p0_bead("bf-p0-get-labels", "Critical bug", "retrieval-test");
    storage.create_issue(&p0_bead).unwrap();

    // Get labels specifically for the P0 bead
    let labels = storage.get_labels("bf-p0-get-labels").unwrap();
    assert_eq!(labels.len(), 1);
    assert!(labels.contains(&"retrieval-test".to_string()));
}

#[test]
fn test_p0_single_label_close_reopen() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let p0_bead = create_p0_bead("bf-p0-close-reopen", "Critical bug", "reopen-test");
    storage.create_issue(&p0_bead).unwrap();

    // Close the P0 bead
    let close_changes = bead_forge::model::IssueChanges {
        status: Some(bead_forge::model::Status::Closed),
        actor: Some("fixer".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-p0-close-reopen", &close_changes).unwrap();

    let closed = storage.get_issue("bf-p0-close-reopen").unwrap().unwrap();
    assert_eq!(closed.status, Status::Closed);
    assert_eq!(closed.labels.len(), 1);
    assert_eq!(closed.labels[0], "reopen-test");
    assert_eq!(closed.priority, Priority::CRITICAL);

    // Reopen the P0 bead
    let reopen_changes = bead_forge::model::IssueChanges {
        status: Some(bead_forge::model::Status::Open),
        actor: Some("test-actor".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-p0-close-reopen", &reopen_changes).unwrap();

    let reopened = storage.get_issue("bf-p0-close-reopen").unwrap().unwrap();
    assert_eq!(reopened.status, Status::Open);
    assert_eq!(reopened.labels.len(), 1);
    assert_eq!(reopened.labels[0], "reopen-test");
    assert_eq!(reopened.priority, Priority::CRITICAL);
}

#[test]
fn test_p0_single_label_various_types() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create P0 beads of different types, each with single label
    let bead_types = vec![
        ("bf-p0-task", IssueType::Task, "task-label"),
        ("bf-p0-bug", IssueType::Bug, "bug-label"),
        ("bf-p0-feature", IssueType::Feature, "feature-label"),
        ("bf-p0-chore", IssueType::Chore, "chore-label"),
    ];

    for (id, issue_type, label) in &bead_types {
        let mut bead = create_p0_bead(id, "P0 bead", label);
        bead.issue_type = issue_type.clone();
        storage.create_issue(&bead).unwrap();
    }

    // Verify each P0 bead has its single label and correct type
    for (id, expected_type, expected_label) in bead_types {
        let retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(retrieved.priority, Priority::CRITICAL);
        assert_eq!(retrieved.labels.len(), 1);
        assert_eq!(retrieved.labels[0], expected_label);
        assert_eq!(retrieved.issue_type, expected_type);
    }
}

#[test]
fn test_p0_single_label_special_characters() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Test single labels with special formats
    let special_labels = vec![
        ("bf-p0-special-1", "priority:p0"),
        ("bf-p0-special-2", "severity:critical"),
        ("bf-p0-special-3", "🔥-urgent"),
        ("bf-p0-special-4", "area:auth"),
    ];

    for (id, label) in &special_labels {
        let bead = create_p0_bead(id, "P0 special label", label);
        storage.create_issue(&bead).unwrap();

        let retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(retrieved.priority, Priority::CRITICAL);
        assert_eq!(retrieved.labels.len(), 1);
        assert_eq!(retrieved.labels[0], *label);
    }
}

#[test]
fn test_p0_single_label_roundtrip() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let p0_bead = create_p0_bead("bf-p0-roundtrip", "Roundtrip test", "roundtrip-label");
    storage.create_issue(&p0_bead).unwrap();

    // Retrieve and serialize
    let retrieved = storage.get_issue("bf-p0-roundtrip").unwrap().unwrap();
    let json = serde_json::to_string(&retrieved).unwrap();

    // Deserialize
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    // Verify all fields preserved including single label
    assert_eq!(deserialized.id, "bf-p0-roundtrip");
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.status, Status::Open);
    assert_eq!(deserialized.labels.len(), 1);
    assert_eq!(deserialized.labels[0], "roundtrip-label");
}
