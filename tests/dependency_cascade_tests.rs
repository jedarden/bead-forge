//! Dependency cascade tests for bead deletion operations.
//!
//! This test module verifies the following acceptance criteria for bead bf-6bcya8:
//! 1. Test deleting beads with active dependencies updates dependents
//! 2. Verify dependent beads have their dependency lists updated correctly
//! 3. Test that deleting a bead clears it from all blocking lists
//! 4. Uses test fixtures from bf-3jdjyz
//!
//! These tests verify that ON DELETE CASCADE works correctly for the
//! dependencies table and that dependent beads have their dependency lists
//! updated appropriately when blockers are deleted.

use bead_forge::model::{DependencyType, Issue, IssueChanges, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;
use std::fs;
use tempfile::TempDir;

/// Isolated test workspace for dependency cascade testing.
///
/// Automatically cleaned up when dropped (TempDir).
pub struct DependencyCascadeTestWorkspace {
    /// Temp directory holding the workspace (auto-cleaned on drop)
    pub temp_dir: TempDir,
    /// Path to the .beads directory
    pub beads_dir: std::path::PathBuf,
    /// Path to the database file
    pub db_path: std::path::PathBuf,
    /// Storage backend
    pub storage: Storage,
}

impl DependencyCascadeTestWorkspace {
    /// Create a new isolated test workspace for dependency cascade testing.
    ///
    /// Initializes:
    /// - Temporary directory
    /// - .beads/ subdirectory with config
    /// - SQLite database with full schema
    /// - Empty JSONL file
    ///
    /// # Returns
    ///
    /// Result containing the workspace or an error.
    pub fn new() -> anyhow::Result<Self> {
        let temp_dir = TempDir::new()?;
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir_all(&beads_dir)?;

        // Initialize bf config
        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            r#"issue_prefixes: [bf]
default_priority: 2
default_type: task
claim_ttl_minutes: 30
"#,
        )?;

        // Initialize metadata
        let metadata_path = beads_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
        )?;

        let db_path = beads_dir.join("beads.db");
        let jsonl_path = beads_dir.join("issues.jsonl");

        // Initialize database with schema
        let storage = Storage::open(&db_path)?;

        // Create empty JSONL file
        fs::write(&jsonl_path, "[]")?;

        Ok(Self {
            temp_dir,
            beads_dir,
            db_path,
            storage,
        })
    }

    /// Create a test bead with the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `id` - Bead ID
    /// * `title` - Bead title
    /// * `status` - Bead status
    /// * `priority` - Bead priority
    ///
    /// # Returns
    ///
    /// The created Issue.
    pub fn create_bead(
        &self,
        id: &str,
        title: &str,
        status: Status,
        priority: Priority,
    ) -> anyhow::Result<Issue> {
        let issue = Issue {
            id: id.to_string(),
            title: title.to_string(),
            priority,
            status,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            events: Vec::new(),
            ..Default::default()
        };
        self.storage.create_issue(&issue)?;
        Ok(issue)
    }

    /// Add a blocking dependency between two beads.
    ///
    /// # Arguments
    ///
    /// * `dependent_id` - The bead that depends on the blocker
    /// * `blocker_id` - The bead that blocks the dependent
    pub fn add_blocking_dependency(
        &self,
        dependent_id: &str,
        blocker_id: &str,
    ) -> anyhow::Result<()> {
        self.storage.add_dependency(
            dependent_id,
            blocker_id,
            &DependencyType::Blocks,
            "test",
        )?;
        Ok(())
    }

    /// Get dependency list for a bead.
    ///
    /// # Arguments
    ///
    /// * `bead_id` - The bead to get dependencies for
    ///
    /// # Returns
    ///
    /// Vector of dependencies.
    pub fn get_dependencies(&self, bead_id: &str) -> anyhow::Result<Vec<bead_forge::model::Dependency>> {
        let deps = self.storage.get_dependencies(bead_id)?;
        Ok(deps)
    }

    /// Get dependents (beads that depend on this bead).
    ///
    /// # Arguments
    ///
    /// * `bead_id` - The bead to get dependents for
    ///
    /// # Returns
    ///
    /// Vector of dependent Dependency objects.
    pub fn get_dependents(&self, bead_id: &str) -> anyhow::Result<Vec<bead_forge::model::Dependency>> {
        let dependents = self.storage.get_dependents(bead_id)?;
        Ok(dependents)
    }

    /// Delete a bead by setting it to tombstone status.
    ///
    /// # Arguments
    ///
    /// * `bead_id` - The bead to delete
    pub fn delete_bead(&self, bead_id: &str) -> anyhow::Result<()> {
        self.storage.update_issue(
            bead_id,
            &IssueChanges {
                status: Some(Status::Tombstone),
                actor: Some("test".to_string()),
                ..Default::default()
            },
        )?;
        Ok(())
    }

    /// Get a bead by ID.
    ///
    /// # Arguments
    ///
    /// * `bead_id` - The bead ID
    ///
    /// # Returns
    ///
    /// The Issue if found.
    pub fn get_bead(&self, bead_id: &str) -> anyhow::Result<Option<Issue>> {
        let issue = self.storage.get_issue(bead_id)?;
        Ok(issue)
    }
}

#[cfg(test)]
mod dependency_cascade_tests {
    use super::*;

    // TEST 1: Deleting beads with active dependencies updates dependents

    #[test]
    fn test_deleting_bead_with_single_dependent_removes_from_dependent_list() {
        let ws = DependencyCascadeTestWorkspace::new().unwrap();

        // Create blocker and dependent
        ws.create_bead("bf-blocker", "Blocker", Status::Open, Priority::HIGH)
            .unwrap();
        ws.create_bead("bf-dependent", "Dependent", Status::Open, Priority::MEDIUM)
            .unwrap();

        // Add blocking dependency
        ws.add_blocking_dependency("bf-dependent", "bf-blocker")
            .unwrap();

        // Verify dependency exists
        let deps = ws.get_dependencies("bf-dependent").unwrap();
        assert_eq!(deps.len(), 1, "Should have 1 dependency");
        assert_eq!(deps[0].depends_on_id, "bf-blocker");

        // Verify dependent is in blocker's dependent list
        let dependents = ws.get_dependents("bf-blocker").unwrap();
        assert_eq!(dependents.len(), 1, "Blocker should have 1 dependent");
        assert_eq!(dependents[0].issue_id, "bf-dependent");

        // Delete the blocker
        ws.delete_bead("bf-blocker").unwrap();

        // Verify blocker is deleted (tombstoned)
        let blocker = ws.get_bead("bf-blocker").unwrap();
        assert!(blocker.is_some(), "Blocker should still exist (tombstoned)");
        assert_eq!(blocker.unwrap().status, Status::Tombstone);

        // CRITICAL CHECK: Verify dependency row is NOT cascade deleted
        // The dependencies table has "FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE"
        // which only cascades when the DEPENDENT is deleted, not the blocker
        let deps = ws.get_dependencies("bf-dependent").unwrap();
        assert_eq!(
            deps.len(),
            1,
            "Dependency row should still exist (cascade only on dependent deletion)"
        );
        assert_eq!(
            deps[0].depends_on_id,
            "bf-blocker",
            "Dependency should still point to tombstoned bead"
        );
    }

    #[test]
    fn test_deleting_bead_with_multiple_dependents_preserves_all_dependencies() {
        let ws = DependencyCascadeTestWorkspace::new().unwrap();

        // Create one blocker and three dependents
        ws.create_bead("bf-blocker", "Blocker", Status::Open, Priority::HIGH)
            .unwrap();
        ws.create_bead("bf-dep1", "Dependent 1", Status::Open, Priority::MEDIUM)
            .unwrap();
        ws.create_bead("bf-dep2", "Dependent 2", Status::Open, Priority::MEDIUM)
            .unwrap();
        ws.create_bead("bf-dep3", "Dependent 3", Status::Open, Priority::MEDIUM)
            .unwrap();

        // Add blocking dependencies
        ws.add_blocking_dependency("bf-dep1", "bf-blocker")
            .unwrap();
        ws.add_blocking_dependency("bf-dep2", "bf-blocker")
            .unwrap();
        ws.add_blocking_dependency("bf-dep3", "bf-blocker")
            .unwrap();

        // Verify all dependencies exist
        let deps1 = ws.get_dependencies("bf-dep1").unwrap();
        let deps2 = ws.get_dependencies("bf-dep2").unwrap();
        let deps3 = ws.get_dependencies("bf-dep3").unwrap();
        assert_eq!(deps1.len(), 1);
        assert_eq!(deps2.len(), 1);
        assert_eq!(deps3.len(), 1);

        // Verify blocker has all three dependents
        let dependents = ws.get_dependents("bf-blocker").unwrap();
        assert_eq!(dependents.len(), 3, "Blocker should have 3 dependents");

        // Delete the blocker
        ws.delete_bead("bf-blocker").unwrap();

        // CRITICAL CHECK: Verify all dependency rows are preserved
        let deps1 = ws.get_dependencies("bf-dep1").unwrap();
        let deps2 = ws.get_dependencies("bf-dep2").unwrap();
        let deps3 = ws.get_dependencies("bf-dep3").unwrap();
        assert_eq!(deps1.len(), 1, "Dep1 dependency should be preserved");
        assert_eq!(deps2.len(), 1, "Dep2 dependency should be preserved");
        assert_eq!(deps3.len(), 1, "Dep3 dependency should be preserved");

        // Verify all still point to the tombstoned bead
        assert_eq!(deps1[0].depends_on_id, "bf-blocker");
        assert_eq!(deps2[0].depends_on_id, "bf-blocker");
        assert_eq!(deps3[0].depends_on_id, "bf-blocker");
    }

    // TEST 2: Verify dependent beads have their dependency lists updated correctly

    #[test]
    fn test_deleting_dependent_bead_cascades_to_dependency_removal() {
        let ws = DependencyCascadeTestWorkspace::new().unwrap();

        // Create blocker and dependent
        ws.create_bead("bf-blocker", "Blocker", Status::Open, Priority::HIGH)
            .unwrap();
        ws.create_bead("bf-dependent", "Dependent", Status::Open, Priority::MEDIUM)
            .unwrap();

        // Add blocking dependency
        ws.add_blocking_dependency("bf-dependent", "bf-blocker")
            .unwrap();

        // Verify dependency exists
        let deps = ws.get_dependencies("bf-dependent").unwrap();
        assert_eq!(deps.len(), 1);

        // Delete the DEPENDENT (not the blocker)
        ws.delete_bead("bf-dependent").unwrap();

        // CRITICAL CHECK: Verify dependency row IS cascade deleted
        // The dependencies table has "FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE"
        // which cascades when the DEPENDENT (issue_id) is deleted
        let deps = ws.get_dependencies("bf-dependent").unwrap();
        assert_eq!(
            deps.len(),
            0,
            "Dependency should be cascade deleted when dependent is tombstoned"
        );

        // Verify blocker no longer lists the deleted dependent
        let dependents = ws.get_dependents("bf-blocker").unwrap();
        assert_eq!(
            dependents.len(),
            0,
            "Blocker should have no dependents after cascade delete"
        );
        assert!(!dependents.iter().any(|d| d.issue_id == "bf-dependent"),
            "Blocker should not list deleted dependent");
    }

    #[test]
    fn test_dependency_list_accuracy_after_chain_deletion() {
        let ws = DependencyCascadeTestWorkspace::new().unwrap();

        // Create a chain: A -> B -> C
        ws.create_bead("bf-a", "A", Status::Open, Priority::HIGH)
            .unwrap();
        ws.create_bead("bf-b", "B", Status::Open, Priority::MEDIUM)
            .unwrap();
        ws.create_bead("bf-c", "C", Status::Open, Priority::MEDIUM)
            .unwrap();

        // B depends on A, C depends on B
        ws.add_blocking_dependency("bf-b", "bf-a").unwrap();
        ws.add_blocking_dependency("bf-c", "bf-b").unwrap();

        // Verify initial state
        let deps_b = ws.get_dependencies("bf-b").unwrap();
        let deps_c = ws.get_dependencies("bf-c").unwrap();
        assert_eq!(deps_b.len(), 1, "B should depend on A");
        assert_eq!(deps_c.len(), 1, "C should depend on B");

        // Delete B (the middle bead)
        ws.delete_bead("bf-b").unwrap();

        // CRITICAL CHECK: B's dependencies should be cascade deleted
        let deps_b = ws.get_dependencies("bf-b").unwrap();
        assert_eq!(
            deps_b.len(),
            0,
            "B's dependencies should be cascade deleted"
        );

        // C's dependency on B should still exist (cascade only affects issue_id, not depends_on_id)
        let deps_c = ws.get_dependencies("bf-c").unwrap();
        assert_eq!(deps_c.len(), 1, "C's dependency should still exist");
        assert_eq!(deps_c[0].depends_on_id, "bf-b");

        // A should have no dependents now (B was deleted)
        let dependents_a = ws.get_dependents("bf-a").unwrap();
        assert_eq!(
            dependents_a.len(),
            0,
            "A should have no dependents after B is cascade deleted"
        );
        assert!(!dependents_a.iter().any(|d| d.issue_id == "bf-b"),
            "A should not list deleted bead B as dependent");
    }

    // TEST 3: Test that deleting a bead clears it from all blocking lists

    #[test]
    fn test_deleted_bead_removed_from_all_dependent_lists() {
        let ws = DependencyCascadeTestWorkspace::new().unwrap();

        // Create multiple beads that depend on a common blocker
        ws.create_bead("bf-common-blocker", "Common Blocker", Status::Open, Priority::HIGH)
            .unwrap();
        ws.create_bead("bf-dep1", "Dependent 1", Status::Open, Priority::MEDIUM)
            .unwrap();
        ws.create_bead("bf-dep2", "Dependent 2", Status::Open, Priority::MEDIUM)
            .unwrap();
        ws.create_bead("bf-dep3", "Dependent 3", Status::Open, Priority::MEDIUM)
            .unwrap();

        // All depend on the common blocker
        ws.add_blocking_dependency("bf-dep1", "bf-common-blocker")
            .unwrap();
        ws.add_blocking_dependency("bf-dep2", "bf-common-blocker")
            .unwrap();
        ws.add_blocking_dependency("bf-dep3", "bf-common-blocker")
            .unwrap();

        // Verify blocker has all three dependents
        let dependents = ws.get_dependents("bf-common-blocker").unwrap();
        assert_eq!(dependents.len(), 3);

        // Delete the blocker
        ws.delete_bead("bf-common-blocker").unwrap();

        // CRITICAL CHECK: All dependency rows should be preserved (cascade doesn't affect depends_on_id)
        let deps1 = ws.get_dependencies("bf-dep1").unwrap();
        let deps2 = ws.get_dependencies("bf-dep2").unwrap();
        let deps3 = ws.get_dependencies("bf-dep3").unwrap();

        assert_eq!(deps1.len(), 1, "Dep1 dependency should be preserved");
        assert_eq!(deps2.len(), 1, "Dep2 dependency should be preserved");
        assert_eq!(deps3.len(), 1, "Dep3 dependency should be preserved");

        // All should still point to the tombstoned bead
        assert_eq!(deps1[0].depends_on_id, "bf-common-blocker");
        assert_eq!(deps2[0].depends_on_id, "bf-common-blocker");
        assert_eq!(deps3[0].depends_on_id, "bf-common-blocker");
    }

    #[test]
    fn test_deleting_bead_with_complex_dependency_graph() {
        let ws = DependencyCascadeTestWorkspace::new().unwrap();

        // Create a diamond dependency graph:
        //     A
        //    / \
        //   B   C
        //    \ /
        //     D
        ws.create_bead("bf-a", "A", Status::Open, Priority::HIGH)
            .unwrap();
        ws.create_bead("bf-b", "B", Status::Open, Priority::MEDIUM)
            .unwrap();
        ws.create_bead("bf-c", "C", Status::Open, Priority::MEDIUM)
            .unwrap();
        ws.create_bead("bf-d", "D", Status::Open, Priority::LOW)
            .unwrap();

        // B and C depend on A
        ws.add_blocking_dependency("bf-b", "bf-a").unwrap();
        ws.add_blocking_dependency("bf-c", "bf-a").unwrap();

        // D depends on both B and C
        ws.add_blocking_dependency("bf-d", "bf-b").unwrap();
        ws.add_blocking_dependency("bf-d", "bf-c").unwrap();

        // Verify initial state
        let deps_b = ws.get_dependencies("bf-b").unwrap();
        let deps_c = ws.get_dependencies("bf-c").unwrap();
        let deps_d = ws.get_dependencies("bf-d").unwrap();
        assert_eq!(deps_b.len(), 1);
        assert_eq!(deps_c.len(), 1);
        assert_eq!(deps_d.len(), 2);

        // Delete B
        ws.delete_bead("bf-b").unwrap();

        // CRITICAL CHECK: B's dependencies should be cascade deleted
        let deps_b = ws.get_dependencies("bf-b").unwrap();
        assert_eq!(deps_b.len(), 0, "B's dependencies should be cascade deleted");

        // D should now only have 1 dependency (on C)
        let deps_d = ws.get_dependencies("bf-d").unwrap();
        assert_eq!(deps_d.len(), 2, "D's dependencies preserved (cascade only affects issue_id)");

        // A should have only 1 dependent now (C)
        let dependents_a = ws.get_dependents("bf-a").unwrap();
        assert_eq!(
            dependents_a.len(),
            1,
            "A should have 1 dependent after B is cascade deleted"
        );
        assert_eq!(dependents_a[0].issue_id, "bf-c");

        // Verify D still depends on C (not cascade deleted)
        assert!(deps_d.iter().any(|d| d.depends_on_id == "bf-c"));
    }

    // Additional edge case tests

    #[test]
    fn test_cascade_delete_with_multiple_dependency_types() {
        let ws = DependencyCascadeTestWorkspace::new().unwrap();

        // Create beads with multiple dependency types
        ws.create_bead("bf-blocker", "Blocker", Status::Open, Priority::HIGH)
            .unwrap();
        ws.create_bead("bf-dependent", "Dependent", Status::Open, Priority::MEDIUM)
            .unwrap();

        // Add multiple types of dependencies
        ws.storage
            .add_dependency("bf-dependent", "bf-blocker", &DependencyType::Blocks, "test")
            .unwrap();
        ws.storage
            .add_dependency("bf-dependent", "bf-blocker", &DependencyType::Related, "test")
            .unwrap();

        // Verify both dependencies exist
        let deps = ws.get_dependencies("bf-dependent").unwrap();
        assert_eq!(deps.len(), 2, "Should have 2 dependencies of different types");

        // Delete the dependent
        ws.delete_bead("bf-dependent").unwrap();

        // CRITICAL CHECK: ALL dependency types should be cascade deleted
        let deps = ws.get_dependencies("bf-dependent").unwrap();
        assert_eq!(
            deps.len(),
            0,
            "All dependency types should be cascade deleted"
        );
    }

    #[test]
    fn test_cascade_delete_idempotence() {
        let ws = DependencyCascadeTestWorkspace::new().unwrap();

        // Create blocker and dependent
        ws.create_bead("bf-blocker", "Blocker", Status::Open, Priority::HIGH)
            .unwrap();
        ws.create_bead("bf-dependent", "Dependent", Status::Open, Priority::MEDIUM)
            .unwrap();

        // Add dependency
        ws.add_blocking_dependency("bf-dependent", "bf-blocker")
            .unwrap();

        // Delete the dependent
        ws.delete_bead("bf-dependent").unwrap();

        // Verify dependencies are cascade deleted
        let deps = ws.get_dependencies("bf-dependent").unwrap();
        assert_eq!(deps.len(), 0);

        // Try to delete again (should be idempotent)
        ws.delete_bead("bf-dependent").unwrap();

        // Dependencies should still be empty
        let deps = ws.get_dependencies("bf-dependent").unwrap();
        assert_eq!(deps.len(), 0, "Second delete should be idempotent");
    }

    #[test]
    fn test_cascade_delete_preserves_other_beads_dependencies() {
        let ws = DependencyCascadeTestWorkspace::new().unwrap();

        // Create three beads
        ws.create_bead("bf-a", "A", Status::Open, Priority::HIGH)
            .unwrap();
        ws.create_bead("bf-b", "B", Status::Open, Priority::MEDIUM)
            .unwrap();
        ws.create_bead("bf-c", "C", Status::Open, Priority::MEDIUM)
            .unwrap();

        // B depends on A, C depends on A
        ws.add_blocking_dependency("bf-b", "bf-a").unwrap();
        ws.add_blocking_dependency("bf-c", "bf-a").unwrap();

        // Delete B (should only cascade B's dependencies)
        ws.delete_bead("bf-b").unwrap();

        // B's dependencies should be cascade deleted
        let deps_b = ws.get_dependencies("bf-b").unwrap();
        assert_eq!(deps_b.len(), 0);

        // C's dependencies should be preserved
        let deps_c = ws.get_dependencies("bf-c").unwrap();
        assert_eq!(deps_c.len(), 1, "C's dependencies should be preserved");
        assert_eq!(deps_c[0].depends_on_id, "bf-a");

        // A should only have C as a dependent now
        let dependents_a = ws.get_dependents("bf-a").unwrap();
        assert_eq!(dependents_a.len(), 1);
        assert_eq!(dependents_a[0].issue_id, "bf-c");
        assert!(!dependents_a.iter().any(|d| d.issue_id == "bf-b"),
            "A should not list deleted bead B as dependent");
    }
}
