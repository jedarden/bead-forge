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
    let issue = Issue {
        id: id.to_string(),
        title: title.to_string(),
        status,
        priority,
        issue_type: IssueType::Task,
        created_at: Utc::now(),
        updated_at: Utc::now(),
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
            Status::Open,
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
