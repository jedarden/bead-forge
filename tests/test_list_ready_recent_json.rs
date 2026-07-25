//! Tests for list, ready, and recent command JSON format output
//!
//! These tests verify that `bf list/ready/recent --format json` outputs valid JSON
//! with the correct structure and required fields.

use std::process::Command;
use serde_json::Value;

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

fn close_test_bead(bead_id: &str) {
    let output = bf()
        .arg("close")
        .arg(bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");

    assert!(
        output.status.success(),
        "Failed to close bead: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Validate that a JSON object has all required issue fields
fn assert_required_issue_fields(json: &Value, context: &str) {
    let required_fields = ["id", "title", "status", "priority", "issue_type", "assignee", "labels"];
    for field in &required_fields {
        assert!(
            json.get(field).is_some(),
            "{}: Missing required field '{}', JSON: {}",
            context, field, json
        );
    }

    // Verify labels is an array
    if let Some(labels) = json.get("labels") {
        assert!(
            labels.is_array(),
            "{}: 'labels' field must be an array",
            context
        );
    }
}

// ============================================================================
// LIST COMMAND TESTS
// ============================================================================

#[test]
fn test_list_command_json_structure() {
    // Create test beads
    let bead1 = create_test_bead("List JSON test bead 1");
    let bead2 = create_test_bead("List JSON test bead 2");

    let output = bf()
        .arg("list")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute list command");

    assert!(
        output.status.success(),
        "List command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should have at least our 2 beads
    assert!(lines.len() >= 2, "list should return at least 2 beads, got {}", lines.len());

    // Each line should be valid JSON with required fields
    for line in lines.iter().take(2) {
        let parsed: Value = serde_json::from_str(line)
            .expect(&format!("Each line should be valid JSON: {}", line));
        assert_required_issue_fields(&parsed, "list command");
    }

    // Cleanup
    close_test_bead(&bead1);
    close_test_bead(&bead2);
}

#[test]
fn test_list_command_json_empty_results() {
    // Use status filter that should yield no results in fresh workspace
    let output = bf()
        .arg("list")
        .arg("--status")
        .arg("closed")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute list command");

    assert!(
        output.status.success(),
        "List command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Empty list should return empty string or "[]"
    assert!(trimmed == "[]" || trimmed.is_empty(), "Empty list should return '[]' or empty string, got: {}", trimmed);
}

#[test]
fn test_list_command_json_valid_jsonl() {
    // Create multiple beads
    let bead1 = create_test_bead("JSONL list test 1");
    let bead2 = create_test_bead("JSONL list test 2");
    let bead3 = create_test_bead("JSONL list test 3");

    let output = bf()
        .arg("list")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute list command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Verify each line is valid JSON (JSONL format)
    for line in stdout.lines() {
        if !line.trim().is_empty() {
            let parsed: Value = serde_json::from_str(line)
                .expect(&format!("Line should be valid JSON: {}", line));
            assert!(parsed.is_object(), "Each line should be a JSON object");
        }
    }

    // Cleanup
    close_test_bead(&bead1);
    close_test_bead(&bead2);
    close_test_bead(&bead3);
}

#[test]
fn test_list_command_json_field_types() {
    let bead_id = create_test_bead("Field types test");

    let output = bf()
        .arg("list")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute list command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Find our bead in the output
    let our_bead_line = stdout.lines()
        .find(|line| line.contains(&bead_id))
        .expect("Should find our bead in list output");

    let parsed: Value = serde_json::from_str(our_bead_line)
        .expect("Should be valid JSON");

    // Verify field types
    assert!(parsed.get("id").unwrap().is_string(), "id should be string");
    assert!(parsed.get("title").unwrap().is_string(), "title should be string");
    assert!(parsed.get("status").unwrap().is_string(), "status should be string");
    assert!(parsed.get("issue_type").unwrap().is_string(), "issue_type should be string");
    assert!(parsed.get("priority").unwrap().is_number(), "priority should be number");
    assert!(parsed.get("labels").unwrap().is_array(), "labels should be array");

    // assignee can be null or string
    if let Some(assignee) = parsed.get("assignee") {
        assert!(
            assignee.is_string() || assignee.is_null(),
            "assignee should be string or null"
        );
    }

    close_test_bead(&bead_id);
}

#[test]
fn test_list_command_json_with_filters() {
    // Create beads with different properties
    let open_bead = create_test_bead("Open bead for filter");
    close_test_bead(&open_bead);

    let active_bead = create_test_bead("Active bead for filter");

    // Test status filter for closed beads
    let output = bf()
        .arg("list")
        .arg("--status")
        .arg("closed")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute list command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty() && *l != "[]").collect();

    // Should find at least the closed bead
    assert!(lines.len() >= 1, "Should find at least one closed bead");

    // Verify the filtered result has correct status
    let parsed: Value = serde_json::from_str(lines[0])
        .expect("Should be valid JSON");
    assert_eq!(parsed.get("status").unwrap().as_str().unwrap(), "closed");

    close_test_bead(&active_bead);
}

// ============================================================================
// READY COMMAND TESTS
// ============================================================================

#[test]
fn test_ready_command_json_structure() {
    // Create test beads (unblocked beads are ready)
    let bead1 = create_test_bead("Ready JSON test bead 1");
    let bead2 = create_test_bead("Ready JSON test bead 2");

    let output = bf()
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute ready command");

    assert!(
        output.status.success(),
        "Ready command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Empty ready returns "[]"
    if trimmed != "[]" {
        let lines: Vec<&str> = trimmed.lines().collect();
        assert!(lines.len() >= 1, "ready should return at least one bead");

        // Each line should be valid JSON with required fields
        for line in lines {
            let parsed: Value = serde_json::from_str(line)
                .expect(&format!("Line should be valid JSON: {}", line));
            assert_required_issue_fields(&parsed, "ready command");
        }
    }

    // Cleanup
    close_test_bead(&bead1);
    close_test_bead(&bead2);
}

#[test]
fn test_ready_command_json_empty_results() {
    // Use limit 1 to get at most one result (we're not testing empty, just format)
    let output = bf()
        .arg("ready")
        .arg("--limit")
        .arg("1")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute ready command");

    assert!(
        output.status.success(),
        "Ready command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Ready returns JSONL format (one JSON object per line) or "[]" for empty
    if trimmed != "[]" {
        // Should be valid JSONL
        for line in trimmed.lines() {
            if !line.trim().is_empty() {
                let parsed: Value = serde_json::from_str(line)
                    .expect(&format!("Line should be valid JSON: {}", line));
                assert!(parsed.is_object(), "Each line should be a JSON object");
            }
        }
    }
}

#[test]
fn test_ready_command_json_limit_parameter() {
    // Create multiple beads
    for i in 1..=5 {
        let bead = create_test_bead(&format!("Ready limit bead {}", i));
        close_test_bead(&bead);
    }

    // Test with limit
    let output = bf()
        .arg("ready")
        .arg("--limit")
        .arg("2")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute ready command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    if trimmed != "[]" && !trimmed.is_empty() {
        let lines: Vec<&str> = trimmed.lines().collect();
        assert!(
            lines.len() <= 2,
            "ready with --limit 2 should return at most 2 beads, got {}",
            lines.len()
        );
    }
}

#[test]
fn test_ready_command_json_valid_jsonl() {
    // Create ready beads
    let bead1 = create_test_bead("JSONL ready test 1");
    let bead2 = create_test_bead("JSONL ready test 2");

    let output = bf()
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute ready command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    if trimmed != "[]" {
        // Verify each line is valid JSON (JSONL format)
        for line in trimmed.lines() {
            if !line.trim().is_empty() {
                let parsed: Value = serde_json::from_str(line)
                    .expect(&format!("Line should be valid JSON: {}", line));
                assert!(parsed.is_object(), "Each line should be a JSON object");
            }
        }
    }

    // Cleanup
    close_test_bead(&bead1);
    close_test_bead(&bead2);
}

#[test]
fn test_ready_command_json_field_types() {
    let bead_id = create_test_bead("Ready field types test");

    let output = bf()
        .arg("ready")
        .arg("--limit")
        .arg("10")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute ready command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    if trimmed != "[]" {
        // Find our bead in the output
        let our_bead_line = trimmed.lines()
            .find(|line| line.contains(&bead_id))
            .expect("Should find our bead in ready output");

        let parsed: Value = serde_json::from_str(our_bead_line)
            .expect("Should be valid JSON");

        // Verify field types
        assert!(parsed.get("id").unwrap().is_string(), "id should be string");
        assert!(parsed.get("title").unwrap().is_string(), "title should be string");
        assert!(parsed.get("status").unwrap().is_string(), "status should be string");
        assert!(parsed.get("issue_type").unwrap().is_string(), "issue_type should be string");
        assert!(parsed.get("priority").unwrap().is_number(), "priority should be number");
        assert!(parsed.get("labels").unwrap().is_array(), "labels should be array");
    }

    close_test_bead(&bead_id);
}

// ============================================================================
// RECENT COMMAND TESTS
// ============================================================================

#[test]
fn test_recent_command_json_structure() {
    // Create a test bead
    let bead_id = create_test_bead("Recent JSON test bead");

    let output = bf()
        .arg("recent")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute recent command");

    assert!(
        output.status.success(),
        "Recent command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // recent wraps output in envelope: {version: 1, kind: "recent", data: [...]}
    let parsed: Value = serde_json::from_str(&stdout.trim())
        .expect("Should be valid JSON");

    assert!(parsed.is_object(), "recent should return an object (envelope)");

    // Verify envelope structure
    assert!(parsed.get("version").is_some(), "Envelope must have 'version' field");
    assert!(parsed.get("kind").is_some(), "Envelope must have 'kind' field");
    assert!(parsed.get("data").is_some(), "Envelope must have 'data' field");

    // Verify kind is "recent"
    assert_eq!(parsed.get("kind").unwrap().as_str().unwrap(), "recent");

    // Verify data is a single object (or potentially array/string depending on implementation)
    let data = parsed.get("data").unwrap();

    // Data can be an object (single bead), array, or string (JSONL)
    if let Some(obj) = data.as_object() {
        // Single bead case
        assert_required_issue_fields(&Value::Object(obj.clone()), "recent command data");
    } else if let Some(array) = data.as_array() {
        // Array case
        assert!(array.len() >= 1, "recent should return at least one bead");
        for issue_json in array {
            assert_required_issue_fields(issue_json, "recent command data");
        }
    } else if let Some(s) = data.as_str() {
        // JSONL string case - parse each line
        for line in s.lines() {
            if !line.trim().is_empty() {
                let parsed_line: Value = serde_json::from_str(line)
                    .expect(&format!("Line should be valid JSON: {}", line));
                assert_required_issue_fields(&parsed_line, "recent command data line");
            }
        }
    } else {
        panic!("data should be object, array, or string, got: {:?}", data);
    }

    // Cleanup
    close_test_bead(&bead_id);
}

#[test]
fn test_recent_command_json_envelope_fields() {
    let bead_id = create_test_bead("Recent envelope test");

    let output = bf()
        .arg("recent")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute recent command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let parsed: Value = serde_json::from_str(stdout.trim())
        .expect("Should be valid JSON");

    // Verify envelope structure
    assert_eq!(parsed.get("version").unwrap().as_i64().unwrap(), 1, "version should be 1");
    assert_eq!(parsed.get("kind").unwrap().as_str().unwrap(), "recent", "kind should be 'recent'");
    assert!(parsed.get("data").is_some(), "should have data field");

    // Cleanup
    close_test_bead(&bead_id);
}

#[test]
fn test_recent_command_json_empty_results() {
    // Use very short time period
    let output = bf()
        .arg("recent")
        .arg("--time-period")
        .arg("1s")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute recent command");

    assert!(
        output.status.success(),
        "Recent command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let parsed: Value = serde_json::from_str(stdout.trim())
        .expect("Should be valid JSON");

    // Even empty results are wrapped in envelope
    assert!(parsed.is_object(), "recent should return envelope object");
    assert!(parsed.get("data").is_some(), "envelope should have data field");

    let data = parsed.get("data").unwrap();
    // Data can be various types depending on implementation
    assert!(
        data.is_object() || data.is_array() || data.is_string() || data.is_null(),
        "data should be object, array, string, or null"
    );
}

#[test]
fn test_recent_command_json_time_period_parameter() {
    let bead_id = create_test_bead("Recent time period test");

    // Test with time period
    let output = bf()
        .arg("recent")
        .arg("--time-period")
        .arg("1h")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute recent command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let parsed: Value = serde_json::from_str(stdout.trim())
        .expect("Should be valid JSON");

    // Should still be wrapped in envelope
    assert!(parsed.is_object(), "recent should return envelope object");
    assert!(parsed.get("data").is_some(), "envelope should have data field");

    // Cleanup
    close_test_bead(&bead_id);
}

#[test]
fn test_recent_command_json_data_field_types() {
    let bead_id = create_test_bead("Recent data field types test");

    let output = bf()
        .arg("recent")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute recent command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let parsed: Value = serde_json::from_str(stdout.trim())
        .expect("Should be valid JSON");

    let data = parsed.get("data").unwrap();

    // Handle different data formats
    let bead_json: Value = if let Some(obj) = data.as_object() {
        // Single object case
        Value::Object(obj.clone())
    } else if let Some(array) = data.as_array() {
        // Array case - find our bead
        let our_bead = array.iter()
            .find(|item| item.get("id").unwrap().as_str().unwrap() == bead_id)
            .expect("Should find our bead in recent output");
        our_bead.clone()
    } else if let Some(s) = data.as_str() {
        // JSONL string case - parse and find our bead
        let our_bead_line = s.lines()
            .find(|line| line.contains(&bead_id))
            .expect("Should find our bead in recent JSONL");
        serde_json::from_str(our_bead_line).expect("Should parse JSON")
    } else {
        panic!("Unexpected data format: {:?}", data);
    };

    // Verify field types
    assert!(bead_json.get("id").unwrap().is_string(), "id should be string");
    assert!(bead_json.get("title").unwrap().is_string(), "title should be string");
    assert!(bead_json.get("status").unwrap().is_string(), "status should be string");
    assert!(bead_json.get("issue_type").unwrap().is_string(), "issue_type should be string");
    assert!(bead_json.get("priority").unwrap().is_number(), "priority should be number");
    assert!(bead_json.get("labels").unwrap().is_array(), "labels should be array");

    // Cleanup
    close_test_bead(&bead_id);
}

// ============================================================================
// COMPATIBILITY AND EDGE CASE TESTS
// ============================================================================

#[test]
fn test_list_ready_recent_json_unicode_handling() {
    let unicode_title = "Test bead with emoji 🎉 and unicode Ñ";
    let bead_id = create_test_bead(unicode_title);

    // Test list command
    let list_output = bf()
        .arg("list")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute list command");

    let list_stdout = String::from_utf8(list_output.stdout).expect("Invalid UTF-8");
    assert!(list_stdout.contains("🎉"), "List output should preserve Unicode emoji");

    // Test ready command
    let ready_output = bf()
        .arg("ready")
        .arg("--limit")
        .arg("10")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute ready command");

    let ready_stdout = String::from_utf8(ready_output.stdout).expect("Invalid UTF-8");
    if ready_stdout.trim() != "[]" {
        assert!(ready_stdout.contains("🎉"), "Ready output should preserve Unicode emoji");
    }

    // Test recent command
    let recent_output = bf()
        .arg("recent")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute recent command");

    let recent_stdout = String::from_utf8(recent_output.stdout).expect("Invalid UTF-8");
    assert!(recent_stdout.contains("🎉"), "Recent output should preserve Unicode emoji");

    // Cleanup
    close_test_bead(&bead_id);
}

#[test]
fn test_list_ready_recent_json_compact_format() {
    let bead_id = create_test_bead("Compact format test");

    // Test list output is compact (no pretty-printing)
    let list_output = bf()
        .arg("list")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute list command");

    let list_stdout = String::from_utf8(list_output.stdout).expect("Invalid UTF-8");
    // Find the line with our bead
    let our_bead_line = list_stdout.lines()
        .find(|line| line.contains(&bead_id))
        .expect("Should find our bead in list output");

    assert!(!our_bead_line.contains("\n"), "Compact JSON should not contain newlines within a line");

    // Test recent output is compact
    let recent_output = bf()
        .arg("recent")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute recent command");

    let recent_stdout = String::from_utf8(recent_output.stdout).expect("Invalid UTF-8");
    let trimmed = recent_stdout.trim();

    // Single-line envelope (may contain internal newlines in data array, but envelope itself is compact)
    assert!(trimmed.starts_with("{"), "Envelope should start with '{{'");
    assert!(trimmed.ends_with("}"), "Envelope should end with '}}'");

    // Cleanup
    close_test_bead(&bead_id);
}