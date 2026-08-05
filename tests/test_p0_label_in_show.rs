//! Integration test for P0 label in `bf show` output
//!
//! This test verifies that when a bead is created with a P0 label,
//! the label appears correctly in the `bf show` command output.
//! Tests both text and toon formats.

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

/// Test 1: Create bead with P0 label and verify it appears in `bf show` output (text format)
#[test]
fn test_p0_label_appears_in_show_text_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with P0 label
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Test bead with P0 label",
            "--label",
            "P0",
        ],
    );
    assert!(success, "bf create failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Show the bead in text format
    let (show_stdout, show_stderr, show_success) = run_bf_command(workspace, &["show", &bead_id]);
    assert!(show_success, "bf show failed: {}", show_stderr);

    // Verify label appears in output with correct format
    assert!(show_stdout.contains("Labels: P0"), "Expected 'Labels: P0' in show output");
    assert!(show_stdout.contains("Test bead with P0 label"), "Expected title in show output");
}

/// Test 2: Create bead with P0 label and verify it appears in `bf show` output (toon format)
#[test]
fn test_p0_label_appears_in_show_toon_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with P0 label
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Test bead with P0 label for toon",
            "--label",
            "P0",
        ],
    );
    assert!(success, "bf create failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Show the bead in toon format
    let (show_stdout, show_stderr, show_success) =
        run_bf_command(workspace, &["show", &bead_id, "--format", "toon"]);
    assert!(show_success, "bf show failed: {}", show_stderr);

    // Verify label appears in output with correct format
    assert!(show_stdout.contains("Labels: P0"), "Expected 'Labels: P0' in show output");
    assert!(
        show_stdout.contains("Test bead with P0 label for toon"),
        "Expected title in show output"
    );
}

/// Test 3: Create bead with P0 label and verify in JSON format
#[test]
fn test_p0_label_appears_in_show_json_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with P0 label
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Test bead with P0 label for JSON",
            "--label",
            "P0",
        ],
    );
    assert!(success, "bf create failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Show the bead in JSON format
    let (show_stdout, show_stderr, show_success) =
        run_bf_command(workspace, &["show", &bead_id, "--format", "json"]);
    assert!(show_success, "bf show failed: {}", show_stderr);

    // Parse JSON and verify labels array contains P0
    let json: serde_json::Value = serde_json::from_str(&show_stdout).unwrap();
    let issue = &json[0];

    // Verify labels array exists and contains P0
    assert!(
        issue["labels"].is_array(),
        "Expected labels to be an array in JSON output"
    );

    let labels = issue["labels"]
        .as_array()
        .expect("Labels should be an array");

    assert!(
        labels.iter().any(|l| l.as_str() == Some("P0")),
        "Expected labels array to contain 'P0'"
    );
}

/// Test 4: Create bead with P0 label alongside other labels
#[test]
fn test_p0_label_with_other_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with P0 label and other labels
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Test bead with multiple labels including P0",
            "--label",
            "P0",
            "--label",
            "urgent",
            "--label",
            "backend",
        ],
    );
    assert!(success, "bf create failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Show the bead
    let (show_stdout, show_stderr, show_success) = run_bf_command(workspace, &["show", &bead_id]);
    assert!(show_success, "bf show failed: {}", show_stderr);

    // Verify all labels appear including P0
    assert!(show_stdout.contains("Labels:"), "Expected 'Labels:' in show output");
    assert!(show_stdout.contains("P0"), "Expected 'P0' label in show output");
    assert!(show_stdout.contains("urgent"), "Expected 'urgent' label in show output");
    assert!(show_stdout.contains("backend"), "Expected 'backend' label in show output");

    // Verify labels are joined by comma in format: "Labels: P0, urgent, backend"
    assert!(
        show_stdout.contains("Labels: ") && show_stdout.contains("P0") && show_stdout.contains(","),
        "Expected comma-separated labels"
    );
}

/// Test 5: Verify exact label format "Labels: P0" for single P0 label
#[test]
fn test_p0_label_exact_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with only P0 label
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &["create", "--title", "Single P0 label test", "--label", "P0"],
    );
    assert!(success, "bf create failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Show the bead in text format
    let (show_stdout, show_stderr, show_success) = run_bf_command(workspace, &["show", &bead_id]);
    assert!(show_success, "bf show failed: {}", show_stderr);

    // Verify exact format: "Labels: P0" (not "Label:" or "Labels:P0")
    assert!(
        show_stdout.contains("Labels: P0"),
        "Expected exact format 'Labels: P0' in show output, got: {}",
        show_stdout
    );

    // Make sure there's no trailing comma for single label
    assert!(
        !show_stdout.contains("Labels: P0,"),
        "Should not have trailing comma for single label"
    );
}

/// Test 6: Create bead with P0 label via CLI, then verify with `bf labels` command
#[test]
fn test_p0_label_via_labels_command() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with P0 label
    let (stdout, stderr, success) =
        run_bf_command(workspace, &["create", "--title", "P0 label test", "--label", "P0"]);
    assert!(success, "bf create failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Use `bf labels` command to verify
    let (labels_stdout, labels_stderr, labels_success) =
        run_bf_command(workspace, &["labels", &bead_id]);
    assert!(labels_success, "bf labels failed: {}", labels_stderr);

    // Verify P0 appears in labels output
    assert!(labels_stdout.contains("P0"), "Expected 'P0' in labels command output");
}

/// Test 7: P0 label appears in correct position in show output
#[test]
fn test_p0_label_position_in_show_output() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Create bead with P0 label
    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "P0 label position test",
            "--label",
            "P0",
            "--description",
            "Test description for position check",
        ],
    );
    assert!(success, "bf create failed: {}", stderr);
    let bead_id = extract_bead_id(&stdout);

    // Show the bead
    let (show_stdout, show_stderr, show_success) = run_bf_command(workspace, &["show", &bead_id]);
    assert!(show_success, "bf show failed: {}", show_stderr);

    // Verify expected order: ID, Title, Status, Priority, Type, Description, Updated, Labels
    let lines: Vec<&str> = show_stdout.lines().collect();

    // Find positions
    let id_pos = lines.iter().position(|l| l.starts_with("ID:"));
    let title_pos = lines.iter().position(|l| l.starts_with("Title:"));
    let labels_pos = lines.iter().position(|l| l.starts_with("Labels:"));

    assert!(id_pos.is_some(), "Expected 'ID:' line in output");
    assert!(title_pos.is_some(), "Expected 'Title:' line in output");
    assert!(labels_pos.is_some(), "Expected 'Labels:' line in output");

    // Labels should come after ID and Title
    assert!(
        labels_pos > id_pos && labels_pos > title_pos,
        "Labels should appear after ID and Title in output"
    );

    // Verify the specific label line
    let labels_line = lines[labels_pos.unwrap()];
    assert_eq!(labels_line, "Labels: P0", "Expected exact labels line");
}
