//! Comprehensive tests for bead close and reopen operations
//!
//! This test file validates that:
//! 1. Beads can be closed with a reason
//! 2. Close events are created properly
//! 3. Beads can be reopened
//! 4. Reopen events are created properly
//! 5. Closed fields are cleared on reopen
//! 6. Status transitions are tracked correctly

use bead_forge::config::load_config;
use bead_forge::model::{EventType, Issue, IssueChanges, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;

// Global counter for unique test IDs
static TEST_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);

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

/// Create a test bead
fn create_test_bead(storage: &Storage, title: &str) -> Issue {
    let id = format!("bf-test-{}", TEST_ID_COUNTER.fetch_add(1, Ordering::SeqCst));
    let now = Utc::now();

    let issue = Issue {
        id: id.clone(),
        title: title.to_string(),
        description: Some("Test description".to_string()),
        acceptance_criteria: None,
        design: None,
        notes: None,
        status: Status::Open,
        priority: Priority(2),
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
        labels: vec![],
        dependencies: vec![],
        comments: vec![],
        annotations: Default::default(),
    };

    storage.create_issue(&issue).unwrap();
    storage.get_issue(&id).unwrap().unwrap()
}

#[test]
fn test_close_bead_creates_close_event() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create a test bead
    let bead = create_test_bead(&storage, "Test bead for close");

    // Close the bead
    let close_reason = "Task completed successfully";
    storage
        .close_issue(&bead.id, close_reason, "test-user")
        .unwrap();

    // Verify the bead is closed
    let closed_bead = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(closed_bead.status, Status::Closed);
    assert_eq!(closed_bead.close_reason.as_ref().unwrap(), close_reason);
    assert!(closed_bead.closed_at.is_some());

    // Verify a close event was created
    let events = storage.list_events(&bead.id).unwrap();
    assert_eq!(events.len(), 2); // Created + Closed events

    let close_event = events
        .iter()
        .find(|e| matches!(e.event_type, EventType::Closed))
        .unwrap();
    assert_eq!(close_event.actor, "test-user");
    assert_eq!(close_event.new_value.as_ref().unwrap(), close_reason);
    assert!(close_event.old_value.is_none()); // Close events have no old_value
}

#[test]
fn test_reopen_bead_changes_status_to_open() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create and close a bead
    let bead = create_test_bead(&storage, "Test bead for reopen");
    storage
        .close_issue(&bead.id, "Initial close", "test-user")
        .unwrap();

    // Reopen the bead
    use bead_forge::model::IssueChanges;
    let changes = IssueChanges {
        status: Some(Status::Open),
        actor: Some("test-user".to_string()),
        ..Default::default()
    };
    storage.update_issue(&bead.id, &changes).unwrap();

    // Verify the bead is open again
    let reopened_bead = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(reopened_bead.status, Status::Open);

    // Verify closed fields are cleared
    assert!(reopened_bead.closed_at.is_none());
    assert!(reopened_bead.close_reason.is_none());
    assert!(reopened_bead.closed_by_session.is_none());
}

#[test]
fn test_close_then_reopen_creates_reopened_event() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create and close a bead
    let bead = create_test_bead(&storage, "Test bead for reopen event");
    storage
        .close_issue(&bead.id, "Initial close", "test-user")
        .unwrap();

    // Reopen the bead
    use bead_forge::model::IssueChanges;
    let changes = IssueChanges {
        status: Some(Status::Open),
        actor: Some("test-user".to_string()),
        ..Default::default()
    };
    storage.update_issue(&bead.id, &changes).unwrap();

    // Verify events: Created -> Closed -> (Reopened expected but currently missing)
    let events = storage.list_events(&bead.id).unwrap();
    println!("Events after reopen: {:?}", events);

    // Count events by type
    let created_events = events
        .iter()
        .filter(|e| matches!(e.event_type, EventType::Created))
        .count();
    let closed_events = events
        .iter()
        .filter(|e| matches!(e.event_type, EventType::Closed))
        .count();
    let reopened_events = events
        .iter()
        .filter(|e| matches!(e.event_type, EventType::Reopened))
        .count();

    assert_eq!(created_events, 1, "Should have 1 Created event");
    assert_eq!(closed_events, 1, "Should have 1 Closed event");
    assert_eq!(
        reopened_events, 1,
        "Should have 1 Reopened event after reopening"
    );

    // Verify the reopened event has the correct actor
    let reopened_event = events
        .iter()
        .find(|e| matches!(e.event_type, EventType::Reopened))
        .unwrap();
    assert_eq!(reopened_event.actor, "test-user");
    assert_eq!(reopened_event.old_value.as_ref().unwrap(), "closed");
    assert_eq!(reopened_event.new_value.as_ref().unwrap(), "open");
}

#[test]
fn test_multiple_close_reopen_cycles() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create a bead
    let bead = create_test_bead(&storage, "Test bead for multiple cycles");

    // Close -> Reopen -> Close -> Reopen
    storage
        .close_issue(&bead.id, "First close", "user1")
        .unwrap();

    use bead_forge::model::IssueChanges;
    let changes1 = IssueChanges {
        status: Some(Status::Open),
        actor: Some("user1".to_string()),
        ..Default::default()
    };
    storage.update_issue(&bead.id, &changes1).unwrap();

    storage
        .close_issue(&bead.id, "Second close", "user2")
        .unwrap();
    let changes2 = IssueChanges {
        status: Some(Status::Open),
        actor: Some("user2".to_string()),
        ..Default::default()
    };
    storage.update_issue(&bead.id, &changes2).unwrap();

    // Verify final state is open
    let final_bead = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(final_bead.status, Status::Open);
    assert!(final_bead.closed_at.is_none());
    assert!(final_bead.close_reason.is_none());

    // Verify events
    let events = storage.list_events(&bead.id).unwrap();
    println!("Events after multiple cycles: {:?}", events);

    let created_events = events
        .iter()
        .filter(|e| matches!(e.event_type, EventType::Created))
        .count();
    let closed_events = events
        .iter()
        .filter(|e| matches!(e.event_type, EventType::Closed))
        .count();

    assert_eq!(created_events, 1, "Should have 1 Created event");
    assert_eq!(closed_events, 2, "Should have 2 Closed events");
}

#[test]
fn test_close_reason_preserved_in_event() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open(&db_path).unwrap();

    let bead = create_test_bead(&storage, "Test bead for close reason");

    let close_reason =
        "This is a detailed close reason with context: task completed, tested, and verified";
    storage
        .close_issue(&bead.id, close_reason, "tester")
        .unwrap();

    let events = storage.list_events(&bead.id).unwrap();
    let close_event = events
        .iter()
        .find(|e| matches!(e.event_type, EventType::Closed))
        .unwrap();

    assert_eq!(close_event.new_value.as_ref().unwrap(), close_reason);
}

#[test]
fn test_close_preserves_other_fields() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create a bead with various fields
    let id = format!("bf-test-{}", TEST_ID_COUNTER.fetch_add(1, Ordering::SeqCst));
    let now = Utc::now();

    let issue = Issue {
        id: id.clone(),
        title: "Test bead preservation".to_string(),
        description: Some("Original description".to_string()),
        acceptance_criteria: Some("Original AC".to_string()),
        design: Some("Original design".to_string()),
        notes: Some("Original notes".to_string()),
        status: Status::Open,
        priority: Priority(3),
        issue_type: IssueType::Bug,
        assignee: Some("assignee1".to_string()),
        owner: Some("owner1".to_string()),
        estimated_minutes: Some(120),
        created_at: now,
        created_by: Some("creator".to_string()),
        updated_at: now,
        closed_at: None,
        close_reason: None,
        closed_by_session: None,
        due_at: Some(now + chrono::Duration::days(7)),
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
        labels: vec![],
        dependencies: vec![],
        comments: vec![],
        annotations: Default::default(),
    };

    storage.create_issue(&issue).unwrap();
    storage.close_issue(&id, "Fixed", "fixer").unwrap();

    let closed_bead = storage.get_issue(&id).unwrap().unwrap();
    assert_eq!(closed_bead.title, "Test bead preservation");
    assert_eq!(
        closed_bead.description.as_ref().unwrap(),
        "Original description"
    );
    assert_eq!(
        closed_bead.acceptance_criteria.as_ref().unwrap(),
        "Original AC"
    );
    assert_eq!(closed_bead.design.as_ref().unwrap(), "Original design");
    assert_eq!(closed_bead.notes.as_ref().unwrap(), "Original notes");
    assert_eq!(closed_bead.priority, Priority(3));
    assert_eq!(closed_bead.issue_type, IssueType::Bug);
    assert_eq!(closed_bead.assignee.as_ref().unwrap(), "assignee1");
    assert_eq!(closed_bead.owner.as_ref().unwrap(), "owner1");
    assert_eq!(closed_bead.estimated_minutes, Some(120));
}

#[test]
fn test_reopen_clears_assignee() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create a bead with an assignee
    let id = format!("bf-test-{}", TEST_ID_COUNTER.fetch_add(1, Ordering::SeqCst));
    let now = Utc::now();

    let issue = Issue {
        id: id.clone(),
        title: "Test bead assignee clearing".to_string(),
        description: Some("Test description".to_string()),
        acceptance_criteria: None,
        design: None,
        notes: None,
        status: Status::Open,
        priority: Priority(2),
        issue_type: IssueType::Task,
        assignee: Some("worker-1".to_string()), // Bead has an assignee
        owner: None,
        estimated_minutes: None,
        created_at: now,
        created_by: Some("creator".to_string()),
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
        labels: vec![],
        dependencies: vec![],
        comments: vec![],
        annotations: Default::default(),
    };

    storage.create_issue(&issue).unwrap();

    // Close the bead
    storage.close_issue(&id, "Completed", "closer").unwrap();

    // Verify assignee is preserved on close
    let closed_bead = storage.get_issue(&id).unwrap().unwrap();
    assert_eq!(closed_bead.status, Status::Closed);
    assert_eq!(
        closed_bead.assignee.as_ref().unwrap(),
        "worker-1",
        "Assignee should be preserved when bead is closed"
    );

    // Reopen the bead with assignee clearing (mimics cmd_reopen behavior)
    use bead_forge::model::IssueChanges;
    let changes = IssueChanges {
        status: Some(Status::Open),
        assignee: Some(String::new()), // Empty string clears to NULL
        actor: Some("reopener".to_string()),
        ..Default::default()
    };
    storage.update_issue(&id, &changes).unwrap();

    // Verify the bead is open and assignee is cleared
    let reopened_bead = storage.get_issue(&id).unwrap().unwrap();
    assert_eq!(
        reopened_bead.status,
        Status::Open,
        "Bead should be open after reopen"
    );
    assert!(
        reopened_bead.assignee.is_none(),
        "Assignee should be cleared after reopen (should be NULL, not empty string)"
    );
}

#[test]
fn test_reopen_with_no_assignee_is_noop() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create a bead without an assignee
    let bead = create_test_bead(&storage, "Test bead with no assignee");
    assert!(bead.assignee.is_none());

    // Close the bead
    storage
        .close_issue(&bead.id, "Completed", "closer")
        .unwrap();

    // Reopen the bead with assignee clearing
    use bead_forge::model::IssueChanges;
    let changes = IssueChanges {
        status: Some(Status::Open),
        assignee: Some(String::new()), // Empty string clears to NULL
        actor: Some("reopener".to_string()),
        ..Default::default()
    };
    storage.update_issue(&bead.id, &changes).unwrap();

    // Verify the bead is open and still has no assignee
    let reopened_bead = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(reopened_bead.status, Status::Open);
    assert!(
        reopened_bead.assignee.is_none(),
        "Bead with no assignee should still have no assignee after reopen"
    );
}
