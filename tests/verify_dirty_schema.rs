// Test to verify dirty_issues table schema is correctly defined and created

use bead_forge::storage::Storage;

#[test]
fn test_dirty_issues_table_exists() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    // Open storage (this should apply schema and create dirty_issues table)
    let storage = Storage::open(&db_path).expect("Failed to open storage");

    // Get the underlying connection to inspect schema
    let conn = storage.conn.lock().unwrap();

    // Check if dirty_issues table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='dirty_issues'",
            [],
            |row| row.get::<_, i64>(0).map(|n| n > 0),
        )
        .expect("Failed to query table existence");

    assert!(table_exists, "dirty_issues table should exist after DB initialization");

    // Verify table structure
    let columns: Vec<(String, String, String)> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='dirty_issues'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map(|sql| {
            // Parse the CREATE TABLE statement to extract column info
            // For now just verify the table exists and has expected columns
            vec![
                ("issue_id".to_string(), "TEXT".to_string(), "PRIMARY KEY".to_string()),
                ("marked_at".to_string(), "DATETIME".to_string(), "DEFAULT CURRENT_TIMESTAMP".to_string()),
            ]
        })
        .expect("Failed to get table schema");

    // Verify the columns using PRAGMA
    let mut stmt = conn
        .prepare("PRAGMA table_info(dirty_issues)")
        .expect("Failed to prepare PRAGMA");

    let column_info: Vec<(String, String)> = stmt
        .query_map([], |row| {
            let name: String = row.get(1)?;
            let type_: String = row.get(2)?;
            Ok((name, type_))
        })
        .expect("Failed to query column info")
        .collect::<Result<_, _>>()
        .expect("Failed to collect columns");

    // Should have 2 columns: issue_id and marked_at
    assert_eq!(column_info.len(), 2, "dirty_issues should have 2 columns");

    // Verify column names and types
    let col_map: std::collections::HashMap<String, String> = column_info.into_iter().collect();

    assert!(col_map.contains_key("issue_id"), "Should have issue_id column");
    assert_eq!(col_map.get("issue_id"), Some(&"TEXT".to_string()));

    assert!(col_map.contains_key("marked_at"), "Should have marked_at column");
    assert_eq!(col_map.get("marked_at"), Some(&"DATETIME".to_string()));
}

#[test]
fn test_dirty_issues_mark_and_clear() {
    use tempfile::TempDir;
    use bead_forge::model::{Issue, IssueType, Priority, Status};

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let storage = Storage::open(&db_path).expect("Failed to open storage");

    // Create a test issue
    let issue = Issue {
        id: "bf-dirty-test".to_string(),
        title: "Test Dirty Issue".to_string(),
        status: Status::Open,
        priority: Priority::MEDIUM,
        issue_type: IssueType::Task,
        source_repo: Some(".".to_string()),
        ..Default::default()
    };

    storage.create_issue(&issue).expect("Failed to create issue");

    // Verify it's marked as dirty (new issues are automatically dirty)
    let dirty_issues = storage.list_dirty_issues().expect("Failed to list dirty issues");
    assert_eq!(dirty_issues.len(), 1, "Newly created issue should be dirty");
    assert_eq!(dirty_issues[0].id, "bf-dirty-test");

    // Clear dirty flags
    storage.clear_dirty().expect("Failed to clear dirty");

    // Verify it's no longer dirty
    let dirty_issues = storage.list_dirty_issues().expect("Failed to list dirty issues");
    assert_eq!(dirty_issues.len(), 0, "Should have no dirty issues after clear");

    // Mark it dirty explicitly
    storage.mark_dirty("bf-dirty-test").expect("Failed to mark dirty");

    // Verify it's dirty again
    let dirty_issues = storage.list_dirty_issues().expect("Failed to list dirty issues");
    assert_eq!(dirty_issues.len(), 1, "Should have 1 dirty issue after explicit mark");
    assert_eq!(dirty_issues[0].id, "bf-dirty-test");
}
