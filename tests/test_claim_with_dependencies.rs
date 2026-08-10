/// Integration tests for bead claiming with dependencies
///
/// This test module verifies:
/// 1. Basic claiming functionality works
/// 2. Dependencies can be created between beads
/// 3. Claiming is blocked by open dependencies
/// 4. Dependencies appear in bead show output
/// 5. Claim scoring considers downstream impact

use bead_forge::claim::{claim, get_ready_candidates, WorkerMetadata};
use bead_forge::model::{Issue, IssueType, Priority, Status, DependencyType};
use bead_forge::storage::Storage;
use chrono::Utc;
use tempfile::NamedTempFile;

#[cfg(test)]
mod claim_with_dependencies_tests {
    use super::*;

    fn setup_test_db() -> (NamedTempFile, Storage) {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();
        (temp_file, storage)
    }

    #[test]
    fn test_basic_claim_single_bead() {
        let (_temp, storage) = setup_test_db();

        // Create a simple open bead
        let issue = Issue::new(
            "bf-test1".to_string(),
            "Test bead for claiming".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&issue).unwrap();

        // Claim it
        let result = storage
            .with_immediate_transaction(|tx| claim(tx, "worker1", 30, Utc::now(), None))
            .unwrap();

        assert!(result.is_some(), "Should successfully claim a bead");
        let claim_result = result.unwrap();
        assert_eq!(claim_result.bead_id, "bf-test1");
        assert_eq!(claim_result.reclaimed, 0);

        // Verify the bead is now in_progress
        let updated = storage.get_issue("bf-test1").unwrap().unwrap();
        assert_eq!(updated.status, Status::InProgress);
        assert_eq!(updated.assignee.as_ref().unwrap(), "worker1");
    }

    #[test]
    fn test_claim_creates_dependency_link() {
        let (_temp, storage) = setup_test_db();

        // Create two beads
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

        // Add a dependency link
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
    fn test_dependencies_appear_in_show() {
        let (_temp, storage) = setup_test_db();

        // Create beads with dependency
        let blocker = Issue::new(
            "bf-blocker".to_string(),
            "Blocker bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&blocker).unwrap();

        let dependent = Issue::new(
            "bf-dependent".to_string(),
            "Dependent with visible dependencies".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&dependent).unwrap();

        storage
            .add_dependency(
                "bf-dependent",
                "bf-blocker",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();

        // Get the dependent bead and verify dependencies are included
        let issue = storage.get_issue("bf-dependent").unwrap().unwrap();
        assert_eq!(issue.dependencies.len(), 1);
        assert_eq!(issue.dependencies[0].depends_on_id, "bf-blocker");
        assert_eq!(issue.dependencies[0].dep_type, DependencyType::Blocks);
    }

    #[test]
    fn test_claim_blocked_by_open_dependency() {
        let (_temp, storage) = setup_test_db();

        // Create a blocker bead (still open)
        let blocker = Issue::new(
            "bf-blocker".to_string(),
            "Blocker that must close first".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&blocker).unwrap();

        // Create a dependent bead
        let dependent = Issue::new(
            "bf-dependent".to_string(),
            "Dependent that should be blocked".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&dependent).unwrap();

        // Add the blocking dependency
        storage
            .add_dependency(
                "bf-dependent",
                "bf-blocker",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();

        // Try to get ready candidates - dependent should NOT be in the list
        let candidates = storage
            .with_immediate_transaction(|tx| get_ready_candidates(tx, 10, None, None))
            .unwrap();

        // The dependent should not appear in ready candidates
        assert!(
            !candidates.iter().any(|c| c.id == "bf-dependent"),
            "Dependent bead should not be ready while blocker is open"
        );

        // But the blocker should be available
        assert!(
            candidates.iter().any(|c| c.id == "bf-blocker"),
            "Blocker bead should be available for claiming"
        );
    }

    #[test]
    fn test_claim_unblocked_after_blocker_closes() {
        let (_temp, storage) = setup_test_db();

        // Create blocker
        let blocker = Issue::new(
            "bf-blocker".to_string(),
            "Blocker".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&blocker).unwrap();

        // Create dependent
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

        // Now the dependent should be available for claiming
        let candidates = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
            .unwrap();

        assert!(
            candidates.iter().any(|c| c.id == "bf-dependent"),
            "Dependent should be ready after blocker closes"
        );

        // And we should be able to claim it
        let result = storage
            .with_immediate_transaction(|tx| Ok(claim(tx, "worker1", 30, Utc::now(), None)?))
            .unwrap();

        assert!(result.is_some());
        // Since blocker is closed and dependent is now available,
        // either could be claimed depending on scoring
    }

    #[test]
    fn test_claim_prioritizes_high_downstream_impact() {
        let (_temp, storage) = setup_test_db();

        // Create a bead with high downstream impact (many dependents)
        let high_impact = Issue::new(
            "bf-high-impact".to_string(),
            "High impact bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&high_impact).unwrap();

        // Create 5 dependents on the high impact bead
        for i in 0..5 {
            let dependent = Issue::new(
                format!("bf-dep-{}", i),
                format!("Dependent {}", i),
                ".".to_string(),
            );
            storage.create_issue(&dependent).unwrap();

            storage
                .add_dependency(
                    &format!("bf-dep-{}", i),
                    "bf-high-impact",
                    &DependencyType::Blocks,
                    "test",
                )
                .unwrap();
        }

        // Create another bead with no dependents
        let low_impact = Issue::new(
            "bf-low-impact".to_string(),
            "Low impact bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&low_impact).unwrap();

        // Get ready candidates - high impact should be ranked higher
        let candidates = storage
            .with_immediate_transaction(|tx| get_ready_candidates(tx, 10, None, None))
            .unwrap();

        // The high impact bead should appear first (downstream_impact DESC ordering)
        if candidates.len() >= 2 {
            assert_eq!(candidates[0].id, "bf-high-impact");
            assert_eq!(candidates[0].downstream_impact, 5);
        }
    }
}
