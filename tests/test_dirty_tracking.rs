//! Tests for dirty_issues table and tracking infrastructure.
//!
//! These tests verify that issues are correctly marked as dirty when mutated,
//! which is essential for efficient JSONL export synchronization.

use bead_forge::model::{Issue, IssueChanges};
use bead_forge::storage::sqlite::Storage;
use tempfile::TempDir;

fn setup_storage() -> (Storage, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();
    (storage, temp_dir)
}

#[test]
fn test_dirty_issues_table_exists() {
    let (storage, _temp_dir) = setup_storage();

    // Verify we can query the dirty_issues table schema
    let conn = storage.conn.lock().unwrap();
    let table_exists: Result<bool, _> = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='dirty_issues'",
        [],
        |row| row.get::<_, i64>(0).map(|count| count > 0),
    );

    assert!(table_exists.is_ok());
    assert!(table_exists.unwrap(), "dirty_issues table should exist");

    // Verify the table has the correct columns
    let mut stmt = conn.prepare("PRAGMA table_info(dirty_issues)").unwrap();
    let mut rows = stmt.query([]).unwrap();
    let mut column_names = Vec::new();
    while let Some(row) = rows.next().unwrap() {
        if let Ok(name) = row.get::<_, String>(1) {
            column_names.push(name);
        }
    }

    assert!(column_names.contains(&"issue_id".to_string()));
    assert!(column_names.contains(&"marked_at".to_string()));
}

#[test]
fn test_create_issue_marks_dirty() {
    let (storage, _temp_dir) = setup_storage();

    // Create a new issue
    let issue = Issue::new("bf-test".to_string(), "Test Issue".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Verify it's in the dirty list
    let dirty_issues = storage.list_dirty_issues().unwrap();
    assert_eq!(dirty_issues.len(), 1);
    assert_eq!(dirty_issues[0].id, "bf-test");
}

#[test]
fn test_update_issue_marks_dirty() {
    let (storage, _temp_dir) = setup_storage();

    // Create an issue
    let issue = Issue::new("bf-test".to_string(), "Test Issue".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Clear dirty flags
    storage.clear_dirty().unwrap();

    // Update the issue
    let changes = IssueChanges {
        title: Some("Updated Title".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test", &changes).unwrap();

    // Verify it's marked dirty again
    let dirty_issues = storage.list_dirty_issues().unwrap();
    assert_eq!(dirty_issues.len(), 1);
    assert_eq!(dirty_issues[0].id, "bf-test");
}

#[test]
fn test_mark_dirty_persists() {
    let (storage, _temp_dir) = setup_storage();

    // Create an issue
    let issue = Issue::new("bf-test".to_string(), "Test Issue".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Mark as dirty explicitly
    storage.mark_dirty("bf-test").unwrap();

    // Reopen storage and verify dirty flag persists
    let db_path = _temp_dir.path().join("test.db");
    let storage2 = Storage::open(&db_path).unwrap();
    let dirty_issues = storage2.list_dirty_issues().unwrap();

    assert_eq!(dirty_issues.len(), 1);
    assert_eq!(dirty_issues[0].id, "bf-test");
}

#[test]
fn test_add_dependency_marks_dirty() {
    let (storage, _temp_dir) = setup_storage();

    // Create two issues
    let issue1 = Issue::new("bf-1".to_string(), "Issue 1".to_string(), ".".to_string());
    let issue2 = Issue::new("bf-2".to_string(), "Issue 2".to_string(), ".".to_string());
    storage.create_issue(&issue1).unwrap();
    storage.create_issue(&issue2).unwrap();

    // Clear dirty flags
    storage.clear_dirty().unwrap();

    // Add dependency
    storage
        .add_dependency("bf-1", "bf-2", &bead_forge::model::DependencyType::Blocks, "test")
        .unwrap();

    // Verify bf-1 is marked dirty (the issue that owns the dependency)
    let dirty_issues = storage.list_dirty_issues().unwrap();
    assert_eq!(dirty_issues.len(), 1);
    assert!(dirty_issues.iter().any(|i| i.id == "bf-1"));
}

#[test]
fn test_add_label_marks_dirty() {
    let (storage, _temp_dir) = setup_storage();

    // Create an issue
    let issue = Issue::new("bf-test".to_string(), "Test Issue".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Clear dirty flags
    storage.clear_dirty().unwrap();

    // Add label
    storage.add_label("bf-test", "urgent").unwrap();

    // Verify it's marked dirty
    let dirty_issues = storage.list_dirty_issues().unwrap();
    assert_eq!(dirty_issues.len(), 1);
    assert_eq!(dirty_issues[0].id, "bf-test");
}

#[test]
fn test_add_comment_marks_dirty() {
    let (storage, _temp_dir) = setup_storage();

    // Create an issue
    let issue = Issue::new("bf-test".to_string(), "Test Issue".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Clear dirty flags
    storage.clear_dirty().unwrap();

    // Add comment
    storage.add_comment("bf-test", "testuser", "Test comment").unwrap();

    // Verify it's marked dirty
    let dirty_issues = storage.list_dirty_issues().unwrap();
    assert_eq!(dirty_issues.len(), 1);
    assert_eq!(dirty_issues[0].id, "bf-test");
}

#[test]
fn test_clear_dirty() {
    let (storage, _temp_dir) = setup_storage();

    // Create multiple issues
    for i in 0..3 {
        let issue = Issue::new(format!("bf-{}", i), format!("Issue {}", i), ".".to_string());
        storage.create_issue(&issue).unwrap();
    }

    // Verify all are dirty
    let dirty_issues = storage.list_dirty_issues().unwrap();
    assert_eq!(dirty_issues.len(), 3);

    // Clear all dirty flags
    storage.clear_dirty().unwrap();

    // Verify no dirty issues remain
    let dirty_issues = storage.list_dirty_issues().unwrap();
    assert_eq!(dirty_issues.len(), 0);
}

#[test]
fn test_close_issue_marks_dirty() {
    let (storage, _temp_dir) = setup_storage();

    // Create an issue
    let issue = Issue::new("bf-test".to_string(), "Test Issue".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Clear dirty flags
    storage.clear_dirty().unwrap();

    // Close the issue
    storage.close_issue("bf-test", "Test complete", "test-session").unwrap();

    // Verify it's marked dirty
    let dirty_issues = storage.list_dirty_issues().unwrap();
    assert_eq!(dirty_issues.len(), 1);
    assert_eq!(dirty_issues[0].id, "bf-test");
}

#[test]
fn test_multiple_mutations_update_dirty_timestamp() {
    let (storage, _temp_dir) = setup_storage();

    // Create an issue
    let issue = Issue::new("bf-test".to_string(), "Test Issue".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Get initial marked_at time
    let conn = storage.conn.lock().unwrap();
    let initial_time: String = conn
        .query_row("SELECT marked_at FROM dirty_issues WHERE issue_id = 'bf-test'", [], |row| row.get(0))
        .unwrap();
    drop(conn);

    // Wait a bit and make another mutation
    std::thread::sleep(std::time::Duration::from_millis(10));
    let changes = IssueChanges {
        title: Some("Updated".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test", &changes).unwrap();

    // Get updated marked_at time
    let conn = storage.conn.lock().unwrap();
    let updated_time: String = conn
        .query_row("SELECT marked_at FROM dirty_issues WHERE issue_id = 'bf-test'", [], |row| row.get(0))
        .unwrap();
    drop(conn);

    // Verify timestamp was updated (should be later)
    assert_ne!(initial_time, updated_time);
}
