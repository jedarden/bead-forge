//! Test assignee field display in show command
//!
//! Verifies that the assignee field is properly displayed in:
//! - Text format (default)
//! - Toon format
//! - JSON format
//! - Handles None/empty cases correctly

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

/// Create a test bead via CLI
fn create_test_bead(workspace: impl AsRef<std::path::Path>, title: &str) -> String {
    let bf_path = get_bf_binary();
    let result = std::process::Command::new(&bf_path)
        .arg("create")
        .arg("--title")
        .arg(title)
        .arg("--type")
        .arg("task")
        .arg("--priority")
        .arg("2")
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
fn test_show_assignee_text_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a bead with assignee
    let bead_id = create_test_bead(workspace, "Test assignee text format");

    let update_result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--assignee")
        .arg("test-assignee-user")
        .current_dir(workspace)
        .output()
        .expect("Failed to update bead");

    assert!(
        update_result.status.success(),
        "bf update failed: {}",
        String::from_utf8_lossy(&update_result.stderr)
    );

    // Show in default text format
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
    println!("Text format output:\n{}", output);

    // Verify assignee is displayed
    assert!(
        output.contains("Assignee: test-assignee-user"),
        "Output should contain assignee field in text format"
    );
}

#[test]
fn test_show_assignee_toon_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a bead with assignee
    let bead_id = create_test_bead(workspace, "Test assignee toon format");

    let update_result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--assignee")
        .arg("another-test-user")
        .current_dir(workspace)
        .output()
        .expect("Failed to update bead");

    assert!(
        update_result.status.success(),
        "bf update failed: {}",
        String::from_utf8_lossy(&update_result.stderr)
    );

    // Show in toon format
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
        "bf show failed: {}",
        String::from_utf8_lossy(&show_result.stderr)
    );

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Toon format output:\n{}", output);

    // Verify assignee is displayed
    assert!(
        output.contains("Assignee: another-test-user"),
        "Output should contain assignee field in toon format"
    );
}

#[test]
fn test_show_assignee_json_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a bead with assignee
    let bead_id = create_test_bead(workspace, "Test assignee JSON format");

    let update_result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--assignee")
        .arg("json-test-user")
        .current_dir(workspace)
        .output()
        .expect("Failed to update bead");

    assert!(
        update_result.status.success(),
        "bf update failed: {}",
        String::from_utf8_lossy(&update_result.stderr)
    );

    // Show in JSON format
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
        "bf show failed: {}",
        String::from_utf8_lossy(&show_result.stderr)
    );

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("JSON format output:\n{}", output);

    // Parse and verify assignee is in JSON
    let beads: Vec<serde_json::Value> =
        serde_json::from_str(&output).expect("Failed to parse JSON output");

    assert_eq!(beads.len(), 1);
    let bead = &beads[0];
    assert_eq!(bead["assignee"], "json-test-user");
}

#[test]
fn test_show_assignee_none_case() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a bead without assignee
    let bead_id = create_test_bead(workspace, "Test no assignee");

    // Show in text format
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
    println!("No assignee output:\n{}", output);

    // Verify assignee line is NOT present when None
    assert!(
        !output.contains("Assignee:"),
        "Output should NOT contain Assignee line when assignee is None"
    );

    // Verify in JSON format as well
    let show_json_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    let json_output = String::from_utf8(show_json_result.stdout).unwrap();
    let beads: Vec<serde_json::Value> =
        serde_json::from_str(&json_output).expect("Failed to parse JSON");

    let bead = &beads[0];
    // When None, assignee should either be absent or null
    assert!(
        bead.get("assignee").is_none() || bead["assignee"].is_null(),
        "JSON should not have assignee field or it should be null when None"
    );
}

#[test]
fn test_show_assignee_cleared_via_update() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a bead with assignee
    let bead_id = create_test_bead(workspace, "Test assignee clear");

    let update_result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--assignee")
        .arg("original-assignee")
        .current_dir(workspace)
        .output()
        .expect("Failed to update bead");

    assert!(update_result.status.success());

    // Clear assignee using --clear-assignee
    let clear_result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--clear-assignee")
        .current_dir(workspace)
        .output()
        .expect("Failed to clear assignee");

    assert!(clear_result.status.success());

    // Show and verify assignee is not present
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("After clearing assignee:\n{}", output);

    assert!(
        !output.contains("Assignee:"),
        "Output should NOT contain Assignee line after clearing"
    );
}
