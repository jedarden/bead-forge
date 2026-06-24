//! Tests for `bf update` command field flags
//!
//! This test file validates that all field update flags work correctly:
//! --description, --acceptance-criteria, --notes, --design, --due-at

use bead_forge::config::load_config;
use bead_forge::model::{Issue, IssueChanges};
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

#[test]
fn test_update_description_flag() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let mut issue = Issue::new("bf-test-1".to_string(), "Test".to_string(), ".".to_string());
    issue.description = Some("Original description".to_string());
    storage.create_issue(&issue).unwrap();

    // Update description
    let changes = IssueChanges {
        description: Some("Updated description".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-1", &changes).unwrap();

    let updated = storage.get_issue("bf-test-1").unwrap().unwrap();
    assert_eq!(updated.description, Some("Updated description".to_string()));
}

#[test]
fn test_update_acceptance_criteria_flag() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-2".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update acceptance criteria
    let changes = IssueChanges {
        acceptance_criteria: Some("Must pass all tests".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-2", &changes).unwrap();

    let updated = storage.get_issue("bf-test-2").unwrap().unwrap();
    assert_eq!(updated.acceptance_criteria, Some("Must pass all tests".to_string()));
}

#[test]
fn test_update_notes_flag() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-3".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update notes
    let changes = IssueChanges {
        notes: Some("Implementation notes".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-3", &changes).unwrap();

    let updated = storage.get_issue("bf-test-3").unwrap().unwrap();
    assert_eq!(updated.notes, Some("Implementation notes".to_string()));
}

#[test]
fn test_update_design_flag() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-4".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update design
    let changes = IssueChanges {
        design: Some("Technical design approach".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-4", &changes).unwrap();

    let updated = storage.get_issue("bf-test-4").unwrap().unwrap();
    assert_eq!(updated.design, Some("Technical design approach".to_string()));
}

#[test]
fn test_update_due_at_flag() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-5".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update due_at with RFC3339 format
    let due_at = DateTime::parse_from_rfc3339("2025-12-31T23:59:59Z").unwrap();
    let changes = IssueChanges {
        due_at: Some(due_at.with_timezone(&Utc)),
        ..Default::default()
    };
    storage.update_issue("bf-test-5", &changes).unwrap();

    let updated = storage.get_issue("bf-test-5").unwrap().unwrap();
    assert!(updated.due_at.is_some());
    let due_str = updated.due_at.unwrap().to_rfc3339();
    assert!(due_str.starts_with("2025-12-31"));
}

#[test]
fn test_update_multiple_flags_at_once() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-6".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    // Update multiple fields at once
    let due_at = DateTime::parse_from_rfc3339("2025-01-01T00:00:00Z").unwrap();
    let changes = IssueChanges {
        description: Some("New description".to_string()),
        acceptance_criteria: Some("New criteria".to_string()),
        notes: Some("New notes".to_string()),
        design: Some("New design".to_string()),
        due_at: Some(due_at.with_timezone(&Utc)),
        ..Default::default()
    };
    storage.update_issue("bf-test-6", &changes).unwrap();

    let updated = storage.get_issue("bf-test-6").unwrap().unwrap();
    assert_eq!(updated.description, Some("New description".to_string()));
    assert_eq!(updated.acceptance_criteria, Some("New criteria".to_string()));
    assert_eq!(updated.notes, Some("New notes".to_string()));
    assert_eq!(updated.design, Some("New design".to_string()));
    assert!(updated.due_at.is_some());
}

#[test]
fn test_update_flags_orthogonal() {
    let (_temp, beads_dir) = setup_test_workspace();

    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let mut issue = Issue::new("bf-test-8".to_string(), "Test".to_string(), ".".to_string());
    issue.description = Some("Original".to_string());
    issue.notes = Some("Original notes".to_string());
    storage.create_issue(&issue).unwrap();

    // Update only description, notes should remain unchanged
    let changes = IssueChanges {
        description: Some("Only this changes".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-8", &changes).unwrap();

    let updated = storage.get_issue("bf-test-8").unwrap().unwrap();
    assert_eq!(updated.description, Some("Only this changes".to_string()));
    assert_eq!(updated.notes, Some("Original notes".to_string()));
}
