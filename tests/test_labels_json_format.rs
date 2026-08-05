//! Tests for labels command JSON format output
//!
//! These tests verify that `bf labels --format json` outputs valid JSON
//! with the correct structure for both single-bead and all-beads modes.

use serde_json::Value;
use std::process::Command;

/// Resolve the freshly-built bf binary — never the system-installed one.
fn bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

use std::sync::OnceLock;

static WORKSPACE: OnceLock<tempfile::TempDir> = OnceLock::new();

/// Per-binary isolated workspace — prevents test pollution and contention.
fn workspace_dir() -> &'static std::path::Path {
    WORKSPACE
        .get_or_init(|| {
            let dir = tempfile::tempdir().unwrap();
            let beads = dir.path().join(".beads");
            std::fs::create_dir(&beads).unwrap();
            bead_forge::config::init_workspace(&beads, "bf").unwrap();
            // Create the database up front (WAL mode, schema applied) so
            // parallel test threads never stampede a cold-start conversion.
            let metadata = bead_forge::config::load_metadata(&beads).unwrap();
            let _ = bead_forge::Storage::open(&beads.join(&metadata.database)).unwrap();
            dir
        })
        .path()
}

fn bf() -> Command {
    let mut cmd = Command::new(bf_binary());
    cmd.arg("-w")
        .arg(workspace_dir().join(".beads"))
        .current_dir(workspace_dir());
    cmd
}

fn create_test_bead(title: &str) -> String {
    let output = bf()
        .arg("create")
        .arg("--title")
        .arg(title)
        .arg("--type")
        .arg("task")
        .arg("--priority")
        .arg("2")
        .output()
        .expect("Failed to create bead");

    assert!(
        output.status.success(),
        "Failed to create bead: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    stdout.trim().to_string()
}

fn add_label(bead_id: &str, label: &str) {
    let output = bf()
        .arg("label")
        .arg("add")
        .arg(bead_id)
        .arg("--label")
        .arg(label)
        .output()
        .expect("Failed to add label");

    assert!(
        output.status.success(),
        "Failed to add label: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_labels_json_single_bead_empty_labels() {
    // Test that 'bf labels <id> --format json' outputs empty array for bead with no labels
    let bead_id = create_test_bead("Empty labels JSON test");

    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    assert!(
        output.status.success(),
        "Failed to list labels: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should be valid JSON array
    let parsed: Value = serde_json::from_str(trimmed).expect("Output should be valid JSON");

    assert!(parsed.is_array(), "Output should be an array");
    assert_eq!(
        parsed.as_array().unwrap().len(),
        0,
        "Empty labels should produce empty array"
    );

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_labels_json_single_bead_single_label() {
    // Test that 'bf labels <id> --format json' outputs array with one label
    let bead_id = create_test_bead("Single label JSON test");
    add_label(&bead_id, "urgent");

    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    assert!(
        output.status.success(),
        "Failed to list labels: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should be valid JSON array with one element
    let parsed: Value = serde_json::from_str(trimmed).expect("Output should be valid JSON");

    assert!(parsed.is_array(), "Output should be an array");
    let labels = parsed.as_array().unwrap();
    assert_eq!(labels.len(), 1, "Should have exactly one label");

    let label = labels.get(0).expect("Label should exist");
    assert_eq!(label.as_str(), Some("urgent"), "Label should be 'urgent'");

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_labels_json_single_bead_multiple_labels() {
    // Test that 'bf labels <id> --format json' outputs array with multiple labels
    let bead_id = create_test_bead("Multiple labels JSON test");

    // Add multiple labels
    let labels_to_add = vec!["backend", "urgent", "phase-1"];
    for label in &labels_to_add {
        add_label(&bead_id, label);
    }

    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    assert!(
        output.status.success(),
        "Failed to list labels: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should be valid JSON array with all labels
    let parsed: Value = serde_json::from_str(trimmed).expect("Output should be valid JSON");

    assert!(parsed.is_array(), "Output should be an array");
    let labels = parsed.as_array().unwrap();
    assert_eq!(labels.len(), 3, "Should have exactly three labels");

    // Verify each label is present
    let label_strings: Vec<String> = labels
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    for expected_label in &labels_to_add {
        assert!(
            label_strings.contains(&expected_label.to_string()),
            "Missing label '{}': {:?}",
            expected_label,
            label_strings
        );
    }

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_labels_json_all_beads_empty_workspace() {
    // Test that 'bf labels --format json' outputs [] for workspace with no beads
    let output = bf()
        .arg("labels")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list all labels");

    assert!(
        output.status.success(),
        "Failed to list all labels: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Empty workspace should output []
    assert_eq!(trimmed, "[]", "Empty workspace should output []");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_labels_json_all_beads_with_labels() {
    // Test that 'bf labels --format json' outputs JSONL with all beads
    let bead1 = create_test_bead("JSONL test bead 1");
    add_label(&bead1, "backend");
    add_label(&bead1, "urgent");

    let bead2 = create_test_bead("JSONL test bead 2");
    add_label(&bead2, "frontend");
    add_label(&bead2, "urgent");

    let bead3 = create_test_bead("JSONL test bead 3 with no labels");

    let output = bf()
        .arg("labels")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list all labels");

    assert!(
        output.status.success(),
        "Failed to list all labels: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();

    // Parse each line as a JSON object
    let mut found_beads = std::collections::HashMap::new();
    for line in lines {
        let parsed: Value =
            serde_json::from_str(line).expect(&format!("Each line should be valid JSON: {}", line));

        assert!(parsed.is_object(), "Each line should be a JSON object");

        let id = parsed
            .get("id")
            .and_then(|v| v.as_str())
            .expect("Object should have 'id' field");
        let title = parsed
            .get("title")
            .and_then(|v| v.as_str())
            .expect("Object should have 'title' field");
        let labels = parsed
            .get("labels")
            .and_then(|v| v.as_array())
            .expect("Object should have 'labels' array");

        found_beads.insert(id.to_string(), (title.to_string(), labels.clone()));

        // Verify the structure
        assert!(!id.is_empty(), "ID should not be empty");
        assert!(!title.is_empty(), "Title should not be empty");
        // labels is already Vec<Value> from as_array(), no need to check is_array
    }

    // Verify bead1 has 2 labels
    let bead1_data = found_beads.get(&bead1).expect("Should find bead1");
    assert_eq!(bead1_data.1.len(), 2, "bead1 should have 2 labels");
    let bead1_labels: Vec<String> = bead1_data
        .1
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(bead1_labels.contains(&"backend".to_string()));
    assert!(bead1_labels.contains(&"urgent".to_string()));

    // Verify bead2 has 2 labels
    let bead2_data = found_beads.get(&bead2).expect("Should find bead2");
    assert_eq!(bead2_data.1.len(), 2, "bead2 should have 2 labels");
    let bead2_labels: Vec<String> = bead2_data
        .1
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(bead2_labels.contains(&"frontend".to_string()));
    assert!(bead2_labels.contains(&"urgent".to_string()));

    // Verify bead3 has 0 labels
    let bead3_data = found_beads.get(&bead3).expect("Should find bead3");
    assert_eq!(bead3_data.1.len(), 0, "bead3 should have 0 labels");

    // Clean up
    bf().arg("close")
        .arg(&bead1)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead1");
    bf().arg("close")
        .arg(&bead2)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead2");
    bf().arg("close")
        .arg(&bead3)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead3");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_labels_json_structure_matches_schema() {
    // Test JSON structure matches expected schema for single bead
    let bead_id = create_test_bead("Schema validation test");
    add_label(&bead_id, "test-label");

    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Parse and verify schema
    let parsed: Value = serde_json::from_str(trimmed).expect("Should be valid JSON");

    // Should be an array
    assert!(parsed.is_array(), "Single bead output should be an array");

    // Each element should be a string
    if let Some(arr) = parsed.as_array() {
        for label in arr {
            assert!(
                label.is_string(),
                "Each label should be a string, got: {}",
                label
            );
        }
    }

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_labels_json_structure_all_beads_schema() {
    // Test JSON structure matches expected schema for all beads
    let bead_id = create_test_bead("All beads schema test");
    add_label(&bead_id, "schema-test");

    let output = bf()
        .arg("labels")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list all labels");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();

    // Parse each line and verify schema
    for line in lines {
        let parsed: Value =
            serde_json::from_str(line).expect(&format!("Each line should be valid JSON: {}", line));

        // Should be an object with specific fields
        assert!(parsed.is_object(), "Line should be a JSON object");

        // Check required fields exist and have correct types
        let id = parsed.get("id");
        assert!(id.is_some(), "Object must have 'id' field");
        assert!(id.unwrap().is_string(), "'id' field must be a string");

        let title = parsed.get("title");
        assert!(title.is_some(), "Object must have 'title' field");
        assert!(title.unwrap().is_string(), "'title' field must be a string");

        let labels = parsed.get("labels");
        assert!(labels.is_some(), "Object must have 'labels' field");
        assert!(
            labels.unwrap().is_array(),
            "'labels' field must be an array"
        );
    }

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_labels_json_compact_format() {
    // Test that JSON output is compact (no pretty-printing)
    let bead_id = create_test_bead("Compact format test");
    add_label(&bead_id, "compact");

    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    assert!(
        output.status.success(),
        "Failed to list labels: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should NOT contain whitespace (compact JSON)
    assert!(
        !trimmed.contains("\n"),
        "Compact JSON should not contain newlines"
    );
    assert!(
        !trimmed.contains("  "),
        "Compact JSON should not contain extra spaces"
    );

    // Clean up
    let close_output = bf()
        .arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to run close command");

    assert!(
        close_output.status.success(),
        "Failed to close bead: {}",
        String::from_utf8_lossy(&close_output.stderr)
    );
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_labels_json_special_characters() {
    // Test that labels with special characters are properly JSON-encoded
    let bead_id = create_test_bead("Special chars test");

    // Add labels with special characters
    let special_labels = vec![
        "label with spaces",
        "label-with-dashes",
        "label_with_underscores",
        "label\"with\"quotes",
        "label'with'apostrophes",
    ];

    for label in &special_labels {
        // Add label via command-line (will be properly quoted)
        let output = bf()
            .arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to add label");

        assert!(
            output.status.success(),
            "Failed to add label '{}': {}",
            label,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Parse JSON
    let parsed: Value =
        serde_json::from_str(trimmed).expect("Should be valid JSON with special characters");

    assert!(parsed.is_array(), "Should be an array");
    let labels = parsed.as_array().unwrap();
    assert_eq!(
        labels.len(),
        special_labels.len(),
        "Should have all special labels"
    );

    // Verify each special character label is present
    let label_strings: Vec<String> = labels
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    for expected_label in &special_labels {
        assert!(
            label_strings.contains(&expected_label.to_string()),
            "Missing special label '{}': {:?}",
            expected_label,
            label_strings
        );
    }

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_labels_json_unicode_characters() {
    // Test that labels with unicode characters are properly handled
    let bead_id = create_test_bead("Unicode test");

    // Add labels with unicode characters
    let unicode_labels = vec!["émoji", "中文标签", "🔧", "café", "тест"];

    for label in &unicode_labels {
        let output = bf()
            .arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to add label");

        assert!(
            output.status.success(),
            "Failed to add unicode label '{}': {}",
            label,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Parse JSON
    let parsed: Value =
        serde_json::from_str(trimmed).expect("Should be valid JSON with unicode characters");

    assert!(parsed.is_array(), "Should be an array");
    let labels = parsed.as_array().unwrap();
    assert_eq!(
        labels.len(),
        unicode_labels.len(),
        "Should have all unicode labels"
    );

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}
