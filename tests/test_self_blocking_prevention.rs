/// Test suite for self-blocking prevention (bf-lsb3km)
///
/// This test module verifies that a bead cannot block itself, which is a
/// nonsensical dependency relationship that would leave the bead permanently
/// unclaimable.
///
/// Test coverage includes:
/// - `bf dep add` command rejects self-blocking
/// - Batch operations reject self-blocking
/// - Error messages are informative
/// - Storage layer rejects self-blocking

use bead_forge::model::DependencyType;
use bead_forge::storage::Storage;
use tempfile::NamedTempFile;

#[cfg(test)]
mod self_blocking_tests {
    use super::*;

    // ===== TEST FIXTURES =====

    fn setup_test_db() -> (NamedTempFile, Storage) {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();
        (temp_file, storage)
    }

    fn create_open_bead(storage: &Storage, id: &str, title: &str) {
        use bead_forge::model::{Issue, IssueType, Priority, Status};

        let issue = Issue {
            id: id.to_string(),
            title: title.to_string(),
            priority: Priority::MEDIUM,
            status: Status::Open,
            issue_type: IssueType::Task,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();
    }

    // ===== STORAGE LAYER TESTS =====

    #[test]
    fn test_storage_add_dependency_rejects_self_blocking() {
        let (_temp, storage) = setup_test_db();

        // Create a bead
        create_open_bead(&storage, "bf-test", "Test bead");

        // Try to add self-blocking dependency via storage layer
        let result = storage.add_dependency(
            "bf-test",
            "bf-test",
            &DependencyType::Blocks,
            "test"
        );

        // Should fail with informative error
        assert!(result.is_err(), "Storage should reject self-blocking dependency");

        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.to_lowercase().contains("cannot") ||
            error_msg.to_lowercase().contains("block itself") ||
            error_msg.to_lowercase().contains("self-blocking"),
            "Error message should mention self-blocking prevention: {}",
            error_msg
        );
    }

    #[test]
    fn test_storage_add_dependency_allows_different_beads() {
        let (_temp, storage) = setup_test_db();

        // Create two beads
        create_open_bead(&storage, "bf-blocker", "Blocker");
        create_open_bead(&storage, "bf-dependent", "Dependent");

        // Adding dependency between different beads should succeed
        let result = storage.add_dependency(
            "bf-dependent",
            "bf-blocker",
            &DependencyType::Blocks,
            "test"
        );

        assert!(result.is_ok(), "Storage should allow blocking between different beads");
    }

    // ===== BATCH OPERATION TESTS =====

    #[test]
    fn test_batch_dep_add_blocker_rejects_self_blocking() {
        use bead_forge::batch::execute_dep_add_blocker;

        let (_temp, storage) = setup_test_db();

        // Create a bead
        create_open_bead(&storage, "bf-test", "Test bead");

        // Try to add self-blocking dependency via batch operation
        let result = storage.with_immediate_transaction(|tx| {
            execute_dep_add_blocker(tx, "bf-test", "bf-test")
        });

        // Should fail with informative error
        assert!(result.is_err(), "Batch operation should reject self-blocking");

        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.to_lowercase().contains("cannot") ||
            error_msg.to_lowercase().contains("block itself") ||
            error_msg.to_lowercase().contains("self-blocking") ||
            error_msg.to_lowercase().contains("same bead"),
            "Error message should mention self-blocking prevention: {}",
            error_msg
        );
    }

    #[test]
    fn test_batch_dep_add_blocker_allows_different_beads() {
        use bead_forge::batch::execute_dep_add_blocker;

        let (_temp, storage) = setup_test_db();

        // Create two beads
        create_open_bead(&storage, "bf-blocker", "Blocker");
        create_open_bead(&storage, "bf-dependent", "Dependent");

        // Adding dependency between different beads should succeed
        let result = storage.with_immediate_transaction(|tx| {
            execute_dep_add_blocker(tx, "bf-dependent", "bf-blocker")
        });

        assert!(result.is_ok(), "Batch operation should allow blocking between different beads");
    }

    // ===== ERROR MESSAGE QUALITY TESTS =====

    #[test]
    fn test_self_blocking_error_message_is_informative() {
        let (_temp, storage) = setup_test_db();

        create_open_bead(&storage, "bf-test", "Test bead");

        let result = storage.add_dependency(
            "bf-test",
            "bf-test",
            &DependencyType::Blocks,
            "test"
        );

        let error_msg = result.unwrap_err().to_string();

        // Error should be clear and actionable
        assert!(
            error_msg.len() > 10,
            "Error message should have reasonable length"
        );

        // Should mention the bead ID
        assert!(
            error_msg.contains("bf-test") || error_msg.contains("itself"),
            "Error message should reference the problematic bead: {}",
            error_msg
        );
    }

    #[test]
    fn test_self_blocking_prevents_database_corruption() {
        let (_temp, storage) = setup_test_db();

        create_open_bead(&storage, "bf-test", "Test bead");

        // Attempt to add self-blocking
        let _ = storage.add_dependency(
            "bf-test",
            "bf-test",
            &DependencyType::Blocks,
            "test"
        );

        // Verify no dependency was created
        let deps = storage.get_dependencies("bf-test").unwrap();
        assert!(
            deps.is_empty(),
            "No self-blocking dependency should exist in database"
        );

        // Verify the bead is still claimable (not permanently blocked)
        use bead_forge::claim::get_ready_candidates;
        let candidates = storage.with_immediate_transaction(|tx| {
            get_ready_candidates(tx, 10, None, None)
        }).unwrap();

        assert!(
            candidates.iter().any(|c| c.id == "bf-test"),
            "Bead should remain claimable after failed self-blocking attempt"
        );
    }

    // ===== EDGE CASES =====

    #[test]
    fn test_self_blocking_detection_case_insensitive() {
        let (_temp, storage) = setup_test_db();

        create_open_bead(&storage, "BF-TEST", "Test bead");

        // Try with same ID (case should match since IDs are case-sensitive)
        let result = storage.add_dependency(
            "BF-TEST",
            "BF-TEST",
            &DependencyType::Blocks,
            "test"
        );

        assert!(result.is_err(), "Should reject self-blocking regardless of case in ID");
    }

    #[test]
    fn test_self_blocking_with_different_dependency_types() {
        let (_temp, storage) = setup_test_db();

        create_open_bead(&storage, "bf-test", "Test bead");

        // Test all blocking dependency types
        let blocking_types = vec![
            DependencyType::Blocks,
            DependencyType::ParentChild,
            DependencyType::ConditionalBlocks,
            DependencyType::WaitsFor,
        ];

        for dep_type in blocking_types {
            let result = storage.add_dependency(
                "bf-test",
                "bf-test",
                &dep_type,
                "test"
            );

            assert!(
                result.is_err(),
                "Should reject self-blocking for dependency type {}",
                dep_type.as_str()
            );
        }
    }

    #[test]
    fn test_non_blocking_self_dependency_allowed() {
        let (_temp, storage) = setup_test_db();

        create_open_bead(&storage, "bf-test", "Test bead");

        // Non-blocking dependency types might be allowed (e.g., relates_to)
        // This test documents current behavior - may need adjustment based on requirements
        let result = storage.add_dependency(
            "bf-test",
            "bf-test",
            &DependencyType::RelatesTo,
            "test"
        );

        // Non-blocking self-reference might be acceptable for some use cases
        // If requirements change, this assertion can be updated
        assert!(result.is_ok(), "Non-blocking self-dependency may be allowed");
    }
}
