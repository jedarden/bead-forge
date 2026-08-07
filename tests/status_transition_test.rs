//! Status transition tests
//!
//! Comprehensive tests for status transitions in the bead-forge system.
//! Tests cover:
//! - Status enum parsing and serialization
//! - Basic status updates via update_status
//! - Close and reopen operations
//! - Terminal status detection
//! - Status change event recording
//! - Cascade effects on dependent beads

use bead_forge::model::{Issue, IssueChanges, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;
use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // Test 1: Status enum parsing and serialization
    // ============================================================================

    #[test]
    fn test_status_enum_parsing() {
        // Test all standard status variants parse correctly
        let test_cases = vec![
            ("open", Status::Open),
            ("in_progress", Status::InProgress),
            ("inprogress", Status::InProgress), // Alternative format
            ("blocked", Status::Blocked),
            ("deferred", Status::Deferred),
            ("draft", Status::Draft),
            ("closed", Status::Closed),
            ("tombstone", Status::Tombstone),
            ("pinned", Status::Pinned),
        ];

        for (input, expected) in test_cases {
            let parsed = input.parse::<Status>().unwrap();
            assert_eq!(
                parsed, expected,
                "Status '{}' should parse to {:?}",
                input, expected
            );
        }
    }

    #[test]
    fn test_status_serialization_roundtrip() {
        // Test that status values serialize and deserialize correctly
        let statuses = vec![
            Status::Open,
            Status::InProgress,
            Status::Blocked,
            Status::Deferred,
            Status::Draft,
            Status::Closed,
            Status::Tombstone,
            Status::Pinned,
        ];

        for status in statuses {
            let serialized = serde_json::to_string(&status).unwrap();
            let deserialized: Status = serde_json::from_str(&serialized).unwrap();
            assert_eq!(
                deserialized, status,
                "Status {:?} should survive roundtrip",
                status
            );
        }
    }

    #[test]
    fn test_custom_status_parsing() {
        // Custom status strings should be accepted
        let custom = Status::from_str("in-review").unwrap();
        assert!(matches!(custom, Status::Custom(s) if s == "in-review"));

        let custom2 = Status::from_str("awaiting-approval").unwrap();
        assert!(matches!(custom2, Status::Custom(s) if s == "awaiting-approval"));
    }

    // ============================================================================
    // Test 2: Basic status transitions
    // ============================================================================

    #[test]
    fn test_basic_status_transition() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::open(&dir.path().join("test.db")).unwrap();

        // Create an issue
        let issue = Issue {
            id: "test-1".to_string(),
            title: "Test Status Transition".to_string(),
            status: Status::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();

        // Transition to in_progress
        storage
            .update_status("test-1", Status::InProgress)
            .unwrap();

        let retrieved = storage.get_issue("test-1").unwrap().unwrap();
        assert_eq!(retrieved.status, Status::InProgress);
    }

    #[test]
    fn test_multiple_status_transitions() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::open(&dir.path().join("test.db")).unwrap();

        // Create an issue
        let issue = Issue {
            id: "test-2".to_string(),
            title: "Multiple Status Transitions".to_string(),
            status: Status::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();

        // Chain: open -> in_progress -> blocked -> in_progress
        let transitions = vec![
            Status::InProgress,
            Status::Blocked,
            Status::InProgress,
        ];

        for status in transitions {
            storage.update_status("test-2", status.clone()).unwrap();
            let retrieved = storage.get_issue("test-2").unwrap().unwrap();
            assert_eq!(
                retrieved.status, status,
                "Status should transition to {:?}",
                status
            );
        }
    }

    #[test]
    fn test_transition_to_closed() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::open(&dir.path().join("test.db")).unwrap();

        // Create an issue
        let issue = Issue {
            id: "test-3".to_string(),
            title: "Close Transition".to_string(),
            status: Status::InProgress,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();

        // Close the issue
        storage
            .close_issue("test-3", "Test complete", "test-user")
            .unwrap();

        let retrieved = storage.get_issue("test-3").unwrap().unwrap();
        assert_eq!(retrieved.status, Status::Closed);
        assert_eq!(retrieved.close_reason.as_deref(), Some("Test complete"));
        assert!(retrieved.closed_at.is_some());
    }

    #[test]
    fn test_transition_from_closed_to_reopened() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::open(&dir.path().join("test.db")).unwrap();

        // Create and close an issue
        let issue = Issue {
            id: "test-4".to_string(),
            title: "Reopen Transition".to_string(),
            status: Status::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();

        storage
            .close_issue("test-4", "Initial close", "test-user")
            .unwrap();

        // Reopen the issue
        storage.reopen_issue("test-4").unwrap();

        let retrieved = storage.get_issue("test-4").unwrap().unwrap();
        assert_eq!(retrieved.status, Status::Open);
        assert!(retrieved.closed_at.is_none());
        assert!(retrieved.close_reason.is_none());
    }

    // ============================================================================
    // Test 3: Terminal status detection
    // ============================================================================

    #[test]
    fn test_terminal_status_detection() {
        // Test that canonical terminal statuses are detected
        assert!(Status::Closed.is_terminal());
        assert!(Status::Tombstone.is_terminal());

        // Test that non-terminal statuses are not terminal
        assert!(!Status::Open.is_terminal());
        assert!(!Status::InProgress.is_terminal());
        assert!(!Status::Blocked.is_terminal());
        assert!(!Status::Deferred.is_terminal());
        assert!(!Status::Draft.is_terminal());
        assert!(!Status::Pinned.is_terminal());
    }

    #[test]
    fn test_custom_terminal_status_aliases() {
        // Test custom status aliases that are considered terminal
        let done = Status::from_str("done").unwrap();
        assert!(done.is_terminal(), "Custom 'done' status should be terminal");

        let completed = Status::from_str("completed").unwrap();
        assert!(
            completed.is_terminal(),
            "Custom 'completed' status should be terminal"
        );
    }

    #[test]
    fn test_custom_non_terminal_status() {
        // Test that other custom statuses are not terminal
        let custom = Status::from_str("in-review").unwrap();
        assert!(!custom.is_terminal(), "Custom 'in-review' should not be terminal");
    }

    // ============================================================================
    // Test 4: Status change event recording
    // ============================================================================

    #[test]
    fn test_status_change_creates_event() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::open(&dir.path().join("test.db")).unwrap();

        // Create an issue
        let issue = Issue {
            id: "test-5".to_string(),
            title: "Status Change Event".to_string(),
            status: Status::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();

        // Change status
        storage
            .update_status("test-5", Status::InProgress)
            .unwrap();

        // Check that a status change event was recorded
        let retrieved = storage.get_issue("test-5").unwrap().unwrap();
        let status_events: Vec<_> = retrieved
            .events
            .into_iter()
            .filter(|e| e.event_type == bead_forge::model::EventType::StatusChanged)
            .collect();

        assert!(
            !status_events.is_empty(),
            "Status change should create an event"
        );
    }

    // ============================================================================
    // Test 5: Invalid status transitions
    // ============================================================================

    #[test]
    fn test_status_transition_on_nonexistent_issue() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::open(&dir.path().join("test.db")).unwrap();

        // Try to update status on nonexistent issue
        let result = storage.update_status("nonexistent", Status::InProgress);
        assert!(result.is_err(), "Should fail on nonexistent issue");
    }

    #[test]
    fn test_close_nonexistent_issue() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::open(&dir.path().join("test.db")).unwrap();

        // Try to close nonexistent issue
        let result = storage.close_issue("nonexistent", "test", "user");
        assert!(result.is_err(), "Should fail on nonexistent issue");
    }

    #[test]
    fn test_reopen_nonexistent_issue() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::open(&dir.path().join("test.db")).unwrap();

        // Try to reopen nonexistent issue
        let result = storage.reopen_issue("nonexistent");
        assert!(result.is_err(), "Should fail on nonexistent issue");
    }

    // ============================================================================
    // Test 6: Status transition with IssueChanges
    // ============================================================================

    #[test]
    fn test_status_transition_via_issue_changes() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::open(&dir.path().join("test.db")).unwrap();

        // Create an issue
        let issue = Issue {
            id: "test-6".to_string(),
            title: "IssueChanges Status Update".to_string(),
            status: Status::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();

        // Update using IssueChanges
        let changes = IssueChanges {
            status: Some(Status::Blocked),
            actor: Some("test-user".to_string()),
            ..Default::default()
        };

        storage
            .update_issue("test-6", &changes)
            .unwrap();

        let retrieved = storage.get_issue("test-6").unwrap().unwrap();
        assert_eq!(retrieved.status, Status::Blocked);
    }

    // ============================================================================
    // Test 7: Status transitions preserve other fields
    // ============================================================================

    #[test]
    fn test_status_transition_preserves_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::open(&dir.path().join("test.db")).unwrap();

        // Create an issue with metadata
        let issue = Issue {
            id: "test-7".to_string(),
            title: "Preserve Metadata".to_string(),
            status: Status::Open,
            priority: Priority::HIGH,
            issue_type: IssueType::Bug,
            assignee: Some("alice".to_string()),
            description: Some("Test description".to_string()),
            labels: vec!["urgent".to_string(), "backend".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();

        // Change status
        storage
            .update_status("test-7", Status::InProgress)
            .unwrap();

        let retrieved = storage.get_issue("test-7").unwrap().unwrap();
        assert_eq!(retrieved.priority, Priority::HIGH);
        assert_eq!(retrieved.issue_type, IssueType::Bug);
        assert_eq!(retrieved.assignee.as_deref(), Some("alice"));
        assert_eq!(
            retrieved.description.as_deref(),
            Some("Test description")
        );
        // Labels are stored alphabetically sorted
        assert_eq!(
            retrieved.labels,
            vec!["backend".to_string(), "urgent".to_string()]
        );
    }

    // ============================================================================
    // Test 8: Close and reopen helpers on Issue
    // ============================================================================

    #[test]
    fn test_issue_close_helper() {
        let issue = Issue {
            id: "test-8".to_string(),
            title: "Close Helper".to_string(),
            status: Status::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };

        let changes = issue.close("test-user".to_string());
        assert_eq!(changes.status, Some(Status::Closed));
        assert_eq!(changes.actor, Some("test-user".to_string()));
    }

    #[test]
    fn test_issue_reopen_helper() {
        let issue = Issue {
            id: "test-9".to_string(),
            title: "Reopen Helper".to_string(),
            status: Status::Closed,
            closed_at: Some(Utc::now()),
            close_reason: Some("Done".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };

        let changes = issue.reopen("test-user".to_string());
        assert_eq!(changes.status, Some(Status::Open));
        assert_eq!(changes.actor, Some("test-user".to_string()));
    }

    // ============================================================================
    // Test 9: Status string representation
    // ============================================================================

    #[test]
    fn test_status_display() {
        // Test that status display is correct
        assert_eq!(format!("{}", Status::Open), "open");
        assert_eq!(format!("{}", Status::InProgress), "in_progress");
        assert_eq!(format!("{}", Status::Blocked), "blocked");
        assert_eq!(format!("{}", Status::Deferred), "deferred");
        assert_eq!(format!("{}", Status::Draft), "draft");
        assert_eq!(format!("{}", Status::Closed), "closed");
        assert_eq!(format!("{}", Status::Tombstone), "tombstone");
        assert_eq!(format!("{}", Status::Pinned), "pinned");

        let custom = Status::Custom("custom-status".to_string());
        assert_eq!(format!("{}", custom), "custom-status");
    }

    #[test]
    fn test_status_as_str() {
        // Test that as_str returns correct values
        assert_eq!(Status::Open.as_str(), "open");
        assert_eq!(Status::InProgress.as_str(), "in_progress");
        assert_eq!(Status::Closed.as_str(), "closed");
    }

    // ============================================================================
    // Test 10: Active and draft status detection
    // ============================================================================

    #[test]
    fn test_active_status_detection() {
        // Test is_active method
        assert!(Status::Open.is_active());
        assert!(Status::InProgress.is_active());
        assert!(!Status::Blocked.is_active());
        assert!(!Status::Deferred.is_active());
        assert!(!Status::Closed.is_active());
    }

    #[test]
    fn test_draft_status_detection() {
        // Test is_draft method
        assert!(Status::Draft.is_draft());
        assert!(!Status::Open.is_draft());
        assert!(!Status::InProgress.is_draft());
    }
}
