//! Tests for ready queue query functionality
//!
//! This test file validates that:
//! 1. Query returns empty when no beads exist
//! 2. Query returns beads with status=open (ready)
//! 3. Query excludes beads with status=blocked/closed/in_progress
//! 4. Query sorts by priority (P0 > P1 > P2)

use bead_forge::claim::get_ready_candidates;
use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;
use rusqlite::params;
use tempfile::NamedTempFile;

/// Create a test database
fn setup_test_db() -> (NamedTempFile, Storage) {
    let temp_file = NamedTempFile::new().unwrap();
    let storage = Storage::open(temp_file.path()).unwrap();
    (temp_file, storage)
}

/// Create a helper function to setup test beads
fn create_test_bead(
    storage: &Storage,
    id: &str,
    title: &str,
    status: Status,
    priority: Priority,
) -> Issue {
    let now = Utc::now();
    let closed_at = match status {
        Status::Closed | Status::Tombstone => Some(now.clone()),
        _ => None,
    };

    let issue = Issue {
        id: id.to_string(),
        title: title.to_string(),
        status,
        priority,
        issue_type: IssueType::Task,
        created_at: now.clone(),
        updated_at: now,
        closed_at,
        source_repo: Some(".".to_string()),
        events: Vec::new(),
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();
    issue
}

// =============================================================================
// Test 1: Query returns empty when no beads exist
// =============================================================================

#[test]
fn test_ready_queue_empty_when_no_beads() {
    let (_temp, storage) = setup_test_db();

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(
        candidates.len(),
        0,
        "Ready queue should be empty when no beads exist"
    );
}

#[test]
fn test_ready_queue_empty_with_only_closed_beads() {
    let (_temp, storage) = setup_test_db();

    // Create only closed beads
    create_test_bead(
        &storage,
        "bf-closed-1",
        "Closed bead 1",
        Status::Closed,
        Priority::HIGH,
    );
    create_test_bead(
        &storage,
        "bf-closed-2",
        "Closed bead 2",
        Status::Closed,
        Priority::MEDIUM,
    );

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(
        candidates.len(),
        0,
        "Ready queue should be empty when all beads are closed"
    );
}

// =============================================================================
// Test 2: Query returns beads with status=open (ready)
// =============================================================================

#[test]
fn test_ready_queue_returns_open_beads() {
    let (_temp, storage) = setup_test_db();

    // Create open beads
    create_test_bead(
        &storage,
        "bf-ready-1",
        "Ready bead 1",
        Status::Open,
        Priority::MEDIUM,
    );
    create_test_bead(
        &storage,
        "bf-ready-2",
        "Ready bead 2",
        Status::Open,
        Priority::LOW,
    );

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(
        candidates.len(),
        2,
        "Ready queue should return all open beads"
    );

    // Verify the returned beads are indeed open
    for candidate in &candidates {
        assert_eq!(
            candidate.status,
            "open",
            "All candidates should have status=open"
        );
    }

    // Verify IDs match
    let ids: Vec<&str> = candidates.iter().map(|c| c.id.as_str()).collect();
    assert!(ids.contains(&"bf-ready-1"), "Should contain bf-ready-1");
    assert!(ids.contains(&"bf-ready-2"), "Should contain bf-ready-2");
}

#[test]
fn test_ready_queue_mixed_statuses_returns_only_open() {
    let (_temp, storage) = setup_test_db();

    // Create beads with various statuses
    create_test_bead(
        &storage,
        "bf-open",
        "Open bead",
        Status::Open,
        Priority::MEDIUM,
    );
    create_test_bead(
        &storage,
        "bf-closed",
        "Closed bead",
        Status::Closed,
        Priority::MEDIUM,
    );
    create_test_bead(
        &storage,
        "bf-blocked",
        "Blocked bead",
        Status::Blocked,
        Priority::MEDIUM,
    );
    create_test_bead(
        &storage,
        "bf-in-progress",
        "In progress bead",
        Status::InProgress,
        Priority::MEDIUM,
    );

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(
        candidates.len(),
        1,
        "Ready queue should return only open beads"
    );
    assert_eq!(
        candidates[0].id, "bf-open",
        "Only the open bead should be in the queue"
    );
}

// =============================================================================
// Test 3: Query excludes beads with status=blocked/closed/in_progress
// =============================================================================

#[test]
fn test_ready_queue_excludes_blocked_beads() {
    let (_temp, storage) = setup_test_db();

    // Create open and blocked beads
    create_test_bead(
        &storage,
        "bf-open",
        "Open bead",
        Status::Open,
        Priority::MEDIUM,
    );
    create_test_bead(
        &storage,
        "bf-blocked",
        "Blocked bead",
        Status::Blocked,
        Priority::HIGH,
    );

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].id, "bf-open");
    assert!(!candidates.iter().any(|c| c.id == "bf-blocked"));
}

#[test]
fn test_ready_queue_excludes_closed_beads() {
    let (_temp, storage) = setup_test_db();

    // Create open and closed beads
    create_test_bead(
        &storage,
        "bf-open",
        "Open bead",
        Status::Open,
        Priority::MEDIUM,
    );
    create_test_bead(
        &storage,
        "bf-closed",
        "Closed bead",
        Status::Closed,
        Priority::HIGH,
    );

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].id, "bf-open");
    assert!(!candidates.iter().any(|c| c.id == "bf-closed"));
}

#[test]
fn test_ready_queue_excludes_in_progress_beads() {
    let (_temp, storage) = setup_test_db();

    // Create open and in-progress beads
    create_test_bead(
        &storage,
        "bf-open",
        "Open bead",
        Status::Open,
        Priority::MEDIUM,
    );
    create_test_bead(
        &storage,
        "bf-in-progress",
        "In progress bead",
        Status::InProgress,
        Priority::HIGH,
    );

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].id, "bf-open");
    assert!(!candidates
        .iter()
        .any(|c| c.id == "bf-in-progress"));
}

#[test]
fn test_ready_queue_excludes_deferred_beads() {
    let (_temp, storage) = setup_test_db();

    // Create open and deferred beads
    create_test_bead(
        &storage,
        "bf-open",
        "Open bead",
        Status::Open,
        Priority::MEDIUM,
    );
    create_test_bead(
        &storage,
        "bf-deferred",
        "Deferred bead",
        Status::Deferred,
        Priority::HIGH,
    );

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].id, "bf-open");
    assert!(!candidates.iter().any(|c| c.id == "bf-deferred"));
}

// =============================================================================
// Test 4: Query sorts by priority (P0 > P1 > P2)
// =============================================================================

#[test]
fn test_ready_queue_sorts_by_priority() {
    let (_temp, storage) = setup_test_db();

    // Create beads with different priorities
    create_test_bead(
        &storage,
        "bf-p2",
        "Priority 2",
        Status::Open,
        Priority(2),
    );
    create_test_bead(
        &storage,
        "bf-p0",
        "Priority 0",
        Status::Open,
        Priority(0),
    );
    create_test_bead(
        &storage,
        "bf-p1",
        "Priority 1",
        Status::Open,
        Priority(1),
    );

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(candidates.len(), 3);

    // Verify priority order: P0 first, then P1, then P2
    assert_eq!(candidates[0].id, "bf-p0", "First should be P0");
    assert_eq!(candidates[1].id, "bf-p1", "Second should be P1");
    assert_eq!(candidates[2].id, "bf-p2", "Third should be P2");
}

#[test]
fn test_ready_queue_same_priority_fifo_order() {
    let (_temp, storage) = setup_test_db();

    // Create beads with same priority but different timestamps
    let now = Utc::now();

    let mut bead1 = Issue {
        id: "bf-first".to_string(),
        title: "First bead".to_string(),
        status: Status::Open,
        priority: Priority(1),
        issue_type: IssueType::Task,
        created_at: now - chrono::Duration::seconds(10),
        updated_at: now - chrono::Duration::seconds(10),
        source_repo: Some(".".to_string()),
        events: Vec::new(),
        ..Default::default()
    };
    storage.create_issue(&bead1).unwrap();

    let mut bead2 = Issue {
        id: "bf-second".to_string(),
        title: "Second bead".to_string(),
        status: Status::Open,
        priority: Priority(1),
        issue_type: IssueType::Task,
        created_at: now,
        updated_at: now,
        source_repo: Some(".".to_string()),
        events: Vec::new(),
        ..Default::default()
    };
    storage.create_issue(&bead2).unwrap();

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(candidates.len(), 2);

    // Older bead should come first (FIFO)
    assert_eq!(candidates[0].id, "bf-first", "Older bead should come first");
    assert_eq!(candidates[1].id, "bf-second");
}

#[test]
fn test_ready_queue_all_priorities() {
    let (_temp, storage) = setup_test_db();

    // Create beads with all standard priorities
    create_test_bead(&storage, "bf-p4", "P4", Status::Open, Priority(4));
    create_test_bead(&storage, "bf-p3", "P3", Status::Open, Priority(3));
    create_test_bead(&storage, "bf-p2", "P2", Status::Open, Priority(2));
    create_test_bead(&storage, "bf-p1", "P1", Status::Open, Priority(1));
    create_test_bead(&storage, "bf-p0", "P0", Status::Open, Priority(0));

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(candidates.len(), 5);

    // Verify priority order from highest to lowest
    let ids: Vec<&str> = candidates.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        ids, vec!["bf-p0", "bf-p1", "bf-p2", "bf-p3", "bf-p4"],
        "Should be ordered P0, P1, P2, P3, P4"
    );
}

// =============================================================================
// Additional edge case tests
// =============================================================================

#[test]
fn test_ready_queue_respects_limit() {
    let (_temp, storage) = setup_test_db();

    // Create 5 open beads
    for i in 0..5 {
        create_test_bead(
            &storage,
            &format!("bf-{}", i),
            &format!("Bead {}", i),
            Status::Open,
            Priority::MEDIUM,
        );
    }

    // Request only 3
    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 3, None, None)?))
        .unwrap();

    assert_eq!(
        candidates.len(),
        3,
        "Should return only the requested limit"
    );
}

#[test]
fn test_ready_queue_limit_zero_means_unlimited() {
    let (_temp, storage) = setup_test_db();

    // Create 5 open beads
    for i in 0..5 {
        create_test_bead(
            &storage,
            &format!("bf-{}", i),
            &format!("Bead {}", i),
            Status::Open,
            Priority::MEDIUM,
        );
    }

    // Request unlimited (limit = 0)
    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 0, None, None)?))
        .unwrap();

    assert_eq!(
        candidates.len(),
        5,
        "Limit=0 should return all beads (unlimited)"
    );
}

#[test]
fn test_ready_queue_empty_when_all_blocked() {
    let (_temp, storage) = setup_test_db();

    // Create only blocked beads
    create_test_bead(
        &storage,
        "bf-blocked-1",
        "Blocked 1",
        Status::Blocked,
        Priority::HIGH,
    );
    create_test_bead(
        &storage,
        "bf-blocked-2",
        "Blocked 2",
        Status::Blocked,
        Priority::MEDIUM,
    );
    create_test_bead(
        &storage,
        "bf-blocked-3",
        "Blocked 3",
        Status::Blocked,
        Priority::LOW,
    );

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(
        candidates.len(),
        0,
        "Ready queue should be empty when all beads are blocked"
    );
}

// =============================================================================
// Filtering helper functions
// =============================================================================

/// Filter candidates by a single priority level
fn filter_by_priority<'a>(candidates: &'a [bead_forge::claim::ScoredBead], priority: i32) -> Vec<&'a bead_forge::claim::ScoredBead> {
    candidates.iter().filter(|c| c.priority == priority).collect()
}

/// Filter candidates by multiple priority levels (OR logic)
fn filter_by_priorities<'a>(candidates: &'a [bead_forge::claim::ScoredBead], priorities: &[i32]) -> Vec<&'a bead_forge::claim::ScoredBead> {
    candidates.iter().filter(|c| priorities.contains(&c.priority)).collect()
}

/// Filter candidates by assignee
fn filter_by_assignee<'a>(candidates: &'a [bead_forge::claim::ScoredBead], assignee: &str) -> Vec<&'a bead_forge::claim::ScoredBead> {
    candidates.iter().filter(|c| c.id.contains(assignee) || c.title.contains(assignee)).collect()
}

/// Filter candidates by label substring in ID or title
fn filter_by_label<'a>(candidates: &'a [bead_forge::claim::ScoredBead], label: &str) -> Vec<&'a bead_forge::claim::ScoredBead> {
    candidates.iter().filter(|c| c.id.contains(label) || c.title.contains(label)).collect()
}

/// Filter candidates by multiple criteria (priority AND label)
fn filter_by_priority_and_label<'a>(
    candidates: &'a [bead_forge::claim::ScoredBead],
    priority: i32,
    label: &str,
) -> Vec<&'a bead_forge::claim::ScoredBead> {
    candidates.iter().filter(|c| c.priority == priority && (c.id.contains(label) || c.title.contains(label))).collect()
}

// =============================================================================
// Test 5: Filter by single priority (P0-only)
// =============================================================================

#[test]
fn test_filter_p0_only() {
    let (_temp, storage) = setup_test_db();

    // Create beads with different priorities
    create_test_bead(&storage, "bf-p0-1", "P0 task 1", Status::Open, Priority(0));
    create_test_bead(&storage, "bf-p0-2", "P0 task 2", Status::Open, Priority(0));
    create_test_bead(&storage, "bf-p1-1", "P1 task 1", Status::Open, Priority(1));
    create_test_bead(&storage, "bf-p2-1", "P2 task 1", Status::Open, Priority(2));
    create_test_bead(&storage, "bf-p3-1", "P3 task 1", Status::Open, Priority(3));

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    // Filter to P0 only
    let p0_only = filter_by_priority(&candidates, 0);

    assert_eq!(
        p0_only.len(),
        2,
        "Should return exactly 2 P0 beads"
    );

    // Verify all returned beads are P0
    for bead in &p0_only {
        assert_eq!(bead.priority, 0, "All filtered beads should be P0");
    }

    // Verify P0 priority order is maintained (older first)
    let ids: Vec<&str> = p0_only.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, vec!["bf-p0-1", "bf-p0-2"], "P0 beads should maintain FIFO order");

    // Verify exclusion: P1, P2, P3 not in results
    assert!(!p0_only.iter().any(|c| c.id == "bf-p1-1"), "P1 bead should be excluded");
    assert!(!p0_only.iter().any(|c| c.id == "bf-p2-1"), "P2 bead should be excluded");
    assert!(!p0_only.iter().any(|c| c.id == "bf-p3-1"), "P3 bead should be excluded");
}

#[test]
fn test_filter_p1_only() {
    let (_temp, storage) = setup_test_db();

    // Create beads with different priorities
    create_test_bead(&storage, "bf-p0-1", "P0 task", Status::Open, Priority(0));
    create_test_bead(&storage, "bf-p1-1", "P1 task 1", Status::Open, Priority(1));
    create_test_bead(&storage, "bf-p1-2", "P1 task 2", Status::Open, Priority(1));
    create_test_bead(&storage, "bf-p2-1", "P2 task", Status::Open, Priority(2));

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    // Filter to P1 only
    let p1_only = filter_by_priority(&candidates, 1);

    assert_eq!(
        p1_only.len(),
        2,
        "Should return exactly 2 P1 beads"
    );

    // Verify all returned beads are P1
    for bead in &p1_only {
        assert_eq!(bead.priority, 1, "All filtered beads should be P1");
    }

    // Verify exclusion
    assert!(!p1_only.iter().any(|c| c.id == "bf-p0-1"), "P0 bead should be excluded");
    assert!(!p1_only.iter().any(|c| c.id == "bf-p2-1"), "P2 bead should be excluded");
}

#[test]
fn test_filter_p2_only() {
    let (_temp, storage) = setup_test_db();

    // Create beads with different priorities
    create_test_bead(&storage, "bf-p0-1", "P0 task", Status::Open, Priority(0));
    create_test_bead(&storage, "bf-p1-1", "P1 task", Status::Open, Priority(1));
    create_test_bead(&storage, "bf-p2-1", "P2 task 1", Status::Open, Priority(2));
    create_test_bead(&storage, "bf-p2-2", "P2 task 2", Status::Open, Priority(2));
    create_test_bead(&storage, "bf-p2-3", "P2 task 3", Status::Open, Priority(2));

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    // Filter to P2 only
    let p2_only = filter_by_priority(&candidates, 2);

    assert_eq!(
        p2_only.len(),
        3,
        "Should return exactly 3 P2 beads"
    );

    // Verify all returned beads are P2
    for bead in &p2_only {
        assert_eq!(bead.priority, 2, "All filtered beads should be P2");
    }

    // Verify priority ordering maintained (FIFO within P2)
    let ids: Vec<&str> = p2_only.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, vec!["bf-p2-1", "bf-p2-2", "bf-p2-3"], "P2 beads should maintain FIFO order");
}

// =============================================================================
// Test 6: Filter by multiple priorities (P0 or P1)
// =============================================================================

#[test]
fn test_filter_p0_or_p1() {
    let (_temp, storage) = setup_test_db();

    // Create beads across all priorities
    create_test_bead(&storage, "bf-p0-1", "P0 task", Status::Open, Priority(0));
    create_test_bead(&storage, "bf-p1-1", "P1 task 1", Status::Open, Priority(1));
    create_test_bead(&storage, "bf-p1-2", "P1 task 2", Status::Open, Priority(1));
    create_test_bead(&storage, "bf-p2-1", "P2 task", Status::Open, Priority(2));
    create_test_bead(&storage, "bf-p3-1", "P3 task", Status::Open, Priority(3));
    create_test_bead(&storage, "bf-p4-1", "P4 task", Status::Open, Priority(4));

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    // Filter to P0 or P1
    let p0_or_p1 = filter_by_priorities(&candidates, &[0, 1]);

    assert_eq!(
        p0_or_p1.len(),
        3,
        "Should return exactly 3 beads (1 P0 + 2 P1)"
    );

    // Verify all returned beads are P0 or P1
    for bead in &p0_or_p1 {
        assert!(
            bead.priority == 0 || bead.priority == 1,
            "All filtered beads should be P0 or P1"
        );
    }

    // Verify priority ordering: P0 first, then P1s
    assert_eq!(p0_or_p1[0].priority, 0, "First should be P0");
    assert_eq!(p0_or_p1[0].id, "bf-p0-1");
    assert_eq!(p0_or_p1[1].priority, 1, "Second should be P1");
    assert_eq!(p0_or_p1[2].priority, 1, "Third should be P1");

    // Verify exclusion: P2, P3, P4 not in results
    assert!(!p0_or_p1.iter().any(|c| c.id == "bf-p2-1"), "P2 bead should be excluded");
    assert!(!p0_or_p1.iter().any(|c| c.id == "bf-p3-1"), "P3 bead should be excluded");
    assert!(!p0_or_p1.iter().any(|c| c.id == "bf-p4-1"), "P4 bead should be excluded");
}

#[test]
fn test_filter_multiple_priorities_wide_range() {
    let (_temp, storage) = setup_test_db();

    // Create beads across all priorities
    create_test_bead(&storage, "bf-p0-1", "P0 task", Status::Open, Priority(0));
    create_test_bead(&storage, "bf-p1-1", "P1 task", Status::Open, Priority(1));
    create_test_bead(&storage, "bf-p2-1", "P2 task", Status::Open, Priority(2));
    create_test_bead(&storage, "bf-p3-1", "P3 task", Status::Open, Priority(3));
    create_test_bead(&storage, "bf-p4-1", "P4 task", Status::Open, Priority(4));

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    // Filter to P1, P2, P3
    let p1_p2_p3 = filter_by_priorities(&candidates, &[1, 2, 3]);

    assert_eq!(
        p1_p2_p3.len(),
        3,
        "Should return exactly 3 beads (P1, P2, P3)"
    );

    // Verify priority ordering is maintained
    assert_eq!(p1_p2_p3[0].priority, 1, "First should be P1");
    assert_eq!(p1_p2_p3[1].priority, 2, "Second should be P2");
    assert_eq!(p1_p2_p3[2].priority, 3, "Third should be P3");

    // Verify exclusion
    assert!(!p1_p2_p3.iter().any(|c| c.priority == 0), "P0 bead should be excluded");
    assert!(!p1_p2_p3.iter().any(|c| c.priority == 4), "P4 bead should be excluded");
}

// =============================================================================
// Test 7: Filter by assignee
// =============================================================================

#[test]
fn test_filter_by_assignee_single_worker() {
    let (_temp, storage) = setup_test_db();

    // Create beads for different workers (encoded in ID/title)
    create_test_bead(&storage, "bf-worker-1", "Task for worker-1", Status::Open, Priority(1));
    create_test_bead(&storage, "bf-worker-1-b", "Another for worker-1", Status::Open, Priority(1));
    create_test_bead(&storage, "bf-worker-2", "Task for worker-2", Status::Open, Priority(1));
    create_test_bead(&storage, "bf-worker-3", "Task for worker-3", Status::Open, Priority(1));

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    // Filter to worker-1 only
    let worker_1_tasks = filter_by_assignee(&candidates, "worker-1");

    assert_eq!(
        worker_1_tasks.len(),
        2,
        "Should return exactly 2 tasks for worker-1"
    );

    // Verify all returned beads are for worker-1
    for bead in &worker_1_tasks {
        assert!(
            bead.id.contains("worker-1") || bead.title.contains("worker-1"),
            "All filtered beads should be for worker-1"
        );
    }

    // Verify exclusion
    assert!(!worker_1_tasks.iter().any(|c| c.id.contains("worker-2")), "worker-2 tasks should be excluded");
    assert!(!worker_1_tasks.iter().any(|c| c.id.contains("worker-3")), "worker-3 tasks should be excluded");
}

#[test]
fn test_filter_by_assignee_empty_result() {
    let (_temp, storage) = setup_test_db();

    // Create beads for specific workers
    create_test_bead(&storage, "bf-worker-1", "Task for worker-1", Status::Open, Priority(1));
    create_test_bead(&storage, "bf-worker-2", "Task for worker-2", Status::Open, Priority(1));

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    // Filter to non-existent worker
    let worker_x_tasks = filter_by_assignee(&candidates, "worker-x");

    assert_eq!(
        worker_x_tasks.len(),
        0,
        "Should return empty result for non-existent assignee"
    );
}

// =============================================================================
// Test 8: Filter by label combination
// =============================================================================

#[test]
fn test_filter_by_label_single() {
    let (_temp, storage) = setup_test_db();

    // Create beads with different labels (encoded in ID/title)
    create_test_bead(&storage, "bf-backend-1", "Backend task 1", Status::Open, Priority(1));
    create_test_bead(&storage, "bf-backend-2", "Backend task 2", Status::Open, Priority(1));
    create_test_bead(&storage, "bf-frontend-1", "Frontend task", Status::Open, Priority(1));
    create_test_bead(&storage, "bf-docs-1", "Docs task", Status::Open, Priority(1));

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    // Filter to "backend" label
    let backend_tasks = filter_by_label(&candidates, "backend");

    assert_eq!(
        backend_tasks.len(),
        2,
        "Should return exactly 2 backend tasks"
    );

    // Verify all returned beads contain "backend" label
    for bead in &backend_tasks {
        assert!(
            bead.id.contains("backend") || bead.title.contains("backend"),
            "All filtered beads should have backend label"
        );
    }

    // Verify exclusion
    assert!(!backend_tasks.iter().any(|c| c.id.contains("frontend")), "frontend tasks should be excluded");
    assert!(!backend_tasks.iter().any(|c| c.id.contains("docs")), "docs tasks should be excluded");
}

#[test]
fn test_filter_by_priority_and_label_composition() {
    let (_temp, storage) = setup_test_db();

    // Create beads with mixed priorities and labels
    create_test_bead(&storage, "bf-backend-p0", "Backend P0", Status::Open, Priority(0));
    create_test_bead(&storage, "bf-backend-p1", "Backend P1", Status::Open, Priority(1));
    create_test_bead(&storage, "bf-frontend-p0", "Frontend P0", Status::Open, Priority(0));
    create_test_bead(&storage, "bf-frontend-p1", "Frontend P1", Status::Open, Priority(1));
    create_test_bead(&storage, "bf-docs-p0", "Docs P0", Status::Open, Priority(0));

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    // Filter to P0 AND backend (composable filter)
    let p0_backend = filter_by_priority_and_label(&candidates, 0, "backend");

    assert_eq!(
        p0_backend.len(),
        1,
        "Should return exactly 1 P0 backend task"
    );

    // Verify the result matches both criteria
    assert_eq!(p0_backend[0].priority, 0, "Should be P0");
    assert!(
        p0_backend[0].id.contains("backend") || p0_backend[0].title.contains("backend"),
        "Should have backend label"
    );
    assert_eq!(p0_backend[0].id, "bf-backend-p0");

    // Verify exclusion: P0 non-backend excluded
    assert!(!p0_backend.iter().any(|c| c.id == "bf-frontend-p0"), "P0 frontend should be excluded");
    assert!(!p0_backend.iter().any(|c| c.id == "bf-docs-p0"), "P0 docs should be excluded");

    // Verify exclusion: backend non-P0 excluded
    assert!(!p0_backend.iter().any(|c| c.id == "bf-backend-p1"), "P1 backend should be excluded");
}

#[test]
fn test_filter_by_priority_and_label_empty_result() {
    let (_temp, storage) = setup_test_db();

    // Create beads that don't match the combination
    create_test_bead(&storage, "bf-backend-p0", "Backend P0", Status::Open, Priority(0));
    create_test_bead(&storage, "bf-frontend-p1", "Frontend P1", Status::Open, Priority(1));

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    // Filter to P2 AND backend (no matching beads)
    let p2_backend = filter_by_priority_and_label(&candidates, 2, "backend");

    assert_eq!(
        p2_backend.len(),
        0,
        "Should return empty result when no beads match both criteria"
    );
}

// =============================================================================
// Test 9: Verify priority ordering within filtered results
// =============================================================================

#[test]
fn test_priority_ordering_maintained_after_filtering() {
    let (_temp, storage) = setup_test_db();

    let base_time = Utc::now();

    // Create P1 beads at different timestamps to test FIFO within priority
    let mut p1_oldest = Issue {
        id: "bf-p1-oldest".to_string(),
        title: "Oldest P1".to_string(),
        status: Status::Open,
        priority: Priority(1),
        issue_type: IssueType::Task,
        created_at: base_time,
        updated_at: base_time,
        source_repo: Some(".".to_string()),
        events: Vec::new(),
        ..Default::default()
    };
    storage.create_issue(&p1_oldest).unwrap();

    let mut p1_middle = Issue {
        id: "bf-p1-middle".to_string(),
        title: "Middle P1".to_string(),
        status: Status::Open,
        priority: Priority(1),
        issue_type: IssueType::Task,
        created_at: base_time + chrono::Duration::seconds(10),
        updated_at: base_time + chrono::Duration::seconds(10),
        source_repo: Some(".".to_string()),
        events: Vec::new(),
        ..Default::default()
    };
    storage.create_issue(&p1_middle).unwrap();

    let mut p1_newest = Issue {
        id: "bf-p1-newest".to_string(),
        title: "Newest P1".to_string(),
        status: Status::Open,
        priority: Priority(1),
        issue_type: IssueType::Task,
        created_at: base_time + chrono::Duration::seconds(20),
        updated_at: base_time + chrono::Duration::seconds(20),
        source_repo: Some(".".to_string()),
        events: Vec::new(),
        ..Default::default()
    };
    storage.create_issue(&p1_newest).unwrap();

    // Create some P2 beads (should be excluded by filter)
    let mut p2_bead = Issue {
        id: "bf-p2-1".to_string(),
        title: "P2 bead".to_string(),
        status: Status::Open,
        priority: Priority(2),
        issue_type: IssueType::Task,
        created_at: base_time - chrono::Duration::seconds(100), // Even older than P1s
        updated_at: base_time - chrono::Duration::seconds(100),
        source_repo: Some(".".to_string()),
        events: Vec::new(),
        ..Default::default()
    };
    storage.create_issue(&p2_bead).unwrap();

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    // All candidates ordered by priority then time
    assert_eq!(candidates.len(), 4);
    assert_eq!(candidates[0].id, "bf-p1-oldest");
    assert_eq!(candidates[1].id, "bf-p1-middle");
    assert_eq!(candidates[2].id, "bf-p1-newest");
    assert_eq!(candidates[3].id, "bf-p2-1");

    // Filter to P1 only
    let p1_only = filter_by_priority(&candidates, 1);

    assert_eq!(p1_only.len(), 3);

    // Verify FIFO order is maintained within the filtered P1 results
    assert_eq!(p1_only[0].id, "bf-p1-oldest", "Oldest P1 should be first");
    assert_eq!(p1_only[1].id, "bf-p1-middle", "Middle P1 should be second");
    assert_eq!(p1_only[2].id, "bf-p1-newest", "Newest P1 should be third");
}

#[test]
fn test_priority_ordering_with_multiple_priorities_filtered() {
    let (_temp, storage) = setup_test_db();

    // Create beads with different priorities and timestamps
    let base_time = Utc::now();

    // Create P0 beads
    for i in 0..3 {
        let mut bead = Issue {
            id: format!("bf-p0-{}", i),
            title: format!("P0 task {}", i),
            status: Status::Open,
            priority: Priority(0),
            issue_type: IssueType::Task,
            created_at: base_time + chrono::Duration::seconds(i as i64 * 10),
            updated_at: base_time + chrono::Duration::seconds(i as i64 * 10),
            source_repo: Some(".".to_string()),
            events: Vec::new(),
            ..Default::default()
        };
        storage.create_issue(&bead).unwrap();
    }

    // Create P1 beads
    for i in 0..3 {
        let mut bead = Issue {
            id: format!("bf-p1-{}", i),
            title: format!("P1 task {}", i),
            status: Status::Open,
            priority: Priority(1),
            issue_type: IssueType::Task,
            created_at: base_time + chrono::Duration::seconds(i as i64 * 10),
            updated_at: base_time + chrono::Duration::seconds(i as i64 * 10),
            source_repo: Some(".".to_string()),
            events: Vec::new(),
            ..Default::default()
        };
        storage.create_issue(&bead).unwrap();
    }

    // Create P2 beads
    for i in 0..3 {
        let mut bead = Issue {
            id: format!("bf-p2-{}", i),
            title: format!("P2 task {}", i),
            status: Status::Open,
            priority: Priority(2),
            issue_type: IssueType::Task,
            created_at: base_time + chrono::Duration::seconds(i as i64 * 10),
            updated_at: base_time + chrono::Duration::seconds(i as i64 * 10),
            source_repo: Some(".".to_string()),
            events: Vec::new(),
            ..Default::default()
        };
        storage.create_issue(&bead).unwrap();
    }

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    // Full ordering: all P0s (FIFO), then all P1s (FIFO), then all P2s (FIFO)
    assert_eq!(candidates.len(), 9);
    let ids: Vec<&str> = candidates.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "bf-p0-0", "bf-p0-1", "bf-p0-2",
            "bf-p1-0", "bf-p1-1", "bf-p1-2",
            "bf-p2-0", "bf-p2-1", "bf-p2-2"
        ]
    );

    // Filter to P0 and P2
    let p0_p2 = filter_by_priorities(&candidates, &[0, 2]);

    assert_eq!(p0_p2.len(), 6);

    // Verify ordering: all P0s (FIFO), then all P2s (FIFO)
    let filtered_ids: Vec<&str> = p0_p2.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        filtered_ids,
        vec!["bf-p0-0", "bf-p0-1", "bf-p0-2", "bf-p2-0", "bf-p2-1", "bf-p2-2"],
        "Priority ordering should be maintained: P0s first (FIFO), then P2s (FIFO)"
    );
}

// =============================================================================
// Test 10: Edge cases for filtering
// =============================================================================

#[test]
fn test_filter_empty_candidates_list() {
    let candidates: Vec<bead_forge::claim::ScoredBead> = Vec::new();

    // Filtering should not panic on empty list
    let p0_only = filter_by_priority(&candidates, 0);
    assert_eq!(p0_only.len(), 0, "Filtering empty list should return empty result");

    let p0_or_p1 = filter_by_priorities(&candidates, &[0, 1]);
    assert_eq!(p0_or_p1.len(), 0, "Filtering empty list should return empty result");
}

#[test]
fn test_filter_no_matches_in_candidates() {
    let (_temp, storage) = setup_test_db();

    // Create only P1 and P2 beads
    create_test_bead(&storage, "bf-p1-1", "P1 task", Status::Open, Priority(1));
    create_test_bead(&storage, "bf-p2-1", "P2 task", Status::Open, Priority(2));

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    // Filter to P0 (no matches)
    let p0_only = filter_by_priority(&candidates, 0);

    assert_eq!(
        p0_only.len(),
        0,
        "Should return empty result when no beads match filter"
    );
}

#[test]
fn test_filter_all_candidates_match() {
    let (_temp, storage) = setup_test_db();

    // Create only P0 beads
    create_test_bead(&storage, "bf-p0-1", "P0 task 1", Status::Open, Priority(0));
    create_test_bead(&storage, "bf-p0-2", "P0 task 2", Status::Open, Priority(0));
    create_test_bead(&storage, "bf-p0-3", "P0 task 3", Status::Open, Priority(0));

    let candidates = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    // Filter to P0 (all match)
    let p0_only = filter_by_priority(&candidates, 0);

    assert_eq!(
        p0_only.len(),
        3,
        "Should return all candidates when all match filter"
    );

    // Verify order is preserved
    let filtered_ids: Vec<&str> = p0_only.iter().map(|c| c.id.as_str()).collect();
    let original_ids: Vec<&str> = candidates.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        filtered_ids, original_ids,
        "Filtering should preserve original order when all items match"
    );
}

// =============================================================================
// Concurrency Tests: Ready queue behavior under concurrent claiming
// =============================================================================

/// Helper function to spawn a concurrent query thread
///
/// This helper spawns a thread that queries the ready queue and returns
/// the bead IDs that were visible at that moment. Used for testing
/// concurrent read operations during write transactions.
fn spawn_concurrent_query(
    storage: std::sync::Arc<Storage>,
    delay_ms: u64,
) -> std::thread::JoinHandle<Vec<String>> {
    std::thread::spawn(move || {
        // Simulate timing variance
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));

        storage
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap()
            .iter()
            .map(|c| c.id.clone())
            .collect()
    })
}

/// Test 1: Query ready queue while another claim is in progress
///
/// This test verifies that read-only queries can proceed while a claim
/// operation (BEGIN IMMEDIATE transaction) is in progress. SQLite's
/// WAL mode allows readers to proceed without blocking on writers.
#[test]
fn test_concurrent_query_during_claim_transaction() {
    use std::sync::Arc;
    use std::thread;

    let (_temp, storage) = setup_test_db();
    let storage = Arc::new(storage);

    // Create 10 open beads
    for i in 0..10 {
        create_test_bead(
            &storage,
            &format!("bf-concurrent-{}", i),
            &format!("Concurrent test bead {}", i),
            Status::Open,
            Priority::MEDIUM,
        );
    }

    // Spawn a thread that will perform a claim (write transaction)
    let storage_clone = Arc::clone(&storage);
    let claim_thread = thread::spawn(move || {
        // Sleep a bit to let readers start first
        std::thread::sleep(std::time::Duration::from_millis(20));

        // Perform claim which uses BEGIN IMMEDIATE
        let result = storage_clone
            .with_immediate_transaction(|tx| {
                Ok(bead_forge::claim::claim(
                    tx,
                    "worker-concurrent-test",
                    30,
                    Utc::now(),
                    None,
                )?)
            })
            .unwrap();

        // Should successfully claim one bead
        assert!(result.is_some(), "Claim should succeed with beads available");
        result.unwrap().bead_id
    });

    // Immediately spawn multiple query threads (read operations)
    // These will run at 0, 5, 10, 15, 25 ms (claim happens at 20ms)
    let mut query_handles = Vec::new();
    for i in 0..5 {
        let delay = if i == 4 { 25 } else { i * 5 }; // Last query after claim
        query_handles.push(spawn_concurrent_query(Arc::clone(&storage), delay));
    }

    // Wait for all operations to complete
    let claimed_bead_id = claim_thread.join().unwrap();
    let mut query_results = Vec::new();
    for handle in query_handles {
        query_results.push(handle.join().unwrap());
    }

    // Verify all queries completed successfully (no deadlocks or blocking)
    assert_eq!(
        query_results.len(),
        5,
        "All concurrent queries should complete successfully"
    );

    // The last query (at 25ms, after claim at 20ms) should NOT see the claimed bead
    let final_query = &query_results[4];
    assert!(
        !final_query.contains(&claimed_bead_id),
        "Query after claim should not see claimed bead {}",
        claimed_bead_id
    );

    // Earlier queries (before/during claim) should have seen 10 beads
    // The last query should see 9 beads (one claimed)
    assert_eq!(
        query_results[0].len(),
        10,
        "First query should see all 10 beads before claim"
    );
    assert_eq!(
        final_query.len(),
        9,
        "Final query should see 9 beads after claim"
    );

    // Verify the claimed bead is no longer in the ready queue
    let ready_after = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();
    assert!(
        !ready_after.iter().any(|c| c.id == claimed_bead_id),
        "Claimed bead {} should not appear in ready queue after claim completes",
        claimed_bead_id
    );
}

/// Test 2: Claim bead then verify it disappears from ready queue
///
/// This test verifies the atomicity of the claim operation: once a bead
/// is claimed, it should immediately disappear from the ready queue for
/// all subsequent queries.
#[test]
fn test_claim_removes_bead_from_ready_queue() {
    use std::sync::Arc;
    use std::thread;

    let (_temp, storage) = setup_test_db();
    let storage = Arc::new(storage);

    // Create 5 open beads
    for i in 0..5 {
        create_test_bead(
            &storage,
            &format!("bf-claim-test-{}", i),
            &format!("Claim test bead {}", i),
            Status::Open,
            Priority::MEDIUM,
        );
    }

    // Query ready queue before claim
    let before_claim = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(
        before_claim.len(),
        5,
        "Should have 5 beads in ready queue before claim"
    );

    // Claim a bead (this will be the first one due to priority ordering)
    let claimed = storage
        .with_immediate_transaction(|tx| {
            Ok(bead_forge::claim::claim(
                tx,
                "worker-claim-removal-test",
                30,
                Utc::now(),
                None,
            )?)
        })
        .unwrap();

    assert!(claimed.is_some(), "Claim should succeed");
    let claimed_bead_id = claimed.unwrap().bead_id;

    // Spawn multiple threads to verify the bead is gone from all perspectives
    let storage_clone = Arc::clone(&storage);
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let s = Arc::clone(&storage_clone);
            thread::spawn(move || {
                s.with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
                    .unwrap()
                    .iter()
                    .map(|c| c.id.clone())
                    .collect::<Vec<_>>()
            })
        })
        .collect();

    // Verify all threads see the claimed bead is missing
    for handle in handles {
        let bead_ids = handle.join().unwrap();
        assert!(
            !bead_ids.contains(&claimed_bead_id),
            "Claimed bead {} should not appear in ready queue from any thread",
            claimed_bead_id
        );
        assert_eq!(
            bead_ids.len(),
            4,
            "Ready queue should have exactly 4 beads after claim"
        );
    }

    // Verify the claimed bead is now in_progress
    let claimed_issue = storage.get_issue(&claimed_bead_id).unwrap().unwrap();
    assert_eq!(
        claimed_issue.status,
        Status::InProgress,
        "Claimed bead should be in_progress"
    );
    assert_eq!(
        claimed_issue.assignee.as_ref().unwrap(),
        "worker-claim-removal-test",
        "Claimed bead should be assigned to the worker"
    );
}

/// Test 3: Multiple concurrent queries return consistent snapshots
///
/// This test verifies that multiple threads querying the ready queue
/// simultaneously see consistent state - no race conditions or
/// partial updates should be visible.
#[test]
fn test_multiple_concurrent_queries_consistent_snapshots() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let (_temp, storage) = setup_test_db();
    let storage = Arc::new(storage);

    // Create 20 open beads with different priorities
    for i in 0..20 {
        let priority = Priority(i % 5); // Mix of priorities 0-4
        create_test_bead(
            &storage,
            &format!("bf-snapshot-{:0>2}", i),
            &format!("Snapshot test bead {}", i),
            Status::Open,
            priority,
        );
    }

    // Spawn 15 concurrent query threads
    let results: Arc<Mutex<Vec<Vec<String>>>> = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();

    for i in 0..15 {
        let storage_clone = Arc::clone(&storage);
        let results_clone = Arc::clone(&results);

        let handle = thread::spawn(move || {
            // Add slight timing variation
            std::thread::sleep(std::time::Duration::from_micros((i * 100) as u64));

            let bead_ids = storage_clone
                .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
                .unwrap()
                .iter()
                .map(|c| c.id.clone())
                .collect::<Vec<_>>();

            let mut results = results_clone.lock().unwrap();
            results.push(bead_ids);
        });

        handles.push(handle);
    }

    // Wait for all queries to complete
    for handle in handles {
        handle.join().unwrap();
    }

    let all_results = results.lock().unwrap();

    // All queries should return the same number of beads
    for query_result in all_results.iter() {
        assert_eq!(
            query_result.len(),
            20,
            "Each concurrent query should see all 20 beads"
        );
    }

    // Verify all queries see the same set of beads
    let first_result = &all_results[0];
    let first_set: std::collections::HashSet<_> = first_result.iter().collect();

    for query_result in all_results.iter() {
        let current_set: std::collections::HashSet<_> = query_result.iter().collect();
        assert_eq!(
            first_set, current_set,
            "All concurrent queries should see the same bead IDs"
        );
    }

    // Verify ordering is consistent (priority-based)
    for query_result in all_results.iter() {
        // All queries should return beads ordered by priority
        let mut prev_priority = None;
        for bead_id in query_result {
            // Extract priority from bead ID (format: bf-snapshot-XX where XX = priority)
            // Actually we need to look at the bead itself to check priority
            let issue = storage.get_issue(bead_id).unwrap().unwrap();
            let current_priority = issue.priority.0;

            if let Some(prev) = prev_priority {
                assert!(
                    current_priority >= prev,
                    "Beads should be ordered by priority (ascending), but found {} after {}",
                    current_priority, prev
                );
            }
            prev_priority = Some(current_priority);
        }
    }

    assert_eq!(
        all_results.len(),
        15,
        "All 15 concurrent queries should complete successfully"
    );
}

/// Test 4: BEGIN IMMEDIATE transaction doesn't block read-only queries
///
/// This test specifically verifies that write transactions (BEGIN IMMEDIATE)
/// do not block read-only queries. In WAL mode, readers should never block
/// on writers or other readers.
#[test]
fn test_immediate_transaction_doesnt_block_read_queries() {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    let (_temp, storage) = setup_test_db();
    let storage = Arc::new(storage);

    // Create some beads
    for i in 0..5 {
        create_test_bead(
            &storage,
            &format!("bf-blocking-{}", i),
            &format!("Non-blocking test bead {}", i),
            Status::Open,
            Priority::MEDIUM,
        );
    }

    // Start a write transaction (BEGIN IMMEDIATE) but keep it open
    let storage_clone = Arc::clone(&storage);
    let write_thread = thread::spawn(move || {
        storage_clone
            .with_immediate_transaction(|tx| {
                // BEGIN IMMEDIATE acquires a reserved lock
                // Sleep while holding the transaction
                thread::sleep(Duration::from_millis(50));

                // Do a write operation
                tx.execute(
                    "UPDATE issues SET updated_at = ?1 WHERE id = ?2",
                    params![Utc::now().to_rfc3339(), "bf-blocking-0"],
                )
                .ok();

                Ok(())
            })
            .unwrap();
    });

    // Immediately spawn a read query while write is still in progress
    let storage_read = Arc::clone(&storage);
    let read_thread = thread::spawn(move || {
        // Small delay to ensure write transaction starts first
        thread::sleep(Duration::from_millis(10));

        let start = std::time::Instant::now();
        let bead_ids = storage_read
            .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
            .unwrap()
            .iter()
            .map(|c| c.id.clone())
            .collect::<Vec<_>>();
        let elapsed = start.elapsed();

        (bead_ids, elapsed)
    });

    // Wait for both operations
    write_thread.join().unwrap();
    let (read_beads, read_duration) = read_thread.join().unwrap();

    // Verify read completed successfully
    assert_eq!(
        read_beads.len(),
        5,
        "Read should still see all 5 beads despite concurrent write"
    );

    // Verify read didn't block on the write transaction
    // In WAL mode, reads should proceed without waiting for writes to complete
    assert!(
        read_duration < Duration::from_millis(100),
        "Read query should not block on write transaction (took {:?}, expected < 100ms)",
        read_duration
    );

    // Verify the read didn't fail or timeout
    assert!(
        read_beads.contains(&"bf-blocking-0".to_string()),
        "Read should see all beads including the one being updated"
    );
}

/// Test 5: Concurrent claim operations don't create race conditions
///
/// This test verifies that multiple workers claiming simultaneously
/// correctly handle race conditions - each bead is claimed exactly once,
/// and no two workers claim the same bead.
#[test]
fn test_concurrent_claims_no_race_conditions() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let (_temp, storage) = setup_test_db();
    let storage = Arc::new(storage);

    // Create 20 open beads
    for i in 0..20 {
        create_test_bead(
            &storage,
            &format!("bf-race-{}", i),
            &format!("Race condition test bead {}", i),
            Status::Open,
            Priority::MEDIUM,
        );
    }

    let claimed_beads: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();

    // Spawn 20 workers trying to claim concurrently
    for worker_id in 0..20 {
        let storage_clone = Arc::clone(&storage);
        let claimed_clone = Arc::clone(&claimed_beads);

        let handle = thread::spawn(move || {
            let result = storage_clone
                .with_immediate_transaction(|tx| {
                    Ok(bead_forge::claim::claim(
                        tx,
                        &format!("worker-race-{}", worker_id),
                        30,
                        Utc::now(),
                        None,
                    )?)
                })
                .unwrap();

            if let Some(claim_result) = result {
                let mut claimed = claimed_clone.lock().unwrap();
                claimed.push(claim_result.bead_id);
            }
        });

        handles.push(handle);
    }

    // Wait for all workers to complete
    for handle in handles {
        handle.join().unwrap();
    }

    let claimed = claimed_beads.lock().unwrap();

    // All 20 beads should be claimed exactly once
    assert_eq!(
        claimed.len(),
        20,
        "All 20 beads should be claimed (got {})",
        claimed.len()
    );

    // Verify no duplicates - each bead claimed at most once
    let mut unique_beads = claimed.clone();
    unique_beads.sort();
    unique_beads.dedup();
    assert_eq!(
        unique_beads.len(),
        20,
        "No duplicates allowed - found duplicate claims in {:?}",
        claimed
    );

    // Verify all claimed beads are now in_progress
    for bead_id in claimed.iter() {
        let issue = storage.get_issue(bead_id).unwrap().unwrap();
        assert_eq!(
            issue.status,
            Status::InProgress,
            "Claimed bead {} should be in_progress",
            bead_id
        );
    }

    // Verify ready queue is now empty
    let ready_beads = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(
        ready_beads.len(),
        0,
        "Ready queue should be empty after all beads claimed"
    );
}
