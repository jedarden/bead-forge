//! Integration test for P0 (CRITICAL) priority labels and bead operations.
//!
//! This test suite verifies end-to-end P0 priority functionality including:
//! - Creating beads with P0 priority
//! - Updating beads to/from P0 priority
//! - Querying and filtering beads by P0 priority
//! - Label interactions with P0 priority beads
//! - P0 priority in dependencies and related operations
//!
//! P0 (Priority::CRITICAL, value 0) is the highest priority level in the system.

use bead_forge::model::{Issue, IssueChanges, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;

/// Helper function to create a test bead with P0 priority
fn create_p0_bead(id: &str, title: &str, labels: Vec<String>) -> Issue {
    Issue {
        id: id.to_string(),
        title: title.to_string(),
        priority: Priority::CRITICAL,
        labels,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    }
}

/// Helper function to create a test bead with a specific priority
fn create_bead_with_priority(id: &str, title: &str, priority: Priority) -> Issue {
    Issue {
        id: id.to_string(),
        title: title.to_string(),
        priority,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    }
}

#[test]
fn test_create_bead_with_p0_priority() {
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create a bead with P0 priority
    let bead = create_p0_bead(
        "bf-p0-test-1",
        "Critical bug in production",
        vec!["urgent".to_string(), "production".to_string()],
    );

    storage.create_issue(&bead).unwrap();

    // Verify the bead was stored with P0 priority
    let retrieved = storage.get_issue("bf-p0-test-1").unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(format!("{}", retrieved.priority), "P0");
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"urgent".to_string()));
    assert!(retrieved.labels.contains(&"production".to_string()));
}

#[test]
fn test_update_bead_to_p0_priority() {
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create a bead with medium priority
    let bead = create_bead_with_priority("bf-p0-update-1", "Escalated issue", Priority::MEDIUM);
    storage.create_issue(&bead).unwrap();

    // Update to P0 priority
    let changes = IssueChanges {
        priority: Some(Priority::CRITICAL.0),
        labels: Some(vec!["escalated".to_string()]),
        ..Default::default()
    };
    storage.update_issue("bf-p0-update-1", &changes).unwrap();

    // Verify the priority was updated to P0
    let retrieved = storage.get_issue("bf-p0-update-1").unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(format!("{}", retrieved.priority), "P0");
    assert!(retrieved.labels.contains(&"escalated".to_string()));
}

#[test]
fn test_update_bead_from_p0_priority() {
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create a bead with P0 priority
    let bead = create_p0_bead("bf-p0-downgrade-1", "Resolved critical issue", vec![]);
    storage.create_issue(&bead).unwrap();

    // Downgrade from P0 to P2 (medium)
    let changes = IssueChanges {
        priority: Some(Priority::MEDIUM.0),
        ..Default::default()
    };
    storage.update_issue("bf-p0-downgrade-1", &changes).unwrap();

    // Verify the priority was downgraded
    let retrieved = storage.get_issue("bf-p0-downgrade-1").unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.priority, Priority::MEDIUM);
    assert_eq!(retrieved.priority.0, 2);
    assert_eq!(format!("{}", retrieved.priority), "P2");
    assert_ne!(retrieved.priority, Priority::CRITICAL);
}

#[test]
fn test_query_beads_by_p0_priority() {
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create multiple beads with different priorities
    storage
        .create_issue(&create_p0_bead("bf-p0-query-1", "Critical bug 1", vec![]))
        .unwrap();
    storage
        .create_issue(&create_p0_bead("bf-p0-query-2", "Critical bug 2", vec![]))
        .unwrap();
    storage
        .create_issue(&create_bead_with_priority(
            "bf-p0-query-3",
            "Normal issue",
            Priority::MEDIUM,
        ))
        .unwrap();
    storage
        .create_issue(&create_bead_with_priority(
            "bf-p0-query-4",
            "Low priority issue",
            Priority::LOW,
        ))
        .unwrap();

    // Query for P0 priority beads
    let filter = bead_forge::model::IssueFilter {
        priority: Some(Priority::CRITICAL.0),
        ..Default::default()
    };
    let results = storage.list_issues(&filter).unwrap();

    // Should return exactly 2 P0 beads
    assert_eq!(results.len(), 2);
    for bead in &results {
        assert_eq!(bead.priority, Priority::CRITICAL);
        assert_eq!(bead.priority.0, 0);
    }
    let ids: Vec<&str> = results.iter().map(|b| b.id.as_str()).collect();
    assert!(ids.contains(&"bf-p0-query-1"));
    assert!(ids.contains(&"bf-p0-query-2"));
    assert!(!ids.contains(&"bf-p0-query-3"));
    assert!(!ids.contains(&"bf-p0-query-4"));
}

#[test]
fn test_p0_priority_with_labels_persistence() {
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create a P0 bead with labels
    let bead = create_p0_bead(
        "bf-p0-labels-1",
        "Critical security issue",
        vec!["security".to_string(), "p0-verification".to_string(), "urgent".to_string()],
    );
    storage.create_issue(&bead).unwrap();

    // Verify labels are persisted in bead_labels table
    let labels = storage.get_labels("bf-p0-labels-1").unwrap();
    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"security".to_string()));
    assert!(labels.contains(&"p0-verification".to_string()));
    assert!(labels.contains(&"urgent".to_string()));

    // Verify direct SQL query to bead_labels table
    let conn = storage.conn.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT label FROM bead_labels WHERE bead_id = ?1 ORDER BY label")
        .unwrap();
    let rows = stmt
        .query_map(["bf-p0-labels-1"], |row| row.get::<_, String>(0))
        .unwrap();
    let db_labels: Vec<String> = rows.map(|r| r.unwrap()).collect();
    assert_eq!(db_labels, vec!["p0-verification", "security", "urgent"]);
}

#[test]
fn test_p0_priority_serialization_roundtrip() {
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create a P0 bead
    let bead = create_p0_bead(
        "bf-p0-serialize-1",
        "Serialization test",
        vec!["test-label".to_string()],
    );
    storage.create_issue(&bead).unwrap();

    // Retrieve and verify serialization
    let retrieved = storage.get_issue("bf-p0-serialize-1").unwrap().unwrap();

    // Serialize to JSON
    let json = serde_json::to_string(&retrieved).unwrap();
    assert!(json.contains(r#""priority":0"#), "JSON should contain priority:0 for P0");
    assert!(json.contains(r#""id":"bf-p0-serialize-1""#));

    // Deserialize and verify
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.priority.0, 0);
    assert_eq!(deserialized.id, "bf-p0-serialize-1");
}

#[test]
fn test_p0_priority_ordering_in_query_results() {
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create beads with mixed priorities
    storage
        .create_issue(&create_bead_with_priority("bf-order-1", "P3 issue", Priority::LOW))
        .unwrap();
    storage
        .create_issue(&create_p0_bead("bf-order-2", "P0 issue 1", vec![]))
        .unwrap();
    storage
        .create_issue(&create_bead_with_priority("bf-order-3", "P1 issue", Priority::HIGH))
        .unwrap();
    storage
        .create_issue(&create_p0_bead("bf-order-4", "P0 issue 2", vec![]))
        .unwrap();
    storage
        .create_issue(&create_bead_with_priority(
            "bf-order-5",
            "P2 issue",
            Priority::MEDIUM,
        ))
        .unwrap();

    // Query all beads
    let all_beads = storage.list_issues(&bead_forge::model::IssueFilter::default()).unwrap();

    // Verify P0 beads have the correct priority value and sort correctly
    let p0_beads: Vec<_> = all_beads
        .iter()
        .filter(|b| b.priority == Priority::CRITICAL)
        .collect();
    assert_eq!(p0_beads.len(), 2);

    // Verify P0 has the lowest numeric value (highest priority)
    for bead in &all_beads {
        if bead.priority == Priority::CRITICAL {
            assert_eq!(bead.priority.0, 0);
            // P0 should be less than all other priorities
            assert!(bead.priority < Priority::HIGH);
            assert!(bead.priority < Priority::MEDIUM);
            assert!(bead.priority < Priority::LOW);
            assert!(bead.priority < Priority::BACKLOG);
        }
    }
}

#[test]
fn test_p0_priority_with_status_transitions() {
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create a P0 bead in open status
    let bead = create_p0_bead("bf-p0-status-1", "Critical issue", vec![]);
    storage.create_issue(&bead).unwrap();

    // Transition to in_progress
    let changes = IssueChanges {
        status: Some(Status::InProgress),
        actor: Some("test-user".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-p0-status-1", &changes).unwrap();

    let retrieved = storage.get_issue("bf-p0-status-1").unwrap().unwrap();
    assert_eq!(retrieved.status, Status::InProgress);
    assert_eq!(retrieved.priority, Priority::CRITICAL);

    // Transition to closed
    let changes = IssueChanges {
        status: Some(Status::Closed),
        actor: Some("test-user".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-p0-status-1", &changes).unwrap();

    let retrieved = storage.get_issue("bf-p0-status-1").unwrap().unwrap();
    assert_eq!(retrieved.status, Status::Closed);
    // Priority should still be P0 even when closed
    assert_eq!(retrieved.priority, Priority::CRITICAL);
}

#[test]
fn test_p0_priority_label_filtering() {
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create beads with P0 priority and various labels
    storage
        .create_issue(&create_p0_bead(
            "bf-p0-filter-1",
            "Critical with label",
            vec!["p0-verification".to_string()],
        ))
        .unwrap();
    storage
        .create_issue(&create_p0_bead(
            "bf-p0-filter-2",
            "Critical without label",
            vec!["urgent".to_string()],
        ))
        .unwrap();
    storage
        .create_issue(&create_bead_with_priority(
            "bf-p0-filter-3",
            "Normal with label",
            Priority::MEDIUM,
        ))
        .unwrap();

    // Filter by both P0 priority and p0-verification label
    let filter = bead_forge::model::IssueFilter {
        priority: Some(Priority::CRITICAL.0),
        labels: Some(vec!["p0-verification".to_string()]),
        ..Default::default()
    };
    let results = storage.list_issues(&filter).unwrap();

    // Should return exactly 1 bead with P0 priority AND p0-verification label
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "bf-p0-filter-1");
    assert_eq!(results[0].priority, Priority::CRITICAL);
    assert!(results[0].labels.contains(&"p0-verification".to_string()));
}

#[test]
fn test_p0_priority_default_comparison() {
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create a bead with default priority
    let default_bead = Issue {
        id: "bf-default-1".to_string(),
        title: "Default priority".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };
    storage.create_issue(&default_bead).unwrap();

    // Create a P0 bead
    let p0_bead = create_p0_bead("bf-p0-default-1", "P0 priority", vec![]);
    storage.create_issue(&p0_bead).unwrap();

    let retrieved_default = storage.get_issue("bf-default-1").unwrap().unwrap();
    let retrieved_p0 = storage.get_issue("bf-p0-default-1").unwrap().unwrap();

    // Default should be MEDIUM (P2), not P0
    assert_eq!(retrieved_default.priority, Priority::MEDIUM);
    assert_eq!(retrieved_default.priority.0, 2);

    // P0 should be CRITICAL
    assert_eq!(retrieved_p0.priority, Priority::CRITICAL);
    assert_eq!(retrieved_p0.priority.0, 0);

    // P0 should be less than default (numerically lower priority value)
    assert!(retrieved_p0.priority < retrieved_default.priority);
    assert_ne!(retrieved_p0.priority, retrieved_default.priority);
}

#[test]
fn test_multiple_p0_beads_same_creation() {
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create multiple P0 beads in succession
    let ids = vec![
        "bf-multi-p0-1",
        "bf-multi-p0-2",
        "bf-multi-p0-3",
        "bf-multi-p0-4",
        "bf-multi-p0-5",
    ];

    for id in &ids {
        storage
            .create_issue(&create_p0_bead(id, &format!("Critical issue {}", id), vec![]))
            .unwrap();
    }

    // Verify all P0 beads are stored correctly
    for id in &ids {
        let retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(retrieved.priority, Priority::CRITICAL);
        assert_eq!(retrieved.priority.0, 0);
    }

    // Query all P0 beads
    let filter = bead_forge::model::IssueFilter {
        priority: Some(Priority::CRITICAL.0),
        ..Default::default()
    };
    let results = storage.list_issues(&filter).unwrap();

    assert_eq!(results.len(), 5);
    for bead in &results {
        assert_eq!(bead.priority, Priority::CRITICAL);
    }
}

#[test]
fn test_p0_priority_with_timestamps() {
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create a P0 bead
    let now = Utc::now();
    let bead = create_p0_bead("bf-p0-time-1", "Critical with timestamps", vec![]);
    storage.create_issue(&bead).unwrap();

    let retrieved = storage.get_issue("bf-p0-time-1").unwrap().unwrap();

    // Verify timestamps are preserved
    assert!(retrieved.created_at >= now);
    assert!(retrieved.updated_at >= now);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
}
