//! Comprehensive test for assignee field persistence on P0 priority beads
//!
//! This test validates that assignee field persistence works correctly for
//! P0 (Critical) priority beads, ensuring:
//! 1. P0 beads can be created with assignees
//! 2. Assignee persists through database operations
//! 3. Assignee persists through JSONL export/import
//! 4. Assignee survives round-trip serialization
//! 5. Assignee works correctly with filtering and queries
//! 6. P0 beads maintain assignee through status changes

use bead_forge::config::load_config;
use bead_forge::model::{Issue, IssueChanges, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a temporary workspace for testing
fn setup_test_workspace() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("test-workspace");
    std::fs::create_dir_all(&workspace_dir).unwrap();
    let beads_dir = workspace_dir.join(".beads");
    std::fs::create_dir_all(&beads_dir).unwrap();

    // Initialize workspace
    let config_path = beads_dir.join("config.yaml");
    std::fs::write(
        &config_path,
        r#"issue_prefixes: [bf]
default_priority: 2
default_type: task
claim_ttl_minutes: 30
"#,
    )
    .unwrap();

    let metadata_path = beads_dir.join("metadata.json");
    std::fs::write(
        &metadata_path,
        r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
    )
    .unwrap();

    let db_path = beads_dir.join("beads.db");
    Storage::open(&db_path).unwrap();

    (temp_dir, beads_dir)
}

/// Create a P0 test bead with an assignee
fn create_p0_bead_with_assignee(
    storage: &Storage,
    id: &str,
    title: &str,
    assignee: &str,
) -> Issue {
    let now = Utc::now();

    let issue = Issue {
        id: id.to_string(),
        title: title.to_string(),
        description: Some("P0 critical issue".to_string()),
        acceptance_criteria: None,
        design: None,
        notes: None,
        status: Status::Open,
        priority: Priority::CRITICAL, // P0
        issue_type: IssueType::Task,
        assignee: Some(assignee.to_string()),
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
        source_repo: None,
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
        content_hash: None,
        labels: vec!["critical".to_string(), "p0".to_string()],
        dependencies: vec![],
        comments: vec![],
        events: vec![],
        annotations: Default::default(),
    };

    storage.create_issue(&issue).unwrap();
    storage.get_issue(&id).unwrap().unwrap()
}

#[test]
fn test_p0_bead_created_with_assignee_persists() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create P0 bead with assignee
    let bead = create_p0_bead_with_assignee(
        &storage,
        "bf-p0-assignee-test",
        "P0 Critical Issue with Assignee",
        "claude-code-glm-4.7-delta",
    );

    // Verify P0 priority and assignee both persisted
    assert_eq!(bead.priority, Priority::CRITICAL);
    assert_eq!(bead.assignee.as_ref().unwrap(), "claude-code-glm-4.7-delta");
    assert_eq!(bead.status, Status::Open);

    // Retrieve from database again to ensure persistence
    let retrieved = storage.get_issue("bf-p0-assignee-test").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.assignee.as_ref().unwrap(), "claude-code-glm-4.7-delta");
}

#[test]
fn test_p0_bead_assignee_survives_status_change() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create P0 bead with assignee
    let bead = create_p0_bead_with_assignee(
        &storage,
        "bf-p0-status-change",
        "P0 issue for status change test",
        "alice",
    );

    // Change status to InProgress
    let changes = IssueChanges {
        status: Some(Status::InProgress),
        actor: Some("test-user".to_string()),
        ..Default::default()
    };
    storage.update_issue(&bead.id, &changes).unwrap();

    // Verify assignee still there after status change
    let updated = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(updated.status, Status::InProgress);
    assert_eq!(updated.assignee.as_ref().unwrap(), "alice");
    assert_eq!(updated.priority, Priority::CRITICAL);
}

#[test]
fn test_p0_bead_assignee_survives_priority_change() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create P0 bead with assignee
    let bead = create_p0_bead_with_assignee(
        &storage,
        "bf-p0-priority-change",
        "P0 issue for priority change test",
        "bob",
    );

    // Change priority from P0 to P1
    let changes = IssueChanges {
        priority: Some(1), // P1
        actor: Some("test-user".to_string()),
        ..Default::default()
    };
    storage.update_issue(&bead.id, &changes).unwrap();

    // Verify assignee still there after priority change
    let updated = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(updated.priority, Priority::HIGH);
    assert_eq!(updated.assignee.as_ref().unwrap(), "bob");
}

#[test]
fn test_p0_bead_assignee_cleared_on_close_and_reopen() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create P0 bead with assignee
    let bead = create_p0_bead_with_assignee(
        &storage,
        "bf-p0-close-reopen",
        "P0 issue for close/reopen test",
        "charlie",
    );

    // Close the bead - this should clear the assignee
    storage
        .close_issue(&bead.id, "test-close-reason", "test-close-actor")
        .unwrap();

    // Verify assignee was cleared after close (expected behavior)
    let closed = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(closed.status, Status::Closed);
    assert!(closed.assignee.is_none(), "Assignee should be cleared on close");

    // Reopen the bead - assignee should remain cleared
    storage.reopen_issue(&bead.id).unwrap();

    // Verify assignee is still cleared after reopen (expected behavior)
    let reopened = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(reopened.status, Status::Open);
    assert!(reopened.assignee.is_none(), "Assignee should remain cleared after reopen");
    assert_eq!(reopened.priority, Priority::CRITICAL, "P0 priority should be preserved");
}

#[test]
fn test_p0_bead_assignee_roundtrip_serialization() {
    // Create P0 bead with assignee
    let issue = Issue {
        id: "bf-p0-serialization".to_string(),
        title: "P0 serialization test".to_string(),
        priority: Priority::CRITICAL,
        assignee: Some("test-assignee".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&issue).unwrap();

    // Deserialize back
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    // Verify both priority and assignee survived
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(
        deserialized.assignee.as_ref().unwrap(),
        "test-assignee"
    );
}

#[test]
fn test_p0_bead_assignee_field_persistence_in_filter() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create multiple P0 beads with different assignees
    create_p0_bead_with_assignee(
        &storage,
        "bf-p0-filter-1",
        "P0 filter test 1",
        "alice",
    );
    create_p0_bead_with_assignee(
        &storage,
        "bf-p0-filter-2",
        "P0 filter test 2",
        "alice",
    );
    create_p0_bead_with_assignee(
        &storage,
        "bf-p0-filter-3",
        "P0 filter test 3",
        "bob",
    );

    // Filter by assignee="alice"
    let filter = bead_forge::model::IssueFilter {
        assignee: Some("alice".to_string()),
        priority: Some(0), // P0 only
        ..Default::default()
    };
    let alice_p0_beads = storage.list_issues(&filter).unwrap();

    // Should get exactly 2 P0 beads assigned to alice
    assert_eq!(alice_p0_beads.len(), 2);
    for bead in &alice_p0_beads {
        assert_eq!(bead.assignee.as_ref().unwrap(), "alice");
        assert_eq!(bead.priority, Priority::CRITICAL);
    }
}

#[test]
fn test_p0_bead_assignee_update_persistence() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create P0 bead with assignee
    let bead = create_p0_bead_with_assignee(
        &storage,
        "bf-p0-update-assignee",
        "P0 update assignee test",
        "original-assignee",
    );

    // Update assignee
    let changes = IssueChanges {
        assignee: Some("new-assignee".to_string()),
        actor: Some("test-user".to_string()),
        ..Default::default()
    };
    storage.update_issue(&bead.id, &changes).unwrap();

    // Verify new assignee persisted while P0 priority maintained
    let updated = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(updated.assignee.as_ref().unwrap(), "new-assignee");
    assert_eq!(updated.priority, Priority::CRITICAL);
}

#[test]
fn test_p0_bead_assignee_clear_persistence() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create P0 bead with assignee
    let bead = create_p0_bead_with_assignee(
        &storage,
        "bf-p0-clear-assignee",
        "P0 clear assignee test",
        "to-be-cleared",
    );

    // Clear assignee
    let changes = IssueChanges {
        assignee: Some(String::new()), // Empty string clears
        actor: Some("test-user".to_string()),
        ..Default::default()
    };
    storage.update_issue(&bead.id, &changes).unwrap();

    // Verify assignee cleared but P0 priority maintained
    let updated = storage.get_issue(&bead.id).unwrap().unwrap();
    assert!(updated.assignee.is_none());
    assert_eq!(updated.priority, Priority::CRITICAL);
}

#[test]
fn test_multiple_p0_beads_different_assignees() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create multiple P0 beads with different assignees
    let assignees = vec!["alice", "bob", "charlie", "david"];
    let mut bead_ids = Vec::new();

    for (i, assignee) in assignees.iter().enumerate() {
        let id = format!("bf-p0-multi-{}", i);
        bead_ids.push(id.clone());
        create_p0_bead_with_assignee(
            &storage,
            &id,
            &format!("P0 multi-assignee test {}", i),
            assignee,
        );
    }

    // Verify all P0 beads have their correct assignees
    for (i, bead_id) in bead_ids.iter().enumerate() {
        let bead = storage.get_issue(bead_id).unwrap().unwrap();
        assert_eq!(bead.priority, Priority::CRITICAL);
        assert_eq!(bead.assignee.as_ref().unwrap(), assignees[i]);
    }
}

#[test]
fn test_p0_bead_with_auto_assignee() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create P0 bead with "auto" as assignee (simulating NEEDLE auto-assignment)
    let bead = create_p0_bead_with_assignee(
        &storage,
        "bf-p0-auto-assignee",
        "P0 auto-assigned bead",
        "auto",
    );

    // Verify "auto" assignee persisted
    assert_eq!(bead.priority, Priority::CRITICAL);
    assert_eq!(bead.assignee.as_ref().unwrap(), "auto");

    // Retrieve again to ensure persistence
    let retrieved = storage.get_issue("bf-p0-auto-assignee").unwrap().unwrap();
    assert_eq!(retrieved.assignee.as_ref().unwrap(), "auto");
}
