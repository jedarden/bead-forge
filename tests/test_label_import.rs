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

    // Labels are unordered - compare as sets
    assert_eq!(imported.labels.len(), 3, "Should have 3 labels");
    assert!(imported.labels.contains(&"phase-1".to_string()), "Should contain phase-1 label");
    assert!(imported.labels.contains(&"storage".to_string()), "Should contain storage label");
    assert!(imported.labels.contains(&"critical".to_string()), "Should contain critical label");
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
    // JSONL requires each record on a single line (no newlines within the JSON)
    let issue_json = r#"{"id":"bf-no-labels","title":"Test No Labels","status":"open","priority":2,"issue_type":"task","created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z","source_repo":"."}"#;

    {
        let mut file = fs::File::create(&jsonl_path).unwrap();
        writeln!(file, "{}", issue_json).unwrap();
    } // File is closed here

    // Verify file exists and has content
    assert!(jsonl_path.exists(), "JSONL file should exist");
    let content = fs::read_to_string(&jsonl_path).unwrap();
    assert!(!content.is_empty(), "JSONL file should not be empty");

    // Import from JSONL
    let result = sync::import(workspace).unwrap();

    assert_eq!(result.imported, 1);

    // Verify the issue was imported with no labels
    let storage = Storage::open(&db_path).unwrap();
    let imported = storage.get_issue("bf-no-labels").unwrap().unwrap();

    assert_eq!(imported.labels, Vec::<String>::new(), "Issue should have no labels");
}

#[test]
fn test_label_import_null_field_rejected() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    let jsonl_path = beads_dir.join("issues.jsonl");

    // Create a JSONL file with explicit null labels field
    // This should be rejected during JSON deserialization
    let issue_json = r#"{"id":"bf-null-labels","title":"Test Null Labels","status":"open","priority":2,"issue_type":"task","created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z","source_repo":".","labels":null}"#;

    {
        let mut file = fs::File::create(&jsonl_path).unwrap();
        writeln!(file, "{}", issue_json).unwrap();
    }

    // Verify file exists and has content
    assert!(jsonl_path.exists(), "JSONL file should exist");
    let content = fs::read_to_string(&jsonl_path).unwrap();
    assert!(!content.is_empty(), "JSONL file should not be empty");

    // Import from JSONL should fail because null is not a valid value for labels array
    let result = sync::import(workspace);

    assert!(result.is_err(), "Import should fail when labels field is null");
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("invalid type: null") || err_msg.contains("expected a sequence"),
        "Error should indicate null value was rejected: {}",
        err_msg
    );
}

#[test]
fn test_label_import_empty_array() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    let db_path = beads_dir.join("beads.db");
    let jsonl_path = beads_dir.join("issues.jsonl");

    // Create a JSONL file with explicit empty labels array
    let issue_json = r#"{"id":"bf-empty-array","title":"Test Empty Array","status":"open","priority":2,"issue_type":"task","created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z","source_repo":".","labels":[]}"#;

    {
        let mut file = fs::File::create(&jsonl_path).unwrap();
        writeln!(file, "{}", issue_json).unwrap();
    }

    // Verify file exists and has content
    assert!(jsonl_path.exists(), "JSONL file should exist");
    let content = fs::read_to_string(&jsonl_path).unwrap();
    assert!(!content.is_empty(), "JSONL file should not be empty");

    // Import from JSONL
    let result = sync::import(workspace).unwrap();

    assert_eq!(result.imported, 1, "Should import 1 issue with empty labels array");

    // Verify the issue was imported with no labels
    let storage = Storage::open(&db_path).unwrap();
    let imported = storage.get_issue("bf-empty-array").unwrap().unwrap();

    assert_eq!(imported.labels, Vec::<String>::new(), "Issue with empty labels array should have no labels");
}

#[test]
fn test_label_import_mixed_empty_scenarios() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    let db_path = beads_dir.join("beads.db");
    let jsonl_path = beads_dir.join("issues.jsonl");

    // Create JSONL with multiple empty label scenarios (excluding null which is rejected)
    let json_lines = vec![
        r#"{"id":"bf-mixed-missing","title":"Missing Labels","status":"open","priority":2,"issue_type":"task","created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z","source_repo":"."}"#,
        r#"{"id":"bf-mixed-empty","title":"Empty Array Labels","status":"open","priority":2,"issue_type":"task","created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z","source_repo":".","labels":[]}"#,
        r#"{"id":"bf-mixed-valid","title":"Valid Labels","status":"open","priority":2,"issue_type":"task","created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z","source_repo":".","labels":["phase-1"]}"#,
    ];

    {
        let mut file = fs::File::create(&jsonl_path).unwrap();
        for line in json_lines {
            writeln!(file, "{}", line).unwrap();
        }
    }

    // Import from JSONL
    let result = sync::import(workspace).unwrap();

    assert_eq!(result.imported, 3, "Should import all 3 issues");

    // Verify all issues were imported correctly
    let storage = Storage::open(&db_path).unwrap();

    let missing = storage.get_issue("bf-mixed-missing").unwrap().unwrap();
    assert_eq!(missing.labels, Vec::<String>::new(), "Missing labels should be empty");

    let empty = storage.get_issue("bf-mixed-empty").unwrap().unwrap();
    assert_eq!(empty.labels, Vec::<String>::new(), "Empty array labels should be empty");

    let valid = storage.get_issue("bf-mixed-valid").unwrap().unwrap();
    assert_eq!(valid.labels, vec!["phase-1"], "Valid labels should be preserved");
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

    // Verify labels survived the roundtrip (labels are unordered)
    let storage2 = Storage::open(&db_path).unwrap();
    let imported = storage2.get_issue("bf-roundtrip").unwrap().unwrap();

    assert_eq!(imported.labels.len(), 3, "Should have 3 labels");
    assert!(imported.labels.contains(&"bug".to_string()), "Should contain 'bug' label");
    assert!(imported.labels.contains(&"backend".to_string()), "Should contain 'backend' label");
    assert!(imported.labels.contains(&"urgent".to_string()), "Should contain 'urgent' label");
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

#[test]
fn test_label_roundtrip_verification_comprehensive() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    let db_path = beads_dir.join("beads.db");
    let jsonl_path = beads_dir.join("issues.jsonl");

    // Create test cases covering various label scenarios
    let test_cases = vec![
        // Empty labels
        Issue {
            id: "bf-empty-labels".to_string(),
            title: "Empty Labels Test".to_string(),
            description: Some("Issue with no labels".to_string()),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec![],
            ..Default::default()
        },
        // Single label
        Issue {
            id: "bf-single-label".to_string(),
            title: "Single Label Test".to_string(),
            description: Some("Issue with one label".to_string()),
            status: Status::Open,
            priority: Priority::HIGH,
            issue_type: IssueType::Bug,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec!["urgent".to_string()],
            ..Default::default()
        },
        // Multiple labels
        Issue {
            id: "bf-multi-labels".to_string(),
            title: "Multiple Labels Test".to_string(),
            description: Some("Issue with multiple labels".to_string()),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec![
                "phase-1".to_string(),
                "storage".to_string(),
                "critical".to_string(),
                "backend".to_string()
            ],
            ..Default::default()
        },
        // Labels with special characters - spaces
        Issue {
            id: "bf-space-label".to_string(),
            title: "Space in Label Test".to_string(),
            description: Some("Label with spaces".to_string()),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec!["needs review".to_string(), "in progress".to_string()],
            ..Default::default()
        },
        // Labels with special characters - unicode
        Issue {
            id: "bf-unicode-label".to_string(),
            title: "Unicode Label Test".to_string(),
            description: Some("Label with unicode characters".to_string()),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec!["bugfix🔧".to_string(), "tést".to_string(), "café".to_string()],
            ..Default::default()
        },
        // Labels with special characters - punctuation
        Issue {
            id: "bf-punct-label".to_string(),
            title: "Punctuation Label Test".to_string(),
            description: Some("Label with punctuation".to_string()),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec!["high-priority".to_string(), "won't-fix".to_string(), "maybe?".to_string()],
            ..Default::default()
        },
        // Labels with numbers
        Issue {
            id: "bf-number-label".to_string(),
            title: "Number Label Test".to_string(),
            description: Some("Label with numbers".to_string()),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec!["p1".to_string(), "v2.0".to_string(), "2024-q4".to_string()],
            ..Default::default()
        },
        // Long label
        Issue {
            id: "bf-long-label".to_string(),
            title: "Long Label Test".to_string(),
            description: Some("Very long label".to_string()),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec!["this-is-a-very-long-label-name-that-might-be-used-in-some-organizations-to-describe-complex-hierarchical-relationships".to_string()],
            ..Default::default()
        },
        // Mixed edge cases
        Issue {
            id: "bf-mixed-labels".to_string(),
            title: "Mixed Edge Cases Test".to_string(),
            description: Some("Mixed edge case labels".to_string()),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec![
                "empty".to_string(),
                "".to_string(),  // Empty string label
                " ".to_string(),  // Space-only label
                "a-b-c".to_string(),
                "x".to_string(),  // Single character
            ],
            ..Default::default()
        },
    ];

    // Step 1: Create all beads with labels in the database
    let storage = Storage::open(&db_path).unwrap();
    for issue in &test_cases {
        storage.create_issue(issue).unwrap();
    }

    // Verify they were created correctly
    let all_issues = storage.list_all_issues().unwrap();
    assert_eq!(all_issues.len(), test_cases.len(), "All test issues should be created");

    // Step 2: Run sync --flush-only (export to JSONL)
    let export_count = sync::flush(workspace).unwrap();
    assert_eq!(export_count, test_cases.len(), "All issues should be exported");

    // Verify JSONL file was created and contains our data
    assert!(jsonl_path.exists(), "JSONL file should exist after flush");
    let jsonl_content = fs::read_to_string(&jsonl_path).unwrap();
    let lines: Vec<&str> = jsonl_content.lines().collect();
    assert_eq!(lines.len(), test_cases.len(), "JSONL should have one line per issue");

    // Verify labels are in the JSONL
    for issue in &test_cases {
        let json_line = lines.iter()
            .find(|line| line.contains(&format!("\"id\":\"{}\"", issue.id)))
            .expect(&format!("Issue {} should be in JSONL", issue.id));

        // Parse and verify labels
        let parsed: serde_json::Value = serde_json::from_str(json_line).unwrap();
        let labels_array = parsed.get("labels").and_then(|v| v.as_array());

        if issue.labels.is_empty() {
            // Empty labels should either be missing or empty array
            assert!(
                labels_array.map_or(true, |arr| arr.is_empty()),
                "Issue {} should have no labels in JSONL",
                issue.id
            );
        } else {
            assert!(
                labels_array.is_some(),
                "Issue {} should have labels array in JSONL",
                issue.id
            );
            let labels = labels_array.unwrap();
            assert_eq!(
                labels.len(),
                issue.labels.len(),
                "Issue {} should have {} labels in JSONL",
                issue.id,
                issue.labels.len()
            );

            for label in &issue.labels {
                assert!(
                    labels.iter().any(|l| l.as_str() == Some(label)),
                    "Label {} should be in JSONL for issue {}",
                    label,
                    issue.id
                );
            }
        }
    }

    // Step 3: Clear database (simulate fresh workspace)
    drop(storage);
    fs::remove_file(&db_path).unwrap();
    assert!(!db_path.exists(), "Database file should be deleted");

    // Step 4: Run sync --import (restore from JSONL)
    let result = sync::import(workspace).unwrap();
    assert_eq!(result.imported, test_cases.len(), "All issues should be imported");

    // Step 5: Verify all labels are restored correctly
    let storage2 = Storage::open(&db_path).unwrap();
    let imported_issues = storage2.list_all_issues().unwrap();
    assert_eq!(
        imported_issues.len(),
        test_cases.len(),
        "All issues should be imported"
    );

    // Verify each issue's labels survived the round-trip
    for original_issue in &test_cases {
        let imported = storage2
            .get_issue(&original_issue.id)
            .unwrap()
            .expect(&format!("Issue {} should be imported", original_issue.id));

        assert_eq!(
            imported.id,
            original_issue.id,
            "Issue ID should match"
        );
        assert_eq!(
            imported.title,
            original_issue.title,
            "Issue title should match for {}",
            original_issue.id
        );
        assert_eq!(
            imported.labels.len(),
            original_issue.labels.len(),
            "Issue {} should have {} labels, got {}",
            original_issue.id,
            original_issue.labels.len(),
            imported.labels.len()
        );

        // Verify each label individually
        for expected_label in &original_issue.labels {
            assert!(
                imported.labels.contains(expected_label),
                "Issue {} should contain label '{}', got: {:?}",
                original_issue.id,
                expected_label,
                imported.labels
            );
        }

        // For deterministic comparison, sort and compare as arrays
        let mut imported_labels_sorted = imported.labels.clone();
        imported_labels_sorted.sort();
        let mut original_labels_sorted = original_issue.labels.clone();
        original_labels_sorted.sort();
        assert_eq!(
            imported_labels_sorted,
            original_labels_sorted,
            "Issue {} labels should match exactly after round-trip",
            original_issue.id
        );
    }

    // Verify the database bead_labels table directly
    let conn = storage2.conn.lock().unwrap();

    // Check total label count
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM bead_labels").unwrap();
    let total_labels: i64 = stmt.query([]).unwrap().next().unwrap().unwrap().get(0).unwrap();

    let expected_total: usize = test_cases.iter()
        .map(|issue| issue.labels.len())
        .sum();

    assert_eq!(
        total_labels as usize,
        expected_total,
        "Total label count should match: expected {}, got {}",
        expected_total,
        total_labels
    );

    // Check each issue's label count in database
    for issue in &test_cases {
        let mut stmt = conn
            .prepare(&format!(
                "SELECT COUNT(*) FROM bead_labels WHERE bead_id = '{}'",
                issue.id
            ))
            .unwrap();
        let count: i64 = stmt.query([]).unwrap().next().unwrap().unwrap().get(0).unwrap();

        assert_eq!(
            count as usize,
            issue.labels.len(),
            "Issue {} should have {} label entries in bead_labels table",
            issue.id,
            issue.labels.len()
        );
    }
}

#[test]
fn test_label_multiple_import_cycles() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    let db_path = beads_dir.join("beads.db");
    let jsonl_path = beads_dir.join("issues.jsonl");

    // Create test issues with various label configurations
    let test_issues = vec![
        Issue {
            id: "bf-cycle-1-empty".to_string(),
            title: "Cycle Test - Empty Labels".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec![],
            ..Default::default()
        },
        Issue {
            id: "bf-cycle-1-single".to_string(),
            title: "Cycle Test - Single Label".to_string(),
            status: Status::Open,
            priority: Priority::HIGH,
            issue_type: IssueType::Bug,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec!["phase-1".to_string()],
            ..Default::default()
        },
        Issue {
            id: "bf-cycle-1-multiple".to_string(),
            title: "Cycle Test - Multiple Labels".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec![
                "phase-1".to_string(),
                "storage".to_string(),
                "critical".to_string(),
                "backend".to_string(),
                "urgent".to_string(),
            ],
            ..Default::default()
        },
        Issue {
            id: "bf-cycle-1-special".to_string(),
            title: "Cycle Test - Special Characters".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec![
                "needs-review".to_string(),
                "bugfix🔧".to_string(),
                "v2.0".to_string(),
                "won't-fix".to_string(),
            ],
            ..Default::default()
        },
    ];

    // Perform 4 complete cycles of export/import
    let num_cycles = 4;

    for cycle in 0..num_cycles {
        // Create or re-create issues in database
        let storage = Storage::open(&db_path).unwrap();

        if cycle == 0 {
            // First cycle: create all issues
            for issue in &test_issues {
                storage.create_issue(issue).unwrap();
            }
        }

        // Verify current state before export
        let all_issues = storage.list_all_issues().unwrap();
        assert_eq!(
            all_issues.len(),
            test_issues.len(),
            "Cycle {}: Should have {} issues before export",
            cycle + 1,
            test_issues.len()
        );

        // Verify labels for each issue
        for original_issue in &test_issues {
            let current = storage.get_issue(&original_issue.id).unwrap().unwrap();
            assert_eq!(
                current.labels.len(),
                original_issue.labels.len(),
                "Cycle {}: Issue {} should have {} labels before export",
                cycle + 1,
                original_issue.id,
                original_issue.labels.len()
            );

            for label in &original_issue.labels {
                assert!(
                    current.labels.contains(label),
                    "Cycle {}: Issue {} should contain label '{}'",
                    cycle + 1,
                    original_issue.id,
                    label
                );
            }
        }

        drop(storage);

        // Export to JSONL
        let export_count = sync::flush(workspace).unwrap();
        assert_eq!(
            export_count,
            test_issues.len(),
            "Cycle {}: Should export {} issues",
            cycle + 1,
            test_issues.len()
        );

        // Verify JSONL file exists and has correct content
        assert!(jsonl_path.exists(), "Cycle {}: JSONL should exist after flush", cycle + 1);
        let jsonl_content = fs::read_to_string(&jsonl_path).unwrap();
        let lines: Vec<&str> = jsonl_content.lines().collect();
        assert_eq!(
            lines.len(),
            test_issues.len(),
            "Cycle {}: JSONL should have {} lines",
            cycle + 1,
            test_issues.len()
        );

        // Clear database
        fs::remove_file(&db_path).unwrap();
        assert!(!db_path.exists(), "Cycle {}: Database should be deleted", cycle + 1);

        // Import from JSONL
        let result = sync::import(workspace).unwrap();
        assert_eq!(
            result.imported,
            test_issues.len(),
            "Cycle {}: Should import {} issues",
            cycle + 1,
            test_issues.len()
        );

        // Verify all issues were imported correctly
        let storage2 = Storage::open(&db_path).unwrap();
        let imported_issues = storage2.list_all_issues().unwrap();
        assert_eq!(
            imported_issues.len(),
            test_issues.len(),
            "Cycle {}: Should have {} issues after import",
            cycle + 1,
            test_issues.len()
        );

        // Verify each issue's labels survived the cycle
        for original_issue in &test_issues {
            let imported = storage2
                .get_issue(&original_issue.id)
                .unwrap()
                .expect(&format!("Cycle {}: Issue {} should be imported", cycle + 1, original_issue.id));

            assert_eq!(
                imported.id,
                original_issue.id,
                "Cycle {}: Issue ID should match",
                cycle + 1
            );
            assert_eq!(
                imported.title,
                original_issue.title,
                "Cycle {}: Issue title should match for {}",
                cycle + 1,
                original_issue.id
            );
            assert_eq!(
                imported.labels.len(),
                original_issue.labels.len(),
                "Cycle {}: Issue {} should have {} labels after import, got {}",
                cycle + 1,
                original_issue.id,
                original_issue.labels.len(),
                imported.labels.len()
            );

            // Verify each label individually
            for expected_label in &original_issue.labels {
                assert!(
                    imported.labels.contains(expected_label),
                    "Cycle {}: Issue {} should contain label '{}', got: {:?}",
                    cycle + 1,
                    original_issue.id,
                    expected_label,
                    imported.labels
                );
            }

            // Sort and compare for exact match
            let mut imported_labels_sorted = imported.labels.clone();
            imported_labels_sorted.sort();
            let mut original_labels_sorted = original_issue.labels.clone();
            original_labels_sorted.sort();
            assert_eq!(
                imported_labels_sorted,
                original_labels_sorted,
                "Cycle {}: Issue {} labels should match exactly after cycle",
                cycle + 1,
                original_issue.id
            );
        }

        // Verify database integrity
        let conn = storage2.conn.lock().unwrap();

        // Check total label count in database
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM bead_labels").unwrap();
        let total_labels: i64 = stmt.query([]).unwrap().next().unwrap().unwrap().get(0).unwrap();

        let expected_total: usize = test_issues.iter().map(|issue| issue.labels.len()).sum();

        assert_eq!(
            total_labels as usize,
            expected_total,
            "Cycle {}: Total label count should be {}",
            cycle + 1,
            expected_total
        );

        // Verify no duplicate labels for any issue
        for issue in &test_issues {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT COUNT(DISTINCT label) FROM bead_labels WHERE bead_id = '{}'",
                    issue.id
                ))
                .unwrap();
            let distinct_count: i64 = stmt.query([]).unwrap().next().unwrap().unwrap().get(0).unwrap();

            let mut stmt2 = conn
                .prepare(&format!(
                    "SELECT COUNT(*) FROM bead_labels WHERE bead_id = '{}'",
                    issue.id
                ))
                .unwrap();
            let total_count: i64 = stmt2.query([]).unwrap().next().unwrap().unwrap().get(0).unwrap();

            assert_eq!(
                distinct_count,
                total_count,
                "Cycle {}: Issue {} should have no duplicate labels (distinct: {}, total: {})",
                cycle + 1,
                issue.id,
                distinct_count,
                total_count
            );
        }

        drop(storage2);
    }

    // Final verification after all cycles
    let final_storage = Storage::open(&db_path).unwrap();
    let final_issues = final_storage.list_all_issues().unwrap();

    assert_eq!(
        final_issues.len(),
        test_issues.len(),
        "After all cycles: Should have {} issues",
        test_issues.len()
    );

    // Final label integrity check
    for original_issue in &test_issues {
        let final_issue = final_storage.get_issue(&original_issue.id).unwrap().unwrap();

        let mut final_labels_sorted = final_issue.labels.clone();
        final_labels_sorted.sort();
        let mut original_labels_sorted = original_issue.labels.clone();
        original_labels_sorted.sort();

        assert_eq!(
            final_labels_sorted,
            original_labels_sorted,
            "After all cycles: Issue {} labels should match original",
            original_issue.id
        );
    }
}
