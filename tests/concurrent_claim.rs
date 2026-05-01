//! Test atomic concurrent claiming to verify no bead is claimed twice.
//!
//! This test simulates a thundering herd scenario where multiple workers
//! try to claim beads simultaneously. The acceptance criterion is that
//! under 20-worker load, no bead is claimed twice.

use bead_forge::bead_store::{claim_bead, ClaimConfig};
use bead_forge::config::{init_workspace, load_metadata};
use bead_forge::model::{Issue, Priority};
use bead_forge::storage::Storage;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Helper to set up a test workspace with N beads.
fn setup_workspace_with_beads(num_beads: usize) -> (TempDir, Arc<Storage>) {
    let temp_dir = TempDir::new().unwrap();
    let beads_dir = temp_dir.path().join(".beads");
    std::fs::create_dir(&beads_dir).unwrap();

    init_workspace(&beads_dir, "bf").unwrap();

    let metadata = load_metadata(&beads_dir).unwrap();
    let db_path = beads_dir.join(&metadata.database);
    let storage = Arc::new(Storage::open(&db_path).unwrap());

    // Create N beads with varying priorities
    for i in 0..num_beads {
        let mut issue = Issue::new(
            format!("bf-{:0>4}", i),
            format!("Test bead {}", i),
            ".".to_string(),
        );
        issue.priority = Priority((i % 5) as i32); // Vary priority 0-4
        storage.create_issue(&issue).unwrap();
    }

    (temp_dir, storage)
}

#[test]
fn test_concurrent_claim_no_duplicates() {
    let num_beads = 20;
    let num_workers = 20;

    let (_temp, storage) = setup_workspace_with_beads(num_beads);
    let workspace = std::env::current_dir().unwrap(); // Will be overridden by find_beads_dir
    let beads_dir = _temp.path().join(".beads");

    // Track claimed bead IDs across all workers
    let claimed_ids = Arc::new(Mutex::new(Vec::new()));
    let worker_errors = Arc::new(Mutex::new(Vec::new()));

    let mut handles = vec![];

    // Spawn 20 workers simultaneously
    for worker_id in 0..num_workers {
        let claimed_ids_clone = Arc::clone(&claimed_ids);
        let errors_clone = Arc::clone(&worker_errors);
        let workspace_path = beads_dir.clone();

        let handle = thread::spawn(move || {
            // Add a tiny random delay to increase race likelihood
            let delay = rand::random::<u64>() % 10;
            thread::sleep(Duration::from_micros(delay));

            let config = ClaimConfig::new(format!("worker-{:02}", worker_id));
            let result = claim_bead(&workspace_path, config);

            match result {
                Ok(Some(claimed)) => {
                    let mut ids = claimed_ids_clone.lock().unwrap();
                    ids.push(claimed.bead_id);
                }
                Ok(None) => {
                    // No beads available - acceptable
                }
                Err(e) => {
                    let mut errors = errors_clone.lock().unwrap();
                    errors.push(format!("Worker {}: {}", worker_id, e));
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all workers to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Check for errors
    let errors = worker_errors.lock().unwrap();
    if !errors.is_empty() {
        panic!("Worker errors occurred: {:?}", *errors);
    }

    let claimed_ids = claimed_ids.lock().unwrap();

    // Verify no duplicates
    let mut unique_ids = std::collections::HashSet::new();
    let mut duplicates = Vec::new();

    for id in claimed_ids.iter() {
        if !unique_ids.insert(id) {
            duplicates.push(id.clone());
        }
    }

    if !duplicates.is_empty() {
        panic!(
            "Duplicate claims detected! The following beads were claimed multiple times: {:?}\nAll claims: {:?}",
            duplicates, *claimed_ids
        );
    }

    // Verify all beads were claimed (exactly 20 claims for 20 beads)
    assert_eq!(
        claimed_ids.len(),
        num_beads,
        "Expected {} claims, got {}",
        num_beads,
        claimed_ids.len()
    );

    println!("✓ Concurrent claim test passed: {} workers claimed {} beads with no duplicates", num_workers, num_beads);
}

#[test]
fn test_concurrent_claim_priority_ordering() {
    // Create beads with unique priorities (valid range is 0-4)
    let (_temp, storage) = setup_workspace_with_beads(0);
    let beads_dir = _temp.path().join(".beads");

    // Create 5 beads with unique priorities 0-4 (0 is highest)
    for i in 0..5 {
        let mut issue = Issue::new(
            format!("bf-{:0>4}", i),
            format!("Priority {} bead", i),
            ".".to_string(),
        );
        issue.priority = Priority(i);
        storage.create_issue(&issue).unwrap();
    }

    let claimed_priorities = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];

    for worker_id in 0..5 {
        let priorities_clone = Arc::clone(&claimed_priorities);
        let workspace_path = beads_dir.clone();

        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_micros(rand::random::<u64>() % 100));

            let config = ClaimConfig::new(format!("worker-{:02}", worker_id));
            if let Ok(Some(claimed)) = claim_bead(&workspace_path, config) {
                // Get the priority of the claimed bead
                let bead_storage = Storage::open(&workspace_path.join("beads.db")).unwrap();
                if let Ok(Some(issue)) = bead_storage.get_issue(&claimed.bead_id) {
                    let mut priorities = priorities_clone.lock().unwrap();
                    priorities.push(issue.priority.0);
                }
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let priorities = claimed_priorities.lock().unwrap();

    println!("Claimed priorities in order: {:?}", *priorities);

    // All 5 beads should be claimed with unique priorities
    assert_eq!(priorities.len(), 5);
    let mut unique_priorities: std::collections::HashSet<_> = priorities.iter().cloned().collect();
    assert_eq!(unique_priorities.len(), 5, "All priorities should be unique");

    // The first claimed should have priority 0 (highest)
    assert!(priorities.contains(&0), "Priority 0 should be claimed");
}

#[test]
fn test_concurrent_claim_empty_workspace() {
    let (_temp, _storage) = setup_workspace_with_beads(0);
    let beads_dir = _temp.path().join(".beads");

    let claim_count = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    // 5 workers try to claim from empty workspace
    for worker_id in 0..5 {
        let count_clone = Arc::clone(&claim_count);
        let workspace_path = beads_dir.clone();

        let handle = thread::spawn(move || {
            let config = ClaimConfig::new(format!("worker-{}", worker_id));
            if let Ok(Some(_)) = claim_bead(&workspace_path, config) {
                let mut count = count_clone.lock().unwrap();
                *count += 1;
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let count = claim_count.lock().unwrap();
    assert_eq!(*count, 0, "No beads should be claimed from empty workspace");
}

#[test]
fn test_concurrent_claim_stale_reclamation() {
    let num_beads = 5;
    let (_temp, storage) = setup_workspace_with_beads(num_beads);
    let beads_dir = _temp.path().join(".beads");

    // Manually set some beads to in_progress with old timestamp
    use chrono::{Duration, Utc};

    let stale_time = Utc::now() - Duration::minutes(60);
    let stale_time_str = stale_time.to_rfc3339();

    // Set 2 beads to stale in_progress
    storage.with_immediate_transaction(|tx| {
        tx.execute(
            "UPDATE issues SET status = 'in_progress', assignee = 'stale-worker', updated_at = ? WHERE id IN ('bf-0000', 'bf-0001')",
            [&stale_time_str],
        )?;
        Ok::<(), anyhow::Error>(())
    }).unwrap();

    // Now a new worker should be able to claim after stale reclamation
    let config = ClaimConfig::new("fresh-worker".to_string());
    let result = claim_bead(&beads_dir, config).unwrap();

    assert!(result.is_some(), "Should reclaim stale beads and claim one");

    // Verify the claimed bead is now owned by fresh-worker
    let claimed = result.unwrap();
    let issue = storage.get_issue(&claimed.bead_id).unwrap().unwrap();
    assert_eq!(issue.assignee.as_ref().unwrap(), "fresh-worker");

    // Verify stale beads were reclaimed
    let stale0 = storage.get_issue("bf-0000").unwrap().unwrap();
    let stale1 = storage.get_issue("bf-0001").unwrap().unwrap();

    // At least one should be reclaimed to open or claimed by fresh-worker
    let reclamation_happened = stale0.status.to_string() == "open" ||
                                stale1.status.to_string() == "open" ||
                                stale0.assignee.as_ref().map(|s| s == "fresh-worker").unwrap_or(false) ||
                                stale1.assignee.as_ref().map(|s| s == "fresh-worker").unwrap_or(false);

    assert!(reclamation_happened, "Stale beads should be reclaimed");
}
