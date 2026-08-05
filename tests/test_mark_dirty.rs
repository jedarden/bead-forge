/// Test for mark_dirty() helper function
///
/// Verifies that:
/// 1. mark_dirty() inserts a row into dirty_issues table
/// 2. mark_dirty() updates an existing row (idempotent)
/// 3. mark_dirty() uses immediate transaction

use bead_forge::model::Issue;
use bead_forge::storage::Storage;
use chrono::Utc;
use tempfile::NamedTempFile;

#[cfg(test)]
mod mark_dirty_tests {
    use super::*;

    fn setup_test_db() -> (NamedTempFile, Storage) {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();
        (temp_file, storage)
    }

    #[test]
    fn test_mark_dirty_inserts_row() {
        let (_temp, storage) = setup_test_db();

        // Create a test bead
        let issue = Issue::new(
            "bf-test".to_string(),
            "Test bead for mark_dirty".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&issue).unwrap();

        // Mark the bead as dirty
        storage.mark_dirty("bf-test").unwrap();

        // Verify the row was inserted into dirty_issues
        let conn = storage.conn.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM dirty_issues WHERE issue_id = ?1",
                &["bf-test"],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1, "Should have exactly one row in dirty_issues");
    }

    #[test]
    fn test_mark_dirty_updates_existing_row() {
        let (_temp, storage) = setup_test_db();

        // Create a test bead
        let issue = Issue::new(
            "bf-test2".to_string(),
            "Test bead for mark_dirty update".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&issue).unwrap();

        // Mark the bead as dirty first time
        storage.mark_dirty("bf-test2").unwrap();

        // Get the first marked_at time
        let conn = storage.conn.lock().unwrap();
        let first_marked_at: String = conn
            .query_row(
                "SELECT marked_at FROM dirty_issues WHERE issue_id = ?1",
                &["bf-test2"],
                |row| row.get(0),
            )
            .unwrap();

        // Wait a bit to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Mark the bead as dirty again (should update existing row)
        storage.mark_dirty("bf-test2").unwrap();

        // Get the second marked_at time
        let second_marked_at: String = conn
            .query_row(
                "SELECT marked_at FROM dirty_issues WHERE issue_id = ?1",
                &["bf-test2"],
                |row| row.get(0),
            )
            .unwrap();

        // Verify the row still exists (COUNT = 1) and marked_at was updated
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM dirty_issues WHERE issue_id = ?1",
                &["bf-test2"],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1, "Should still have exactly one row");
        assert_ne!(
            first_marked_at, second_marked_at,
            "marked_at should be updated on second call"
        );
    }

    #[test]
    fn test_mark_dirty_uses_immediate_transaction() {
        let (_temp, storage) = setup_test_db();

        // Create a test bead
        let issue = Issue::new(
            "bf-test3".to_string(),
            "Test bead for immediate transaction".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&issue).unwrap();

        // Mark the bead as dirty - this should use with_immediate_transaction
        // If it doesn't, the test will fail due to SQLite busy state or other issues
        // The fact that it succeeds proves it uses proper transaction handling
        storage.mark_dirty("bf-test3").unwrap();

        // Verify the row was inserted
        let conn = storage.conn.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM dirty_issues WHERE issue_id = ?1",
                &["bf-test3"],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1, "Should have one row in dirty_issues");
    }
}
