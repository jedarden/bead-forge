// Epic Label Edge Cases and Type Preservation Tests
//
// Comprehensive tests for epic label edge cases and type preservation through all operations.
// These tests ensure that epic type is preserved and labels work correctly across all scenarios.

use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Get the path to the bf binary
fn get_bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Create a temporary workspace for testing
fn setup_test_workspace() -> (TempDir, std::path::PathBuf) {
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
fn run_labels(workspace: &std::path::Path, bead_id: &str) -> Vec<String> {
    let out = Command::new(get_bf_binary())
        .args(["labels", bead_id])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf labels");
    assert!(
        out.status.success(),
        "bf labels failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

// ============================================================================
// ACCEPTANCE CRITERION 1: Test epic with NO labels (empty label set)
// ============================================================================

#[test]
fn test_epic_no_labels_cli_create() {
    // Test creating an epic with NO labels via CLI
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic without any --label flags
    let out = Command::new(get_bf_binary())
        .args(["create", "--title", "Epic No Labels", "--type", "epic"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(
        out.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8(out.stdout).unwrap();
    let bead_id = extract_bead_id(&stdout);

    // Verify epic has empty label set
    let labels = run_labels(workspace, &bead_id);
    assert_eq!(
        labels.len(),
        0,
        "epic created without labels should have empty label set, got {:?}",
        labels
    );

    // Verify epic type is preserved
    let show_out = Command::new(get_bf_binary())
        .args(["show", &bead_id, "--format", "json"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");
    assert!(show_out.status.success());

    let json = String::from_utf8(show_out.stdout).unwrap();
    assert!(
        json.contains(r#""issue_type":"epic""#),
        "epic type should be preserved in show output"
    );
}

#[test]
fn test_epic_no_labels_storage_api() {
    // Test epic with no labels via storage API
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let epic = Issue {
        id: "epic-no-labels".to_string(),
        title: "Epic with No Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec![], // Empty label set
        priority: Priority::CRITICAL,
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Retrieve and verify
    let retrieved = storage.get_issue("epic-no-labels").unwrap().unwrap();
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.labels.len(), 0);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
}

#[test]
fn test_epic_no_labels_json_serialization() {
    // Test epic with no labels serializes correctly
    let epic = Issue {
        id: "epic-empty-serialize".to_string(),
        title: "Empty Labels Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };

    let json = serde_json::to_string(&epic).unwrap();

    // Empty labels array should be skipped in serialization
    assert!(!json.contains(r#""labels":"#));
    assert!(json.contains(r#""issue_type":"epic""#));

    // Deserialize and verify
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.labels.len(), 0);
}

#[test]
fn test_epic_no_labels_add_and_remove() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic without labels
    let out = Command::new(get_bf_binary())
        .args(["create", "--title", "Empty Labels Epic", "--type", "epic"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(out.status.success());

    let stdout = String::from_utf8(out.stdout).unwrap();
    let bead_id = extract_bead_id(&stdout);

    // Verify empty labels
    let labels = run_labels(workspace, &bead_id);
    assert_eq!(labels.len(), 0);

    // Add label to empty epic
    let add_out = Command::new(get_bf_binary())
        .args(["label", "add", &bead_id, "--label", "first-label"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf label add");
    assert!(add_out.status.success());

    // Verify now has one label
    let labels = run_labels(workspace, &bead_id);
    assert_eq!(labels.len(), 1);
    assert!(labels.contains(&"first-label".to_string()));

    // Remove the label
    let remove_out = Command::new(get_bf_binary())
        .args(["label", "remove", &bead_id, "--label", "first-label"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf label remove");
    assert!(remove_out.status.success());

    // Verify back to empty
    let labels = run_labels(workspace, &bead_id);
    assert_eq!(labels.len(), 0);
}

// ============================================================================
// ACCEPTANCE CRITERION 2: Test epic with special characters in labels
// ============================================================================

#[test]
fn test_epic_special_characters_labels_cli() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Test various special character labels
    let special_labels = vec![
        "label-with-dashes",
        "label_with_underscores",
        "label.with.dots",
        "label/with/slashes",
        "label:with:colons",
        "label@with@at",
        "label+with+plus",
        "label=with=equals",
        "label#with#hash",
    ];

    // Create epic with special character labels
    let mut cmd = Command::new(get_bf_binary());
    cmd.args(["create", "--title", "Special Chars Epic", "--type", "epic"]);
    for label in &special_labels {
        cmd.args(["--label", label]);
    }
    let out = cmd
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(out.status.success());

    let stdout = String::from_utf8(out.stdout).unwrap();
    let bead_id = extract_bead_id(&stdout);

    // Verify all special character labels are preserved
    let labels = run_labels(workspace, &bead_id);
    assert_eq!(labels.len(), special_labels.len());
    for label in &special_labels {
        assert!(
            labels.contains(&label.to_string()),
            "special label '{}' should be present in {:?}",
            label,
            labels
        );
    }

    // Verify epic type is preserved with special character labels
    let show_out = Command::new(get_bf_binary())
        .args(["show", &bead_id, "--format", "json"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");
    assert!(show_out.status.success());

    let json = String::from_utf8(show_out.stdout).unwrap();
    assert!(json.contains(r#""issue_type":"epic""#));
}

#[test]
fn test_epic_unicode_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Test Unicode labels
    let unicode_labels = vec![
        "label-日本語",
        "label-emoji-🎉",
        "label-русский",
        "label-العربية",
        "label-ελληνικά",
    ];

    let mut cmd = Command::new(get_bf_binary());
    cmd.args(["create", "--title", "Unicode Epic", "--type", "epic"]);
    for label in &unicode_labels {
        cmd.args(["--label", label]);
    }
    let out = cmd
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(out.status.success());

    let stdout = String::from_utf8(out.stdout).unwrap();
    let bead_id = extract_bead_id(&stdout);

    // Verify all Unicode labels are preserved
    let labels = run_labels(workspace, &bead_id);
    assert_eq!(labels.len(), unicode_labels.len());
    for label in &unicode_labels {
        assert!(
            labels.contains(&label.to_string()),
            "Unicode label '{}' should be present in {:?}",
            label,
            labels
        );
    }

    // Verify epic type preserved with Unicode labels
    let show_out = Command::new(get_bf_binary())
        .args(["show", &bead_id, "--format", "json"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");
    assert!(show_out.status.success());

    let json = String::from_utf8(show_out.stdout).unwrap();
    assert!(json.contains(r#""issue_type":"epic""#));
}

#[test]
fn test_epic_label_whitespace_handling() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic with labels that have leading/trailing spaces
    let out = Command::new(get_bf_binary())
        .args([
            "create",
            "--title",
            "Whitespace Epic",
            "--type",
            "epic",
            "--label",
            "  spaces-around  ",
            "--label",
            "tabs\taround\t",
        ])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(out.status.success());

    let stdout = String::from_utf8(out.stdout).unwrap();
    let bead_id = extract_bead_id(&stdout);

    // Verify labels are stored (preserving or trimming depending on implementation)
    let labels = run_labels(workspace, &bead_id);
    assert!(!labels.is_empty());

    // Verify epic type preserved
    let show_out = Command::new(get_bf_binary())
        .args(["show", &bead_id, "--format", "json"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");
    assert!(show_out.status.success());

    let json = String::from_utf8(show_out.stdout).unwrap();
    assert!(json.contains(r#""issue_type":"epic""#));
}

// ============================================================================
// ACCEPTANCE CRITERION 3: Verify epic type preserved through all label operations
// ============================================================================

#[test]
fn test_epic_type_preserved_through_label_add() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic
    let out = Command::new(get_bf_binary())
        .args(["create", "--title", "Type Preserve Epic", "--type", "epic"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(out.status.success());

    let stdout = String::from_utf8(out.stdout).unwrap();
    let bead_id = extract_bead_id(&stdout);

    // Add labels
    let add_out = Command::new(get_bf_binary())
        .args([
            "label", "add", &bead_id, "--label", "label1", "--label", "label2",
        ])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf label add");
    assert!(add_out.status.success());

    // Verify epic type still epic
    let show_out = Command::new(get_bf_binary())
        .args(["show", &bead_id, "--format", "json"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");
    assert!(show_out.status.success());

    let json = String::from_utf8(show_out.stdout).unwrap();
    assert!(json.contains(r#""issue_type":"epic""#));
}

#[test]
fn test_epic_type_preserved_through_label_remove() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic with labels
    let out = Command::new(get_bf_binary())
        .args([
            "create",
            "--title",
            "Type Remove Epic",
            "--type",
            "epic",
            "--label",
            "label1",
            "--label",
            "label2",
        ])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(out.status.success());

    let stdout = String::from_utf8(out.stdout).unwrap();
    let bead_id = extract_bead_id(&stdout);

    // Remove labels
    let remove_out = Command::new(get_bf_binary())
        .args(["label", "remove", &bead_id, "--label", "label1"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf label remove");
    assert!(remove_out.status.success());

    // Verify epic type still epic
    let show_out = Command::new(get_bf_binary())
        .args(["show", &bead_id, "--format", "json"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");
    assert!(show_out.status.success());

    let json = String::from_utf8(show_out.stdout).unwrap();
    assert!(json.contains(r#""issue_type":"epic""#));
}

#[test]
fn test_epic_type_preserved_through_update() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic
    let out = Command::new(get_bf_binary())
        .args(["create", "--title", "Type Update Epic", "--type", "epic"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(out.status.success());

    let stdout = String::from_utf8(out.stdout).unwrap();
    let bead_id = extract_bead_id(&stdout);

    // Update epic with different fields (but NOT type)
    let update_out = Command::new(get_bf_binary())
        .args([
            "update",
            &bead_id,
            "--status",
            "in_progress",
            "--priority",
            "0",
        ])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf update");
    assert!(update_out.status.success());

    // Verify epic type still epic
    let show_out = Command::new(get_bf_binary())
        .args(["show", &bead_id, "--format", "json"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");
    assert!(show_out.status.success());

    let json = String::from_utf8(show_out.stdout).unwrap();
    assert!(json.contains(r#""issue_type":"epic""#));
    assert!(json.contains(r#""status":"in_progress""#));
    assert!(json.contains(r#""priority":0"#));
}

#[test]
fn test_epic_type_preserved_through_jsonl_sync() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic with labels
    let out = Command::new(get_bf_binary())
        .args([
            "create",
            "--title",
            "Sync Epic",
            "--type",
            "epic",
            "--label",
            "sync-test",
        ])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(out.status.success());

    let stdout = String::from_utf8(out.stdout).unwrap();
    let bead_id = extract_bead_id(&stdout);

    // Flush to JSONL
    let sync_out = Command::new(get_bf_binary())
        .args(["sync", "--flush-only"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf sync");
    assert!(sync_out.status.success());

    // Read JSONL and verify epic type and labels
    let jsonl_path = workspace.join(".beads/issues.jsonl");
    let jsonl_content = fs::read_to_string(&jsonl_path).unwrap();

    // Verify epic type in JSONL
    assert!(jsonl_content.contains(r#""issue_type":"epic""#));
    assert!(jsonl_content.contains(r#""labels":["sync-test"]"#));
}

// ============================================================================
// ACCEPTANCE CRITERION 4: Test epic labels vs other issue types
// ============================================================================

#[test]
fn test_epic_labels_vs_task_labels_no_special_casing() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic with labels
    let epic_out = Command::new(get_bf_binary())
        .args([
            "create",
            "--title",
            "Epic",
            "--type",
            "epic",
            "--label",
            "shared-label",
            "--label",
            "epic-only",
        ])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(epic_out.status.success());

    let epic_stdout = String::from_utf8(epic_out.stdout).unwrap();
    let epic_id = extract_bead_id(&epic_stdout);

    // Create task with same labels
    let task_out = Command::new(get_bf_binary())
        .args([
            "create",
            "--title",
            "Task",
            "--type",
            "task",
            "--label",
            "shared-label",
            "--label",
            "task-only",
        ])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(task_out.status.success());

    let task_stdout = String::from_utf8(task_out.stdout).unwrap();
    let task_id = extract_bead_id(&task_stdout);

    // Verify both have their labels
    let epic_labels = run_labels(workspace, &epic_id);
    let task_labels = run_labels(workspace, &task_id);

    assert_eq!(epic_labels.len(), 2);
    assert!(epic_labels.contains(&"shared-label".to_string()));
    assert!(epic_labels.contains(&"epic-only".to_string()));

    assert_eq!(task_labels.len(), 2);
    assert!(task_labels.contains(&"shared-label".to_string()));
    assert!(task_labels.contains(&"task-only".to_string()));

    // Verify types are different
    let epic_show_out = Command::new(get_bf_binary())
        .args(["show", &epic_id, "--format", "json"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");
    let epic_json = String::from_utf8(epic_show_out.stdout).unwrap();
    assert!(epic_json.contains(r#""issue_type":"epic""#));

    let task_show_out = Command::new(get_bf_binary())
        .args(["show", &task_id, "--format", "json"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");
    let task_json = String::from_utf8(task_show_out.stdout).unwrap();
    assert!(task_json.contains(r#""issue_type":"task""#));
}

#[test]
fn test_all_issue_types_with_labels_work_identically() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let issue_types = vec![
        ("task", "task-label"),
        ("bug", "bug-label"),
        ("feature", "feature-label"),
        ("epic", "epic-label"),
        ("chore", "chore-label"),
        ("docs", "docs-label"),
        ("question", "question-label"),
    ];

    let mut bead_ids = Vec::new();

    // Create one of each type with a unique label
    for (issue_type, label) in &issue_types {
        let out = Command::new(get_bf_binary())
            .args([
                "create",
                "--title",
                &format!("{} Test", issue_type),
                "--type",
                issue_type,
                "--label",
                label,
            ])
            .current_dir(workspace)
            .output()
            .expect("Failed to run bf create");
        assert!(out.status.success());

        let stdout = String::from_utf8(out.stdout).unwrap();
        let bead_id = extract_bead_id(&stdout);
        bead_ids.push((bead_id, issue_type.to_string(), label.to_string()));
    }

    // Verify all have their labels
    for (bead_id, issue_type, expected_label) in &bead_ids {
        let labels = run_labels(workspace, bead_id);
        assert_eq!(
            labels.len(),
            1,
            "{} should have exactly one label, got {:?}",
            issue_type,
            labels
        );
        assert!(
            labels.contains(&expected_label.to_string()),
            "{} should have label '{}', got {:?}",
            issue_type,
            expected_label,
            labels
        );

        // Verify type is correct
        let show_out = Command::new(get_bf_binary())
            .args(["show", bead_id, "--format", "json"])
            .current_dir(workspace)
            .output()
            .expect("Failed to run bf show");
        let json = String::from_utf8(show_out.stdout).unwrap();
        assert!(
            json.contains(&format!(r#""issue_type":"{}""#, issue_type)),
            "{} type should be preserved",
            issue_type
        );
    }
}

#[test]
fn test_label_operations_across_issue_types_no_special_casing() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic and task with same initial label
    for issue_type in &["epic", "task", "bug"] {
        let out = Command::new(get_bf_binary())
            .args([
                "create",
                "--title",
                &format!("{} Label Test", issue_type),
                "--type",
                issue_type,
                "--label",
                "initial-label",
            ])
            .current_dir(workspace)
            .output()
            .expect("Failed to run bf create");
        assert!(out.status.success());

        let stdout = String::from_utf8(out.stdout).unwrap();
        let bead_id = extract_bead_id(&stdout);

        // Add label to each
        let add_out = Command::new(get_bf_binary())
            .args(["label", "add", &bead_id, "--label", "added-label"])
            .current_dir(workspace)
            .output()
            .expect("Failed to run bf label add");
        assert!(add_out.status.success());

        // Verify label added
        let labels = run_labels(workspace, &bead_id);
        assert_eq!(labels.len(), 2);
        assert!(labels.contains(&"initial-label".to_string()));
        assert!(labels.contains(&"added-label".to_string()));

        // Remove label from each
        let remove_out = Command::new(get_bf_binary())
            .args(["label", "remove", &bead_id, "--label", "initial-label"])
            .current_dir(workspace)
            .output()
            .expect("Failed to run bf label remove");
        assert!(remove_out.status.success());

        // Verify label removed
        let labels = run_labels(workspace, &bead_id);
        assert_eq!(labels.len(), 1);
        assert!(labels.contains(&"added-label".to_string()));

        // Verify type still correct after label operations
        let show_out = Command::new(get_bf_binary())
            .args(["show", &bead_id, "--format", "json"])
            .current_dir(workspace)
            .output()
            .expect("Failed to run bf show");
        let json = String::from_utf8(show_out.stdout).unwrap();
        assert!(
            json.contains(&format!(r#""issue_type":"{}""#, issue_type)),
            "{} type should be preserved after label operations",
            issue_type
        );
    }
}

// ============================================================================
// ACCEPTANCE CRITERION 5: Comprehensive CLI integration test suite
// ============================================================================

#[test]
fn test_comprehensive_epic_label_workflow() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // 1. Create epic with no labels
    let create_out = Command::new(get_bf_binary())
        .args(["create", "--title", "Comprehensive Epic", "--type", "epic"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(create_out.status.success());

    let create_stdout = String::from_utf8(create_out.stdout).unwrap();
    let bead_id = extract_bead_id(&create_stdout);

    // 2. Verify empty label set
    let labels = run_labels(workspace, &bead_id);
    assert_eq!(labels.len(), 0);

    // 3. Add multiple labels at once
    let add_out = Command::new(get_bf_binary())
        .args([
            "label", "add", &bead_id, "--label", "label1", "--label", "label2", "--label", "label3",
        ])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf label add");
    assert!(add_out.status.success());

    // 4. Verify all labels added
    let labels = run_labels(workspace, &bead_id);
    assert_eq!(labels.len(), 3);

    // 5. Add duplicate label (should be idempotent)
    let dup_out = Command::new(get_bf_binary())
        .args(["label", "add", &bead_id, "--label", "label1"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf label add");
    assert!(dup_out.status.success());

    let labels = run_labels(workspace, &bead_id);
    assert_eq!(labels.len(), 3, "duplicate add should not increase count");

    // 6. Remove one label
    let remove_out = Command::new(get_bf_binary())
        .args(["label", "remove", &bead_id, "--label", "label2"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf label remove");
    assert!(remove_out.status.success());

    let labels = run_labels(workspace, &bead_id);
    assert_eq!(labels.len(), 2);
    assert!(!labels.contains(&"label2".to_string()));

    // 7. Remove non-existent label (should be no-op)
    let noop_out = Command::new(get_bf_binary())
        .args(["label", "remove", &bead_id, "--label", "does-not-exist"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf label remove");
    assert!(noop_out.status.success());

    let labels = run_labels(workspace, &bead_id);
    assert_eq!(labels.len(), 2, "no-op remove should not change count");

    // 8. Update epic status and priority
    let update_out = Command::new(get_bf_binary())
        .args([
            "update",
            &bead_id,
            "--status",
            "in_progress",
            "--priority",
            "0",
        ])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf update");
    assert!(update_out.status.success());

    // 9. Verify epic type preserved through all operations
    let show_out = Command::new(get_bf_binary())
        .args(["show", &bead_id, "--format", "json"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");
    assert!(show_out.status.success());

    let json = String::from_utf8(show_out.stdout).unwrap();
    assert!(json.contains(r#""issue_type":"epic""#));
    assert!(json.contains(r#""status":"in_progress""#));
    assert!(json.contains(r#""priority":0"#));
    assert!(json.contains(r#""labels":["#));
}

#[test]
fn test_epic_label_search_integration() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create multiple epics with different labels
    let test_cases = vec![
        ("epic-search-1", "search-label-a"),
        ("epic-search-2", "search-label-b"),
        ("epic-search-3", "search-label-a"), // Same label as epic 1
    ];

    for (name, label) in &test_cases {
        let out = Command::new(get_bf_binary())
            .args([
                "create", "--title", name, "--type", "epic", "--label", label,
            ])
            .current_dir(workspace)
            .output()
            .expect("Failed to run bf create");
        assert!(out.status.success());
    }

    // Search by label
    let search_out = Command::new(get_bf_binary())
        .args(["search", "--label", "search-label-a", "--type", "epic"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf search");
    assert!(search_out.status.success());

    let search_output = String::from_utf8(search_out.stdout).unwrap();
    // Should find epic-search-1 and epic-search-3
    assert!(search_output.contains("epic-search-1"));
    assert!(search_output.contains("epic-search-3"));
}

#[test]
fn test_epic_label_list_command() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create multiple issues with various labels
    let test_cases = vec![
        ("epic-1", "epic", "label-a"),
        ("task-1", "task", "label-a"),
        ("task-2", "task", "label-b"),
    ];

    for (name, issue_type, label) in &test_cases {
        let out = Command::new(get_bf_binary())
            .args([
                "create", "--title", name, "--type", issue_type, "--label", label,
            ])
            .current_dir(workspace)
            .output()
            .expect("Failed to run bf create");
        assert!(out.status.success());
    }

    // List all labels
    let list_out = Command::new(get_bf_binary())
        .args(["label", "list"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf label list");
    assert!(list_out.status.success());

    let list_output = String::from_utf8(list_out.stdout).unwrap();
    // Should show label-a (count 2) and label-b (count 1)
    assert!(list_output.contains("label-a"));
    assert!(list_output.contains("label-b"));
}
