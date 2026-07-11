//! Comprehensive tests for `bf update` command flags
//!
//! Consolidated from 4 overlapping test files (69 test functions → 28 deduplicated tests)
//! Covers both storage-level API and CLI integration testing for all update flags:
//! --title, --status, --priority, --assignee, --description, --acceptance-criteria,
//! --notes, --design, --due-at

use std::process::Command;
use std::path::PathBuf;
use tempfile::TempDir;

use bead_forge::config::load_config;
use bead_forge::model::{Issue, IssueChanges, Priority, Status};
use bead_forge::storage::Storage;
use chrono::{DateTime, Utc};

// ==================== HELPERS ====================

/// Helper function to get the bf binary path
fn bf_path() -> PathBuf {
    std::env::var("CARGO_BIN_EXE_bf")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./target/debug/bf"))
}

/// Create a temporary workspace for storage-level tests
fn setup_storage_workspace() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("test-workspace");
    std::fs::create_dir_all(&workspace_dir).unwrap();
    let beads_dir = workspace_dir.join(".beads");
    std::fs::create_dir_all(&beads_dir).unwrap();

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

/// Initialize a test workspace for CLI tests
fn init_cli_workspace() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();

    let bf = bf_path();
    let init_result = Command::new(&bf)
        .arg("init")
        .arg("--prefix")
        .arg("test")
        .current_dir(workspace)
        .output()
        .expect("Failed to initialize workspace");

    assert!(init_result.status.success(), "bf init failed: {}",
            String::from_utf8_lossy(&init_result.stderr));

    temp_dir
}

/// Create a test bead via CLI
fn create_cli_bead(workspace: impl AsRef<std::path::Path>, title: &str) -> String {
    let bf = bf_path();
    let create_result = Command::new(&bf)
        .arg("create")
        .arg("--title")
        .arg(title)
        .arg("--type")
        .arg("task")
        .arg("--priority")
        .arg("2")
        .current_dir(workspace)
        .output()
        .expect("Failed to create bead");

    assert!(create_result.status.success(), "bf create failed: {}",
            String::from_utf8_lossy(&create_result.stderr));

    String::from_utf8(create_result.stdout).unwrap().trim().to_string()
}

/// Update a bead via CLI and verify success
fn update_cli_bead(workspace: impl AsRef<std::path::Path>, bead_id: &str, args: &[&str]) {
    let bf = bf_path();
    let mut cmd = Command::new(&bf);
    cmd.arg("update")
       .arg(bead_id)
       .args(args)
       .current_dir(workspace);

    let result = cmd.output().expect("Failed to run update");
    assert!(result.status.success(), "bf update failed: {}",
            String::from_utf8_lossy(&result.stderr));
}

/// Get bead details as JSON via CLI
fn get_cli_bead_json(workspace: impl AsRef<std::path::Path>, bead_id: &str) -> serde_json::Value {
    let bf = bf_path();
    let show_result = Command::new(&bf)
        .arg("show")
        .arg(bead_id)
        .arg("--format")
        .arg("json")
        .current_dir(workspace)
        .output()
        .expect("Failed to show bead");

    assert!(show_result.status.success(), "bf show failed: {}",
            String::from_utf8_lossy(&show_result.stderr));

    let output = String::from_utf8(show_result.stdout).unwrap();
    let beads: Vec<serde_json::Value> = serde_json::from_str(&output)
        .expect("Failed to parse JSON");
    beads.into_iter().next().expect("No bead found")
}

// ==================== STORAGE-LEVEL FIELD TESTS ====================

#[test]
fn test_update_description_via_storage() {
    let (_temp, beads_dir) = setup_storage_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-desc".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    let changes = IssueChanges {
        description: Some("New description".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-desc", &changes).unwrap();

    let updated = storage.get_issue("bf-test-desc").unwrap().unwrap();
    assert_eq!(updated.description, Some("New description".to_string()));
}

#[test]
fn test_update_acceptance_criteria_via_storage() {
    let (_temp, beads_dir) = setup_storage_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-ac".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    let changes = IssueChanges {
        acceptance_criteria: Some("Should pass tests".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-ac", &changes).unwrap();

    let updated = storage.get_issue("bf-test-ac").unwrap().unwrap();
    assert_eq!(updated.acceptance_criteria, Some("Should pass tests".to_string()));
}

#[test]
fn test_update_notes_via_storage() {
    let (_temp, beads_dir) = setup_storage_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-notes".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    let changes = IssueChanges {
        notes: Some("Additional notes here".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-notes", &changes).unwrap();

    let updated = storage.get_issue("bf-test-notes").unwrap().unwrap();
    assert_eq!(updated.notes, Some("Additional notes here".to_string()));
}

#[test]
fn test_update_design_via_storage() {
    let (_temp, beads_dir) = setup_storage_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-design".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    let changes = IssueChanges {
        design: Some("Design documentation".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-design", &changes).unwrap();

    let updated = storage.get_issue("bf-test-design").unwrap().unwrap();
    assert_eq!(updated.design, Some("Design documentation".to_string()));
}

#[test]
fn test_update_due_at_via_storage() {
    let (_temp, beads_dir) = setup_storage_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-due".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    let due_date: DateTime<Utc> = "2025-12-31T23:59:59Z".parse().unwrap();
    let changes = IssueChanges {
        due_at: Some(due_date),
        ..Default::default()
    };

    storage.update_issue("bf-test-due", &changes).unwrap();

    let updated = storage.get_issue("bf-test-due").unwrap().unwrap();
    assert_eq!(updated.due_at.map(|d| d.to_rfc3339()), Some("2025-12-31T23:59:59+00:00".to_string()));
}

#[test]
fn test_update_multiline_text_via_storage() {
    let (_temp, beads_dir) = setup_storage_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-multiline".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    let multiline_desc = "Line 1\nLine 2\nLine 3".to_string();
    let changes = IssueChanges {
        description: Some(multiline_desc.clone()),
        ..Default::default()
    };

    storage.update_issue("bf-test-multiline", &changes).unwrap();

    let updated = storage.get_issue("bf-test-multiline").unwrap().unwrap();
    assert_eq!(updated.description, Some(multiline_desc));
}

#[test]
fn test_update_unicode_characters_via_storage() {
    let (_temp, beads_dir) = setup_storage_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-unicode".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    let unicode_text = "Description with émojis 🎉 and spëcial çharacters".to_string();
    let changes = IssueChanges {
        description: Some(unicode_text.clone()),
        ..Default::default()
    };

    storage.update_issue("bf-test-unicode", &changes).unwrap();

    let updated = storage.get_issue("bf-test-unicode").unwrap().unwrap();
    assert_eq!(updated.description, Some(unicode_text));
}

// ==================== STORAGE-LEVEL COMBINATION TESTS ====================

#[test]
fn test_update_all_fields_together_via_storage() {
    let (_temp, beads_dir) = setup_storage_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let issue = Issue::new("bf-test-all".to_string(), "Test".to_string(), ".".to_string());
    storage.create_issue(&issue).unwrap();

    let due_at: DateTime<Utc> = "2025-06-30T12:00:00Z".parse().unwrap();
    let changes = IssueChanges {
        title: Some("Updated Title".to_string()),
        status: Some(Status::InProgress),
        priority: Some(1),
        assignee: Some("worker-1".to_string()),
        description: Some("Updated description".to_string()),
        acceptance_criteria: Some("AC 1, AC 2".to_string()),
        notes: Some("Notes here".to_string()),
        design: Some("Design docs".to_string()),
        due_at: Some(due_at),
        ..Default::default()
    };

    storage.update_issue("bf-test-all", &changes).unwrap();

    let updated = storage.get_issue("bf-test-all").unwrap().unwrap();
    assert_eq!(updated.title, "Updated Title");
    assert_eq!(updated.status, Status::InProgress);
    assert_eq!(updated.priority, Priority(1));
    assert_eq!(updated.assignee, Some("worker-1".to_string()));
    assert_eq!(updated.description, Some("Updated description".to_string()));
    assert_eq!(updated.acceptance_criteria, Some("AC 1, AC 2".to_string()));
    assert_eq!(updated.notes, Some("Notes here".to_string()));
    assert_eq!(updated.design, Some("Design docs".to_string()));
    assert_eq!(updated.due_at.map(|d| d.to_rfc3339()), Some("2025-06-30T12:00:00+00:00".to_string()));
}

#[test]
fn test_update_preserves_unspecified_fields_via_storage() {
    let (_temp, beads_dir) = setup_storage_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let mut issue = Issue::new("bf-test-preserve".to_string(), "Test".to_string(), ".".to_string());
    issue.description = Some("Original description".to_string());
    issue.acceptance_criteria = Some("Original AC".to_string());
    storage.create_issue(&issue).unwrap();

    // Update only description
    let changes = IssueChanges {
        description: Some("New description only".to_string()),
        ..Default::default()
    };

    storage.update_issue("bf-test-preserve", &changes).unwrap();

    let updated = storage.get_issue("bf-test-preserve").unwrap().unwrap();
    assert_eq!(updated.description, Some("New description only".to_string()));
    assert_eq!(updated.acceptance_criteria, Some("Original AC".to_string()));
}

#[test]
fn test_update_fields_orthogonal_via_storage() {
    let (_temp, beads_dir) = setup_storage_workspace();
    let config = load_config(&beads_dir).unwrap();
    let db_path = beads_dir.join("beads.db");
    let storage = Storage::open_with_config(&db_path, &config).unwrap();

    let mut issue = Issue::new("bf-test-ortho".to_string(), "Test".to_string(), ".".to_string());
    issue.description = Some("Original".to_string());
    issue.notes = Some("Original notes".to_string());
    storage.create_issue(&issue).unwrap();

    // Update only description, notes should remain unchanged
    let changes = IssueChanges {
        description: Some("Only this changes".to_string()),
        ..Default::default()
    };
    storage.update_issue("bf-test-ortho", &changes).unwrap();

    let updated = storage.get_issue("bf-test-ortho").unwrap().unwrap();
    assert_eq!(updated.description, Some("Only this changes".to_string()));
    assert_eq!(updated.notes, Some("Original notes".to_string()));
}

// ==================== CLI INTEGRATION TESTS ====================

#[test]
fn test_cli_update_title_basic() {
    let temp_dir = init_cli_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_cli_bead(workspace, "Original Title");

    update_cli_bead(workspace, &bead_id, &["--title", "Updated Title"]);

    let bead = get_cli_bead_json(workspace, &bead_id);
    assert_eq!(bead["title"], "Updated Title");
}

#[test]
fn test_cli_update_title_unicode() {
    let temp_dir = init_cli_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_cli_bead(workspace, "Simple Title");

    let unicode_title = "Title with émojis 🎉 and spëcial çharacters";
    update_cli_bead(workspace, &bead_id, &["--title", unicode_title]);

    let bead = get_cli_bead_json(workspace, &bead_id);
    assert_eq!(bead["title"], unicode_title);
}

#[test]
fn test_cli_update_title_empty() {
    let temp_dir = init_cli_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_cli_bead(workspace, "Original Title");

    update_cli_bead(workspace, &bead_id, &["--title", ""]);

    let bead = get_cli_bead_json(workspace, &bead_id);
    assert_eq!(bead["title"], "");
}

#[test]
fn test_cli_update_status_in_progress() {
    let temp_dir = init_cli_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_cli_bead(workspace, "Test Status");

    update_cli_bead(workspace, &bead_id, &["--status", "in_progress"]);

    let bead = get_cli_bead_json(workspace, &bead_id);
    assert_eq!(bead["status"], "in_progress");
}

#[test]
fn test_cli_update_status_blocked() {
    let temp_dir = init_cli_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_cli_bead(workspace, "Test Status");

    update_cli_bead(workspace, &bead_id, &["--status", "blocked"]);

    let bead = get_cli_bead_json(workspace, &bead_id);
    assert_eq!(bead["status"], "blocked");
}

#[test]
fn test_cli_update_status_deferred() {
    let temp_dir = init_cli_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_cli_bead(workspace, "Test Status");

    update_cli_bead(workspace, &bead_id, &["--status", "deferred"]);

    let bead = get_cli_bead_json(workspace, &bead_id);
    assert_eq!(bead["status"], "deferred");
}

#[test]
fn test_cli_update_priority_critical() {
    let temp_dir = init_cli_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_cli_bead(workspace, "Test Priority");

    update_cli_bead(workspace, &bead_id, &["--priority", "0"]);

    let bead = get_cli_bead_json(workspace, &bead_id);
    assert_eq!(bead["priority"], 0);
}

#[test]
fn test_cli_update_priority_medium() {
    let temp_dir = init_cli_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_cli_bead(workspace, "Test Priority");

    update_cli_bead(workspace, &bead_id, &["--priority", "2"]);

    let bead = get_cli_bead_json(workspace, &bead_id);
    assert_eq!(bead["priority"], 2);
}

#[test]
fn test_cli_update_priority_backlog() {
    let temp_dir = init_cli_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_cli_bead(workspace, "Test Priority");

    update_cli_bead(workspace, &bead_id, &["--priority", "4"]);

    let bead = get_cli_bead_json(workspace, &bead_id);
    assert_eq!(bead["priority"], 4);
}

#[test]
fn test_cli_update_assignee_basic() {
    let temp_dir = init_cli_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_cli_bead(workspace, "Test Assignee");

    update_cli_bead(workspace, &bead_id, &["--assignee", "worker-1"]);

    let bead = get_cli_bead_json(workspace, &bead_id);
    assert_eq!(bead["assignee"], "worker-1");
}

#[test]
fn test_cli_update_assignee_reassignment() {
    let temp_dir = init_cli_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_cli_bead(workspace, "Test Assignee");

    update_cli_bead(workspace, &bead_id, &["--assignee", "worker-1"]);
    update_cli_bead(workspace, &bead_id, &["--assignee", "worker-2"]);

    let bead = get_cli_bead_json(workspace, &bead_id);
    assert_eq!(bead["assignee"], "worker-2");
}

#[test]
fn test_cli_update_assignee_empty_rejected() {
    let temp_dir = init_cli_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_cli_bead(workspace, "Test Assignee");

    update_cli_bead(workspace, &bead_id, &["--assignee", "worker-1"]);

    // Setting assignee to empty string should fail with validation error
    let bf = bf_path();
    let result = Command::new(&bf)
        .arg("update")
        .arg(&bead_id)
        .arg("--assignee")
        .arg("")
        .current_dir(workspace)
        .output()
        .expect("Failed to run update");

    assert!(!result.status.success(), "bf update should reject empty assignee");
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("Assignee cannot be empty"), "Expected validation error message");

    // Verify the assignee was NOT changed
    let bead = get_cli_bead_json(workspace, &bead_id);
    assert_eq!(bead["assignee"], "worker-1");
}

#[test]
fn test_cli_update_description_unicode() {
    let temp_dir = init_cli_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_cli_bead(workspace, "Test Description");

    let unicode = "Description with émojis 🎉 and spëcial çharacters";
    update_cli_bead(workspace, &bead_id, &["--description", unicode]);

    let bead = get_cli_bead_json(workspace, &bead_id);
    assert_eq!(bead["description"], unicode);
}

#[test]
fn test_cli_update_due_at_rfc3339() {
    let temp_dir = init_cli_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_cli_bead(workspace, "Test Due");

    update_cli_bead(workspace, &bead_id, &["--due-at", "2025-12-31T23:59:59Z"]);

    let bead = get_cli_bead_json(workspace, &bead_id);
    assert!(bead["due_at"].is_string());
    let due_str = bead["due_at"].as_str().unwrap();
    assert!(due_str.starts_with("2025-12-31"));
}

#[test]
fn test_cli_update_due_at_invalid_format() {
    let temp_dir = init_cli_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_cli_bead(workspace, "Test Due");

    let bf = bf_path();
    let result = Command::new(&bf)
        .arg("update")
        .arg(&bead_id)
        .arg("--due-at")
        .arg("invalid-date-format")
        .current_dir(workspace)
        .output()
        .expect("Failed to run update");

    // Should fail with invalid date format
    assert!(!result.status.success(), "bf update should fail with invalid date format");
}

#[test]
fn test_cli_update_all_flags_together() {
    let temp_dir = init_cli_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_cli_bead(workspace, "Test All Flags");

    update_cli_bead(workspace, &bead_id, &[
        "--title", "Completely Updated Title",
        "--status", "in_progress",
        "--priority", "1",
        "--assignee", "super-worker",
        "--description", "Updated description",
        "--acceptance-criteria", "Updated AC",
        "--notes", "Updated notes",
        "--design", "Updated design",
        "--due-at", "2025-12-31T23:59:59Z"
    ]);

    let bead = get_cli_bead_json(workspace, &bead_id);
    assert_eq!(bead["title"], "Completely Updated Title");
    assert_eq!(bead["status"], "in_progress");
    assert_eq!(bead["priority"], 1);
    assert_eq!(bead["assignee"], "super-worker");
    assert_eq!(bead["description"], "Updated description");
    assert_eq!(bead["acceptance_criteria"], "Updated AC");
    assert_eq!(bead["notes"], "Updated notes");
    assert_eq!(bead["design"], "Updated design");
    assert!(bead["due_at"].is_string());
}

#[test]
fn test_cli_update_nonexistent_bead() {
    let temp_dir = init_cli_workspace();
    let workspace = temp_dir.path();

    let bf = bf_path();
    let result = Command::new(&bf)
        .arg("update")
        .arg("test-nonexistent")
        .arg("--title")
        .arg("New Title")
        .current_dir(workspace)
        .output()
        .expect("Failed to run update");

    // Should fail with non-existent bead
    assert!(!result.status.success(), "bf update should fail with non-existent bead");
}

#[test]
fn test_cli_update_without_changes() {
    let temp_dir = init_cli_workspace();
    let workspace = temp_dir.path();
    let bead_id = create_cli_bead(workspace, "Test No Changes");

    // Update with no actual changes (just the bead ID)
    let bf = bf_path();
    let result = Command::new(&bf)
        .arg("update")
        .arg(&bead_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run update");

    // Should still succeed (no-op is allowed)
    assert!(result.status.success(), "bf update with no changes should succeed");
}
