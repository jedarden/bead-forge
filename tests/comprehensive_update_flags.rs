//! Comprehensive tests for ALL `bf update` command flags
//!
//! This test file validates that every single update flag works correctly:
//! --title, --status, --priority, --assignee, --description, --acceptance-criteria,
//! --notes, --design, --due-at

use bead_forge::config::load_config;
use bead_forge::model::{Issue, IssueChanges, Priority, IssueType, Status};
use bead_forge::storage::Storage;
use chrono::{DateTime, Utc};
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

// ==================== TITLE FLAG TESTS ====================

#[test]
fn test_update_title_flag() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-title".to_string(), "Original Title".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update title
    let changes = IssueChanges {
        title: Some("Updated Title".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-title", &changes).unwrap();

    let updated = storage.get_issue("bf-test-title").unwrap().unwrap();
    assert_eq!(updated.title, "Updated Title");
}

#[test]
fn test_update_title_with_special_characters() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-special".to_string(), "Simple Title".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update title with special characters
    let special_title = "Title with émojis 🎉 and spëcial çharacters! @#$%";
    let changes = IssueChanges {
        title: Some(special_title.to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-special", &changes).unwrap();

    let updated = storage.get_issue("bf-test-special").unwrap().unwrap();
    assert_eq!(updated.title, special_title);
}

#[test]
fn test_update_title_very_long() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-long".to_string(), "Short".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update with very long title (exactly 500 characters to test constraint boundary)
    let base = "A very long title that tests the boundary of the 500 character limit constraint in the database schema. ";
    let long_title = format!("{}({} chars)", base.repeat(8), "x".repeat(400));
    // Truncate to exactly 500 to avoid CHECK constraint failure
    let long_title = if long_title.len() > 500 { &long_title[..500] } else { &long_title };

    let changes = IssueChanges {
        title: Some(long_title.to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-long", &changes).unwrap();

    let updated = storage.get_issue("bf-test-long").unwrap().unwrap();
    assert_eq!(updated.title, long_title);
    assert_eq!(updated.title.len(), 500);
}

// ==================== STATUS FLAG TESTS ====================

#[test]
fn test_update_status_flag_to_open() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let mut issue = Issue::new("bf-test-status-open".to_string(), "Test".to_string(), ".".to_string());
    issue.status = Status::Blocked;
    storage.create_issue(&issue).unwrap();

    // Update status to Open
    let changes = IssueChanges {
        status: Some(Status::Open),
        ..Default::default()
    };
    storage.update_issue("bf-test-status-open", &changes).unwrap();

    let updated = storage.get_issue("bf-test-status-open").unwrap().unwrap();
    assert_eq!(updated.status, Status::Open);
}

#[test]
fn test_update_status_flag_to_in_progress() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-status-progress".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update status to InProgress
    let changes = IssueChanges {
        status: Some(Status::InProgress),
        ..Default::default()
    };
    storage.update_issue("bf-test-status-progress", &changes).unwrap();

    let updated = storage.get_issue("bf-test-status-progress").unwrap().unwrap();
    assert_eq!(updated.status, Status::InProgress);
}

#[test]
fn test_update_status_flag_to_blocked() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-status-blocked".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update status to Blocked
    let changes = IssueChanges {
        status: Some(Status::Blocked),
        ..Default::default()
    };
    storage.update_issue("bf-test-status-blocked", &changes).unwrap();

    let updated = storage.get_issue("bf-test-status-blocked").unwrap().unwrap();
    assert_eq!(updated.status, Status::Blocked);
}

// Note: Setting status to Closed via update is blocked by CHECK constraint
// (requires closed_at to be set). Use close_issue() method instead.
// This is intentional design - status='closed' requires proper audit trail.

#[test]
fn test_update_status_flag_to_deferred() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-status-deferred".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update status to Deferred
    let changes = IssueChanges {
        status: Some(Status::Deferred),
        ..Default::default()
    };
    storage.update_issue("bf-test-status-deferred", &changes).unwrap();

    let updated = storage.get_issue("bf-test-status-deferred").unwrap().unwrap();
    assert_eq!(updated.status, Status::Deferred);
}

// ==================== PRIORITY FLAG TESTS ====================

#[test]
fn test_update_priority_flag_critical() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-prio-crit".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update priority to Critical (0)
    let changes = IssueChanges {
        priority: Some(0),
        ..Default::default()
    };
    storage.update_issue("bf-test-prio-crit", &changes).unwrap();

    let updated = storage.get_issue("bf-test-prio-crit").unwrap().unwrap();
    assert_eq!(updated.priority, Priority(0));
}

#[test]
fn test_update_priority_flag_high() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-prio-high".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update priority to High (1)
    let changes = IssueChanges {
        priority: Some(1),
        ..Default::default()
    };
    storage.update_issue("bf-test-prio-high", &changes).unwrap();

    let updated = storage.get_issue("bf-test-prio-high").unwrap().unwrap();
    assert_eq!(updated.priority, Priority(1));
}

#[test]
fn test_update_priority_flag_medium() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-prio-med".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update priority to Medium (2)
    let changes = IssueChanges {
        priority: Some(2),
        ..Default::default()
    };
    storage.update_issue("bf-test-prio-med", &changes).unwrap();

    let updated = storage.get_issue("bf-test-prio-med").unwrap().unwrap();
    assert_eq!(updated.priority, Priority(2));
}

#[test]
fn test_update_priority_flag_low() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-prio-low".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update priority to Low (3)
    let changes = IssueChanges {
        priority: Some(3),
        ..Default::default()
    };
    storage.update_issue("bf-test-prio-low", &changes).unwrap();

    let updated = storage.get_issue("bf-test-prio-low").unwrap().unwrap();
    assert_eq!(updated.priority, Priority(3));
}

#[test]
fn test_update_priority_flag_backlog() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-prio-backlog".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update priority to Backlog (4)
    let changes = IssueChanges {
        priority: Some(4),
        ..Default::default()
    };
    storage.update_issue("bf-test-prio-backlog", &changes).unwrap();

    let updated = storage.get_issue("bf-test-prio-backlog").unwrap().unwrap();
    assert_eq!(updated.priority, Priority(4));
}

// ==================== ASSIGNEE FLAG TESTS ====================

#[test]
fn test_update_assignee_flag() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-assignee".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update assignee
    let changes = IssueChanges {
        assignee: Some("worker-1".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-assignee", &changes).unwrap();

    let updated = storage.get_issue("bf-test-assignee").unwrap().unwrap();
    assert_eq!(updated.assignee, Some("worker-1".to_string()));
}

#[test]
fn test_update_assignee_flag_multiple_times() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-reassign".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // First assignment
    let changes1 = IssueChanges {
        assignee: Some("worker-1".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-reassign", &changes1).unwrap();

    let updated = storage.get_issue("bf-test-reassign").unwrap().unwrap();
    assert_eq!(updated.assignee, Some("worker-1".to_string()));

    // Reassignment
    let changes2 = IssueChanges {
        assignee: Some("worker-2".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-reassign", &changes2).unwrap();

    let updated = storage.get_issue("bf-test-reassign").unwrap().unwrap();
    assert_eq!(updated.assignee, Some("worker-2".to_string()));
}

// Note: Clearing assignee behavior - setting to empty string results in empty string stored
#[test]
fn test_update_assignee_flag_clear() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let mut issue = Issue::new("bf-test-clear-assignee".to_string(), "Test".to_string(), ".".to_string());
    issue.assignee = Some("worker-1".to_string());
    storage.create_issue(&issue).unwrap();

    // Clear assignee by setting to empty string
    let changes = IssueChanges {
        assignee: Some("".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-clear-assignee", &changes).unwrap();

    let updated = storage.get_issue("bf-test-clear-assignee").unwrap().unwrap();
    // Empty string is stored as-is (assignee is TEXT, not NOT NULL with DEFAULT '')
    assert_eq!(updated.assignee, Some("".to_string()));
}

// ==================== COMBINED UPDATE TESTS ====================

#[test]
fn test_update_all_flags_together() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-all-flags".to_string(), "Original Title".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update ALL flags at once
    let due_at: DateTime<Utc> = "2025-12-31T23:59:59Z".parse().unwrap();
    let changes = IssueChanges {
        title: Some("Completely Updated Title".to_string()),
        status: Some(Status::InProgress),
        priority: Some(1),
        assignee: Some("super-worker".to_string()),
        description: Some("Updated description".to_string()),
        acceptance_criteria: Some("Updated AC".to_string()),
        notes: Some("Updated notes".to_string()),
        design: Some("Updated design".to_string()),
        due_at: Some(due_at),
        ..Default::default()
    };
    storage.update_issue("bf-test-all-flags", &changes).unwrap();

    let updated = storage.get_issue("bf-test-all-flags").unwrap().unwrap();
    assert_eq!(updated.title, "Completely Updated Title");
    assert_eq!(updated.status, Status::InProgress);
    assert_eq!(updated.priority, Priority(1));
    assert_eq!(updated.assignee, Some("super-worker".to_string()));
    assert_eq!(updated.description, Some("Updated description".to_string()));
    assert_eq!(updated.acceptance_criteria, Some("Updated AC".to_string()));
    assert_eq!(updated.notes, Some("Updated notes".to_string()));
    assert_eq!(updated.design, Some("Updated design".to_string()));
    assert!(updated.due_at.is_some());
}

#[test]
fn test_update_preserves_unspecified_fields() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    // Create with all fields set
    let due_at: DateTime<Utc> = "2025-01-01T00:00:00Z".parse().unwrap();
    let mut issue = Issue::new("bf-test-preserve".to_string(), "Title".to_string(), ".".to_string());
    issue.status = Status::InProgress;
    issue.priority = Priority(1);
    issue.assignee = Some("worker".to_string());
    issue.description = Some("Description".to_string());
    issue.acceptance_criteria = Some("AC".to_string());
    issue.notes = Some("Notes".to_string());
    issue.design = Some("Design".to_string());
    issue.due_at = Some(due_at);
    storage.create_issue(&issue).unwrap();

    // Update only title
    let changes = IssueChanges {
        title: Some("New Title Only".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-preserve", &changes).unwrap();

    let updated = storage.get_issue("bf-test-preserve").unwrap().unwrap();
    assert_eq!(updated.title, "New Title Only");
    // All other fields should be preserved
    assert_eq!(updated.status, Status::InProgress);
    assert_eq!(updated.priority, Priority(1));
    assert_eq!(updated.assignee, Some("worker".to_string()));
    assert_eq!(updated.description, Some("Description".to_string()));
    assert_eq!(updated.acceptance_criteria, Some("AC".to_string()));
    assert_eq!(updated.notes, Some("Notes".to_string()));
    assert_eq!(updated.design, Some("Design".to_string()));
    assert!(updated.due_at.is_some());
}

#[test]
fn test_update_status_priority_combination() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-combo".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update status to InProgress and priority together
    let changes = IssueChanges {
        status: Some(Status::InProgress),
        priority: Some(0),
        ..Default::default()
    };
    storage.update_issue("bf-test-combo", &changes).unwrap();

    let updated = storage.get_issue("bf-test-combo").unwrap().unwrap();
    assert_eq!(updated.status, Status::InProgress);
    assert_eq!(updated.priority, Priority(0));
}

#[test]
fn test_update_title_assignee_combination() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-title-assign".to_string(), "Old Title".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update title and assignee together
    let changes = IssueChanges {
        title: Some("New Title with Assignee".to_string()),
        assignee: Some("new-worker".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-title-assign", &changes).unwrap();

    let updated = storage.get_issue("bf-test-title-assign").unwrap().unwrap();
    assert_eq!(updated.title, "New Title with Assignee");
    assert_eq!(updated.assignee, Some("new-worker".to_string()));
}