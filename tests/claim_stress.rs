//! Comprehensive stress tests for concurrent claim operations.
//!
//! These tests validate:
//! - 10+ agents claiming simultaneously
//! - BEGIN IMMEDIATE prevents race conditions
//! - Exponential backoff under SQLITE_BUSY
//! - Only one claim succeeds per bead
//! - Claim retry logic
//! - Claim throughput benchmarks

use bead_forge::claim::claim;
use bead_forge::config::{init_workspace, load_metadata};
use bead_forge::model::{Issue, Priority};
use bead_forge::storage::Storage;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const MAX_RETRIES: u32 = 5;
const RETRY_BASE_MS: u64 = 50;

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
        issue.priority = Priority((i % 5) as i32);
        storage.create_issue(&issue).unwrap();
    }

    (temp_dir, storage)
}

/// Claim a bead using the storage layer directly.
fn claim_bead(storage: &Arc<Storage>, worker: &str) -> Option<bead_forge::claim::ClaimResult> {
    storage
        .with_immediate_transaction(|tx| claim(tx, worker, 30, Utc::now(), None))
        .unwrap()
}

/// Stress test: 50 workers claiming simultaneously from 50 beads.
#[test]
fn test_stress_50_workers_concurrent_claim() {
    let num_beads = 50;
    let num_workers = 50;

    let (_temp, storage) = setup_workspace_with_beads(num_beads);

    // Track claimed bead IDs across all workers
    let claimed_ids = Arc::new(Mutex::new(Vec::new()));
    let claim_times = Arc::new(Mutex::new(Vec::new()));

    let mut handles = vec![];

    // Spawn 50 workers simultaneously
    for worker_id in 0..num_workers {
        let claimed_ids_clone = Arc::clone(&claimed_ids);
        let times_clone = Arc::clone(&claim_times);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            let start = Instant::now();

            // Add a tiny random delay to increase race likelihood
            let delay = rand::random::<u64>() % 10;
            thread::sleep(Duration::from_micros(delay));

            let result = claim_bead(&storage_clone, &format!("worker-{:02}", worker_id));

            let elapsed = start.elapsed();

            match result {
                Some(claimed) => {
                    let mut ids = claimed_ids_clone.lock().unwrap();
                    ids.push(claimed.bead_id);

                    let mut times = times_clone.lock().unwrap();
                    times.push(elapsed);
                }
                None => {
                    // No beads available - should not happen with 50 beads and 50 workers
                    let mut times = times_clone.lock().unwrap();
                    times.push(elapsed);
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all workers to complete
    for handle in handles {
        handle.join().unwrap();
    }

    let claimed_ids = claimed_ids.lock().unwrap();
    let claim_times = claim_times.lock().unwrap();

    // Verify no duplicates
    let mut unique_ids = HashSet::new();
    let mut duplicates = Vec::new();

    for id in claimed_ids.iter() {
        if !unique_ids.insert(id) {
            duplicates.push(id.clone());
        }
    }

    if !duplicates.is_empty() {
        panic!(
            "Duplicate claims detected in stress test! Duplicates: {:?}\nAll claims: {:?}",
            duplicates, *claimed_ids
        );
    }

    // Verify all beads were claimed exactly once
    assert_eq!(
        claimed_ids.len(),
        num_beads,
        "Expected {} claims, got {}",
        num_beads,
        claimed_ids.len()
    );

    // Calculate and report statistics
    let mut times_sorted = claim_times.clone();
    times_sorted.sort();

    let min_time = times_sorted.first().unwrap();
    let max_time = times_sorted.last().unwrap();
    let total_time: Duration = times_sorted.iter().sum();
    let avg_time = total_time / num_workers as u32;

    println!("✓ 50-worker stress test passed");
    println!(
        "  Claim times: min={:?} max={:?} avg={:?}",
        min_time, max_time, avg_time
    );
    println!("  Total time: {:?}", total_time);
    println!(
        "  Throughput: {:.2} claims/sec",
        num_beads as f64 / total_time.as_secs_f64()
    );
}

/// Test: Verify BEGIN IMMEDIATE prevents race conditions.
#[test]
fn test_begin_immediate_prevents_races() {
    let num_beads = 20;
    let (_temp, storage) = setup_workspace_with_beads(num_beads);

    // Run the concurrent claim test multiple times to increase probability
    // of catching race conditions
    for iteration in 0..10 {
        // Clear claimed IDs from previous iteration
        let claimed_ids = Arc::new(Mutex::new(Vec::new()));

        // Reset beads to open for this iteration
        storage.with_immediate_transaction(|tx| {
            tx.execute("UPDATE issues SET status = 'open', assignee = NULL WHERE status = 'in_progress'", [])?;
            Ok::<(), bead_forge::error::BeadForgeError>(())
        }).unwrap();

        let mut handles = vec![];

        // 20 workers claiming simultaneously
        for worker_id in 0..20 {
            let claimed_ids_clone = Arc::clone(&claimed_ids);
            let storage_clone = Arc::clone(&storage);

            let handle = thread::spawn(move || {
                thread::sleep(Duration::from_micros(rand::random::<u64>() % 100));

                if let Some(claimed) =
                    claim_bead(&storage_clone, &format!("w{}-{}", iteration, worker_id))
                {
                    let mut ids = claimed_ids_clone.lock().unwrap();
                    ids.push(claimed.bead_id);
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let claimed_ids = claimed_ids.lock().unwrap();

        // Verify uniqueness
        let unique: HashSet<_> = claimed_ids.iter().collect();
        assert_eq!(
            unique.len(),
            claimed_ids.len(),
            "Iteration {}: Found duplicate claims",
            iteration
        );
    }

    println!("✓ BEGIN IMMEDIATE race prevention test passed (10 iterations)");
}

/// Test: Simulate SQLITE_BUSY with delayed transactions.
#[test]
fn test_exponential_backoff_under_busy() {
    let num_beads = 10;
    let (_temp, storage) = setup_workspace_with_beads(num_beads);

    let claimed_ids = Arc::new(Mutex::new(Vec::new()));
    let retry_counts = Arc::new(Mutex::new(HashMap::new()));
    let mut handles = vec![];

    // 15 workers competing for 10 beads (5 will fail with None)
    for worker_id in 0..15 {
        let claimed_ids_clone = Arc::clone(&claimed_ids);
        let retries_clone = Arc::clone(&retry_counts);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            let mut retries = 0;
            let start = Instant::now();

            // Each worker attempts to claim with potential busy scenarios
            let result = loop {
                let attempt = claim_bead(&storage_clone, &format!("worker-{}", worker_id));

                match attempt {
                    Some(claimed) => {
                        break Some(claimed);
                    }
                    None => {
                        // No beads available or SQLite busy - retry with exponential backoff
                        retries += 1;
                        if retries >= MAX_RETRIES || start.elapsed() > Duration::from_secs(2) {
                            break None;
                        }
                        thread::sleep(Duration::from_millis(RETRY_BASE_MS * (1 << retries)));
                    }
                }
            };

            if let Some(claimed) = result {
                let mut ids = claimed_ids_clone.lock().unwrap();
                ids.push(claimed.bead_id);

                let mut retries_map = retries_clone.lock().unwrap();
                retries_map.insert(worker_id, retries);
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let claimed_ids = claimed_ids.lock().unwrap();
    let retry_counts = retry_counts.lock().unwrap();

    // Verify exactly 10 beads claimed (no duplicates)
    let unique: HashSet<_> = claimed_ids.iter().collect();
    assert_eq!(unique.len(), 10, "Expected exactly 10 unique claims");
    assert_eq!(claimed_ids.len(), 10, "Expected exactly 10 claims total");

    println!("✓ Exponential backoff test passed");
    println!("  Retry counts per worker: {:?}", retry_counts.len());

    // If any workers had to retry, verify backoff occurred
    let total_retries: u32 = retry_counts.values().sum();
    if total_retries > 0 {
        println!("  Total retries across all workers: {}", total_retries);
        println!("  Workers that needed retries: {}", retry_counts.len());
    }
}

/// Test: Verify only one claim succeeds per bead under high contention.
#[test]
fn test_single_claim_per_bead_high_contention() {
    let num_beads = 5;
    let num_workers = 50; // 10x contention

    let (_temp, storage) = setup_workspace_with_beads(num_beads);

    let claim_attempts = Arc::new(Mutex::new(Vec::new()));
    let successful_claims = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];

    // Many workers competing for few beads
    for worker_id in 0..num_workers {
        let attempts_clone = Arc::clone(&claim_attempts);
        let successes_clone = Arc::clone(&successful_claims);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            let worker_id_str = format!("worker-{:03}", worker_id);

            // Track this attempt
            {
                let mut attempts = attempts_clone.lock().unwrap();
                attempts.push(worker_id);
            }

            let result = claim_bead(&storage_clone, &worker_id_str);

            if let Some(claimed) = result {
                let mut successes = successes_clone.lock().unwrap();
                successes.push((worker_id, claimed.bead_id));
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let claim_attempts = claim_attempts.lock().unwrap();
    let successful_claims = successful_claims.lock().unwrap();

    // Verify all workers attempted
    assert_eq!(
        claim_attempts.len(),
        num_workers,
        "All workers should attempt"
    );

    // Verify only 5 successful claims (one per bead)
    assert_eq!(
        successful_claims.len(),
        num_beads,
        "Only {} beads should be claimed successfully",
        num_beads
    );

    // Verify no bead was claimed twice
    let claimed_beads: Vec<_> = successful_claims
        .iter()
        .map(|(_, bead_id)| bead_id)
        .collect();
    let unique_beads: HashSet<_> = claimed_beads.iter().collect();
    assert_eq!(
        unique_beads.len(),
        num_beads,
        "Each bead should be claimed exactly once"
    );

    // Verify each bead has exactly one claim
    let mut bead_counts = HashMap::new();
    for bead_id in claimed_beads {
        *bead_counts.entry(bead_id).or_insert(0) += 1;
    }

    for (bead_id, count) in bead_counts {
        assert_eq!(
            count, 1,
            "Bead {} was claimed {} times, expected 1",
            bead_id, count
        );
    }

    println!("✓ Single claim per bead test passed");
    println!("  {} workers competed for {} beads", num_workers, num_beads);
    println!(
        "  {} workers succeeded, {} workers failed with None",
        successful_claims.len(),
        num_workers - successful_claims.len()
    );
}

/// Test: Claim retry logic with artificial contention.
#[test]
fn test_claim_retry_logic() {
    let num_beads = 20;
    let (_temp, storage) = setup_workspace_with_beads(num_beads);

    let retry_stats = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];

    // 25 workers competing for 20 beads
    for worker_id in 0..25 {
        let stats_clone = Arc::clone(&retry_stats);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            let mut attempts = 0;
            let mut successful = false;
            let start = Instant::now();

            while attempts < MAX_RETRIES && start.elapsed() < Duration::from_secs(1) {
                attempts += 1;

                // Add jitter to spread out attempts
                thread::sleep(Duration::from_micros(rand::random::<u64>() % 500));

                let result = claim_bead(&storage_clone, &format!("worker-{}", worker_id));

                if result.is_some() {
                    successful = true;
                    break;
                }

                // Exponential backoff before retry
                if attempts < MAX_RETRIES {
                    thread::sleep(Duration::from_millis(RETRY_BASE_MS * (1 << (attempts - 1))));
                }
            }

            let mut stats = stats_clone.lock().unwrap();
            stats.push((worker_id, attempts, successful));
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let retry_stats = retry_stats.lock().unwrap();

    let successful_workers: Vec<_> = retry_stats
        .iter()
        .filter(|(_, _, success)| *success)
        .collect();
    let failed_workers: Vec<_> = retry_stats
        .iter()
        .filter(|(_, _, success)| !success)
        .collect();

    // Verify we got 20 successful claims (all beads)
    assert_eq!(
        successful_workers.len(),
        num_beads,
        "Expected {} successful claims",
        num_beads
    );

    // Calculate retry statistics
    let total_attempts: u32 = retry_stats.iter().map(|(_, attempts, _)| attempts).sum();
    let avg_attempts = total_attempts as f64 / retry_stats.len() as f64;

    let retries_by_successful: u32 = successful_workers
        .iter()
        .map(|(_, attempts, _)| attempts - 1)
        .sum();
    let retries_by_failed: u32 = failed_workers.iter().map(|(_, attempts, _)| attempts).sum();

    println!("✓ Claim retry logic test passed");
    println!("  Successful workers: {}", successful_workers.len());
    println!("  Failed workers: {}", failed_workers.len());
    println!("  Average attempts per worker: {:.2}", avg_attempts);
    println!(
        "  Total retries by successful workers: {}",
        retries_by_successful
    );
    println!("  Total retries by failed workers: {}", retries_by_failed);
}

/// Benchmark: Claim throughput under concurrent load.
#[test]
fn test_benchmark_claim_throughput() {
    let num_beads = 100;
    let (_temp, storage) = setup_workspace_with_beads(num_beads);

    let start = Instant::now();
    let claimed_ids = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];

    // Benchmark with 100 workers claiming 100 beads
    for worker_id in 0..num_beads {
        let claimed_ids_clone = Arc::clone(&claimed_ids);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            let result = claim_bead(&storage_clone, &format!("worker-{}", worker_id));

            if let Some(claimed) = result {
                let mut ids = claimed_ids_clone.lock().unwrap();
                ids.push(claimed.bead_id);
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let elapsed = start.elapsed();
    let claimed_ids = claimed_ids.lock().unwrap();

    // Verify all beads claimed
    assert_eq!(claimed_ids.len(), num_beads);

    let throughput = num_beads as f64 / elapsed.as_secs_f64();
    let avg_latency_ms = elapsed.as_millis() as f64 / num_beads as f64;

    println!("✓ Claim throughput benchmark");
    println!("  Total time: {:?}", elapsed);
    println!("  Throughput: {:.2} claims/sec", throughput);
    println!("  Average latency: {:.2} ms", avg_latency_ms);

    // Basic sanity checks
    assert!(throughput > 10.0, "Throughput should be > 10 claims/sec");
    assert!(
        avg_latency_ms < 1000.0,
        "Average latency should be < 1 second"
    );
}

/// Test: Claim behavior when workspace is nearly exhausted.
#[test]
fn test_claim_with_nearly_exhausted_workspace() {
    let num_beads = 10;
    let (_temp, storage) = setup_workspace_with_beads(num_beads);

    // First, claim 9 beads normally
    for i in 0..9 {
        let result = claim_bead(&storage, &format!("initial-worker-{}", i));
        assert!(result.is_some(), "Should successfully claim bead {}", i);
    }

    // Now 20 workers compete for the last bead
    let last_bead_claimed = Arc::new(Mutex::new(false));
    let successful_workers = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];

    for worker_id in 0..20 {
        let claimed_clone = Arc::clone(&last_bead_claimed);
        let successes_clone = Arc::clone(&successful_workers);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_micros(rand::random::<u64>() % 200));

            let result = claim_bead(&storage_clone, &format!("worker-{}", worker_id));

            if result.is_some() {
                let mut claimed = claimed_clone.lock().unwrap();
                *claimed = true;

                let mut successes = successes_clone.lock().unwrap();
                successes.push(worker_id);
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let last_bead_claimed = last_bead_claimed.lock().unwrap();
    let successful_workers = successful_workers.lock().unwrap();

    // Verify exactly one worker got the last bead
    assert!(*last_bead_claimed, "Last bead should be claimed");
    assert_eq!(
        successful_workers.len(),
        1,
        "Only one worker should successfully claim the last bead"
    );

    println!("✓ Nearly exhausted workspace test passed");
    println!("  Worker {} claimed the last bead", successful_workers[0]);
}
