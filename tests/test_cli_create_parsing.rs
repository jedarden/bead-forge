//! Unit tests for bf create CLI argument parsing
//!
//! Tests that the CLI correctly parses arguments for the `bf create` command,
//! specifically focusing on the multi-label functionality.

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

/// Parse JSON output from bf create --json
fn parse_create_json(output: &str) -> serde_json::Value {
    // The output is wrapped in an envelope, so we need to parse the data field
    let json: serde_json::Value = serde_json::from_str(output).unwrap();
    json["data"].clone()
}

/// Test: Verify parsing with 0 labels
#[test]
fn test_create_parsing_zero_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &["create", "--title", "No labels", "--json"],
    );
    assert!(success, "bf create failed: {}", stderr);

    let data = parse_create_json(&stdout);
    let labels = data["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 0, "Expected 0 labels, got {}", labels.len());
}

/// Test: Verify parsing with 1 label
#[test]
fn test_create_parsing_single_label() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &["create", "--title", "Single label", "--label", "urgent", "--json"],
    );
    assert!(success, "bf create failed: {}", stderr);

    let data = parse_create_json(&stdout);
    let labels = data["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 1, "Expected 1 label, got {}", labels.len());
    assert_eq!(labels[0].as_str().unwrap(), "urgent");
}

/// Test: Verify parsing with 2 labels
#[test]
fn test_create_parsing_two_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Two labels",
            "--label",
            "frontend",
            "--label",
            "ui",
            "--json",
        ],
    );
    assert!(success, "bf create failed: {}", stderr);

    let data = parse_create_json(&stdout);
    let labels = data["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 2, "Expected 2 labels, got {}", labels.len());
    assert_eq!(labels[0].as_str().unwrap(), "frontend");
    assert_eq!(labels[1].as_str().unwrap(), "ui");
}

/// Test: Verify parsing with 3+ labels
#[test]
fn test_create_parsing_three_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Three labels",
            "--label",
            "backend",
            "--label",
            "api",
            "--label",
            "high-priority",
            "--json",
        ],
    );
    assert!(success, "bf create failed: {}", stderr);

    let data = parse_create_json(&stdout);
    let labels = data["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 3, "Expected 3 labels, got {}", labels.len());
    assert_eq!(labels[0].as_str().unwrap(), "backend");
    assert_eq!(labels[1].as_str().unwrap(), "api");
    assert_eq!(labels[2].as_str().unwrap(), "high-priority");
}

/// Test: Verify parsing with many labels (5+)
#[test]
fn test_create_parsing_many_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let mut args = vec![
        "create",
        "--title",
        "Many labels",
        "--label",
        "l1",
        "--label",
        "l2",
        "--label",
        "l3",
        "--label",
        "l4",
        "--label",
        "l5",
        "--json",
    ];

    let (stdout, stderr, success) = run_bf_command(workspace, &args);
    assert!(success, "bf create failed: {}", stderr);

    let data = parse_create_json(&stdout);
    let labels = data["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 5, "Expected 5 labels, got {}", labels.len());

    let expected_labels = vec!["l1", "l2", "l3", "l4", "l5"];
    for (i, expected) in expected_labels.iter().enumerate() {
        assert_eq!(labels[i].as_str().unwrap(), *expected);
    }
}

/// Test: Verify labels are stored in order
#[test]
fn test_create_parsing_label_order_preserved() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Label order",
            "--label",
            "zebra",
            "--label",
            "alpha",
            "--label",
            "beta",
            "--json",
        ],
    );
    assert!(success, "bf create failed: {}", stderr);

    let data = parse_create_json(&stdout);
    let labels = data["labels"].as_array().unwrap();

    // Verify order is preserved (not sorted)
    assert_eq!(labels[0].as_str().unwrap(), "zebra");
    assert_eq!(labels[1].as_str().unwrap(), "alpha");
    assert_eq!(labels[2].as_str().unwrap(), "beta");
}

/// Test: Verify labels with special characters
#[test]
fn test_create_parsing_label_special_chars() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let (stdout, stderr, success) = run_bf_command(
        workspace,
        &[
            "create",
            "--title",
            "Special chars",
            "--label",
            "phase-1",
            "--label",
            "p2/backend",
            "--label",
            "high_priority",
            "--json",
        ],
    );
    assert!(success, "bf create failed: {}", stderr);

    let data = parse_create_json(&stdout);
    let labels = data["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 3);
    assert_eq!(labels[0].as_str().unwrap(), "phase-1");
    assert_eq!(labels[1].as_str().unwrap(), "p2/backend");
    assert_eq!(labels[2].as_str().unwrap(), "high_priority");
}
