// Integration tests for close/reopen lifecycle
//
// Tests the full close and reopen cycle including:
// - Single close-then-reopen cycle
// - Multiple close/reopen cycles
// - Event recording through the cycle
// - Status transitions
// - Field persistence and clearing
// - Dirty tracking
// - Cross-cycle event history

use bead_forge::close;
use bead_forge::model::{Event, EventType, Issue, Status};
use bead_forge::reopen;
use bead_forge::storage::Storage;
use chrono::{Duration, Utc};
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test bead in open state
fn create_open_bead(storage: &Storage, bead_id: &str, title: &str) -> Issue {
    let now = Utc::now();
    let bead = Issue {
        id: bead_id.to_string(),
        title: title.to_string(),
        status: Status::Open,
        created_at: now,
        updated_at: now,
        ..Default::default()
    };

    storage
        .create_issue(&bead)
        .expect("Failed to create test bead");
    bead
}

/// Helper to create a test database with a bead
fn setup_test_db_with_bead() -> (TempDir, PathBuf, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let storage = Storage::open(&db_path).expect("Failed to open storage");

    let bead_id = "bf-test-cycle-1".to_string();
    create_open_bead(&storage, &bead_id, "Test bead for close/reopen cycle");

    (temp_dir, db_path, bead_id)
}

#[test]
fn test_close_then_reopen_cycle() {
    let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

    let storage = Storage::open(&db_path).expect("Failed to open storage");

    // Verify initial state
    let bead = storage
        .get_issue(&bead_id)
        .expect("Failed to get bead")
        .unwrap();
    assert_eq!(bead.status, Status::Open, "Initial status should be Open");
    assert!(bead.closed_at.is_none(), "Initial closed_at should be None");
    assert!(
        bead.close_reason.is_none(),
        "Initial close_reason should be None"
    );

    // Close the bead
    let close_reason = "Implementation complete and verified";
    let actor = "test-actor";
    close::close_bead(&db_path, &bead_id, close_reason, actor).expect("Close should succeed");

    // Verify closed state
    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let bead = storage
        .get_issue(&bead_id)
        .expect("Failed to get bead")
        .unwrap();

    assert_eq!(bead.status, Status::Closed, "Status should be Closed");
    assert!(bead.closed_at.is_some(), "closed_at should be set");
    assert_eq!(
        bead.close_reason.as_deref(),
        Some(close_reason),
        "close_reason should match"
    );
    assert_eq!(
        bead.closed_by_session.as_deref(),
        Some(actor),
        "closed_by_session should match"
    );

    // Reopen the bead
    reopen::reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");

    // Verify reopened state
    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let bead = storage
        .get_issue(&bead_id)
        .expect("Failed to get bead")
        .unwrap();

    assert_eq!(
        bead.status,
        Status::Open,
        "Status should be Open after reopen"
    );
    assert!(bead.closed_at.is_none(), "closed_at should be cleared");
    assert!(
        bead.close_reason.is_none(),
        "close_reason should be cleared"
    );
    assert!(
        bead.closed_by_session.is_none(),
        "closed_by_session should be cleared"
    );
}

#[test]
fn test_close_reopen_creates_correct_events() {
    let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

    let actor = "test-actor";
    let close_reason = "Test close";

    // Close the bead
    close::close_bead(&db_path, &bead_id, close_reason, actor).expect("Close should succeed");

    // Reopen the bead
    reopen::reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");

    // Verify events
    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let events = storage
        .list_events(&bead_id)
        .expect("Failed to list events");

    // Should have 3 events: created, closed, and reopened
    assert_eq!(
        events.len(),
        3,
        "Should have exactly 3 events (created + closed + reopened)"
    );

    let closed_event = events
        .iter()
        .find(|e| e.event_type == EventType::Closed)
        .expect("Should have a Closed event");

    let reopened_event = events
        .iter()
        .find(|e| e.event_type == EventType::Reopened)
        .expect("Should have a Reopened event");

    // Verify closed event
    assert_eq!(
        closed_event.issue_id, bead_id,
        "Closed event should have correct issue_id"
    );
    assert_eq!(
        closed_event.actor, actor,
        "Closed event should have correct actor"
    );
    assert_eq!(
        closed_event.new_value.as_deref(),
        Some(close_reason),
        "Closed event should have correct new_value (reason)"
    );
    assert!(
        closed_event.old_value.is_none(),
        "Closed event should have no old_value"
    );

    // Verify reopened event
    assert_eq!(
        reopened_event.issue_id, bead_id,
        "Reopened event should have correct issue_id"
    );
    assert_eq!(
        reopened_event.new_value.as_deref(),
        Some("open"),
        "Reopened event should show new status as 'open'"
    );
    assert_eq!(
        reopened_event.old_value.as_deref(),
        Some("closed"),
        "Reopened event should show old status as 'closed'"
    );
}

#[test]
fn test_multiple_close_reopen_cycles() {
    let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

    let actors = vec!["actor-1", "actor-2", "actor-3"];
    let reasons = vec![
        "First close reason",
        "Second close reason",
        "Third close reason",
    ];

    // Perform three close/reopen cycles
    for i in 0..3 {
        // Close
        close::close_bead(&db_path, &bead_id, reasons[i], actors[i]).expect("Close should succeed");

        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let bead = storage
            .get_issue(&bead_id)
            .expect("Failed to get bead")
            .unwrap();
        assert_eq!(
            bead.status,
            Status::Closed,
            "Should be closed after close {}",
            i + 1
        );
        assert_eq!(
            bead.close_reason.as_deref(),
            Some(reasons[i]),
            "Close reason should match for cycle {}",
            i + 1
        );

        // Reopen
        reopen::reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");

        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let bead = storage
            .get_issue(&bead_id)
            .expect("Failed to get bead")
            .unwrap();
        assert_eq!(
            bead.status,
            Status::Open,
            "Should be open after reopen {}",
            i + 1
        );
    }

    // Verify final state is open
    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let bead = storage
        .get_issue(&bead_id)
        .expect("Failed to get bead")
        .unwrap();
    assert_eq!(bead.status, Status::Open, "Final status should be Open");
    assert!(bead.closed_at.is_none(), "Final closed_at should be None");
    assert!(
        bead.close_reason.is_none(),
        "Final close_reason should be None"
    );
}

#[test]
fn test_event_history_across_multiple_cycles() {
    let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

    let actors = vec!["actor-1", "actor-2"];
    let reasons = vec!["First close", "Second close"];

    // Perform two close/reopen cycles
    for i in 0..2 {
        close::close_bead(&db_path, &bead_id, reasons[i], actors[i]).expect("Close should succeed");
        reopen::reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");
    }

    // Verify event history
    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let events = storage
        .list_events(&bead_id)
        .expect("Failed to list events");

    // Should have 5 events: created + 2 closed + 2 reopened
    assert_eq!(
        events.len(),
        5,
        "Should have 5 events (created + 2 cycles x 2 operations)"
    );

    // Verify order and types (skip the first "created" event)
    assert_eq!(
        events[1].event_type,
        EventType::Closed,
        "Second event should be Closed"
    );
    assert_eq!(
        events[2].event_type,
        EventType::Reopened,
        "Third event should be Reopened"
    );
    assert_eq!(
        events[3].event_type,
        EventType::Closed,
        "Fourth event should be Closed"
    );
    assert_eq!(
        events[4].event_type,
        EventType::Reopened,
        "Fifth event should be Reopened"
    );

    // Verify each closed event has correct data
    let closed_events: Vec<&Event> = events
        .iter()
        .filter(|e| e.event_type == EventType::Closed)
        .collect();

    assert_eq!(closed_events.len(), 2, "Should have 2 closed events");
    assert_eq!(
        closed_events[0].new_value.as_deref(),
        Some(reasons[0]),
        "First closed event should have first reason"
    );
    assert_eq!(
        closed_events[1].new_value.as_deref(),
        Some(reasons[1]),
        "Second closed event should have second reason"
    );
}

#[test]
fn test_close_reopen_preserves_non_close_fields() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let storage = Storage::open(&db_path).expect("Failed to open storage");

    // Create a bead with various fields set
    let bead_id = "bf-test-preserve-1".to_string();
    let now = Utc::now();
    let bead = Issue {
        id: bead_id.clone(),
        title: "Test bead with many fields".to_string(),
        description: Some("Original description".to_string()),
        priority: bead_forge::model::Priority::HIGH,
        issue_type: bead_forge::model::IssueType::Feature,
        assignee: Some("test-assignee".to_string()),
        created_at: now,
        updated_at: now,
        ..Default::default()
    };

    storage
        .create_issue(&bead)
        .expect("Failed to create test bead");

    // Close and reopen
    close::close_bead(&db_path, &bead_id, "Test", "test-actor").expect("Close should succeed");
    reopen::reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");

    // Verify non-close fields are preserved
    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let bead_after = storage
        .get_issue(&bead_id)
        .expect("Failed to get bead")
        .unwrap();

    assert_eq!(bead_after.title, bead.title, "Title should be preserved");
    assert_eq!(
        bead_after.description, bead.description,
        "Description should be preserved"
    );
    assert_eq!(
        bead_after.priority, bead.priority,
        "Priority should be preserved"
    );
    assert_eq!(
        bead_after.issue_type, bead.issue_type,
        "Issue type should be preserved"
    );
    assert_eq!(
        bead_after.created_at, bead.created_at,
        "Created_at should be preserved"
    );

    // Note: assignee should be cleared after reopen
    assert!(
        bead_after.assignee.is_none(),
        "Assignee should be cleared after reopen"
    );
}

#[test]
fn test_close_reopen_marks_dirty_both_times() {
    let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

    // Note: bead creation marks it as dirty, so we expect it to be dirty initially
    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let dirty_before = storage.list_dirty_issues().expect("Failed to list dirty");
    assert!(
        dirty_before.iter().any(|b| b.id == bead_id),
        "Bead should be dirty initially (from creation)"
    );

    // Close should keep it marked as dirty
    close::close_bead(&db_path, &bead_id, "Test", "test-actor").expect("Close should succeed");

    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let dirty_after_close = storage.list_dirty_issues().expect("Failed to list dirty");
    assert!(
        dirty_after_close.iter().any(|b| b.id == bead_id),
        "Bead should still be dirty after close"
    );

    // Reopen should also keep it marked as dirty
    reopen::reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");

    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let dirty_after_reopen = storage.list_dirty_issues().expect("Failed to list dirty");
    assert!(
        dirty_after_reopen.iter().any(|b| b.id == bead_id),
        "Bead should still be dirty after reopen"
    );
}

#[test]
fn test_close_reopen_updates_timestamps() {
    let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let bead_initial = storage
        .get_issue(&bead_id)
        .expect("Failed to get bead")
        .unwrap();
    let updated_at_initial = bead_initial.updated_at;

    // Wait to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Close
    close::close_bead(&db_path, &bead_id, "Test", "test-actor").expect("Close should succeed");

    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let bead_after_close = storage
        .get_issue(&bead_id)
        .expect("Failed to get bead")
        .unwrap();

    assert!(
        bead_after_close.updated_at > updated_at_initial,
        "updated_at should increase after close"
    );

    // Wait again
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Reopen
    reopen::reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");

    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let bead_after_reopen = storage
        .get_issue(&bead_id)
        .expect("Failed to get bead")
        .unwrap();

    assert!(
        bead_after_reopen.updated_at > bead_after_close.updated_at,
        "updated_at should increase after reopen"
    );
}

#[test]
fn test_close_with_assignee_then_reopen_clears_assignee() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let storage = Storage::open(&db_path).expect("Failed to open storage");

    // Create a bead with an assignee
    let bead_id = "bf-test-assignee-1".to_string();
    let now = Utc::now();
    let bead = Issue {
        id: bead_id.clone(),
        title: "Test bead with assignee".to_string(),
        status: Status::Open,
        assignee: Some("worker-123".to_string()),
        created_at: now,
        updated_at: now,
        ..Default::default()
    };

    storage
        .create_issue(&bead)
        .expect("Failed to create test bead");

    // Close
    close::close_bead(&db_path, &bead_id, "Test", "test-actor").expect("Close should succeed");

    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let bead_after_close = storage
        .get_issue(&bead_id)
        .expect("Failed to get bead")
        .unwrap();

    // Assignee should still be set after close
    assert_eq!(
        bead_after_close.assignee.as_deref(),
        Some("worker-123"),
        "Assignee should be preserved after close"
    );

    // Reopen
    reopen::reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");

    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let bead_after_reopen = storage
        .get_issue(&bead_id)
        .expect("Failed to get bead")
        .unwrap();

    // Assignee should be cleared after reopen
    assert!(
        bead_after_reopen.assignee.is_none(),
        "Assignee should be cleared after reopen"
    );
}

#[test]
fn test_reopen_fails_on_non_closed_status() {
    let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

    // Try to reopen an open bead
    let result = reopen::reopen_bead(&db_path, &bead_id);
    assert!(result.is_err(), "Reopening an open bead should fail");

    // Close the bead
    close::close_bead(&db_path, &bead_id, "Test", "test-actor").expect("Close should succeed");

    // Reopen should succeed
    let result = reopen::reopen_bead(&db_path, &bead_id);
    assert!(result.is_ok(), "Reopening a closed bead should succeed");

    // Try to reopen again (should fail - it's now open)
    let result = reopen::reopen_bead(&db_path, &bead_id);
    assert!(
        result.is_err(),
        "Reopening an already-open bead should fail"
    );
}

#[test]
fn test_close_reopen_database_persistence() {
    let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

    // Close
    close::close_bead(&db_path, &bead_id, "Test", "test-actor").expect("Close should succeed");

    // Reopen
    reopen::reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");

    // Close and reopen the database connection to verify persistence
    let storage2 = Storage::open(&db_path).expect("Failed to reopen storage");
    let bead = storage2
        .get_issue(&bead_id)
        .expect("Failed to get bead")
        .unwrap();

    assert_eq!(
        bead.status,
        Status::Open,
        "Status should persist as Open after database reopen"
    );
    assert!(
        bead.closed_at.is_none(),
        "closed_at should persist as None after database reopen"
    );
    assert!(
        bead.close_reason.is_none(),
        "close_reason should persist as None after database reopen"
    );

    // Verify events also persist (created + closed + reopened)
    let events = storage2
        .list_events(&bead_id)
        .expect("Failed to list events");
    assert_eq!(
        events.len(),
        3,
        "All 3 events should persist (created + closed + reopened)"
    );
}

#[test]
fn test_close_reopen_cycle_empty_reason() {
    let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

    // Close with empty reason
    close::close_bead(&db_path, &bead_id, "", "test-actor").expect("Close should succeed");

    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let bead_after_close = storage
        .get_issue(&bead_id)
        .expect("Failed to get bead")
        .unwrap();

    assert_eq!(
        bead_after_close.status,
        Status::Closed,
        "Should be closed with empty reason"
    );
    assert_eq!(
        bead_after_close.close_reason.as_deref(),
        Some(""),
        "Empty reason should be stored"
    );

    // Reopen
    reopen::reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");

    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let bead_after_reopen = storage
        .get_issue(&bead_id)
        .expect("Failed to get bead")
        .unwrap();

    assert_eq!(
        bead_after_reopen.status,
        Status::Open,
        "Should be open after reopen"
    );
    assert!(
        bead_after_reopen.close_reason.is_none(),
        "Empty close_reason should be cleared"
    );
}

#[test]
fn test_close_reopen_cycle_with_special_characters() {
    let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

    let special_reason = "Closed with \"quotes\", 'apostrophes', & symbols <>, and \\backslashes";

    // Close with special characters
    close::close_bead(&db_path, &bead_id, special_reason, "test-actor")
        .expect("Close should succeed");

    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let bead_after_close = storage
        .get_issue(&bead_id)
        .expect("Failed to get bead")
        .unwrap();

    assert_eq!(
        bead_after_close.close_reason.as_deref(),
        Some(special_reason),
        "Special characters should be preserved in close_reason"
    );

    // Reopen
    reopen::reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");

    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let bead_after_reopen = storage
        .get_issue(&bead_id)
        .expect("Failed to get bead")
        .unwrap();

    assert_eq!(
        bead_after_reopen.status,
        Status::Open,
        "Should be open after reopen"
    );
    assert!(
        bead_after_reopen.close_reason.is_none(),
        "Special character close_reason should be cleared"
    );
}

#[test]
fn test_close_timestamp_accuracy_through_cycle() {
    let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

    let before_close = Utc::now();

    // Close
    close::close_bead(&db_path, &bead_id, "Test", "test-actor").expect("Close should succeed");

    let after_close = Utc::now();

    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let bead_after_close = storage
        .get_issue(&bead_id)
        .expect("Failed to get bead")
        .unwrap();

    // Verify closed_at timestamp is accurate
    let closed_at = bead_after_close.closed_at.expect("closed_at should be set");
    assert!(
        closed_at >= before_close && closed_at <= after_close,
        "closed_at should be between before and after close timestamps"
    );

    // Reopen
    reopen::reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");

    let storage = Storage::open(&db_path).expect("Failed to open storage");
    let bead_after_reopen = storage
        .get_issue(&bead_id)
        .expect("Failed to get bead")
        .unwrap();

    // Verify closed_at is cleared
    assert!(
        bead_after_reopen.closed_at.is_none(),
        "closed_at should be cleared after reopen"
    );
}
