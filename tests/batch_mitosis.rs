//! Test atomic batch operations for NEEDLE mitosis pattern.
//!
//! Mitosis: split one parent bead into multiple child beads with dependencies.
//! All operations must be atomic - if the process crashes mid-batch, no
//! partial state is committed.

use bead_forge::batch::{execute_batch, BatchOp};
use bead_forge::config::{init_workspace, load_metadata};
use bead_forge::model::{Issue, IssueFilter, IssueType, Priority};
use bead_forge::storage::Storage;
use chrono::Utc;
use tempfile::TempDir;

#[test]
fn test_mitosis_atomic_batch() {
    // Setup workspace with a parent bead
    let temp_dir = TempDir::new().unwrap();
    let beads_dir = temp_dir.path().join(".beads");
    std::fs::create_dir(&beads_dir).unwrap();
    init_workspace(&beads_dir, "bf").unwrap();

    let metadata = load_metadata(&beads_dir).unwrap();
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path).unwrap();

    // Create parent bead
    let parent_id = "bf-parent".to_string();
    let parent = Issue::new(parent_id.clone(), "Parent task".to_string(), ".".to_string());
    storage.create_issue(&parent).unwrap();

    // Verify parent exists and is open
    let parent = storage.get_issue(&parent_id).unwrap().unwrap();
    assert_eq!(parent.status.to_string(), "open");

    // Execute mitosis batch: create 2 children, link them as blockers, close parent
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
        // Use placeholder references @0 and @1 for the created children
        // For mitosis: children block the parent (parent depends on children)
        BatchOp::DepAddBlocker {
            parent: "@0".to_string(),      // first created child (blocks)
            child: parent_id.clone(),      // parent is blocked
        },
        BatchOp::DepAddBlocker {
            parent: "@1".to_string(),      // second created child (blocks)
            child: parent_id.clone(),      // parent is blocked
        },
        BatchOp::Close {
            id: parent_id.clone(),
            reason: "Split into children".to_string(),
        },
    ];

    let results = execute_batch(&storage, ops, temp_dir.path()).unwrap();

    // All operations should succeed
    assert_eq!(results.len(), 5);
    assert_eq!(results[0].status, "ok");
    assert_eq!(results[1].status, "ok");
    assert_eq!(results[2].status, "ok");
    assert_eq!(results[3].status, "ok");
    assert_eq!(results[4].status, "ok");

    // Get the created child IDs
    let child1_id = results[0].id.as_ref().unwrap();
    let child2_id = results[1].id.as_ref().unwrap();

    // Verify parent is closed
    let parent = storage.get_issue(&parent_id).unwrap().unwrap();
    assert_eq!(parent.status.to_string(), "closed");

    // Verify children exist and are blocked by the (closed) parent
    let child1 = storage.get_issue(child1_id).unwrap().unwrap();
    let child2 = storage.get_issue(child2_id).unwrap().unwrap();

    assert_eq!(child1.title, "Child 1");
    assert_eq!(child2.title, "Child 2");
    assert_eq!(child1.status.to_string(), "open");
    assert_eq!(child2.status.to_string(), "open");

    // Verify dependencies: parent depends on children (children block parent)
    let parent_deps = storage.get_dependencies(&parent_id).unwrap();

    assert_eq!(parent_deps.len(), 2);
    assert!(parent_deps.iter().any(|d| d.depends_on_id == *child1_id));
    assert!(parent_deps.iter().any(|d| d.depends_on_id == *child2_id));
}

#[test]
fn test_batch_rollback_on_error() {
    // Setup workspace
    let temp_dir = TempDir::new().unwrap();
    let beads_dir = temp_dir.path().join(".beads");
    std::fs::create_dir(&beads_dir).unwrap();
    init_workspace(&beads_dir, "bf").unwrap();

    let metadata = load_metadata(&beads_dir).unwrap();
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path).unwrap();

    // Create parent bead
    let parent_id = "bf-parent".to_string();
    let parent = Issue::new(parent_id.clone(), "Parent task".to_string(), ".".to_string());
    storage.create_issue(&parent).unwrap();

    // Count beads before
    let before_count = storage.list_issues(&IssueFilter::default()).unwrap().len();

    // Try to add dependency to non-existent parent (should fail and rollback)
    let ops = vec![
        BatchOp::Create {
            title: "Child 1".to_string(),
            type_: "task".to_string(),
            priority: 2,
            description: None,
            assignee: None,
            labels: vec![],
        },
        BatchOp::DepAddBlocker {
            parent: "@0".to_string(),                 // first created child (blocks)
            child: "non-existent-parent".to_string(), // This will fail
        },
    ];

    let result = execute_batch(&storage, ops, temp_dir.path());

    // Should fail
    assert!(result.is_err());

    // Verify no partial state: bead count should be unchanged
    let after_count = storage.list_issues(&IssueFilter::default()).unwrap().len();
    assert_eq!(before_count, after_count, "Batch should have rolled back completely");
}
