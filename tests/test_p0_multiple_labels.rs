// Comprehensive test for P0 beads with multiple labels
// This test verifies that beads with P0 (critical) priority can have multiple labels
// and that those labels are correctly stored, retrieved, and serialized.

use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_p0_bead_with_multiple_labels_creation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead with multiple labels
    let bead = Issue {
        id: "bf-p0-multi".to_string(),
        title: "P0 Bead with Multiple Labels".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL, // P0
        labels: vec![
            "critical".to_string(),
            "urgent".to_string(),
            "security".to_string(),
        ],
        description: Some("Testing P0 bead with multiple labels".to_string()),
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Verify storage and retrieval
    let retrieved = storage.get_issue("bf-p0-multi").unwrap().unwrap();
    
    // Test P0 priority
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    
    // Test multiple labels
    assert_eq!(retrieved.labels.len(), 3);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"urgent".to_string()));
    assert!(retrieved.labels.contains(&"security".to_string()));
}

#[test]
fn test_p0_bead_multiple_labels_serialization() {
    // Create P0 bead with multiple labels
    let bead = Issue {
        id: "bf-p0-serialize".to_string(),
        title: "P0 Serialization Test".to_string(),
        issue_type: IssueType::Bug,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["p0".to_string(), "critical".to_string(), "hotfix".to_string()],
        ..Default::default()
    };

    // Test JSON serialization
    let json = serde_json::to_string(&bead).unwrap();
    
    // Verify P0 priority is serialized as 0
    assert!(json.contains("\"priority\":0"));
    
    // Verify all labels are in the JSON
    assert!(json.contains("p0"));
    assert!(json.contains("critical"));
    assert!(json.contains("hotfix"));

    // Test deserialization
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.labels.len(), 3);
}

#[test]
fn test_p0_bead_label_operations() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead with initial labels
    let bead = Issue {
        id: "bf-p0-ops".to_string(),
        title: "P0 Label Operations".to_string(),
        issue_type: IssueType::Feature,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string()],
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Add more labels
    storage.add_label("bf-p0-ops", "urgent").unwrap();
    storage.add_label("bf-p0-ops", "security").unwrap();
    storage.add_label("bf-p0-ops", "performance").unwrap();

    // Verify all labels
    let retrieved = storage.get_issue("bf-p0-ops").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 4);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
}

#[test]
fn test_p0_bead_multiple_labels_filtering() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple P0 beads with different labels
    for i in 1..=3 {
        let bead = Issue {
            id: format!("bf-p0-filter-{}", i),
            title: format!("P0 Filter Test {}", i),
            issue_type: IssueType::Task,
            status: Status::Open,
            priority: Priority::CRITICAL,
            labels: vec![
                "critical".to_string(),
                format!("label-{}", i),
            ],
            ..Default::default()
        };
        storage.create_issue(&bead).unwrap();
    }

    // Filter by P0 priority
    let filter = bead_forge::model::IssueFilter {
        priority: Some(0),
        ..Default::default()
    };
    let p0_beads = storage.list_issues(&filter).unwrap();
    
    // Should have 3 P0 beads
    assert_eq!(p0_beads.len(), 3);
    
    // All should have "critical" label
    for bead in p0_beads {
        assert_eq!(bead.priority, Priority::CRITICAL);
        assert!(bead.labels.contains(&"critical".to_string()));
    }
}

#[test]
fn test_p0_bead_label_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead with multiple labels
    let bead = Issue {
        id: "bf-p0-persist".to_string(),
        title: "P0 Label Persistence".to_string(),
        issue_type: IssueType::Task,
        status: Status::InProgress,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "wip".to_string()],
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Simulate database close and reopen
    drop(storage);
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Verify labels persist
    let retrieved = storage.get_issue("bf-p0-persist").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 2);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"wip".to_string()));
}

#[test]
fn test_p0_bead_with_various_label_counts() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Test with varying numbers of labels
    let label_counts = vec![1, 3, 5, 10];
    
    for (i, count) in label_counts.iter().enumerate() {
        let labels: Vec<String> = (0..*count)
            .map(|j| format!("label-{}", j))
            .collect();
        
        let bead = Issue {
            id: format!("bf-p0-var-{}", i),
            title: format!("P0 with {} labels", count),
            issue_type: IssueType::Task,
            status: Status::Open,
            priority: Priority::CRITICAL,
            labels: labels.clone(),
            ..Default::default()
        };

        storage.create_issue(&bead).unwrap();

        let retrieved = storage.get_issue(&format!("bf-p0-var-{}", i)).unwrap().unwrap();
        assert_eq!(retrieved.labels.len(), *count);
        assert_eq!(retrieved.priority, Priority::CRITICAL);
    }
}

#[test]
fn test_p0_priority_multiple_labels_integration() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 epic with child beads, all with labels
    let epic = Issue {
        id: "bf-p0-epic".to_string(),
        title: "P0 Epic with Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["epic".to_string(), "critical".to_string()],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Create child beads
    for i in 1..=2 {
        let child = Issue {
            id: format!("bf-p0-child-{}", i),
            title: format!("P0 Child {}", i),
            issue_type: IssueType::Task,
            status: Status::Open,
            priority: Priority::CRITICAL,
            labels: vec!["child".to_string(), format!("group-{}", i)],
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();
        
        // Add dependency
        storage.add_dependency(
            &format!("bf-p0-child-{}", i),
            "bf-p0-epic",
            &bead_forge::model::DependencyType::ParentChild,
            "test",
        ).unwrap();
    }

    // Verify all have P0 priority and labels
    let p0_items = storage.list_issues(&bead_forge::model::IssueFilter {
        priority: Some(0),
        ..Default::default()
    }).unwrap();

    assert_eq!(p0_items.len(), 3);
    for item in p0_items {
        assert_eq!(item.priority, Priority::CRITICAL);
        assert!(!item.labels.is_empty());
    }
}

// ============================================================================
// CLI TESTS - P0 beads with multiple labels
// ============================================================================

/// Create a temporary workspace for CLI testing
fn setup_test_workspace() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("test-workspace");
    fs::create_dir_all(&workspace_dir).unwrap();
    let beads_dir = workspace_dir.join(".beads");
    fs::create_dir_all(&beads_dir).unwrap();

    let config_path = beads_dir.join("config.yaml");
    fs::write(
        &config_path,
        r#"issue_prefixes: [bf]
default_priority: 2
default_type: task
claim_ttl_minutes: 30
"#,
    )
    .unwrap();

    let metadata_path = beads_dir.join("metadata.json");
    fs::write(
        &metadata_path,
        r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
    )
    .unwrap();

    let db_path = beads_dir.join("beads.db");
    bead_forge::storage::Storage::open(&db_path).unwrap();

    (temp_dir, beads_dir)
}

/// Get the path to the bf binary
fn get_bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf")
        .unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Run a bf command and return the output
fn run_bf_command(workspace: &Path, args: &[&str]) -> (String, String, bool) {
    let out = Command::new(get_bf_binary())
        .args(args)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf command");
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    let success = out.status.success();
    (stdout, stderr, success)
}

/// Extract bead ID from command output
fn extract_bead_id(output: &str) -> String {
    output
        .lines()
        .find(|line| line.contains("bf-"))
        .and_then(|line| line.split("bf-").nth(1))
        .map(|id| format!("bf-{}", id.trim().split_whitespace().next().unwrap_or(id)))
        .expect("Could not extract bead ID from output")
}

/// Run `bf labels <id>` and return the labels
fn run_labels(workspace: &Path, bead_id: &str) -> Vec<String> {
    let (stdout, stderr, success) = run_bf_command(workspace, &["labels", bead_id]);
    assert!(success, "bf labels failed: {}", stderr);
    stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

#[test]
fn test_p0_cli_create_with_multiple_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with multiple labels via CLI
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 CLI Multiple Labels",
            "--type",
            "bug",
            "--priority",
            "0",
            "--label",
            "urgent",
            "--label",
            "security",
            "--label",
            "hotfix",
        ],
    );
    assert!(success, "bf create failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Verify all labels are present
    let labels = run_labels(workspace, &bead_id);
    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"urgent".to_string()));
    assert!(labels.contains(&"security".to_string()));
    assert!(labels.contains(&"hotfix".to_string()));
}

#[test]
fn test_p0_cli_show_displays_multiple_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 Show Test",
            "--priority",
            "0",
            "--label",
            "critical",
            "--label",
            "production",
            "--label",
            "customer-impact",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Show the bead in text format
    let (show_stdout, show_stderr, show_success) =
        run_bf_command(workspace, &["show", &bead_id]);
    assert!(show_success, "bf show failed: {}", show_stderr);

    // Verify P0 priority and labels are displayed
    assert!(show_stdout.contains("Priority: P0"));
    assert!(show_stdout.contains("Labels:"));
    assert!(show_stdout.contains("critical"));
    assert!(show_stdout.contains("production"));
    assert!(show_stdout.contains("customer-impact"));
}

#[test]
fn test_p0_cli_json_output_with_multiple_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 JSON Test",
            "--priority",
            "0",
            "--label",
            "json-label1",
            "--label",
            "json-label2",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Show in JSON format
    let (show_stdout, show_stderr, show_success) =
        run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    assert!(show_success, "bf show failed: {}", show_stderr);

    // Parse JSON and verify
    let json: serde_json::Value = serde_json::from_str(&show_stdout).unwrap();
    let issue = &json[0];

    // Verify P0 priority
    assert_eq!(issue["priority"], 0);

    // Verify labels array
    let labels = issue["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 2);
    assert!(labels.iter().any(|l| l == "json-label1"));
    assert!(labels.iter().any(|l| l == "json-label2"));
}

#[test]
fn test_p0_cli_label_add_remove() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with one label
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 Label Add/Remove",
            "--priority",
            "0",
            "--label",
            "initial",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Add multiple labels
    let (_, add1_stderr, add1_success) = run_bf_command(
        workspace,
        &["label", "add", &bead_id, "--label", "added1", "--label", "added2"],
    );
    assert!(add1_success, "bf label add failed: {}", add1_stderr);

    // Verify three labels total
    let labels = run_labels(workspace, &bead_id);
    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"initial".to_string()));
    assert!(labels.contains(&"added1".to_string()));
    assert!(labels.contains(&"added2".to_string()));

    // Remove labels
    let (_, rem_stderr, rem_success) = run_bf_command(
        workspace,
        &["label", "remove", &bead_id, "--label", "initial", "--label", "added1"],
    );
    assert!(rem_success, "bf label remove failed: {}", rem_stderr);

    // Verify only added2 remains
    let labels = run_labels(workspace, &bead_id);
    assert_eq!(labels.len(), 1);
    assert!(labels.contains(&"added2".to_string()));
}

#[test]
fn test_p0_cli_jsonl_export_with_multiple_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with multiple labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 JSONL Export",
            "--priority",
            "0",
            "--label",
            "jsonl-test",
            "--label",
            "export-test",
            "--label",
            "p0-test",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Flush to JSONL
    let (_, sync_stderr, sync_success) = run_bf_command(workspace, &["sync", "--flush-only"]);
    assert!(sync_success, "bf sync failed: {}", sync_stderr);

    // Read JSONL file and verify
    let jsonl_path = beads_dir.join("issues.jsonl");
    let jsonl_content = fs::read_to_string(&jsonl_path).unwrap();

    // Find the bead's line
    let bead_line = jsonl_content
        .lines()
        .find(|line| line.contains(&format!("\"id\":\"{}\"", bead_id)))
        .expect("Bead not found in JSONL");

    // Verify P0 priority and labels are in JSONL
    assert!(bead_line.contains(r#""priority":0"#));
    assert!(bead_line.contains("jsonl-test"));
    assert!(bead_line.contains("export-test"));
    assert!(bead_line.contains("p0-test"));
    assert!(bead_line.contains(r#""labels":["#));
}

#[test]
fn test_p0_cli_list_with_priority_filter() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create multiple P0 beads with labels
    let mut p0_ids = Vec::new();
    for i in 1..=3 {
        let (stdout, _, _) = run_bf_command(
            workspace,
            &[
                "create",
                "--title",
                &format!("P0 List Test {}", i),
                "--priority",
                "0",
                "--label",
                &format!("p0-label-{}", i),
            ],
        );
        p0_ids.push(extract_bead_id(&stdout));
    }

    // Create a non-P0 bead
    run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Non-P0 List Test",
            "--priority",
            "2",
            "--label",
            "regular",
        ],
    );

    // List only P0 beads
    let (list_stdout, list_stderr, list_success) =
        run_bf_command(workspace, &["list", "--priority", "0"]);
    assert!(list_success, "bf list failed: {}", list_stderr);

    // Verify all P0 beads are listed
    for p0_id in &p0_ids {
        assert!(list_stdout.contains(p0_id));
    }
    // Should show P0 priority
    assert!(list_stdout.contains("P0"));
}

// ============================================================================
// EDGE CASES AND INTEGRATION TESTS
// ============================================================================

#[test]
fn test_p0_bead_with_all_issue_types_multiple_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 beads of different types with multiple labels
    let test_cases = vec![
        ("p0-task", IssueType::Task, vec!["task-label1".to_string(), "task-label2".to_string()]),
        ("p0-bug", IssueType::Bug, vec!["bug-label1".to_string(), "bug-label2".to_string()]),
        ("p0-feature", IssueType::Feature, vec!["feature-label1".to_string(), "feature-label2".to_string()]),
        ("p0-epic", IssueType::Epic, vec!["epic-label1".to_string(), "epic-label2".to_string()]),
        ("p0-chore", IssueType::Chore, vec!["chore-label1".to_string(), "chore-label2".to_string()]),
    ];

    for (id, issue_type, labels) in test_cases {
        let issue = Issue {
            id: id.to_string(),
            title: format!("P0 {}", issue_type.as_str()),
            issue_type: issue_type.clone(),
            status: Status::Open,
            priority: Priority::CRITICAL,
            labels: labels.clone(),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();

        // Verify
        let retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(retrieved.priority, Priority::CRITICAL);
        assert_eq!(retrieved.issue_type, issue_type);
        assert_eq!(retrieved.labels.len(), 2);
        assert!(retrieved.labels.contains(&labels[0]));
        assert!(retrieved.labels.contains(&labels[1]));
    }
}

#[test]
fn test_p0_bead_with_large_label_set() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead with many labels
    let many_labels: Vec<String> = (1..=15)
        .map(|i| format!("label-{}", i))
        .collect();

    let issue = Issue {
        id: "p0-many-labels".to_string(),
        title: "P0 with Many Labels".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: many_labels.clone(),
        ..Default::default()
    };

    storage.create_issue(&issue).unwrap();

    // Verify all labels preserved
    let retrieved = storage.get_issue("p0-many-labels").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 15);
    for label in &many_labels {
        assert!(retrieved.labels.contains(label));
    }
}

#[test]
fn test_p0_different_states_with_multiple_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let states = vec![
        ("p0-open", Status::Open),
        ("p0-in-progress", Status::InProgress),
        ("p0-blocked", Status::Blocked),
    ];

    for (id, status) in states {
        let issue = Issue {
            id: id.to_string(),
            title: format!("P0 {:?}", status),
            issue_type: IssueType::Task,
            status: status.clone(),
            priority: Priority::CRITICAL,
            labels: vec![
                format!("state-{:?}", status),
                "p0-test".to_string(),
                "multi-label".to_string(),
            ],
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();

        // Verify
        let retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(retrieved.priority, Priority::CRITICAL);
        assert_eq!(retrieved.status, status);
        assert_eq!(retrieved.labels.len(), 3);
        assert!(retrieved.labels.contains(&"p0-test".to_string()));
        assert!(retrieved.labels.contains(&"multi-label".to_string()));
    }
}

#[test]
fn test_p0_bead_empty_label_handling() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead with empty labels
    let issue = Issue {
        id: "p0-empty-labels".to_string(),
        title: "P0 with Empty Labels".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![],
        ..Default::default()
    };

    storage.create_issue(&issue).unwrap();

    // Verify
    let retrieved = storage.get_issue("p0-empty-labels").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 0);

    // Add labels to empty P0 bead
    storage.add_label("p0-empty-labels", "new-label").unwrap();
    storage.add_label("p0-empty-labels", "another-label").unwrap();

    let retrieved = storage.get_issue("p0-empty-labels").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 2);

    // Remove all labels
    storage.remove_label("p0-empty-labels", "new-label").unwrap();
    storage.remove_label("p0-empty-labels", "another-label").unwrap();

    let retrieved = storage.get_issue("p0-empty-labels").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 0);
    assert_eq!(retrieved.priority, Priority::CRITICAL); // P0 should persist
}
