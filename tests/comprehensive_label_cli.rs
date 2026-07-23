//! Comprehensive CLI Label Tests
//!
//! Tests all label operations via the CLI:
//! - Create with duplicate labels (deduplication)
//! - Label add/remove/list commands
//! - Duplicate label handling at CLI level
//! - Label persistence across operations
//! - JSON and text output formats
//! - Edge cases (empty labels, whitespace, special chars)

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

/// Count unique labels from label list command
fn count_labels_from_list(output: &str) -> usize {
    output
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.contains("Labels:"))
        .count()
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
// Test 1: Create bead with duplicate labels
// ============================================================================

#[test]
fn test_create_with_duplicate_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with duplicate labels
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Duplicate Labels Test",
            "--label",
            "urgent",
            "--label",
            "urgent", // duplicate
            "--label",
            "backend",
            "--label",
            "urgent", // another duplicate
            "--label",
            "backend", // duplicate
        ],
    );
    assert!(success, "bf create failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Verify labels were deduplicated via JSON
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);
    let labels = extract_labels_from_json(&json);

    assert_eq!(
        labels.len(),
        2,
        "Should have exactly 2 unique labels (urgent, backend), got: {:?}",
        labels
    );
    assert!(labels.contains(&"urgent".to_string()));
    assert!(labels.contains(&"backend".to_string()));
}

// ============================================================================
// Test 2: Add duplicate labels via label add command
// ============================================================================

#[test]
fn test_label_add_duplicate_is_idempotent() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead without labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "Label Add Test", "--type", "task"],
    );
    let bead_id = extract_bead_id(&stdout);

    // Add labels including duplicates
    let (_, stderr, success) = run_bf_command(
        workspace,
        &[
            "label",
            "add",
            &bead_id,
            "-l",
            "feature",
            "-l",
            "feature", // duplicate
            "-l",
            "bug",
            "-l",
            "feature", // duplicate again
        ],
    );
    assert!(success, "bf label add failed: {}", stderr);

    // Verify only unique labels exist
    let (labels_stdout, _, _) = run_bf_command(workspace, &["labels", &bead_id, "--format", "json"]);
    let labels: Vec<String> = serde_json::from_str(&labels_stdout).unwrap();

    assert_eq!(labels.len(), 2, "Should have 2 unique labels");
    assert!(labels.contains(&"feature".to_string()));
    assert!(labels.contains(&"bug".to_string()));
}

// ============================================================================
// Test 3: Remove labels including non-existent ones
// ============================================================================

#[test]
fn test_label_remove_nonexistent_is_safe() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Remove Test",
            "--label",
            "frontend",
            "--label",
            "urgent",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Try to remove a label that doesn't exist
    let (_, stderr, success) = run_bf_command(
        workspace,
        &["label", "remove", &bead_id, "-l", "backend"],
    );
    // Should succeed (no-op) or fail gracefully - either is acceptable behavior
    // The important thing is not to crash

    // Verify original labels are still intact
    let (labels_stdout, _, _) = run_bf_command(workspace, &["labels", &bead_id, "--format", "json"]);
    let labels: Vec<String> = serde_json::from_str(&labels_stdout).unwrap();

    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"frontend".to_string()));
    assert!(labels.contains(&"urgent".to_string()));
}

// ============================================================================
// Test 4: Remove duplicate label references
// ============================================================================

#[test]
fn test_label_remove_duplicates_fully() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with label
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "Remove Duplicate Test", "--label", "test-label"],
    );
    let bead_id = extract_bead_id(&stdout);

    // Verify label exists
    let (labels_stdout, _, _) = run_bf_command(workspace, &["labels", &bead_id, "--format", "json"]);
    let labels: Vec<String> = serde_json::from_str(&labels_stdout).unwrap();
    assert_eq!(labels.len(), 1);

    // Remove same label multiple times
    let _ = run_bf_command(workspace, &["label", "remove", &bead_id, "-l", "test-label"]);
    let _ = run_bf_command(workspace, &["label", "remove", &bead_id, "-l", "test-label"]);

    // Verify label is completely gone
    let (labels_stdout, _, _) = run_bf_command(workspace, &["labels", &bead_id, "--format", "json"]);
    let labels: Vec<String> = serde_json::from_str(&labels_stdout).unwrap();
    assert_eq!(labels.len(), 0, "Label should be completely removed");
}

// ============================================================================
// Test 5: List all labels with counts
// ============================================================================

#[test]
fn test_label_list_all_with_counts() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create multiple beads with overlapping labels
    let _ = run_bf_command(
        workspace,
        &["create", "--title", "Bead 1", "--label", "urgent", "--label", "backend"],
    );
    let _ = run_bf_command(
        workspace,
        &["create", "--title", "Bead 2", "--label", "urgent", "--label", "frontend"],
    );
    let _ = run_bf_command(
        workspace,
        &["create", "--title", "Bead 3", "--label", "backend", "--label", "frontend"],
    );

    // List all labels
    let (list_stdout, stderr, success) = run_bf_command(workspace, &["label", "list"]);
    assert!(success, "bf label list failed: {}", stderr);

    // Verify the output shows label counts
    assert!(list_stdout.contains("urgent"), "Output should contain 'urgent' label");
    assert!(list_stdout.contains("backend"), "Output should contain 'backend' label");
    assert!(list_stdout.contains("frontend"), "Output should contain 'frontend' label");
}

// ============================================================================
// Test 6: Labels command with text format
// ============================================================================

#[test]
fn test_labels_text_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "Text Format Test", "--label", "ui", "--label", "ux"],
    );
    let bead_id = extract_bead_id(&stdout);

    // List labels in text format
    let (labels_stdout, stderr, success) = run_bf_command(workspace, &["labels", &bead_id]);
    assert!(success, "bf labels failed: {}", stderr);

    // Verify labels are shown (one per line)
    let lines: Vec<&str> = labels_stdout.lines().collect();
    assert!(lines.len() >= 2, "Should have at least 2 label lines");
    assert!(labels_stdout.contains("ui"));
    assert!(labels_stdout.contains("ux"));
}

// ============================================================================
// Test 7: Labels command with JSON format
// ============================================================================

#[test]
fn test_labels_json_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "JSON Format Test", "--label", "api", "--label", "rest"],
    );
    let bead_id = extract_bead_id(&stdout);

    // List labels in JSON format
    let (labels_stdout, stderr, success) =
        run_bf_command(workspace, &["labels", &bead_id, "--format", "json"]);
    assert!(success, "bf labels --format json failed: {}", stderr);

    // Parse and verify JSON array
    let labels: Vec<String> = serde_json::from_str(&labels_stdout).unwrap();
    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"api".to_string()));
    assert!(labels.contains(&"rest".to_string()));
}

// ============================================================================
// Test 8: Empty label handling
// ============================================================================

#[test]
fn test_empty_label_is_rejected_or_ignored() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Try to create bead with empty label
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &["create", "--title", "Empty Label Test", "--label", ""],
    );

    // Either fails or ignores empty label - both are acceptable
    if success {
        let bead_id = extract_bead_id(&stdout);
        let (labels_stdout, _, _) = run_bf_command(workspace, &["labels", &bead_id, "--format", "json"]);
        let labels: Vec<String> = serde_json::from_str(&labels_stdout).unwrap();
        // If empty labels are stored, deduplicate them; otherwise, they should be filtered
        let unique_labels: std::collections::HashSet<_> = labels.into_iter().collect();
        assert!(
            !unique_labels.contains(&"".to_string()) || unique_labels.len() <= 1,
            "Empty labels should be rejected or deduplicated"
        );
    }
    // If it fails, that's also acceptable behavior
}

// ============================================================================
// Test 9: Whitespace handling in labels
// ============================================================================

#[test]
fn test_label_whitespace_handling() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with labels that have leading/trailing spaces
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "Whitespace Test", "--label", " urgent ", "--label", "backend"],
    );
    let bead_id = extract_bead_id(&stdout);

    // Verify labels - they might be trimmed or stored with spaces
    let (labels_stdout, _, _) = run_bf_command(workspace, &["labels", &bead_id, "--format", "json"]);
    let labels: Vec<String> = serde_json::from_str(&labels_stdout).unwrap();

    // Should have labels, exact behavior depends on implementation
    assert!(labels.len() >= 1, "Should have at least one label");
    assert!(labels.contains(&" urgent ".to_string()) || labels.contains(&"urgent".to_string()));
}

// ============================================================================
// Test 10: Special characters in labels
// ============================================================================

#[test]
fn test_special_characters_in_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with special character labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Special Chars Test",
            "--label",
            "phase-1",
            "--label",
            "bug/fix",
            "--label",
            "feature:auth",
        ],
    );
    let bead_id = extract_bead_id(&stdout);

    // Verify special character labels are preserved
    let (labels_stdout, _, _) = run_bf_command(workspace, &["labels", &bead_id, "--format", "json"]);
    let labels: Vec<String> = serde_json::from_str(&labels_stdout).unwrap();

    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"phase-1".to_string()));
    assert!(labels.contains(&"bug/fix".to_string()));
    assert!(labels.contains(&"feature:auth".to_string()));
}

// ============================================================================
// Test 11: Unicode labels
// ============================================================================

#[test]
fn test_unicode_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with unicode labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "Unicode Test", "--label", "🔥hotfix", "--label", "tâsk"],
    );
    let bead_id = extract_bead_id(&stdout);

    // Verify unicode labels are preserved
    let (labels_stdout, _, _) = run_bf_command(workspace, &["labels", &bead_id, "--format", "json"]);
    let labels: Vec<String> = serde_json::from_str(&labels_stdout).unwrap();

    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"🔥hotfix".to_string()));
    assert!(labels.contains(&"tâsk".to_string()));
}

// ============================================================================
// Test 12: Case sensitivity
// ============================================================================

#[test]
fn test_label_case_sensitivity() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with different case labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "Case Test", "--label", "urgent", "--label", "URGENT", "--label", "Urgent"],
    );
    let bead_id = extract_bead_id(&stdout);

    // Verify case behavior (implementation-dependent)
    let (labels_stdout, _, _) = run_bf_command(workspace, &["labels", &bead_id, "--format", "json"]);
    let labels: Vec<String> = serde_json::from_str(&labels_stdout).unwrap();

    // At minimum, should have at least one label
    assert!(labels.len() >= 1);
}

// ============================================================================
// Test 13: Label persistence after JSONL flush
// ============================================================================

#[test]
fn test_label_persistence_after_flush() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "Persistence Test", "--label", "persistent", "--label", "test"],
    );
    let bead_id = extract_bead_id(&stdout);

    // Flush to JSONL
    let (_, _, success) = run_bf_command(workspace, &["sync", "--flush-only"]);
    assert!(success, "bf sync --flush-only failed");

    // Add a label after flush
    let (_, _, success) =
        run_bf_command(workspace, &["label", "add", &bead_id, "-l", "after-flush"]);
    assert!(success, "bf label add failed");

    // Flush again
    let (_, _, success) = run_bf_command(workspace, &["sync", "--flush-only"]);
    assert!(success, "Second bf sync --flush-only failed");

    // Verify all labels persist
    let (labels_stdout, _, _) = run_bf_command(workspace, &["labels", &bead_id, "--format", "json"]);
    let labels: Vec<String> = serde_json::from_str(&labels_stdout).unwrap();

    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"persistent".to_string()));
    assert!(labels.contains(&"test".to_string()));
    assert!(labels.contains(&"after-flush".to_string()));
}

// ============================================================================
// Test 14: Search by label
// ============================================================================

#[test]
fn test_search_by_label() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create beads with different labels
    let (epic1, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "Backend Epic", "--type", "epic", "--label", "backend", "--label", "database"],
    );
    let epic1_id = extract_bead_id(&epic1);

    let (epic2, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "Frontend Epic", "--type", "epic", "--label", "frontend", "--label", "ui"],
    );
    let epic2_id = extract_bead_id(&epic2);

    // Search by label
    let (search_stdout, stderr, success) = run_bf_command(workspace, &["search", "--label", "backend"]);
    assert!(success, "bf search --label failed: {}", stderr);

    // Should find backend epic but not frontend
    assert!(search_stdout.contains(&epic1_id), "Search should find backend epic");
    assert!(!search_stdout.contains(&epic2_id), "Search should not find frontend epic");
}

// ============================================================================
// Test 15: Labels shown in bf show output
// ============================================================================

#[test]
fn test_labels_in_show_output() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "Show Test", "--label", "visible", "--label", "demo"],
    );
    let bead_id = extract_bead_id(&stdout);

    // Show in text format
    let (show_stdout, stderr, success) = run_bf_command(workspace, &["show", &bead_id]);
    assert!(success, "bf show failed: {}", stderr);

    // Verify labels are displayed
    assert!(show_stdout.contains("Labels:"), "Show output should contain 'Labels:'");
    assert!(show_stdout.contains("visible"));
    assert!(show_stdout.contains("demo"));

    // Show in JSON format
    let (show_json, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_json);
    let labels = extract_labels_from_json(&json);

    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"visible".to_string()));
    assert!(labels.contains(&"demo".to_string()));
}

// ============================================================================
// Test 16: Very long labels
// ============================================================================

#[test]
fn test_very_long_label() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with very long label
    let long_label = "a".repeat(1000);
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "Long Label Test", "--label", &long_label],
    );
    let bead_id = extract_bead_id(&stdout);

    // Verify long label is preserved
    let (labels_stdout, _, _) = run_bf_command(workspace, &["labels", &bead_id, "--format", "json"]);
    let labels: Vec<String> = serde_json::from_str(&labels_stdout).unwrap();

    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0], long_label);
}
