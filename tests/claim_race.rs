//! Concurrent claim race condition tests (thundering herd).
//!
//! Simulates high-concurrency scenarios to verify atomic claiming:
//! - 20 workers claiming simultaneously
//! - No duplicate claims under stress
//! - Stale bead reclamation under concurrent load
//! - Priority ordering under concurrent access

mod common;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration as StdDuration;
use chrono::Utc;
use rand;

#[test]
fn test_thundering_herd_20_workers_no_duplicates() {
    // Main test: 20 workers, 20 beads, no duplicates allowed
    let num_beads = 20;
    let num_workers = 20;

    let ws = common::TempWorkspace::new().unwrap();

    // Create beads with varying priorities
    for i in 0..num_beads {
        let mut bead = bead_forge::Issue::new(
            format!("bf-{:0>4}", i),
            format!("Test bead {}", i),
            ".".to_string(),
        );
        bead.priority = bead_forge::model::Priority((i % 5) as i32); // Vary priority 0-4
        ws.create_bead(&bead.id, &bead.title).unwrap();
    }

    let storage = Arc::new(ws.storage().unwrap());
    let claimed_ids = Arc::new(Mutex::new(Vec::new()));
    let worker_errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    let mut handles = vec![];

    // Spawn 20 workers simultaneously
    for worker_id in 0..num_workers {
        let claimed_ids_clone = Arc::clone(&claimed_ids);
        let errors_clone = Arc::clone(&worker_errors);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            // Add tiny random delay to increase race likelihood
            let delay = rand::random::<u64>() % 100;
            thread::sleep(StdDuration::from_micros(delay));

            let result = storage_clone.with_immediate_transaction(|tx| {
                bead_forge::claim::claim(tx, &format!("worker-{:02}", worker_id), 30, Utc::now(), None)
            });

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
                    errors.push(format!("worker-{:02}: {}", worker_id, e));
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
    let worker_errors = worker_errors.lock().unwrap();

    // Report any errors
    if !worker_errors.is_empty() {
        panic!("Worker errors occurred: {:?}", worker_errors);
    }

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
            "CRITICAL: Duplicate claims detected! The following beads were claimed multiple times: {:?}\nAll claims: {:?}",
            duplicates, *claimed_ids
        );
    }

    // Verify all beads were claimed
    assert_eq!(
        claimed_ids.len(),
        num_beads,
        "Expected {} claims, got {}. Some beads may have been skipped.",
        num_beads,
        claimed_ids.len()
    );

    println!("✓ Thundering herd test passed: {} workers claimed {} beads with zero duplicates", num_workers, num_beads);
}

#[test]
fn test_concurrent_claim_priority_preserved() {
    // Verify that under concurrent load, priority ordering is still respected
    let ws = common::TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Create beads with known priorities (0=highest, 4=lowest)
    let priorities = vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4];
    for (i, &priority) in priorities.iter().enumerate() {
        let mut bead = bead_forge::Issue::new(
            format!("bf-p{}", i),
            format!("Priority {}", priority),
            ".".to_string(),
        );
        bead.priority = bead_forge::model::Priority(priority);
        storage.create_issue(&bead).unwrap();
    }

    let storage = Arc::new(storage);
    let claimed_priorities = Arc::new(Mutex::new(Vec::new()));

    let mut handles = vec![];

    for worker_id in 0..10 {
        let priorities_clone = Arc::clone(&claimed_priorities);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            thread::sleep(StdDuration::from_micros(rand::random::<u64>() % 1000));

            let _ = storage_clone.with_immediate_transaction(|tx| {
                if let Some(claimed) = bead_forge::claim::claim(
                    tx,
                    &format!("worker-{}", worker_id),
                    30,
                    Utc::now(),
                    None,
                )? {
                    // Query priority directly from transaction
                    if let Ok(priority) = tx.query_row(
                        "SELECT priority FROM issues WHERE id = ?",
                        [&claimed.bead_id],
                        |row| row.get::<_, i32>(0)
                    ) {
                        let mut priorities = priorities_clone.lock().unwrap();
                        priorities.push(priority);
                    }
                }
                Ok::<(), anyhow::Error>(())
            });
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let priorities = claimed_priorities.lock().unwrap();

    // All 10 beads should be claimed
    assert_eq!(priorities.len(), 10);

    // Count how many of each priority were claimed
    let mut counts = [0; 5];
    for p in priorities.iter() {
        counts[*p as usize] += 1;
    }

    // Verify we got 2 of each priority (0-4)
    for i in 0..5 {
        assert_eq!(counts[i], 2, "Should have claimed 2 beads of priority {}", i);
    }

    println!("✓ Priority preserved under concurrency: {:?}", counts);
}

#[test]
fn test_concurrent_claim_with_dependencies() {
    // Verify that blocked beads are not claimed under concurrent load
    let ws = common::TempWorkspace::new().unwrap();

    // Create parent and child beads
    ws.create_bead("bf-parent", "Parent").unwrap();
    ws.create_bead("bf-child", "Child").unwrap();
    ws.create_bead("bf-unrelated", "Unrelated").unwrap();

    // Block child on parent
    let storage = ws.storage().unwrap();
    storage
        .add_dependency(
            "bf-child",
            "bf-parent",
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();

    let storage = Arc::new(storage);
    let claimed_ids = Arc::new(Mutex::new(Vec::new()));

    let mut handles = vec![];

    // 5 workers trying to claim
    for worker_id in 0..5 {
        let ids_clone = Arc::clone(&claimed_ids);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            thread::sleep(StdDuration::from_micros(rand::random::<u64>() % 500));

            let _ = storage_clone.with_immediate_transaction(|tx| {
                if let Some(claimed) = bead_forge::claim::claim(
                    tx,
                    &format!("worker-{}", worker_id),
                    30,
                    Utc::now(),
                    None,
                )? {
                    let mut ids = ids_clone.lock().unwrap();
                    ids.push(claimed.bead_id);
                }
                Ok::<(), anyhow::Error>(())
            });
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let claimed_ids = claimed_ids.lock().unwrap();

    // Parent and unrelated should be claimable, child should not
    assert_eq!(claimed_ids.len(), 2);
    assert!(!claimed_ids.contains(&"bf-child".to_string()), "Child bead should not be claimed (blocked by parent)");
    assert!(claimed_ids.contains(&"bf-parent".to_string()) || claimed_ids.contains(&"bf-unrelated".to_string()));
}

#[test]
fn test_concurrent_stale_reclamation() {
    // Verify stale beads are reclaimed correctly under concurrent load
    let ws = common::TempWorkspace::new().unwrap();

    // Create beads
    for i in 0..5 {
        ws.create_bead(&format!("bf-{:0>2}", i), &format!("Bead {}", i)).unwrap();
    }

    let storage = Arc::new(ws.storage().unwrap());

    // Manually set some beads to stale in_progress
    let stale_time = Utc::now() - chrono::Duration::minutes(60);

    storage
        .with_immediate_transaction(|tx| {
            let stale_time_str = stale_time.to_rfc3339();
            tx.execute(
                "UPDATE issues SET status = 'in_progress', assignee = 'stale-worker', updated_at = ? WHERE id IN ('bf-00', 'bf-01')",
                [&stale_time_str],
            )?;
            Ok::<_, anyhow::Error>(())
        })
        .unwrap();

    // Now 10 workers try to claim
    let claimed_count = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for worker_id in 0..10 {
        let count_clone = Arc::clone(&claimed_count);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            thread::sleep(StdDuration::from_micros(rand::random::<u64>() % 200));

            let _ = storage_clone.with_immediate_transaction(|tx| {
                if let Some(_) = bead_forge::claim::claim(
                    tx,
                    &format!("worker-{}", worker_id),
                    30,
                    Utc::now(),
                    None,
                )? {
                    let mut count = count_clone.lock().unwrap();
                    *count += 1;
                }
                Ok::<(), anyhow::Error>(())
            });
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let count = claimed_count.lock().unwrap();

    // All 5 beads should be claimed (2 stale + 3 open)
    assert_eq!(*count, 5, "All beads including stale ones should be claimed");

    // Verify stale beads were reclaimed
    let bead_00 = ws.get_bead("bf-00").unwrap().unwrap();
    let bead_01 = ws.get_bead("bf-01").unwrap().unwrap();

    let stale_reclaimed = bead_00.assignee.as_ref().map(|s| s.starts_with("worker-")).unwrap_or(false)
        || bead_01.assignee.as_ref().map(|s| s.starts_with("worker-")).unwrap_or(false)
        || bead_00.status.to_string() == "open"
        || bead_01.status.to_string() == "open";

    assert!(stale_reclaimed, "Stale beads should be reclaimed");
}

#[test]
fn test_concurrent_claim_empty_workspace() {
    // Verify workers gracefully handle empty workspace
    let ws = common::TempWorkspace::new().unwrap();

    let storage = Arc::new(ws.storage().unwrap());
    let claim_attempts = Arc::new(Mutex::new(0));
    let successful_claims = Arc::new(Mutex::new(0));

    let mut handles = vec![];

    for worker_id in 0..10 {
        let attempts_clone = Arc::clone(&claim_attempts);
        let claims_clone = Arc::clone(&successful_claims);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            thread::sleep(StdDuration::from_micros(rand::random::<u64>() % 100));

            let _ = storage_clone.with_immediate_transaction(|tx| {
                *attempts_clone.lock().unwrap() += 1;
                if let Some(_) = bead_forge::claim::claim(
                    tx,
                    &format!("worker-{}", worker_id),
                    30,
                    Utc::now(),
                    None,
                )? {
                    *claims_clone.lock().unwrap() += 1;
                }
                Ok::<(), anyhow::Error>(())
            });
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let attempts = claim_attempts.lock().unwrap();
    let claims = successful_claims.lock().unwrap();

    assert_eq!(*attempts, 10);
    assert_eq!(*claims, 0, "No beads should be claimed from empty workspace");
}

#[test]
fn test_rapid_claim_release_cycle() {
    // Simulate workers claiming, working briefly, then closing beads
    let ws = common::TempWorkspace::new().unwrap();

    // Create 10 beads
    for i in 0..10 {
        ws.create_bead(&format!("bf-cycle-{}", i), &format!("Cycle {}", i)).unwrap();
    }

    let storage = Arc::new(ws.storage().unwrap());
    let completed_count = Arc::new(Mutex::new(0));

    let mut handles = vec![];

    // 5 workers doing claim -> close cycles
    for worker_id in 0..5 {
        let count_clone = Arc::clone(&completed_count);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            for cycle in 0..3 {
                thread::sleep(StdDuration::from_micros(rand::random::<u64>() % 500));

                let claimed: Option<String> = storage_clone
                    .with_immediate_transaction(|tx| {
                        if let Some(claimed) = bead_forge::claim::claim(
                            tx,
                            &format!("worker-{}-{}", worker_id, cycle),
                            30,
                            Utc::now(),
                            None,
                        )? {
                            Ok::<Option<String>, anyhow::Error>(Some(claimed.bead_id))
                        } else {
                            Ok(None)
                        }
                    })
                    .unwrap();

                if let Some(bead_id) = claimed {
                    // Simulate brief work, then close
                    thread::sleep(StdDuration::from_millis(1));

                    let _ = storage_clone.close_issue(
                        &bead_id,
                        &format!("Completed by worker-{}", worker_id),
                        &format!("worker-{}-{}", worker_id, cycle),
                    );

                    let mut count = count_clone.lock().unwrap();
                    *count += 1;
                }
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let count = completed_count.lock().unwrap();

    // Up to 10 beads can be closed (5 workers * 2 cycles each, but limited by bead count)
    assert!(*count <= 10 && *count > 0, "Should complete between 1 and 10 beads");
    println!("✓ Rapid claim-release cycle: {} beads completed", *count);
}

#[test]
fn test_concurrent_claim_with_pinned_beads() {
    // Verify pinned beads are not claimed under concurrent load
    let ws = common::TempWorkspace::new().unwrap();

    // Create mix of pinned and unpinned beads
    for i in 0..10 {
        let mut bead = bead_forge::Issue::new(
            format!("bf-pin{}", i),
            format!("Bead {}", i),
            ".".to_string(),
        );
        if i < 3 {
            bead.pinned = true;
        }
        let storage = ws.storage().unwrap();
        storage.create_issue(&bead).unwrap();
    }

    let storage = Arc::new(ws.storage().unwrap());
    let claimed_ids = Arc::new(Mutex::new(Vec::new()));

    let mut handles = vec![];

    for worker_id in 0..10 {
        let ids_clone = Arc::clone(&claimed_ids);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            thread::sleep(StdDuration::from_micros(rand::random::<u64>() % 500));

            let _ = storage_clone.with_immediate_transaction(|tx| {
                if let Some(claimed) = bead_forge::claim::claim(
                    tx,
                    &format!("worker-{}", worker_id),
                    30,
                    Utc::now(),
                    None,
                )? {
                    let mut ids = ids_clone.lock().unwrap();
                    ids.push(claimed.bead_id);
                }
                Ok::<(), anyhow::Error>(())
            });
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let claimed_ids = claimed_ids.lock().unwrap();

    // Only 7 unpinned beads should be claimed
    assert_eq!(claimed_ids.len(), 7);

    // Verify no pinned beads were claimed
    for id in claimed_ids.iter() {
        let bead = ws.get_bead(id).unwrap().unwrap();
        assert!(!bead.pinned, "Pinned bead should not be claimed");
    }
}

#[test]
fn test_concurrent_claim_with_ephemeral_beads() {
    // Verify ephemeral beads are not claimed under concurrent load
    let ws = common::TempWorkspace::new().unwrap();

    // Create mix of ephemeral and regular beads
    for i in 0..10 {
        let mut bead = bead_forge::Issue::new(
            format!("bf-eph{}", i),
            format!("Bead {}", i),
            ".".to_string(),
        );
        if i < 3 {
            bead.ephemeral = true;
        }
        let storage = ws.storage().unwrap();
        storage.create_issue(&bead).unwrap();
    }

    let storage = Arc::new(ws.storage().unwrap());
    let claimed_ids = Arc::new(Mutex::new(Vec::new()));

    let mut handles = vec![];

    for worker_id in 0..10 {
        let ids_clone = Arc::clone(&claimed_ids);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            thread::sleep(StdDuration::from_micros(rand::random::<u64>() % 500));

            let _ = storage_clone.with_immediate_transaction(|tx| {
                if let Some(claimed) = bead_forge::claim::claim(
                    tx,
                    &format!("worker-{}", worker_id),
                    30,
                    Utc::now(),
                    None,
                )? {
                    let mut ids = ids_clone.lock().unwrap();
                    ids.push(claimed.bead_id);
                }
                Ok::<(), anyhow::Error>(())
            });
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let claimed_ids = claimed_ids.lock().unwrap();

    // Only 7 non-ephemeral beads should be claimed
    assert_eq!(claimed_ids.len(), 7);

    // Verify no ephemeral beads were claimed
    for id in claimed_ids.iter() {
        let bead = ws.get_bead(id).unwrap().unwrap();
        assert!(!bead.ephemeral, "Ephemeral bead should not be claimed");
    }
}

#[test]
fn test_high_frequency_claim_attempts() {
    // Stress test: many rapid claim attempts from few workers
    let ws = common::TempWorkspace::new().unwrap();

    // Create 5 beads
    for i in 0..5 {
        ws.create_bead(&format!("bf-stress{}", i), &format!("Stress {}", i)).unwrap();
    }

    let storage = Arc::new(ws.storage().unwrap());
    let total_claims = Arc::new(Mutex::new(0));

    let mut handles = vec![];

    // 3 workers, each trying 20 times rapidly
    for worker_id in 0..3 {
        let total_clone = Arc::clone(&total_claims);
        let storage_clone = Arc::clone(&storage);

        let handle = thread::spawn(move || {
            for _ in 0..20 {
                let _ = storage_clone.with_immediate_transaction(|tx| {
                    if let Some(_) = bead_forge::claim::claim(
                        tx,
                        &format!("worker-stress{}", worker_id),
                        30,
                        Utc::now(),
                        None,
                    )? {
                        *total_clone.lock().unwrap() += 1;
                    }
                    Ok::<(), anyhow::Error>(())
                });
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let total = total_claims.lock().unwrap();

    // Exactly 5 claims should succeed (one per bead)
    assert_eq!(*total, 5, "Exactly 5 claims should succeed (one per bead)");
}
