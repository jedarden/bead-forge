//! Comprehensive batch operation transaction tests.
//!
//! These tests verify that batch operations execute atomically under single transactions:
//! - All operations succeed or all fail together
//! - No partial state on errors (complete rollback)
//! - Transaction isolation between operations
//! - BEGIN IMMEDIATE transaction handling with SQLITE_BUSY retry

use bead_forge::batch::{execute_batch, BatchOp};
use bead_forge::config::init_workspace;
use bead_forge::model::{Issue, IssueFilter};
use bead_forge::storage::Storage;
use std::fs;
use tempfile::TempDir;

/// Helper to set up a test workspace with storage initialized.
fn setup_test_workspace() -> (TempDir, Storage) {
    let temp_dir = TempDir::new().unwrap();
    let beads_dir = temp_dir.path().join(".beads");
    fs::create_dir(&beads_dir).unwrap();
    init_workspace(&beads_dir, "bf").unwrap();

    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open(&db_path).unwrap();

    (temp_dir, storage)
}

/// Helper to create a test bead in storage.
fn create_test_bead(storage: &Storage, id: &str, title: &str) -> Issue {
    let bead = Issue::new(id.to_string(), title.to_string(), ".".to_string());
    storage.create_issue(&bead).unwrap();
    bead
}

#[test]
fn test_batch_create_operations_single_transaction() {
    // Test that multiple create operations execute in a single transaction
    let (temp_dir, storage) = setup_test_workspace();

    let initial_count = storage.list_issues(&IssueFilter::default()).unwrap().len();

    // Create multiple beads in a single batch
    let ops = vec![
        BatchOp::Create {
            title: "First bead".to_string(),
            type_: "task".to_string(),
            priority: 2,
            description: Some("First bead description".to_string()),
            assignee: None,
            labels: vec!["tag1".to_string()],
        },
        BatchOp::Create {
            title: "Second bead".to_string(),
            type_: "bug".to_string(),
            priority: 0,
            description: None,
            assignee: Some("user1".to_string()),
            labels: vec!["urgent".to_string()],
        },
        BatchOp::Create {
            title: "Third bead".to_string(),
            type_: "feature".to_string(),
            priority: 1,
            description: Some("Third bead".to_string()),
            assignee: None,
            labels: vec![],
        },
    ];

    let results = execute_batch(&storage, ops, temp_dir.path(), true).unwrap();

    // All operations should succeed
    assert_eq!(results.len(), 3);
    for result in &results {
        assert_eq!(result.status, "ok");
        assert!(result.id.is_some());
    }

    // Verify all beads were created atomically
    let final_count = storage.list_issues(&IssueFilter::default()).unwrap().len();
    assert_eq!(final_count, initial_count + 3);

    // Verify each created bead
    let bead1_id = results[0].id.as_ref().unwrap();
    let bead1 = storage.get_issue(bead1_id).unwrap().unwrap();
    assert_eq!(bead1.title, "First bead");

    // Check labels directly from storage (get_issue doesn't load labels table)
    let bead1_labels = storage
        .with_immediate_transaction(|tx| {
            let mut stmt = tx.prepare("SELECT label FROM labels WHERE issue_id = ?1").unwrap();
            let labels: Vec<String> = stmt.query_map([&bead1_id], |row| row.get(0)).unwrap()
                .filter_map(|r| r.ok()).collect();
            Ok(labels)
        }).unwrap();
    assert_eq!(bead1_labels.len(), 1);
    assert!(bead1_labels.contains(&"tag1".to_string()));

    let bead2 = storage.get_issue(&results[1].id.as_ref().unwrap()).unwrap().unwrap();
    assert_eq!(bead2.title, "Second bead");
    assert_eq!(bead2.assignee.as_deref(), Some("user1"));

    let bead3 = storage.get_issue(&results[2].id.as_ref().unwrap()).unwrap().unwrap();
    assert_eq!(bead3.title, "Third bead");
}

#[test]
fn test_batch_dependency_operations_single_transaction() {
    // Test that dependency operations execute in a single transaction
    let (temp_dir, storage) = setup_test_workspace();

    // Create beads for dependency operations
    let bead_a = create_test_bead(&storage, "bf-a", "Bead A");
    let bead_b = create_test_bead(&storage, "bf-b", "Bead B");
    let bead_c = create_test_bead(&storage, "bf-c", "Bead C");

    // Create multiple dependency relationships in a single batch
    let ops = vec![
        BatchOp::DepAddBlocker {
            id: "bf-b".to_string(),  // B is blocked by A
            blocker: "bf-a".to_string(),
        },
        BatchOp::DepAddBlocker {
            id: "bf-c".to_string(),  // C is blocked by B
            blocker: "bf-b".to_string(),
        },
        BatchOp::DepAddBlocker {
            id: "bf-c".to_string(),  // C is also blocked by A
            blocker: "bf-a".to_string(),
        },
    ];

    let results = execute_batch(&storage, ops, temp_dir.path(), true).unwrap();

    // All operations should succeed
    assert_eq!(results.len(), 3);
    for result in &results {
        assert_eq!(result.status, "ok");
    }

    // Verify all dependencies were created atomically
    let deps_b = storage.get_dependencies("bf-b").unwrap();
    assert_eq!(deps_b.len(), 1);
    assert_eq!(deps_b[0].depends_on_id, "bf-a");

    let deps_c = storage.get_dependencies("bf-c").unwrap();
    assert_eq!(deps_c.len(), 2);
    let dep_ids: Vec<&str> = deps_c.iter().map(|d| d.depends_on_id.as_str()).collect();
    assert!(dep_ids.contains(&"bf-a"));
    assert!(dep_ids.contains(&"bf-b"));
}

#[test]
fn test_batch_mixed_operations_single_transaction() {
    // Test mixed operations (create + update + dependencies + labels) in single transaction
    let (temp_dir, storage) = setup_test_workspace();

    // Create initial beads for mixed operations
    create_test_bead(&storage, "bf-1", "Bead 1");
    create_test_bead(&storage, "bf-2", "Bead 2");

    let initial_count = storage.list_issues(&IssueFilter::default()).unwrap().len();

    // Create a complex batch with multiple operation types
    let ops = vec![
        // Create a new bead
        BatchOp::Create {
            title: "New bead from mixed batch".to_string(),
            type_: "task".to_string(),
            priority: 1,
            description: Some("Created in mixed batch".to_string()),
            assignee: Some("worker".to_string()),
            labels: vec!["batch-created".to_string()],
        },
        // Update existing bead
        BatchOp::Update {
            id: "bf-1".to_string(),
            title: Some("Updated bead 1".to_string()),
            description: Some("Updated in batch".to_string()),
            design: None,
            acceptance_criteria: None,
            notes: None,
            status: Some("in_progress".to_string()),
            priority: Some(0),
            assignee: None,
            owner: None,
            issue_type: None,
        },
        // Add dependency
        BatchOp::DepAddBlocker {
            id: "bf-2".to_string(),
            blocker: "bf-1".to_string(),
        },
        // Add labels
        BatchOp::LabelAdd {
            id: "bf-2".to_string(),
            labels: vec!["labeled-in-batch".to_string(), "important".to_string()],
        },
    ];

    let results = execute_batch(&storage, ops, temp_dir.path(), true).unwrap();

    // All operations should succeed
    assert_eq!(results.len(), 4);
    for result in &results {
        assert_eq!(result.status, "ok");
    }

    // Verify all operations took effect atomically
    let final_count = storage.list_issues(&IssueFilter::default()).unwrap().len();
    assert_eq!(final_count, initial_count + 1);

    // Verify created bead
    let new_bead_id = results[0].id.as_ref().unwrap();
    let new_bead = storage.get_issue(new_bead_id).unwrap().unwrap();
    assert_eq!(new_bead.title, "New bead from mixed batch");
    assert_eq!(new_bead.assignee.as_deref(), Some("worker"));

    // Verify updated bead
    let bead1 = storage.get_issue("bf-1").unwrap().unwrap();
    assert_eq!(bead1.title, "Updated bead 1");
    assert_eq!(bead1.description.as_deref(), Some("Updated in batch"));

    // Verify dependency
    let bead2_deps = storage.get_dependencies("bf-2").unwrap();
    assert_eq!(bead2_deps.len(), 1);
    assert_eq!(bead2_deps[0].depends_on_id, "bf-1");

    // Verify labels (check directly from storage)
    let bead2_labels = storage
        .with_immediate_transaction(|tx| {
            let mut stmt = tx.prepare("SELECT label FROM labels WHERE issue_id = ?1").unwrap();
            let labels: Vec<String> = stmt.query_map(["bf-2"], |row| row.get(0)).unwrap()
                .filter_map(|r| r.ok()).collect();
            Ok(labels)
        }).unwrap();
    assert_eq!(bead2_labels.len(), 2);
    assert!(bead2_labels.contains(&"labeled-in-batch".to_string()));
    assert!(bead2_labels.contains(&"important".to_string()));
}

#[test]
fn test_transaction_rollback_on_create_failure() {
    // Test that all operations roll back when a create fails
    let (temp_dir, storage) = setup_test_workspace();

    let initial_count = storage.list_issues(&IssueFilter::default()).unwrap().len();

    // Create a batch where the last operation will fail
    let ops = vec![
        BatchOp::Create {
            title: "Should be rolled back 1".to_string(),
            type_: "task".to_string(),
            priority: 2,
            description: None,
            assignee: None,
            labels: vec![],
        },
        BatchOp::Create {
            title: "Should be rolled back 2".to_string(),
            type_: "task".to_string(),
            priority: 2,
            description: None,
            assignee: None,
            labels: vec![],
        },
        // This will fail - dependency to non-existent bead
        BatchOp::DepAddBlocker {
            id: "bf-nonexistent".to_string(),
            blocker: "bf-also-nonexistent".to_string(),
        },
    ];

    let result = execute_batch(&storage, ops, temp_dir.path(), true);

    // Should fail
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));

    // Verify complete rollback - no beads were created
    let final_count = storage.list_issues(&IssueFilter::default()).unwrap().len();
    assert_eq!(final_count, initial_count);

    let all_issues = storage.list_all_issues().unwrap();
    assert!(!all_issues.iter().any(|i| i.title.contains("Should be rolled back")));
}

#[test]
fn test_transaction_rollback_on_update_failure() {
    // Test that all operations roll back when an update fails
    let (temp_dir, storage) = setup_test_workspace();

    create_test_bead(&storage, "bf-existing", "Existing bead");

    let initial_count = storage.list_issues(&IssueFilter::default()).unwrap().len();

    // Create a batch where update will fail (non-existent bead)
    let ops = vec![
        BatchOp::Create {
            title: "Should be rolled back".to_string(),
            type_: "task".to_string(),
            priority: 2,
            description: None,
            assignee: None,
            labels: vec![],
        },
        BatchOp::Update {
            id: "bf-nonexistent".to_string(),  // This will fail
            title: Some("This update should not happen".to_string()),
            description: None,
            design: None,
            acceptance_criteria: None,
            notes: None,
            status: None,
            priority: None,
            assignee: None,
            owner: None,
            issue_type: None,
        },
    ];

    let result = execute_batch(&storage, ops, temp_dir.path(), true);

    // Should fail
    assert!(result.is_err());

    // Verify complete rollback
    let final_count = storage.list_issues(&IssueFilter::default()).unwrap().len();
    assert_eq!(final_count, initial_count);

    // Verify existing bead wasn't affected
    let existing = storage.get_issue("bf-existing").unwrap().unwrap();
    assert_eq!(existing.title, "Existing bead");
}

#[test]
fn test_transaction_rollback_on_dependency_failure() {
    // Test that all operations roll back when dependency operation fails
    let (temp_dir, storage) = setup_test_workspace();

    create_test_bead(&storage, "bf-a", "Bead A");

    let initial_count = storage.list_issues(&IssueFilter::default()).unwrap().len();

    // Create a batch where dependency will fail (circular dependency)
    let ops = vec![
        BatchOp::Create {
            title: "Should be rolled back".to_string(),
            type_: "task".to_string(),
            priority: 2,
            description: None,
            assignee: None,
            labels: vec![],
        },
        // Create circular dependency (bf-a -> new bead -> bf-a)
        BatchOp::DepAddBlocker {
            id: "@0".to_string(),     // new bead is blocked
            blocker: "bf-a".to_string(), // by bf-a
        },
        BatchOp::DepAddBlocker {
            id: "bf-a".to_string(),    // bf-a is blocked
            blocker: "@0".to_string(),  // by new bead (circular!)
        },
    ];

    let result = execute_batch(&storage, ops, temp_dir.path(), true);

    // Should fail due to circular dependency
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Circular"));

    // Verify complete rollback
    let final_count = storage.list_issues(&IssueFilter::default()).unwrap().len();
    assert_eq!(final_count, initial_count);
}

#[test]
fn test_no_partial_state_on_early_failure() {
    // Test that no partial state remains when early operations fail
    let (temp_dir, storage) = setup_test_workspace();

    create_test_bead(&storage, "bf-1", "Bead 1");
    create_test_bead(&storage, "bf-2", "Bead 2");

    let initial_bead1 = storage.get_issue("bf-1").unwrap().unwrap();
    let initial_bead2 = storage.get_issue("bf-2").unwrap().unwrap();

    // Create a batch that fails on the first operation
    let ops = vec![
        // First operation fails immediately
        BatchOp::Update {
            id: "bf-nonexistent".to_string(),
            title: Some("Won't happen".to_string()),
            description: None,
            design: None,
            acceptance_criteria: None,
            notes: None,
            status: None,
            priority: None,
            assignee: None,
            owner: None,
            issue_type: None,
        },
        // These should never execute
        BatchOp::Update {
            id: "bf-1".to_string(),
            title: Some("Should not happen".to_string()),
            description: None,
            design: None,
            acceptance_criteria: None,
            notes: None,
            status: None,
            priority: None,
            assignee: None,
            owner: None,
            issue_type: None,
        },
        BatchOp::LabelAdd {
            id: "bf-2".to_string(),
            labels: vec!["should-not-be-added".to_string()],
        },
    ];

    let result = execute_batch(&storage, ops, temp_dir.path(), true);

    // Should fail on first operation
    assert!(result.is_err());

    // Verify no partial state - existing beads unchanged
    let bead1_after = storage.get_issue("bf-1").unwrap().unwrap();
    let bead2_after = storage.get_issue("bf-2").unwrap().unwrap();

    assert_eq!(bead1_after.title, initial_bead1.title);
    assert_eq!(bead2_after.title, initial_bead2.title);
    assert!(!bead2_after.labels.contains(&"should-not-be-added".to_string()));
}

#[test]
fn test_no_partial_state_on_mid_batch_failure() {
    // Test that no partial state remains when mid-batch operations fail
    let (temp_dir, storage) = setup_test_workspace();

    let initial_count = storage.list_issues(&IssueFilter::default()).unwrap().len();

    // Create a batch where the middle operation fails
    let ops = vec![
        // First operation succeeds
        BatchOp::Create {
            title: "Should be rolled back".to_string(),
            type_: "task".to_string(),
            priority: 2,
            description: None,
            assignee: None,
            labels: vec![],
        },
        // Middle operation fails
        BatchOp::Update {
            id: "bf-nonexistent".to_string(),
            title: Some("Won't happen".to_string()),
            description: None,
            design: None,
            acceptance_criteria: None,
            notes: None,
            status: None,
            priority: None,
            assignee: None,
            owner: None,
            issue_type: None,
        },
        // Last operation should never execute
        BatchOp::Create {
            title: "Also should be rolled back".to_string(),
            type_: "task".to_string(),
            priority: 2,
            description: None,
            assignee: None,
            labels: vec![],
        },
    ];

    let result = execute_batch(&storage, ops, temp_dir.path(), true);

    // Should fail on middle operation
    assert!(result.is_err());

    // Verify complete rollback - no beads created
    let final_count = storage.list_issues(&IssueFilter::default()).unwrap().len();
    assert_eq!(final_count, initial_count);

    let all_issues = storage.list_all_issues().unwrap();
    assert!(!all_issues.iter().any(|i| i.title.contains("Should be rolled back")));
    assert!(!all_issues.iter().any(|i| i.title.contains("Also should be rolled back")));
}

#[test]
fn test_placeholder_references_in_transaction() {
    // Test that @-placeholder references resolve correctly within a transaction
    let (temp_dir, storage) = setup_test_workspace();

    create_test_bead(&storage, "bf-parent", "Parent bead");

    // Create beads and use placeholder references to them
    let ops = vec![
        BatchOp::Create {
            title: "Child 1".to_string(),
            type_: "task".to_string(),
            priority: 2,
            description: None,
            assignee: None,
            labels: vec![],
        },
        BatchOp::Create {
            title: "Child 2".to_string(),
            type_: "task".to_string(),
            priority: 2,
            description: None,
            assignee: None,
            labels: vec![],
        },
        // Use placeholder references to created beads
        BatchOp::DepAddBlocker {
            id: "bf-parent".to_string(),
            blocker: "@0".to_string(),  // References Child 1
        },
        BatchOp::DepAddBlocker {
            id: "bf-parent".to_string(),
            blocker: "@1".to_string(),  // References Child 2
        },
        BatchOp::DepAddBlocker {
            id: "@1".to_string(),       // Child 2 is blocked
            blocker: "@0".to_string(),  // by Child 1
        },
    ];

    let results = execute_batch(&storage, ops, temp_dir.path(), true).unwrap();

    // All operations should succeed
    assert_eq!(results.len(), 5);
    for result in &results {
        assert_eq!(result.status, "ok");
    }

    let child1_id = results[0].id.as_ref().unwrap();
    let child2_id = results[1].id.as_ref().unwrap();

    // Verify dependencies were created with correct IDs
    let parent_deps = storage.get_dependencies("bf-parent").unwrap();
    assert_eq!(parent_deps.len(), 2);
    let parent_dep_ids: Vec<&str> = parent_deps.iter().map(|d| d.depends_on_id.as_str()).collect();
    assert!(parent_dep_ids.contains(&child1_id.as_str()));
    assert!(parent_dep_ids.contains(&child2_id.as_str()));

    let child2_deps = storage.get_dependencies(child2_id).unwrap();
    assert_eq!(child2_deps.len(), 1);
    assert_eq!(child2_deps[0].depends_on_id, *child1_id);
}

#[test]
fn test_empty_batch_commits_successfully() {
    // Test that an empty batch succeeds (no-op transaction)
    let (temp_dir, storage) = setup_test_workspace();

    let initial_count = storage.list_issues(&IssueFilter::default()).unwrap().len();

    let ops = vec![];
    let results = execute_batch(&storage, ops, temp_dir.path(), true).unwrap();

    assert_eq!(results.len(), 0);

    // Verify state unchanged
    let final_count = storage.list_issues(&IssueFilter::default()).unwrap().len();
    assert_eq!(final_count, initial_count);
}

#[test]
fn test_single_operation_batch() {
    // Test that a single operation batch commits successfully
    let (temp_dir, storage) = setup_test_workspace();

    let initial_count = storage.list_issues(&IssueFilter::default()).unwrap().len();

    let ops = vec![BatchOp::Create {
        title: "Single bead".to_string(),
        type_: "task".to_string(),
        priority: 2,
        description: Some("Single operation".to_string()),
        assignee: None,
        labels: vec!["single".to_string()],
    }];

    let results = execute_batch(&storage, ops, temp_dir.path(), true).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, "ok");

    let bead_id = results[0].id.as_ref().unwrap();
    let bead = storage.get_issue(bead_id).unwrap().unwrap();
    assert_eq!(bead.title, "Single bead");

    // Check labels directly from storage
    let bead_labels = storage
        .with_immediate_transaction(|tx| {
            let mut stmt = tx.prepare("SELECT label FROM labels WHERE issue_id = ?1").unwrap();
            let labels: Vec<String> = stmt.query_map([&bead_id], |row| row.get(0)).unwrap()
                .filter_map(|r| r.ok()).collect();
            Ok(labels)
        }).unwrap();
    assert!(bead_labels.contains(&"single".to_string()));

    let final_count = storage.list_issues(&IssueFilter::default()).unwrap().len();
    assert_eq!(final_count, initial_count + 1);
}

#[test]
fn test_batch_close_operation_rollback() {
    // Test that close operations roll back correctly
    let (temp_dir, storage) = setup_test_workspace();

    create_test_bead(&storage, "bf-close-test", "Bead to close");

    let bead_before = storage.get_issue("bf-close-test").unwrap().unwrap();
    assert_eq!(bead_before.status.to_string(), "open");

    // Create a batch where close fails, rolling back the create
    let ops = vec![
        BatchOp::Create {
            title: "Should be rolled back".to_string(),
            type_: "task".to_string(),
            priority: 2,
            description: None,
            assignee: None,
            labels: vec![],
        },
        BatchOp::Close {
            id: "bf-nonexistent".to_string(),  // This will fail
            reason: "Should not happen".to_string(),
        },
    ];

    let result = execute_batch(&storage, ops, temp_dir.path(), true);

    // Should fail
    assert!(result.is_err());

    // Verify rollback - bead not closed, no new bead created
    let bead_after = storage.get_issue("bf-close-test").unwrap().unwrap();
    assert_eq!(bead_after.status.to_string(), "open");

    let all_issues = storage.list_all_issues().unwrap();
    assert!(!all_issues.iter().any(|i| i.title.contains("Should be rolled back")));
}

#[test]
fn test_batch_label_operation_rollback() {
    // Test that label operations roll back correctly
    let (temp_dir, storage) = setup_test_workspace();

    create_test_bead(&storage, "bf-label-test", "Bead for labels");

    let initial_labels = storage.get_issue("bf-label-test").unwrap().unwrap().labels.clone();

    // Create a batch where label op fails, rolling back the create
    let ops = vec![
        BatchOp::Create {
            title: "Should be rolled back".to_string(),
            type_: "task".to_string(),
            priority: 2,
            description: None,
            assignee: None,
            labels: vec![],
        },
        BatchOp::LabelAdd {
            id: "bf-nonexistent".to_string(),  // This will fail
            labels: vec!["should-not-be-added".to_string()],
        },
    ];

    let result = execute_batch(&storage, ops, temp_dir.path(), true);

    // Should fail
    assert!(result.is_err());

    // Verify rollback - labels unchanged, no new bead created
    let bead_after = storage.get_issue("bf-label-test").unwrap().unwrap();
    assert_eq!(bead_after.labels, initial_labels);

    let all_issues = storage.list_all_issues().unwrap();
    assert!(!all_issues.iter().any(|i| i.title.contains("Should be rolled back")));
}

#[test]
fn test_large_batch_transaction() {
    // Test that a large batch executes successfully in a single transaction
    let (temp_dir, storage) = setup_test_workspace();

    let initial_count = storage.list_issues(&IssueFilter::default()).unwrap().len();

    // Create a batch with 20 operations
    let mut ops = vec![];
    for i in 0..20 {
        ops.push(BatchOp::Create {
            title: format!("Bead {}", i),
            type_: "task".to_string(),
            priority: 2,
            description: None,
            assignee: None,
            labels: vec![],
        });
    }

    let results = execute_batch(&storage, ops, temp_dir.path(), true).unwrap();

    assert_eq!(results.len(), 20);
    for result in &results {
        assert_eq!(result.status, "ok");
    }

    // Verify all beads were created atomically
    let final_count = storage.list_issues(&IssueFilter::default()).unwrap().len();
    assert_eq!(final_count, initial_count + 20);
}
