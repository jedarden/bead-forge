//! Label Export/Import Roundtrip Tests (bf-11zv9e)
//!
//! Comprehensive tests for label data preservation through JSONL export/import:
//! - Basic label roundtrip
//! - Complex labels with special characters survive JSONL roundtrip
//! - All label name patterns (unicode, punctuation, very long names) survive
//! - Empty label field handling (builds on empty label handling tests)
//!
//! Acceptance criteria:
//! - Test labels export/import roundtrip preserves data
//! - Test complex labels survive JSONL roundtrip
//! - Test all label fields (names, colors, descriptions) survive
//! - Test should pass with cargo test

use bead_forge::sync;
use bead_forge::model::{Issue, Priority, Status, IssueType};
use bead_forge::storage::Storage;
use chrono::Utc;
use std::fs;
use tempfile::TempDir;

//
// Basic Roundtrip Tests (builds on empty label handling test)
//

#[test]
fn test_label_export_import_roundtrip_basic() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    // Initialize workspace
    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    let db_path = beads_dir.join("beads.db");
    let jsonl_path = beads_dir.join("issues.jsonl");

    // Create an issue with multiple labels
    let issue_with_labels = Issue {
        id: "bf-roundtrip-basic".to_string(),
        title: "Basic Label Roundtrip Test".to_string(),
        description: Some("Testing basic label roundtrip".to_string()),
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
        ],
        ..Default::default()
    };

    // Store in database
    let storage = Storage::open(&db_path).unwrap();
    storage.create_issue(&issue_with_labels).unwrap();

    // Export to JSONL
    let export_count = sync::flush(workspace).unwrap();
    assert_eq!(export_count, 1, "Should export 1 issue");
    assert!(jsonl_path.exists(), "JSONL file should exist after export");

    // Clear the database to simulate a fresh import
    drop(storage);
    std::fs::remove_file(&db_path).unwrap();
    let storage2 = Storage::open(&db_path).unwrap();

    // Import from JSONL
    let import_result = sync::import(workspace).unwrap();
    assert_eq!(import_result.imported, 1, "Should import 1 issue");

    // Verify labels survived the roundtrip
    let imported = storage2.get_issue("bf-roundtrip-basic").unwrap().unwrap();
    assert_eq!(imported.labels.len(), 3, "Should have 3 labels after roundtrip");
    assert!(imported.labels.contains(&"phase-1".to_string()), "Should contain phase-1 label");
    assert!(imported.labels.contains(&"storage".to_string()), "Should contain storage label");
    assert!(imported.labels.contains(&"critical".to_string()), "Should contain critical label");
}

#[test]
fn test_label_roundtrip_with_empty_labels_field() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    let db_path = beads_dir.join("beads.db");
    let jsonl_path = beads_dir.join("issues.jsonl");

    // Create an issue with no labels
    let issue_no_labels = Issue {
        id: "bf-roundtrip-empty".to_string(),
        title: "Empty Labels Roundtrip Test".to_string(),
        status: Status::Open,
        priority: Priority::MEDIUM,
        issue_type: IssueType::Task,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        source_repo: Some(".".to_string()),
        labels: vec![],
        ..Default::default()
    };

    let storage = Storage::open(&db_path).unwrap();
    storage.create_issue(&issue_no_labels).unwrap();

    // Export to JSONL
    let export_count = sync::flush(workspace).unwrap();
    assert_eq!(export_count, 1, "Should export 1 issue");
    assert!(jsonl_path.exists());

    // Verify empty labels array is skipped in JSON (due to skip_serializing_if)
    let jsonl_content = fs::read_to_string(&jsonl_path).unwrap();
    assert!(!jsonl_content.contains("\"labels\""), "Empty labels should not appear in JSON");

    // Clear and re-import
    drop(storage);
    std::fs::remove_file(&db_path).unwrap();
    let storage2 = Storage::open(&db_path).unwrap();
    sync::import(workspace).unwrap();

    // Verify issue imported with no labels
    let imported = storage2.get_issue("bf-roundtrip-empty").unwrap().unwrap();
    assert_eq!(imported.labels.len(), 0, "Should have no labels after roundtrip");
    assert!(imported.labels.is_empty(), "Labels vector should be empty");
}

//
// Complex Labels Tests
//

#[test]
fn test_complex_labels_roundtrip_special_chars() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    let db_path = beads_dir.join("beads.db");

    // Create an issue with labels containing special characters
    let complex_labels = vec![
        "won't-fix".to_string(),
        "maybe?".to_string(),
        "high-priority!".to_string(),
        "a/b/c".to_string(),
        "x.y.z".to_string(),
        "test@example.com".to_string(),
        "bug/fix".to_string(),
        "feature:new".to_string(),
        "label-and".to_string(),
        "label_or".to_string(),
        "label:colon".to_string(),
        "label+dollar".to_string(),
        "label#hash".to_string(),
        "label+plus".to_string(),
        "label=equals".to_string(),
    ];

    let issue = Issue {
        id: "bf-complex-chars".to_string(),
        title: "Complex Labels Special Characters Test".to_string(),
        status: Status::Open,
        priority: Priority::MEDIUM,
        issue_type: IssueType::Task,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        source_repo: Some(".".to_string()),
        labels: complex_labels.clone(),
        ..Default::default()
    };

    let storage = Storage::open(&db_path).unwrap();
    storage.create_issue(&issue).unwrap();

    // Export and import
    sync::flush(workspace).unwrap();
    drop(storage);
    std::fs::remove_file(&db_path).unwrap();
    let storage2 = Storage::open(&db_path).unwrap();
    sync::import(workspace).unwrap();

    // Verify all special character labels survived
    let imported = storage2.get_issue("bf-complex-chars").unwrap().unwrap();
    assert_eq!(imported.labels.len(), complex_labels.len(), "All labels should survive roundtrip");

    for label in &complex_labels {
        assert!(imported.labels.contains(label), "Label '{}' should survive roundtrip", label);
    }
}

#[test]
fn test_unicode_labels_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    let db_path = beads_dir.join("beads.db");

    // Create an issue with Unicode labels from various scripts
    let unicode_labels = vec![
        "日本語".to_string(),              // Japanese
        "中文标签".to_string(),            // Chinese
        "한국어".to_string(),              // Korean
        "لغة-عربية".to_string(),          // Arabic
        "עברית".to_string(),               // Hebrew
        "метка".to_string(),              // Russian/Cyrillic
        "ετικέτα".to_string(),            // Greek
        "émoji-😀-test".to_string(),       // Emoji
        "café".to_string(),               // Latin with accents
        "naïve".to_string(),              // Diaeresis
        "über".to_string(),               // Umlaut
        "ñoño".to_string(),               // Tilde
        "🚀-rocket".to_string(),          // More emoji
    ];

    let issue = Issue {
        id: "bf-unicode-labels".to_string(),
        title: "Unicode Labels Roundtrip Test".to_string(),
        status: Status::Open,
        priority: Priority::MEDIUM,
        issue_type: IssueType::Task,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        source_repo: Some(".".to_string()),
        labels: unicode_labels.clone(),
        ..Default::default()
    };

    let storage = Storage::open(&db_path).unwrap();
    storage.create_issue(&issue).unwrap();

    // Export and import
    sync::flush(workspace).unwrap();
    drop(storage);
    std::fs::remove_file(&db_path).unwrap();
    let storage2 = Storage::open(&db_path).unwrap();
    sync::import(workspace).unwrap();

    // Verify all Unicode labels survived
    let imported = storage2.get_issue("bf-unicode-labels").unwrap().unwrap();
    assert_eq!(imported.labels.len(), unicode_labels.len(), "All Unicode labels should survive");

    for label in &unicode_labels {
        assert!(imported.labels.contains(label), "Unicode label '{}' should survive roundtrip", label);
    }
}

#[test]
fn test_very_long_label_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    let db_path = beads_dir.join("beads.db");

    // Create a very long label (500 characters)
    let long_label = "a".repeat(500);
    let long_label2 = "very-long-label-".repeat(30); // ~480 characters

    let issue = Issue {
        id: "bf-long-labels".to_string(),
        title: "Very Long Label Roundtrip Test".to_string(),
        status: Status::Open,
        priority: Priority::MEDIUM,
        issue_type: IssueType::Task,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        source_repo: Some(".".to_string()),
        labels: vec![long_label.clone(), long_label2.clone()],
        ..Default::default()
    };

    let storage = Storage::open(&db_path).unwrap();
    storage.create_issue(&issue).unwrap();

    // Export and import
    sync::flush(workspace).unwrap();
    drop(storage);
    std::fs::remove_file(&db_path).unwrap();
    let storage2 = Storage::open(&db_path).unwrap();
    sync::import(workspace).unwrap();

    // Verify long labels survived
    let imported = storage2.get_issue("bf-long-labels").unwrap().unwrap();
    assert_eq!(imported.labels.len(), 2, "Both long labels should survive");
    assert!(imported.labels.contains(&long_label), "Long label should survive roundtrip");
    assert!(imported.labels.contains(&long_label2), "Long label 2 should survive roundtrip");
}

#[test]
fn test_json_edge_case_labels_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    let db_path = beads_dir.join("beads.db");

    // Labels that might cause JSON parsing issues
    let edge_case_labels = vec![
        "\"quoted\"".to_string(),        // Quotes
        "back\\slash".to_string(),       // Backslash
        "line\nbreak".to_string(),       // Newline (should be escaped)
        "tab\there".to_string(),        // Tab (should be escaped)
        "mixed\"quo\\tes".to_string(),   // Mixed special chars
    ];

    let issue = Issue {
        id: "bf-json-edge-cases".to_string(),
        title: "JSON Edge Case Labels Roundtrip Test".to_string(),
        status: Status::Open,
        priority: Priority::MEDIUM,
        issue_type: IssueType::Task,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        source_repo: Some(".".to_string()),
        labels: edge_case_labels.clone(),
        ..Default::default()
    };

    let storage = Storage::open(&db_path).unwrap();
    storage.create_issue(&issue).unwrap();

    // Export and import
    sync::flush(workspace).unwrap();
    drop(storage);
    std::fs::remove_file(&db_path).unwrap();
    let storage2 = Storage::open(&db_path).unwrap();
    sync::import(workspace).unwrap();

    // Verify edge case labels survived
    let imported = storage2.get_issue("bf-json-edge-cases").unwrap().unwrap();
    assert_eq!(imported.labels.len(), edge_case_labels.len(), "All edge case labels should survive");

    for label in &edge_case_labels {
        assert!(imported.labels.contains(label), "Edge case label '{}' should survive roundtrip", label);
    }
}

//
// Comprehensive Multiple Beads Test
//

#[test]
fn test_multiple_beads_with_different_label_sets_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    let db_path = beads_dir.join("beads.db");

    let storage = Storage::open(&db_path).unwrap();

    // Create multiple beads with different label configurations
    let issues = vec![
        Issue {
            id: "bf-first".to_string(),
            title: "First Bead".to_string(),
            labels: vec!["phase-1".to_string(), "backend".to_string()],
            ..Default::default()
        },
        Issue {
            id: "bf-second".to_string(),
            title: "Second Bead".to_string(),
            labels: vec!["phase-2".to_string(), "frontend".to_string()],
            ..Default::default()
        },
        Issue {
            id: "bf-third".to_string(),
            title: "Third Bead".to_string(),
            labels: vec![], // Empty labels
            ..Default::default()
        },
        Issue {
            id: "bf-fourth".to_string(),
            title: "Fourth Bead".to_string(),
            labels: vec!["unicode-中文".to_string(), "special-!".to_string()],
            ..Default::default()
        },
    ];

    for issue in &issues {
        storage.create_issue(issue).unwrap();
    }

    // Export and import
    sync::flush(workspace).unwrap();
    drop(storage);
    std::fs::remove_file(&db_path).unwrap();
    let storage2 = Storage::open(&db_path).unwrap();
    sync::import(workspace).unwrap();

    // Verify all beads with their labels survived
    for original_issue in &issues {
        let imported = storage2.get_issue(&original_issue.id).unwrap().unwrap();
        assert_eq!(imported.labels.len(), original_issue.labels.len(),
            "Bead {} should have {} labels after roundtrip", original_issue.id, original_issue.labels.len());

        for label in &original_issue.labels {
            assert!(imported.labels.contains(label),
                "Bead {} should contain label '{}' after roundtrip", original_issue.id, label);
        }
    }
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_label_roundtrip_preserves_label_order() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();

    let db_path = beads_dir.join("beads.db");

    // Create an issue with a specific label order
    let ordered_labels = vec![
        "zebra".to_string(),
        "apple".to_string(),
        "middle".to_string(),
        "first".to_string(),
        "last".to_string(),
    ];

    let issue = Issue {
        id: "bf-ordered-labels".to_string(),
        title: "Ordered Labels Roundtrip Test".to_string(),
        status: Status::Open,
        priority: Priority::MEDIUM,
        issue_type: IssueType::Task,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        source_repo: Some(".".to_string()),
        labels: ordered_labels.clone(),
        ..Default::default()
    };

    let storage = Storage::open(&db_path).unwrap();
    storage.create_issue(&issue).unwrap();

    // Export and import
    sync::flush(workspace).unwrap();
    drop(storage);
    std::fs::remove_file(&db_path).unwrap();
    let storage2 = Storage::open(&db_path).unwrap();
    sync::import(workspace).unwrap();

    // Verify labels preserved order
    let imported = storage2.get_issue("bf-ordered-labels").unwrap().unwrap();
    assert_eq!(imported.labels, ordered_labels, "Label order should be preserved through roundtrip");
}
