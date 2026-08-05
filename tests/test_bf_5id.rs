//! Regression test for bf-5id: close_issue() should cascade status transitions
//! from 'blocked' to 'open' when a dependent's last blocker closes.
//!
//! This test ensures that when a bead is closed, all beads that were blocked by it
//! and have no remaining active blockers should transition to 'open' status.

use bead_forge::model::{Dependency, DependencyType, Issue, Status};
use bead_forge::storage::Storage;
use chrono::Utc;
use std::path::PathBuf;
use tempfile::TempDir;

fn create_test_bead(id: &str, title: &str, status: Status) -> Issue {
    Issue {
        id: id.to_string(),
        content_hash: None,
        title: title.to_string(),
        description: Some(format!("Test bead: {}", title)),
        design: None,
        acceptance_criteria: None,
        notes: None,
        status,
        priority: bead_forge::model::Priority::MEDIUM,
        issue_type: bead_forge::model::IssueType::Task,
        assignee: None,
        owner: None,
        estimated_minutes: None,
        created_at: Utc::now(),
        created_by: Some("test".to_string()),
        updated_at: Utc::now(),
        closed_at: None,
        close_reason: None,
        closed_by_session: None,
        due_at: None,
        defer_until: None,
        external_ref: None,
        source_system: None,
        source_repo: None,
        deleted_at: None,
        deleted_by: None,
        delete_reason: None,
        original_type: None,
        compaction_level: None,
        compacted_at: None,
        compacted_at_commit: None,
        original_size: None,
        sender: None,
        ephemeral: false,
        pinned: false,
        is_template: false,
        labels: vec![],
        dependencies: vec![],
        comments: vec![],
        annotations: std::collections::BTreeMap::new(),
    }
}

#[test]
fn test_close_unblocks_dependent_with_single_blocker() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create bead A (the blocker)
    let bead_a = create_test_bead("bf-a", "Blocker A", Status::Open);
    storage.create_issue(&bead_a).unwrap();

    // Create bead B (the dependent) with status='blocked' and a dependency on A
    let mut bead_b = create_test_bead("bf-b", "Dependent B", Status::Blocked);
    bead_b.dependencies = vec![Dependency {
        issue_id: "bf-b".to_string(),
        depends_on_id: "bf-a".to_string(),
        dep_type: DependencyType::Blocks,
        metadata: None,
        thread_id: None,
        created_at: Utc::now(),
        created_by: Some("test".to_string()),
        title: None,
    }];
    storage.create_issue(&bead_b).unwrap();

    // Verify B is blocked
    let retrieved_b = storage.get_issue("bf-b").unwrap().unwrap();
    assert_eq!(retrieved_b.status, Status::Blocked);

    // Verify B appears in blocked_issues_cache
    let blocked_cache = storage.get_blocked_issues().unwrap();
    assert!(blocked_cache.iter().any(|(id, _)| id == "bf-b"));

    // Close A
    storage.close_issue("bf-a", "Done", "test-user").unwrap();

    // Verify A is closed
    let retrieved_a = storage.get_issue("bf-a").unwrap().unwrap();
    assert_eq!(retrieved_a.status, Status::Closed);

    // Verify B transitioned to 'open' (the fix being tested)
    let retrieved_b_after = storage.get_issue("bf-b").unwrap().unwrap();
    assert_eq!(
        retrieved_b_after.status,
        Status::Open,
        "Dependent should transition from blocked to open when last blocker closes"
    );

    // Verify B no longer appears in blocked_issues_cache
    let blocked_cache_after = storage.get_blocked_issues().unwrap();
    assert!(
        !blocked_cache_after.iter().any(|(id, _)| id == "bf-b"),
        "Dependent should not appear in blocked_issues_cache after all blockers close"
    );
}

#[test]
fn test_close_does_not_unblock_dependent_with_multiple_blockers() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create bead A (blocker 1)
    let bead_a = create_test_bead("bf-a", "Blocker A", Status::Open);
    storage.create_issue(&bead_a).unwrap();

    // Create bead D (blocker 2, still open)
    let bead_d = create_test_bead("bf-d", "Blocker D", Status::Open);
    storage.create_issue(&bead_d).unwrap();

    // Create bead C (the dependent) blocked by both A and D
    let mut bead_c = create_test_bead("bf-c", "Dependent C", Status::Blocked);
    bead_c.dependencies = vec![
        Dependency {
            issue_id: "bf-c".to_string(),
            depends_on_id: "bf-a".to_string(),
            dep_type: DependencyType::Blocks,
            metadata: None,
            thread_id: None,
            created_at: Utc::now(),
            created_by: Some("test".to_string()),
            title: None,
        },
        Dependency {
            issue_id: "bf-c".to_string(),
            depends_on_id: "bf-d".to_string(),
            dep_type: DependencyType::Blocks,
            metadata: None,
            thread_id: None,
            created_at: Utc::now(),
            created_by: Some("test".to_string()),
            title: None,
        },
    ];
    storage.create_issue(&bead_c).unwrap();

    // Verify C is blocked
    let retrieved_c = storage.get_issue("bf-c").unwrap().unwrap();
    assert_eq!(retrieved_c.status, Status::Blocked);

    // Close A
    storage.close_issue("bf-a", "Done", "test-user").unwrap();

    // Verify C is STILL blocked (D is still open)
    let retrieved_c_after = storage.get_issue("bf-c").unwrap().unwrap();
    assert_eq!(
        retrieved_c_after.status,
        Status::Blocked,
        "Dependent should remain blocked when other blockers remain open"
    );

    // Verify C still appears in blocked_issues_cache (blocked by D)
    let blocked_cache_after = storage.get_blocked_issues().unwrap();
    assert!(
        blocked_cache_after.iter().any(|(id, _)| id == "bf-c"),
        "Dependent should remain in blocked_issues_cache while any blocker is still open"
    );
}

#[test]
fn test_close_cascades_chain_of_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create a chain: Phase1 -> Phase2 -> Phase3
    // Phase1 blocks Phase2, Phase2 blocks Phase3
    let phase1 = create_test_bead("bf-phase1", "Phase 1", Status::Open);
    storage.create_issue(&phase1).unwrap();

    let mut phase2 = create_test_bead("bf-phase2", "Phase 2", Status::Blocked);
    phase2.dependencies = vec![Dependency {
        issue_id: "bf-phase2".to_string(),
        depends_on_id: "bf-phase1".to_string(),
        dep_type: DependencyType::Blocks,
        metadata: None,
        thread_id: None,
        created_at: Utc::now(),
        created_by: Some("test".to_string()),
        title: None,
    }];
    storage.create_issue(&phase2).unwrap();

    let mut phase3 = create_test_bead("bf-phase3", "Phase 3", Status::Blocked);
    phase3.dependencies = vec![Dependency {
        issue_id: "bf-phase3".to_string(),
        depends_on_id: "bf-phase2".to_string(),
        dep_type: DependencyType::Blocks,
        metadata: None,
        thread_id: None,
        created_at: Utc::now(),
        created_by: Some("test".to_string()),
        title: None,
    }];
    storage.create_issue(&phase3).unwrap();

    // Verify initial state
    assert_eq!(
        storage.get_issue("bf-phase1").unwrap().unwrap().status,
        Status::Open
    );
    assert_eq!(
        storage.get_issue("bf-phase2").unwrap().unwrap().status,
        Status::Blocked
    );
    assert_eq!(
        storage.get_issue("bf-phase3").unwrap().unwrap().status,
        Status::Blocked
    );

    // Close Phase1
    storage
        .close_issue("bf-phase1", "Phase 1 complete", "test-user")
        .unwrap();

    // Phase2 should transition to 'open'
    assert_eq!(
        storage.get_issue("bf-phase2").unwrap().unwrap().status,
        Status::Open,
        "Phase2 should transition to open when Phase1 closes"
    );

    // Phase3 should remain blocked (blocked by Phase2)
    assert_eq!(
        storage.get_issue("bf-phase3").unwrap().unwrap().status,
        Status::Blocked,
        "Phase3 should remain blocked (blocked by Phase2)"
    );

    // Close Phase2
    storage
        .close_issue("bf-phase2", "Phase 2 complete", "test-user")
        .unwrap();

    // Phase3 should now transition to 'open'
    assert_eq!(
        storage.get_issue("bf-phase3").unwrap().unwrap().status,
        Status::Open,
        "Phase3 should transition to open when Phase2 closes"
    );
}

#[test]
fn test_close_is_idempotent() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create and close a bead
    let bead = create_test_bead("bf-idem", "Idempotent close", Status::Open);
    storage.create_issue(&bead).unwrap();

    storage.close_issue("bf-idem", "Reason", "actor").unwrap();
    storage.close_issue("bf-idem", "Reason", "actor").unwrap();
    storage.close_issue("bf-idem", "Reason", "actor").unwrap();

    // Should still be closed, no error
    let retrieved = storage.get_issue("bf-idem").unwrap().unwrap();
    assert_eq!(retrieved.status, Status::Closed);
}

#[test]
fn test_non_blocked_dependent_is_unchanged() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create bead A (blocker)
    let bead_a = create_test_bead("bf-a", "Blocker A", Status::Open);
    storage.create_issue(&bead_a).unwrap();

    // Create bead B with a dependency on A but status='in_progress' (not blocked)
    let mut bead_b = create_test_bead("bf-b", "Dependent B", Status::InProgress);
    bead_b.dependencies = vec![Dependency {
        issue_id: "bf-b".to_string(),
        depends_on_id: "bf-a".to_string(),
        dep_type: DependencyType::Blocks,
        metadata: None,
        thread_id: None,
        created_at: Utc::now(),
        created_by: Some("test".to_string()),
        title: None,
    }];
    storage.create_issue(&bead_b).unwrap();

    // Close A
    storage.close_issue("bf-a", "Done", "test-user").unwrap();

    // B should remain in 'in_progress' (NOT transition to 'open')
    let retrieved_b = storage.get_issue("bf-b").unwrap().unwrap();
    assert_eq!(
        retrieved_b.status,
        Status::InProgress,
        "Non-blocked dependents should not be affected by blocker closing"
    );
}
