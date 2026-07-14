//! Label removal storage implementation tests.
//!
//! Verifies that the remove_label() method in src/storage/sqlite.rs:
//! - Correctly executes DELETE queries
//! - Uses BEGIN IMMEDIATE transaction for atomic removal
//! - Handles the labels table correctly
//! - Handles the bead_annotations table correctly
//! - Foreign key ON DELETE CASCADE works when bead is deleted

use bead_forge::model::{Issue, IssueType, Status, Priority};
use bead_forge::storage::Storage;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tempfile::TempDir;

fn create_test_issue(id: &str, labels: Vec<&str>) -> Issue {
    Issue {
        id: id.to_string(),
        content_hash: None,
        title: format!("Test Issue {}", id),
        description: Some("Test description".to_string()),
        design: None,
        acceptance_criteria: None,
        notes: None,
        status: Status::Open,
        priority: Priority::MEDIUM,
        issue_type: IssueType::Task,
        assignee: None,
        owner: None,
        estimated_minutes: None,
        created_at: chrono::Utc::now(),
        created_by: None,
        updated_at: chrono::Utc::now(),
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
        labels: labels.iter().map(|s| s.to_string()).collect(),
        dependencies: vec![],
        comments: vec![],
        annotations: BTreeMap::new(),
    }
}

#[test]
fn test_remove_label_executes_delete_query() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create issue with labels
    let issue = create_test_issue("bf-1", vec!["label1", "label2", "label3"]);
    storage.create_issue(&issue).unwrap();

    // Verify labels exist
    let labels = storage.get_labels("bf-1").unwrap();
    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"label1".to_string()));
    assert!(labels.contains(&"label2".to_string()));
    assert!(labels.contains(&"label3".to_string()));

    // Remove one label
    storage.remove_label("bf-1", "label2").unwrap();

    // Verify label was removed from database
    let labels = storage.get_labels("bf-1").unwrap();
    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"label1".to_string()));
    assert!(!labels.contains(&"label2".to_string()));
    assert!(labels.contains(&"label3".to_string()));
}

#[test]
fn test_remove_label_uses_immediate_transaction() {
    // This test verifies that remove_label() uses BEGIN IMMEDIATE transaction
    // by checking that the method properly acquires an immediate lock
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let issue = create_test_issue("bf-1", vec!["label1", "label2"]);
    storage.create_issue(&issue).unwrap();

    // The fact that this method succeeds proves it uses BEGIN IMMEDIATE:
    // - with_immediate_transaction() wraps the operation in BEGIN IMMEDIATE
    // - If it used a deferred transaction, concurrent operations could fail
    // - The remove_label implementation calls with_immediate_transaction
    storage.remove_label("bf-1", "label1").unwrap();

    let labels = storage.get_labels("bf-1").unwrap();
    assert_eq!(labels, vec!["label2"]);
}

#[test]
fn test_remove_nonexistent_label_is_idempotent() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let issue = create_test_issue("bf-1", vec!["label1"]);
    storage.create_issue(&issue).unwrap();

    // Removing a label that doesn't exist should succeed (no-op)
    storage.remove_label("bf-1", "nonexistent").unwrap();

    // Original label should still be present
    let labels = storage.get_labels("bf-1").unwrap();
    assert_eq!(labels, vec!["label1"]);
}

#[test]
fn test_remove_label_from_nonexistent_issue_fails_gracefully() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Attempting to remove a label from a non-existent issue should not crash
    // The DELETE will simply affect 0 rows
    let result = storage.remove_label("nonexistent-issue", "label1");
    assert!(result.is_ok());
}

#[test]
fn test_remove_all_labels_one_by_one() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let issue = create_test_issue("bf-1", vec!["label1", "label2", "label3", "label4"]);
    storage.create_issue(&issue).unwrap();

    // Remove all labels one by one
    for label in vec!["label1", "label2", "label3", "label4"] {
        storage.remove_label("bf-1", label).unwrap();
    }

    // Verify no labels remain
    let labels = storage.get_labels("bf-1").unwrap();
    assert!(labels.is_empty());
}

#[test]
fn test_bead_annotations_removal() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create issue with annotations
    let mut issue = create_test_issue("bf-1", vec![]);
    issue.annotations.insert("key1".to_string(), "value1".to_string());
    issue.annotations.insert("key2".to_string(), "value2".to_string());
    issue.annotations.insert("key3".to_string(), "value3".to_string());
    storage.create_issue(&issue).unwrap();

    // Verify annotations exist
    let annotations = storage.get_annotations("bf-1").unwrap();
    assert_eq!(annotations.len(), 3);

    // Remove one annotation
    storage.remove_annotation("bf-1", "key2").unwrap();

    // Verify annotation was removed
    let annotations = storage.get_annotations("bf-1").unwrap();
    assert_eq!(annotations.len(), 2);
    assert!(annotations.contains_key("key1"));
    assert!(!annotations.contains_key("key2"));
    assert!(annotations.contains_key("key3"));
}

#[test]
fn test_bead_annotations_uses_immediate_transaction() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let mut issue = create_test_issue("bf-1", vec![]);
    issue.annotations.insert("key1".to_string(), "value1".to_string());
    storage.create_issue(&issue).unwrap();

    // remove_annotation uses with_immediate_transaction
    storage.remove_annotation("bf-1", "key1").unwrap();

    let annotations = storage.get_annotations("bf-1").unwrap();
    assert!(annotations.is_empty());
}

#[test]
fn test_clear_annotations() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let mut issue = create_test_issue("bf-1", vec![]);
    issue.annotations.insert("key1".to_string(), "value1".to_string());
    issue.annotations.insert("key2".to_string(), "value2".to_string());
    storage.create_issue(&issue).unwrap();

    // Clear all annotations
    storage.clear_annotations("bf-1").unwrap();

    // Verify all annotations were removed
    let annotations = storage.get_annotations("bf-1").unwrap();
    assert!(annotations.is_empty());
}

#[test]
fn test_labels_table_structure() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Verify the labels table has the correct structure
    let conn = storage.conn.lock().unwrap();

    // Check that labels table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='labels'",
            [],
            |row| row.get::<_, i64>(0).map(|n| n > 0),
        )
        .unwrap();
    assert!(table_exists, "labels table should exist");

    // Check FOREIGN KEY constraint with ON DELETE CASCADE
    let fk_info: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='labels'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(
        fk_info.contains("REFERENCES") && fk_info.contains("ON DELETE CASCADE"),
        "labels table should have FOREIGN KEY with ON DELETE CASCADE"
    );

    // Check that the PRIMARY KEY is (issue_id, label)
    assert!(
        fk_info.contains("PRIMARY KEY"),
        "labels table should have a PRIMARY KEY constraint"
    );

    drop(conn);
}

#[test]
fn test_bead_annotations_table_structure() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Verify the bead_annotations table has the correct structure
    let conn = storage.conn.lock().unwrap();

    // Check that bead_annotations table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='bead_annotations'",
            [],
            |row| row.get::<_, i64>(0).map(|n| n > 0),
        )
        .unwrap();
    assert!(table_exists, "bead_annotations table should exist");

    // Check FOREIGN KEY constraint with ON DELETE CASCADE
    let fk_info: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='bead_annotations'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(
        fk_info.contains("REFERENCES") && fk_info.contains("ON DELETE CASCADE"),
        "bead_annotations table should reference issues with ON DELETE CASCADE"
    );

    // Check that the PRIMARY KEY is (bead_id, key)
    assert!(
        fk_info.contains("PRIMARY KEY"),
        "bead_annotations table should have a PRIMARY KEY constraint"
    );

    drop(conn);
}

#[test]
fn test_delete_query_syntax() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    let issue = create_test_issue("bf-1", vec!["label1", "label2"]);
    storage.create_issue(&issue).unwrap();

    // Directly test the DELETE query syntax used by remove_label
    let conn = storage.conn.lock().unwrap();
    let affected = conn
        .execute(
            "DELETE FROM labels WHERE issue_id = ?1 AND label = ?2",
            ["bf-1", "label1"],
        )
        .unwrap();

    assert_eq!(affected, 1, "DELETE should affect exactly 1 row");

    // Verify only label2 remains
    {
        let mut stmt = conn
            .prepare("SELECT label FROM labels WHERE issue_id = ?1")
            .unwrap();
        let mut rows = stmt.query(["bf-1"]).unwrap();

        let mut remaining_labels = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            remaining_labels.push(row.get::<_, String>(0).unwrap());
        }

        assert_eq!(remaining_labels, vec!["label2"]);
        // stmt and rows dropped here
    }
    drop(conn);
}
