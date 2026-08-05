//! Integration tests for extended field display in `bf show` command
//!
//! Tests the show command's ability to display extended fields:
//! - Annotations (custom key-value metadata stored in bead_annotations table)
//! - Assignee (already tested in test_show_command.rs, but validated here)
//! - Labels/Tags (already tested in test_show_command.rs, but validated here)
//! - Custom fields (annotations are the custom field mechanism)

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

/// Add an annotation to a bead via CLI
fn add_annotation(workspace: impl AsRef<std::path::Path>, bead_id: &str, key: &str, value: &str) {
    let bf_path = get_bf_binary();
    let result = std::process::Command::new(&bf_path)
        .arg("annotate")
        .arg("set")
        .arg(bead_id)
        .arg(key)
        .arg(value)
        .current_dir(&workspace.as_ref())
        .output()
        .expect("Failed to add annotation");

    assert!(
        result.status.success(),
        "bf annotate set failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn test_show_displays_annotations_text_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a test bead
    let bead_id = create_test_bead(workspace, "Test annotations display");

    // Add multiple annotations
    add_annotation(workspace, &bead_id, "severity", "high");
    add_annotation(workspace, &bead_id, "component", "backend");
    add_annotation(workspace, &bead_id, "estimated_hours", "4");

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
    println!("Show output with annotations:\n{}", output);

    // Verify annotations section is present
    assert!(
        output.contains("Annotations:"),
        "Should show Annotations section"
    );

    // Verify all annotations are displayed
    assert!(
        output.contains("severity: high"),
        "Should show severity annotation"
    );
    assert!(
        output.contains("component: backend"),
        "Should show component annotation"
    );
    assert!(
        output.contains("estimated_hours: 4"),
        "Should show estimated_hours annotation"
    );
}

#[test]
fn test_show_displays_annotations_toon_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a test bead
    let bead_id = create_test_bead(workspace, "Test annotations in toon format");

    // Add annotations
    add_annotation(workspace, &bead_id, "category", "testing");
    add_annotation(workspace, &bead_id, "priority", "p0");

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
    println!("Toon format output with annotations:\n{}", output);

    // Verify annotations are shown in toon format
    assert!(
        output.contains("Annotations:"),
        "Should show Annotations section"
    );
    assert!(
        output.contains("category: testing"),
        "Should show category annotation"
    );
    assert!(
        output.contains("priority: p0"),
        "Should show priority annotation"
    );
}

#[test]
fn test_show_displays_annotations_json_format() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a test bead
    let bead_id = create_test_bead(workspace, "Test annotations in JSON format");

    // Add annotations
    add_annotation(workspace, &bead_id, "story_points", "5");
    add_annotation(workspace, &bead_id, "sprint", "42");

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
    println!("JSON output with annotations:\n{}", output);

    // Parse JSON and verify annotations are present
    let beads: Vec<serde_json::Value> =
        serde_json::from_str(&output).expect("Failed to parse JSON output");

    assert_eq!(beads.len(), 1, "Should return exactly one bead");
    let bead = &beads[0];

    // Verify annotations object exists and contains our values
    assert!(
        bead.get("annotations").is_some(),
        "Should have annotations field"
    );

    let annotations = bead["annotations"].as_object().unwrap();
    assert_eq!(
        annotations.get("story_points").and_then(|v| v.as_str()),
        Some("5"),
        "Should have story_points annotation"
    );
    assert_eq!(
        annotations.get("sprint").and_then(|v| v.as_str()),
        Some("42"),
        "Should have sprint annotation"
    );
}

#[test]
fn test_show_displays_assignee_when_present() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a test bead with assignee
    let bead_id = create_test_bead(workspace, "Test assignee display");

    // Update with assignee
    let update_result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--assignee")
        .arg("test-worker-123")
        .current_dir(workspace)
        .output()
        .expect("Failed to update bead");

    assert!(
        update_result.status.success(),
        "bf update failed: {}",
        String::from_utf8_lossy(&update_result.stderr)
    );

    // Show the bead
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
    println!("Show output with assignee:\n{}", output);

    // Verify assignee is displayed
    assert!(
        output.contains("Assignee: test-worker-123"),
        "Should show assignee field when present"
    );
}

#[test]
fn test_show_displays_labels_when_present() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a test bead
    let bead_id = create_test_bead(workspace, "Test labels display");

    // Add labels
    let label_result = std::process::Command::new(&bf_path)
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("phase-1")
        .arg("--label")
        .arg("backend")
        .arg("--label")
        .arg("urgent")
        .current_dir(workspace)
        .output()
        .expect("Failed to add labels");

    assert!(
        label_result.status.success(),
        "bf label add failed: {}",
        String::from_utf8_lossy(&label_result.stderr)
    );

    // Show the bead
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
    println!("Show output with labels:\n{}", output);

    // Verify labels section is present and all labels are shown
    assert!(output.contains("Labels:"), "Should show Labels section");
    assert!(output.contains("phase-1"), "Should show phase-1 label");
    assert!(output.contains("backend"), "Should show backend label");
    assert!(output.contains("urgent"), "Should show urgent label");
}

#[test]
fn test_show_all_extended_fields_together() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a test bead
    let bead_id = create_test_bead(workspace, "Test all extended fields together");

    // Update with assignee
    let update_result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--assignee")
        .arg("worker-abc")
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
        .arg("integration-test")
        .current_dir(workspace)
        .output()
        .expect("Failed to add labels");
    assert!(label_result.status.success());

    // Add annotations
    add_annotation(workspace, &bead_id, "test_type", "integration");
    add_annotation(workspace, &bead_id, "coverage", "full");

    // Show the bead
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
    println!("Show output with all extended fields:\n{}", output);

    // Verify all extended fields are present
    // 1. Assignee
    assert!(
        output.contains("Assignee: worker-abc"),
        "Should show assignee"
    );

    // 2. Labels
    assert!(output.contains("Labels:"), "Should show labels section");
    assert!(output.contains("integration-test"), "Should show label");

    // 3. Annotations
    assert!(
        output.contains("Annotations:"),
        "Should show annotations section"
    );
    assert!(
        output.contains("test_type: integration"),
        "Should show test_type annotation"
    );
    assert!(
        output.contains("coverage: full"),
        "Should show coverage annotation"
    );
}

#[test]
fn test_show_empty_annotations_not_displayed() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a test bead without annotations
    let bead_id = create_test_bead(workspace, "Test no annotations");

    // Show the bead
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
    println!("Show output without annotations:\n{}", output);

    // Verify annotations section is NOT shown when empty
    assert!(
        !output.contains("Annotations:"),
        "Should NOT show Annotations section when no annotations are present"
    );
}

#[test]
fn test_show_annotations_verify_database_integration() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a test bead
    let bead_id = create_test_bead(workspace, "Test database integration");

    // Add annotations via CLI
    add_annotation(workspace, &bead_id, "db_key", "db_value");

    // Verify the annotation is in the database directly
    let db_path = beads_dir.join("beads.db");
    let storage = bead_forge::storage::Storage::open(&db_path).unwrap();

    let issue = storage
        .get_issue(&bead_id)
        .expect("Failed to get issue")
        .expect("Issue should exist");

    // Verify annotation is stored correctly
    assert!(
        issue.annotations.contains_key("db_key"),
        "Issue should have db_key annotation"
    );
    assert_eq!(
        issue.annotations.get("db_key"),
        Some(&"db_value".to_string()),
        "Annotation value should match"
    );

    // Now verify it shows up in CLI output
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    let output = String::from_utf8(show_result.stdout).unwrap();
    let beads: Vec<serde_json::Value> =
        serde_json::from_str(&output).expect("Failed to parse JSON output");

    let bead = &beads[0];
    let annotations = bead["annotations"].as_object().unwrap();

    assert_eq!(
        annotations.get("db_key").and_then(|v| v.as_str()),
        Some("db_value"),
        "CLI output should match database content"
    );
}

#[test]
fn test_show_annotations_with_special_characters() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a test bead
    let bead_id = create_test_bead(workspace, "Test special characters in annotations");

    // Add annotations with special characters
    add_annotation(workspace, &bead_id, "key-with-dash", "value with spaces");
    add_annotation(
        workspace,
        &bead_id,
        "key_with_underscore",
        "value:with:colons",
    );
    add_annotation(workspace, &bead_id, "json_data", r#"{"complex": "data"}"#);

    // Show the bead in JSON format to verify proper escaping
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
    let beads: Vec<serde_json::Value> =
        serde_json::from_str(&output).expect("Failed to parse JSON output");

    let bead = &beads[0];
    let annotations = bead["annotations"].as_object().unwrap();

    // Verify special characters are preserved
    assert_eq!(
        annotations.get("key-with-dash").and_then(|v| v.as_str()),
        Some("value with spaces"),
        "Should preserve spaces in values"
    );
    assert_eq!(
        annotations
            .get("key_with_underscore")
            .and_then(|v| v.as_str()),
        Some("value:with:colons"),
        "Should preserve colons in values"
    );
    assert_eq!(
        annotations.get("json_data").and_then(|v| v.as_str()),
        Some(r#"{"complex": "data"}"#),
        "Should preserve JSON strings in values"
    );
}
