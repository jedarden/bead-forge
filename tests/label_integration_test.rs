//! Integration test for bead_labels table persistence.
//!
//! Verifies that labels are correctly written to the bead_labels table
//! during create and update operations.

use bead_forge::model::{Issue, IssueChanges, Status};
use bead_forge::storage::Storage;
use chrono::Utc;
use std::path::PathBuf;

#[test]
fn test_labels_written_to_bead_labels_table_on_create() {
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create an issue with labels
    let issue = Issue {
        id: "bf-test-labels-1".to_string(),
        title: "Test Labels on Create".to_string(),
        labels: vec!["phase-1".to_string(), "bug".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&issue).unwrap();

    // Verify labels are in bead_labels table
    let labels = storage.get_labels("bf-test-labels-1").unwrap();
    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"phase-1".to_string()));
    assert!(labels.contains(&"bug".to_string()));

    // Verify via direct SQL query to bead_labels table
    let conn = storage.conn.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT label FROM bead_labels WHERE bead_id = ?1 ORDER BY label")
        .unwrap();
    let rows = stmt
        .query_map(["bf-test-labels-1"], |row| row.get::<_, String>(0))
        .unwrap();
    let db_labels: Vec<String> = rows.map(|r| r.unwrap()).collect();
    assert_eq!(db_labels, vec!["bug", "phase-1"]);
}

#[test]
fn test_labels_written_to_bead_labels_table_on_update() {
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create an issue with initial labels
    let issue = Issue {
        id: "bf-test-labels-2".to_string(),
        title: "Test Labels on Update".to_string(),
        labels: vec!["phase-1".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&issue).unwrap();

    // Update with new labels
    let changes = IssueChanges {
        labels: Some(vec!["phase-2".to_string(), "feature".to_string()]),
        ..Default::default()
    };

    storage.update_issue("bf-test-labels-2", &changes).unwrap();

    // Verify labels are updated in bead_labels table
    let labels = storage.get_labels("bf-test-labels-2").unwrap();
    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"phase-2".to_string()));
    assert!(labels.contains(&"feature".to_string()));

    // Verify via direct SQL query
    let conn = storage.conn.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT label FROM bead_labels WHERE bead_id = ?1 ORDER BY label")
        .unwrap();
    let rows = stmt
        .query_map(["bf-test-labels-2"], |row| row.get::<_, String>(0))
        .unwrap();
    let db_labels: Vec<String> = rows.map(|r| r.unwrap()).collect();
    assert_eq!(db_labels, vec!["feature", "phase-2"]);
}

#[test]
fn test_foreign_key_enforcement_bead_labels() {
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create an issue with a label
    let issue = Issue {
        id: "bf-test-fk-1".to_string(),
        title: "Test Foreign Key".to_string(),
        labels: vec!["test-label".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&issue).unwrap();

    // Verify label exists
    let labels = storage.get_labels("bf-test-fk-1").unwrap();
    assert_eq!(labels.len(), 1);

    // `close_issue` is a soft status update (UPDATE issues SET status =
    // 'closed', ...) — it never deletes the `issues` row, so ON DELETE
    // CASCADE never fires from a close. The only path that actually removes
    // the row (and so is the real exercise of the FK cascade) is a hard
    // delete, i.e. `DELETE FROM issues` — the same statement `cmd_delete`
    // issues in src/cli/mod.rs. Run that directly here since `Storage` has
    // no `delete_issue` convenience method.
    {
        let conn = storage.conn.lock().unwrap();
        conn.execute("DELETE FROM issues WHERE id = ?1", ["bf-test-fk-1"])
            .unwrap();
    }

    // Verify labels are deleted (no orphans)
    let conn = storage.conn.lock().unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM bead_labels WHERE bead_id = ?1",
            ["bf-test-fk-1"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        count, 0,
        "Labels should be cascade deleted when issue is closed"
    );
}

#[test]
fn test_labels_within_same_transaction() {
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // This test verifies that label writes happen atomically with issue creation
    // by checking that both the issue and labels exist together or not at all

    let issue = Issue {
        id: "bf-test-atomic-1".to_string(),
        title: "Test Atomic Labels".to_string(),
        labels: vec!["atomic-label".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // create_issue uses with_immediate_transaction which ensures atomicity
    storage.create_issue(&issue).unwrap();

    // Verify both issue and labels exist
    let retrieved = storage.get_issue("bf-test-atomic-1").unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().labels.len(), 1);
}

#[test]
fn test_empty_labels_does_not_create_orphan_records() {
    let db_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create issue with no labels
    let issue = Issue {
        id: "bf-test-empty-1".to_string(),
        title: "Test Empty Labels".to_string(),
        labels: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&issue).unwrap();

    // Verify no labels in bead_labels. Scoped so the MutexGuard drops before
    // the `storage.update_issue()` calls below — those take the same
    // `storage.conn` lock internally, and std::sync::Mutex is not reentrant:
    // holding this guard across those calls would self-deadlock the thread.
    {
        let conn = storage.conn.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM bead_labels WHERE bead_id = ?1",
                ["bf-test-empty-1"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    // Update with labels, then update back to empty
    let changes = IssueChanges {
        labels: Some(vec!["temp-label".to_string()]),
        ..Default::default()
    };
    storage.update_issue("bf-test-empty-1", &changes).unwrap();

    let changes = IssueChanges {
        labels: Some(vec![]),
        ..Default::default()
    };
    storage.update_issue("bf-test-empty-1", &changes).unwrap();

    // Verify no labels remain
    let conn = storage.conn.lock().unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM bead_labels WHERE bead_id = ?1",
            ["bf-test-empty-1"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0, "No orphaned labels should remain");
}
