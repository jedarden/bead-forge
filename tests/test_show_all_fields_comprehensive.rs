//! Comprehensive test for verifying ALL fields are displayed in show command
//!
//! This test creates a bead with every possible field populated and verifies
//! that each field is properly displayed in all output formats (text, toon, json).
//! This ensures complete coverage of the show command's field display functionality.

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
fn test_show_displays_all_fields_text_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a bead and populate ALL possible fields
    let bead_id = create_test_bead(workspace, "Test all fields display");

    // Update with all optional fields
    let update_result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--description")
        .arg("Test description with complete details")
        .arg("--acceptance-criteria")
        .arg("AC 1: Must display all fields")
        .arg("--notes")
        .arg("Test notes for comprehensive verification")
        .arg("--design")
        .arg("Design reference document")
        .arg("--due-at")
        .arg("2026-12-31T23:59:59Z")
        .arg("--assignee")
        .arg("comprehensive-test-user")
        .current_dir(workspace)
        .output()
        .expect("Failed to update bead");

    assert!(
        update_result.status.success(),
        "bf update failed: {}",
        String::from_utf8_lossy(&update_result.stderr)
    );

    // Add labels
    let label_result = std::process::Command::new(&bf_path)
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("test-label")
        .arg("--label")
        .arg("comprehensive")
        .arg("--label")
        .arg("all-fields")
        .current_dir(workspace)
        .output()
        .expect("Failed to add labels");

    assert!(label_result.status.success());

    // Add annotations
    let annotate_result = std::process::Command::new(&bf_path)
        .arg("annotate")
        .arg("set")
        .arg(&bead_id)
        .arg("test-key")
        .arg("test-value")
        .current_dir(workspace)
        .output()
        .expect("Failed to set annotation");

    assert!(annotate_result.status.success());

    // Show the bead in default text format
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

    // Verify ALL standard fields are displayed
    // Core fields
    assert!(output.contains(&format!("ID: {}", bead_id)), "ID field must be displayed");
    assert!(output.contains("Title: Test all fields display"), "Title field must be displayed");
    assert!(output.contains("Status:"), "Status field must be displayed");
    assert!(output.contains("Priority:"), "Priority field must be displayed");
    assert!(output.contains("Type:"), "Type field must be displayed");

    // Optional fields
    assert!(
        output.contains("Description: Test description with complete details"),
        "Description field must be displayed"
    );
    assert!(
        output.contains("Assignee: comprehensive-test-user"),
        "Assignee field must be displayed"
    );

    // Labels
    assert!(output.contains("Labels:"), "Labels section must be displayed");
    assert!(output.contains("test-label"), "Label 'test-label' must be shown");
    assert!(output.contains("comprehensive"), "Label 'comprehensive' must be shown");
    assert!(output.contains("all-fields"), "Label 'all-fields' must be shown");

    // Annotations
    assert!(output.contains("Annotations:"), "Annotations section must be displayed");
    assert!(output.contains("test-key"), "Annotation key must be shown");
    assert!(output.contains("test-value"), "Annotation value must be shown");
}

#[test]
fn test_show_displays_all_fields_verbose_mode() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a bead with all fields
    let bead_id = create_test_bead(workspace, "Test verbose all fields");

    let update_result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--description")
        .arg("Verbose test description")
        .arg("--acceptance-criteria")
        .arg("AC: All verbose fields must display")
        .arg("--notes")
        .arg("Verbose notes")
        .arg("--design")
        .arg("Verbose design doc")
        .arg("--due-at")
        .arg("2026-12-31T23:59:59Z")
        .current_dir(workspace)
        .output()
        .expect("Failed to update bead");

    assert!(update_result.status.success());

    // Show with verbose flag
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .arg("--verbose")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show --verbose");

    assert!(show_result.status.success());

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Verbose output:\n{}", output);

    // Verify verbose-specific fields are displayed
    assert!(
        output.contains("Acceptance Criteria: AC: All verbose fields must display"),
        "Acceptance criteria must be shown in verbose mode"
    );
    assert!(
        output.contains("Notes: Verbose notes"),
        "Notes must be shown in verbose mode"
    );
    assert!(
        output.contains("Design: Verbose design doc"),
        "Design must be shown in verbose mode"
    );
    assert!(
        output.contains("Due at: 2026-12-31T23:59:59"),
        "Due at must be shown in verbose mode"
    );
    assert!(
        output.contains("Created at:"),
        "Created at timestamp must be shown in verbose mode"
    );
    assert!(
        output.contains("Updated at:"),
        "Updated at timestamp must be shown in verbose mode"
    );
}

#[test]
fn test_show_displays_all_fields_json_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a bead with all fields
    let bead_id = create_test_bead(workspace, "Test JSON all fields");

    let update_result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--description")
        .arg("JSON test description")
        .arg("--acceptance-criteria")
        .arg("AC: JSON must include all fields")
        .arg("--notes")
        .arg("JSON notes")
        .arg("--design")
        .arg("JSON design")
        .arg("--assignee")
        .arg("json-test-user")
        .current_dir(workspace)
        .output()
        .expect("Failed to update bead");

    assert!(update_result.status.success());

    // Add labels
    let label_result = std::process::Command::new(&bf_path)
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("json-label")
        .current_dir(workspace)
        .output()
        .expect("Failed to add labels");

    assert!(label_result.status.success());

    // Show in JSON format
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show --format json");

    assert!(show_result.status.success());

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("JSON output:\n{}", output);

    // Parse JSON
    let beads: Vec<serde_json::Value> =
        serde_json::from_str(&output).expect("Failed to parse JSON output");

    assert_eq!(beads.len(), 1, "Should return exactly one bead");
    let bead = &beads[0];

    // Verify ALL fields are present in JSON
    // Core identifier fields
    assert_eq!(bead["id"], bead_id);
    assert_eq!(bead["title"], "Test JSON all fields");

    // Status fields
    assert!(bead.get("status").is_some(), "status must be present");
    assert!(bead.get("priority").is_some(), "priority must be present");
    assert!(bead.get("issue_type").is_some(), "issue_type must be present");

    // Content fields
    assert_eq!(bead["description"], "JSON test description");
    assert_eq!(bead["assignee"], "json-test-user");
    assert_eq!(bead["acceptance_criteria"], "AC: JSON must include all fields");
    assert_eq!(bead["notes"], "JSON notes");
    assert_eq!(bead["design"], "JSON design");

    // Labels array
    assert!(bead["labels"].is_array(), "labels must be an array");
    let labels: Vec<&str> = bead["labels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap())
        .collect();
    assert!(labels.contains(&"json-label"), "label must be present");

    // Timestamps
    assert!(bead.get("created_at").is_some(), "created_at must be present");
    assert!(bead.get("updated_at").is_some(), "updated_at must be present");

    // Verify ISO 8601 format for timestamps
    let created_at = bead["created_at"].as_str().unwrap();
    assert!(created_at.contains('T'), "created_at should be in ISO 8601 format");

    let updated_at = bead["updated_at"].as_str().unwrap();
    assert!(updated_at.contains('T'), "updated_at should be in ISO 8601 format");
}

#[test]
fn test_show_displays_closed_bead_all_fields() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create, update, and close a bead
    let bead_id = create_test_bead(workspace, "Test closed all fields");

    let update_result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--description")
        .arg("Closed bead description")
        .arg("--assignee")
        .arg("closed-test-user")
        .current_dir(workspace)
        .output()
        .expect("Failed to update bead");

    assert!(update_result.status.success());

    let close_result = std::process::Command::new(&bf_path)
        .arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test completed successfully")
        .current_dir(workspace)
        .output()
        .expect("Failed to close bead");

    assert!(close_result.status.success());

    // Show in JSON format to check all fields including close_reason and closed_at
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(show_result.status.success());

    let output = String::from_utf8(show_result.stdout).unwrap();
    let beads: Vec<serde_json::Value> =
        serde_json::from_str(&output).expect("Failed to parse JSON");

    let bead = &beads[0];

    // Verify closed bead specific fields
    assert_eq!(bead["status"], "closed");
    assert_eq!(bead["close_reason"], "Test completed successfully");
    assert!(bead.get("closed_at").is_some(), "closed_at must be present for closed beads");

    // Verify other fields are still present
    assert_eq!(bead["description"], "Closed bead description");
    assert_eq!(bead["assignee"], "closed-test-user");

    // Verify closed_at timestamp format
    let closed_at = bead["closed_at"].as_str().unwrap();
    assert!(closed_at.contains('T'), "closed_at should be in ISO 8601 format");
}

#[test]
fn test_show_displays_all_fields_toon_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a bead with all fields
    let bead_id = create_test_bead(workspace, "Test toon all fields");

    let update_result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--description")
        .arg("Toon format description")
        .arg("--assignee")
        .arg("toon-test-user")
        .current_dir(workspace)
        .output()
        .expect("Failed to update bead");

    assert!(update_result.status.success());

    // Add labels
    let label_result = std::process::Command::new(&bf_path)
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("toon-label")
        .current_dir(workspace)
        .output()
        .expect("Failed to add labels");

    assert!(label_result.status.success());

    // Show in toon format
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .arg("--format")
        .arg("toon")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show --format toon");

    assert!(show_result.status.success());

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Toon output:\n{}", output);

    // Verify all fields are displayed in toon format
    assert!(output.contains(&format!("ID: {}", bead_id)));
    assert!(output.contains("Title: Test toon all fields"));
    assert!(output.contains("Status:"));
    assert!(output.contains("Priority:"));
    assert!(output.contains("Type:"));
    assert!(output.contains("Description: Toon format description"));
    assert!(output.contains("Assignee: toon-test-user"));
    assert!(output.contains("Labels:"));
    assert!(output.contains("toon-label"));
}
