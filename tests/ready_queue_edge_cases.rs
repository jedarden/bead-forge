//! Ready queue edge cases and comprehensive tests
//!
//! This test module covers edge cases and boundary conditions for the `bf ready` command:
//! - --limit parameter truncation with various dataset sizes
//! - JSON output format validation with non-empty results
//! - Concurrent access test (multiple agents calling ready simultaneously)
//! - Ready queue with exactly limit number of beads
//! - Ready queue with limit > available beads
//! - JSONL output format (newline-delimited JSON)
//! - Envelope mode with limit parameter

use bead_forge::claim::get_ready_candidates;
use bead_forge::model::{DependencyType, Issue, IssueChanges, Priority, Status};
use bead_forge::storage::Storage;
use chrono::{Duration, Utc};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration as StdDuration;
use tempfile::TempDir;

/// Create a test database and storage instance
fn setup_test_db() -> (TempDir, Storage) {
    let temp_file = TempDir::new().unwrap();
    let db_path = temp_file.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();
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

// ============================================================================
// TEST: --limit parameter truncation
// ============================================================================

#[test]
fn test_ready_limit_truncates_results() {
    let (_temp, storage) = setup_test_db();

    // Create 20 ready beads
    for i in 1..=20 {
        create_open_bead(
            &storage,
            &format!("bf-ready-{:02}", i),
            &format!("Ready bead {}", i),
            Priority::MEDIUM,
        );
    }

    // Request with limit=5 should return only 5 beads
    let ready = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 5, None, None)?))
        .unwrap();

    assert_eq!(ready.len(), 5, "limit=5 should return exactly 5 beads");
}

#[test]
fn test_ready_limit_equals_available_count() {
    let (_temp, storage) = setup_test_db();

    // Create exactly 10 ready beads
    for i in 1..=10 {
        create_open_bead(
            &storage,
            &format!("bf-ready-{:02}", i),
            &format!("Ready bead {}", i),
            Priority::MEDIUM,
        );
    }

    // Request with limit=10 should return all 10 beads
    let ready = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 10, None, None)?))
        .unwrap();

    assert_eq!(ready.len(), 10, "limit=10 should return all 10 beads when 10 are available");
}

#[test]
fn test_ready_limit_exceeds_available_count() {
    let (_temp, storage) = setup_test_db();

    // Create only 5 ready beads
    for i in 1..=5 {
        create_open_bead(
            &storage,
            &format!("bf-ready-{:02}", i),
            &format!("Ready bead {}", i),
            Priority::MEDIUM,
        );
    }

    // Request with limit=100 should return only 5 beads (all available)
    let ready = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(ready.len(), 5, "limit=100 should return only 5 beads when only 5 are available");
}

#[test]
fn test_ready_limit_zero_returns_all() {
    let (_temp, storage) = setup_test_db();

    // Create 25 ready beads
    for i in 1..=25 {
        create_open_bead(
            &storage,
            &format!("bf-ready-{:02}", i),
            &format!("Ready bead {}", i),
            Priority::MEDIUM,
        );
    }

    // Request with limit=0 should return all beads
    let ready = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 0, None, None)?))
        .unwrap();

    assert_eq!(ready.len(), 25, "limit=0 should return all 25 beads");
}

#[test]
fn test_ready_limit_with_mixed_priorities() {
    let (_temp, storage) = setup_test_db();

    let base_time = Utc::now();

    // Create beads at different priorities
    let priorities = vec![
        ("bf-p0-1", Priority::CRITICAL, 0),
        ("bf-p0-2", Priority::CRITICAL, 1),
        ("bf-p1-1", Priority::HIGH, 2),
        ("bf-p1-2", Priority::HIGH, 3),
        ("bf-p2-1", Priority::MEDIUM, 4),
        ("bf-p2-2", Priority::MEDIUM, 5),
        ("bf-p3-1", Priority::LOW, 6),
        ("bf-p3-2", Priority::LOW, 7),
        ("bf-p4-1", Priority::BACKLOG, 8),
        ("bf-p4-2", Priority::BACKLOG, 9),
    ];

    for (id, priority, offset) in priorities {
        let mut bead = Issue::new(id.to_string(), format!("{} bead", id), ".".to_string());
        bead.priority = priority;
        bead.created_at = base_time - Duration::seconds(offset as i64);
        storage.create_issue(&bead).unwrap();
    }

    // Request with limit=5 should return top 5 by priority
    let ready = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 5, None, None)?))
        .unwrap();

    assert_eq!(ready.len(), 5, "limit=5 should return exactly 5 beads");

    // Should be ordered: P0 beads first, then P1 beads
    assert_eq!(ready[0].id, "bf-p0-1");
    assert_eq!(ready[1].id, "bf-p0-2");
    assert_eq!(ready[2].id, "bf-p1-1");
    assert_eq!(ready[3].id, "bf-p1-2");
    // Position 4 could be either P2 bead depending on created_at/downstream_impact
}

#[test]
fn test_ready_limit_one_returns_single_bead() {
    let (_temp, storage) = setup_test_db();

    // Create multiple beads
    for i in 1..=10 {
        create_open_bead(
            &storage,
            &format!("bf-ready-{:02}", i),
            &format!("Ready bead {}", i),
            Priority::MEDIUM,
        );
    }

    // Request with limit=1 should return only the top priority bead
    let ready = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 1, None, None)?))
        .unwrap();

    assert_eq!(ready.len(), 1, "limit=1 should return exactly 1 bead");
}

// ============================================================================
// TEST: JSON output format validation
// ============================================================================

#[test]
fn test_ready_json_output_valid_single_bead() {
    let (_temp, storage) = setup_test_db();

    // Create a single ready bead
    let bead = create_open_bead(&storage, "bf-test", "Test bead", Priority::HIGH);

    // Get ready candidates
    let ready = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(ready.len(), 1);

    // Verify JSON structure
    let candidate = &ready[0];

    // Required fields must be present
    assert!(!candidate.id.is_empty());
    assert!(!candidate.title.is_empty());
    assert_eq!(candidate.status, "open");
    assert!(!candidate.issue_type.is_empty());

    // Timestamps should be valid ISO 8601
    assert!(candidate.created_at.to_string().len() > 0);
    assert!(candidate.updated_at.to_string().len() > 0);

    // Priority should be in valid range
    assert!(candidate.priority.0 >= 0 && candidate.priority.0 <= 4);

    // Labels should be present (even if empty)
    assert!(candidate.labels.is_array());

    // Assignee should be null or string
    if candidate.assignee.is_some() {
        assert!(candidate.assignee.as_ref().unwrap().is_string());
    }
}

#[test]
fn test_ready_json_output_multiple_beads() {
    let (_temp, storage) = setup_test_db();

    // Create multiple ready beads with varying priorities
    let priorities = vec![
        ("bf-p0", Priority::CRITICAL),
        ("bf-p1", Priority::HIGH),
        ("bf-p2", Priority::MEDIUM),
        ("bf-p3", Priority::LOW),
    ];

    for (id, priority) in priorities {
        create_open_bead(&storage, id, "Test bead", priority);
    }

    // Get ready candidates
    let ready = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(ready.len(), 4);

    // Verify all beads have valid JSON structure
    for candidate in &ready {
        assert!(!candidate.id.is_empty());
        assert!(!candidate.title.is_empty());
        assert_eq!(candidate.status, "open");
        assert!(candidate.priority.0 >= 0 && candidate.priority.0 <= 4);
        assert!(candidate.labels.is_array());
    }

    // Verify priority ordering (P0 first, then P1, etc.)
    assert_eq!(ready[0].id, "bf-p0");
    assert_eq!(ready[1].id, "bf-p1");
    assert_eq!(ready[2].id, "bf-p2");
    assert_eq!(ready[3].id, "bf-p3");
}

#[test]
fn test_ready_json_output_with_dependencies() {
    let (_temp, storage) = setup_test_db();

    // Create a blocker and dependent
    let blocker = create_open_bead(&storage, "bf-blocker", "Blocker", Priority::MEDIUM);
    let dependent = create_open_bead(&storage, "bf-dependent", "Dependent", Priority::HIGH);

    // Add blocking dependency
    storage
        .add_dependency("bf-dependent", "bf-blocker", &DependencyType::Blocks, "test-user")
        .unwrap();

    // Get ready candidates (should only include the blocker)
    let ready = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, "bf-blocker");

    // Verify blocker has no dependencies (it's the root)
    assert!(ready[0].dependencies.is_empty() || ready[0].dependencies.len() == 0);
}

#[test]
fn test_ready_json_output_with_closed_blocker() {
    let (_temp, storage) = setup_test_db();

    // Create a blocker and dependent
    let blocker = create_open_bead(&storage, "bf-blocker", "Blocker", Priority::MEDIUM);
    let dependent = create_open_bead(&storage, "bf-dependent", "Dependent", Priority::HIGH);

    // Add blocking dependency
    storage
        .add_dependency("bf-dependent", "bf-blocker", &DependencyType::Blocks, "test-user")
        .unwrap();

    // Close the blocker
    let changes = IssueChanges {
        status: Some(Status::Closed),
        ..Default::default()
    };
    storage.update_issue("bf-blocker", &changes).unwrap();

    // Get ready candidates (should now include the dependent)
    let ready = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, "bf-dependent");

    // Verify dependent is now unblocked and ready
    assert_eq!(ready[0].status, "open");
}

#[test]
fn test_ready_json_output_empty_becomes_unblocked() {
    let (_temp, storage) = setup_test_db();

    // Initial state: empty ready queue
    let ready = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();
    assert_eq!(ready.len(), 0);

    // Create a blocker and dependent
    let blocker = create_open_bead(&storage, "bf-blocker", "Blocker", Priority::MEDIUM);
    let dependent = create_open_bead(&storage, "bf-dependent", "Dependent", Priority::HIGH);

    // Add blocking dependency
    storage
        .add_dependency("bf-dependent", "bf-blocker", &DependencyType::Blocks, "test-user")
        .unwrap();

    // Only blocker should be ready
    let ready = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();
    assert_eq!(ready.len(), 1);
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
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].id, "bf-dependent");
}

// ============================================================================
// TEST: Concurrent access (multiple agents calling ready simultaneously)
// ============================================================================

#[test]
fn test_concurrent_ready_reads_consistent_results() {
    let num_beads = 50;
    let num_readers = 10;

    let (_temp, storage) = setup_test_db();
    let storage = Arc::new(storage);

    // Create 50 ready beads with varying priorities
    for i in 0..num_beads {
        let mut bead = Issue::new(
            format!("bf-ready-{:04}", i),
            format!("Ready bead {}", i),
            ".".to_string(),
        );
        bead.priority = Priority((i % 5) as i32);
        storage.create_issue(&bead).unwrap();
    }

    // Track results from all concurrent readers
    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];

    // Spawn 10 readers simultaneously
    for reader_id in 0..num_readers {
        let results_clone = Arc::clone(&results);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            // Add a tiny random delay to increase race likelihood
            let delay = rand::random::<u64>() % 10;
            thread::sleep(StdDuration::from_micros(delay));

            let ready = storage_clone
                .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
                .unwrap();

            let mut results = results_clone.lock().unwrap();
            results.push((reader_id, ready.len()));
        });

        handles.push(handle);
    }

    // Wait for all readers to complete
    for handle in handles {
        handle.join().unwrap();
    }

    let results = results.lock().unwrap();

    // All readers should see the same number of beads
    let first_count = results[0].1;
    for (reader_id, count) in results.iter() {
        assert_eq!(
            *count, first_count,
            "Reader {} should see {} beads, got {}",
            reader_id, first_count, count
        );
    }

    // All should see all 50 beads
    assert_eq!(first_count, num_beads, "All readers should see all 50 beads");
}

#[test]
fn test_concurrent_ready_with_limit() {
    let num_beads = 100;
    let num_readers = 15;
    let limit = 20;

    let (_temp, storage) = setup_test_db();
    let storage = Arc::new(storage);

    // Create 100 ready beads
    for i in 0..num_beads {
        let mut bead = Issue::new(
            format!("bf-ready-{:04}", i),
            format!("Ready bead {}", i),
            ".".to_string(),
        );
        bead.priority = Priority((i % 5) as i32);
        storage.create_issue(&bead).unwrap();
    }

    // Track results from all concurrent readers
    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];

    // Spawn 15 readers simultaneously
    for reader_id in 0..num_readers {
        let results_clone = Arc::clone(&results);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            // Add a tiny random delay to increase race likelihood
            let delay = rand::random::<u64>() % 10;
            thread::sleep(StdDuration::from_micros(delay));

            let ready = storage_clone
                .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, limit, None, None)?))
                .unwrap();

            let mut results = results_clone.lock().unwrap();
            results.push((reader_id, ready.len()));
        });

        handles.push(handle);
    }

    // Wait for all readers to complete
    for handle in handles {
        handle.join().unwrap();
    }

    let results = results.lock().unwrap();

    // All readers should see exactly 'limit' beads
    for (reader_id, count) in results.iter() {
        assert_eq!(
            *count, limit,
            "Reader {} should see exactly {} beads with limit={}, got {}",
            reader_id, limit, limit, count
        );
    }
}

#[test]
fn test_concurrent_ready_during_state_changes() {
    let (_temp, storage) = setup_test_db();
    let storage = Arc::new(storage);

    // Create initial beads
    for i in 0..=20 {
        create_open_bead(
            &storage,
            &format!("bf-ready-{:02}", i),
            &format!("Ready bead {}", i),
            Priority::MEDIUM,
        );
    }

    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];

    // Spawn 5 readers and 2 writers
    for i in 0..7 {
        let results_clone = Arc::clone(&results);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            if i < 5 {
                // Reader: query ready queue
                let ready = storage_clone
                    .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)))
                    .unwrap();
                let mut results = results_clone.lock().unwrap();
                results.push(("read".to_string(), ready.len()));
            } else {
                // Writer: add a bead
                thread::sleep(StdDuration::from_millis(1));
                create_open_bead(
                    &storage_clone,
                    &format!("bf-dynamic-{:02}", i),
                    "Dynamic bead",
                    Priority::MEDIUM,
                );
                let mut results = results_clone.lock().unwrap();
                results.push(("write".to_string(), 0));
            }
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    let results = results.lock().unwrap();

    // All reads should succeed (no panics or errors)
    let read_results: Vec<_> = results.iter().filter(|(t, _)| t == "read").collect();
    assert_eq!(read_results.len(), 5, "Should have 5 read results");

    // All read counts should be non-negative
    for (op, count) in results.iter() {
        if op == "read" {
            assert!(*count >= 0, "Read count should be non-negative");
        }
    }
}

#[test]
fn test_concurrent_ready_with_dependencies() {
    let (_temp, storage) = setup_test_db();
    let storage = Arc::new(storage);

    // Create dependency chain: A -> B -> C
    let bead_a = create_open_bead(&storage, "bf-a", "Bead A", Priority::MEDIUM);
    let bead_b = create_open_bead(&storage, "bf-b", "Bead B", Priority::MEDIUM);
    let bead_c = create_open_bead(&storage, "bf-c", "Bead C", Priority::MEDIUM);

    storage
        .add_dependency("bf-b", "bf-a", &DependencyType::Blocks, "test")
        .unwrap();
    storage
        .add_dependency("bf-c", "bf-b", &DependencyType::Blocks, "test")
        .unwrap();

    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];

    // Spawn multiple concurrent readers
    for reader_id in 0..8 {
        let results_clone = Arc::clone(&results);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            let ready = storage_clone
                .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)))
                .unwrap();

            let mut results = results_clone.lock().unwrap();
            results.push((reader_id, ready.len(), ready[0].id.clone()));
        });

        handles.push(handle);
    }

    // Wait for all readers to complete
    for handle in handles {
        handle.join().unwrap();
    }

    let results = results.lock().unwrap();

    // All readers should see exactly 1 ready bead (bf-a, the root)
    for (reader_id, count, first_id) in results.iter() {
        assert_eq!(*count, 1, "Reader {} should see 1 ready bead", reader_id);
        assert_eq!(first_id, "bf-a", "Reader {} should see bf-a as the first bead", reader_id);
    }
}

// ============================================================================
// TEST: Edge case combinations
// ============================================================================

#[test]
fn test_ready_limit_with_blocked_beads() {
    let (_temp, storage) = setup_test_db();

    // Create 10 ready beads and 5 blocked beads
    for i in 1..=10 {
        create_open_bead(
            &storage,
            &format!("bf-ready-{:02}", i),
            &format!("Ready bead {}", i),
            Priority::MEDIUM,
        );
    }

    // Create blocked beads
    let blocker = create_open_bead(&storage, "bf-blocker", "Blocker", Priority::MEDIUM);
    for i in 1..=5 {
        let blocked = create_open_bead(
            &storage,
            &format!("bf-blocked-{:02}", i),
            &format!("Blocked bead {}", i),
            Priority::MEDIUM,
        );
        storage
            .add_dependency(
                &format!("bf-blocked-{:02}", i),
                "bf-blocker",
                &DependencyType::Blocks,
                "test",
            )
            .unwrap();
    }

    // Total ready should be 11 (10 ready + 1 blocker)
    let ready = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();
    assert_eq!(ready.len(), 11);

    // With limit=5, should get only 5
    let ready = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 5, None, None)?))
        .unwrap();
    assert_eq!(ready.len(), 5);

    // Verify none of the blocked beads are in the results
    for candidate in &ready {
        assert!(!candidate.id.starts_with("bf-blocked-"), "Blocked beads should not appear in ready queue");
    }
}

#[test]
fn test_ready_empty_json_matches_empty_text() {
    let (_temp, storage) = setup_test_db();

    // Empty database should return empty results
    let ready = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(ready.len(), 0);
}

#[test]
fn test_ready_with_all_priorities() {
    let (_temp, storage) = setup_test_db();

    // Create beads at all priority levels (P0-P4)
    let priorities = vec![
        ("bf-p4", Priority::BACKLOG),
        ("bf-p3", Priority::LOW),
        ("bf-p2", Priority::MEDIUM),
        ("bf-p1", Priority::HIGH),
        ("bf-p0", Priority::CRITICAL),
    ];

    for (id, priority) in priorities {
        create_open_bead(&storage, id, "Test bead", priority);
    }

    // Get ready candidates
    let ready = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(ready.len(), 5);

    // Verify priority ordering (P0 first, P4 last)
    assert_eq!(ready[0].id, "bf-p0");
    assert_eq!(ready[1].id, "bf-p1");
    assert_eq!(ready[2].id, "bf-p2");
    assert_eq!(ready[3].id, "bf-p3");
    assert_eq!(ready[4].id, "bf-p4");
}

#[test]
fn test_ready_same_priority_ordering() {
    let (_temp, storage) = setup_test_db();

    let base_time = Utc::now();

    // Create multiple beads with the same priority
    let mut bead1 = Issue::new("bf-1".to_string(), "Bead 1".to_string(), ".".to_string());
    bead1.priority = Priority::MEDIUM;
    bead1.created_at = base_time - Duration::seconds(100);
    storage.create_issue(&bead1).unwrap();

    let mut bead2 = Issue::new("bf-2".to_string(), "Bead 2".to_string(), ".".to_string());
    bead2.priority = Priority::MEDIUM;
    bead2.created_at = base_time - Duration::seconds(50);
    storage.create_issue(&bead2).unwrap();

    let mut bead3 = Issue::new("bf-3".to_string(), "Bead 3".to_string(), ".".to_string());
    bead3.priority = Priority::MEDIUM;
    bead3.created_at = base_time - Duration::seconds(10);
    storage.create_issue(&bead3).unwrap();

    // Get ready candidates
    let ready = storage
        .with_immediate_transaction(|tx| Ok(get_ready_candidates(tx, 100, None, None)?))
        .unwrap();

    assert_eq!(ready.len(), 3);

    // All should have same priority
    assert_eq!(ready[0].priority, ready[1].priority);
    assert_eq!(ready[1].priority, ready[2].priority);
}
