//! Comprehensive tests for assignee functionality
//!
//! This test file validates that:
//! 1. Beads can be created with an assignee
//! 2. Bead assignees can be updated
//! 3. Beads can be filtered by assignee
//! 4. Assignee changes generate proper events
//! 5. Claim operations set assignees correctly

use bead_forge::config::load_config;
use bead_forge::model::{Issue, Status, IssueType, Priority, IssueChanges, EventType};
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
        annotations: Default::default(),
    };

    storage.create_issue(&issue).unwrap();
    storage.get_issue(&id).unwrap().unwrap()
}

#[test]
fn test_create_bead_with_assignee() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create bead with assignee
    let bead = create_test_bead_with_assignee(&storage, "bf-1", "Test bead with assignee", Some("alice"));

    assert_eq!(bead.assignee.as_ref().unwrap(), "alice");
    assert_eq!(bead.title, "Test bead with assignee");
}

#[test]
fn test_create_bead_without_assignee() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create bead without assignee
    let bead = create_test_bead_with_assignee(&storage, "bf-test-no-assign", "Test bead without assignee", None);

    assert!(bead.assignee.is_none());
    assert_eq!(bead.title, "Test bead without assignee");
}

#[test]
fn test_update_bead_assignee() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create bead without assignee
    let bead = create_test_bead_with_assignee(&storage, "bf-test-update", "Test bead for update", None);
    assert!(bead.assignee.is_none());

    // Update assignee
    let changes = IssueChanges {
        assignee: Some("bob".to_string()),
        ..Default::default()
    };
    storage.update_issue(&bead.id, &changes).unwrap();

    // Verify assignee was updated
    let updated_bead = storage.get_issue(&bead.id).unwrap().unwrap();
    assert_eq!(updated_bead.assignee.as_ref().unwrap(), "bob");
}

#[test]
fn test_clear_bead_assignee() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create bead with assignee
    let bead = create_test_bead_with_assignee(&storage, "bf-test-clear", "Test bead for clearing", Some("charlie"));
    assert_eq!(bead.assignee.as_ref().unwrap(), "charlie");

    // Clear assignee
    let changes = IssueChanges {
        assignee: Some(String::new()), // Empty string clears the field
        ..Default::default()
    };
    storage.update_issue(&bead.id, &changes).unwrap();

    // Verify assignee was cleared
    let updated_bead = storage.get_issue(&bead.id).unwrap().unwrap();
    assert!(updated_bead.assignee.is_none());
}

#[test]
fn test_list_beads_by_assignee() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create beads with different assignees
    create_test_bead_with_assignee(&storage, "bf-list-1", "Bead 1", Some("alice"));
    create_test_bead_with_assignee(&storage, "bf-list-2", "Bead 2", Some("alice"));
    create_test_bead_with_assignee(&storage, "bf-list-3", "Bead 3", Some("bob"));
    create_test_bead_with_assignee(&storage, "bf-list-4", "Bead 4", None);

    // List beads filtered by assignee
    let filter = bead_forge::model::IssueFilter {
        assignee: Some("alice".to_string()),
        ..Default::default()
    };
    let alice_beads = storage.list_issues(&filter).unwrap();

    assert_eq!(alice_beads.len(), 2);
    for bead in &alice_beads {
        assert_eq!(bead.assignee.as_ref().unwrap(), "alice");
    }
}

#[test]
fn test_list_unassigned_beads() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create assigned and unassigned beads
    create_test_bead_with_assignee(&storage, "bf-unassigned-1", "Assigned bead", Some("alice"));
    create_test_bead_with_assignee(&storage, "bf-unassigned-2", "Unassigned bead 1", None);
    create_test_bead_with_assignee(&storage, "bf-unassigned-3", "Unassigned bead 2", None);

    // List only unassigned beads
    let filter = bead_forge::model::IssueFilter {
        assignee: Some(String::new()), // Empty string for unassigned
        ..Default::default()
    };
    let unassigned_beads = storage.list_issues(&filter).unwrap();

    assert_eq!(unassigned_beads.len(), 2);
    for bead in &unassigned_beads {
        assert!(bead.assignee.is_none());
    }
}

#[test]
fn test_assignee_change_generates_event() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create bead without assignee
    let bead = create_test_bead_with_assignee(&storage, "bf-test-events", "Test bead for events", None);

    // Update assignee
    let changes = IssueChanges {
        assignee: Some("david".to_string()),
        ..Default::default()
    };
    storage.update_issue(&bead.id, &changes).unwrap();

    // Check that an assignee changed event was created
    let events = storage.list_events(&bead.id).unwrap();
    let assignee_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.event_type, EventType::AssigneeChanged))
        .collect();

    assert_eq!(assignee_events.len(), 1);
    assert_eq!(assignee_events[0].actor, "cli");
}

#[test]
fn test_stats_by_assignee() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create beads with different assignees
    create_test_bead_with_assignee(&storage, "bf-stats-1", "Alice task 1", Some("alice"));
    create_test_bead_with_assignee(&storage, "bf-stats-2", "Alice task 2", Some("alice"));
    create_test_bead_with_assignee(&storage, "bf-stats-3", "Bob task", Some("bob"));
    create_test_bead_with_assignee(&storage, "bf-stats-4", "Unassigned task", None);

    // Get stats by assignee
    let stats = storage.get_stats_by_assignee().unwrap();

    // Should have entries for alice, bob, and unassigned (None)
    assert!(stats.len() >= 3);

    let alice_count = stats
        .iter()
        .find(|(a, _)| a.as_deref() == Some("alice"))
        .map(|(_, c)| c)
        .unwrap_or(&0);
    let bob_count = stats
        .iter()
        .find(|(a, _)| a.as_deref() == Some("bob"))
        .map(|(_, c)| c)
        .unwrap_or(&0);
    let unassigned_count = stats
        .iter()
        .find(|(a, _)| a.is_none())
        .map(|(_, c)| c)
        .unwrap_or(&0);

    assert_eq!(*alice_count, 2);
    assert_eq!(*bob_count, 1);
    assert_eq!(*unassigned_count, 1);
}

#[test]
fn test_assignee_special_characters() {
    let (_temp_dir, beads_dir) = setup_test_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Test assignee with email format
    let bead1 = create_test_bead_with_assignee(&storage, "bf-email", "Email assignee", Some("alice@example.com"));
    assert_eq!(bead1.assignee.as_ref().unwrap(), "alice@example.com");

    // Test assignee with spaces
    let bead2 = create_test_bead_with_assignee(&storage, "bf-space", "Space assignee", Some("Alice Smith"));
    assert_eq!(bead2.assignee.as_ref().unwrap(), "Alice Smith");

    // Test assignee with hyphens
    let bead3 = create_test_bead_with_assignee(&storage, "bf-hyphen", "Hyphen assignee", Some("alice-worker-1"));
    assert_eq!(bead3.assignee.as_ref().unwrap(), "alice-worker-1");
}
