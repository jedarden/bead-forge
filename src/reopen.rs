//! Reopen bead functionality.

use crate::model::Status;
use crate::storage::Storage;
use anyhow::{anyhow, Result};
use std::path::Path;

/// Reopen a closed bead.
///
/// This function:
/// - Verifies the bead exists and is in 'closed' status
/// - Transitions the bead's status back to 'open'
/// - Clears the assignee (stale from when it was closed)
/// - Marks bead as dirty in SQLite
/// - Uses with_immediate_transaction for atomicity
///
/// # Arguments
/// * `db_path` - Path to the SQLite database
/// * `id` - Bead ID to reopen
///
/// # Errors
/// Returns error if:
/// - Bead not found
/// - Bead is not in Closed status
/// - Database operation fails
pub fn reopen_bead(db_path: &Path, id: &str) -> Result<()> {
    let storage = Storage::open(db_path)?;

    // Fetch the bead first to verify it exists and is closed
    let bead = storage
        .get_issue(id)?
        .ok_or_else(|| anyhow!("Bead {} not found", id))?;

    // Verify the bead is in Closed status
    if bead.status != Status::Closed {
        return Err(anyhow!(
            "Cannot reopen bead {}: status is '{}', must be 'closed'",
            id,
            bead.status.as_str()
        ));
    }

    // Perform the reopen operation
    storage.reopen_issue(id)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Issue, Status};
    use chrono::Utc;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Helper to create a test bead that is closed
    fn setup_closed_bead() -> (TempDir, PathBuf, String) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");

        let storage = Storage::open(&db_path).expect("Failed to open storage");

        // Create and close a test bead
        let bead_id = "bf-test-reopen-1".to_string();
        let now = Utc::now();
        let bead = Issue {
            id: bead_id.clone(),
            title: "Test bead for reopen operation".to_string(),
            status: Status::Closed,
            created_at: now,
            updated_at: now,
            closed_at: Some(now),
            close_reason: Some("Test close".to_string()),
            ..Default::default()
        };

        storage.create_issue(&bead).expect("Failed to create test bead");

        (temp_dir, db_path, bead_id)
    }

    /// Helper to create a test bead that is open (not closed)
    fn setup_open_bead() -> (TempDir, PathBuf, String) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");

        let storage = Storage::open(&db_path).expect("Failed to open storage");

        // Create an open test bead
        let bead_id = "bf-test-open-1".to_string();
        let now = Utc::now();
        let bead = Issue {
            id: bead_id.clone(),
            title: "Open test bead".to_string(),
            status: Status::Open,
            created_at: now,
            updated_at: now,
            ..Default::default()
        };

        storage.create_issue(&bead).expect("Failed to create test bead");

        (temp_dir, db_path, bead_id)
    }

    #[test]
    fn test_reopen_closed_bead_succeeds() {
        let (_temp_dir, db_path, bead_id) = setup_closed_bead();

        // Reopen the bead
        let result = reopen_bead(&db_path, &bead_id);
        assert!(result.is_ok(), "Reopening a closed bead should succeed");

        // Verify the bead is open
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let bead = storage.get_issue(&bead_id).expect("Failed to get bead");
        assert!(bead.is_some(), "Bead should still exist");
        assert_eq!(bead.unwrap().status, Status::Open, "Bead status should be open");
    }

    #[test]
    fn test_reopen_non_existent_bead_fails() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");

        let fake_bead_id = "bf-test-nonexistent-12345";

        // Try to reopen non-existent bead
        let result = reopen_bead(&db_path, fake_bead_id);
        assert!(result.is_err(), "Reopening non-existent bead should fail");

        let err = result.unwrap_err();
        let err_msg = err.to_string().to_lowercase();
        assert!(
            err_msg.contains("not found"),
            "Error message should mention bead not found, got: {}",
            err
        );
    }

    #[test]
    fn test_reopen_open_bead_fails() {
        let (_temp_dir, db_path, bead_id) = setup_open_bead();

        // Try to reopen an already open bead
        let result = reopen_bead(&db_path, &bead_id);
        assert!(result.is_err(), "Reopening an open bead should fail");

        let err = result.unwrap_err();
        let err_msg = err.to_string().to_lowercase();
        assert!(
            err_msg.contains("status") && err_msg.contains("must be"),
            "Error message should mention status requirement, got: {}",
            err
        );
    }

    #[test]
    fn test_reopen_in_progress_bead_fails() {
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

        storage.create_issue(&bead).expect("Failed to create test bead");

        // Try to reopen an in_progress bead
        let result = reopen_bead(&db_path, &bead_id);
        assert!(result.is_err(), "Reopening an in_progress bead should fail");

        let err = result.unwrap_err();
        let err_msg = err.to_string().to_lowercase();
        assert!(
            err_msg.contains("status") && err_msg.contains("must be"),
            "Error message should mention status requirement, got: {}",
            err
        );
    }

    #[test]
    fn test_reopen_blocked_bead_fails() {
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

        storage.create_issue(&bead).expect("Failed to create test bead");

        // Try to reopen a blocked bead
        let result = reopen_bead(&db_path, &bead_id);
        assert!(result.is_err(), "Reopening a blocked bead should fail");

        let err = result.unwrap_err();
        let err_msg = err.to_string().to_lowercase();
        assert!(
            err_msg.contains("status") && err_msg.contains("must be"),
            "Error message should mention status requirement, got: {}",
            err
        );
    }

    #[test]
    fn test_reopen_clears_assignee() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");

        let storage = Storage::open(&db_path).expect("Failed to open storage");

        // Create a closed bead with an assignee
        let bead_id = "bf-test-assignee-1".to_string();
        let now = Utc::now();
        let bead = Issue {
            id: bead_id.clone(),
            title: "Test bead with assignee".to_string(),
            status: Status::Closed,
            assignee: Some("previous-worker".to_string()),
            created_at: now,
            updated_at: now,
            closed_at: Some(now),
            close_reason: Some("Test".to_string()),
            ..Default::default()
        };

        storage.create_issue(&bead).expect("Failed to create test bead");

        // Reopen the bead
        reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");

        // Verify assignee is cleared
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let bead = storage.get_issue(&bead_id).expect("Failed to get bead").unwrap();

        assert!(
            bead.assignee.is_none(),
            "Assignee should be cleared after reopening"
        );
    }

    #[test]
    fn test_reopen_clears_closed_fields() {
        let (_temp_dir, db_path, bead_id) = setup_closed_bead();

        // Reopen the bead
        reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");

        // Verify closed_at and close_reason are cleared
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let bead = storage.get_issue(&bead_id).expect("Failed to get bead").unwrap();

        assert!(
            bead.closed_at.is_none(),
            "closed_at should be cleared after reopening"
        );
        assert!(
            bead.close_reason.is_none(),
            "close_reason should be cleared after reopening"
        );
    }

    #[test]
    fn test_reopen_marks_bead_as_dirty() {
        let (_temp_dir, db_path, bead_id) = setup_closed_bead();

        // Reopen the bead
        reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");

        // Verify bead is marked as dirty
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let dirty_issues = storage.list_dirty_issues().expect("Failed to list dirty issues");

        assert!(
            dirty_issues.iter().any(|b| b.id == bead_id),
            "Reopened bead should be in dirty list"
        );
    }

    #[test]
    fn test_reopen_creates_reopened_event() {
        let (_temp_dir, db_path, bead_id) = setup_closed_bead();

        // Reopen the bead
        reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");

        // Verify a 'reopened' event was created
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let events = storage.list_events(&bead_id).expect("Failed to list events");

        assert!(
            events.iter().any(|e| e.event_type == crate::model::EventType::Reopened),
            "Should have a 'reopened' event after reopening"
        );
    }

    #[test]
    fn test_reopen_updates_updated_at_timestamp() {
        let (_temp_dir, db_path, bead_id) = setup_closed_bead();

        let storage_before = Storage::open(&db_path).expect("Failed to open storage");
        let bead_before = storage_before
            .get_issue(&bead_id)
            .expect("Failed to get bead")
            .unwrap();
        let updated_at_before = bead_before.updated_at;

        // Wait a moment to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Reopen the bead
        reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");

        // Verify updated_at changed
        let storage_after = Storage::open(&db_path).expect("Failed to open storage");
        let bead_after = storage_after
            .get_issue(&bead_id)
            .expect("Failed to get bead")
            .unwrap();

        assert!(
            bead_after.updated_at > updated_at_before,
            "updated_at should be updated after reopen"
        );
    }

    #[test]
    fn test_reopen_preserves_other_fields() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");

        let storage = Storage::open(&db_path).expect("Failed to open storage");

        // Create a closed bead with various fields set
        let bead_id = "bf-test-preserve-1".to_string();
        let now = Utc::now();
        let bead = Issue {
            id: bead_id.clone(),
            title: "Test bead with many fields".to_string(),
            description: Some("Original description".to_string()),
            priority: crate::model::Priority::HIGH,
            issue_type: crate::model::IssueType::Feature,
            created_at: now,
            updated_at: now,
            status: Status::Closed,
            closed_at: Some(now),
            close_reason: Some("Test".to_string()),
            ..Default::default()
        };

        storage.create_issue(&bead).expect("Failed to create test bead");

        // Reopen the bead
        reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");

        // Verify other fields are preserved
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let bead_after = storage.get_issue(&bead_id).expect("Failed to get bead").unwrap();

        assert_eq!(bead_after.title, bead.title, "Title should be preserved");
        assert_eq!(
            bead_after.description, bead.description,
            "Description should be preserved"
        );
        assert_eq!(bead_after.priority, bead.priority, "Priority should be preserved");
        assert_eq!(
            bead_after.issue_type, bead.issue_type,
            "Issue type should be preserved"
        );
        assert_eq!(bead_after.created_at, bead.created_at, "Created_at should be preserved");
    }

    #[test]
    fn test_reopen_creates_event_with_correct_fields() {
        let (_temp_dir, db_path, bead_id) = setup_closed_bead();

        // Reopen the bead
        reopen_bead(&db_path, &bead_id).expect("Reopen should succeed");

        // Verify event details
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let events = storage.list_events(&bead_id).expect("Failed to list events");

        let reopened_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == crate::model::EventType::Reopened)
            .collect();

        assert_eq!(reopened_events.len(), 1, "Should have exactly one reopened event");

        let reopened_event = reopened_events[0];
        assert_eq!(reopened_event.issue_id, bead_id, "Event should have correct issue_id");
        assert_eq!(reopened_event.new_value.as_deref(), Some("open"), "Event should show new status as 'open'");
        assert_eq!(reopened_event.old_value.as_deref(), Some("closed"), "Event should show old status as 'closed'");
    }

    #[test]
    fn test_reopen_rolls_back_on_transaction_error() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");

        let storage = Storage::open(&db_path).expect("Failed to open storage");

        // Create a closed bead with an assignee
        let bead_id = "bf-test-rollback-1".to_string();
        let now = Utc::now();
        let bead = Issue {
            id: bead_id.clone(),
            title: "Test bead for rollback".to_string(),
            status: Status::Closed,
            assignee: Some("test-worker".to_string()),
            created_at: now,
            updated_at: now,
            closed_at: Some(now),
            close_reason: Some("Test close".to_string()),
            ..Default::default()
        };

        storage.create_issue(&bead).expect("Failed to create test bead");

        // Manually corrupt the database to force a transaction error
        // by deleting the events table (which reopen_issue needs)
        let conn = storage.conn.lock().unwrap();
        conn.execute("DROP TABLE events", []).expect("Failed to drop events table");
        drop(conn);

        // Attempt to reopen - this should fail partway through the transaction
        let result = reopen_bead(&db_path, &bead_id);

        // Should fail due to missing events table
        assert!(result.is_err(), "Reopen should fail when events table is missing");

        // Verify rollback occurred - bead should still be closed with original values
        let storage = Storage::open(&db_path).expect("Failed to open storage");
        let bead = storage.get_issue(&bead_id).expect("Failed to get bead").unwrap();

        // All original fields should be intact (no partial update)
        assert_eq!(bead.status, Status::Closed, "Status should remain closed after rollback");
        assert_eq!(bead.assignee, Some("test-worker".to_string()), "Assignee should remain after rollback");
        assert!(bead.closed_at.is_some(), "closed_at should still be set after rollback");
        assert_eq!(bead.close_reason, Some("Test close".to_string()), "close_reason should remain after rollback");

        // Verify bead is NOT marked as dirty (transaction was rolled back)
        let dirty_issues = storage.list_dirty_issues().expect("Failed to list dirty issues");
        assert!(!dirty_issues.iter().any(|b| b.id == bead_id), "Bead should not be marked as dirty after rollback");
    }
}
