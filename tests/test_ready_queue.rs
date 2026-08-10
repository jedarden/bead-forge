//! Basic ready queue query tests
//!
//! This test file validates the fundamental ready queue query functionality:
//! 1. Query returns empty when no beads exist
//! 2. Query returns beads with status=ready (Open and unblocked)
//! 3. Query excludes beads with status=blocked/closed/in_progress
//! 4. Query sorts by priority (P0 > P1 > P2)
//!
//! Bead: bf-5pc0v1

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

/// Create a bead with a specific status
fn create_bead_with_status(
    storage: &Storage,
    id: &str,
    title: &str,
    priority: Priority,
    status: Status,
) -> Issue {
    let is_closed = status == Status::Closed;
    let mut bead = Issue::new(id.to_string(), title.to_string(), ".".to_string());
    bead.priority = priority;
    bead.status = status;
    if is_closed {
        bead.closed_at = Some(Utc::now());
    }
    storage.create_issue(&bead).unwrap();
    bead
}

/// Setup test beads in various states for ready queue testing
fn setup_test_beads(storage: &Storage) {
    let base_time = Utc::now();

    // Create ready beads (Open and unblocked) at different priorities
    let mut p0_ready = Issue::new("bf-ready-p0".to_string(), "P0 Ready task".to_string(), ".".to_string());
    p0_ready.priority = Priority::CRITICAL;
    p0_ready.created_at = base_time - Duration::seconds(100);
    storage.create_issue(&p0_ready).unwrap();

    let mut p1_ready = Issue::new("bf-ready-p1".to_string(), "P1 Ready task".to_string(), ".".to_string());
    p1_ready.priority = Priority::HIGH;
    p1_ready.created_at = base_time - Duration::seconds(50);
    storage.create_issue(&p1_ready).unwrap();

    let mut p2_ready = Issue::new("bf-ready-p2".to_string(), "P2 Ready task".to_string(), ".".to_string());
    p2_ready.priority = Priority::MEDIUM;
    p2_ready.created_at = base_time - Duration::seconds(25);
    storage.create_issue(&p2_ready).unwrap();

    // Create blocked bead
    let mut blocker = Issue::new("bf-blocker".to_string(), "Blocker task".to_string(), ".".to_string());
    blocker.priority = Priority::MEDIUM;
    blocker.created_at = base_time - Duration::seconds(10);
    storage.create_issue(&blocker).unwrap();

    let mut blocked = Issue::new("bf-blocked".to_string(), "Blocked task".to_string(), ".".to_string());
    blocked.priority = Priority::HIGH;
    blocked.created_at = base_time - Duration::seconds(75);
    storage.create_issue(&blocked).unwrap();
    storage
        .add_dependency("bf-blocked", "bf-blocker", &DependencyType::Blocks, "test")
        .unwrap();

    // Create closed bead
    let mut closed = Issue::new("bf-closed".to_string(), "Closed task".to_string(), ".".to_string());
    closed.priority = Priority::HIGH;
    closed.status = Status::Closed;
    closed.closed_at = Some(base_time - Duration::seconds(5));
    closed.created_at = base_time - Duration::seconds(200);
    storage.create_issue(&closed).unwrap();

    // Create in_progress bead
    let mut in_progress = Issue::new("bf-inprogress".to_string(), "In progress task".to_string(), ".".to_string());
    in_progress.priority = Priority::HIGH;
    in_progress.status = Status::InProgress;
    in_progress.assignee = Some("worker1".to_string());
    in_progress.created_at = base_time - Duration::seconds(150);
    storage.create_issue(&in_progress).unwrap();
}

#[cfg(test)]
mod ready_queue_query_tests {
    use super::*;

    // TEST 1: Query returns empty when no beads exist

    #[test]
    fn test_query_returns_empty_when_no_beads_exist() {
        let (_temp, storage) = setup_test_db();

        // Get ready candidates from empty database
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Should return empty vector
        assert_eq!(ready.len(), 0, "Ready queue should be empty when no beads exist");
    }

    #[test]
    fn test_query_returns_empty_with_only_closed_beads() {
        let (_temp, storage) = setup_test_db();

        // Create only closed beads
        create_bead_with_status(&storage, "bf-closed-1", "Closed 1", Priority::MEDIUM, Status::Closed);
        create_bead_with_status(&storage, "bf-closed-2", "Closed 2", Priority::HIGH, Status::Closed);

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Should return empty vector (closed beads are not ready)
        assert_eq!(ready.len(), 0, "Ready queue should be empty when only closed beads exist");
    }

    #[test]
    fn test_query_returns_empty_with_only_blocked_beads() {
        let (_temp, storage) = setup_test_db();

        // Create a chain where all beads are blocked
        let base_time = Utc::now();
        let mut blocker = Issue::new("bf-blocker".to_string(), "Blocker".to_string(), ".".to_string());
        blocker.created_at = base_time;
        storage.create_issue(&blocker).unwrap();

        let mut blocked1 = Issue::new("bf-blocked-1".to_string(), "Blocked 1".to_string(), ".".to_string());
        blocked1.created_at = base_time - Duration::seconds(10);
        storage.create_issue(&blocked1).unwrap();
        storage
            .add_dependency("bf-blocked-1", "bf-blocker", &DependencyType::Blocks, "test")
            .unwrap();

        let mut blocked2 = Issue::new("bf-blocked-2".to_string(), "Blocked 2".to_string(), ".".to_string());
        blocked2.created_at = base_time - Duration::seconds(20);
        storage.create_issue(&blocked2).unwrap();
        storage
            .add_dependency("bf-blocked-2", "bf-blocker", &DependencyType::Blocks, "test")
            .unwrap();

        // Get ready candidates (should only include the blocker, not the blocked beads)
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Should have only the blocker (blocked beads are not ready)
        assert_eq!(ready.len(), 1, "Only the unblocked bead should be ready");
        assert_eq!(ready[0].id, "bf-blocker");
        assert!(!ready.iter().any(|b| b.id == "bf-blocked-1"));
        assert!(!ready.iter().any(|b| b.id == "bf-blocked-2"));
    }

    // TEST 2: Query returns beads with status=ready

    #[test]
    fn test_query_returns_open_unblocked_beads() {
        let (_temp, storage) = setup_test_db();

        // Create multiple open, unblocked beads
        create_open_bead(&storage, "bf-ready-1", "Ready bead 1", Priority::MEDIUM);
        create_open_bead(&storage, "bf-ready-2", "Ready bead 2", Priority::HIGH);
        create_open_bead(&storage, "bf-ready-3", "Ready bead 3", Priority::CRITICAL);

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // All three beads should appear in ready queue
        assert_eq!(ready.len(), 3, "All open, unblocked beads should be ready");
        assert!(ready.iter().any(|b| b.id == "bf-ready-1"), "Ready bead 1 should be in queue");
        assert!(ready.iter().any(|b| b.id == "bf-ready-2"), "Ready bead 2 should be in queue");
        assert!(ready.iter().any(|b| b.id == "bf-ready-3"), "Ready bead 3 should be in queue");
    }

    #[test]
    fn test_query_returns_ready_beads_mixed_with_non_ready() {
        let (_temp, storage) = setup_test_db();

        // Setup mixed bead states
        setup_test_beads(&storage);

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Should return 4 ready beads: 3 explicit ready beads + the blocker
        // (bf-blocker, bf-ready-p0, bf-ready-p1, bf-ready-p2)
        assert_eq!(ready.len(), 4, "All ready beads should be returned (including blocker)");

        // All should have status "open"
        for candidate in &ready {
            assert_eq!(candidate.status, "open", "All ready candidates should have open status");
        }

        // Verify the correct beads are present
        assert!(ready.iter().any(|b| b.id == "bf-ready-p0"), "P0 ready bead should be in queue");
        assert!(ready.iter().any(|b| b.id == "bf-ready-p1"), "P1 ready bead should be in queue");
        assert!(ready.iter().any(|b| b.id == "bf-ready-p2"), "P2 ready bead should be in queue");
        assert!(ready.iter().any(|b| b.id == "bf-blocker"), "Blocker bead should be in queue");

        // Verify blocked/closed/in_progress beads are NOT present
        assert!(!ready.iter().any(|b| b.id == "bf-blocked"), "Blocked bead should not be in queue");
        assert!(!ready.iter().any(|b| b.id == "bf-closed"), "Closed bead should not be in queue");
        assert!(!ready.iter().any(|b| b.id == "bf-inprogress"), "In-progress bead should not be in queue");
    }

    #[test]
    fn test_query_ready_beads_after_blocking_dependency_closes() {
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
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();
        assert_eq!(ready.len(), 1, "Only blocker should be ready initially");
        assert_eq!(ready[0].id, "bf-blocker");

        // Close the blocker
        let changes = IssueChanges {
            status: Some(Status::Closed),
            ..Default::default()
        };
        storage.update_issue("bf-blocker", &changes).unwrap();

        // Now dependent should be ready
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();
        assert_eq!(ready.len(), 1, "Dependent should be ready after blocker closes");
        assert_eq!(ready[0].id, "bf-dependent");
    }

    // TEST 3: Query excludes beads with status=blocked/closed/in_progress

    #[test]
    fn test_query_excludes_blocked_beads() {
        let (_temp, storage) = setup_test_db();

        // Create a blocker and a dependent
        let blocker = create_open_bead(&storage, "bf-blocker", "Blocker", Priority::MEDIUM);
        let dependent =
            create_open_bead(&storage, "bf-dependent", "Dependent", Priority::MEDIUM);

        // Add blocking dependency
        storage
            .add_dependency("bf-dependent", "bf-blocker", &DependencyType::Blocks, "test-user")
            .unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Only the blocker should be ready
        assert_eq!(ready.len(), 1, "Blocked beads should be excluded from ready queue");
        assert_eq!(ready[0].id, "bf-blocker");
        assert!(!ready.iter().any(|b| b.id == "bf-dependent"), "Blocked dependent should not be in queue");
    }

    #[test]
    fn test_query_excludes_closed_beads() {
        let (_temp, storage) = setup_test_db();

        // Create an open bead and a closed bead
        create_open_bead(&storage, "bf-open", "Open bead", Priority::MEDIUM);
        create_bead_with_status(&storage, "bf-closed", "Closed bead", Priority::HIGH, Status::Closed);

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Only the open bead should be ready
        assert_eq!(ready.len(), 1, "Closed beads should be excluded from ready queue");
        assert_eq!(ready[0].id, "bf-open");
        assert!(!ready.iter().any(|b| b.id == "bf-closed"), "Closed bead should not be in queue");
    }

    #[test]
    fn test_query_excludes_in_progress_beads() {
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
        let open_bead = create_open_bead(&storage, "bf-open", "Open bead", Priority::MEDIUM);

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Only the open bead should be ready
        assert_eq!(ready.len(), 1, "In-progress beads should be excluded from ready queue");
        assert_eq!(ready[0].id, "bf-open");
        assert!(
            !ready.iter().any(|b| b.id == "bf-inprogress"),
            "In-progress bead should not be in queue"
        );
    }

    #[test]
    fn test_query_excludes_all_non_ready_statuses() {
        let (_temp, storage) = setup_test_db();

        // Setup beads in all non-ready states
        setup_test_beads(&storage);

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Verify excluded beads are not in the ready queue
        assert!(
            !ready.iter().any(|b| b.id == "bf-blocked"),
            "Blocked bead should be excluded"
        );
        assert!(
            !ready.iter().any(|b| b.id == "bf-closed"),
            "Closed bead should be excluded"
        );
        assert!(
            !ready.iter().any(|b| b.id == "bf-inprogress"),
            "In-progress bead should be excluded"
        );

        // Verify only ready beads are present (including the blocker)
        assert_eq!(ready.len(), 4, "Only ready beads should be in queue (including blocker)");
    }

    // TEST 4: Query sorts by priority (P0 > P1 > P2)

    #[test]
    fn test_query_sorts_by_priority_p0_before_p1_before_p2() {
        let (_temp, storage) = setup_test_db();

        let base_time = Utc::now();

        // Create beads with different priorities (intentionally create in reverse order)
        let mut p2_bead = Issue::new("bf-p2".to_string(), "P2 bead".to_string(), ".".to_string());
        p2_bead.priority = Priority::MEDIUM; // P2
        p2_bead.created_at = base_time - Duration::seconds(150); // Oldest
        storage.create_issue(&p2_bead).unwrap();

        let mut p1_bead = Issue::new("bf-p1".to_string(), "P1 bead".to_string(), ".".to_string());
        p1_bead.priority = Priority::HIGH; // P1
        p1_bead.created_at = base_time - Duration::seconds(100); // Middle
        storage.create_issue(&p1_bead).unwrap();

        let mut p0_bead = Issue::new("bf-p0".to_string(), "P0 bead".to_string(), ".".to_string());
        p0_bead.priority = Priority::CRITICAL; // P0
        p0_bead.created_at = base_time - Duration::seconds(50); // Newest
        storage.create_issue(&p0_bead).unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Should be ordered: P0, P1, P2 (priority trumps age)
        assert_eq!(ready.len(), 3, "All three beads should be ready");
        assert_eq!(ready[0].id, "bf-p0", "P0 should be first (highest priority)");
        assert_eq!(ready[1].id, "bf-p1", "P1 should be second");
        assert_eq!(ready[2].id, "bf-p2", "P2 should be third (lowest priority)");
    }

    #[test]
    fn test_query_sorts_all_priority_levels() {
        let (_temp, storage) = setup_test_db();

        let base_time = Utc::now();

        // Create beads at all priority levels (intentionally create in reverse order)
        let priorities = vec![
            ("bf-p4", Priority::BACKLOG, 200),   // P4, oldest
            ("bf-p3", Priority::LOW, 150),        // P3
            ("bf-p2", Priority::MEDIUM, 100),     // P2
            ("bf-p1", Priority::HIGH, 50),        // P1
            ("bf-p0", Priority::CRITICAL, 0),     // P0, newest
        ];

        for (id, priority, offset) in priorities.into_iter().rev() {
            let mut bead = Issue::new(id.to_string(), format!("{} bead", id), ".".to_string());
            bead.priority = priority;
            bead.created_at = base_time - Duration::seconds(offset);
            storage.create_issue(&bead).unwrap();
        }

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Should be ordered: P0, P1, P2, P3, P4 (priority order, age irrelevant)
        assert_eq!(ready.len(), 5, "All beads should be ready");
        assert_eq!(ready[0].id, "bf-p0", "P0 should be first");
        assert_eq!(ready[1].id, "bf-p1", "P1 should be second");
        assert_eq!(ready[2].id, "bf-p2", "P2 should be third");
        assert_eq!(ready[3].id, "bf-p3", "P3 should be fourth");
        assert_eq!(ready[4].id, "bf-p4", "P4 should be fifth");
    }

    #[test]
    fn test_priority_trumps_age_in_ordering() {
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
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // P0 should come first despite being newer (priority trumps age)
        assert_eq!(ready.len(), 2, "Both beads should be ready");
        assert_eq!(
            ready[0].id,
            "bf-p0-new",
            "P0 should be first despite being newer (priority trumps age)"
        );
        assert_eq!(ready[1].id, "bf-p1-old", "P1 should be second");
    }

    #[test]
    fn test_priority_ordering_with_mixed_ready_and_non_ready() {
        let (_temp, storage) = setup_test_db();

        // Setup mixed bead states
        setup_test_beads(&storage);

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Should have 4 ready beads sorted by priority: the 3 explicit ready beads + the blocker
        assert_eq!(ready.len(), 4, "Should have 4 ready beads (3 explicit + 1 blocker)");

        // Verify priority ordering: P0 (CRITICAL) < P1 (HIGH) < P2 beads (MEDIUM)
        // Note: Lower priority number = higher priority
        assert_eq!(ready[0].id, "bf-ready-p0", "P0 ready bead should be first");
        assert_eq!(ready[1].id, "bf-ready-p1", "P1 ready bead should be second");

        // Positions 2 and 3 should be the two P2 beads (bf-ready-p2 and bf-blocker)
        // Order between same-priority beads depends on secondary sort (created_at/downstream_impact)
        let p2_beads: Vec<_> = ready.iter().skip(2).take(2).map(|b| b.id.as_str()).collect();
        assert!(p2_beads.contains(&"bf-ready-p2"), "bf-ready-p2 should be among P2 beads");
        assert!(p2_beads.contains(&"bf-blocker"), "bf-blocker should be among P2 beads");

        // Verify priority values are in ascending order (lower number = higher priority)
        assert!(ready[0].priority < ready[1].priority, "Priority should be in ascending order");
        assert!(ready[1].priority < ready[2].priority, "Priority should be in ascending order");
    }
}

#[cfg(test)]
mod dependency_and_blocker_tests {
    use super::*;

    // TEST: Beads with unresolved blockers don't appear in ready queue

    #[test]
    fn test_bead_with_single_unresolved_blocker_not_ready() {
        let (_temp, storage) = setup_test_db();

        // Create a blocker and a dependent
        let blocker = create_open_bead(&storage, "bf-blocker", "Blocker", Priority::MEDIUM);
        let dependent = create_open_bead(&storage, "bf-dependent", "Dependent", Priority::HIGH);

        // Add blocking dependency
        storage
            .add_dependency("bf-dependent", "bf-blocker", &DependencyType::Blocks, "test-user")
            .unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Only the blocker should be ready (dependent is blocked)
        assert_eq!(ready.len(), 1, "Only blocker should be ready");
        assert_eq!(ready[0].id, "bf-blocker");
        assert!(!ready.iter().any(|b| b.id == "bf-dependent"), "Blocked dependent should not be in queue");
    }

    #[test]
    fn test_bead_with_multiple_unresolved_blockers_not_ready() {
        let (_temp, storage) = setup_test_db();

        // Create multiple blockers and a dependent
        let blocker1 = create_open_bead(&storage, "bf-blocker1", "Blocker 1", Priority::MEDIUM);
        let blocker2 = create_open_bead(&storage, "bf-blocker2", "Blocker 2", Priority::MEDIUM);
        let dependent = create_open_bead(&storage, "bf-dependent", "Dependent", Priority::HIGH);

        // Add multiple blocking dependencies
        storage
            .add_dependency("bf-dependent", "bf-blocker1", &DependencyType::Blocks, "test-user")
            .unwrap();
        storage
            .add_dependency("bf-dependent", "bf-blocker2", &DependencyType::Blocks, "test-user")
            .unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Both blockers should be ready, dependent should not
        assert_eq!(ready.len(), 2, "Both blockers should be ready");
        assert!(ready.iter().any(|b| b.id == "bf-blocker1"));
        assert!(ready.iter().any(|b| b.id == "bf-blocker2"));
        assert!(!ready.iter().any(|b| b.id == "bf-dependent"), "Dependent with unresolved blockers should not be in queue");
    }

    // TEST: Beads with all blockers closed appear in ready queue

    #[test]
    fn test_bead_with_closed_blockers_appears_in_ready_queue() {
        let (_temp, storage) = setup_test_db();

        // Create a blocker and a dependent
        let blocker = create_open_bead(&storage, "bf-blocker", "Blocker", Priority::MEDIUM);
        let dependent = create_open_bead(&storage, "bf-dependent", "Dependent", Priority::HIGH);

        // Add blocking dependency
        storage
            .add_dependency("bf-dependent", "bf-blocker", &DependencyType::Blocks, "test-user")
            .unwrap();

        // Initially, dependent should be blocked
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();
        assert_eq!(ready.len(), 1, "Only blocker should be ready initially");
        assert_eq!(ready[0].id, "bf-blocker");

        // Close the blocker
        let changes = IssueChanges {
            status: Some(Status::Closed),
            ..Default::default()
        };
        storage.update_issue("bf-blocker", &changes).unwrap();

        // Now dependent should be ready
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();
        assert_eq!(ready.len(), 1, "Dependent should be ready after blocker closes");
        assert_eq!(ready[0].id, "bf-dependent");
    }

    #[test]
    fn test_bead_with_one_closed_one_open_blocker_not_ready() {
        let (_temp, storage) = setup_test_db();

        // Create two blockers and a dependent
        let blocker1 = create_open_bead(&storage, "bf-blocker1", "Blocker 1", Priority::MEDIUM);
        let blocker2 = create_open_bead(&storage, "bf-blocker2", "Blocker 2", Priority::MEDIUM);
        let dependent = create_open_bead(&storage, "bf-dependent", "Dependent", Priority::HIGH);

        // Add blocking dependencies
        storage
            .add_dependency("bf-dependent", "bf-blocker1", &DependencyType::Blocks, "test-user")
            .unwrap();
        storage
            .add_dependency("bf-dependent", "bf-blocker2", &DependencyType::Blocks, "test-user")
            .unwrap();

        // Close only the first blocker
        let changes = IssueChanges {
            status: Some(Status::Closed),
            ..Default::default()
        };
        storage.update_issue("bf-blocker1", &changes).unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Dependent should still NOT be ready (one blocker remains open)
        assert_eq!(ready.len(), 1, "Only the remaining blocker should be ready");
        assert_eq!(ready[0].id, "bf-blocker2");
        assert!(!ready.iter().any(|b| b.id == "bf-dependent"), "Dependent should not be ready while any blocker is open");
    }

    // TEST: Transitive blocker relationships

    #[test]
    fn test_transitive_blocking_chain() {
        let (_temp, storage) = setup_test_db();

        // Create a chain: A blocks B, B blocks C
        let bead_a = create_open_bead(&storage, "bf-a", "Bead A", Priority::MEDIUM);
        let bead_b = create_open_bead(&storage, "bf-b", "Bead B", Priority::MEDIUM);
        let bead_c = create_open_bead(&storage, "bf-c", "Bead C", Priority::MEDIUM);

        // Add blocking dependencies: A -> B -> C
        storage
            .add_dependency("bf-b", "bf-a", &DependencyType::Blocks, "test-user")
            .unwrap();
        storage
            .add_dependency("bf-c", "bf-b", &DependencyType::Blocks, "test-user")
            .unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Only A should be ready (B is blocked by A, C is blocked by B)
        assert_eq!(ready.len(), 1, "Only the root blocker should be ready");
        assert_eq!(ready[0].id, "bf-a");
        assert!(!ready.iter().any(|b| b.id == "bf-b"));
        assert!(!ready.iter().any(|b| b.id == "bf-c"));
    }

    #[test]
    fn test_transitive_blocking_chain_progressive_unblocking() {
        let (_temp, storage) = setup_test_db();

        // Create a chain: A blocks B, B blocks C
        let bead_a = create_open_bead(&storage, "bf-a", "Bead A", Priority::MEDIUM);
        let bead_b = create_open_bead(&storage, "bf-b", "Bead B", Priority::MEDIUM);
        let bead_c = create_open_bead(&storage, "bf-c", "Bead C", Priority::MEDIUM);

        // Add blocking dependencies: A -> B -> C
        storage
            .add_dependency("bf-b", "bf-a", &DependencyType::Blocks, "test-user")
            .unwrap();
        storage
            .add_dependency("bf-c", "bf-b", &DependencyType::Blocks, "test-user")
            .unwrap();

        // Initially only A is ready
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-a");

        // Close A - now B should become ready
        let changes = IssueChanges {
            status: Some(Status::Closed),
            ..Default::default()
        };
        storage.update_issue("bf-a", &changes).unwrap();

        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();
        assert_eq!(ready.len(), 1, "B should be ready after A closes");
        assert_eq!(ready[0].id, "bf-b");

        // Close B - now C should become ready
        storage.update_issue("bf-b", &changes).unwrap();

        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();
        assert_eq!(ready.len(), 1, "C should be ready after B closes");
        assert_eq!(ready[0].id, "bf-c");
    }

    #[test]
    fn test_complex_transitive_blocking_diamond() {
        let (_temp, storage) = setup_test_db();

        // Create a diamond dependency: A blocks B and C, both B and C block D
        let bead_a = create_open_bead(&storage, "bf-a", "Bead A", Priority::MEDIUM);
        let bead_b = create_open_bead(&storage, "bf-b", "Bead B", Priority::MEDIUM);
        let bead_c = create_open_bead(&storage, "bf-c", "Bead C", Priority::MEDIUM);
        let bead_d = create_open_bead(&storage, "bf-d", "Bead D", Priority::MEDIUM);

        // Add blocking dependencies: A -> B, A -> C, B -> D, C -> D
        storage
            .add_dependency("bf-b", "bf-a", &DependencyType::Blocks, "test-user")
            .unwrap();
        storage
            .add_dependency("bf-c", "bf-a", &DependencyType::Blocks, "test-user")
            .unwrap();
        storage
            .add_dependency("bf-d", "bf-b", &DependencyType::Blocks, "test-user")
            .unwrap();
        storage
            .add_dependency("bf-d", "bf-c", &DependencyType::Blocks, "test-user")
            .unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Only A should be ready (all others are transitively blocked)
        assert_eq!(ready.len(), 1, "Only the root should be ready in diamond dependency");
        assert_eq!(ready[0].id, "bf-a");
        assert!(!ready.iter().any(|b| b.id == "bf-b"));
        assert!(!ready.iter().any(|b| b.id == "bf-c"));
        assert!(!ready.iter().any(|b| b.id == "bf-d"));
    }

    // TEST: Downstream impact ranking

    #[test]
    fn test_downstream_impact_ranking_basic() {
        let (_temp, storage) = setup_test_db();

        // Create beads with different downstream impacts
        // bead_a blocks 3 beads, bead_b blocks 1 bead, bead_c blocks 0
        let bead_a = create_open_bead(&storage, "bf-a", "Bead A (blocks 3)", Priority::MEDIUM);
        let bead_b = create_open_bead(&storage, "bf-b", "Bead B (blocks 1)", Priority::MEDIUM);
        let bead_c = create_open_bead(&storage, "bf-c", "Bead C (blocks 0)", Priority::MEDIUM);

        // Beads blocked by A
        let dep_a1 = create_open_bead(&storage, "bf-dep-a1", "Dep A1", Priority::MEDIUM);
        let dep_a2 = create_open_bead(&storage, "bf-dep-a2", "Dep A2", Priority::MEDIUM);
        let dep_a3 = create_open_bead(&storage, "bf-dep-a3", "Dep A3", Priority::MEDIUM);

        // Bead blocked by B
        let dep_b1 = create_open_bead(&storage, "bf-dep-b1", "Dep B1", Priority::MEDIUM);

        // Add dependencies
        storage
            .add_dependency("bf-dep-a1", "bf-a", &DependencyType::Blocks, "test")
            .unwrap();
        storage
            .add_dependency("bf-dep-a2", "bf-a", &DependencyType::Blocks, "test")
            .unwrap();
        storage
            .add_dependency("bf-dep-a3", "bf-a", &DependencyType::Blocks, "test")
            .unwrap();
        storage
            .add_dependency("bf-dep-b1", "bf-b", &DependencyType::Blocks, "test")
            .unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // All three ready beads should be present
        assert_eq!(ready.len(), 3, "All three unblocked beads should be ready");

        // Should be ordered by downstream impact: A (3) > B (1) > C (0)
        assert_eq!(ready[0].id, "bf-a", "Bead blocking 3 should be first");
        assert_eq!(ready[1].id, "bf-b", "Bead blocking 1 should be second");
        assert_eq!(ready[2].id, "bf-c", "Bead blocking 0 should be third");

        // Verify downstream_impact values
        assert_eq!(ready[0].downstream_impact, 3);
        assert_eq!(ready[1].downstream_impact, 1);
        assert_eq!(ready[2].downstream_impact, 0);
    }

    #[test]
    fn test_downstream_impact_with_priority_tiebreaker() {
        let (_temp, storage) = setup_test_db();

        // Create two beads with same priority but different downstream impact
        let bead_high_impact = create_open_bead(&storage, "bf-high", "High impact", Priority::HIGH);
        let bead_low_impact = create_open_bead(&storage, "bf-low", "Low impact", Priority::HIGH);

        // Add dependencies to create different impacts
        let dep1 = create_open_bead(&storage, "bf-dep1", "Dep 1", Priority::MEDIUM);
        let dep2 = create_open_bead(&storage, "bf-dep2", "Dep 2", Priority::MEDIUM);
        storage
            .add_dependency("bf-dep1", "bf-high", &DependencyType::Blocks, "test")
            .unwrap();
        storage
            .add_dependency("bf-dep2", "bf-high", &DependencyType::Blocks, "test")
            .unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Both should be present
        assert_eq!(ready.len(), 2);

        // High impact bead should come first (same priority, higher impact wins)
        assert_eq!(ready[0].id, "bf-high");
        assert_eq!(ready[1].id, "bf-low");
        assert_eq!(ready[0].downstream_impact, 2);
        assert_eq!(ready[1].downstream_impact, 0);
    }

    #[test]
    fn test_downstream_impact_with_mixed_priorities() {
        let (_temp, storage) = setup_test_db();

        // Create beads with mixed priorities and impacts
        // P0 with 0 impact vs P1 with 10 impact - priority should win
        let p0_low_impact = create_open_bead(&storage, "bf-p0", "P0 low impact", Priority::CRITICAL);
        let p1_high_impact = create_open_bead(&storage, "bf-p1", "P1 high impact", Priority::HIGH);

        // Give P1 many dependencies
        for i in 1..=5 {
            let dep = create_open_bead(&storage, &format!("bf-dep-{}", i), "Dep", Priority::MEDIUM);
            storage
                .add_dependency(&format!("bf-dep-{}", i), "bf-p1", &DependencyType::Blocks, "test")
                .unwrap();
        }

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Both should be present
        assert_eq!(ready.len(), 2);

        // P0 should come first (priority trumps impact)
        assert_eq!(ready[0].id, "bf-p0", "P0 should be first despite lower impact");
        assert_eq!(ready[1].id, "bf-p1");
    }

    // TEST: Different dependency types

    #[test]
    fn test_non_blocking_dependency_types_dont_affect_ready_queue() {
        let (_temp, storage) = setup_test_db();

        // Create beads with non-blocking dependency types
        let bead_a = create_open_bead(&storage, "bf-a", "Bead A", Priority::MEDIUM);
        let bead_b = create_open_bead(&storage, "bf-b", "Bead B", Priority::MEDIUM);

        // Add a non-blocking dependency (Related type)
        storage
            .add_dependency("bf-b", "bf-a", &DependencyType::Related, "test-user")
            .unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Both beads should be ready (Related doesn't block)
        assert_eq!(ready.len(), 2, "Non-blocking dependencies should not affect ready queue");
        assert!(ready.iter().any(|b| b.id == "bf-a"));
        assert!(ready.iter().any(|b| b.id == "bf-b"));
    }

    #[test]
    fn test_conditional_blocks_affects_ready_queue() {
        let (_temp, storage) = setup_test_db();

        // Create beads with conditional-blocks dependency
        let blocker = create_open_bead(&storage, "bf-blocker", "Blocker", Priority::MEDIUM);
        let dependent = create_open_bead(&storage, "bf-dependent", "Dependent", Priority::HIGH);

        // Add conditional-blocks dependency (should block)
        storage
            .add_dependency("bf-dependent", "bf-blocker", &DependencyType::ConditionalBlocks, "test-user")
            .unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Only blocker should be ready
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-blocker");
        assert!(!ready.iter().any(|b| b.id == "bf-dependent"));
    }

    #[test]
    fn test_parent_child_blocks_ready_queue() {
        let (_temp, storage) = setup_test_db();

        // Create parent and child beads
        let parent = create_open_bead(&storage, "bf-parent", "Parent", Priority::MEDIUM);
        let child = create_open_bead(&storage, "bf-child", "Child", Priority::HIGH);

        // Add parent-child dependency (should block child)
        storage
            .add_dependency("bf-child", "bf-parent", &DependencyType::ParentChild, "test-user")
            .unwrap();

        // Get ready candidates
        let ready = storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap();

        // Only parent should be ready
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "bf-parent");
        assert!(!ready.iter().any(|b| b.id == "bf-child"));
    }
}
