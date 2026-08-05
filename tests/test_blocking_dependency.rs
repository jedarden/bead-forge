/// Unit tests for blocking dependency functionality
///
/// This test module verifies the blocked_issues_cache mechanism:
/// 1. Cache is correctly populated when blocking dependencies are added
/// 2. Cache is correctly updated when blockers change status
/// 3. Cache correctly handles multiple blockers
/// 4. Cache correctly handles different blocking dependency types
/// 5. Non-blocking dependency types don't affect the cache

use bead_forge::model::{Issue, Status, DependencyType};
use bead_forge::storage::Storage;
use tempfile::NamedTempFile;

#[cfg(test)]
mod blocking_dependency_tests {
    use super::*;

    fn setup_test_db() -> (NamedTempFile, Storage) {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();
        (temp_file, storage)
    }

    #[test]
    fn test_blocked_cache_populated_on_dependency_add() {
        let (_temp, storage) = setup_test_db();

        // Create a blocker bead
        let blocker = Issue::new(
            "bf-blocker".to_string(),
            "Blocker bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&blocker).unwrap();

        // Create a dependent bead
        let dependent = Issue::new(
            "bf-dependent".to_string(),
            "Dependent bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&dependent).unwrap();

        // Add a blocking dependency
        storage
            .add_dependency(
                "bf-dependent",
                "bf-blocker",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();

        // Verify the blocked_issues_cache was populated
        let blocked = storage.get_blocked_issues().unwrap();
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].0, "bf-dependent");

        // Verify the blocked_by JSON contains the blocker
        let blocked_by: Vec<String> = serde_json::from_str(&blocked[0].1).unwrap();
        assert_eq!(blocked_by.len(), 1);
        assert_eq!(blocked_by[0], "bf-blocker");
    }

    #[test]
    fn test_blocked_cache_cleared_when_blocker_closed() {
        let (_temp, storage) = setup_test_db();

        // Create blocker and dependent
        let blocker = Issue::new(
            "bf-blocker".to_string(),
            "Blocker bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&blocker).unwrap();

        let dependent = Issue::new(
            "bf-dependent".to_string(),
            "Dependent bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&dependent).unwrap();

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
        let blocked = storage.get_blocked_issues().unwrap();
        assert_eq!(blocked.len(), 1);

        // Close the blocker
        storage
            .update_issue(
                "bf-blocker",
                &bead_forge::model::IssueChanges {
                    status: Some(Status::Closed),
                    actor: Some("test".to_string()),
                    ..Default::default()
                },
            )
            .unwrap();

        // Verify the blocked_issues_cache was cleared
        let blocked = storage.get_blocked_issues().unwrap();
        assert_eq!(blocked.len(), 0, "Blocked cache should be empty when blocker is closed");
    }

    #[test]
    fn test_blocked_cache_handles_multiple_blockers() {
        let (_temp, storage) = setup_test_db();

        // Create two blockers
        let blocker1 = Issue::new(
            "bf-blocker1".to_string(),
            "First blocker".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&blocker1).unwrap();

        let blocker2 = Issue::new(
            "bf-blocker2".to_string(),
            "Second blocker".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&blocker2).unwrap();

        // Create dependent
        let dependent = Issue::new(
            "bf-dependent".to_string(),
            "Dependent with two blockers".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&dependent).unwrap();

        // Add two blocking dependencies
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

        // Verify blocked_issues_cache contains both blockers
        let blocked = storage.get_blocked_issues().unwrap();
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].0, "bf-dependent");

        let blocked_by: Vec<String> = serde_json::from_str(&blocked[0].1).unwrap();
        assert_eq!(blocked_by.len(), 2);
        assert!(blocked_by.contains(&"bf-blocker1".to_string()));
        assert!(blocked_by.contains(&"bf-blocker2".to_string()));
    }

    #[test]
    fn test_blocked_cache_partially_cleared_one_of_many_blockers() {
        let (_temp, storage) = setup_test_db();

        // Create two blockers
        let blocker1 = Issue::new(
            "bf-blocker1".to_string(),
            "First blocker".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&blocker1).unwrap();

        let blocker2 = Issue::new(
            "bf-blocker2".to_string(),
            "Second blocker".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&blocker2).unwrap();

        // Create dependent
        let dependent = Issue::new(
            "bf-dependent".to_string(),
            "Dependent with two blockers".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&dependent).unwrap();

        // Add two blocking dependencies
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

        // Close only the first blocker
        storage
            .update_issue(
                "bf-blocker1",
                &bead_forge::model::IssueChanges {
                    status: Some(Status::Closed),
                    actor: Some("test".to_string()),
                    ..Default::default()
                },
            )
            .unwrap();

        // Verify dependent is still blocked (by blocker2)
        let blocked = storage.get_blocked_issues().unwrap();
        assert_eq!(blocked.len(), 1);

        let blocked_by: Vec<String> = serde_json::from_str(&blocked[0].1).unwrap();
        assert_eq!(blocked_by.len(), 1);
        assert_eq!(blocked_by[0], "bf-blocker2");
    }

    #[test]
    fn test_non_blocking_dependency_not_in_cache() {
        let (_temp, storage) = setup_test_db();

        // Create two beads
        let bead1 = Issue::new(
            "bf-bead1".to_string(),
            "First bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&bead1).unwrap();

        let bead2 = Issue::new(
            "bf-bead2".to_string(),
            "Second bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&bead2).unwrap();

        // Add a non-blocking dependency (Related type)
        storage
            .add_dependency(
                "bf-bead2",
                "bf-bead1",
                &DependencyType::Related,
                "test",
            )
            .unwrap();

        // Verify the blocked_issues_cache is empty
        let blocked = storage.get_blocked_issues().unwrap();
        assert_eq!(blocked.len(), 0, "Non-blocking dependencies should not populate blocked cache");
    }

    #[test]
    fn test_all_blocking_dependency_types_in_cache() {
        let (_temp, storage) = setup_test_db();

        // Test all blocking dependency types
        let blocking_types = vec![
            DependencyType::Blocks,
            DependencyType::ParentChild,
            DependencyType::ConditionalBlocks,
            DependencyType::WaitsFor,
        ];

        for (idx, dep_type) in blocking_types.iter().enumerate() {
            let blocker_id = format!("bf-blocker-{}", idx);
            let dependent_id = format!("bf-dependent-{}", idx);

            let blocker = Issue::new(
                blocker_id.clone(),
                format!("Blocker {}", idx),
                ".".to_string(),
            );
            storage.create_issue(&blocker).unwrap();

            let dependent = Issue::new(
                dependent_id.clone(),
                format!("Dependent {}", idx),
                ".".to_string(),
            );
            storage.create_issue(&dependent).unwrap();

            storage
                .add_dependency(
                    &dependent_id,
                    &blocker_id,
                    dep_type,
                    "test",
                )
                .unwrap();
        }

        // Verify all blocking types populated the cache
        let blocked = storage.get_blocked_issues().unwrap();
        assert_eq!(blocked.len(), 4, "All 4 blocking dependency types should populate cache");
    }

    #[test]
    fn test_blocked_cache_rebuild() {
        let (_temp, storage) = setup_test_db();

        // Create blocker and dependent
        let blocker = Issue::new(
            "bf-blocker".to_string(),
            "Blocker bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&blocker).unwrap();

        let dependent = Issue::new(
            "bf-dependent".to_string(),
            "Dependent bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&dependent).unwrap();

        // Add blocking dependency (this should populate cache automatically)
        storage
            .add_dependency(
                "bf-dependent",
                "bf-blocker",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();

        // Manually rebuild the cache
        storage.rebuild_blocked_cache().unwrap();

        // Verify cache is still correct after rebuild
        let blocked = storage.get_blocked_issues().unwrap();
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].0, "bf-dependent");

        let blocked_by: Vec<String> = serde_json::from_str(&blocked[0].1).unwrap();
        assert_eq!(blocked_by.len(), 1);
        assert_eq!(blocked_by[0], "bf-blocker");
    }

    #[test]
    fn test_blocked_cache_empty_after_dependency_removal() {
        let (_temp, storage) = setup_test_db();

        // Create blocker and dependent
        let blocker = Issue::new(
            "bf-blocker".to_string(),
            "Blocker bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&blocker).unwrap();

        let dependent = Issue::new(
            "bf-dependent".to_string(),
            "Dependent bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&dependent).unwrap();

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
        let blocked = storage.get_blocked_issues().unwrap();
        assert_eq!(blocked.len(), 1);

        // Remove the dependency
        storage
            .remove_dependency(
                "bf-dependent",
                "bf-blocker",
            )
            .unwrap();

        // Verify the blocked_issues_cache was cleared
        let blocked = storage.get_blocked_issues().unwrap();
        assert_eq!(blocked.len(), 0, "Blocked cache should be empty after dependency removal");
    }

    #[test]
    fn test_custom_terminal_status_unblocks_dependent() {
        let (_temp, storage) = setup_test_db();

        // Create blocker and dependent
        let blocker = Issue::new(
            "bf-blocker".to_string(),
            "Blocker bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&blocker).unwrap();

        let dependent = Issue::new(
            "bf-dependent".to_string(),
            "Dependent bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&dependent).unwrap();

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
        let blocked = storage.get_blocked_issues().unwrap();
        assert_eq!(blocked.len(), 1);

        // Update blocker to custom terminal status "done"
        storage
            .update_issue(
                "bf-blocker",
                &bead_forge::model::IssueChanges {
                    status: Some(Status::Custom("done".to_string())),
                    actor: Some("test".to_string()),
                    ..Default::default()
                },
            )
            .unwrap();

        // Verify the blocked_issues_cache was cleared (custom terminal status unblocks)
        let blocked = storage.get_blocked_issues().unwrap();
        assert_eq!(blocked.len(), 0, "Blocked cache should be empty when blocker has custom terminal status");
    }
}
