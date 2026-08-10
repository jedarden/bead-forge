//! Comprehensive JSON output tests for `bf ready` command.
//!
//! Tests the JSON output format for the ready command with:
//! - Empty candidates (no ready beads)
//! - Single candidate (one ready bead)
//! - Multiple candidates (multiple ready beads)
//! - Field correctness (all Issue fields present)
//! - Format switching (--format json vs --format text)
//!
//! Run with: `cargo test ready_json`

mod common;

use serde_json::Value;
use std::process::Command;

/// Get the path to the bf binary
fn bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Create a Command builder for bf with workspace configured
fn bf_command(workspace: &common::TempWorkspace) -> Command {
    let mut cmd = Command::new(&bf_binary());
    cmd.arg("-w").arg(&workspace.beads_dir);
    cmd.current_dir(workspace.workspace_path());
    cmd
}

/// Parse a JSON string and panic if invalid
fn parse_json(json: &str) -> Value {
    serde_json::from_str(json)
        .unwrap_or_else(|e| panic!("Failed to parse JSON: {}\nJSON was: {}", e, json))
}

/// Parse a JSONL string (newline-delimited JSON) into a Vec of values
fn parse_jsonl(jsonl: &str) -> Vec<Value> {
    jsonl
        .lines()
        .filter(|line| !line.trim().is_empty() && *line != "[]")
        .map(|line| parse_json(line))
        .collect()
}

/// Get a string field from JSON, panic if missing or not a string
fn get_string(json: &Value, field: &str) -> String {
    json.get(field)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("Field '{}' is not a string or is missing: {}", field, json))
        .to_string()
}

/// Check if JSON has a specific field
fn has_field(json: &Value, field: &str) -> bool {
    json.get(field).is_some()
}

/// Get an array field from JSON, panic if missing or not an array
fn get_array(json: &Value, field: &str) -> Vec<Value> {
    json.get(field)
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_else(|| panic!("Field '{}' is not an array or is missing: {}", field, json))
}

/// Helper to check required issue fields in JSON
fn assert_issue_fields_present(json: &Value, context: &str) {
    assert!(
        has_field(json, "id"),
        "{}: Missing 'id' field",
        context
    );
    assert!(
        has_field(json, "title"),
        "{}: Missing 'title' field",
        context
    );
    assert!(
        has_field(json, "status"),
        "{}: Missing 'status' field",
        context
    );
    assert!(
        has_field(json, "priority"),
        "{}: Missing 'priority' field",
        context
    );
    assert!(
        has_field(json, "issue_type"),
        "{}: Missing 'issue_type' field",
        context
    );
    // These should always be present even if null/empty (display normalization)
    assert!(
        has_field(json, "assignee"),
        "{}: Missing 'assignee' field",
        context
    );
    assert!(
        has_field(json, "labels"),
        "{}: Missing 'labels' field",
        context
    );
}

/// Helper to check if binary exists
fn require_binary() {
    let binary = bf_binary();
    if !std::path::Path::new(&binary).exists() {
        eprintln!(
            "Skipping test - binary not found at: {}. Run 'cargo build' first.",
            binary
        );
        panic!("Binary not found");
    }
}

// ============================================================================
// Empty Candidates Tests
// ============================================================================

#[test]
fn test_ready_json_empty_candidates_returns_empty_array() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Empty workspace - no beads at all
    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Empty ready should return "[]"
    assert_eq!(trimmed, "[]", "Empty ready should return '[]', got: {}", trimmed);
}

#[test]
fn test_ready_json_empty_candidates_when_all_blocked() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create beads with dependencies (they will be blocked)
    let blocker = ws.create_bead("bf-blocker", "Blocker bead").unwrap();

    let blocked = bead_forge::Issue {
        id: "bf-blocked".to_string(),
        title: "Blocked bead".to_string(),
        ..Default::default()
    };
    ws.create_issue(&blocked).unwrap();

    // Add dependency
    let storage = ws.storage().unwrap();
    storage
        .add_dependency(
            "bf-blocked",
            "bf-blocker",
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();

    // Ready should return empty (both beads are blocked)
    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should return empty array (only blocker is ready, but it's claimed by the create)
    assert_eq!(trimmed, "[]", "When no unclaimed ready beads, should return '[]'");
}

#[test]
fn test_ready_json_empty_candidates_when_all_closed() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create closed beads (they should not appear in ready)
    let closed = bead_forge::Issue {
        id: "bf-closed".to_string(),
        title: "Closed bead".to_string(),
        status: bead_forge::Status::Closed,
        closed_at: Some(chrono::Utc::now()),
        close_reason: Some("Test".to_string()),
        ..Default::default()
    };
    ws.create_issue(&closed).unwrap();

    // Ready should return empty
    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    assert_eq!(trimmed, "[]", "Closed beads should not appear in ready, got: {}", trimmed);
}

#[test]
fn test_ready_json_empty_candidates_exit_code_zero() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Empty workspace
    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    // Exit code should be 0 even with empty results
    assert_eq!(
        output.status.code().unwrap(),
        0,
        "Exit code should be 0 for empty results"
    );
}

// ============================================================================
// Single Candidate Tests
// ============================================================================

#[test]
fn test_ready_json_single_candidate_valid_json() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create a single ready bead
    let bead_id = "bf-ready-single";
    ws.create_bead(bead_id, "Single ready bead").unwrap();

    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should parse as valid JSON
    let parsed = parse_json(trimmed);

    // Verify it's our bead
    assert_eq!(get_string(&parsed, "id"), bead_id);
    assert_eq!(get_string(&parsed, "title"), "Single ready bead");
}

#[test]
fn test_ready_json_single_candidate_has_all_fields() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create a bead with various fields
    let bead_id = "bf-ready-fields";
    ws.create_bead(bead_id, "Bead with all fields").unwrap();

    // Add more fields
    let storage = ws.storage().unwrap();
    let changes = bead_forge::IssueChanges {
        description: Some("Test description".to_string()),
        assignee: Some("test-assignee".to_string()),
        labels: Some(vec!["test-label".to_string()]),
        ..Default::default()
    };
    storage.update_issue(bead_id, &changes).unwrap();

    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    let parsed = parse_json(trimmed);

    // Verify all required fields
    assert_issue_fields_present(&parsed, "single candidate");

    // Verify specific values
    assert_eq!(get_string(&parsed, "id"), bead_id);
    assert_eq!(get_string(&parsed, "title"), "Bead with all fields");

    // Verify optional fields we set
    let description = get_string(&parsed, "description");
    assert!(description.contains("Test description"));

    let assignee = get_string(&parsed, "assignee");
    assert_eq!(assignee, "test-assignee");

    let labels = get_array(&parsed, "labels");
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].as_str().unwrap(), "test-label");
}

#[test]
fn test_ready_json_single_candidate_no_trailing_comma() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    ws.create_bead("bf-single-no-comma", "Test bead").unwrap();

    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should be valid JSON (no trailing commas)
    parse_json(trimmed);

    // Should not have array wrapper (single object, not [object])
    assert!(
        !trimmed.starts_with('['),
        "Single candidate should not be array-wrapped"
    );
}

// ============================================================================
// Multiple Candidates Tests
// ============================================================================

#[test]
fn test_ready_json_multiple_candidates_jsonl_format() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create multiple ready beads
    for i in 1..=3 {
        let id = format!("bf-ready-{}", i);
        let title = format!("Ready bead {}", i);
        ws.create_bead(&id, &title).unwrap();
    }

    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should parse as JSONL (multiple lines)
    let lines: Vec<&str> = trimmed.lines().collect();
    assert!(lines.len() >= 3, "Should have at least 3 lines for 3 beads");

    // Each line should be valid JSON
    for (i, line) in lines.iter().enumerate() {
        let parsed = parse_json(line);
        let id = get_string(&parsed, "id");
        assert!(
            id.contains(&format!("bf-ready-{}", i + 1)),
            "Line {} should have correct bead ID",
            i
        );
    }
}

#[test]
fn test_ready_json_multiple_candidates_correct_count() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create exactly 5 ready beads
    for i in 1..=5 {
        let id = format!("bf-ready-count-{}", i);
        let title = format!("Ready bead {}", i);
        ws.create_bead(&id, &title).unwrap();
    }

    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Count lines (each bead is one line in JSONL)
    let lines: Vec<&str> = trimmed.lines().collect();
    assert_eq!(lines.len(), 5, "Should have exactly 5 lines for 5 beads");
}

#[test]
fn test_ready_json_multiple_candidates_no_array_wrapper() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create multiple beads
    ws.create_bead("bf-ready-1", "First").unwrap();
    ws.create_bead("bf-ready-2", "Second").unwrap();

    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should NOT start with array bracket
    assert!(
        !trimmed.starts_with('['),
        "Multiple candidates should use JSONL, not array wrapper"
    );

    // Should NOT have commas between lines (JSONL format)
    assert!(
        !trimmed.contains(",\n"),
        "JSONL should not have comma separators"
    );
}

#[test]
fn test_ready_json_multiple_candidates_maintains_order() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create beads in specific order
    let ids = vec!["bf-first", "bf-second", "bf-third"];
    for id in &ids {
        ws.create_bead(id, &format!("Bead {}", id)).unwrap();
    }

    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Parse all lines and verify order
    let lines: Vec<&str> = trimmed.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let parsed = parse_json(line);
        let id = get_string(&parsed, "id");
        assert_eq!(id, ids[i], "Bead {} should be {}", i + 1, ids[i]);
    }
}

// ============================================================================
// Field Correctness Tests
// ============================================================================

#[test]
fn test_ready_json_all_required_fields_present() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    ws.create_bead("bf-fields-test", "Test all fields").unwrap();

    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    let parsed = parse_json(trimmed);

    // All required fields
    assert!(has_field(&parsed, "id"));
    assert!(has_field(&parsed, "title"));
    assert!(has_field(&parsed, "status"));
    assert!(has_field(&parsed, "priority"));
    assert!(has_field(&parsed, "issue_type"));
    assert!(has_field(&parsed, "created_at"));
    assert!(has_field(&parsed, "updated_at"));
    assert!(has_field(&parsed, "source_repo"));
}

#[test]
fn test_ready_json_optional_fields_present_when_set() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    let bead_id = "bf-optional-fields";
    ws.create_bead(bead_id, "Test optional fields").unwrap();

    // Set optional fields
    let storage = ws.storage().unwrap();
    let changes = bead_forge::IssueChanges {
        description: Some("Test description".to_string()),
        design: Some("Test design".to_string()),
        acceptance_criteria: Some("Test criteria".to_string()),
        notes: Some("Test notes".to_string()),
        assignee: Some("test-assignee".to_string()),
        labels: Some(vec!["label1".to_string(), "label2".to_string()]),
        ..Default::default()
    };
    storage.update_issue(bead_id, &changes).unwrap();

    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    let parsed = parse_json(trimmed);

    // Verify optional fields are present
    assert!(has_field(&parsed, "description"));
    assert!(has_field(&parsed, "design"));
    assert!(has_field(&parsed, "acceptance_criteria"));
    assert!(has_field(&parsed, "notes"));
    assert!(has_field(&parsed, "assignee"));
    assert!(has_field(&parsed, "labels"));

    // Verify values
    assert_eq!(get_string(&parsed, "description"), "Test description");
    assert_eq!(get_string(&parsed, "design"), "Test design");
    assert_eq!(get_string(&parsed, "acceptance_criteria"), "Test criteria");
    assert_eq!(get_string(&parsed, "notes"), "Test notes");
    assert_eq!(get_string(&parsed, "assignee"), "test-assignee");

    let labels = get_array(&parsed, "labels");
    assert_eq!(labels.len(), 2);
}

#[test]
fn test_ready_json_dependencies_and_comments_stripped() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create bead with dependencies
    let bead_id = "bf-deps-test";
    ws.create_bead(bead_id, "Test with dependencies").unwrap();

    // Add dependencies and comments
    let storage = ws.storage().unwrap();
    storage
        .add_dependency(
            bead_id,
            "bf-other",
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();

    storage
        .add_comment(bead_id, "test-comment", "Test comment body")
        .unwrap();

    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    let parsed = parse_json(trimmed);

    // Dependencies and comments should be stripped/empty
    let deps = parsed.get("dependencies");
    match deps {
        Some(dep_value) => {
            let dep_array = dep_value.as_array();
            assert!(
                dep_array.is_some() && dep_array.unwrap().is_empty(),
                "dependencies should be empty or absent"
            );
        }
        None => {
            // Also acceptable to be absent
        }
    }

    let comments = parsed.get("comments");
    match comments {
        Some(comment_value) => {
            let comment_array = comment_value.as_array();
            assert!(
                comment_array.is_some() && comment_array.unwrap().is_empty(),
                "comments should be empty or absent"
            );
        }
        None => {
            // Also acceptable to be absent
        }
    }
}

#[test]
fn test_ready_json_field_types_correct() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    ws.create_bead("bf-types-test", "Test field types").unwrap();

    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    let parsed = parse_json(trimmed);

    // Check field types
    assert!(parsed.get("id").and_then(|v| v.as_str()).is_some(), "id should be string");
    assert!(parsed.get("title").and_then(|v| v.as_str()).is_some(), "title should be string");
    assert!(parsed.get("status").and_then(|v| v.as_str()).is_some(), "status should be string");
    assert!(parsed.get("priority").and_then(|v| v.as_i64()).is_some(), "priority should be integer");
    assert!(
        parsed.get("issue_type").and_then(|v| v.as_str()).is_some(),
        "issue_type should be string"
    );
    assert!(
        parsed.get("created_at").and_then(|v| v.as_str()).is_some(),
        "created_at should be string (ISO timestamp)"
    );
    assert!(
        parsed.get("updated_at").and_then(|v| v.as_str()).is_some(),
        "updated_at should be string (ISO timestamp)"
    );
}

// ============================================================================
// Format Switching Tests
// ============================================================================

#[test]
fn test_ready_json_format_vs_text_format() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    ws.create_bead("bf-format-test", "Test format switching").unwrap();

    // Test JSON format
    let json_output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(json_output.status.success(), "bf ready json should succeed");

    let json_stdout = String::from_utf8(json_output.stdout).expect("Invalid UTF-8");

    // JSON should be parseable
    parse_json(json_stdout.trim());

    // Test text format
    let text_output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("text")
        .output()
        .expect("Failed to execute bf ready");

    assert!(text_output.status.success(), "bf ready text should succeed");

    let text_stdout = String::from_utf8(text_output.stdout).expect("Invalid UTF-8");

    // Text should NOT be parseable as JSON
    let result = serde_json::from_str::<Value>(&text_stdout.trim());
    assert!(
        result.is_err(),
        "Text format should not be valid JSON, got: {:?}",
        text_stdout
    );

    // Text should contain bead info in human-readable format
    assert!(
        text_stdout.contains("bf-format-test") || text_stdout.contains("Test format switching"),
        "Text output should contain bead information"
    );
}

#[test]
fn test_ready_json_format_vs_toon_format() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    ws.create_bead("bf-toon-test", "Test toon format").unwrap();

    // Test JSON format
    let json_output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(json_output.status.success(), "bf ready json should succeed");

    let json_stdout = String::from_utf8(json_output.stdout).expect("Invalid UTF-8");
    parse_json(json_stdout.trim()); // Should be valid JSON

    // Test toon format
    let toon_output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("toon")
        .output()
        .expect("Failed to execute bf ready");

    assert!(toon_output.status.success(), "bf ready toon should succeed");

    let toon_stdout = String::from_utf8(toon_output.stdout).expect("Invalid UTF-8");

    // Toon should NOT be parseable as JSON
    let result = serde_json::from_str::<Value>(&toon_stdout.trim());
    assert!(
        result.is_err(),
        "Toon format should not be valid JSON, got: {:?}",
        toon_stdout
    );
}

#[test]
fn test_ready_format_flag_defaults_to_text() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    ws.create_bead("bf-default-test", "Test default format").unwrap();

    // Test without --format flag (should default to text)
    let default_output = bf_command(&ws)
        .arg("ready")
        .output()
        .expect("Failed to execute bf ready");

    assert!(default_output.status.success(), "bf ready should succeed");

    let default_stdout = String::from_utf8(default_output.stdout).expect("Invalid UTF-8");

    // Default should NOT be JSON
    let result = serde_json::from_str::<Value>(&default_stdout.trim());
    assert!(
        result.is_err(),
        "Default format should not be JSON, got: {:?}",
        default_stdout
    );
}

// ============================================================================
// Exit Code Tests
// ============================================================================

#[test]
fn test_ready_json_exit_code_success_with_results() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    ws.create_bead("bf-exit-test", "Test exit code").unwrap();

    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert_eq!(
        output.status.code().unwrap(),
        0,
        "Exit code should be 0 when command succeeds with results"
    );
}

#[test]
fn test_ready_json_exit_code_success_empty_results() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Empty workspace
    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert_eq!(
        output.status.code().unwrap(),
        0,
        "Exit code should be 0 even with empty results"
    );
}

#[test]
fn test_ready_json_exit_code_error_invalid_workspace() {
    require_binary();

    // Use non-existent workspace
    let output = Command::new(&bf_binary())
        .arg("-w")
        .arg("/nonexistent/workspace")
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(
        !output.status.success(),
        "Exit code should be non-zero for invalid workspace"
    );

    assert!(
        output.status.code().map(|c| c > 0).unwrap_or(true),
        "Exit code should be greater than 0 for errors"
    );
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[test]
fn test_ready_json_handles_special_characters() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    let bead_id = "bf-special";
    ws.create_bead(bead_id, "Bead with special chars: \"quotes\", \\backslashes\").
        unwrap();

    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should parse successfully despite special characters
    let parsed = parse_json(trimmed);

    let title = get_string(&parsed, "title");
    assert!(title.contains("quotes"), "Special characters should be preserved");
}

#[test]
fn test_ready_json_handles_unicode() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    let bead_id = "bf-unicode";
    ws.create_bead(bead_id, "Unicode test: ñ, emoji 🎉, chinese 中文").unwrap();

    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should parse successfully
    let parsed = parse_json(trimmed);

    let title = get_string(&parsed, "title");
    assert!(title.contains("🎉"), "Emoji should be preserved");
    assert!(title.contains("中文"), "Chinese characters should be preserved");
}

#[test]
fn test_ready_json_handles_newlines_in_description() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    let bead_id = "bf-newlines";
    ws.create_bead(bead_id, "Bead with newlines in description").unwrap();

    // Add description with newlines
    let storage = ws.storage().unwrap();
    let changes = bead_forge::IssueChanges {
        description: Some("Line 1\nLine 2\nLine 3".to_string()),
        ..Default::default()
    };
    storage.update_issue(bead_id, &changes).unwrap();

    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should parse successfully despite newlines
    let parsed = parse_json(trimmed);

    let description = get_string(&parsed, "description");
    assert!(description.contains("Line 1"), "Newlines should be preserved in description");
}

#[test]
fn test_ready_json_limit_parameter_works() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    // Create 5 ready beads
    for i in 1..=5 {
        let id = format!("bf-limit-{}", i);
        ws.create_bead(&id, &format!("Bead {}", i)).unwrap();
    }

    // Request only 2
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

    let lines: Vec<&str> = trimmed.lines().collect();
    assert_eq!(lines.len(), 2, "Should return exactly 2 beads when --limit 2");
}

#[test]
fn test_ready_json_with_envelope_wrapper() {
    require_binary();

    let ws = common::TempWorkspace::new().unwrap();

    ws.create_bead("bf-envelope-test", "Test envelope wrapping").unwrap();

    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .arg("--envelope")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success(), "bf ready should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should parse as envelope
    let parsed = parse_json(trimmed);

    // Check envelope structure
    let version = parsed.get("version").and_then(|v| v.as_i64());
    assert_eq!(version, Some(1), "Envelope version should be 1");

    let kind = parsed.get("kind").and_then(|k| k.as_str());
    assert_eq!(kind, Some("ready"), "Envelope kind should be 'ready'");

    assert!(parsed.get("data").is_some(), "Envelope should have 'data' field");

    // Data should be an array
    let data = parsed.get("data").and_then(|d| d.as_array());
    assert!(data.is_some(), "Envelope data should be an array");

    let data_array = data.unwrap();
    assert!(data_array.len() >= 1, "Envelope data should contain at least one bead");

    // First bead should have our test bead
    let first_bead = &data_array[0];
    assert_eq!(
        get_string(first_bead, "id"),
        "bf-envelope-test"
    );
}
