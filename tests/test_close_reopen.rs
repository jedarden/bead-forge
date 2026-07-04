//! Test close and reopen functionality

use bead_forge::storage::Storage;
use bead_forge::model::{Issue, Status, Priority, IssueType};
use std::path::PathBuf;
use tempfile::TempDir;
use chrono::Utc;

fn setup_test_storage() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Initialize the database
    let _storage = Storage::open(&db_path).unwrap();

    (temp_dir, db_path)
}

#[test]
fn test_close_and_reopen_bead() {
    let (_temp_dir, db_path) = setup_test_storage();
    let storage = Storage::open(&db_path).unwrap();

    // Create a test bead
    let bead = Issue {
        id: "test-close-1".to_string(),
        title: "Test Close and Reopen".to_string(),
        status: Status::Open,
        priority: Priority::MEDIUM,
        issue_type: IssueType::Task,
        description: Some("Test description".to_string()),
        acceptance_criteria: None,
        notes: None,
        design: None,
        assignee: None,
        owner: None,
        estimated_minutes: None,
        created_at: Utc::now(),
        created_by: None,
        updated_at: Utc::now(),
        due_at: None,
        closed_at: None,
        close_reason: None,
        closed_by_session: None,
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
        labels: Vec::new(),
        dependencies: Vec::new(),
        comments: Vec::new(),
        annotations: Default::default(),
    };

    // Insert the bead
    storage.create_issue(&bead).unwrap();

    // Verify it's open
    let loaded = storage.get_issue("test-close-1").unwrap().unwrap();
    assert_eq!(loaded.status, Status::Open);
    assert!(loaded.closed_at.is_none());
    assert!(loaded.close_reason.is_none());

    // Close the bead
    let close_reason = "Test completed successfully";
    storage.close_issue("test-close-1", close_reason, "test_actor").unwrap();

    // Verify it's closed
    let closed = storage.get_issue("test-close-1").unwrap().unwrap();
    assert_eq!(closed.status, Status::Closed);
    assert!(closed.closed_at.is_some());
    assert_eq!(closed.close_reason, Some(close_reason.to_string()));

    // Check that the event was recorded
    let events = storage.list_events("test-close-1").unwrap();
    assert!(events.iter().any(|e| e.event_type == bead_forge::model::EventType::Closed));

    // Reopen the bead
    let changes = bead_forge::model::IssueChanges {
        status: Some(Status::Open),
        ..Default::default()
    };
    storage.update_issue("test-close-1", &changes).unwrap();

    // Verify it's open again
    let reopened = storage.get_issue("test-close-1").unwrap().unwrap();
    assert_eq!(reopened.status, Status::Open);
    // closed_at and close_reason should remain for historical record
}

#[test]
fn test_close_already_closed_bead() {
    let (_temp_dir, db_path) = setup_test_storage();
    let storage = Storage::open(&db_path).unwrap();

    let bead = Issue {
        id: "test-close-2".to_string(),
        title: "Test Double Close".to_string(),
        status: Status::Open,
        priority: Priority::MEDIUM,
        issue_type: IssueType::Task,
        description: Some("Test description".to_string()),
        acceptance_criteria: None,
        notes: None,
        design: None,
        assignee: None,
        owner: None,
        estimated_minutes: None,
        created_at: Utc::now(),
        created_by: None,
        updated_at: Utc::now(),
        due_at: None,
        closed_at: None,
        close_reason: None,
        closed_by_session: None,
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
        labels: Vec::new(),
        dependencies: Vec::new(),
        comments: Vec::new(),
        annotations: Default::default(),
    };

    storage.create_issue(&bead).unwrap();
    storage.close_issue("test-close-2", "First close", "test_actor").unwrap();

    // Try to close again - this should succeed (idempotent)
    let result = storage.close_issue("test-close-2", "Second close", "test_actor");
    assert!(result.is_ok());
}

#[test]
fn test_reopen_in_progress_bead() {
    let (_temp_dir, db_path) = setup_test_storage();
    let storage = Storage::open(&db_path).unwrap();

    let bead = Issue {
        id: "test-reopen-1".to_string(),
        title: "Test Reopen In Progress".to_string(),
        status: Status::InProgress,
        priority: Priority::MEDIUM,
        issue_type: IssueType::Task,
        description: Some("Test description".to_string()),
        acceptance_criteria: None,
        notes: None,
        design: None,
        assignee: None,
        owner: None,
        estimated_minutes: None,
        created_at: Utc::now(),
        created_by: None,
        updated_at: Utc::now(),
        due_at: None,
        closed_at: None,
        close_reason: None,
        closed_by_session: None,
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
        labels: Vec::new(),
        dependencies: Vec::new(),
        comments: Vec::new(),
        annotations: Default::default(),
    };

    storage.create_issue(&bead).unwrap();

    // Close from in_progress
    storage.close_issue("test-reopen-1", "Completed", "test_actor").unwrap();

    // Reopen back to in_progress
    let changes = bead_forge::model::IssueChanges {
        status: Some(Status::InProgress),
        ..Default::default()
    };
    storage.update_issue("test-reopen-1", &changes).unwrap();

    let reopened = storage.get_issue("test-reopen-1").unwrap().unwrap();
    assert_eq!(reopened.status, Status::InProgress);
}

#[test]
fn test_close_nonexistent_bead() {
    let (_temp_dir, db_path) = setup_test_storage();
    let storage = Storage::open(&db_path).unwrap();

    // Try to close a bead that doesn't exist
    let result = storage.close_issue("nonexistent", "reason", "test_actor");
    assert!(result.is_err());
}

#[test]
fn test_reopen_nonexistent_bead() {
    let (_temp_dir, db_path) = setup_test_storage();
    let storage = Storage::open(&db_path).unwrap();

    // Try to reopen a bead that doesn't exist
    let changes = bead_forge::model::IssueChanges {
        status: Some(Status::Open),
        ..Default::default()
    };
    let result = storage.update_issue("nonexistent", &changes);
    assert!(result.is_err());
}
