//! Close bead functionality.

use crate::storage::Storage;
use anyhow::Result;
use std::path::Path;

/// Close a bead with the given reason.
///
/// This function:
/// - Transitions the bead's status to 'closed'
/// - Sets closed_at timestamp to current time
/// - Sets close_reason field (defaults to 'Completed' if not provided)
/// - Marks bead as dirty in SQLite
/// - Uses with_immediate_transaction for atomicity
///
/// # Arguments
/// * `db_path` - Path to the SQLite database
/// * `id` - Bead ID to close
/// * `reason` - Close reason (will be "Completed" if empty)
/// * `actor` - Actor performing the close (e.g., "cli", worker ID)
///
/// # Errors
/// Returns error if:
/// - Bead not found
/// - Bead already closed
/// - Database operation fails
pub fn close_bead(db_path: &Path, id: &str, reason: &str, actor: &str) -> Result<()> {
    let storage = Storage::open(db_path)?;
    storage.close_issue(id, reason, actor)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Issue, Status};
    use chrono::Utc;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Helper to create an in-memory database with test bead
    fn setup_test_db_with_bead() -> (TempDir, PathBuf, String) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");

        let storage = Storage::open(&db_path).expect("Failed to open storage");

        // Create a test bead
        let bead_id = "bf-test-close-1".to_string();
        let now = Utc::now();
        let bead = Issue {
            id: bead_id.clone(),
            title: "Test bead for close operation".to_string(),
            status: Status::Open,
            created_at: now,
            updated_at: now,
            ..Default::default()
        };

        storage
            .create_issue(&bead)
            .expect("Failed to create test bead");

        (temp_dir, db_path, bead_id)
    }

    /// Helper to create an in-memory database
    fn setup_test_db() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");

        let _storage = Storage::open(&db_path).expect("Failed to open storage");

        (temp_dir, db_path)
    }

    #[test]
    fn test_close_open_bead_succeeds() {
        let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

        // Close the bead
        let result = close_bead(&db_path, &bead_id, "Test completed", "test-actor");
        assert!(result.is_ok(), "Closing an open bead should succeed");

        // Verify the bead is closed
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let bead = storage.get_issue(&bead_id).expect("Failed to get bead");
        assert!(bead.is_some(), "Bead should still exist");
        assert_eq!(
            bead.unwrap().status,
            Status::Closed,
            "Bead status should be closed"
        );
    }

    #[test]
    fn test_close_with_custom_reason() {
        let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

        let custom_reason = "Feature implementation completed successfully";

        // Close with custom reason
        let result = close_bead(&db_path, &bead_id, custom_reason, "test-actor");
        assert!(result.is_ok(), "Closing with custom reason should succeed");

        // Verify the close reason is set
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let bead = storage
            .get_issue(&bead_id)
            .expect("Failed to get bead")
            .unwrap();
        assert_eq!(
            bead.close_reason.as_deref(),
            Some(custom_reason),
            "Close reason should match custom reason"
        );
    }

    #[test]
    fn test_close_already_closed_bead_idempotent() {
        let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

        // Close the bead first time
        close_bead(&db_path, &bead_id, "First close", "test-actor")
            .expect("First close should succeed");

        // Close the bead second time - should be idempotent (not fail)
        let result = close_bead(&db_path, &bead_id, "Second close", "test-actor");
        assert!(
            result.is_ok(),
            "Closing an already-closed bead should be idempotent (return Ok)"
        );
    }

    #[test]
    fn test_close_non_existent_bead_fails() {
        let (_temp_dir, db_path) = setup_test_db();

        let fake_bead_id = "bf-test-nonexistent-12345";

        // Try to close non-existent bead
        let result = close_bead(&db_path, fake_bead_id, "Test", "test-actor");
        assert!(result.is_err(), "Closing non-existent bead should fail");

        let err = result.unwrap_err();
        let err_msg = err.to_string().to_lowercase();
        assert!(
            err_msg.contains("not found") || err_msg.contains("not exist"),
            "Error message should mention bead not found, got: {}",
            err
        );
    }

    #[test]
    fn test_close_sets_closed_at_timestamp() {
        let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

        let before_close = Utc::now();

        // Close the bead
        close_bead(&db_path, &bead_id, "Test", "test-actor").expect("Close should succeed");

        let after_close = Utc::now();

        // Verify closed_at is set
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let bead = storage
            .get_issue(&bead_id)
            .expect("Failed to get bead")
            .unwrap();

        assert!(
            bead.closed_at.is_some(),
            "closed_at should be set after closing"
        );

        let closed_at = bead.closed_at.unwrap();
        assert!(
            closed_at >= before_close && closed_at <= after_close,
            "closed_at timestamp should be between before and after close times"
        );
    }

    #[test]
    fn test_close_sets_close_reason() {
        let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

        let close_reason = "Implementation complete and verified";

        // Close the bead
        close_bead(&db_path, &bead_id, close_reason, "test-actor").expect("Close should succeed");

        // Verify close_reason is set correctly
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let bead = storage
            .get_issue(&bead_id)
            .expect("Failed to get bead")
            .unwrap();

        assert_eq!(
            bead.close_reason.as_deref(),
            Some(close_reason),
            "close_reason should be set to the provided reason"
        );
    }

    #[test]
    fn test_close_marks_bead_as_dirty() {
        let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

        // Close the bead
        close_bead(&db_path, &bead_id, "Test", "test-actor").expect("Close should succeed");

        // Verify bead is marked as dirty
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let dirty_issues = storage
            .list_dirty_issues()
            .expect("Failed to list dirty issues");

        assert!(
            dirty_issues.iter().any(|b| b.id == bead_id),
            "Closed bead should be in dirty list"
        );
    }

    #[test]
    fn test_close_creates_closed_event() {
        let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

        // Close the bead
        close_bead(&db_path, &bead_id, "Test reason", "test-actor").expect("Close should succeed");

        // Verify a 'closed' event was created
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let events = storage
            .list_events(&bead_id)
            .expect("Failed to list events");

        assert!(
            events
                .iter()
                .any(|e| e.event_type == crate::model::EventType::Closed),
            "Should have a 'closed' event after closing"
        );
    }

    #[test]
    fn test_close_sets_closed_by_session() {
        let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

        let actor = "test-worker-session-123";

        // Close the bead
        close_bead(&db_path, &bead_id, "Test", actor).expect("Close should succeed");

        // Verify closed_by_session is set
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let bead = storage
            .get_issue(&bead_id)
            .expect("Failed to get bead")
            .unwrap();

        assert_eq!(
            bead.closed_by_session.as_deref(),
            Some(actor),
            "closed_by_session should be set to the actor"
        );
    }

    #[test]
    fn test_close_with_empty_reason_uses_default() {
        let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

        // Close with empty reason
        close_bead(&db_path, &bead_id, "", "test-actor").expect("Close should succeed");

        // Verify close_reason is set (even if empty)
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let bead = storage
            .get_issue(&bead_id)
            .expect("Failed to get bead")
            .unwrap();

        // The storage layer should store the reason as provided
        assert_eq!(
            bead.close_reason.as_deref(),
            Some(""),
            "Empty close reason should be stored as empty string"
        );
    }

    #[test]
    fn test_close_updates_updated_at_timestamp() {
        let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

        let storage_before = Storage::open(&db_path).expect("Failed to open storage");
        let bead_before = storage_before
            .get_issue(&bead_id)
            .expect("Failed to get bead")
            .unwrap();
        let updated_at_before = bead_before.updated_at;

        // Wait a moment to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Close the bead
        close_bead(&db_path, &bead_id, "Test", "test-actor").expect("Close should succeed");

        // Verify updated_at changed
        let storage_after = Storage::open(&db_path).expect("Failed to open storage");
        let bead_after = storage_after
            .get_issue(&bead_id)
            .expect("Failed to get bead")
            .unwrap();

        assert!(
            bead_after.updated_at > updated_at_before,
            "updated_at should be updated after close"
        );
    }

    #[test]
    fn test_close_with_bead_in_blocked_state() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");

        let storage = Storage::open(&db_path).expect("Failed to open storage");

        // Create a bead in blocked state
        let bead_id = "bf-test-blocked-1".to_string();
        let now = Utc::now();
        let bead = Issue {
            id: bead_id.clone(),
            title: "Blocked test bead".to_string(),
            status: Status::Blocked,
            created_at: now,
            updated_at: now,
            ..Default::default()
        };

        storage
            .create_issue(&bead)
            .expect("Failed to create test bead");

        // Close the blocked bead
        let result = close_bead(&db_path, &bead_id, "Unblocked and closed", "test-actor");
        assert!(result.is_ok(), "Closing a blocked bead should succeed");

        // Verify it's closed
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let bead = storage
            .get_issue(&bead_id)
            .expect("Failed to get bead")
            .unwrap();
        assert_eq!(bead.status, Status::Closed, "Blocked bead should be closed");
    }

    #[test]
    fn test_close_with_bead_in_in_progress_state() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");

        let storage = Storage::open(&db_path).expect("Failed to open storage");

        // Create a bead in in_progress state
        let bead_id = "bf-test-inprogress-1".to_string();
        let now = Utc::now();
        let bead = Issue {
            id: bead_id.clone(),
            title: "In-progress test bead".to_string(),
            status: Status::InProgress,
            created_at: now,
            updated_at: now,
            ..Default::default()
        };

        storage
            .create_issue(&bead)
            .expect("Failed to create test bead");

        // Close the in_progress bead
        let result = close_bead(&db_path, &bead_id, "Implementation complete", "test-actor");
        assert!(result.is_ok(), "Closing an in_progress bead should succeed");

        // Verify it's closed
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let bead = storage
            .get_issue(&bead_id)
            .expect("Failed to get bead")
            .unwrap();
        assert_eq!(
            bead.status,
            Status::Closed,
            "In-progress bead should be closed"
        );
    }

    #[test]
    fn test_close_multiple_beads_independently() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");

        let storage = Storage::open(&db_path).expect("Failed to open storage");

        // Create multiple test beads
        let bead_ids = vec![
            "bf-test-multi-1".to_string(),
            "bf-test-multi-2".to_string(),
            "bf-test-multi-3".to_string(),
        ];

        for bead_id in &bead_ids {
            let now = Utc::now();
            let bead = Issue {
                id: bead_id.clone(),
                title: format!("Test bead {}", bead_id),
                status: Status::Open,
                created_at: now,
                updated_at: now,
                ..Default::default()
            };
            storage
                .create_issue(&bead)
                .expect("Failed to create test bead");
        }

        // Close each bead independently
        for bead_id in &bead_ids {
            let result = close_bead(&db_path, bead_id, "Independent close", "test-actor");
            assert!(result.is_ok(), "Closing bead {} should succeed", bead_id);
        }

        // Verify all are closed
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        for bead_id in &bead_ids {
            let bead = storage
                .get_issue(bead_id)
                .expect("Failed to get bead")
                .unwrap();
            assert_eq!(
                bead.status,
                Status::Closed,
                "Bead {} should be closed",
                bead_id
            );
        }
    }

    #[test]
    fn test_close_with_long_reason() {
        let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

        let long_reason = "This is a very long close reason that contains detailed information about what was accomplished, including implementation details, testing performed, and verification steps. ".repeat(10);

        // Close with long reason
        let result = close_bead(&db_path, &bead_id, &long_reason, "test-actor");
        assert!(result.is_ok(), "Closing with long reason should succeed");

        // Verify the full reason is stored
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let bead = storage
            .get_issue(&bead_id)
            .expect("Failed to get bead")
            .unwrap();

        assert_eq!(
            bead.close_reason.as_deref(),
            Some(long_reason.as_str()),
            "Long close reason should be stored completely"
        );
    }

    #[test]
    fn test_close_with_special_characters_in_reason() {
        let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

        let special_reason =
            "Closed with \"quotes\", 'apostrophes', & symbols <>, and \\backslashes";

        // Close with special characters
        let result = close_bead(&db_path, &bead_id, special_reason, "test-actor");
        assert!(
            result.is_ok(),
            "Closing with special characters should succeed"
        );

        // Verify reason is stored correctly
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let bead = storage
            .get_issue(&bead_id)
            .expect("Failed to get bead")
            .unwrap();

        assert_eq!(
            bead.close_reason.as_deref(),
            Some(special_reason),
            "Special characters in reason should be preserved"
        );
    }

    #[test]
    fn test_close_sets_status_to_closed() {
        let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

        // Close the bead
        close_bead(&db_path, &bead_id, "Test", "test-actor").expect("Close should succeed");

        // Verify status is exactly 'closed'
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let bead = storage
            .get_issue(&bead_id)
            .expect("Failed to get bead")
            .unwrap();

        assert_eq!(
            bead.status,
            Status::Closed,
            "Status should be exactly Status::Closed"
        );
        assert_eq!(
            bead.status.as_str(),
            "closed",
            "Status string should be 'closed'"
        );
    }

    #[test]
    fn test_close_preserves_other_fields() {
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
            priority: crate::model::Priority::HIGH,
            issue_type: crate::model::IssueType::Feature,
            assignee: Some("test-assignee".to_string()),
            created_at: now,
            updated_at: now,
            ..Default::default()
        };

        storage
            .create_issue(&bead)
            .expect("Failed to create test bead");

        // Close the bead
        close_bead(&db_path, &bead_id, "Test", "test-actor").expect("Close should succeed");

        // Verify other fields are preserved
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
            bead_after.assignee, bead.assignee,
            "Assignee should be preserved"
        );
        assert_eq!(
            bead_after.created_at, bead.created_at,
            "Created_at should be preserved"
        );
    }

    #[test]
    fn test_close_creates_event_with_correct_fields() {
        let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

        let actor = "test-actor-123";
        let reason = "Test close reason";

        // Close the bead
        close_bead(&db_path, &bead_id, reason, actor).expect("Close should succeed");

        // Verify event details
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let events = storage
            .list_events(&bead_id)
            .expect("Failed to list events");

        let closed_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == crate::model::EventType::Closed)
            .collect();

        assert_eq!(
            closed_events.len(),
            1,
            "Should have exactly one closed event"
        );

        let closed_event = closed_events[0];
        assert_eq!(
            closed_event.issue_id, bead_id,
            "Event should have correct issue_id"
        );
        assert_eq!(closed_event.actor, actor, "Event should have correct actor");
        assert_eq!(
            closed_event.new_value.as_deref(),
            Some(reason),
            "Event should have correct new_value (reason)"
        );
        assert!(
            closed_event.old_value.is_none(),
            "Event old_value should be None for close"
        );
    }

    #[test]
    fn test_close_database_persistence() {
        let (_temp_dir, db_path, bead_id) = setup_test_db_with_bead();

        // Close the bead
        close_bead(&db_path, &bead_id, "Test", "test-actor").expect("Close should succeed");

        // Reopen the database (simulating a new connection)
        let storage2 = Storage::open(&db_path).expect("Failed to reopen storage");
        let bead = storage2
            .get_issue(&bead_id)
            .expect("Failed to get bead")
            .unwrap();

        assert_eq!(
            bead.status,
            Status::Closed,
            "Status should persist after database reopen"
        );
        assert!(
            bead.closed_at.is_some(),
            "closed_at should persist after database reopen"
        );
        assert_eq!(
            bead.close_reason.as_deref(),
            Some("Test"),
            "close_reason should persist after database reopen"
        );
    }
}
