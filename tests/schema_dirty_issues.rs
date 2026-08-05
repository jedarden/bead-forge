//! Test dirty_issues table schema
//!
//! Verify the dirty_issues table is created correctly on DB init
//! and has the expected schema: issue_id TEXT PRIMARY KEY, marked_at DATETIME

use bead_forge::storage::Storage;
use tempfile::TempDir;
use rusqlite::Connection;

#[test]
fn test_dirty_issues_table_schema() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Open storage (this applies the schema)
    let storage = Storage::open(&db_path).unwrap();

    // Connect to verify schema
    let conn = Connection::open(&db_path).unwrap();

    // Check table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='dirty_issues'",
            [],
            |row| row.get::<_, i64>(0).map(|n| n > 0),
        )
        .unwrap();

    assert!(table_exists, "dirty_issues table should exist");

    // Check columns
    let columns: Vec<String> = conn
        .prepare("SELECT name FROM pragma_table_info('dirty_issues') ORDER BY cid")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(Result::ok)
        .collect();

    assert_eq!(columns.len(), 2, "dirty_issues should have 2 columns");
    assert_eq!(columns[0], "issue_id", "First column should be issue_id");
    assert_eq!(columns[1], "marked_at", "Second column should be marked_at");

    // Verify issue_id is PRIMARY KEY
    let is_pk: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('dirty_issues') WHERE name='issue_id' AND pk > 0",
            [],
            |row| row.get::<_, i64>(0).map(|n| n > 0),
        )
        .unwrap();

    assert!(is_pk, "issue_id should be PRIMARY KEY");

    // Verify marked_at has DATETIME type (stored as TEXT in SQLite)
    let marked_at_type: String = conn
        .query_row(
            "SELECT type FROM pragma_table_info('dirty_issues') WHERE name='marked_at'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(marked_at_type, "DATETIME", "marked_at should be DATETIME type");

    // Check foreign key to issues table
    let has_fk: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_foreign_key_list('dirty_issues') WHERE \"table\"='issues'",
            [],
            |row| row.get::<_, i64>(0).map(|n| n > 0),
        )
        .unwrap();

    assert!(has_fk, "dirty_issues should have foreign key to issues table");

    println!("✅ dirty_issues table schema is correct");
}

#[test]
fn test_dirty_issues_mark_and_list() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let storage = Storage::open(&db_path).unwrap();

    // Create an issue first
    let issue = bead_forge::model::Issue::new(
        "bf-test-456".to_string(),
        "Test issue".to_string(),
        ".".to_string(),
    );
    storage.create_issue(&issue).unwrap();

    // Mark it dirty
    storage.mark_dirty("bf-test-456").unwrap();

    // List dirty issues
    let dirty_issues = storage.list_dirty_issues().unwrap();
    assert_eq!(dirty_issues.len(), 1);
    assert_eq!(dirty_issues[0].id, "bf-test-456");

    // Clear dirty flags
    storage.clear_dirty().unwrap();

    // Verify empty
    let dirty_issues = storage.list_dirty_issues().unwrap();
    assert_eq!(dirty_issues.len(), 0);

    println!("✅ dirty_issues operations work correctly");
}
