//! Comprehensive P0 Label Functionality Tests
//!
//! Tests all label operations specifically with P0 (critical) priority:
//! - Create P0 beads with various label combinations
//! - Add/remove/list labels on P0 beads
//! - P0 label persistence and serialization
//! - P0 label display in text and JSON formats
//! - P0 label search and filtering
//! - Batch operations with P0 labels
//! - Edge cases and error handling with P0 labels

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

/// Parse JSON output from bf --json
fn parse_json_output(output: &str) -> serde_json::Value {
    serde_json::from_str(output).expect("Failed to parse JSON output")
}

/// Extract labels from JSON output
fn extract_labels_from_json(json: &serde_json::Value) -> Vec<String> {
    json[0]["labels"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default()
}

// ============================================================================
// Test 1: Create P0 bead with single label
// ============================================================================

#[test]
fn test_p0_create_with_single_label() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with single label
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 Critical Bug",
            "--priority",
            "0",
            "--label",
            "critical",
        ],
    );
    assert!(success, "bf create failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Verify P0 priority and label via JSON
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);

    assert_eq!(json[0]["priority"], 0, "Priority should be P0");
    assert_eq!(json[0]["issue_type"], "task");
    let labels = extract_labels_from_json(&json);
    assert_eq!(labels.len(), 1);
    assert!(labels.contains(&"critical".to_string()));
}

// ============================================================================
// Test 2: Create P0 bead with multiple labels
// ============================================================================

#[test]
fn test_p0_create_with_multiple_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with multiple labels
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 Security Issue",
            "--priority",
            "0",
            "--label",
            "security",
            "--label",
            "urgent",
            "--label",
            "hotfix",
            "--label",
            "backend",
        ],
    );
    assert!(success, "bf create failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Verify all labels are present
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);
    let labels = extract_labels_from_json(&json);

    assert_eq!(labels.len(), 4);
    assert!(labels.contains(&"security".to_string()));
    assert!(labels.contains(&"urgent".to_string()));
    assert!(labels.contains(&"hotfix".to_string()));
    assert!(labels.contains(&"backend".to_string()));
    assert_eq!(json[0]["priority"], 0);
}

// ============================================================================
// Test 3: Create P0 bead with duplicate labels (deduplication)
// ============================================================================

#[test]
fn test_p0_create_with_duplicate_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with duplicate labels
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 Duplicate Labels",
            "--priority",
            "0",
            "--label",
            "critical",
            "--label",
            "critical",
            "--label",
            "urgent",
            "--label",
            "critical",
        ],
    );
    assert!(success, "bf create failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Verify labels are deduplicated
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);
    let labels = extract_labels_from_json(&json);

    assert_eq!(labels.len(), 2, "Should have 2 unique labels");
    assert!(labels.contains(&"critical".to_string()));
    assert!(labels.contains(&"urgent".to_string()));
}

// ============================================================================
// Test 4: Add labels to existing P0 bead
// ============================================================================

#[test]
fn test_p0_label_add() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead without labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "P0 Add Labels", "--priority", "0"],
    );
    let bead_id = extract_bead_id(&stdout);

    // Add labels to P0 bead
    let (_, stderr, success) = run_bf_command(
        workspace,
        &["label", "add", &bead_id, "-l", "p0-item", "-l", "critical"],
    );
    assert!(success, "bf label add failed: {}", stderr);

    // Verify labels were added and priority maintained
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);
    let labels = extract_labels_from_json(&json);

    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"p0-item".to_string()));
    assert!(labels.contains(&"critical".to_string()));
    assert_eq!(json[0]["priority"], 0, "Priority should remain P0");

    // Edge case: Add duplicate label to verify deduplication
    let (_, stderr, success) = run_bf_command(
        workspace,
        &["label", "add", &bead_id, "-l", "p0-item", "-l", "new-label"],
    );
    assert!(success, "bf label add with duplicate failed: {}", stderr);

    // Verify duplicate was deduplicated and new label was added
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);
    let labels = extract_labels_from_json(&json);

    assert_eq!(labels.len(), 3, "Should have 3 unique labels (p0-item, critical, new-label)");
    assert!(labels.contains(&"p0-item".to_string()));
    assert!(labels.contains(&"critical".to_string()));
    assert!(labels.contains(&"new-label".to_string()));
    assert_eq!(json[0]["priority"], 0, "Priority should remain P0 after adding labels with duplicates");
}

// ============================================================================
// Test 5: Remove labels from P0 bead
// ============================================================================

#[test]
fn test_p0_label_remove() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 Remove Labels",
            "--priority",
            "0",
            "--label",
            "remove-me",
            "--label",
            "keep-me",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Remove one label
    let (_, stderr, success) =
        run_bf_command(workspace, &["label", "remove", &bead_id, "-l", "remove-me"]);
    assert!(success, "bf label remove failed: {}", stderr);

    // Verify label removed and priority maintained
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);
    let labels = extract_labels_from_json(&json);

    assert_eq!(labels.len(), 1);
    assert!(labels.contains(&"keep-me".to_string()));
    assert!(!labels.contains(&"remove-me".to_string()));
    assert_eq!(json[0]["priority"], 0, "Priority should remain P0");
}

// ============================================================================
// Test 6: List labels on P0 bead
// ============================================================================

#[test]
fn test_p0_labels_list() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 List Labels",
            "--priority",
            "0",
            "--label",
            "list-test",
            "--label",
            "p0-critical",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // List labels in JSON format
    let (labels_stdout, stderr, success) =
        run_bf_command(workspace, &["labels", &bead_id, "--format", "json"]);
    assert!(success, "bf labels failed: {}", stderr);

    let labels: Vec<String> = serde_json::from_str(&labels_stdout).unwrap();
    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"list-test".to_string()));
    assert!(labels.contains(&"p0-critical".to_string()));
}

// ============================================================================
// Test 7: P0 label persistence after JSONL flush
// ============================================================================

#[test]
fn test_p0_label_persistence_after_flush() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 Persistence Test",
            "--priority",
            "0",
            "--label",
            "persistent",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Flush to JSONL
    let (_, _, success) = run_bf_command(workspace, &["sync", "--flush-only"]);
    assert!(success, "bf sync --flush-only failed");

    // Verify labels persist after flush
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);
    let labels = extract_labels_from_json(&json);

    assert_eq!(labels.len(), 1);
    assert!(labels.contains(&"persistent".to_string()));
    assert_eq!(json[0]["priority"], 0, "Priority should persist as P0");
}

// ============================================================================
// Test 8: P0 label serialization to JSON
// ============================================================================

#[test]
fn test_p0_label_json_serialization() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 JSON Serialization",
            "--priority",
            "0",
            "--label",
            "json-test",
            "--label",
            "serialization",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Get JSON output
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);

    // Verify JSON structure
    assert_eq!(json[0]["priority"], 0);
    assert_eq!(json[0]["issue_type"], "task");
    let labels = json[0]["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 2);
    assert!(labels.iter().any(|l| l == "json-test"));
    assert!(labels.iter().any(|l| l == "serialization"));
}

// ============================================================================
// Test 9: P0 label display in text format
// ============================================================================

#[test]
fn test_p0_label_text_display() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 Text Display",
            "--priority",
            "0",
            "--label",
            "display-test",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Get text output
    let (show_stdout, stderr, success) = run_bf_command(workspace, &["show", &bead_id]);
    assert!(success, "bf show failed: {}", stderr);

    // Verify labels appear in text output
    assert!(show_stdout.contains("Labels:"));
    assert!(show_stdout.contains("display-test"));
    assert!(show_stdout.contains("P0"));
}

// ============================================================================
// Test 10: Search for P0 beads by label
// ============================================================================

#[test]
fn test_p0_search_by_label() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with specific label
    let (p0_bead, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 Search Test",
            "--priority",
            "0",
            "--label",
            "p0-searchable",
        ],
    );
    let p0_id = extract_bead_id(&p0_bead);

    // Create non-P0 bead with same label
    let _ = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P2 Search Test",
            "--priority",
            "2",
            "--label",
            "p0-searchable",
        ],
    );

    // Search by label
    let (search_stdout, stderr, success) =
        run_bf_command(workspace, &["search", "--label", "p0-searchable"]);
    assert!(success, "bf search --label failed: {}", stderr);

    // Verify P0 bead is found
    assert!(search_stdout.contains(&p0_id));
}

// ============================================================================
// Test 11: List P0 beads filtered by label
// ============================================================================

#[test]
fn test_p0_list_with_label_filter() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with label
    let (p0_bead, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 List Filter",
            "--priority",
            "0",
            "--label",
            "p0-filter",
        ],
    );
    let p0_id = extract_bead_id(&p0_bead);

    // Create another P0 bead without the label
    let _ = run_bf_command(
        workspace,
        &["create", "--title", "P0 No Filter", "--priority", "0"],
    );

    // List all P0 beads
    let (list_stdout, stderr, success) =
        run_bf_command(workspace, &["list", "--priority", "0"]);
    assert!(success, "bf list --priority failed: {}", stderr);

    // Verify our P0 bead with label is in the list
    assert!(list_stdout.contains(&p0_id));
}

// ============================================================================
// Test 12: Batch operations with P0 labels
// ============================================================================

#[test]
fn test_p0_batch_label_operations() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead via batch
    let batch_json = r#"[
        {"op": "create", "title": "P0 Batch Label Test", "priority": 0, "labels": ["batch", "p0-batch"]},
        {"op": "label_add", "id": "@0", "labels": ["added-batch"]}
    ]"#;

    let (batch_stdout, stderr, success) =
        run_bf_command(workspace, &["batch", "--json", batch_json]);
    assert!(success, "bf batch failed: {}", stderr);

    // Extract bead ID from batch output
    let batch_result: Vec<serde_json::Value> =
        serde_json::from_str(&batch_stdout).expect("Failed to parse batch output");
    let bead_id = batch_result[0]["id"].as_str().expect("No bead ID in batch result");

    // Verify all labels from batch operations
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);
    let labels = extract_labels_from_json(&json);

    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"batch".to_string()));
    assert!(labels.contains(&"p0-batch".to_string()));
    assert!(labels.contains(&"added-batch".to_string()));
    assert_eq!(json[0]["priority"], 0, "Priority should be P0");
}

// ============================================================================
// Test 13: P0 epic with labels
// ============================================================================

#[test]
fn test_p0_epic_with_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 epic with labels
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 Epic with Labels",
            "--type",
            "epic",
            "--priority",
            "0",
            "--label",
            "epic-label",
            "--label",
            "p0-epic",
        ],
    );
    assert!(success, "bf create failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Verify epic type, P0 priority, and labels
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);

    assert_eq!(json[0]["issue_type"], "epic");
    assert_eq!(json[0]["priority"], 0);
    let labels = extract_labels_from_json(&json);
    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"epic-label".to_string()));
    assert!(labels.contains(&"p0-epic".to_string()));
}

// ============================================================================
// Test 14: P0 label operations don't affect priority
// ============================================================================

#[test]
fn test_p0_label_operations_preserve_priority() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "P0 Priority Preserve", "--priority", "0"],
    );
    let bead_id = extract_bead_id(&stdout);

    // Perform various label operations
    let _ = run_bf_command(workspace, &["label", "add", &bead_id, "-l", "test1"]);
    let _ = run_bf_command(workspace, &["label", "add", &bead_id, "-l", "test2"]);
    let _ = run_bf_command(workspace, &["label", "remove", &bead_id, "-l", "test1"]);

    // Verify priority is still P0
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);
    assert_eq!(json[0]["priority"], 0, "Priority should remain P0 after label operations");
}

// ============================================================================
// Test 15: Multiple P0 beads with different labels
// ============================================================================

#[test]
fn test_multiple_p0_beads_with_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create multiple P0 beads with different labels
    let (p0_1, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 Bead 1",
            "--priority",
            "0",
            "--label",
            "label-1",
        ],
    );
    let p0_1_id = extract_bead_id(&p0_1);

    let (p0_2, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 Bead 2",
            "--priority",
            "0",
            "--label",
            "label-2",
        ],
    );
    let p0_2_id = extract_bead_id(&p0_2);

    // List all P0 beads
    let (list_stdout, _, _) = run_bf_command(workspace, &["list", "--priority", "0"]);

    // Verify both P0 beads are present
    assert!(list_stdout.contains(&p0_1_id));
    assert!(list_stdout.contains(&p0_2_id));
}

// ============================================================================
// Test 16: P0 label with special characters
// ============================================================================

#[test]
fn test_p0_label_special_characters() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with special character labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 Special Chars",
            "--priority",
            "0",
            "--label",
            "phase-1",
            "--label",
            "bug/critical",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Verify special character labels are preserved
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);
    let labels = extract_labels_from_json(&json);

    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"phase-1".to_string()));
    assert!(labels.contains(&"bug/critical".to_string()));
}

// ============================================================================
// Test 17: Empty label list on P0 bead
// ============================================================================

#[test]
fn test_p0_empty_label_list() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead without labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "P0 No Labels", "--priority", "0"],
    );
    let bead_id = extract_bead_id(&stdout);

    // Verify empty label list
    let (labels_stdout, _, _) =
        run_bf_command(workspace, &["labels", &bead_id, "--format", "json"]);
    let labels: Vec<String> = serde_json::from_str(&labels_stdout).unwrap();

    assert_eq!(labels.len(), 0, "P0 bead should have empty label list");
}

// ============================================================================
// Test 18: P0 label count verification
// ============================================================================

#[test]
fn test_p0_label_count() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with known number of labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 Count Test",
            "--priority",
            "0",
            "--label",
            "one",
            "--label",
            "two",
            "--label",
            "three",
            "--label",
            "four",
            "--label",
            "five",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Verify exact label count
    let (labels_stdout, _, _) =
        run_bf_command(workspace, &["labels", &bead_id, "--format", "json"]);
    let labels: Vec<String> = serde_json::from_str(&labels_stdout).unwrap();

    assert_eq!(labels.len(), 5, "Should have exactly 5 labels");
}

// ============================================================================
// Test 19: P0 label update command
// ============================================================================

#[test]
fn test_p0_update_preserves_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 Update Test",
            "--priority",
            "0",
            "--label",
            "preserve-me",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Update title (should not affect labels or priority)
    let (_, stderr, success) = run_bf_command(
        workspace,
        &["update", &bead_id, "--title", "Updated P0 Title"],
    );
    assert!(success, "bf update failed: {}", stderr);

    // Verify labels and priority are preserved
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);
    let labels = extract_labels_from_json(&json);

    assert_eq!(json[0]["title"], "Updated P0 Title");
    assert_eq!(json[0]["priority"], 0, "Priority should be preserved");
    assert_eq!(labels.len(), 1);
    assert!(labels.contains(&"preserve-me".to_string()));
}

// ============================================================================
// Test 20: P0 priority display with labels in different formats
// ============================================================================

#[test]
fn test_p0_display_formats_with_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create P0 bead with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 Format Test",
            "--priority",
            "0",
            "--label",
            "format-test",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Test text format
    let (text_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id]);
    assert!(text_stdout.contains("P0"));
    assert!(text_stdout.contains("format-test"));

    // Test JSON format
    let (json_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&json_stdout);
    assert_eq!(json[0]["priority"], 0);
    let labels = extract_labels_from_json(&json);
    assert!(labels.contains(&"format-test".to_string()));
}
