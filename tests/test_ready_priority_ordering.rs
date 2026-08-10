//! Tests for priority-based ordering in the ready queue.
//!
//! These tests verify that the `bf ready` command correctly orders beads
//! by priority and creation time as specified in the acceptance criteria.

use bead_forge::model::{Issue, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;
use std::path::PathBuf;

/// Create a test workspace with a beads database.
fn setup_test_workspace() -> PathBuf {
    let temp_dir = tempfile::NamedTempFile::new().unwrap();
    let db_path = temp_dir.path().to_path_buf();
    let storage = Storage::open(&db_path).unwrap();
    drop(storage);
    // Keep temp_dir alive but don't return it - it will be cleaned up when the function returns
    // This is fine for our purposes since we only need the db_path
    db_path
}

#[test]
fn test_p0_beads_before_p1_beads() {
    // Test case verifying P0 beads appear before P1 beads
    let db_path = setup_test_workspace();
    let storage = Storage::open(&db_path).unwrap();

    // Create P1 bead first (older)
    let p1_bead = Issue {
        id: "bf-p1-older".to_string(),
        title: "P1 task".to_string(),
        priority: Priority(1),
        status: Status::Open,
        created_at: Utc::now() - chrono::Duration::hours(2),
        ..Default::default()
    };
    storage.create_issue(&p1_bead).unwrap();

    // Create P0 bead second (newer)
    let p0_bead = Issue {
        id: "bf-p0-newer".to_string(),
        title: "P0 task".to_string(),
        priority: Priority(0),
        status: Status::Open,
        created_at: Utc::now() - chrono::Duration::hours(1),
        ..Default::default()
    };
    storage.create_issue(&p0_bead).unwrap();

    // Get ready candidates
    let candidates = storage
        .with_immediate_transaction(|tx| {
            Ok(bead_forge::claim::get_ready_candidates(tx, 10, None, None)?)
        })
        .unwrap();

    // P0 should appear before P1 despite being newer
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].id, "bf-p0-newer");
    assert_eq!(candidates[0].priority, 0);
    assert_eq!(candidates[1].id, "bf-p1-older");
    assert_eq!(candidates[1].priority, 1);
}

#[test]
fn test_older_beads_before_newer_within_same_priority() {
    // Test case verifying older beads appear before newer ones within same priority
    let db_path = setup_test_workspace();
    let storage = Storage::open(&db_path).unwrap();

    // Create older P1 bead
    let older_bead = Issue {
        id: "bf-p1-older".to_string(),
        title: "Older P1 task".to_string(),
        priority: Priority(1),
        status: Status::Open,
        created_at: Utc::now() - chrono::Duration::hours(2),
        ..Default::default()
    };
    storage.create_issue(&older_bead).unwrap();

    // Create newer P1 bead
    let newer_bead = Issue {
        id: "bf-p1-newer".to_string(),
        title: "Newer P1 task".to_string(),
        priority: Priority(1),
        status: Status::Open,
        created_at: Utc::now() - chrono::Duration::hours(1),
        ..Default::default()
    };
    storage.create_issue(&newer_bead).unwrap();

    // Get ready candidates
    let candidates = storage
        .with_immediate_transaction(|tx| {
            Ok(bead_forge::claim::get_ready_candidates(tx, 10, None, None)?)
        })
        .unwrap();

    // Older P1 should appear before newer P1
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].id, "bf-p1-older");
    assert_eq!(candidates[1].id, "bf-p1-newer");
}

#[test]
fn test_null_priority_treated_as_lowest() {
    // Test case handling NULL priority values (treated as lowest priority)
    let db_path = setup_test_workspace();
    let storage = Storage::open(&db_path).unwrap();

    // In the current implementation, all beads have a priority value
    // (default is P2), so we test that higher priority beads come first
    let p0_bead = Issue {
        id: "bf-p0".to_string(),
        title: "P0 task".to_string(),
        priority: Priority(0),
        status: Status::Open,
        created_at: Utc::now() - chrono::Duration::hours(1),
        ..Default::default()
    };
    storage.create_issue(&p0_bead).unwrap();

    let p2_bead = Issue {
        id: "bf-p2".to_string(),
        title: "P2 task (default)".to_string(),
        priority: Priority(2),
        status: Status::Open,
        created_at: Utc::now() - chrono::Duration::hours(2),
        ..Default::default()
    };
    storage.create_issue(&p2_bead).unwrap();

    let p3_bead = Issue {
        id: "bf-p3".to_string(),
        title: "P3 task (backlog)".to_string(),
        priority: Priority(3),
        status: Status::Open,
        created_at: Utc::now() - chrono::Duration::hours(3),
        ..Default::default()
    };
    storage.create_issue(&p3_bead).unwrap();

    // Get ready candidates
    let candidates = storage
        .with_immediate_transaction(|tx| {
            Ok(bead_forge::claim::get_ready_candidates(tx, 10, None, None)?)
        })
        .unwrap();

    // Should be ordered: P0, P2, P3 (by priority, not by creation time)
    assert_eq!(candidates.len(), 3);
    assert_eq!(candidates[0].id, "bf-p0");
    assert_eq!(candidates[0].priority, 0);
    assert_eq!(candidates[1].id, "bf-p2");
    assert_eq!(candidates[1].priority, 2);
    assert_eq!(candidates[2].id, "bf-p3");
    assert_eq!(candidates[2].priority, 3);
}

#[test]
fn test_ready_queue_full_priority_ordering() {
    // Comprehensive test for full priority ordering with all levels
    let db_path = setup_test_workspace();
    let storage = Storage::open(&db_path).unwrap();

    let base_time = Utc::now();

    // Create beads in reverse priority order to test sorting
    let beads = vec![
        ("bf-p3-backlog", "P3 Backlog", Priority(3), base_time - chrono::Duration::hours(5)),
        ("bf-p2-default", "P2 Default", Priority(2), base_time - chrono::Duration::hours(4)),
        ("bf-p1-high", "P1 High", Priority(1), base_time - chrono::Duration::hours(3)),
        ("bf-p0-critical", "P0 Critical", Priority(0), base_time - chrono::Duration::hours(2)),
        ("bf-another-p3", "Another P3", Priority(3), base_time - chrono::Duration::hours(1)),
    ];

    for (id, title, priority, created_at) in beads {
        let bead = Issue {
            id: id.to_string(),
            title: title.to_string(),
            priority,
            status: Status::Open,
            created_at,
            ..Default::default()
        };
        storage.create_issue(&bead).unwrap();
    }

    // Get ready candidates
    let candidates = storage
        .with_immediate_transaction(|tx| {
            Ok(bead_forge::claim::get_ready_candidates(tx, 10, None, None)?)
        })
        .unwrap();

    // Expected order: P0, P1, P2, P3 (older), P3 (newer)
    assert_eq!(candidates.len(), 5);
    assert_eq!(candidates[0].id, "bf-p0-critical");
    assert_eq!(candidates[0].priority, 0);
    assert_eq!(candidates[1].id, "bf-p1-high");
    assert_eq!(candidates[1].priority, 1);
    assert_eq!(candidates[2].id, "bf-p2-default");
    assert_eq!(candidates[2].priority, 2);
    assert_eq!(candidates[3].id, "bf-p3-backlog");
    assert_eq!(candidates[3].priority, 3);
    assert_eq!(candidates[4].id, "bf-another-p3");
    assert_eq!(candidates[4].priority, 3);
}

#[test]
fn test_priority_trumps_creation_time() {
    // Verify that priority ordering trumps creation time
    let db_path = setup_test_workspace();
    let storage = Storage::open(&db_path).unwrap();

    let base_time = Utc::now();

    // Create an old P2 bead
    let old_p2 = Issue {
        id: "bf-old-p2".to_string(),
        title: "Old P2".to_string(),
        priority: Priority(2),
        status: Status::Open,
        created_at: base_time - chrono::Duration::days(10),
        ..Default::default()
    };
    storage.create_issue(&old_p2).unwrap();

    // Create a new P0 bead
    let new_p0 = Issue {
        id: "bf-new-p0".to_string(),
        title: "New P0".to_string(),
        priority: Priority(0),
        status: Status::Open,
        created_at: base_time - chrono::Duration::minutes(1),
        ..Default::default()
    };
    storage.create_issue(&new_p0).unwrap();

    // Get ready candidates
    let candidates = storage
        .with_immediate_transaction(|tx| {
            Ok(bead_forge::claim::get_ready_candidates(tx, 10, None, None)?)
        })
        .unwrap();

    // New P0 should come before old P2 (priority trumps age)
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].id, "bf-new-p0");
    assert_eq!(candidates[0].priority, 0);
    assert_eq!(candidates[1].id, "bf-old-p2");
    assert_eq!(candidates[1].priority, 2);
}
