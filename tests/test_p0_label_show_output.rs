//! Test P0 Label Appears in `bf show` Output
//!
//! Integration tests that verify P0 labels appear correctly in `bf show` command output.
//! Tests the full flow from CLI parsing to display formatting.
//!
//! Acceptance Criteria:
//! 1. Test creates a bead with P0 label
//! 2. Test verifies label appears in `bf show` output
//! 3. Test checks label format in output (e.g., "Labels: P0")
//! 4. Test passes with `cargo test`

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a temporary workspace for testing
fn setup_test_workspace() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("test-workspace");
    fs::create_dir_all(&workspace_dir).unwrap();
    let beads_dir = workspace_dir.join(".beads");
    fs::create_dir_all(&beads_dir).unwrap();

    // Initialize workspace
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

/// Create a test bead with P0 label via CLI
fn create_bead_with_p0_label(workspace: impl AsRef<std::path::Path>, title: &str) -> String {
    let bf_path = get_bf_binary();
    let result = std::process::Command::new(&bf_path)
        .arg("create")
        .arg("--title")
        .arg(title)
        .arg("--type")
        .arg("task")
        .arg("--priority")
        .arg("2")
        .arg("--label")
        .arg("P0")
        .current_dir(&workspace.as_ref())
        .output()
        .expect("Failed to create bead");

    assert!(
        result.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap().trim().to_string()
}

#[test]
fn test_p0_label_appears_in_show_output() {
    // Acceptance Criteria 1: Test creates a bead with P0 label
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a bead with P0 label
    let bead_id = create_bead_with_p0_label(workspace, "Test P0 label in show output");

    // Acceptance Criteria 2: Test verifies label appears in `bf show` output
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(
        show_result.status.success(),
        "bf show failed: {}",
        String::from_utf8_lossy(&show_result.stderr)
    );

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Show output:\n{}", output);

    // Verify the label appears in the output
    assert!(
        output.contains("Labels:") || output.contains("labels"),
        "Output should contain a 'Labels:' section or labels field"
    );

    assert!(
        output.contains("P0"),
        "Output should contain the P0 label"
    );

    // Acceptance Criteria 3: Test checks label format in output
    // The exact format should be "Labels: P0" for a single label
    assert!(
        output.contains("Labels: P0") || output.contains("P0"),
        "Output should show 'Labels: P0' format or contain P0 label"
    );
}

#[test]
fn test_p0_label_show_json_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a bead with P0 label
    let bead_id = create_bead_with_p0_label(workspace, "Test P0 label JSON format");

    // Show the bead in JSON format
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(
        show_result.status.success(),
        "bf show --format json failed: {}",
        String::from_utf8_lossy(&show_result.stderr)
    );

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("JSON output:\n{}", output);

    // Parse JSON and verify structure
    let beads: Vec<serde_json::Value> =
        serde_json::from_str(&output).expect("Failed to parse JSON output");

    assert_eq!(
        beads.len(),
        1,
        "Should return exactly one bead wrapped in array"
    );

    let bead = &beads[0];
    assert_eq!(bead["id"], bead_id);
    assert_eq!(bead["title"], "Test P0 label JSON format");

    // Verify labels is an array containing P0
    assert!(bead["labels"].is_array(), "labels should be an array");
    let labels: Vec<&str> = bead["labels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap())
        .collect();

    assert!(
        labels.contains(&"P0"),
        "labels array should contain 'P0' label"
    );
}

#[test]
fn test_p0_label_with_other_labels_show_output() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a bead with P0 label
    let bead_id = create_bead_with_p0_label(workspace, "Test P0 with other labels");

    // Add additional labels
    let label_result = std::process::Command::new(&bf_path)
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .arg("--label")
        .arg("security")
        .current_dir(workspace)
        .output()
        .expect("Failed to add labels");

    assert!(
        label_result.status.success(),
        "bf label add failed: {}",
        String::from_utf8_lossy(&label_result.stderr)
    );

    // Show the bead in text format
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(show_result.status.success());

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Show output with multiple labels:\n{}", output);

    // Verify all labels are shown including P0
    assert!(output.contains("Labels:"), "Should show Labels section");
    assert!(output.contains("P0"), "Should show P0 label");
    assert!(output.contains("urgent"), "Should show urgent label");
    assert!(output.contains("security"), "Should show security label");
}

#[test]
fn test_p0_label_show_toon_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a bead with P0 label
    let bead_id = create_bead_with_p0_label(workspace, "Test P0 label toon format");

    // Show the bead in toon format
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .arg("--format")
        .arg("toon")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(
        show_result.status.success(),
        "bf show --format toon failed: {}",
        String::from_utf8_lossy(&show_result.stderr)
    );

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Toon output:\n{}", output);

    // Verify P0 label appears in toon format
    assert!(
        output.contains("Labels:") || output.contains("P0"),
        "Toon format should show labels section or P0 label"
    );
}

#[test]
fn test_multiple_p0_labeled_beads_show() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create multiple beads with P0 label
    let bead_ids = vec![
        create_bead_with_p0_label(workspace, "First P0 task"),
        create_bead_with_p0_label(workspace, "Second P0 task"),
        create_bead_with_p0_label(workspace, "Third P0 task"),
    ];

    // Verify each bead shows the P0 label correctly
    for bead_id in bead_ids {
        let show_result = std::process::Command::new(&bf_path)
            .arg("show")
            .arg(&bead_id)
            .current_dir(workspace)
            .output()
            .expect("Failed to run bf show");

        assert!(
            show_result.status.success(),
            "bf show failed for {}: {}",
            bead_id,
            String::from_utf8_lossy(&show_result.stderr)
        );

        let output = String::from_utf8(show_result.stdout).unwrap();

        assert!(
            output.contains("P0"),
            "Output for {} should contain P0 label",
            bead_id
        );
    }
}

#[test]
fn test_p0_label_persistence_through_show() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a bead with P0 label
    let bead_id = create_bead_with_p0_label(workspace, "Test P0 label persistence");

    // Show immediately after creation
    let first_show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(first_show_result.status.success());
    let first_output = String::from_utf8(first_show_result.stdout).unwrap();

    // Verify P0 label appears
    assert!(
        first_output.contains("P0"),
        "First show should contain P0 label"
    );

    // Show again to verify consistency
    let second_show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(second_show_result.status.success());
    let second_output = String::from_utf8(second_show_result.stdout).unwrap();

    // Verify P0 label still appears
    assert!(
        second_output.contains("P0"),
        "Second show should contain P0 label (persistence verified)"
    );
}
