//! Test label import from JSONL checkpoint
//!
//! This test verifies that labels are properly imported from JSONL during
//! `bf sync --import` or database rebuild operations.

use bead_forge::sync;
use bead_forge::model::{Issue, Priority, Status, IssueType};
use bead_forge::storage::Storage;
use chrono::Utc;
use std::fs;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_label_import_from_jsonl() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    // Initialize workspace
    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    let db_path = beads_dir.join("beads.db");
    let jsonl_path = beads_dir.join("issues.jsonl");

    // Create a JSONL file with labels
    let issue_with_labels = Issue {
        id: "bf-test-labels".to_string(),
        title: "Test Label Import".to_string(),
        description: Some("Testing label import from JSONL".to_string()),
        status: Status::Open,
        priority: Priority::MEDIUM,
        issue_type: IssueType::Task,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        source_repo: Some(".".to_string()),
        labels: vec!["phase-1".to_string(), "storage".to_string(), "critical".to_string()],
        ..Default::default()
    };

    // Write to JSONL
    {
        let mut file = fs::File::create(&jsonl_path).unwrap();
        writeln!(file, "{}", serde_json::to_string(&issue_with_labels).unwrap()).unwrap();
    }

    // Import from JSONL
    let result = sync::import(workspace).unwrap();

    assert_eq!(result.imported, 1, "Should import 1 new issue");

    // Verify the issue was imported with labels
    let storage = Storage::open(&db_path).unwrap();
    let imported = storage.get_issue("bf-test-labels").unwrap().unwrap();

    assert_eq!(imported.id, "bf-test-labels");
    assert_eq!(imported.title, "Test Label Import");
    assert_eq!(
        imported.labels,
        vec!["phase-1", "storage", "critical"],
        "Labels should be imported from JSONL"
    );
}

#[test]
fn test_label_import_with_empty_labels() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    let db_path = beads_dir.join("beads.db");
    let jsonl_path = beads_dir.join("issues.jsonl");

    // Create a JSONL file without labels field (should default to empty)
    let issue_json = r#"{
        "id": "bf-no-labels",
        "title": "Test No Labels",
        "status": "open",
        "priority": 2,
        "issue_type": "task",
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
        "source_repo": "."
    }"#;

    {
        let mut file = fs::File::create(&jsonl_path).unwrap();
        writeln!(file, "{}", issue_json).unwrap();
    }

    // Import from JSONL
    let result = sync::import(workspace).unwrap();

    assert_eq!(result.imported, 1);

    // Verify the issue was imported with no labels
    let storage = Storage::open(&db_path).unwrap();
    let imported = storage.get_issue("bf-no-labels").unwrap().unwrap();

    assert_eq!(imported.labels, vec![], "Issue should have no labels");
}

#[test]
fn test_label_import_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    let db_path = beads_dir.join("beads.db");

    // Create an issue with labels directly in the database
    let storage = Storage::open(&db_path).unwrap();
    let original_issue = Issue {
        id: "bf-roundtrip".to_string(),
        title: "Label Roundtrip Test".to_string(),
        status: Status::Open,
        priority: Priority::HIGH,
        issue_type: IssueType::Bug,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        source_repo: Some(".".to_string()),
        labels: vec!["bug".to_string(), "backend".to_string(), "urgent".to_string()],
        ..Default::default()
    };

    storage.create_issue(&original_issue).unwrap();

    // Flush to JSONL
    sync::flush(workspace).unwrap();

    // Clear and rebuild database
    drop(storage);
    std::fs::remove_file(&db_path).unwrap();

    // Import from JSONL
    let result = sync::import(workspace).unwrap();

    assert_eq!(result.imported, 1);

    // Verify labels survived the roundtrip
    let storage2 = Storage::open(&db_path).unwrap();
    let imported = storage2.get_issue("bf-roundtrip").unwrap().unwrap();

    assert_eq!(
        imported.labels,
        vec!["bug", "backend", "urgent"],
        "Labels should survive export/import roundtrip"
    );
}

#[test]
fn test_label_import_idempotent() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    let db_path = beads_dir.join("beads.db");
    let jsonl_path = beads_dir.join("issues.jsonl");

    // Create a JSONL file with labels
    let issue = Issue {
        id: "bf-idempotent".to_string(),
        title: "Test Idempotent Import".to_string(),
        status: Status::Open,
        priority: Priority::MEDIUM,
        issue_type: IssueType::Task,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        source_repo: Some(".".to_string()),
        labels: vec!["test".to_string()],
        ..Default::default()
    };

    {
        let mut file = fs::File::create(&jsonl_path).unwrap();
        writeln!(file, "{}", serde_json::to_string(&issue).unwrap()).unwrap();
    }

    // Import twice - should not create duplicates
    sync::import(workspace).unwrap();
    let result2 = sync::import(workspace).unwrap();

    assert_eq!(result2.imported, 0, "Second import should not create new issues");
    assert_eq!(result2.skipped, 1, "Second import should skip existing issue");

    // Verify no duplicate labels
    let storage = Storage::open(&db_path).unwrap();
    let imported = storage.get_issue("bf-idempotent").unwrap().unwrap();

    assert_eq!(imported.labels, vec!["test"], "Should have exactly one label");

    // Check database directly for duplicates
    let conn = storage.conn.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT COUNT(*) FROM bead_labels WHERE bead_id = 'bf-idempotent' AND label = 'test'")
        .unwrap();
    let count: i64 = stmt.query([]).unwrap().next().unwrap().unwrap().get(0).unwrap();
    assert_eq!(count, 1, "Should have exactly one label entry in database");
}

#[test]
fn test_label_import_atomic_transaction() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    let db_path = beads_dir.join("beads.db");
    let jsonl_path = beads_dir.join("issues.jsonl");

    // Create multiple issues with labels
    let issues = vec![
        Issue {
            id: "bf-atomic-1".to_string(),
            title: "Atomic Test 1".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec!["label1".to_string()],
            ..Default::default()
        },
        Issue {
            id: "bf-atomic-2".to_string(),
            title: "Atomic Test 2".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec!["label2".to_string()],
            ..Default::default()
        },
    ];

    {
        let mut file = fs::File::create(&jsonl_path).unwrap();
        for issue in &issues {
            writeln!(file, "{}", serde_json::to_string(issue).unwrap()).unwrap();
        }
    }

    // Import all issues in a single transaction
    let result = sync::import(workspace).unwrap();

    assert_eq!(result.imported, 2, "Should import 2 issues atomically");

    // Verify both issues and their labels are present
    let storage = Storage::open(&db_path).unwrap();

    let issue1 = storage.get_issue("bf-atomic-1").unwrap().unwrap();
    assert_eq!(issue1.labels, vec!["label1"]);

    let issue2 = storage.get_issue("bf-atomic-2").unwrap().unwrap();
    assert_eq!(issue2.labels, vec!["label2"]);
}
