//! Integration tests for `bf show` command
//!
//! Tests the show command functionality including:
//! - Basic show in text format
//! - Show in JSON format
//! - Show in toon format
//! - Show with missing bead (error handling)
//! - Show bead with all fields populated
//! - Show bead with dependencies
//! - Show bead with labels

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

    assert!(result.status.success(), "bf create failed: {}", String::from_utf8_lossy(&result.stderr));
    String::from_utf8(result.stdout).unwrap().trim().to_string()
}

#[test]
fn test_show_basic_text_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a test bead
    let bead_id = create_test_bead(workspace, "Test show command");

    // Show the bead in default text format
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(show_result.status.success(), "bf show failed: {}", String::from_utf8_lossy(&show_result.stderr));

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Show output:\n{}", output);

    // Verify basic fields are present
    assert!(output.contains(&format!("ID: {}", bead_id)), "Output should contain bead ID");
    assert!(output.contains("Title: Test show command"), "Output should contain title");
    assert!(output.contains("Status:"), "Output should contain status");
    assert!(output.contains("Priority:"), "Output should contain priority");
    assert!(output.contains("Type:"), "Output should contain type");
}

#[test]
fn test_show_json_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a test bead with additional fields
    let bead_id = create_test_bead(workspace, "Test JSON format");

    // Update with additional fields
    let update_result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--description")
        .arg("Test description")
        .arg("--assignee")
        .arg("test-user")
        .current_dir(workspace)
        .output()
        .expect("Failed to update bead");

    assert!(update_result.status.success(), "bf update failed: {}", String::from_utf8_lossy(&update_result.stderr));

    // Add label using separate command
    let label_result = std::process::Command::new(&bf_path)
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("test-label")
        .current_dir(workspace)
        .output()
        .expect("Failed to add label");

    assert!(label_result.status.success(), "bf label add failed: {}", String::from_utf8_lossy(&label_result.stderr));

    // Show the bead in JSON format
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(show_result.status.success(), "bf show failed: {}", String::from_utf8_lossy(&show_result.stderr));

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("JSON output:\n{}", output);

    // Parse JSON and verify structure
    let beads: Vec<serde_json::Value> = serde_json::from_str(&output)
        .expect("Failed to parse JSON output");

    assert_eq!(beads.len(), 1, "Should return exactly one bead wrapped in array");

    let bead = &beads[0];
    assert_eq!(bead["id"], bead_id);
    assert_eq!(bead["title"], "Test JSON format");
    assert_eq!(bead["description"], "Test description");
    assert_eq!(bead["assignee"], "test-user");

    // Verify labels is an array
    assert!(bead["labels"].is_array());
    let labels: Vec<&str> = bead["labels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap())
        .collect();
    assert!(labels.contains(&"test-label"), "Should contain test-label");

    // Verify dependencies and comments are stripped (NEEDLE compatibility)
    // They should not be present in JSON output at all (not even as empty arrays)
    assert!(bead.get("dependencies").is_none() || bead["dependencies"].as_array().map(|a| a.is_empty()).unwrap_or(false),
               "Dependencies should be stripped for NEEDLE compatibility");
    assert!(bead.get("comments").is_none() || bead["comments"].as_array().map(|a| a.is_empty()).unwrap_or(false),
               "Comments should be stripped for NEEDLE compatibility");
}

#[test]
fn test_show_json_flag() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a test bead
    let bead_id = create_test_bead(workspace, "Test --json flag");

    // Show using --json flag (alias for --format json)
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .arg("--json")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(show_result.status.success(), "bf show --json failed: {}", String::from_utf8_lossy(&show_result.stderr));

    let output = String::from_utf8(show_result.stdout).unwrap();

    // Should parse as JSON array
    let beads: Vec<serde_json::Value> = serde_json::from_str(&output)
        .expect("Failed to parse JSON output");
    assert_eq!(beads.len(), 1);
    assert_eq!(beads[0]["id"], bead_id);
}

#[test]
fn test_show_toon_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a test bead
    let bead_id = create_test_bead(workspace, "Test toon format");

    // Show the bead in toon format
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .arg("--format")
        .arg("toon")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(show_result.status.success(), "bf show --format toon failed: {}", String::from_utf8_lossy(&show_result.stderr));

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Toon output:\n{}", output);

    // Verify basic fields are present (toon format is similar to text)
    assert!(output.contains(&format!("ID: {}", bead_id)));
    assert!(output.contains("Title: Test toon format"));
    assert!(output.contains("Status:"));
    assert!(output.contains("Priority:"));
}

#[test]
fn test_show_missing_bead() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Try to show a non-existent bead
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg("bf-nonexistent")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(!show_result.status.success(), "bf show should fail for non-existent bead");

    let stderr = String::from_utf8(show_result.stderr).unwrap();
    assert!(stderr.contains("Bead not found") || stderr.contains("not found"),
           "Error message should indicate bead not found");
}

#[test]
fn test_show_with_all_fields() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a test bead
    let bead_id = create_test_bead(workspace, "Test all fields");

    // Update with all optional fields
    let update_result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--description")
        .arg("Test description with details")
        .arg("--acceptance-criteria")
        .arg("AC 1: Should work")
        .arg("--notes")
        .arg("Test notes")
        .arg("--design")
        .arg("Test design reference")
        .arg("--assignee")
        .arg("test-assignee")
        .current_dir(workspace)
        .output()
        .expect("Failed to update bead");

    assert!(update_result.status.success(), "bf update failed: {}", String::from_utf8_lossy(&update_result.stderr));

    // Add labels using separate command
    let label_result = std::process::Command::new(&bf_path)
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("label1")
        .arg("--label")
        .arg("label2")
        .current_dir(workspace)
        .output()
        .expect("Failed to add labels");

    assert!(label_result.status.success(), "bf label add failed: {}", String::from_utf8_lossy(&label_result.stderr));

    // Show the bead
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(show_result.status.success(), "bf show failed");

    let output = String::from_utf8(show_result.stdout).unwrap();
    let beads: Vec<serde_json::Value> = serde_json::from_str(&output)
        .expect("Failed to parse JSON");

    let bead = &beads[0];
    assert_eq!(bead["id"], bead_id);
    assert_eq!(bead["description"], "Test description with details");
    assert_eq!(bead["acceptance_criteria"], "AC 1: Should work");
    assert_eq!(bead["notes"], "Test notes");
    assert_eq!(bead["design"], "Test design reference");
    assert_eq!(bead["assignee"], "test-assignee");

    // Verify both labels are present
    let labels: Vec<&str> = bead["labels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap())
        .collect();
    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"label1"));
    assert!(labels.contains(&"label2"));
}

#[test]
fn test_show_with_dependencies() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create three beads: bf-dep-1, bf-dep-2, and bf-main
    let dep1_id = create_test_bead(workspace, "Dependency 1");
    let dep2_id = create_test_bead(workspace, "Dependency 2");
    let main_id = create_test_bead(workspace, "Main task");

    // Add dependencies to main bead using batch
    let batch_json = serde_json::json!([
        {"op": "dep_add_blocker", "parent": &main_id, "child": &dep1_id},
        {"op": "dep_add_blocker", "parent": &main_id, "child": &dep2_id}
    ]);

    let batch_file = workspace.join("batch.json");
    fs::write(&batch_file, batch_json.to_string()).unwrap();

    let batch_result = std::process::Command::new(&bf_path)
        .arg("batch")
        .arg("--file")
        .arg(&batch_file)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf batch");

    assert!(batch_result.status.success(), "bf batch failed");

    // Show the main bead in text format to see dependencies
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&main_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(show_result.status.success(), "bf show failed");

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Show with dependencies:\n{}", output);

    // Verify dependencies are shown
    assert!(output.contains("Dependencies:"), "Should show Dependencies section");
    assert!(output.contains(&dep1_id), "Should show dependency 1");
    assert!(output.contains(&dep2_id), "Should show dependency 2");
    assert!(output.contains("(blocks)"), "Should show dependency type");
}

#[test]
fn test_show_with_labels_only() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a bead with labels
    let bead_id = create_test_bead(workspace, "Test labels");

    let update_result = std::process::Command::new(&bf_path)
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("phase-1")
        .arg("--label")
        .arg("priority-high")
        .arg("--label")
        .arg("backend")
        .current_dir(workspace)
        .output()
        .expect("Failed to add labels");

    assert!(update_result.status.success(), "bf label add failed: {}", String::from_utf8_lossy(&update_result.stderr));

    // Show in text format
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(show_result.status.success());

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Show with labels:\n{}", output);

    // Verify labels are shown as comma-separated list
    assert!(output.contains("Labels:"), "Should show Labels section");
    assert!(output.contains("phase-1"), "Should show phase-1 label");
    assert!(output.contains("priority-high"), "Should show priority-high label");
    assert!(output.contains("backend"), "Should show backend label");
}

#[test]
fn test_show_closed_bead() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create and close a bead
    let bead_id = create_test_bead(workspace, "Test closed bead");

    let close_result = std::process::Command::new(&bf_path)
        .arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test completed")
        .current_dir(workspace)
        .output()
        .expect("Failed to close bead");

    assert!(close_result.status.success());

    // Show the closed bead
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
    let beads: Vec<serde_json::Value> = serde_json::from_str(&output)
        .expect("Failed to parse JSON");

    let bead = &beads[0];
    assert_eq!(bead["status"], "closed");
    assert_eq!(bead["close_reason"], "Test completed");
    assert!(bead["closed_at"].is_string(), "Should have closed_at timestamp");
}

#[test]
fn test_show_in_progress_bead() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create and update bead to in_progress
    let bead_id = create_test_bead(workspace, "Test in progress bead");

    let update_result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--status")
        .arg("in_progress")
        .current_dir(workspace)
        .output()
        .expect("Failed to update bead");

    assert!(update_result.status.success());

    // Show the bead
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(show_result.status.success());

    let output = String::from_utf8(show_result.stdout).unwrap();
    assert!(output.contains("Status: in_progress"), "Should show in_progress status");
}

#[test]
fn test_show_basic_fields_display() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a bead with all basic fields populated
    let bead_id = create_test_bead(workspace, "Test all basic fields");

    // Update with all optional fields
    let update_result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--description")
        .arg("Test description for basic fields")
        .arg("--assignee")
        .arg("test-assignee")
        .current_dir(workspace)
        .output()
        .expect("Failed to update bead");

    assert!(update_result.status.success(), "bf update failed: {}", String::from_utf8_lossy(&update_result.stderr));

    // Show the bead in text format and verify all basic fields are present
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(show_result.status.success(), "bf show failed: {}", String::from_utf8_lossy(&show_result.stderr));

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Show output with all basic fields:\n{}", output);

    // Verify all basic fields are present in the output
    // 1. id
    assert!(output.contains(&format!("ID: {}", bead_id)), "Output should contain id field");

    // 2. title
    assert!(output.contains("Title: Test all basic fields"), "Output should contain title field");

    // 3. description
    assert!(output.contains("Description: Test description for basic fields"), "Output should contain description field");

    // 4. status
    assert!(output.contains("Status:"), "Output should contain status field");
    assert!(output.contains("open") || output.contains("Status: open"), "Output should show status value");

    // 5. priority
    assert!(output.contains("Priority:"), "Output should contain priority field");
    assert!(output.contains("P2") || output.contains("Priority: 2"), "Output should show priority value");

    // 6. issue_type (shown as "Type:" in output)
    assert!(output.contains("Type:"), "Output should contain issue_type field");
    assert!(output.contains("task") || output.contains("Type: task"), "Output should show issue_type value");

    // 7. created_at
    // Timestamps should be in ISO 8601 format (e.g., "2026-07-04T12:34:56Z" or similar)
    // The show command doesn't directly print timestamps in text format, but they're in JSON

    // 8. updated_at
    // Same as created_at - available in JSON format

    // Verify in JSON format for timestamps
    let show_json_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show --format json");

    assert!(show_json_result.status.success(), "bf show json failed: {}", String::from_utf8_lossy(&show_json_result.stderr));

    let json_output = String::from_utf8(show_json_result.stdout).unwrap();
    let beads: Vec<serde_json::Value> = serde_json::from_str(&json_output)
        .expect("Failed to parse JSON output");

    assert_eq!(beads.len(), 1, "Should return exactly one bead");
    let bead = &beads[0];

    // Verify timestamps are properly formatted (ISO 8601)
    let created_at = bead["created_at"].as_str().expect("created_at should be a string");
    assert!(created_at.len() > 0, "created_at should not be empty");
    // ISO 8601 format should contain 'T' and end with 'Z'
    assert!(created_at.contains('T'), "created_at should be in ISO 8601 format (contain 'T')");

    let updated_at = bead["updated_at"].as_str().expect("updated_at should be a string");
    assert!(updated_at.len() > 0, "updated_at should not be empty");
    assert!(updated_at.contains('T'), "updated_at should be in ISO 8601 format (contain 'T')");

    // 9. closed_at - should be null/absent for open beads
    assert!(bead.get("closed_at").is_none() || bead["closed_at"].is_null(),
            "closed_at should be null or absent for open beads");
}

#[test]
fn test_show_closed_bead_timestamps() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create, close, and verify closed_at timestamp
    let bead_id = create_test_bead(workspace, "Test closed bead timestamps");

    let close_result = std::process::Command::new(&bf_path)
        .arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test completed")
        .current_dir(workspace)
        .output()
        .expect("Failed to close bead");

    assert!(close_result.status.success(), "bf close failed");

    // Show the closed bead in JSON format to check timestamps
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
    let beads: Vec<serde_json::Value> = serde_json::from_str(&output)
        .expect("Failed to parse JSON");

    let bead = &beads[0];

    // Verify closed_at timestamp exists and is properly formatted
    let closed_at = bead["closed_at"].as_str().expect("closed_at should be a string when bead is closed");
    assert!(closed_at.len() > 0, "closed_at should not be empty for closed beads");
    assert!(closed_at.contains('T'), "closed_at should be in ISO 8601 format (contain 'T')");

    // Verify the timestamp is recent (within last minute)
    let closed_dt = chrono::DateTime::parse_from_rfc3339(closed_at)
        .expect("closed_at should be valid RFC3339/ISO 8601 format");
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(closed_dt.with_timezone(&chrono::Utc));
    assert!(duration.num_seconds() < 60, "closed_at should be recent (within last minute)");
}
