//! Test NULL assignee persistence in storage layer
//!
//! This test verifies that:
//! 1. rusqlite properly converts Option::None to NULL in SQL
//! 2. Assignee can be set, cleared, and read back correctly
//! 3. The storage layer handles the full cycle: Some → None → Some

use bead_forge::config::load_config;
use bead_forge::model::{Issue, IssueChanges, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a temporary workspace for testing
fn setup_test_workspace() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("test-workspace");
    std::fs::create_dir_all(&workspace_dir).unwrap();
    let beads_dir = workspace_dir.join(".beads");
    std::fs::create_dir_all(&beads_dir).unwrap();

    // Initialize workspace
    let config_path = beads_dir.join("config.yaml");
    std::fs::write(
        &config_path,
        r#"issue_prefixes: [bf]
default_priority: 2
default_type: task
claim_ttl_minutes: 30
"#,
    )
    .unwrap();

    let metadata_path = beads_dir.join("metadata.json");
    std::fs::write(
        &metadata_path,
        r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
    )
    .unwrap();

    let db_path = beads_dir.join("beads.db");
    Storage::open(&db_path).unwrap();

    (temp_dir, beads_dir)
}

/// Create a test bead with optional assignee
fn create_test_bead_with_assignee(
    storage: &Storage,
    id: &str,
    title: &str,
    assignee: Option<&str>,
) -> Issue {
    let now = Utc::now();

    let issue = Issue {
        id: id.to_string(),
        title: title.to_string(),
        description: Some("Test description".to_string()),
        acceptance_criteria: None,
        design: None,
        notes: None,
        status: Status::Open,
        priority: Priority(2),
        issue_type: IssueType::Task,
        assignee: assignee.map(|s| s.to_string()),
        owner: None,
        estimated_minutes: None,
        created_at: now,
        created_by: Some("test".to_string()),
        updated_at: now,
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
        content_hash: None,
        labels: vec![],
        dependencies: vec![],
        comments: vec![],
        events: vec![],
        annotations: Default::default(),
    };

    storage.create_issue(&issue).unwrap();
    storage.get_issue(&id).unwrap().unwrap()
}

#[test]
/// Test that rusqlite properly converts None → NULL when creating a bead
fn test_create_with_none_assignee_persists_as_null() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create bead with None assignee
    let bead = create_test_bead_with_assignee(
        &storage,
        "bf-none-test",
        "Test create with None assignee",
        None,
    );

    // Verify assignee is None (NULL in database)
    assert!(bead.assignee.is_none(), "assignee should be None after creation");

    // Re-read from database to verify persistence
    let reloaded = storage.get_issue("bf-none-test").unwrap().unwrap();
    assert!(
        reloaded.assignee.is_none(),
        "assignee should remain None after database roundtrip"
    );
}

#[test]
/// Test full cycle: create with assignee, clear it, read back as NULL
fn test_assignee_clear_persists_as_null() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Step 1: Create bead with assignee
    let bead = create_test_bead_with_assignee(
        &storage,
        "bf-clear-cycle",
        "Test assignee clear cycle",
        Some("alice"),
    );

    assert_eq!(
        bead.assignee.as_deref(),
        Some("alice"),
        "initial assignee should be set"
    );

    // Step 2: Clear assignee using empty string (storage layer converts to NULL)
    let changes = IssueChanges {
        assignee: Some(String::new()), // Empty string triggers NULL storage
        ..Default::default()
    };
    storage
        .update_issue(&bead.id, &changes)
        .expect("update should succeed");

    // Step 3: Read back and verify assignee is None (NULL in database)
    let cleared_bead = storage.get_issue(&bead.id).unwrap().unwrap();
    assert!(
        cleared_bead.assignee.is_none(),
        "assignee should be None after clearing"
    );

    // Step 4: Re-read from database to confirm NULL persistence
    let reloaded = storage.get_issue(&bead.id).unwrap().unwrap();
    assert!(
        reloaded.assignee.is_none(),
        "assignee should remain None (NULL) after database roundtrip"
    );
}

#[test]
/// Test that assignee can be set again after being cleared (NULL → Some)
fn test_assignee_set_after_clear() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create with assignee
    let bead = create_test_bead_with_assignee(
        &storage,
        "bf-reset-after-clear",
        "Test reassign after clear",
        Some("bob"),
    );

    // Clear assignee
    let changes_clear = IssueChanges {
        assignee: Some(String::new()),
        ..Default::default()
    };
    storage
        .update_issue(&bead.id, &changes_clear)
        .unwrap();

    let cleared = storage.get_issue(&bead.id).unwrap().unwrap();
    assert!(cleared.assignee.is_none(), "should be cleared");

    // Set new assignee (NULL → Some)
    let changes_set = IssueChanges {
        assignee: Some("charlie".to_string()),
        ..Default::default()
    };
    storage
        .update_issue(&bead.id, &changes_set)
        .unwrap();

    let reassigned = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(
        reassigned.assignee.as_deref(),
        Some("charlie"),
        "should have new assignee after being cleared"
    );
}

#[test]
/// Test that empty string assignee does NOT persist as empty string, but as NULL
fn test_empty_string_assignee_becomes_null() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create bead with assignee
    let bead = create_test_bead_with_assignee(
        &storage,
        "bf-empty-string",
        "Test empty string handling",
        Some("dave"),
    );

    // Update with empty string - should become NULL, not empty string
    let changes = IssueChanges {
        assignee: Some("".to_string()),
        ..Default::default()
    };
    storage
        .update_issue(&bead.id, &changes)
        .unwrap();

    let updated = storage.get_issue(&bead.id).unwrap().unwrap();
    assert!(
        updated.assignee.is_none(),
        "empty string should be stored as NULL, not empty string"
    );

    // Verify it's not Some("") but actually None
    match updated.assignee {
        None => (), // Correct - should be NULL
        Some(s) => panic!("Expected None but got Some({:?})", s),
    }
}
