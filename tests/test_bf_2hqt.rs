//! Regression test for bf-2hqt: count_unflushed over-reports after doctor --repair / sync --import cycle
//!
//! Issue: After running `bf doctor --repair` or `bf sync --import` (which rebuilds the db FROM issues.jsonl),
//! the `count_unflushed` metric reports a positive number even though the db and JSONL are in sync.

use std::fs;
use tempfile::TempDir;

/// Helper to initialize a test workspace
fn init_test_workspace() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    // Initialize workspace
    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    temp_dir
}

/// Helper to check unflushed count via public API
fn get_unflushed_count(workspace: &tempfile::TempDir) -> usize {
    let result = bead_forge::doctor::check(workspace.path()).unwrap();
    result.unflushed_count
}

/// Test that after a repair cycle with no subsequent mutations, count_unflushed == 0
#[test]
fn test_repair_cycle_leaves_zero_unflushed() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    let metadata = bead_forge::config::load_metadata(&beads_dir).unwrap();
    let _jsonl_path = beads_dir.join(&metadata.jsonl_export);

    // Create initial database with some beads
    let storage = bead_forge::storage::Storage::open(&beads_dir.join(&metadata.database)).unwrap();

    // Create a test bead
    let issue1 = bead_forge::model::Issue {
        id: "bf-test1".to_string(),
        title: "Test Issue 1".to_string(),
        status: bead_forge::model::Status::Open,
        priority: bead_forge::model::Priority::MEDIUM,
        issue_type: bead_forge::model::IssueType::Task,
        source_repo: Some(".".to_string()),
        ..Default::default()
    };
    storage.create_issue(&issue1).unwrap();

    let issue2 = bead_forge::model::Issue {
        id: "bf-test2".to_string(),
        title: "Test Issue 2".to_string(),
        status: bead_forge::model::Status::Open,
        priority: bead_forge::model::Priority::HIGH,
        issue_type: bead_forge::model::IssueType::Bug,
        source_repo: Some(".".to_string()),
        ..Default::default()
    };
    storage.create_issue(&issue2).unwrap();

    // Flush to JSONL (simulates "recently flushed issues.jsonl")
    bead_forge::sync::flush(workspace).unwrap();

    // Verify unflushed count is 0 after flush
    let unflushed_before = get_unflushed_count(&temp_dir);
    assert_eq!(unflushed_before, 0, "Should be 0 after flush");

    // Run repair (simulates repair scenario)
    let imported = bead_forge::doctor::repair(workspace, false, false).unwrap();
    assert_eq!(imported, 2, "Should import 2 beads from JSONL");

    // After repair, unflushed count should still be 0 (db was rebuilt from JSONL)
    let unflushed_after = get_unflushed_count(&temp_dir);
    assert_eq!(unflushed_after, 0, "Repair should not leave unflushed beads");
}

/// Test that after an import cycle with no subsequent mutations, count_unflushed == 0
#[test]
fn test_import_cycle_leaves_zero_unflushed() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    let metadata = bead_forge::config::load_metadata(&beads_dir).unwrap();

    // Create initial database with some beads
    let storage = bead_forge::storage::Storage::open(&beads_dir.join(&metadata.database)).unwrap();

    let issue1 = bead_forge::model::Issue {
        id: "bf-test1".to_string(),
        title: "Test Issue 1".to_string(),
        status: bead_forge::model::Status::Open,
        priority: bead_forge::model::Priority::MEDIUM,
        issue_type: bead_forge::model::IssueType::Task,
        source_repo: Some(".".to_string()),
        ..Default::default()
    };
    storage.create_issue(&issue1).unwrap();

    // Flush to JSONL
    bead_forge::sync::flush(workspace).unwrap();

    // Verify unflushed count is 0 after flush
    let unflushed_before = get_unflushed_count(&temp_dir);
    assert_eq!(unflushed_before, 0, "Should be 0 after flush");

    // Run import (simulates sync --import scenario)
    let result = bead_forge::sync::import(workspace).unwrap();
    assert_eq!(result.imported, 0, "Should import 0 new beads (already in sync)");
    assert_eq!(result.skipped, 1, "Should skip 1 unchanged bead");

    // After import, unflushed count should still be 0
    let unflushed_after = get_unflushed_count(&temp_dir);
    assert_eq!(unflushed_after, 0, "Import should not leave unflushed beads");
}

/// Test that after a full repair cycle (delete DB, repair), count_unflushed == 0
#[test]
fn test_delete_db_then_repair_leaves_zero_unflushed() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    let metadata = bead_forge::config::load_metadata(&beads_dir).unwrap();
    let db_path = beads_dir.join(&metadata.database);
    let jsonl_path = beads_dir.join(&metadata.jsonl_export);

    // Create initial database with a bead
    let storage = bead_forge::storage::Storage::open(&db_path).unwrap();

    let issue1 = bead_forge::model::Issue {
        id: "bf-test1".to_string(),
        title: "Test Issue 1".to_string(),
        status: bead_forge::model::Status::Open,
        priority: bead_forge::model::Priority::MEDIUM,
        issue_type: bead_forge::model::IssueType::Task,
        source_repo: Some(".".to_string()),
        ..Default::default()
    };
    storage.create_issue(&issue1).unwrap();

    // Flush to JSONL
    bead_forge::sync::flush(workspace).unwrap();

    // Verify JSONL exists and contains the bead
    assert!(jsonl_path.exists(), "JSONL should exist after flush");
    let jsonl_content = fs::read_to_string(&jsonl_path).unwrap();
    assert!(jsonl_content.contains("bf-test1"), "JSONL should contain the bead");

    // Delete the database (simulates a repair scenario where DB is corrupted/missing)
    fs::remove_file(&db_path).unwrap();

    // Run repair (which will recreate DB from JSONL)
    let imported = bead_forge::doctor::repair(workspace, false, false).unwrap();
    assert_eq!(imported, 1, "Should import 1 bead from JSONL");

    // After repair, unflushed count should be 0
    let unflushed_after = get_unflushed_count(&temp_dir);
    assert_eq!(unflushed_after, 0, "Repair after DB deletion should not leave unflushed beads");
}

/// Test the exact scenario: recently flushed JSONL -> repair -> count_unflushed should be 0
#[test]
fn test_recent_flush_then_repair_unflushed_zero() {
    let temp_dir = init_test_workspace();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    let metadata = bead_forge::config::load_metadata(&beads_dir).unwrap();
    let db_path = beads_dir.join(&metadata.database);
    let jsonl_path = beads_dir.join(&metadata.jsonl_export);

    // Create a bead and flush it to JSONL (simulates "recently flushed")
    let storage = bead_forge::storage::Storage::open(&db_path).unwrap();
    let issue = bead_forge::model::Issue {
        id: "bf-recent".to_string(),
        title: "Recently Flushed Bead".to_string(),
        status: bead_forge::model::Status::Open,
        priority: bead_forge::model::Priority::MEDIUM,
        issue_type: bead_forge::model::IssueType::Task,
        source_repo: Some(".".to_string()),
        ..Default::default()
    };
    storage.create_issue(&issue).unwrap();
    bead_forge::sync::flush(workspace).unwrap();

    // Verify flush completed successfully
    let jsonl_content = fs::read_to_string(&jsonl_path).unwrap();
    assert!(jsonl_content.contains("bf-recent"), "JSONL should contain the recently flushed bead");

    // Run repair
    let imported = bead_forge::doctor::repair(workspace, false, false).unwrap();
    assert_eq!(imported, 1, "Should import 1 bead from JSONL");

    // Verify count_unflushed is 0
    let unflushed = get_unflushed_count(&temp_dir);
    assert_eq!(unflushed, 0, "After repair of recently flushed JSONL, count_unflushed should be 0");
}
