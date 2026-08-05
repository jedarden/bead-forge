//! Basic blocker dependency functionality tests
//!
//! This test module validates the fundamental blocker behavior:
//! 1. A bead with an open blocking dependency cannot be claimed
//! 2. When the blocker closes, the dependent becomes claimable
//! 3. The dependency relationship is properly stored and retrieved
//!
//! This is bead bf-2xa7mr: First blocker for dependency testing

use bead_forge::claim::{claim, get_ready_candidates};
use bead_forge::model::{DependencyType, Issue, Status};
use bead_forge::storage::Storage;
use chrono::Utc;
use tempfile::NamedTempFile;

#[cfg(test)]
mod blocker_basics_tests {
    use super::*;

    fn setup_test_db() -> (NamedTempFile, Storage) {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();
        (temp_file, storage)
    }

    #[test]
    fn test_blocker_prevents_claiming_dependent() {
        // BF-2XA7MR: Core blocker test - dependent with open blocker cannot be claimed
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

        // Add blocking dependency: blocker -> dependent
        storage
            .add_dependency(
                "bf-dependent",
                "bf-blocker",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();

        // Verify the dependent is now blocked
        let dependent_issue = storage.get_issue("bf-dependent").unwrap().unwrap();
        assert_eq!(
            dependent_issue.status,
            Status::Blocked,
            "Dependent should be in blocked status when blocker is open"
        );

        // Verify dependent cannot be claimed (not in ready candidates)
        let candidates = storage
            .with_immediate_transaction(|tx| get_ready_candidates(tx, 100, None, None))
            .unwrap();

        assert!(
            !candidates.iter().any(|c| c.id == "bf-dependent"),
            "Blocked dependent should NOT appear in ready candidates"
        );

        // Verify blocker IS available for claiming
        assert!(
            candidates.iter().any(|c| c.id == "bf-blocker"),
            "Blocker should be available for claiming"
        );
    }

    #[test]
    fn test_closing_blocker_enables_claiming() {
        // BF-2XA7MR: After blocker closes, dependent becomes claimable
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

        // Verify dependent is now open (cascade from blocked->open)
        let dependent_issue = storage.get_issue("bf-dependent").unwrap().unwrap();
        assert_eq!(
            dependent_issue.status,
            Status::Open,
            "Dependent should transition to open when blocker closes"
        );

        // Verify dependent is now in ready candidates
        let candidates = storage
            .with_immediate_transaction(|tx| get_ready_candidates(tx, 100, None, None))
            .unwrap();

        assert!(
            candidates.iter().any(|c| c.id == "bf-dependent"),
            "Dependent should be in ready candidates after blocker closes"
        );

        // Verify we can successfully claim the dependent
        let result = storage
            .with_immediate_transaction(|tx| claim(tx, "worker1", 30, Utc::now(), None))
            .unwrap();

        assert!(
            result.is_some(),
            "Should be able to claim dependent after blocker closes"
        );
    }

    #[test]
    fn test_dependency_persistence_and_retrieval() {
        // BF-2XA7MR: Dependencies are correctly stored and retrieved
        let (_temp, storage) = setup_test_db();

        // Create beads
        let blocker = Issue::new(
            "bf-blocker".to_string(),
            "Blocker".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&blocker).unwrap();

        let dependent = Issue::new(
            "bf-dependent".to_string(),
            "Dependent".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&dependent).unwrap();

        // Add dependency
        storage
            .add_dependency(
                "bf-dependent",
                "bf-blocker",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();

        // Retrieve and verify the dependency
        let deps = storage.get_dependencies("bf-dependent").unwrap();
        assert_eq!(deps.len(), 1, "Should have exactly one dependency");

        let dep = &deps[0];
        assert_eq!(dep.issue_id, "bf-dependent");
        assert_eq!(dep.depends_on_id, "bf-blocker");
        assert_eq!(dep.dep_type, DependencyType::Blocks);

        // Verify dependency appears in the full bead object
        let dependent_issue = storage.get_issue("bf-dependent").unwrap().unwrap();
        assert_eq!(dependent_issue.dependencies.len(), 1);
        assert_eq!(dependent_issue.dependencies[0].depends_on_id, "bf-blocker");
    }

    #[test]
    fn test_multiple_blockers_all_must_close() {
        // BF-2XA7MR: With multiple blockers, dependent only opens when ALL close
        let (_temp, storage) = setup_test_db();

        // Create two blockers
        let blocker1 = Issue::new(
            "bf-blocker1".to_string(),
            "Blocker 1".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&blocker1).unwrap();

        let blocker2 = Issue::new(
            "bf-blocker2".to_string(),
            "Blocker 2".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&blocker2).unwrap();

        // Create dependent with TWO blockers
        let dependent = Issue::new(
            "bf-dependent".to_string(),
            "Dependent".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&dependent).unwrap();

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
        let dependent_issue = storage.get_issue("bf-dependent").unwrap().unwrap();
        assert_eq!(dependent_issue.status, Status::Blocked);

        // Close first blocker - dependent should STILL be blocked
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

        let dependent_issue = storage.get_issue("bf-dependent").unwrap().unwrap();
        assert_eq!(
            dependent_issue.status,
            Status::Blocked,
            "Dependent should remain blocked while one blocker is still open"
        );

        // Close second blocker - NOW dependent should open
        storage
            .update_issue(
                "bf-blocker2",
                &bead_forge::model::IssueChanges {
                    status: Some(Status::Closed),
                    actor: Some("test".to_string()),
                    ..Default::default()
                },
            )
            .unwrap();

        let dependent_issue = storage.get_issue("bf-dependent").unwrap().unwrap();
        assert_eq!(
            dependent_issue.status,
            Status::Open,
            "Dependent should open only after ALL blockers close"
        );
    }
}
