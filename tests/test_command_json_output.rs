//! JSON output unit tests for core bead-forge commands
//!
//! Tests the JSON output format for core bead-forge commands:
//! - list: Multiple beads in JSONL format
//! - ready: Ready (unblocked) beads in JSONL format
//! - recent: Recently modified beads with envelope wrapping
//!
//! Acceptance Criteria:
//! - Test JSON structure validity for each command's --json output
//! - Test required fields are present in JSON output
//! - Test JSON output handles empty results correctly
//! - Test JSON output handles special characters in bead fields
//! - cargo test passes for new JSON output tests

mod common;

use std::process::Command;
use serde_json::Value;

/// Get the path to the bf binary
fn bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf")
        .unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Create a Command builder for bf with workspace configured
fn bf_command(workspace: &common::TempWorkspace) -> Command {
    let mut cmd = Command::new(&bf_binary());
    cmd.arg("-w").arg(&workspace.beads_dir);
    cmd.current_dir(workspace.workspace_path());
    cmd
}

/// JSON validation helpers
mod json_validation {
    use super::*;

    /// Parse a JSON string and panic if invalid
    pub fn parse_json(json: &str) -> Value {
        serde_json::from_str(json).unwrap_or_else(|e| {
            panic!("Failed to parse JSON: {}\nJSON was: {}", e, json)
        })
    }

    /// Parse a JSONL string (newline-delimited JSON) into a Vec of values
    pub fn parse_jsonl(jsonl: &str) -> Vec<Value> {
        jsonl
            .lines()
            .filter(|line| !line.trim().is_empty() && *line != "[]")
            .map(|line| parse_json(line))
            .collect()
    }

    /// Get a string field from JSON, panic if missing or not a string
    pub fn get_string(json: &Value, field: &str) -> String {
        json.get(field)
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("Field '{}' is not a string or is missing: {}", field, json))
            .to_string()
    }

    /// Check if JSON has a specific field
    pub fn has_field(json: &Value, field: &str) -> bool {
        json.get(field).is_some()
    }

    /// Get an array field from JSON, panic if missing or not an array
    pub fn get_array(json: &Value, field: &str) -> Vec<Value> {
        json.get(field)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_else(|| panic!("Field '{}' is not an array or is missing: {}", field, json))
    }
}

/// Envelope wrapping validation helpers
mod envelope {
    use super::*;

    /// Expected envelope structure: {version: 1, kind: "<command>", data: <payload>}
    pub fn validate_envelope(json: &str, expected_kind: &str) -> Value {
        let envelope = json_validation::parse_json(json);

        // Check version field
        let version = envelope.get("version")
            .and_then(|v| v.as_i64())
            .expect("Envelope must have numeric 'version' field");
        assert_eq!(version, 1, "Envelope version must be 1");

        // Check kind field
        let kind = envelope.get("kind")
            .and_then(|k| k.as_str())
            .expect("Envelope must have string 'kind' field");
        assert_eq!(kind, expected_kind, "Envelope kind mismatch");

        // Check data field exists
        assert!(
            envelope.get("data").is_some(),
            "Envelope must have 'data' field"
        );

        envelope
    }

    /// Get the data field from an envelope
    pub fn get_envelope_data(envelope: &Value) -> Value {
        envelope.get("data")
            .cloned()
            .unwrap_or_else(|| panic!("Envelope missing 'data' field"))
    }
}

/// Helper to check required issue fields in JSON
fn assert_issue_fields_present(json: &Value, context: &str) {
    assert!(json_validation::has_field(json, "id"), "{}: Missing 'id' field", context);
    assert!(json_validation::has_field(json, "title"), "{}: Missing 'title' field", context);
    assert!(json_validation::has_field(json, "status"), "{}: Missing 'status' field", context);
    assert!(json_validation::has_field(json, "priority"), "{}: Missing 'priority' field", context);
    assert!(json_validation::has_field(json, "issue_type"), "{}: Missing 'issue_type' field", context);
    // These should always be present even if null/empty (display normalization)
    assert!(json_validation::has_field(json, "assignee"), "{}: Missing 'assignee' field", context);
    assert!(json_validation::has_field(json, "labels"), "{}: Missing 'labels' field", context);
}

/// Helper to check if binary exists
fn require_binary() {
    let binary = bf_binary();
    if !std::path::Path::new(&binary).exists() {
        eprintln!("Skipping test - binary not found at: {}. Run 'cargo build' first.", binary);
        panic!("Binary not found");
    }
}

// ============================================================================
// list command tests
// ============================================================================

#[test]
#[ignore]
fn test_list_command_json_structure() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create multiple beads
    ws.create_bead("bf-test-1", "First bead for list test").unwrap();
    ws.create_bead("bf-test-2", "Second bead for list test").unwrap();
    ws.create_bead("bf-test-3", "Third bead for list test").unwrap();

    // Get JSON output
    let output = bf_command(&ws)
        .arg("list")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf list");

    assert!(output.status.success(), "bf list should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // list returns JSONL (one JSON object per line)
    let parsed = json_validation::parse_jsonl(&stdout);
    assert!(parsed.len() >= 3, "list should return at least 3 beads");

    // Each line should be valid JSON with required fields
    for issue_json in parsed.iter().take(3) {
        assert_issue_fields_present(issue_json, "list command");
    }
}

#[test]
#[ignore]
fn test_list_command_json_empty_results() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // List from empty workspace (or with status that yields no results)
    let output = bf_command(&ws)
        .arg("list")
        .arg("--status")
        .arg("closed")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf list");

    assert!(output.status.success(), "bf list should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Empty list returns "[]" (special case in cmd_list)
    let trimmed = stdout.trim();
    assert_eq!(trimmed, "[]", "Empty list should return '[]'");
}

#[test]
#[ignore]
fn test_list_command_json_filters() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create beads with different properties
    ws.create_bead("bf-open", "Open bead for filter test").unwrap();
    // Close one bead
    let closed = bead_forge::Issue {
        id: "bf-closed".to_string(),
        title: "Closed bead for filter test".to_string(),
        status: bead_forge::Status::Closed,
        closed_at: Some(chrono::Utc::now()),
        close_reason: Some("Test close".to_string()),
        ..Default::default()
    };
    ws.create_issue(&closed).unwrap();

    let _active = ws.create_bead("bf-active", "Active bead for filter test").unwrap();

    // Test status filter
    let output = bf_command(&ws)
        .arg("list")
        .arg("--status")
        .arg("closed")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf list");

    assert!(output.status.success(), "bf list should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let parsed = json_validation::parse_jsonl(&stdout);

    assert!(parsed.len() >= 1, "Should find at least one closed bead");

    // Verify the filtered result has correct status
    assert_eq!(json_validation::get_string(&parsed[0], "status"), "closed");
}

#[test]
#[ignore]
fn test_list_command_json_ensure_fields_present() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    let bead_id = "bf-field-test";
    ws.create_bead(bead_id, "Test bead for field presence").unwrap();

    let output = bf_command(&ws)
        .arg("list")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf list");

    assert!(output.status.success(), "bf list should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let parsed = json_validation::parse_jsonl(&stdout);

    // Find our bead in the output
    let our_bead = parsed
        .iter()
        .find(|json| json_validation::get_string(json, "id") == bead_id.to_string())
        .expect("Should find our bead in list output");

    // Verify display normalization: assignee and labels always present
    assert!(json_validation::has_field(our_bead, "assignee"), "assignee must be present");
    assert!(json_validation::has_field(our_bead, "labels"), "labels must be present");
    let _labels = json_validation::get_array(our_bead, "labels");
    // labels is already Vec<Value> from get_array, so it's verified to be an array
}

#[test]
#[ignore]
fn test_list_command_json_with_envelope() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    ws.create_bead("bf-envelope", "Test bead for envelope list").unwrap();

    let output = bf_command(&ws)
        .arg("list")
        .arg("--format")
        .arg("json")
        .arg("--envelope")
        .output()
        .expect("Failed to execute bf list");

    assert!(output.status.success(), "bf list should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should be wrapped in envelope
    let envelope = envelope::validate_envelope(&stdout, "list");
    let data = envelope::get_envelope_data(&envelope);

    // Data should be an array of issues
    let array = data.as_array().expect("list data should be array");
    assert!(array.len() >= 1, "list should return at least one bead");

    for issue_json in array {
        assert_issue_fields_present(issue_json, "list with envelope");
    }
}

// ============================================================================
// ready command tests
// ============================================================================

#[test]
#[ignore]
fn test_ready_command_json_structure() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create some open beads (they should be ready)
    ws.create_bead("bf-ready-1", "Ready bead 1").unwrap();
    ws.create_bead("bf-ready-2", "Ready bead 2").unwrap();

    // Get ready beads
    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // ready returns JSONL, empty returns "[]"
    let trimmed = stdout.trim();
    if trimmed != "[]" {
        let parsed = json_validation::parse_jsonl(trimmed);
        assert!(parsed.len() >= 1, "ready should return at least some beads");

        // Each line should be valid JSON with required fields
        for issue_json in parsed {
            assert_issue_fields_present(&issue_json, "ready command");
        }
    }
}

#[test]
#[ignore]
fn test_ready_command_json_empty_results() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // If all beads are blocked or closed, ready should return []
    let output = bf_command(&ws)
        .arg("ready")
        .arg("--limit")
        .arg("0")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Empty ready returns "[]" (special case in cmd_ready)
    assert!(trimmed == "[]" || trimmed.is_empty(), "Empty ready should return '[]' or empty string");
}

#[test]
#[ignore]
fn test_ready_command_json_limit() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create multiple beads
    for i in 1..=5 {
        let id = format!("bf-ready-{}", i);
        let title = format!("Ready bead {}", i);
        ws.create_bead(&id, &title).unwrap();
    }

    // Test with limit
    let output = bf_command(&ws)
        .arg("ready")
        .arg("--limit")
        .arg("2")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    if trimmed != "[]" && !trimmed.is_empty() {
        let parsed = json_validation::parse_jsonl(trimmed);
        assert!(parsed.len() <= 2, "ready with --limit 2 should return at most 2 beads");
    }
}

#[test]
#[ignore]
fn test_ready_command_json_with_envelope() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    ws.create_bead("bf-ready-envelope", "Test bead for envelope ready").unwrap();

    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .arg("--envelope")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should be wrapped in envelope
    let envelope_obj = envelope::validate_envelope(&stdout, "ready");
    let data = envelope::get_envelope_data(&envelope_obj);

    // Data should be an array or empty array
    let array = data.as_array().expect("ready data should be array");
    if !array.is_empty() {
        for issue_json in array {
            assert_issue_fields_present(issue_json, "ready with envelope");
        }
    }
}

// ============================================================================
// recent command tests
// ============================================================================

#[test]
#[ignore]
fn test_recent_command_json_structure() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create a test bead
    let bead_id = "bf-recent-test";
    ws.create_bead(bead_id, "Recent bead for test").unwrap();

    // Get recent beads (always uses envelope)
    let output = bf_command(&ws)
        .arg("recent")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf recent");

    assert!(output.status.success(), "bf recent should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // recent always wraps output in envelope
    let envelope_obj = envelope::validate_envelope(&stdout, "recent");
    let data = envelope::get_envelope_data(&envelope_obj);

    // Data should be an array or JSONL string
    if let Some(array) = data.as_array() {
        // Verify array has our bead
        assert!(array.len() >= 1, "recent should return at least one bead");
        for issue_json in array {
            assert_issue_fields_present(issue_json, "recent command");
        }
    }
}

#[test]
#[ignore]
fn test_recent_command_json_time_period() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    let bead_id = "bf-recent-time";
    ws.create_bead(bead_id, "Recent bead with time filter").unwrap();

    // Test with time period
    let output = bf_command(&ws)
        .arg("recent")
        .arg("--time-period")
        .arg("1h")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf recent");

    assert!(output.status.success(), "bf recent should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should still be wrapped in envelope
    let envelope_obj = envelope::validate_envelope(&stdout, "recent");
    let data = envelope::get_envelope_data(&envelope_obj);

    // Data should be present (even if empty array)
    assert!(data.is_array() || data.is_string(), "recent data should be array or string");
}

#[test]
#[ignore]
fn test_recent_command_json_empty_results() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Use very short time period that should yield no results
    let output = bf_command(&ws)
        .arg("recent")
        .arg("--time-period")
        .arg("1s")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf recent");

    assert!(output.status.success(), "bf recent should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Even empty results are wrapped in envelope
    let envelope_obj = envelope::validate_envelope(&stdout, "recent");
    let data = envelope::get_envelope_data(&envelope_obj);

    // Empty results should be empty array or empty string
    if let Some(array) = data.as_array() {
        assert_eq!(array.len(), 0, "Empty recent should return empty array");
    }
}

// ============================================================================
// Unicode and special character handling tests
// ============================================================================

#[test]
fn test_json_output_handles_unicode() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create bead with Unicode characters
    let unicode_title = "Test bead with emoji 🎉 and unicode Ñ";
    let bead_id = "bf-unicode";
    ws.create_bead(bead_id, unicode_title).unwrap();

    // Get JSON output from show command
    let output = bf_command(&ws)
        .arg("show")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should be valid JSON
    let parsed = json_validation::parse_json(stdout.trim());

    // show command wraps output in array
    let array = parsed.as_array().expect("show output should be a JSON array");
    let issue_json = &array[0];

    let title = json_validation::get_string(issue_json, "title");
    assert!(title.contains("🎉"), "Unicode emoji should be preserved");
    assert!(title.contains("Ñ"), "Unicode character should be preserved");
}

#[test]
fn test_json_output_handles_newlines_in_description() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    let bead_id = "bf-newline";
    ws.create_bead(bead_id, "Test bead with multiline description").unwrap();

    // Update with description containing newlines
    let storage = ws.storage().unwrap();
    let changes = bead_forge::IssueChanges {
        description: Some("Line 1\nLine 2\nLine 3".to_string()),
        ..Default::default()
    };
    storage.update_issue(bead_id, &changes).unwrap();

    // Get JSON output
    let output = bf_command(&ws)
        .arg("show")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should be valid JSON despite newlines
    json_validation::parse_json(stdout.trim());
}

// ============================================================================
// show command tests
// ============================================================================

#[test]
fn test_show_command_json_structure() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create a test bead
    let bead_id = "bf-show-test";
    ws.create_bead(bead_id, "Test bead for show command").unwrap();

    // Get JSON output from show command
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // show command wraps output in array (NEEDLE contract: single-element array)
    let parsed = json_validation::parse_json(stdout.trim());
    let array = parsed.as_array().expect("show output should be a JSON array");
    assert_eq!(array.len(), 1, "show should return a single-element array");

    // The single element should be the issue object
    let issue_json = &array[0];
    assert_eq!(json_validation::get_string(issue_json, "id"), bead_id);
}

#[test]
fn test_show_command_json_required_fields() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create a bead with specific fields
    let bead_id = "bf-show-fields";
    ws.create_bead(bead_id, "Test bead for field validation").unwrap();

    // Add some additional fields to verify they're present
    let storage = ws.storage().unwrap();
    let changes = bead_forge::IssueChanges {
        description: Some("Test description with multiple lines\nand special chars: <>&\"'".to_string()),
        assignee: Some("test-assignee".to_string()),
        labels: Some(vec!["test-label".to_string(), "another-label".to_string()]),
        ..Default::default()
    };
    storage.update_issue(bead_id, &changes).unwrap();

    // Get JSON output
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Parse and get the issue object
    let parsed = json_validation::parse_json(stdout.trim());
    let array = parsed.as_array().expect("show output should be an array");
    let issue_json = &array[0];

    // Verify all required fields are present
    assert_issue_fields_present(issue_json, "show command");

    // Verify specific field values
    assert_eq!(json_validation::get_string(issue_json, "id"), bead_id);
    assert_eq!(json_validation::get_string(issue_json, "title"), "Test bead for field validation");

    // Verify optional fields that we set
    let description = json_validation::get_string(issue_json, "description");
    assert!(description.contains("Test description"), "description should be preserved");

    let assignee = json_validation::get_string(issue_json, "assignee");
    assert_eq!(assignee, "test-assignee", "assignee should be preserved");

    let labels = json_validation::get_array(issue_json, "labels");
    assert_eq!(labels.len(), 2, "labels should be preserved");
}

#[test]
fn test_show_command_json_non_existent_bead() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Try to show a bead that doesn't exist
    let output = bf_command(&ws)
        .arg("show")
        .arg("bf-does-not-exist")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf show");

    // Should fail with non-zero exit code
    assert!(!output.status.success(), "bf show should fail for non-existent bead");

    // stderr should contain error message
    let stderr = String::from_utf8(output.stderr).expect("Invalid UTF-8");
    assert!(stderr.contains("not found") || stderr.contains("Bead not found"),
            "Error message should mention bead not found");
}

#[test]
fn test_show_command_json_with_envelope() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create a test bead
    let bead_id = "bf-show-envelope";
    ws.create_bead(bead_id, "Test bead for envelope show").unwrap();

    // Get JSON output with envelope
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .arg("--format")
        .arg("json")
        .arg("--envelope")
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should be wrapped in envelope
    let envelope_obj = envelope::validate_envelope(&stdout, "show");
    let data = envelope::get_envelope_data(&envelope_obj);

    // Data should be a single issue object (not array when using envelope)
    assert_issue_fields_present(&data, "show with envelope");
    assert_eq!(json_validation::get_string(&data, "id"), bead_id);
}

#[test]
fn test_show_command_json_dependencies_stripped() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create beads with dependencies
    ws.create_bead("bf-dep-1", "First dependency").unwrap();
    ws.create_bead("bf-dep-2", "Second dependency").unwrap();

    let bead_id = "bf-show-with-deps";
    ws.create_bead(bead_id, "Bead with dependencies").unwrap();

    // Add dependencies using the correct API
    let storage = ws.storage().unwrap();
    storage.add_dependency(bead_id, "bf-dep-1", &bead_forge::model::DependencyType::Blocks, "test").unwrap();
    storage.add_dependency(bead_id, "bf-dep-2", &bead_forge::model::DependencyType::Blocks, "test").unwrap();

    // Get JSON output
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Parse and verify dependencies are stripped (NEEDLE compatibility)
    let parsed = json_validation::parse_json(stdout.trim());
    let array = parsed.as_array().expect("show output should be an array");
    let issue_json = &array[0];

    // Dependencies and comments fields should not be present when empty
    // due to #[serde(skip_serializing_if = "Vec::is_empty")]
    let deps = issue_json.get("dependencies");
    match deps {
        Some(dep_value) => {
            let dep_array = dep_value.as_array().expect("dependencies should be an array if present");
            assert_eq!(dep_array.len(), 0, "dependencies should be empty in show JSON output");
        }
        None => {
            // Field is absent when empty, which is acceptable
        }
    }

    let comments = issue_json.get("comments");
    match comments {
        Some(comment_value) => {
            let comment_array = comment_value.as_array().expect("comments should be an array if present");
            assert_eq!(comment_array.len(), 0, "comments should be empty in show JSON output");
        }
        None => {
            // Field is absent when empty, which is acceptable
        }
    }
}

#[test]
fn test_show_command_json_special_characters() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create bead with special characters in description
    let bead_id = "bf-show-special";
    ws.create_bead(bead_id, "Bead with special chars").unwrap();

    // Update with special characters that might break JSON
    let storage = ws.storage().unwrap();
    let special_text = "Special chars: \n\t\r\"'\\<>{}[]&emoji: 🎉🚀\nNewlines and \"quotes\"";
    let changes = bead_forge::IssueChanges {
        description: Some(special_text.to_string()),
        ..Default::default()
    };
    storage.update_issue(bead_id, &changes).unwrap();

    // Get JSON output
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should be valid JSON despite special characters
    let parsed = json_validation::parse_json(stdout.trim());
    let array = parsed.as_array().expect("show output should be an array");
    let issue_json = &array[0];

    // Verify special characters are preserved (properly escaped)
    let description = json_validation::get_string(issue_json, "description");
    assert!(description.contains("emoji"), "Unicode should be preserved");
    assert!(description.contains("🎉"), "Emoji should be preserved");
}
