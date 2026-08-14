// Unit tests for ready queue filtering logic.
//
// Comprehensive tests ensuring that the ready queue correctly excludes:
// - Closed beads
// - Beads with unresolved dependencies (blocked beads)
// - Beads with unclosed blocking dependencies

#[cfg(test)]
mod ready_queue_filtering_tests {
    use super::super::*;
    use crate::model::{
        DependencyType, Issue, Priority, Status,
    };
    use chrono::{Duration, Utc};
    use tempfile::NamedTempFile;

    // ============================================================================
    // Status Filtering Tests
    // ============================================================================

    #[test]
    fn test_ready_queue_excludes_closed_beads() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an open bead
        let open_bead = Issue::new("bf-open".to_string(), "Open bead".to_string(), ".".to_string());
        storage.create_issue(&open_bead).unwrap();

        // Create a closed bead
        let mut closed_bead = Issue::new("bf-closed".to_string(), "Closed bead".to_string(), ".".to_string());
        closed_bead.status = Status::Closed;
        closed_bead.closed_at = Some(Utc::now());
        storage.create_issue(&closed_bead).unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Only the open bead should be ready
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-open");
        assert!(!ready.iter().any(|i| i.id == "bf-closed"));
    }

    #[test]
    fn test_ready_queue_excludes_tombstone_beads() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an open bead
        let open_bead = Issue::new("bf-open".to_string(), "Open bead".to_string(), ".".to_string());
        storage.create_issue(&open_bead).unwrap();

        // Create a tombstone bead
        let mut tombstone_bead = Issue::new("bf-tombstone".to_string(), "Tombstone bead".to_string(), ".".to_string());
        tombstone_bead.status = Status::Tombstone;
        storage.create_issue(&tombstone_bead).unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Only the open bead should be ready
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-open");
        assert!(!ready.iter().any(|i| i.id == "bf-tombstone"));
    }

    #[test]
    fn test_ready_queue_excludes_blocked_status_beads() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an open bead
        let open_bead = Issue::new("bf-open".to_string(), "Open bead".to_string(), ".".to_string());
        storage.create_issue(&open_bead).unwrap();

        // Create a blocked-status bead
        let mut blocked_bead = Issue::new("bf-blocked".to_string(), "Blocked bead".to_string(), ".".to_string());
        blocked_bead.status = Status::Blocked;
        storage.create_issue(&blocked_bead).unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Only the open bead should be ready
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-open");
        assert!(!ready.iter().any(|i| i.id == "bf-blocked"));
    }

    #[test]
    fn test_ready_queue_excludes_in_progress_beads() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an open bead
        let open_bead = Issue::new("bf-open".to_string(), "Open bead".to_string(), ".".to_string());
        storage.create_issue(&open_bead).unwrap();

        // Create an in_progress bead
        let mut in_progress_bead = Issue::new("bf-inprogress".to_string(), "In progress bead".to_string(), ".".to_string());
        in_progress_bead.status = Status::InProgress;
        storage.create_issue(&in_progress_bead).unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Only the open bead should be ready
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-open");
        assert!(!ready.iter().any(|i| i.id == "bf-inprogress"));
    }

    #[test]
    fn test_ready_queue_excludes_deferred_beads() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an open bead
        let open_bead = Issue::new("bf-open".to_string(), "Open bead".to_string(), ".".to_string());
        storage.create_issue(&open_bead).unwrap();

        // Create a deferred bead
        let mut deferred_bead = Issue::new("bf-deferred".to_string(), "Deferred bead".to_string(), ".".to_string());
        deferred_bead.status = Status::Deferred;
        storage.create_issue(&deferred_bead).unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Only the open bead should be ready
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-open");
        assert!(!ready.iter().any(|i| i.id == "bf-deferred"));
    }

    #[test]
    fn test_ready_queue_excludes_draft_beads() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an open bead
        let open_bead = Issue::new("bf-open".to_string(), "Open bead".to_string(), ".".to_string());
        storage.create_issue(&open_bead).unwrap();

        // Create a draft bead
        let mut draft_bead = Issue::new("bf-draft".to_string(), "Draft bead".to_string(), ".".to_string());
        draft_bead.status = Status::Draft;
        storage.create_issue(&draft_bead).unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Only the open bead should be ready
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-open");
        assert!(!ready.iter().any(|i| i.id == "bf-draft"));
    }

    #[test]
    fn test_ready_queue_excludes_custom_status_done_beads() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an open bead
        let open_bead = Issue::new("bf-open".to_string(), "Open bead".to_string(), ".".to_string());
        storage.create_issue(&open_bead).unwrap();

        // Create a custom "done" status bead (terminal alias)
        let mut done_bead = Issue::new("bf-done".to_string(), "Done bead".to_string(), ".".to_string());
        done_bead.status = Status::Custom("done".to_string());
        storage.create_issue(&done_bead).unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Only the open bead should be ready
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-open");
        assert!(!ready.iter().any(|i| i.id == "bf-done"));
    }

    #[test]
    fn test_ready_queue_excludes_custom_status_completed_beads() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an open bead
        let open_bead = Issue::new("bf-open".to_string(), "Open bead".to_string(), ".".to_string());
        storage.create_issue(&open_bead).unwrap();

        // Create a custom "completed" status bead (terminal alias)
        let mut completed_bead = Issue::new("bf-completed".to_string(), "Completed bead".to_string(), ".".to_string());
        completed_bead.status = Status::Custom("completed".to_string());
        storage.create_issue(&completed_bead).unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Only the open bead should be ready
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-open");
        assert!(!ready.iter().any(|i| i.id == "bf-completed"));
    }

    // ============================================================================
    // Dependency Filtering Tests
    // ============================================================================

    #[test]
    fn test_ready_queue_excludes_beads_with_open_blockers() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a blocker (open) and dependent (open with dependency)
        let blocker = Issue::new("bf-blocker".to_string(), "Blocker".to_string(), ".".to_string());
        storage.create_issue(&blocker).unwrap();

        let dependent = Issue::new("bf-dependent".to_string(), "Dependent".to_string(), ".".to_string());
        storage.create_issue(&dependent).unwrap();

        // Add blocking dependency
        storage.add_dependency("bf-dependent", "bf-blocker", &DependencyType::Blocks, "test-user").unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Only the blocker should be ready (not the dependent)
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-blocker");
        assert!(!ready.iter().any(|i| i.id == "bf-dependent"));
    }

    #[test]
    fn test_ready_queue_includes_beads_with_closed_blockers() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a closed blocker and dependent
        let mut blocker = Issue::new("bf-blocker".to_string(), "Closed blocker".to_string(), ".".to_string());
        blocker.status = Status::Closed;
        blocker.closed_at = Some(Utc::now());
        storage.create_issue(&blocker).unwrap();

        let dependent = Issue::new("bf-dependent".to_string(), "Dependent with closed blocker".to_string(), ".".to_string());
        storage.create_issue(&dependent).unwrap();

        // Add blocking dependency
        storage.add_dependency("bf-dependent", "bf-blocker", &DependencyType::Blocks, "test-user").unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // The dependent should be ready since its blocker is closed
        assert!(ready.iter().any(|i| i.id == "bf-dependent"));
    }

    #[test]
    fn test_ready_queue_includes_beads_with_tombstone_blockers() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a tombstone blocker and dependent
        let mut blocker = Issue::new("bf-blocker".to_string(), "Tombstone blocker".to_string(), ".".to_string());
        blocker.status = Status::Tombstone;
        storage.create_issue(&blocker).unwrap();

        let dependent = Issue::new("bf-dependent".to_string(), "Dependent with tombstone blocker".to_string(), ".".to_string());
        storage.create_issue(&dependent).unwrap();

        // Add blocking dependency
        storage.add_dependency("bf-dependent", "bf-blocker", &DependencyType::Blocks, "test-user").unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // The dependent should be ready since its blocker is tombstone (terminal)
        assert!(ready.iter().any(|i| i.id == "bf-dependent"));
    }

    #[test]
    fn test_ready_queue_includes_beads_with_done_blockers() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a "done" blocker and dependent
        let mut blocker = Issue::new("bf-blocker".to_string(), "Done blocker".to_string(), ".".to_string());
        blocker.status = Status::Custom("done".to_string());
        storage.create_issue(&blocker).unwrap();

        let dependent = Issue::new("bf-dependent".to_string(), "Dependent with done blocker".to_string(), ".".to_string());
        storage.create_issue(&dependent).unwrap();

        // Add blocking dependency
        storage.add_dependency("bf-dependent", "bf-blocker", &DependencyType::Blocks, "test-user").unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // The dependent should be ready since its blocker is "done" (terminal)
        assert!(ready.iter().any(|i| i.id == "bf-dependent"));
    }

    #[test]
    fn test_ready_queue_includes_beads_with_completed_blockers() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a "completed" blocker and dependent
        let mut blocker = Issue::new("bf-blocker".to_string(), "Completed blocker".to_string(), ".".to_string());
        blocker.status = Status::Custom("completed".to_string());
        storage.create_issue(&blocker).unwrap();

        let dependent = Issue::new("bf-dependent".to_string(), "Dependent with completed blocker".to_string(), ".".to_string());
        storage.create_issue(&dependent).unwrap();

        // Add blocking dependency
        storage.add_dependency("bf-dependent", "bf-blocker", &DependencyType::Blocks, "test-user").unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // The dependent should be ready since its blocker is "completed" (terminal)
        assert!(ready.iter().any(|i| i.id == "bf-dependent"));
    }

    #[test]
    fn test_ready_queue_excludes_beads_with_multiple_open_blockers() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create two open blockers and a dependent
        let blocker1 = Issue::new("bf-blocker1".to_string(), "Blocker 1".to_string(), ".".to_string());
        storage.create_issue(&blocker1).unwrap();

        let blocker2 = Issue::new("bf-blocker2".to_string(), "Blocker 2".to_string(), ".".to_string());
        storage.create_issue(&blocker2).unwrap();

        let dependent = Issue::new("bf-dependent".to_string(), "Dependent".to_string(), ".".to_string());
        storage.create_issue(&dependent).unwrap();

        // Add blocking dependencies
        storage.add_dependency("bf-dependent", "bf-blocker1", &DependencyType::Blocks, "test-user").unwrap();
        storage.add_dependency("bf-dependent", "bf-blocker2", &DependencyType::Blocks, "test-user").unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Only the two blockers should be ready (not the dependent)
        assert_eq!(ready.len(), 2);
        assert!(!ready.iter().any(|i| i.id == "bf-dependent"));
    }

    #[test]
    fn test_ready_queue_excludes_beads_with_at_least_one_open_blocker() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create one open blocker and one closed blocker
        let open_blocker = Issue::new("bf-open-blocker".to_string(), "Open blocker".to_string(), ".".to_string());
        storage.create_issue(&open_blocker).unwrap();

        let mut closed_blocker = Issue::new("bf-closed-blocker".to_string(), "Closed blocker".to_string(), ".".to_string());
        closed_blocker.status = Status::Closed;
        closed_blocker.closed_at = Some(Utc::now());
        storage.create_issue(&closed_blocker).unwrap();

        let dependent = Issue::new("bf-dependent".to_string(), "Dependent".to_string(), ".".to_string());
        storage.create_issue(&dependent).unwrap();

        // Add blocking dependencies
        storage.add_dependency("bf-dependent", "bf-open-blocker", &DependencyType::Blocks, "test-user").unwrap();
        storage.add_dependency("bf-dependent", "bf-closed-blocker", &DependencyType::Blocks, "test-user").unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // The dependent should NOT be ready (one blocker is still open)
        assert!(!ready.iter().any(|i| i.id == "bf-dependent"));
    }

    #[test]
    fn test_ready_queue_includes_beads_when_all_blockers_closed() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create two closed blockers
        let mut blocker1 = Issue::new("bf-blocker1".to_string(), "Closed blocker 1".to_string(), ".".to_string());
        blocker1.status = Status::Closed;
        blocker1.closed_at = Some(Utc::now());
        storage.create_issue(&blocker1).unwrap();

        let mut blocker2 = Issue::new("bf-blocker2".to_string(), "Closed blocker 2".to_string(), ".".to_string());
        blocker2.status = Status::Closed;
        blocker2.closed_at = Some(Utc::now());
        storage.create_issue(&blocker2).unwrap();

        let dependent = Issue::new("bf-dependent".to_string(), "Dependent".to_string(), ".".to_string());
        storage.create_issue(&dependent).unwrap();

        // Add blocking dependencies
        storage.add_dependency("bf-dependent", "bf-blocker1", &DependencyType::Blocks, "test-user").unwrap();
        storage.add_dependency("bf-dependent", "bf-blocker2", &DependencyType::Blocks, "test-user").unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // The dependent should be ready (all blockers are closed)
        assert!(ready.iter().any(|i| i.id == "bf-dependent"));
    }

    // ============================================================================
    // Dependency Type Tests
    // ============================================================================

    #[test]
    fn test_ready_queue_respects_blocks_dependency_type() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a blocker and dependent
        let blocker = Issue::new("bf-blocker".to_string(), "Blocker".to_string(), ".".to_string());
        storage.create_issue(&blocker).unwrap();

        let dependent = Issue::new("bf-dependent".to_string(), "Dependent".to_string(), ".".to_string());
        storage.create_issue(&dependent).unwrap();

        // Add blocks dependency
        storage.add_dependency("bf-dependent", "bf-blocker", &DependencyType::Blocks, "test-user").unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Dependent should NOT be ready
        assert!(!ready.iter().any(|i| i.id == "bf-dependent"));
    }

    #[test]
    fn test_ready_queue_respects_parent_child_dependency_type() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a parent and child
        let parent = Issue::new("bf-parent".to_string(), "Parent".to_string(), ".".to_string());
        storage.create_issue(&parent).unwrap();

        let child = Issue::new("bf-child".to_string(), "Child".to_string(), ".".to_string());
        storage.create_issue(&child).unwrap();

        // Add parent-child dependency
        storage.add_dependency("bf-child", "bf-parent", &DependencyType::ParentChild, "test-user").unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Child should NOT be ready (parent is still open)
        assert!(!ready.iter().any(|i| i.id == "bf-child"));
    }

    #[test]
    fn test_ready_queue_respects_conditional_blocks_dependency_type() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a blocker and dependent
        let blocker = Issue::new("bf-blocker".to_string(), "Blocker".to_string(), ".".to_string());
        storage.create_issue(&blocker).unwrap();

        let dependent = Issue::new("bf-dependent".to_string(), "Dependent".to_string(), ".".to_string());
        storage.create_issue(&dependent).unwrap();

        // Add conditional-blocks dependency
        storage.add_dependency("bf-dependent", "bf-blocker", &DependencyType::ConditionalBlocks, "test-user").unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Dependent should NOT be ready
        assert!(!ready.iter().any(|i| i.id == "bf-dependent"));
    }

    #[test]
    fn test_ready_queue_respects_waits_for_dependency_type() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a waiter and waited-for
        let waited_for = Issue::new("bf-waited-for".to_string(), "Waited for".to_string(), ".".to_string());
        storage.create_issue(&waited_for).unwrap();

        let waiter = Issue::new("bf-waiter".to_string(), "Waiter".to_string(), ".".to_string());
        storage.create_issue(&waiter).unwrap();

        // Add waits-for dependency
        storage.add_dependency("bf-waiter", "bf-waited-for", &DependencyType::WaitsFor, "test-user").unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Waiter should NOT be ready
        assert!(!ready.iter().any(|i| i.id == "bf-waiter"));
    }

    #[test]
    fn test_ready_queue_ignores_non_blocking_dependency_types() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create two beads with a non-blocking dependency
        let bead1 = Issue::new("bf-related".to_string(), "Related bead".to_string(), ".".to_string());
        storage.create_issue(&bead1).unwrap();

        let bead2 = Issue::new("bf-dependent".to_string(), "Dependent".to_string(), ".".to_string());
        storage.create_issue(&bead2).unwrap();

        // Add a non-blocking dependency (e.g., "relates-to")
        storage.add_dependency("bf-dependent", "bf-related", &DependencyType::RelatesTo, "test-user").unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Both beads should be ready (relates-to is not a blocking dependency)
        assert_eq!(ready.len(), 2);
        assert!(ready.iter().any(|i| i.id == "bf-related"));
        assert!(ready.iter().any(|i| i.id == "bf-dependent"));
    }

    #[test]
    fn test_ready_queue_ignores_related_dependency() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let bead1 = Issue::new("bf-related".to_string(), "Related bead".to_string(), ".".to_string());
        storage.create_issue(&bead1).unwrap();

        let bead2 = Issue::new("bf-dependent".to_string(), "Dependent".to_string(), ".".to_string());
        storage.create_issue(&bead2).unwrap();

        storage.add_dependency("bf-dependent", "bf-related", &DependencyType::Related, "test-user").unwrap();

        let ready = storage.get_ready_candidates().unwrap();
        assert_eq!(ready.len(), 2);
    }

    #[test]
    fn test_ready_queue_ignores_discovered_from_dependency() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let bead1 = Issue::new("bf-source".to_string(), "Source bead".to_string(), ".".to_string());
        storage.create_issue(&bead1).unwrap();

        let bead2 = Issue::new("bf-discovered".to_string(), "Discovered bead".to_string(), ".".to_string());
        storage.create_issue(&bead2).unwrap();

        storage.add_dependency("bf-discovered", "bf-source", &DependencyType::DiscoveredFrom, "test-user").unwrap();

        let ready = storage.get_ready_candidates().unwrap();
        assert_eq!(ready.len(), 2);
    }

    // ============================================================================
    // Complex Scenario Tests
    // ============================================================================

    #[test]
    fn test_ready_queue_with_transitive_dependencies() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a dependency chain: A -> B -> C
        let bead_a = Issue::new("bf-a".to_string(), "Bead A".to_string(), ".".to_string());
        storage.create_issue(&bead_a).unwrap();

        let bead_b = Issue::new("bf-b".to_string(), "Bead B".to_string(), ".".to_string());
        storage.create_issue(&bead_b).unwrap();

        let bead_c = Issue::new("bf-c".to_string(), "Bead C".to_string(), ".".to_string());
        storage.create_issue(&bead_c).unwrap();

        // B depends on A
        storage.add_dependency("bf-b", "bf-a", &DependencyType::Blocks, "test-user").unwrap();
        // C depends on B
        storage.add_dependency("bf-c", "bf-b", &DependencyType::Blocks, "test-user").unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Only A should be ready (B is blocked by A, C is blocked by B)
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-a");
    }

    #[test]
    fn test_ready_queue_with_diamond_dependencies() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a diamond: A -> B, A -> C, B -> D, C -> D
        let bead_a = Issue::new("bf-a".to_string(), "Bead A".to_string(), ".".to_string());
        storage.create_issue(&bead_a).unwrap();

        let bead_b = Issue::new("bf-b".to_string(), "Bead B".to_string(), ".".to_string());
        storage.create_issue(&bead_b).unwrap();

        let bead_c = Issue::new("bf-c".to_string(), "Bead C".to_string(), ".".to_string());
        storage.create_issue(&bead_c).unwrap();

        let bead_d = Issue::new("bf-d".to_string(), "Bead D".to_string(), ".".to_string());
        storage.create_issue(&bead_d).unwrap();

        // B and C depend on A
        storage.add_dependency("bf-b", "bf-a", &DependencyType::Blocks, "test-user").unwrap();
        storage.add_dependency("bf-c", "bf-a", &DependencyType::Blocks, "test-user").unwrap();
        // D depends on both B and C
        storage.add_dependency("bf-d", "bf-b", &DependencyType::Blocks, "test-user").unwrap();
        storage.add_dependency("bf-d", "bf-c", &DependencyType::Blocks, "test-user").unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Only A should be ready (B and C are blocked by A, D is blocked by B and C)
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-a");
    }

    #[test]
    fn test_ready_queue_excludes_ephemeral_beads() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an open bead
        let open_bead = Issue::new("bf-open".to_string(), "Open bead".to_string(), ".".to_string());
        storage.create_issue(&open_bead).unwrap();

        // Create an ephemeral bead
        let mut ephemeral_bead = Issue::new("bf-ephemeral".to_string(), "Ephemeral bead".to_string(), ".".to_string());
        ephemeral_bead.ephemeral = true;
        storage.create_issue(&ephemeral_bead).unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Only the non-ephemeral bead should be ready
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-open");
        assert!(!ready.iter().any(|i| i.id == "bf-ephemeral"));
    }

    #[test]
    fn test_ready_queue_excludes_pinned_beads() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an open bead
        let open_bead = Issue::new("bf-open".to_string(), "Open bead".to_string(), ".".to_string());
        storage.create_issue(&open_bead).unwrap();

        // Create a pinned bead
        let mut pinned_bead = Issue::new("bf-pinned".to_string(), "Pinned bead".to_string(), ".".to_string());
        pinned_bead.pinned = true;
        storage.create_issue(&pinned_bead).unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Only the non-pinned bead should be ready
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-open");
        assert!(!ready.iter().any(|i| i.id == "bf-pinned"));
    }

    #[test]
    fn test_ready_queue_excludes_template_beads() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an open bead
        let open_bead = Issue::new("bf-open".to_string(), "Open bead".to_string(), ".".to_string());
        storage.create_issue(&open_bead).unwrap();

        // Create a template bead
        let mut template_bead = Issue::new("bf-template".to_string(), "Template bead".to_string(), ".".to_string());
        template_bead.is_template = true;
        storage.create_issue(&template_bead).unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Only the non-template bead should be ready
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-open");
        assert!(!ready.iter().any(|i| i.id == "bf-template"));
    }

    #[test]
    fn test_ready_queue_excludes_deleted_beads() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an open bead
        let open_bead = Issue::new("bf-open".to_string(), "Open bead".to_string(), ".".to_string());
        storage.create_issue(&open_bead).unwrap();

        // Create a deleted (tombstone) bead
        let mut deleted_bead = Issue::new("bf-deleted".to_string(), "Deleted bead".to_string(), ".".to_string());
        deleted_bead.status = Status::Tombstone;
        deleted_bead.deleted_at = Some(Utc::now());
        deleted_bead.deleted_by = Some("test-user".to_string());
        deleted_bead.delete_reason = Some("Test deletion".to_string());
        storage.create_issue(&deleted_bead).unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Only the non-deleted bead should be ready
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-open");
        assert!(!ready.iter().any(|i| i.id == "bf-deleted"));
    }

    // ============================================================================
    // Edge Cases
    // ============================================================================

    #[test]
    fn test_ready_queue_empty_when_no_open_beads() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create only closed beads
        for i in 1..=5 {
            let mut bead = Issue::new(format!("bf-closed-{}", i), format!("Closed bead {}", i), ".".to_string());
            bead.status = Status::Closed;
            bead.closed_at = Some(Utc::now());
            storage.create_issue(&bead).unwrap();
        }

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // No beads should be ready
        assert_eq!(ready.len(), 0);
    }

    #[test]
    fn test_ready_queue_with_assigned_open_beads() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an open, assigned bead
        let mut assigned_bead = Issue::new("bf-assigned".to_string(), "Assigned bead".to_string(), ".".to_string());
        assigned_bead.assignee = Some("worker1".to_string());
        storage.create_issue(&assigned_bead).unwrap();

        // Create an open, unassigned bead
        let unassigned_bead = Issue::new("bf-unassigned".to_string(), "Unassigned bead".to_string(), ".".to_string());
        storage.create_issue(&unassigned_bead).unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Both should be ready (assignment doesn't affect readiness)
        assert_eq!(ready.len(), 2);
        assert!(ready.iter().any(|i| i.id == "bf-assigned"));
        assert!(ready.iter().any(|i| i.id == "bf-unassigned"));
    }

    #[test]
    fn test_ready_queue_ordering_by_priority_and_created_at() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create beads with different priorities and timestamps
        let base_time = Utc::now();

        let mut bead1 = Issue::new("bf-p0".to_string(), "P0 bead".to_string(), ".".to_string());
        bead1.priority = Priority::CRITICAL;
        bead1.created_at = base_time + Duration::seconds(3);
        storage.create_issue(&bead1).unwrap();

        let mut bead2 = Issue::new("bf-p0-old".to_string(), "P0 older bead".to_string(), ".".to_string());
        bead2.priority = Priority::CRITICAL;
        bead2.created_at = base_time + Duration::seconds(1);
        storage.create_issue(&bead2).unwrap();

        let mut bead3 = Issue::new("bf-p1".to_string(), "P1 bead".to_string(), ".".to_string());
        bead3.priority = Priority::HIGH;
        bead3.created_at = base_time + Duration::seconds(2);
        storage.create_issue(&bead3).unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Should be ordered by priority ASC, then created_at ASC
        assert_eq!(ready.len(), 3);
        assert_eq!(ready[0].id, "bf-p0-old"); // Same priority as bead1, but older
        assert_eq!(ready[1].id, "bf-p0");    // Higher priority than bead3
        assert_eq!(ready[2].id, "bf-p1");    // Lower priority
    }

    #[test]
    fn test_ready_queue_handles_beads_with_no_dependencies() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create multiple open beads with no dependencies
        for i in 1..=10 {
            let bead = Issue::new(format!("bf-ready-{}", i), format!("Ready bead {}", i), ".".to_string());
            storage.create_issue(&bead).unwrap();
        }

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // All should be ready
        assert_eq!(ready.len(), 10);
    }

    #[test]
    fn test_ready_queue_after_closing_blocker() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create blocker and dependent
        let blocker = Issue::new("bf-blocker".to_string(), "Blocker".to_string(), ".".to_string());
        storage.create_issue(&blocker).unwrap();

        let dependent = Issue::new("bf-dependent".to_string(), "Dependent".to_string(), ".".to_string());
        storage.create_issue(&dependent).unwrap();

        storage.add_dependency("bf-dependent", "bf-blocker", &DependencyType::Blocks, "test-user").unwrap();

        // Initially, dependent should not be ready
        let ready_before = storage.get_ready_candidates().unwrap();
        assert!(!ready_before.iter().any(|i| i.id == "bf-dependent"));

        // Close the blocker
        storage.close_issue("bf-blocker", "Completed", "test-user").unwrap();

        // Now dependent should be ready
        let ready_after = storage.get_ready_candidates().unwrap();
        assert!(ready_after.iter().any(|i| i.id == "bf-dependent"));
    }

    #[test]
    fn test_ready_queue_self_dependency_excluded() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a bead that attempts to block itself (should be prevented by add_dependency)
        let bead = Issue::new("bf-self".to_string(), "Self bead".to_string(), ".".to_string());
        storage.create_issue(&bead).unwrap();

        // Attempt to add self-blocking dependency (should fail)
        let result = storage.add_dependency("bf-self", "bf-self", &DependencyType::Blocks, "test-user");
        assert!(result.is_err(), "Self-blocking dependency should be rejected");

        // The bead should still be ready
        let ready = storage.get_ready_candidates().unwrap();
        assert!(ready.iter().any(|i| i.id == "bf-self"));
    }

    // ============================================================================
    // Downstream Impact Ranking Tests
    // ============================================================================

    #[test]
    fn test_ready_queue_downstream_impact_ranking_basic() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create beads with different downstream impacts
        // bead_a blocks 3 beads, bead_b blocks 1 bead, bead_c blocks 0
        let bead_a = Issue::new("bf-a".to_string(), "Bead A (blocks 3)".to_string(), ".".to_string());
        storage.create_issue(&bead_a).unwrap();

        let bead_b = Issue::new("bf-b".to_string(), "Bead B (blocks 1)".to_string(), ".".to_string());
        storage.create_issue(&bead_b).unwrap();

        let bead_c = Issue::new("bf-c".to_string(), "Bead C (blocks 0)".to_string(), ".".to_string());
        storage.create_issue(&bead_c).unwrap();

        // Beads blocked by A
        let dep_a1 = Issue::new("bf-dep-a1".to_string(), "Dep A1".to_string(), ".".to_string());
        let dep_a2 = Issue::new("bf-dep-a2".to_string(), "Dep A2".to_string(), ".".to_string());
        let dep_a3 = Issue::new("bf-dep-a3".to_string(), "Dep A3".to_string(), ".".to_string());
        storage.create_issue(&dep_a1).unwrap();
        storage.create_issue(&dep_a2).unwrap();
        storage.create_issue(&dep_a3).unwrap();

        // Bead blocked by B
        let dep_b1 = Issue::new("bf-dep-b1".to_string(), "Dep B1".to_string(), ".".to_string());
        storage.create_issue(&dep_b1).unwrap();

        // Add dependencies
        storage.add_dependency("bf-dep-a1", "bf-a", &DependencyType::Blocks, "test").unwrap();
        storage.add_dependency("bf-dep-a2", "bf-a", &DependencyType::Blocks, "test").unwrap();
        storage.add_dependency("bf-dep-a3", "bf-a", &DependencyType::Blocks, "test").unwrap();
        storage.add_dependency("bf-dep-b1", "bf-b", &DependencyType::Blocks, "test").unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // All three unblocked beads should be present
        assert_eq!(ready.len(), 3, "All three unblocked beads should be ready");

        // Note: Storage::get_ready_candidates returns Vec<Issue> ordered by priority ASC, created_at ASC
        // Downstream impact ranking is only implemented in claim::get_ready_candidates (Vec<ScoredBead>)
        // Here we just verify all three beads are present and accessible
        assert!(ready.iter().any(|i| i.id == "bf-a"));
        assert!(ready.iter().any(|i| i.id == "bf-b"));
        assert!(ready.iter().any(|i| i.id == "bf-c"));
    }

    #[test]
    fn test_ready_queue_downstream_impact_with_priority_tiebreaker() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create two beads with same priority
        let mut bead_high_impact = Issue::new("bf-high".to_string(), "High impact".to_string(), ".".to_string());
        bead_high_impact.priority = Priority::HIGH;
        storage.create_issue(&bead_high_impact).unwrap();

        let mut bead_low_impact = Issue::new("bf-low".to_string(), "Low impact".to_string(), ".".to_string());
        bead_low_impact.priority = Priority::HIGH;
        storage.create_issue(&bead_low_impact).unwrap();

        // Add dependencies to create different impacts
        let dep1 = Issue::new("bf-dep1".to_string(), "Dep 1".to_string(), ".".to_string());
        let dep2 = Issue::new("bf-dep2".to_string(), "Dep 2".to_string(), ".".to_string());
        storage.create_issue(&dep1).unwrap();
        storage.create_issue(&dep2).unwrap();

        storage.add_dependency("bf-dep1", "bf-high", &DependencyType::Blocks, "test").unwrap();
        storage.add_dependency("bf-dep2", "bf-high", &DependencyType::Blocks, "test").unwrap();

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Both should be present (they have the same priority so are ordered by created_at)
        assert_eq!(ready.len(), 2);
        assert!(ready.iter().any(|i| i.id == "bf-high"));
        assert!(ready.iter().any(|i| i.id == "bf-low"));
    }

    #[test]
    fn test_ready_queue_downstream_impact_with_mixed_priorities() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create beads with mixed priorities and impacts
        // P0 with 0 impact vs P1 with 10 impact - priority should win
        let mut p0_low_impact = Issue::new("bf-p0".to_string(), "P0 low impact".to_string(), ".".to_string());
        p0_low_impact.priority = Priority::CRITICAL;
        storage.create_issue(&p0_low_impact).unwrap();

        let mut p1_high_impact = Issue::new("bf-p1".to_string(), "P1 high impact".to_string(), ".".to_string());
        p1_high_impact.priority = Priority::HIGH;
        storage.create_issue(&p1_high_impact).unwrap();

        // Give P1 many dependencies
        for i in 1..=5 {
            let dep = Issue::new(format!("bf-dep-{}", i), "Dep".to_string(), ".".to_string());
            storage.create_issue(&dep).unwrap();
            storage.add_dependency(&format!("bf-dep-{}", i), "bf-p1", &DependencyType::Blocks, "test").unwrap();
        }

        // Get ready candidates
        let ready = storage.get_ready_candidates().unwrap();

        // Both should be present
        assert_eq!(ready.len(), 2);

        // P0 should come first (lower priority number = higher priority)
        assert_eq!(ready[0].id, "bf-p0", "P0 should be first due to higher priority");
        assert_eq!(ready[1].id, "bf-p1");
    }
}
