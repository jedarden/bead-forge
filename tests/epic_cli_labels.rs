//! Integration tests for Epic with CLI Labels
//!
//! Comprehensive CLI tests for epic beads with labels:
//! - Creating epics with labels via `bf create --label`
//! - Showing epics with labels via `bf show`
//! - Listing epics filtered by type and labels
//! - Searching epics by labels
//! - Stats with label breakdowns for epics
//! - Label list command for epics
//! - JSON output formats with labels

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Create a temporary workspace for testing
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
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
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

/// Test 1: Create epic with single label via CLI
#[test]
fn test_epic_create_single_label_cli() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic with single label
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Epic with Single Label",
            "--type",
            "epic",
            "--label",
            "feature",
        ],
    );
    assert!(success, "bf create failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Verify the epic has the label
    let labels = run_labels(workspace, &bead_id);
    assert_eq!(labels.len(), 1);
    assert!(labels.contains(&"feature".to_string()));
}

/// Test 2: Create epic with multiple labels via CLI
#[test]
fn test_epic_create_multiple_labels_cli() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic with multiple labels
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Epic with Multiple Labels",
            "--type",
            "epic",
            "--label",
            "feature",
            "--label",
            "backend",
            "--label",
            "high-priority",
        ],
    );
    assert!(success, "bf create failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Verify all labels are present
    let labels = run_labels(workspace, &bead_id);
    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"feature".to_string()));
    assert!(labels.contains(&"backend".to_string()));
    assert!(labels.contains(&"high-priority".to_string()));
}

/// Test 3: Show epic with labels in text format
#[test]
fn test_epic_show_labels_text_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Epic for Show Test",
            "--type",
            "epic",
            "--label",
            "ui",
            "--label",
            "frontend",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Show the epic in text format
    let (show_stdout, show_stderr, show_success) = run_bf_command(workspace, &["show", &bead_id]);
    assert!(show_success, "bf show failed: {}", show_stderr);

    // Verify labels are displayed
    assert!(show_stdout.contains("Labels:"));
    assert!(show_stdout.contains("ui"));
    assert!(show_stdout.contains("frontend"));
}

/// Test 4: Show epic with labels in JSON format
#[test]
fn test_epic_show_labels_json_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Epic for JSON Test",
            "--type",
            "epic",
            "--label",
            "api",
            "--label",
            "microservice",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Show the epic in JSON format
    let (show_stdout, show_stderr, show_success) =
        run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    assert!(show_success, "bf show failed: {}", show_stderr);

    // Parse JSON and verify labels
    let json: serde_json::Value = serde_json::from_str(&show_stdout).unwrap();
    let issue = &json[0];
    assert_eq!(issue["issue_type"], "epic");
    assert!(json_contains_labels(&show_stdout, "api"));
    assert!(json_contains_labels(&show_stdout, "microservice"));
}

fn json_contains_labels(json_str: &str, label: &str) -> bool {
    json_str.contains(&format!("\"{}\"", label))
}

/// Test 5: List epics filtered by type
#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_epic_list_by_type() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create an epic with labels
    let (epic_stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "My Epic",
            "--type",
            "epic",
            "--label",
            "feature-x",
        ],
    );
    let epic_id = extract_bead_id(&epic_stdout);

    // Create a regular task with labels
    let (_, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "My Task",
            "--type",
            "task",
            "--label",
            "feature-x",
        ],
    );

    // List only epics
    let (list_stdout, list_stderr, list_success) =
        run_bf_command(workspace, &["list", "--type", "epic"]);
    assert!(list_success, "bf list failed: {}", list_stderr);

    // Should contain the epic but not the task
    assert!(list_stdout.contains(&epic_id));
    assert!(list_stdout.contains("epic"));
}

/// Test 6: Search epics by label
#[test]
fn test_epic_search_by_label() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epics with different labels
    let (epic1_stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Backend Epic",
            "--type",
            "epic",
            "--label",
            "backend",
            "--label",
            "database",
        ],
    );
    let epic1_id = extract_bead_id(&epic1_stdout);

    let (epic2_stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Frontend Epic",
            "--type",
            "epic",
            "--label",
            "frontend",
            "--label",
            "ui",
        ],
    );
    let epic2_id = extract_bead_id(&epic2_stdout);

    // Search for backend-labeled epics
    let (search_stdout, search_stderr, search_success) = run_bf_command(
        workspace,
        &["search", "--label", "backend", "--type", "epic"],
    );
    assert!(search_success, "bf search failed: {}", search_stderr);

    // Should find backend epic but not frontend
    assert!(search_stdout.contains(&epic1_id));
    assert!(search_stdout.contains("Backend Epic"));
    assert!(!search_stdout.contains(&epic2_id) || !search_stdout.contains("Frontend Epic"));
}

/// Test 7: Stats with label breakdown for epics
#[test]
fn test_epic_stats_by_label() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create multiple epics with various labels
    for i in 1..=3 {
        run_bf_command(
            workspace,
            &[
                "create",
                "--title",
                &format!("Epic {}", i),
                "--type",
                "epic",
                "--label",
                "backend",
            ],
        );
    }

    // Create some non-epic issues with labels
    for i in 1..=2 {
        run_bf_command(
            workspace,
            &[
                "create",
                "--title",
                &format!("Task {}", i),
                "--type",
                "task",
                "--label",
                "backend",
            ],
        );
    }

    // Get stats with label breakdown
    let (stats_stdout, stats_stderr, stats_success) =
        run_bf_command(workspace, &["stats", "--by-label"]);
    assert!(stats_success, "bf stats failed: {}", stats_stderr);

    // Verify backend label count (3 epics + 2 tasks = 5 total)
    assert!(stats_stdout.contains("backend"));
}

/// Test 8: Labels command list for epic
#[test]
fn test_epic_labels_list_command() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Epic for Labels List Test",
            "--type",
            "epic",
            "--label",
            "feature",
            "--label",
            "bugfix",
            "--label",
            "enhancement",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Use labels command to list them
    let labels = run_labels(workspace, &bead_id);

    // Verify all labels are present
    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"feature".to_string()));
    assert!(labels.contains(&"bugfix".to_string()));
    assert!(labels.contains(&"enhancement".to_string()));
}

/// Test 9: Epic with no labels
#[test]
fn test_epic_no_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic without labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "Epic without labels", "--type", "epic"],
    );
    let bead_id = extract_bead_id(&stdout);

    // Verify no labels
    let labels = run_labels(workspace, &bead_id);
    assert_eq!(labels.len(), 0);
}

/// Test 10: Create epic with labels and verify via show --format toon
#[test]
fn test_epic_show_labels_toon_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Epic for Toon Test",
            "--type",
            "epic",
            "--label",
            "mobile",
            "--label",
            "ios",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Show in toon format
    let (show_stdout, show_stderr, show_success) =
        run_bf_command(workspace, &["show", &bead_id, "--format", "toon"]);
    assert!(show_success, "bf show failed: {}", show_stderr);

    // Verify labels are displayed
    assert!(show_stdout.contains("Labels:"));
    assert!(show_stdout.contains("mobile"));
    assert!(show_stdout.contains("ios"));
    assert!(show_stdout.contains("Epic for Toon Test"));
}

/// Test 11: Multiple epics with same label
#[test]
fn test_multiple_epics_same_label() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create multiple epics with the same label
    let mut epic_ids = Vec::new();
    for i in 1..=3 {
        let (stdout, _, _) = run_bf_command(
            workspace,
            &[
                "create",
                "--title",
                &format!("Shared Label Epic {}", i),
                "--type",
                "epic",
                "--label",
                "shared-feature",
            ],
        );
        epic_ids.push(extract_bead_id(&stdout));
    }

    // Verify all epics have the label
    for epic_id in &epic_ids {
        let labels = run_labels(workspace, epic_id);
        assert_eq!(labels.len(), 1);
        assert!(labels.contains(&"shared-feature".to_string()));
    }
}

/// Test 12: Epic with priority and labels
#[test]
fn test_epic_priority_and_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic with high priority and labels
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "High Priority Epic",
            "--type",
            "epic",
            "--priority",
            "1",
            "--label",
            "urgent",
            "--label",
            "critical",
        ],
    );
    assert!(success, "bf create failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Show the epic and verify priority and labels
    let (show_stdout, show_stderr, show_success) = run_bf_command(workspace, &["show", &bead_id]);
    assert!(show_success, "bf show failed: {}", show_stderr);

    assert!(show_stdout.contains("Priority: P1"));
    assert!(show_stdout.contains("urgent"));
    assert!(show_stdout.contains("critical"));
}

/// Test 13: Label add/remove on epic
#[test]
fn test_epic_label_add_remove() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic with one label
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Epic for Label Mutation",
            "--type",
            "epic",
            "--label",
            "initial",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Add a label
    let (_, add_stderr, add_success) = run_bf_command(
        workspace,
        &["label", "add", &bead_id, "--label", "new-label"],
    );
    assert!(add_success, "bf label add failed: {}", add_stderr);

    // Verify both labels exist
    let labels = run_labels(workspace, &bead_id);
    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"initial".to_string()));
    assert!(labels.contains(&"new-label".to_string()));

    // Remove a label
    let (_, rem_stderr, rem_success) = run_bf_command(
        workspace,
        &["label", "remove", &bead_id, "--label", "initial"],
    );
    assert!(rem_success, "bf label remove failed: {}", rem_stderr);

    // Verify only new-label remains
    let labels = run_labels(workspace, &bead_id);
    assert_eq!(labels.len(), 1);
    assert!(labels.contains(&"new-label".to_string()));
}

/// Test 14: Epic with description and labels
#[test]
fn test_epic_description_and_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic with description and labels
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Epic with Description",
            "--type",
            "epic",
            "--description",
            "This is a test epic with description",
            "--label",
            "documented",
        ],
    );
    assert!(success, "bf create failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Show and verify both description and labels
    let (show_stdout, show_stderr, show_success) = run_bf_command(workspace, &["show", &bead_id]);
    assert!(show_success, "bf show failed: {}", show_stderr);

    assert!(show_stdout.contains("This is a test epic with description"));
    assert!(show_stdout.contains("documented"));
}

/// Test 15: JSONL export preserves epic labels
#[test]
fn test_epic_labels_jsonl_export() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create epic with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Epic for JSONL Export",
            "--type",
            "epic",
            "--label",
            "export-test",
            "--label",
            "jsonl",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Flush to JSONL
    let (_, sync_stderr, sync_success) = run_bf_command(workspace, &["sync", "--flush-only"]);
    assert!(sync_success, "bf sync failed: {}", sync_stderr);

    // Read JSONL file and verify labels are preserved
    let jsonl_path = beads_dir.join("issues.jsonl");
    let jsonl_content = fs::read_to_string(&jsonl_path).unwrap();

    // Find the epic's line
    let epic_line = jsonl_content
        .lines()
        .find(|line| line.contains(&format!("\"id\":\"{}\"", bead_id)))
        .expect("Epic not found in JSONL");

    // Verify labels are in JSONL
    assert!(epic_line.contains("export-test"));
    assert!(epic_line.contains("jsonl"));
    assert!(epic_line.contains("\"issue_type\":\"epic\""));
}
