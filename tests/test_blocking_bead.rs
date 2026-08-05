/// Comprehensive test for blocking bead functionality
///
/// This test verifies:
/// 1. Creating blocking dependencies between beads
/// 2. Blocked beads cannot be claimed
/// 3. Closing blockers cascades to unblock dependents
/// 4. Multiple blockers work correctly
/// 5. blocked_issues_cache is properly maintained
/// 6. Ready candidates excludes blocked beads

use bead_forge::claim::{claim, get_ready_candidates};
use bead_forge::model::{Issue, IssueType, Priority, Status, DependencyType};
use bead_forge::storage::Storage;
use chrono::Utc;
use tempfile::NamedTempFile;

#[cfg(test)]
mod blocking_bead_tests {
    use super::*;

    fn setup_test_db() -> (NamedTempFile, Storage) {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();
        (temp_file, storage)
    }

    fn create_open_bead(storage: &Storage, id: &str, title: &str, priority: Priority) -> Issue {
        let issue = Issue {
            id: id.to_string(),
            title: title.to_string(),
            priority,
            status: Status::Open,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();
        issue
    }

    #[test]
    fn test_create_blocking_dependency() {
        let (_temp, storage) = setup_test_db();

        // Create blocker and dependent beads
        let _blocker = create_open_bead(&storage, "bf-blocker", "Blocker", Priority::HIGH);
        let _dependent = create_open_bead(&storage, "bf-dependent", "Dependent", Priority::MEDIUM);

        // Add blocking dependency
        storage
            .add_dependency(
                "bf-dependent",
                "bf-blocker",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();

        // Verify dependency was created
        let deps = storage.get_dependencies("bf-dependent").unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].issue_id, "bf-dependent");
        assert_eq!(deps[0].depends_on_id, "bf-blocker");
        assert_eq!(deps[0].dep_type, DependencyType::Blocks);
    }

    #[test]
    fn test_blocked_bead_cannot_be_claimed() {
        let (_temp, storage) = setup_test_db();

        // Create blocker and dependent
        create_open_bead(&storage, "bf-blocker", "Blocker", Priority::HIGH);
        create_open_bead(&storage, "bf-dependent", "Dependent", Priority::MEDIUM);

        // Add blocking dependency
        storage
            .add_dependency(
                "bf-dependent",
                "bf-blocker",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();

        // Get ready candidates - dependent should NOT appear
        let candidates = storage
            .with_immediate_transaction(|tx| get_ready_candidates(tx, 10, None, None))
            .unwrap();

        assert!(
            !candidates.iter().any(|c| c.id == "bf-dependent"),
            "Dependent bead should not be ready while blocker is open"
        );

        // But blocker should be available
        assert!(
            candidates.iter().any(|c| c.id == "bf-blocker"),
            "Blocker bead should be available for claiming"
        );
    }

    #[test]
    fn test_closing_blocker_unblocks_dependent() {
        let (_temp, storage) = setup_test_db();

        // Create blocker and dependent
        create_open_bead(&storage, "bf-blocker", "Blocker", Priority::HIGH);
        create_open_bead(&storage, "bf-dependent", "Dependent", Priority::MEDIUM);

        // Add blocking dependency
        storage
            .add_dependency(
                "bf-dependent",
                "bf-blocker",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();

        // Verify dependent is blocked
        let dependent = storage.get_issue("bf-dependent").unwrap().unwrap();
        assert_eq!(
            dependent.status, Status::Blocked,
            "Dependent should be blocked while blocker is open"
        );

        // Close the blocker
        storage
            .close_issue("bf-blocker", "test", "Blocker complete")
            .unwrap();

        // Verify dependent is now open
        let dependent = storage.get_issue("bf-dependent").unwrap().unwrap();
        assert_eq!(
            dependent.status, Status::Open,
            "Dependent should be open after blocker closes"
        );

        // Dependent should now be available for claiming
        let candidates = storage
            .with_immediate_transaction(|tx| get_ready_candidates(tx, 10, None, None))
            .unwrap();

        assert!(
            candidates.iter().any(|c| c.id == "bf-dependent"),
            "Dependent should be ready after blocker closes"
        );
    }

    #[test]
    fn test_multiple_blockers_require_all_closed() {
        let (_temp, storage) = setup_test_db();

        // Create dependent with two blockers
        create_open_bead(&storage, "bf-blocker1", "Blocker 1", Priority::HIGH);
        create_open_bead(&storage, "bf-blocker2", "Blocker 2", Priority::HIGH);
        create_open_bead(&storage, "bf-dependent", "Dependent", Priority::MEDIUM);

        // Add both blocking dependencies
        storage
            .add_dependency(
                "bf-dependent",
                "bf-blocker1",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();
        storage
            .add_dependency(
                "bf-dependent",
                "bf-blocker2",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();

        // Verify dependent is blocked
        let dependent = storage.get_issue("bf-dependent").unwrap().unwrap();
        assert_eq!(dependent.status, Status::Blocked);

        // Close first blocker
        storage
            .close_issue("bf-blocker1", "test", "Blocker 1 complete")
            .unwrap();

        // Dependent should STILL be blocked
        let dependent = storage.get_issue("bf-dependent").unwrap().unwrap();
        assert_eq!(
            dependent.status, Status::Blocked,
            "Dependent should remain blocked while one blocker is still open"
        );

        // Close second blocker
        storage
            .close_issue("bf-blocker2", "test", "Blocker 2 complete")
            .unwrap();

        // NOW dependent should be open
        let dependent = storage.get_issue("bf-dependent").unwrap().unwrap();
        assert_eq!(
            dependent.status, Status::Open,
            "Dependent should be open only after all blockers close"
        );
    }

    #[test]
    fn test_blocked_issues_cache_excludes_blocked_beads() {
        let (_temp, storage) = setup_test_db();

        // Create blocker and dependent
        create_open_bead(&storage, "bf-blocker", "Blocker", Priority::HIGH);
        create_open_bead(&storage, "bf-dependent", "Dependent", Priority::MEDIUM);

        // Add blocking dependency
        storage
            .add_dependency(
                "bf-dependent",
                "bf-blocker",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();

        // Verify dependent is in blocked cache
        let blocked_count = storage
            .with_immediate_transaction(|tx| {
                let mut stmt = tx
                    .prepare("SELECT COUNT(*) FROM blocked_issues_cache WHERE issue_id = ?")
                    .unwrap();
                stmt.query_row(["bf-dependent"], |row| row.get::<_, i64>(0))
                    .map_err(|e| anyhow::anyhow!("Failed to query blocked cache: {}", e))
            })
            .unwrap();

        assert_eq!(
            blocked_count, 1,
            "Dependent should appear in blocked_issues_cache"
        );
    }

    #[test]
    fn test_blocked_bead_claim_returns_none() {
        let (_temp, storage) = setup_test_db();

        // Create blocker and dependent
        create_open_bead(&storage, "bf-blocker", "Blocker", Priority::HIGH);
        create_open_bead(&storage, "bf-dependent", "Dependent", Priority::MEDIUM);

        // Add blocking dependency
        storage
            .add_dependency(
                "bf-dependent",
                "bf-blocker",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();

        // Try to claim - should get blocker, not dependent
        let result = storage
            .with_immediate_transaction(|tx| claim(tx, "worker1", 30, Utc::now(), None))
            .unwrap();

        assert!(result.is_some(), "Should be able to claim blocker");
        assert_eq!(
            result.unwrap().bead_id, "bf-blocker",
            "Should claim blocker, not dependent"
        );
    }

    #[test]
    fn test_dependency_types_affect_blocking() {
        let (_temp, storage) = setup_test_db();

        // Test that non-blocking dependency types don't block
        create_open_bead(&storage, "bf-parent", "Parent", Priority::HIGH);
        create_open_bead(&storage, "bf-related", "Related", Priority::MEDIUM);

        // Add NON-blocking dependency (related type)
        storage
            .add_dependency(
                "bf-related",
                "bf-parent",
                &DependencyType::RelatesTo,
                "test",
            )
            .unwrap();

        // Both should be claimable since RelatesTo is not a blocking type
        let candidates = storage
            .with_immediate_transaction(|tx| get_ready_candidates(tx, 10, None, None))
            .unwrap();

        assert!(
            candidates.iter().any(|c| c.id == "bf-related"),
            "Bead with non-blocking dependency should be claimable"
        );
        assert!(
            candidates.iter().any(|c| c.id == "bf-parent"),
            "Parent bead should be claimable"
        );
    }

    #[test]
    fn test_all_blocking_dependency_types() {
        let (_temp, storage) = setup_test_db();

        // Test all blocking dependency types
        let blocking_types = vec![
            DependencyType::Blocks,
            DependencyType::ParentChild,
            DependencyType::ConditionalBlocks,
            DependencyType::WaitsFor,
        ];

        for (i, dep_type) in blocking_types.iter().enumerate() {
            let blocker_id = format!("bf-blocker-{}", i);
            let dependent_id = format!("bf-dependent-{}", i);

            create_open_bead(&storage, &blocker_id, &format!("Blocker {}", i), Priority::HIGH);
            create_open_bead(
                &storage,
                &dependent_id,
                &format!("Dependent {}", i),
                Priority::MEDIUM,
            );

            storage
                .add_dependency(&dependent_id, &blocker_id, dep_type, "test")
                .unwrap();

            // Verify dependent is blocked
            let dependent = storage.get_issue(&dependent_id).unwrap().unwrap();
            assert_eq!(
                dependent.status, Status::Blocked,
                "Dependent should be blocked by {} type",
                dep_type.as_str()
            );
        }
    }

    #[test]
    fn test_chain_of_blocking_dependencies() {
        let (_temp, storage) = setup_test_db();

        // Create a chain: A -> B -> C
        create_open_bead(&storage, "bf-a", "A", Priority::HIGH);
        create_open_bead(&storage, "bf-b", "B", Priority::MEDIUM);
        create_open_bead(&storage, "bf-c", "C", Priority::MEDIUM);

        // A blocks B, B blocks C
        storage
            .add_dependency("bf-b", "bf-a", &DependencyType::Blocks, "test")
            .unwrap();
        storage
            .add_dependency("bf-c", "bf-b", &DependencyType::Blocks, "test")
            .unwrap();

        // Verify B and C are blocked
        let b = storage.get_issue("bf-b").unwrap().unwrap();
        let c = storage.get_issue("bf-c").unwrap().unwrap();
        assert_eq!(b.status, Status::Blocked);
        assert_eq!(c.status, Status::Blocked);

        // Only A should be claimable
        let candidates = storage
            .with_immediate_transaction(|tx| get_ready_candidates(tx, 10, None, None))
            .unwrap();

        assert_eq!(
            candidates.len(), 1,
            "Only A should be claimable in the chain"
        );
        assert_eq!(candidates[0].id, "bf-a");

        // Close A -> B should open, C still blocked
        storage.close_issue("bf-a", "test", "A complete").unwrap();

        let b = storage.get_issue("bf-b").unwrap().unwrap();
        let c = storage.get_issue("bf-c").unwrap().unwrap();
        assert_eq!(b.status, Status::Open, "B should open after A closes");
        assert_eq!(c.status, Status::Blocked, "C should still be blocked");

        // Close B -> C should open
        storage.close_issue("bf-b", "test", "B complete").unwrap();

        let c = storage.get_issue("bf-c").unwrap().unwrap();
        assert_eq!(c.status, Status::Open, "C should open after B closes");
    }

    #[test]
    fn test_retrieve_blocked_by_list() {
        let (_temp, storage) = setup_test_db();

        // Create a bead with multiple blockers
        create_open_bead(&storage, "bf-blocker1", "Blocker 1", Priority::HIGH);
        create_open_bead(&storage, "bf-blocker2", "Blocker 2", Priority::HIGH);
        create_open_bead(&storage, "bf-dependent", "Dependent", Priority::MEDIUM);

        // Add blocking dependencies
        storage
            .add_dependency(
                "bf-dependent",
                "bf-blocker1",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();
        storage
            .add_dependency(
                "bf-dependent",
                "bf-blocker2",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();

        // Retrieve blocked_by list using get_dependencies
        let blocked_by = storage.get_dependencies("bf-dependent").unwrap();

        assert_eq!(
            blocked_by.len(),
            2,
            "Should have exactly 2 blockers"
        );
        assert!(
            blocked_by
                .iter()
                .any(|d| d.depends_on_id == "bf-blocker1" && d.dep_type == DependencyType::Blocks),
            "Should include blocker1"
        );
        assert!(
            blocked_by
                .iter()
                .any(|d| d.depends_on_id == "bf-blocker2" && d.dep_type == DependencyType::Blocks),
            "Should include blocker2"
        );

        // Verify issue_id is the dependent
        assert_eq!(
            blocked_by[0].issue_id, "bf-dependent",
            "Dependencies should be from dependent's perspective"
        );
    }

    #[test]
    fn test_retrieve_blocks_list() {
        let (_temp, storage) = setup_test_db();

        // Create a blocker with multiple dependents
        create_open_bead(&storage, "bf-blocker", "Blocker", Priority::HIGH);
        create_open_bead(&storage, "bf-dependent1", "Dependent 1", Priority::MEDIUM);
        create_open_bead(&storage, "bf-dependent2", "Dependent 2", Priority::MEDIUM);

        // Add blocking dependencies
        storage
            .add_dependency(
                "bf-dependent1",
                "bf-blocker",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();
        storage
            .add_dependency(
                "bf-dependent2",
                "bf-blocker",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();

        // Retrieve blocks list using get_dependents
        let blocks = storage.get_dependents("bf-blocker").unwrap();

        assert_eq!(blocks.len(), 2, "Should block exactly 2 beads");
        assert!(
            blocks
                .iter()
                .any(|d| d.issue_id == "bf-dependent1" && d.dep_type == DependencyType::Blocks),
            "Should include dependent1"
        );
        assert!(
            blocks
                .iter()
                .any(|d| d.issue_id == "bf-dependent2" && d.dep_type == DependencyType::Blocks),
            "Should include dependent2"
        );

        // Verify depends_on_id is the blocker
        assert_eq!(
            blocks[0].depends_on_id, "bf-blocker",
            "Dependencies should be from blocker's perspective"
        );
    }

    #[test]
    fn test_blocking_relationships_persist_to_sqlite() {
        let (_temp, storage) = setup_test_db();

        // Create beads
        create_open_bead(&storage, "bf-blocker", "Blocker", Priority::HIGH);
        create_open_bead(&storage, "bf-dependent", "Dependent", Priority::MEDIUM);

        // Add blocking dependency
        storage
            .add_dependency(
                "bf-dependent",
                "bf-blocker",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();

        // Directly query SQLite to verify persistence
        let conn = storage.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT issue_id, depends_on_id, type FROM dependencies WHERE issue_id = ?1")
            .unwrap();

        let deps = stmt
            .query_map(["bf-dependent"], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(deps.len(), 1, "Should have exactly 1 dependency in SQLite");
        let (issue_id, depends_on_id, dep_type) = &deps[0];
        assert_eq!(issue_id, "bf-dependent");
        assert_eq!(depends_on_id, "bf-blocker");
        assert_eq!(dep_type, "blocks");
    }

    #[test]
    fn test_empty_blocked_by_and_blocks_lists() {
        let (_temp, storage) = setup_test_db();

        // Create a bead with no dependencies
        create_open_bead(&storage, "bf-standalone", "Standalone", Priority::MEDIUM);

        // Verify empty blocked_by list
        let blocked_by = storage.get_dependencies("bf-standalone").unwrap();
        assert!(
            blocked_by.is_empty(),
            "Standalone bead should have no blocked_by list"
        );

        // Verify empty blocks list
        let blocks = storage.get_dependents("bf-standalone").unwrap();
        assert!(
            blocks.is_empty(),
            "Standalone bead should have no blocks list"
        );
    }
}
