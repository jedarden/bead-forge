//! P0 Label CLI Parsing Tests
//!
//! Unit tests for CLI parsing layer for P0 labels.
//! Tests that the label flag is correctly parsed and attached to beads
//! through the create and label-add commands.

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
    // Handle envelope format if present
    if let Ok(envelope) = serde_json::from_str::<serde_json::Value>(output) {
        if envelope.is_object() && envelope.get("data").is_some() {
            return envelope["data"].clone();
        }
    }
    serde_json::from_str(output).expect("Failed to parse JSON output")
}

/// Extract labels from JSON output
fn extract_labels_from_json(json: &serde_json::Value) -> Vec<String> {
    json["labels"]
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
// Test 1: bf create --label P0 parses correctly
// ============================================================================

#[test]
fn test_create_with_p0_label_cli() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with P0 label
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &["create", "--title", "P0 Label Test", "--label", "P0"],
    );
    assert!(success, "bf create with --label P0 failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Verify the P0 label was parsed and stored via JSON show
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);
    let labels = extract_labels_from_json(&json);

    // Test: P0 label exists
    assert!(
        labels.contains(&"P0".to_string()),
        "P0 label should be present in bead labels, got: {:?}",
        labels
    );

    // Test: Verify the bead was created successfully
    assert_eq!(json["id"].as_str(), Some(bead_id.as_str()));
}

// ============================================================================
// Test 2: bf create with multiple labels including P0
// ============================================================================

#[test]
fn test_create_with_multiple_labels_including_p0() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with P0 and other labels
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Multiple Labels Test",
            "--label",
            "P0",
            "--label",
            "urgent",
            "--label",
            "backend",
        ],
    );
    assert!(success, "bf create with multiple labels failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Verify all labels including P0 were parsed
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);
    let labels = extract_labels_from_json(&json);

    assert_eq!(labels.len(), 3, "Should have 3 labels");
    assert!(labels.contains(&"P0".to_string()), "Should contain P0 label");
    assert!(labels.contains(&"urgent".to_string()), "Should contain urgent label");
    assert!(labels.contains(&"backend".to_string()), "Should contain backend label");
}

// ============================================================================
// Test 3: bf label add <id> --label P0 parses correctly
// ============================================================================

#[test]
fn test_label_add_p0_to_existing_bead() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead without labels
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "Label Add P0 Test", "--type", "task"],
    );
    let bead_id = extract_bead_id(&stdout);

    // Add P0 label via label add command
    let (_, stderr, success) =
        run_bf_command(workspace, &["label", "add", &bead_id, "--label", "P0"]);
    assert!(success, "bf label add --label P0 failed: {}", stderr);

    // Verify P0 label was added via JSON
    let (labels_stdout, _, _) =
        run_bf_command(workspace, &["labels", &bead_id, "--format", "json"]);
    let labels: Vec<String> = serde_json::from_str(&labels_stdout).unwrap();

    assert_eq!(labels.len(), 1, "Should have 1 label");
    assert_eq!(labels[0], "P0", "Label should be P0");
}

// ============================================================================
// Test 4: P0 label appears in bf show text output
// ============================================================================

#[test]
fn test_p0_label_in_show_text_output() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with P0 label
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "Show Text P0 Test", "--label", "P0"],
    );
    let bead_id = extract_bead_id(&stdout);

    // Show in text format (default)
    let (show_stdout, stderr, success) = run_bf_command(workspace, &["show", &bead_id]);
    assert!(success, "bf show failed: {}", stderr);

    // Test: P0 label appears in text output
    assert!(
        show_stdout.contains("Labels:"),
        "Show output should contain 'Labels:' header"
    );
    assert!(
        show_stdout.contains("P0"),
        "Show output should contain P0 label text"
    );
}

// ============================================================================
// Test 5: P0 label appears in bf show json output
// ============================================================================

#[test]
fn test_p0_label_in_show_json_output() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with P0 label
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "Show JSON P0 Test", "--label", "P0"],
    );
    let bead_id = extract_bead_id(&stdout);

    // Show in JSON format
    let (show_json, stderr, success) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    assert!(success, "bf show --format json failed: {}", stderr);

    let json = parse_json_output(&show_json);
    let labels = extract_labels_from_json(&json);

    // Test: P0 label appears in JSON output
    assert_eq!(labels.len(), 1, "Should have 1 label in JSON");
    assert_eq!(labels[0], "P0", "JSON label value should be P0");

    // Verify JSON structure
    assert!(json.is_object(), "Show output should be a JSON object");
    assert!(json["labels"].is_array(), "labels field should be an array");
}

// ============================================================================
// Test 6: Multiple P0 labels are deduplicated
// ============================================================================

#[test]
fn test_duplicate_p0_labels_deduplicated() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with duplicate P0 labels
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Duplicate P0 Test",
            "--label",
            "P0",
            "--label",
            "P0",
            "--label",
            "P0",
        ],
    );
    assert!(success, "bf create with duplicate P0 labels failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Verify only one P0 label exists (deduplication)
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);
    let labels = extract_labels_from_json(&json);

    assert_eq!(labels.len(), 1, "Duplicate P0 labels should be deduplicated to 1");
    assert_eq!(labels[0], "P0", "Single label should be P0");
}

// ============================================================================
// Test 7: P0 label with --json flag on create
// ============================================================================

#[test]
fn test_p0_label_with_create_json_output() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with P0 label and JSON output
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &["create", "--title", "JSON Create P0", "--label", "P0", "--json"],
    );
    assert!(success, "bf create --json with P0 label failed: {}", stderr);

    let json = parse_json_output(&stdout);
    let labels = extract_labels_from_json(&json);

    // Test: P0 label in create --json output
    assert_eq!(labels.len(), 1, "Should have 1 label in create JSON output");
    assert_eq!(labels[0], "P0", "Create JSON output should contain P0 label");
    assert!(json["id"].is_string(), "Create JSON output should have id field");
}

// ============================================================================
// Test 8: P0 label can be added via -l short flag
// ============================================================================

#[test]
fn test_p0_label_via_short_flag() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with P0 label via short flag -l
    let (stdout, stderr, success) =
        run_bf_command(workspace, &["create", "--title", "Short Flag P0", "-l", "P0"]);
    assert!(success, "bf create -l P0 failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Verify P0 label was parsed correctly via short flag
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);
    let labels = extract_labels_from_json(&json);

    assert_eq!(labels.len(), 1, "Should have 1 label");
    assert_eq!(labels[0], "P0", "Label should be P0");
}

// ============================================================================
// Test 9: P0 label add via -l short flag
// ============================================================================

#[test]
fn test_p0_label_add_via_short_flag() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead without labels
    let (stdout, _, _) =
        run_bf_command(workspace, &["create", "--title", "Label Add Short Flag"]);
    let bead_id = extract_bead_id(&stdout);

    // Add P0 label via short flag
    let (_, stderr, success) =
        run_bf_command(workspace, &["label", "add", &bead_id, "-l", "P0"]);
    assert!(success, "bf label add -l P0 failed: {}", stderr);

    // Verify P0 label was added
    let (labels_stdout, _, _) =
        run_bf_command(workspace, &["labels", &bead_id, "--format", "json"]);
    let labels: Vec<String> = serde_json::from_str(&labels_stdout).unwrap();

    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0], "P0");
}

// ============================================================================
// Test 10: P0 label persistence after operations
// ============================================================================

#[test]
fn test_p0_label_persistence() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with P0 label
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "P0 Persistence", "--label", "P0"],
    );
    let bead_id = extract_bead_id(&stdout);

    // Add another label
    let (_, _, success) =
        run_bf_command(workspace, &["label", "add", &bead_id, "-l", "secondary"]);
    assert!(success, "Adding secondary label failed");

    // Verify both labels including P0 persist
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);
    let labels = extract_labels_from_json(&json);

    assert_eq!(labels.len(), 2, "Should have 2 labels after add");
    assert!(labels.contains(&"P0".to_string()), "P0 label should persist");
    assert!(labels.contains(&"secondary".to_string()), "Secondary label should be present");
}

// ============================================================================
// Test 11: P0 label is treated as a regular label (not priority)
// ============================================================================

#[test]
fn test_p0_label_vs_p0_priority() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with P0 label but P1 priority
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 Label P1 Priority",
            "--label",
            "P0",
            "--priority",
            "1",
        ],
    );
    assert!(success, "bf create with P0 label and P1 priority failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Verify: P0 label is stored as a label
    let (show_stdout, _, _) = run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    let json = parse_json_output(&show_stdout);
    let labels = extract_labels_from_json(&json);

    assert!(labels.contains(&"P0".to_string()), "P0 should be in labels");

    // Verify: Priority is P1 (1), not P0 (0)
    let priority = json["priority"].as_i64().unwrap();
    assert_eq!(priority, 1, "Priority should be 1 (P1), not 0 (P0)");

    // This confirms P0 label is independent from priority level
}

// ============================================================================
// Test 12: P0 label in list output
// ============================================================================

#[test]
fn test_p0_label_in_list_output() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with P0 label
    let (stdout, _, _) = run_bf_command(
        workspace,
        &["create", "--title", "P0 List Test", "--label", "P0"],
    );
    let bead_id = extract_bead_id(&stdout);

    // List beads with labels in text format
    let (list_stdout, stderr, success) = run_bf_command(workspace, &["labels"]);
    assert!(success, "bf labels failed: {}", stderr);

    // Test: P0 label appears in list output
    assert!(
        list_stdout.contains("P0"),
        "List output should contain P0 label"
    );
    assert!(
        list_stdout.contains(&bead_id),
        "List output should reference the bead with P0 label"
    );
}
