//! Core ready queue tests
//!
//! Integration tests verifying the ready queue correctly:
//! - Shows open, unblocked beads
//! - Ranks by priority (P0 first, then P1, etc.)
//! - Ranks by age when priorities are equal
//! - Excludes blocked beads
//! - Excludes in_progress beads

use bead_forge::claim::get_ready_candidates;
use bead_forge::model::{DependencyType, Issue, IssueChanges, Priority, Status};
use bead_forge::storage::Storage;
use chrono::{Duration, TimeZone, Utc};
use tempfile::NamedTempFile;

/// Create a test database and storage instance
fn setup_test_db() -> (NamedTempFile, Storage) {
    let temp_file = NamedTempFile::new().unwrap();
    let storage = Storage::open(temp_file.path()).unwrap();
    (temp_file, storage)
}

/// Create a basic open bead with optional priority
fn create_open_bead(
    storage: &Storage,
    id: &str,
    title: &str,
    priority: Priority,
) -> Issue {
    let mut bead = Issue::new(id.to_string(), title.to_string(), ".".to_string());
    bead.priority = priority;
    storage.create_issue(&bead).unwrap();
    bead
}

#[cfg(test)]
mod core_ready_queue_tests {
    use super::*;

    // ============================================================================
    // Basic Inclusion Tests
    // ============================================================================

    #[test]
    fn test_open_unblocked_beads_appear_in_ready_queue() {
        let (_temp, storage) = setup_test_db();

        // Create multiple open, unblocked beads
        create_open_bead(&storage, "bf-ready-1", "Ready bead 1", Priority::MEDIUM);
        create_open_bead(&storage, "bf-ready-2", "Ready bead 2", Priority::HIGH);
        create_open_bead(&storage, "bf-ready-3", "Ready bead 3", Priority::CRITICAL);

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
            .unwrap();

        // All three beads should appear in ready queue
        assert_eq!(ready.len(), 3);
        assert!(ready.iter().any(|b| b.id == "bf-ready-1"));
        assert!(ready.iter().any(|b| b.id == "bf-ready-2"));
        assert!(ready.iter().any(|b| b.id == "bf-ready-3"));
    }

    #[test]
    fn test_ready_queue_excludes_blocked_beads() {
        let (_temp, storage) = setup_test_db();

        // Create a blocker and a dependent
        let blocker = create_open_bead(&storage, "bf-blocker", "Blocker", Priority::MEDIUM);
        let dependent =
            create_open_bead(&storage, "bf-dependent", "Dependent", Priority::MEDIUM);

        // Add blocking dependency
        storage
            .add_dependency(
                "bf-dependent",
                "bf-blocker",
                &DependencyType::Blocks,
                "test-user",
            )
            .unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
            .unwrap();

        // Only the blocker should be ready
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-blocker");
        assert!(!ready.iter().any(|b| b.id == "bf-dependent"));
    }

    #[test]
    fn test_ready_queue_excludes_in_progress_beads() {
        let (_temp, storage) = setup_test_db();

        // Create an in_progress bead
        let mut in_progress_bead = Issue::new(
            "bf-inprogress".to_string(),
            "In progress bead".to_string(),
            ".".to_string(),
        );
        in_progress_bead.status = Status::InProgress;
        in_progress_bead.assignee = Some("worker1".to_string());
        storage.create_issue(&in_progress_bead).unwrap();

        // Create an open bead
        let open_bead =
            create_open_bead(&storage, "bf-open", "Open bead", Priority::MEDIUM);

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
            .unwrap();

        // Only the open bead should be ready
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-open");
        assert!(!ready.iter().any(|b| b.id == "bf-inprogress"));
    }

    // ============================================================================
    // Priority Ordering Tests
    // ============================================================================

    #[test]
    fn test_priority_ranking_p0_before_p1_before_p2() {
        let (_temp, storage) = setup_test_db();

        let base_time = Utc::now();

        // Create beads with different priorities (newer P1, older P0)
        let mut p1_bead = Issue::new("bf-p1".to_string(), "P1 bead".to_string(), ".".to_string());
        p1_bead.priority = Priority::HIGH; // P1
        p1_bead.created_at = base_time - Duration::seconds(100); // Older
        storage.create_issue(&p1_bead).unwrap();

        let mut p0_bead =
            Issue::new("bf-p0".to_string(), "P0 bead".to_string(), ".".to_string());
        p0_bead.priority = Priority::CRITICAL; // P0
        p0_bead.created_at = base_time - Duration::seconds(50); // Newer
        storage.create_issue(&p0_bead).unwrap();

        let mut p2_bead = Issue::new("bf-p2".to_string(), "P2 bead".to_string(), ".".to_string());
        p2_bead.priority = Priority::MEDIUM; // P2
        p2_bead.created_at = base_time - Duration::seconds(150); // Oldest
        storage.create_issue(&p2_bead).unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
            .unwrap();

        // Should be ordered: P0, P1, P2 (priority trumps age)
        assert_eq!(ready.len(), 3);
        assert_eq!(ready[0].id, "bf-p0"); // P0 first (highest priority)
        assert_eq!(ready[1].id, "bf-p1"); // P1 second
        assert_eq!(ready[2].id, "bf-p2"); // P2 last
    }

    #[test]
    fn test_priority_ranking_all_levels() {
        let (_temp, storage) = setup_test_db();

        let base_time = Utc::now();

        // Create beads at all priority levels (newest to oldest: P4, P3, P2, P1, P0)
        let priorities = vec![
            ("bf-p4", Priority::BACKLOG, 200),   // P4, newest
            ("bf-p3", Priority::LOW, 150),        // P3
            ("bf-p2", Priority::MEDIUM, 100),     // P2
            ("bf-p1", Priority::HIGH, 50),        // P1
            ("bf-p0", Priority::CRITICAL, 0),     // P0, oldest
        ];

        for (id, priority, offset) in priorities {
            let mut bead = Issue::new(id.to_string(), format!("{} bead", id), ".".to_string());
            bead.priority = priority;
            bead.created_at = base_time - Duration::seconds(offset);
            storage.create_issue(&bead).unwrap();
        }

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
            .unwrap();

        // Should be ordered: P0, P1, P2, P3, P4 (priority order, age irrelevant)
        assert_eq!(ready.len(), 5);
        assert_eq!(ready[0].id, "bf-p0");
        assert_eq!(ready[1].id, "bf-p1");
        assert_eq!(ready[2].id, "bf-p2");
        assert_eq!(ready[3].id, "bf-p3");
        assert_eq!(ready[4].id, "bf-p4");
    }

    // ============================================================================
    // Age Ordering Tests
    // ============================================================================

    #[test]
    fn test_age_ranking_when_priorities_equal() {
        let (_temp, storage) = setup_test_db();

        let base_time = Utc::now();

        // Create three P0 beads with different ages
        let mut p0_middle = Issue::new("bf-p0-middle".to_string(), "P0 middle".to_string(), ".".to_string());
        p0_middle.priority = Priority::CRITICAL;
        p0_middle.created_at = base_time - Duration::seconds(100);
        storage.create_issue(&p0_middle).unwrap();

        let mut p0_oldest = Issue::new("bf-p0-oldest".to_string(), "P0 oldest".to_string(), ".".to_string());
        p0_oldest.priority = Priority::CRITICAL;
        p0_oldest.created_at = base_time - Duration::seconds(200);
        storage.create_issue(&p0_oldest).unwrap();

        let mut p0_newest = Issue::new("bf-p0-newest".to_string(), "P0 newest".to_string(), ".".to_string());
        p0_newest.priority = Priority::CRITICAL;
        p0_newest.created_at = base_time - Duration::seconds(50);
        storage.create_issue(&p0_newest).unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
            .unwrap();

        // Should be ordered by age (oldest first): oldest, middle, newest
        assert_eq!(ready.len(), 3);
        assert_eq!(ready[0].id, "bf-p0-oldest");
        assert_eq!(ready[1].id, "bf-p0-middle");
        assert_eq!(ready[2].id, "bf-p0-newest");
    }

    #[test]
    fn test_age_fifo_ordering_within_same_priority() {
        let (_temp, storage) = setup_test_db();

        let base_time = Utc::now();

        // Create multiple P1 beads with specific timestamps
        let timestamps = vec![
            ("bf-p1-first", 300),  // Oldest
            ("bf-p1-second", 200),
            ("bf-p1-third", 100),
            ("bf-p1-fourth", 50),  // Newest
        ];

        for (id, offset) in timestamps {
            let mut bead = Issue::new(id.to_string(), format!("P1 bead {}", id), ".".to_string());
            bead.priority = Priority::HIGH; // P1
            bead.created_at = base_time - Duration::seconds(offset);
            storage.create_issue(&bead).unwrap();
        }

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
            .unwrap();

        // Should be in FIFO order (oldest to newest)
        assert_eq!(ready.len(), 4);
        assert_eq!(ready[0].id, "bf-p1-first");
        assert_eq!(ready[1].id, "bf-p1-second");
        assert_eq!(ready[2].id, "bf-p1-third");
        assert_eq!(ready[3].id, "bf-p1-fourth");
    }

    // ============================================================================
    // Combined Priority + Age Tests
    // ============================================================================

    #[test]
    fn test_priority_trumps_age() {
        let (_temp, storage) = setup_test_db();

        let base_time = Utc::now();

        // Create newer P0 and older P1
        let mut p1_old = Issue::new("bf-p1-old".to_string(), "Old P1".to_string(), ".".to_string());
        p1_old.priority = Priority::HIGH;
        p1_old.created_at = base_time - Duration::seconds(200); // Much older
        storage.create_issue(&p1_old).unwrap();

        let mut p0_new = Issue::new("bf-p0-new".to_string(), "New P0".to_string(), ".".to_string());
        p0_new.priority = Priority::CRITICAL;
        p0_new.created_at = base_time - Duration::seconds(10); // Much newer
        storage.create_issue(&p0_new).unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
            .unwrap();

        // P0 should come first despite being newer (priority trumps age)
        assert_eq!(ready.len(), 2);
        assert_eq!(ready[0].id, "bf-p0-new");
        assert_eq!(ready[1].id, "bf-p1-old");
    }

    #[test]
    fn test_complex_priority_and_age_scenario() {
        let (_temp, storage) = setup_test_db();

        let base_time = Utc::now();

        // Create a complex scenario
        let mut p0_medium = Issue::new("bf-p0-med".to_string(), "P0 medium".to_string(), ".".to_string());
        p0_medium.priority = Priority::CRITICAL;
        p0_medium.created_at = base_time - Duration::seconds(100);
        storage.create_issue(&p0_medium).unwrap();

        let mut p1_oldest = Issue::new("bf-p1-old".to_string(), "P1 oldest".to_string(), ".".to_string());
        p1_oldest.priority = Priority::HIGH;
        p1_oldest.created_at = base_time - Duration::seconds(500); // Very old
        storage.create_issue(&p1_oldest).unwrap();

        let mut p0_newest = Issue::new("bf-p0-new".to_string(), "P0 newest".to_string(), ".".to_string());
        p0_newest.priority = Priority::CRITICAL;
        p0_newest.created_at = base_time - Duration::seconds(10); // Very new
        storage.create_issue(&p0_newest).unwrap();

        let mut p2_medium = Issue::new("bf-p2-med".to_string(), "P2 medium".to_string(), ".".to_string());
        p2_medium.priority = Priority::MEDIUM;
        p2_medium.created_at = base_time - Duration::seconds(200);
        storage.create_issue(&p2_medium).unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
            .unwrap();

        // Expected order:
        // 1. p0-medium (older P0)
        // 2. p0-newest (newer P0)
        // 3. p1-oldest (P1, even though very old)
        // 4. p2-medium (P2)
        assert_eq!(ready.len(), 4);
        assert_eq!(ready[0].id, "bf-p0-med");
        assert_eq!(ready[1].id, "bf-p0-new");
        assert_eq!(ready[2].id, "bf-p1-old");
        assert_eq!(ready[3].id, "bf-p2-med");
    }

    // ============================================================================
    // Empty and Edge Case Tests
    // ============================================================================

    #[test]
    fn test_ready_queue_empty_when_no_beads() {
        let (_temp, storage) = setup_test_db();

        // Get ready candidates from empty database
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
            .unwrap();

        assert_eq!(ready.len(), 0);
    }

    #[test]
    fn test_ready_queue_with_single_bead() {
        let (_temp, storage) = setup_test_db();

        create_open_bead(&storage, "bf-single", "Single bead", Priority::MEDIUM);

        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
            .unwrap();

        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-single");
    }

    #[test]
    fn test_ready_queue_limit_parameter() {
        let (_temp, storage) = setup_test_db();

        // Create 10 beads
        for i in 1..=10 {
            create_open_bead(
                &storage,
                &format!("bf-bead-{}", i),
                &format!("Bead {}", i),
                Priority::MEDIUM,
            );
        }

        // Request only 5
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 5, None, None)?))
            .unwrap();

        assert_eq!(ready.len(), 5);
    }

    #[test]
    fn test_ready_queue_unlimited_limit_zero() {
        let (_temp, storage) = setup_test_db();

        // Create 20 beads
        for i in 1..=20 {
            create_open_bead(
                &storage,
                &format!("bf-bead-{}", i),
                &format!("Bead {}", i),
                Priority::MEDIUM,
            );
        }

        // Request unlimited (limit = 0)
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 0, None, None)?))
            .unwrap();

        assert_eq!(ready.len(), 20);
    }

    // ============================================================================
    // Regression Tests
    // ============================================================================

    #[test]
    fn test_zero_dependency_beads_appear_in_ready_queue() {
        let (_temp, storage) = setup_test_db();

        // Regression test for bf-1nprw
        // Beads with zero dependencies should still appear in ready queue
        let bead = create_open_bead(&storage, "bf-nodeps", "No deps", Priority::MEDIUM);

        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
            .unwrap();

        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-nodeps");
    }

    // ============================================================================
    // Edge Case Tests (bf-57wkmy)
    // ============================================================================

    #[test]
    fn test_ready_queue_with_all_beads_blocked() {
        let (_temp, storage) = setup_test_db();

        // Create a chain where blocker-a and blocker-b are both unblocked
        // but dependent-b and dependent-c are blocked
        let blocker_a = create_open_bead(&storage, "bf-blocker-a", "Blocker A", Priority::MEDIUM);
        let blocker_b = create_open_bead(&storage, "bf-blocker-b", "Blocker B", Priority::MEDIUM);
        let _dependent_b = create_open_bead(&storage, "bf-dependent-b", "Dependent B", Priority::MEDIUM);
        let _dependent_c = create_open_bead(&storage, "bf-dependent-c", "Dependent C", Priority::MEDIUM);

        // B depends on A
        storage
            .add_dependency("bf-dependent-b", "bf-blocker-a", &DependencyType::Blocks, "test-user")
            .unwrap();

        // C depends on B
        storage
            .add_dependency("bf-dependent-c", "bf-blocker-b", &DependencyType::Blocks, "test-user")
            .unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
            .unwrap();

        // Both blocker-a and blocker-b should be ready (nothing blocks them)
        assert_eq!(ready.len(), 2);
        assert!(ready.iter().any(|b| b.id == "bf-blocker-a"));
        assert!(ready.iter().any(|b| b.id == "bf-blocker-b"));
        assert!(!ready.iter().any(|b| b.id == "bf-dependent-b"));
        assert!(!ready.iter().any(|b| b.id == "bf-dependent-c"));
    }

    #[test]
    fn test_ready_queue_with_mixed_priorities_and_blocked_statuses() {
        let (_temp, storage) = setup_test_db();

        let base_time = Utc::now();

        // Create P0 blocked bead (should not appear)
        let mut p0_blocked = Issue::new("bf-p0-blocked".to_string(), "P0 blocked".to_string(), ".".to_string());
        p0_blocked.priority = Priority::CRITICAL;
        p0_blocked.created_at = base_time - Duration::seconds(200);
        storage.create_issue(&p0_blocked).unwrap();

        let _blocker = create_open_bead(&storage, "bf-blocker", "Blocker", Priority::MEDIUM);
        storage
            .add_dependency("bf-p0-blocked", "bf-blocker", &DependencyType::Blocks, "test-user")
            .unwrap();

        // Create P1 ready bead (should appear after P0)
        let mut p1_ready = Issue::new("bf-p1-ready".to_string(), "P1 ready".to_string(), ".".to_string());
        p1_ready.priority = Priority::HIGH;
        p1_ready.created_at = base_time - Duration::seconds(150);
        storage.create_issue(&p1_ready).unwrap();

        // Create P0 ready bead (should appear first)
        let mut p0_ready = Issue::new("bf-p0-ready".to_string(), "P0 ready".to_string(), ".".to_string());
        p0_ready.priority = Priority::CRITICAL;
        p0_ready.created_at = base_time - Duration::seconds(100);
        storage.create_issue(&p0_ready).unwrap();

        // Create P2 blocked bead (should not appear)
        let mut p2_blocked = Issue::new("bf-p2-blocked".to_string(), "P2 blocked".to_string(), ".".to_string());
        p2_blocked.priority = Priority::MEDIUM;
        p2_blocked.created_at = base_time - Duration::seconds(50);
        storage.create_issue(&p2_blocked).unwrap();
        storage
            .add_dependency("bf-p2-blocked", "bf-blocker", &DependencyType::Blocks, "test-user")
            .unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
            .unwrap();

        // Should have only 3 beads: P0 ready, P1 ready, and blocker
        // Blocked beads should not appear
        assert_eq!(ready.len(), 3);

        // Verify order: P0 ready (CRITICAL), then P1 ready (HIGH), then blocker (MEDIUM)
        // Priority ordering: CRITICAL(0) < HIGH(1) < MEDIUM(2)
        assert_eq!(ready[0].id, "bf-p0-ready"); // P0 critical
        assert_eq!(ready[1].id, "bf-p1-ready"); // P1 high
        assert_eq!(ready[2].id, "bf-blocker");  // P2 medium blocker

        // Verify blocked beads are not in the ready queue
        assert!(!ready.iter().any(|b| b.id == "bf-p0-blocked"));
        assert!(!ready.iter().any(|b| b.id == "bf-p2-blocked"));
    }

    #[test]
    fn test_bead_becomes_unblocked_when_dependency_closes() {
        let (_temp, storage) = setup_test_db();

        // Create a blocker and a dependent
        let blocker = create_open_bead(&storage, "bf-blocker", "Blocker", Priority::MEDIUM);
        let dependent = create_open_bead(&storage, "bf-dependent", "Dependent", Priority::MEDIUM);

        // Add blocking dependency
        storage
            .add_dependency("bf-dependent", "bf-blocker", &DependencyType::Blocks, "test-user")
            .unwrap();

        // Initially, only blocker should be ready
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
            .unwrap();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-blocker");

        // Close the blocker
        let changes = IssueChanges {
            status: Some(Status::Closed),
            ..Default::default()
        };
        storage.update_issue("bf-blocker", &changes).unwrap();

        // Now dependent should be ready (blocker is closed, so no longer blocks)
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
            .unwrap();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-dependent");
    }

    #[test]
    fn test_bead_blocked_by_closed_dependency_is_ready() {
        let (_temp, storage) = setup_test_db();

        // Create a closed blocker and a dependent
        let mut blocker = Issue::new("bf-blocker".to_string(), "Blocker".to_string(), ".".to_string());
        blocker.status = Status::Closed;
        blocker.closed_at = Some(Utc::now());
        storage.create_issue(&blocker).unwrap();

        let dependent = create_open_bead(&storage, "bf-dependent", "Dependent", Priority::MEDIUM);

        // Add blocking dependency (but blocker is already closed)
        storage
            .add_dependency("bf-dependent", "bf-blocker", &DependencyType::Blocks, "test-user")
            .unwrap();

        // Dependent should be ready because blocker is closed
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
            .unwrap();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-dependent");
    }

    #[test]
    fn test_concurrent_access_multiple_workers() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let temp_file = Arc::new(NamedTempFile::new().unwrap());
        let path = temp_file.path();

        // Initialize storage and create test data
        let storage = Storage::open(path).unwrap();
        for i in 1..=10 {
            create_open_bead(&storage, &format!("bf-bead-{}", i), &format!("Bead {}", i), Priority::MEDIUM);
        }

        let results = Arc::new(Mutex::new(Vec::new()));
        let mut handles = vec![];

        // Spawn 5 concurrent workers
        for worker_id in 0..5 {
            let path_clone = path.to_path_buf();
            let results_clone = Arc::clone(&results);

            let handle = thread::spawn(move || {
                let storage = Storage::open(&path_clone).unwrap();
                let ready = storage
                    .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
                    .unwrap();

                let mut results = results_clone.lock().unwrap();
                results.push((worker_id, ready.len()));
            });
            handles.push(handle);
        }

        // Wait for all workers to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // All workers should have gotten the same results
        let results = results.lock().unwrap();
        assert_eq!(results.len(), 5);

        // All should see 10 ready beads
        for (worker_id, count) in results.iter() {
            assert_eq!(*count, 10, "Worker {} saw {} beads, expected 10", worker_id, count);
        }
    }
}
