//! Test atomic batch operations for NEEDLE mitosis pattern.
//!
//! Mitosis: split one parent bead into multiple child beads with dependencies.
//! All operations must be atomic - if the process crashes mid-batch, no
//! partial state is committed.

use bead_forge::batch::{execute_batch, mitosis, BatchOp};
use bead_forge::config::{init_workspace, load_metadata};
use bead_forge::model::{Issue, IssueFilter};
use bead_forge::storage::Storage;
use std::process::Command;
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
    let parent = Issue::new(
        parent_id.clone(),
        "Parent task".to_string(),
        ".".to_string(),
    );
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
            id: parent_id.clone(),     // parent is blocked
            blocker: "@0".to_string(), // first created child (blocks)
        },
        BatchOp::DepAddBlocker {
            id: parent_id.clone(),     // parent is blocked
            blocker: "@1".to_string(), // second created child (blocks)
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
    let parent = Issue::new(
        parent_id.clone(),
        "Parent task".to_string(),
        ".".to_string(),
    );
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
            id: "non-existent-parent".to_string(), // This will fail
            blocker: "@0".to_string(),             // first created child (blocks)
        },
    ];

    let result = execute_batch(&storage, ops, temp_dir.path());

    // Should fail
    assert!(result.is_err());

    // Verify no partial state: bead count should be unchanged
    let after_count = storage.list_issues(&IssueFilter::default()).unwrap().len();
    assert_eq!(
        before_count, after_count,
        "Batch should have rolled back completely"
    );
}

/// Test that mitosis() produces BatchOps with correct @-reference placeholders.
#[test]
fn test_mitosis_helper_produces_at_references() {
    let ops = mitosis(
        "bf-parent",
        vec![
            ("Child A".to_string(), "task".to_string(), 2),
            ("Child B".to_string(), "bug".to_string(), 0),
        ],
        Some("Splitting parent".to_string()),
    )
    .unwrap();

    // 2 creates + 2 dep_add_blocker + 1 close = 5 ops
    assert_eq!(ops.len(), 5);

    // First two ops are creates
    assert!(matches!(&ops[0], BatchOp::Create { title, .. } if title == "Child A"));
    assert!(matches!(&ops[1], BatchOp::Create { title, .. } if title == "Child B"));

    // Dep ops use @0 and @1 as blockers (children block the parent)
    match &ops[2] {
        BatchOp::DepAddBlocker { id, blocker } => {
            assert_eq!(id, "bf-parent", "parent should be the blocked bead");
            assert_eq!(blocker, "@0", "first child should be referenced by @0");
        }
        _ => panic!("expected DepAddBlocker at op[2]"),
    }
    match &ops[3] {
        BatchOp::DepAddBlocker { id, blocker } => {
            assert_eq!(id, "bf-parent", "parent should be the blocked bead");
            assert_eq!(blocker, "@1", "second child should be referenced by @1");
        }
        _ => panic!("expected DepAddBlocker at op[3]"),
    }

    // Last op closes the parent
    match &ops[4] {
        BatchOp::Close { id, reason } => {
            assert_eq!(id, "bf-parent");
            assert_eq!(reason, "Splitting parent");
        }
        _ => panic!("expected Close at op[4]"),
    }
}

/// End-to-end CLI test: bf batch --json with @-references exercises the mitosis pattern
/// through the actual binary, verifying that the JSON serialization path resolves
/// placeholders correctly.
#[test]
fn test_cli_batch_json_at_references() {
    let temp_dir = TempDir::new().unwrap();
    let beads_dir = temp_dir.path().join(".beads");
    std::fs::create_dir(&beads_dir).unwrap();
    init_workspace(&beads_dir, "bf").unwrap();

    let metadata = load_metadata(&beads_dir).unwrap();
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path).unwrap();

    // Create the parent bead via the storage API
    let parent_id = "bf-parent";
    let parent = Issue::new(
        parent_id.to_string(),
        "Parent task".to_string(),
        ".".to_string(),
    );
    storage.create_issue(&parent).unwrap();
    drop(storage); // release the connection before the subprocess opens it

    // Build the JSON payload with @-references (NEEDLE mitosis pattern)
    let json_payload = serde_json::json!([
        {"op": "create", "title": "Child Alpha", "type_": "task", "priority": 2},
        {"op": "create", "title": "Child Beta",  "type_": "bug",  "priority": 1},
        {"op": "dep_add_blocker", "id": "bf-parent", "blocker": "@0"},
        {"op": "dep_add_blocker", "id": "bf-parent", "blocker": "@1"},
        {"op": "close", "id": "bf-parent", "reason": "Split via CLI batch"}
    ])
    .to_string();

    let bf_bin = env!("CARGO_BIN_EXE_bf");
    let output = Command::new(bf_bin)
        .arg("--workspace")
        .arg(temp_dir.path())
        .arg("batch")
        .arg("--json")
        .arg(&json_payload)
        .output()
        .expect("failed to run bf binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "bf batch --json failed.\nstdout: {stdout}\nstderr: {stderr}"
    );

    // Re-open storage to verify results
    let storage = Storage::open(&db_path).unwrap();

    // Parent should be closed
    let parent = storage.get_issue(parent_id).unwrap().unwrap();
    assert_eq!(
        parent.status.to_string(),
        "closed",
        "parent should be closed after mitosis"
    );
    assert_eq!(parent.close_reason.as_deref(), Some("Split via CLI batch"));

    // Two children should have been created and are open
    let all_issues = storage.list_all_issues().unwrap();
    let children: Vec<_> = all_issues.iter().filter(|i| i.id != parent_id).collect();
    assert_eq!(children.len(), 2, "expected exactly 2 child beads");

    let titles: Vec<&str> = children.iter().map(|i| i.title.as_str()).collect();
    assert!(titles.contains(&"Child Alpha"), "Child Alpha not found");
    assert!(titles.contains(&"Child Beta"), "Child Beta not found");

    for child in &children {
        assert_eq!(
            child.status.to_string(),
            "open",
            "child {} should be open",
            child.id
        );
    }

    // Both children should appear as blockers in the parent's dependencies
    let parent_deps = storage.get_dependencies(parent_id).unwrap();
    assert_eq!(
        parent_deps.len(),
        2,
        "parent should have 2 blocking dependencies"
    );

    let child_ids: Vec<&str> = children.iter().map(|i| i.id.as_str()).collect();
    for dep in &parent_deps {
        assert!(
            child_ids.contains(&dep.depends_on_id.as_str()),
            "dependency {} not in child IDs {:?}",
            dep.depends_on_id,
            child_ids
        );
    }

    // CLI output should mention all 5 ops as ok
    for i in 0..5 {
        assert!(
            stdout.contains(&format!("[op {i}] ok")),
            "expected '[op {i}] ok' in stdout:\n{stdout}"
        );
    }
}
