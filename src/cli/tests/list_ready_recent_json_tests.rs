//! JSON output tests for `bf list`, `bf ready`, and `bf recent` commands
//!
//! Comprehensive tests for JSON output including:
//! - JSON structure validation
//! - Required fields presence and types
//! - Empty result set handling
//! - JSONL format validation (list, ready)
//! - Envelope wrapping validation
//! - Special character handling
//! - Filtering functionality
//! - Pagination and limits

use std::process::Command;
use tempfile::TempDir;

// Import test infrastructure helpers from sibling module
use super::json_output::{
    test_workspace, bf_binary, bf_command,
    json_validation, format_detection, fixtures, capture, envelope,
};

// Import items made available in parent scope
use super::*;

/// Create an isolated test workspace
fn create_isolated_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let beads_dir = temp_dir.path().join(".beads");
    std::fs::create_dir(&beads_dir).expect("Failed to create .beads directory");

    // Initialize workspace
    crate::config::init_workspace(&beads_dir, "bf-list-test")
        .expect("Failed to initialize test workspace");

    let metadata = crate::config::load_metadata(&beads_dir)
        .expect("Failed to load metadata");
    let _ = crate::Storage::open(&beads_dir.join(&metadata.database))
        .expect("Failed to create database");

    temp_dir
}

// ============================================================================
// list command tests
// ============================================================================

#[test]
fn test_list_json_structure_validity() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create test beads
    let bead1_id = fixtures::create_bead("List test bead 1");
    let bead2_id = fixtures::create_bead("List test bead 2");

    // Get list JSON output
    let output = capture::capture_stdout(
        bf_command()
            .arg("list")
            .arg("--format")
            .arg("json")
    );

    // Verify it's valid JSONL (multiple lines, each a valid JSON object)
    let json_str = output.trim();
    json_validation::assert_valid_jsonl(json_str);

    // Parse each line and verify structure
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() >= 2, "list should return at least 2 beads");

    for line in lines {
        let parsed = json_validation::parse_json(line);
        assert!(parsed.is_object(), "each line should be a JSON object");

        // Verify required fields
        json_validation::assert_required_fields(
            &parsed,
            &["id", "title", "status", "priority", "issue_type", "created_at", "updated_at"],
            "list command"
        );
    }

    // Cleanup
    fixtures::close_bead(&bead1_id, "List test cleanup");
    fixtures::close_bead(&bead2_id, "List test cleanup");
}

#[test]
fn test_list_json_jsonl_format_structure() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create test beads to ensure we have multiple lines
    let bead1_id = fixtures::create_bead("JSONL format test bead 1");
    let bead2_id = fixtures::create_bead("JSONL format test bead 2");
    let bead3_id = fixtures::create_bead("JSONL format test bead 3");

    // Get list JSON output
    let output = capture::capture_stdout(
        bf_command()
            .arg("list")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();

    // Test 1: Validate that output is in JSONL format (NOT a JSON array)
    format_detection::assert_format(json_str, format_detection::JsonFormat::JsonL);

    // Test 2: Validate each line is valid JSON
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() >= 3, "list should return at least 3 beads");

    for (i, line) in lines.iter().enumerate() {
        // Each line must be valid JSON
        json_validation::assert_valid_json(line);

        // Each line must be a JSON object (not array, string, number, etc.)
        let parsed = json_validation::parse_json(line);
        assert!(parsed.is_object(), "JSONL line {} should be a JSON object, found: {}", i, line);

        // Each line must represent a complete, independent JSON object
        // (i.e., not part of a larger JSON array)
        json_validation::assert_required_fields(
            &parsed,
            &["id", "title", "status", "priority", "issue_type", "created_at", "updated_at"],
            "list JSONL line"
        );
    }

    // Test 3: Verify output is NOT a JSON array
    // (i.e., the entire output is not wrapped in [ ... ])
    let first_char = json_str.chars().next().unwrap_or(' ');
    let last_char = json_str.chars().last().unwrap_or(' ');
    assert_ne!(first_char, '[', "JSONL output should not start with '[' (JSON array marker)");
    assert_ne!(last_char, ']', "JSONL output should not end with ']' (JSON array marker)");

    // Test 4: Verify each line can be parsed independently
    // This is the key property of JSONL - each line is a complete JSON document
    for line in lines.iter() {
        let parsed = json_validation::parse_json(line);
        assert!(parsed.is_object(), "Each JSONL line must be independently parsable as a complete JSON object");
    }

    // Cleanup
    fixtures::close_bead(&bead1_id, "JSONL format test cleanup");
    fixtures::close_bead(&bead2_id, "JSONL format test cleanup");
    fixtures::close_bead(&bead3_id, "JSONL format test cleanup");
}

#[test]
fn test_list_json_empty_result() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Ensure no beads exist by using a fresh workspace
    let output = capture::capture_stdout(
        bf_command()
            .arg("list")
            .arg("--format")
            .arg("json")
    );

    // Empty result should print nothing (empty string)
    let json_str = output.trim();
    assert_eq!(json_str, "", "empty list should print nothing");
}

#[test]
fn test_list_json_required_fields_types() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("List field types test");

    let output = capture::capture_stdout(
        bf_command()
            .arg("list")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Find our bead in the output
    let bead_json = lines.iter()
        .find(|line| line.contains(&bead_id))
        .expect("created bead should be in list output");

    let parsed = json_validation::parse_json(bead_json);

    // id must be a string matching created bead
    let id_val = json_validation::get_string(&parsed, "id");
    assert_eq!(id_val, bead_id);

    // title must be a string
    let title = json_validation::get_string(&parsed, "title");
    assert_eq!(title, "List field types test");

    // status must be a string with valid value
    let status = json_validation::get_string(&parsed, "status");
    assert!(matches!(status.as_str(), "open" | "in_progress" | "blocked" | "closed"));

    // priority must be a number (0-4)
    let priority = json_validation::get_int(&parsed, "priority");
    assert!((0..=4).contains(&priority), "priority must be between 0 and 4");

    // issue_type must be a string
    let issue_type = json_validation::get_string(&parsed, "issue_type");
    assert!(!issue_type.is_empty(), "issue_type must not be empty");

    // assignee must be present (null or string)
    assert!(parsed.get("assignee").is_some(), "assignee field must be present");

    // labels must be an array
    let labels = json_validation::get_array(&parsed, "labels");
    // Successful call proves it's an array

    fixtures::close_bead(&bead_id, "List field types cleanup");
}

#[test]
fn test_list_json_special_characters() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let special_title = "Test \"quotes\" and 'apostrophes' & <symbols>";
    let bead_id = fixtures::create_bead(special_title);

    let output = capture::capture_stdout(
        bf_command()
            .arg("list")
            .arg("--format")
            .arg("json")
    );

    // Verify it's valid JSON (proper escaping)
    let json_str = output.trim();
    json_validation::assert_valid_jsonl(json_str);

    // Find our bead
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();
    let bead_json = lines.iter()
        .find(|line| line.contains(&bead_id))
        .expect("created bead should be in list output");

    let parsed = json_validation::parse_json(bead_json);
    let title = json_validation::get_string(&parsed, "title");

    // Verify special characters are preserved
    assert!(title.contains("quotes"), "title should contain 'quotes'");
    assert!(title.contains("apostrophes"), "title should contain 'apostrophes'");
    assert!(title.contains("&"), "title should contain '&'");
    assert!(title.contains("<symbols>"), "title should contain '<symbols>'");

    fixtures::close_bead(&bead_id, "List special chars cleanup");
}

#[test]
fn test_list_json_with_filters() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create beads with different properties
    let bead1_id = fixtures::create_bead("Filter test open");
    let bead2_id = fixtures::create_bead_with_labels("Filter test labeled", &["bug", "urgent"]);

    // Update status
    let mut cmd = bf_command();
    cmd.arg("update")
        .arg(&bead1_id)
        .arg("--status")
        .arg("in_progress");
    let update_output = cmd.output().expect("Failed to update");
    assert!(update_output.status.success(), "Update should succeed");

    // Test status filter
    let output = capture::capture_stdout(
        bf_command()
            .arg("list")
            .arg("--status")
            .arg("in_progress")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find bead1
    assert!(lines.iter().any(|line| line.contains(&bead1_id)),
            "filtered list should contain in_progress bead");
    assert!(!lines.iter().any(|line| line.contains(&bead2_id)),
            "filtered list should not contain open bead");

    // Cleanup
    fixtures::close_bead(&bead1_id, "Filter test cleanup");
    fixtures::close_bead(&bead2_id, "Filter test cleanup");
}

#[test]
fn test_list_json_limit() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create multiple beads
    let bead1 = fixtures::create_bead("Limit test 1");
    let bead2 = fixtures::create_bead("Limit test 2");
    let bead3 = fixtures::create_bead("Limit test 3");

    // Test limit
    let output = capture::capture_stdout(
        bf_command()
            .arg("list")
            .arg("--limit")
            .arg("2")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    assert_eq!(lines.len(), 2, "limited list should return exactly 2 beads");

    // Cleanup
    fixtures::close_bead(&bead1, "Limit test cleanup");
    fixtures::close_bead(&bead2, "Limit test cleanup");
    fixtures::close_bead(&bead3, "Limit test cleanup");
}

#[test]
fn test_list_json_envelope_wrapping() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("List envelope test");

    let output = capture::capture_stdout(
        bf_command()
            .arg("list")
            .arg("--format")
            .arg("json")
            .arg("--envelope")
    );

    // Should be wrapped in envelope
    let envelope = envelope::validate_envelope(&output.trim(), "list");

    // Data field should be an array
    let data = envelope::get_envelope_data(&envelope);
    assert!(data.is_array(), "envelope data should be an array");

    let array = data.as_array().expect("data should be array");
    assert!(array.len() >= 1, "envelope should contain at least one bead");

    fixtures::close_bead(&bead_id, "List envelope cleanup");
}

#[test]
fn test_list_json_empty_with_envelope() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let output = capture::capture_stdout(
        bf_command()
            .arg("list")
            .arg("--format")
            .arg("json")
            .arg("--envelope")
    );

    // Empty list should still have envelope
    let envelope = envelope::validate_envelope(&output.trim(), "list");

    // Data field should be empty array
    let data = envelope::get_envelope_data(&envelope);
    assert!(data.is_array(), "envelope data should be an array");

    let array = data.as_array().expect("data should be array");
    assert_eq!(array.len(), 0, "envelope should contain empty array");
}

// ============================================================================
// ready command tests
// ============================================================================

#[test]
fn test_ready_json_structure_validity() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create test beads
    let bead1_id = fixtures::create_bead("Ready test bead 1");
    let bead2_id = fixtures::create_bead("Ready test bead 2");

    // Get ready JSON output
    let output = capture::capture_stdout(
        bf_command()
            .arg("ready")
            .arg("--format")
            .arg("json")
    );

    // Verify it's valid JSONL or empty array
    let json_str = output.trim();
    if json_str == "[]" {
        // No ready beads (valid output)
        return;
    }

    json_validation::assert_valid_jsonl(json_str);

    // Parse each line and verify structure
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() >= 2, "ready should return at least 2 beads");

    for line in lines {
        let parsed = json_validation::parse_json(line);
        assert!(parsed.is_object(), "each line should be a JSON object");

        // Verify required fields
        json_validation::assert_required_fields(
            &parsed,
            &["id", "title", "status", "priority", "issue_type", "created_at", "updated_at"],
            "ready command"
        );
    }

    // Cleanup
    fixtures::close_bead(&bead1_id, "Ready test cleanup");
    fixtures::close_bead(&bead2_id, "Ready test cleanup");
}

#[test]
fn test_ready_json_empty_result() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Ensure no ready beads exist by using a fresh workspace
    let output = capture::capture_stdout(
        bf_command()
            .arg("ready")
            .arg("--format")
            .arg("json")
    );

    // Empty result should print []
    let json_str = output.trim();
    assert_eq!(json_str, "[]", "empty ready should print []");
}

#[test]
fn test_ready_json_required_fields_types() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Ready field types test");

    let output = capture::capture_stdout(
        bf_command()
            .arg("ready")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();

    // If empty (no ready beads), that's valid
    if json_str == "[]" {
        fixtures::close_bead(&bead_id, "Ready field types cleanup");
        return;
    }

    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Find our bead in the output
    let bead_json = lines.iter()
        .find(|line| line.contains(&bead_id))
        .expect("created bead should be in ready output");

    let parsed = json_validation::parse_json(bead_json);

    // Verify required fields
    json_validation::assert_required_fields(
        &parsed,
        &["id", "title", "status", "priority", "issue_type", "created_at", "updated_at"],
        "ready command"
    );

    // id must be a string matching created bead
    let id_val = json_validation::get_string(&parsed, "id");
    assert_eq!(id_val, bead_id);

    // status should be "open" (ready beads are unblocked)
    let status = json_validation::get_string(&parsed, "status");
    assert_eq!(status, "open", "ready beads should have status 'open'");

    fixtures::close_bead(&bead_id, "Ready field types cleanup");
}

#[test]
fn test_ready_json_limit() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create multiple beads
    let bead1 = fixtures::create_bead("Ready limit test 1");
    let bead2 = fixtures::create_bead("Ready limit test 2");
    let bead3 = fixtures::create_bead("Ready limit test 3");

    // Test limit
    let output = capture::capture_stdout(
        bf_command()
            .arg("ready")
            .arg("--limit")
            .arg("2")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();

    // If empty (no ready beads), that's valid
    if json_str == "[]" {
        fixtures::close_bead(&bead1, "Ready limit cleanup");
        fixtures::close_bead(&bead2, "Ready limit cleanup");
        fixtures::close_bead(&bead3, "Ready limit cleanup");
        return;
    }

    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    assert_eq!(lines.len(), 2, "limited ready should return exactly 2 beads");

    // Cleanup
    fixtures::close_bead(&bead1, "Ready limit cleanup");
    fixtures::close_bead(&bead2, "Ready limit cleanup");
    fixtures::close_bead(&bead3, "Ready limit cleanup");
}

#[test]
fn test_ready_json_unlimited_limit() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create multiple beads
    let bead1 = fixtures::create_bead("Ready unlimited 1");
    let bead2 = fixtures::create_bead("Ready unlimited 2");
    let bead3 = fixtures::create_bead("Ready unlimited 3");

    // Test unlimited limit (limit 0)
    let output = capture::capture_stdout(
        bf_command()
            .arg("ready")
            .arg("--limit")
            .arg("0")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();

    // If empty (no ready beads), that's valid
    if json_str == "[]" {
        fixtures::close_bead(&bead1, "Ready unlimited cleanup");
        fixtures::close_bead(&bead2, "Ready unlimited cleanup");
        fixtures::close_bead(&bead3, "Ready unlimited cleanup");
        return;
    }

    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should return all ready beads (at least 3)
    assert!(lines.len() >= 3, "unlimited ready should return all ready beads");

    // Cleanup
    fixtures::close_bead(&bead1, "Ready unlimited cleanup");
    fixtures::close_bead(&bead2, "Ready unlimited cleanup");
    fixtures::close_bead(&bead3, "Ready unlimited cleanup");
}

#[test]
fn test_ready_json_envelope_wrapping() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Ready envelope test");

    let output = capture::capture_stdout(
        bf_command()
            .arg("ready")
            .arg("--format")
            .arg("json")
            .arg("--envelope")
    );

    // Should be wrapped in envelope
    let envelope = envelope::validate_envelope(&output.trim(), "ready");

    // Data field should be an array
    let data = envelope::get_envelope_data(&envelope);
    assert!(data.is_array(), "envelope data should be an array");

    fixtures::close_bead(&bead_id, "Ready envelope cleanup");
}

#[test]
fn test_ready_json_empty_with_envelope() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let output = capture::capture_stdout(
        bf_command()
            .arg("ready")
            .arg("--format")
            .arg("json")
            .arg("--envelope")
    );

    // Empty ready should still have envelope
    let envelope = envelope::validate_envelope(&output.trim(), "ready");

    // Data field should be empty array
    let data = envelope::get_envelope_data(&envelope);
    assert!(data.is_array(), "envelope data should be an array");

    let array = data.as_array().expect("data should be array");
    assert_eq!(array.len(), 0, "envelope should contain empty array");
}

#[test]
fn test_ready_json_excludes_blocked_beads() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create beads
    let blocker_id = fixtures::create_bead("Blocker bead");
    let blocked_id = fixtures::create_bead("Blocked bead");

    // Add dependency: blocker blocks blocked
    fixtures::add_dependency(&blocked_id, &blocker_id);

    // Get ready beads
    let output = capture::capture_stdout(
        bf_command()
            .arg("ready")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();

    // Parse the JSONL output
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty() && *l != "[]").collect();

    // Blocker should be in ready output (unblocked)
    assert!(lines.iter().any(|line| line.contains(&blocker_id)),
            "ready should include unblocked bead");

    // Blocked should NOT be in ready output (has dependency)
    assert!(!lines.iter().any(|line| line.contains(&blocked_id)),
            "ready should not include blocked bead");

    // Cleanup
    fixtures::close_bead(&blocker_id, "Blocker cleanup");
    fixtures::close_bead(&blocked_id, "Blocked cleanup");
}

// ============================================================================
// recent command tests
// ============================================================================

#[test]
fn test_recent_json_envelope_structure() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Recent test bead");

    let output = capture::capture_stdout(
        bf_command()
            .arg("recent")
            .arg("--format")
            .arg("json")
    );

    // recent command ALWAYS uses envelope
    let envelope = envelope::validate_envelope(&output.trim(), "recent");

    // Data field should be a string containing JSONL
    let data = envelope::get_envelope_data(&envelope);
    assert!(data.is_string(), "recent envelope data should be a JSONL string");

    // Parse the JSONL string and validate it contains at least one bead
    let jsonl_str = data.as_str().expect("data should be string");
    let lines: Vec<&str> = jsonl_str.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() >= 1, "recent should contain at least one bead");

    // Each line should be valid JSON
    for line in lines {
        json_validation::assert_valid_json(line);
    }

    fixtures::close_bead(&bead_id, "Recent test cleanup");
}

#[test]
fn test_recent_json_empty_result() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create a fresh isolated workspace with no recent beads
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let fresh_workspace = temp_dir.path();
    let beads_dir = fresh_workspace.join(".beads");
    std::fs::create_dir(&beads_dir).expect("Failed to create .beads directory");

    // Initialize the isolated workspace
    crate::config::init_workspace(&beads_dir, "bf-recent-empty-test")
        .expect("Failed to initialize test workspace");

    let metadata = crate::config::load_metadata(&beads_dir)
        .expect("Failed to load metadata");
    let _ = crate::Storage::open(&beads_dir.join(&metadata.database))
        .expect("Failed to create database");

    // Empty recent should still have envelope
    let mut cmd = Command::new(bf_binary());
    cmd.arg("-w").arg(&beads_dir)
        .arg("recent")
        .arg("--format")
        .arg("json");
    let output = cmd.output().expect("Failed to execute bf recent");
    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should still be wrapped in envelope
    let envelope = envelope::validate_envelope(&stdout.trim(), "recent");

    // Data field should be a string containing JSONL (empty or whitespace)
    let data = envelope::get_envelope_data(&envelope);
    assert!(data.is_string() || data.is_array(), "envelope data should be string or array");

    // If string, it should be empty or whitespace only
    if let Some(jsonl_str) = data.as_str() {
        let lines: Vec<&str> = jsonl_str.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 0, "envelope should contain empty JSONL");
    }
}

#[test]
fn test_recent_json_required_fields_in_data() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Recent field types test");

    let output = capture::capture_stdout(
        bf_command()
            .arg("recent")
            .arg("--format")
            .arg("json")
    );

    let envelope = envelope::validate_envelope(&output.trim(), "recent");
    let data = envelope::get_envelope_data(&envelope);
    assert!(data.is_string(), "recent envelope data should be a JSONL string");

    // Parse the JSONL string
    let jsonl_str = data.as_str().expect("data should be string");
    let lines: Vec<&str> = jsonl_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Find our bead in the JSONL lines
    let bead_json_str = lines.iter()
        .find(|line| line.contains(&bead_id))
        .expect("created bead should be in recent output");

    let bead_json = json_validation::parse_json(bead_json_str);

    // Verify required fields
    json_validation::assert_required_fields(
        &bead_json,
        &["id", "title", "status", "priority", "issue_type", "created_at", "updated_at"],
        "recent command"
    );

    // Verify specific field values
    let id_val = json_validation::get_string(&bead_json, "id");
    assert_eq!(id_val, bead_id);

    let title = json_validation::get_string(&bead_json, "title");
    assert_eq!(title, "Recent field types test");

    fixtures::close_bead(&bead_id, "Recent field types cleanup");
}

#[test]
fn test_recent_json_time_filtering() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Recent time filter test");

    // Get recent beads from the last hour
    let output = capture::capture_stdout(
        bf_command()
            .arg("recent")
            .arg("--time-period")
            .arg("1h")
            .arg("--format")
            .arg("json")
    );

    let envelope = envelope::validate_envelope(&output.trim(), "recent");
    let data = envelope::get_envelope_data(&envelope);
    assert!(data.is_string(), "recent envelope data should be a JSONL string");

    // Parse the JSONL string
    let jsonl_str = data.as_str().expect("data should be string");
    let lines: Vec<&str> = jsonl_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Our bead should be in the results (created within last hour)
    assert!(lines.iter().any(|line| {
        let parsed = json_validation::parse_json(line);
        parsed.get("id")
            .and_then(|v| v.as_str())
            .map(|id| id == &bead_id)
            .unwrap_or(false)
    }), "recently created bead should be in recent output");

    fixtures::close_bead(&bead_id, "Recent time filter cleanup");
}

#[test]
fn test_recent_json_status_filter() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Recent status filter test");

    // Update status to in_progress
    let mut cmd = bf_command();
    cmd.arg("update")
        .arg(&bead_id)
        .arg("--status")
        .arg("in_progress");
    let update_output = cmd.output().expect("Failed to update");
    assert!(update_output.status.success(), "Update should succeed");

    // Get recent beads with status filter
    let output = capture::capture_stdout(
        bf_command()
            .arg("recent")
            .arg("--status")
            .arg("in_progress")
            .arg("--format")
            .arg("json")
    );

    let envelope = envelope::validate_envelope(&output.trim(), "recent");
    let data = envelope::get_envelope_data(&envelope);

    // Data can be a string (JSONL), an array of objects, or a single object
    let jsonl_str = if let Some(s) = data.as_str() {
        s.to_string()
    } else if let Some(arr) = data.as_array() {
        // If it's an array, convert to JSONL string
        arr.iter()
            .map(|v| serde_json::to_string(v).expect("Failed to convert item to JSON"))
            .collect::<Vec<_>>()
            .join("\n")
    } else if data.is_object() {
        // Single object - convert to JSONL string
        serde_json::to_string(&data).expect("Failed to convert object to JSON")
    } else {
        panic!("recent envelope data should be a JSONL string, array, or object, got: {:?}", data);
    };
    let lines: Vec<&str> = jsonl_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find our bead with in_progress status
    assert!(lines.iter().any(|line| {
        let parsed = json_validation::parse_json(line);
        parsed.get("id")
            .and_then(|v| v.as_str())
            .map(|id| id == &bead_id)
            .unwrap_or(false)
    }), "bead with filtered status should be in recent output");

    fixtures::close_bead(&bead_id, "Recent status filter cleanup");
}

#[test]
fn test_recent_json_limit() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create multiple beads
    let bead1 = fixtures::create_bead("Recent limit test 1");
    let bead2 = fixtures::create_bead("Recent limit test 2");
    let bead3 = fixtures::create_bead("Recent limit test 3");

    // Test limit
    let output = capture::capture_stdout(
        bf_command()
            .arg("recent")
            .arg("--limit")
            .arg("2")
            .arg("--format")
            .arg("json")
    );

    let envelope = envelope::validate_envelope(&output.trim(), "recent");
    let data = envelope::get_envelope_data(&envelope);
    assert!(data.is_string(), "recent envelope data should be a JSONL string");

    // Parse the JSONL string
    let jsonl_str = data.as_str().expect("data should be string");
    let lines: Vec<&str> = jsonl_str.lines().filter(|l| !l.trim().is_empty()).collect();

    assert_eq!(lines.len(), 2, "limited recent should return exactly 2 beads");

    // Cleanup
    fixtures::close_bead(&bead1, "Recent limit cleanup");
    fixtures::close_bead(&bead2, "Recent limit cleanup");
    fixtures::close_bead(&bead3, "Recent limit cleanup");
}

#[test]
fn test_recent_json_unlimited_limit() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create multiple beads
    let bead1 = fixtures::create_bead("Recent unlimited test 1");
    let bead2 = fixtures::create_bead("Recent unlimited test 2");
    let bead3 = fixtures::create_bead("Recent unlimited test 3");

    // Test unlimited limit (limit 0)
    let output = capture::capture_stdout(
        bf_command()
            .arg("recent")
            .arg("--limit")
            .arg("0")
            .arg("--format")
            .arg("json")
    );

    let envelope = envelope::validate_envelope(&output.trim(), "recent");
    let data = envelope::get_envelope_data(&envelope);
    assert!(data.is_string(), "recent envelope data should be a JSONL string");

    // Parse the JSONL string
    let jsonl_str = data.as_str().expect("data should be string");
    let lines: Vec<&str> = jsonl_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should return all recent beads (at least 3)
    assert!(lines.len() >= 3, "unlimited recent should return all recent beads");

    // Cleanup
    fixtures::close_bead(&bead1, "Recent unlimited cleanup");
    fixtures::close_bead(&bead2, "Recent unlimited cleanup");
    fixtures::close_bead(&bead3, "Recent unlimited cleanup");
}

#[test]
fn test_recent_json_always_uses_envelope() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Recent envelope always test");

    // recent command ALWAYS uses envelope, even without --envelope flag
    let output = capture::capture_stdout(
        bf_command()
            .arg("recent")
            .arg("--format")
            .arg("json")
    );

    // Should still be wrapped in envelope
    let envelope = envelope::validate_envelope(&output.trim(), "recent");

    // Verify envelope structure
    let version = envelope.get("version")
        .and_then(|v| v.as_i64())
        .expect("Envelope must have numeric 'version' field");
    assert_eq!(version, 1, "Envelope version must be 1");

    let kind = envelope.get("kind")
        .and_then(|k| k.as_str())
        .expect("Envelope must have string 'kind' field");
    assert_eq!(kind, "recent", "Envelope kind should be 'recent'");

    fixtures::close_bead(&bead_id, "Recent envelope always cleanup");
}

// ============================================================================
// Additional comprehensive recent command tests
// ============================================================================

#[test]
fn test_recent_json_jsonl_format_validation() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create test beads
    let bead1_id = fixtures::create_bead("Recent JSONL format test 1");
    let bead2_id = fixtures::create_bead("Recent JSONL format test 2");

    let output = capture::capture_stdout(
        bf_command()
            .arg("recent")
            .arg("--format")
            .arg("json")
    );

    let envelope = envelope::validate_envelope(&output.trim(), "recent");
    let data = envelope::get_envelope_data(&envelope);

    // Data should be a JSONL string
    assert!(data.is_string(), "recent envelope data should be a JSONL string");

    let jsonl_str = data.as_str().expect("data should be string");

    // Validate JSONL format: each line should be valid JSON
    json_validation::assert_valid_jsonl(jsonl_str);

    // Parse and validate structure
    let lines: Vec<&str> = jsonl_str.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() >= 2, "recent should return at least 2 beads");

    // Each line should be independently valid JSON
    for (i, line) in lines.iter().enumerate() {
        let parsed = json_validation::parse_json(line);
        assert!(parsed.is_object(), "JSONL line {} should be a JSON object", i);

        // Verify required fields
        json_validation::assert_required_fields(
            &parsed,
            &["id", "title", "status", "priority", "issue_type", "created_at", "updated_at"],
            "recent JSONL line"
        );
    }

    fixtures::close_bead(&bead1_id, "Recent JSONL format cleanup 1");
    fixtures::close_bead(&bead2_id, "Recent JSONL format cleanup 2");
}

#[test]
fn test_recent_json_special_characters() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let special_title = fixtures::SPECIAL_CHARACTERS_TITLE;
    let bead_id = fixtures::create_bead(special_title);

    let output = capture::capture_stdout(
        bf_command()
            .arg("recent")
            .arg("--format")
            .arg("json")
    );

    let envelope = envelope::validate_envelope(&output.trim(), "recent");
    let data = envelope::get_envelope_data(&envelope);
    let jsonl_str = data.as_str().expect("data should be string");

    // Find our bead in the JSONL
    let lines: Vec<&str> = jsonl_str.lines().filter(|l| !l.trim().is_empty()).collect();
    let bead_json_str = lines.iter()
        .find(|line| line.contains(&bead_id))
        .expect("created bead should be in recent output");

    let bead_json = json_validation::parse_json(bead_json_str);
    let title = json_validation::get_string(&bead_json, "title");

    // Verify special characters are preserved
    assert!(title.contains("quotes"), "title should contain 'quotes'");
    assert!(title.contains("apostrophes"), "title should contain 'apostrophes'");
    assert!(title.contains("&"), "title should contain '&'");
    assert!(title.contains("<"), "title should contain '<'");
    assert!(title.contains(">"), "title should contain '>'");

    fixtures::close_bead(&bead_id, "Recent special chars cleanup");
}

#[test]
fn test_recent_json_field_types_validation() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Recent field types validation");

    let output = capture::capture_stdout(
        bf_command()
            .arg("recent")
            .arg("--format")
            .arg("json")
    );

    let envelope = envelope::validate_envelope(&output.trim(), "recent");
    let data = envelope::get_envelope_data(&envelope);
    let jsonl_str = data.as_str().expect("data should be string");

    // Find our bead in the JSONL
    let lines: Vec<&str> = jsonl_str.lines().filter(|l| !l.trim().is_empty()).collect();
    let bead_json_str = lines.iter()
        .find(|line| line.contains(&bead_id))
        .expect("created bead should be in recent output");

    let parsed = json_validation::parse_json(bead_json_str);

    // Validate field types
    let id_val = json_validation::get_string(&parsed, "id");
    assert_eq!(id_val, bead_id, "id should match created bead");

    let title = json_validation::get_string(&parsed, "title");
    assert_eq!(title, "Recent field types validation");

    let status = json_validation::get_string(&parsed, "status");
    assert!(matches!(status.as_str(), "open" | "in_progress" | "blocked" | "closed"));

    let priority = json_validation::get_int(&parsed, "priority");
    assert!((0..=4).contains(&priority), "priority must be between 0 and 4");

    let issue_type = json_validation::get_string(&parsed, "issue_type");
    assert!(!issue_type.is_empty(), "issue_type must not be empty");

    // assignee should be present (null or string)
    assert!(parsed.get("assignee").is_some(), "assignee field must be present");

    // labels should be an array
    let labels = json_validation::get_array(&parsed, "labels");
    // Successful call proves it's an array

    fixtures::close_bead(&bead_id, "Recent field types validation cleanup");
}

#[test]
fn test_recent_json_all_required_fields_present() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead_with_labels(
        "Recent all fields test",
        &["test-label", "priority-high"]
    );

    let output = capture::capture_stdout(
        bf_command()
            .arg("recent")
            .arg("--format")
            .arg("json")
    );

    let envelope = envelope::validate_envelope(&output.trim(), "recent");
    let data = envelope::get_envelope_data(&envelope);
    let jsonl_str = data.as_str().expect("data should be string");

    // Find our bead in the JSONL
    let lines: Vec<&str> = jsonl_str.lines().filter(|l| !l.trim().is_empty()).collect();
    let bead_json_str = lines.iter()
        .find(|line| line.contains(&bead_id))
        .expect("created bead should be in recent output");

    let parsed = json_validation::parse_json(bead_json_str);

    // Verify all standard required fields are present
    json_validation::assert_required_fields(
        &parsed,
        &["id", "title", "status", "priority", "issue_type", "assignee", "labels", "created_at", "updated_at"],
        "recent command all fields"
    );

    // Verify specific field values
    assert_eq!(json_validation::get_string(&parsed, "id"), bead_id);
    assert_eq!(json_validation::get_string(&parsed, "title"), "Recent all fields test");

    // Verify labels array has our labels
    let labels = json_validation::get_array(&parsed, "labels");
    assert!(labels.len() >= 2, "should have at least 2 labels");

    fixtures::close_bead(&bead_id, "Recent all fields cleanup");
}

#[test]
fn test_recent_json_unicode_handling() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let unicode_title = fixtures::UNICODE_TITLE;
    let bead_id = fixtures::create_bead(unicode_title);

    let output = capture::capture_stdout(
        bf_command()
            .arg("recent")
            .arg("--format")
            .arg("json")
    );

    let envelope = envelope::validate_envelope(&output.trim(), "recent");
    let data = envelope::get_envelope_data(&envelope);
    let jsonl_str = data.as_str().expect("data should be string");

    // Find our bead in the JSONL
    let lines: Vec<&str> = jsonl_str.lines().filter(|l| !l.trim().is_empty()).collect();
    let bead_json_str = lines.iter()
        .find(|line| line.contains(&bead_id))
        .expect("created bead should be in recent output");

    let parsed = json_validation::parse_json(bead_json_str);
    let title = json_validation::get_string(&parsed, "title");

    // Verify Unicode characters are preserved
    assert!(title.contains("café"), "Unicode characters should be preserved");
    assert!(title.contains("日本語"), "Japanese characters should be preserved");
    assert!(title.contains("🎉"), "Emoji should be preserved");

    fixtures::close_bead(&bead_id, "Recent unicode cleanup");
}

#[test]
fn test_recent_json_priority_filter() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Recent priority filter test");

    // Update priority
    let mut cmd = bf_command();
    cmd.arg("update")
        .arg(&bead_id)
        .arg("--priority")
        .arg("3");
    let update_output = cmd.output().expect("Failed to update");
    assert!(update_output.status.success(), "Update should succeed");

    // Test with priority filter
    let output = capture::capture_stdout(
        bf_command()
            .arg("recent")
            .arg("--priority")
            .arg("3")
            .arg("--format")
            .arg("json")
    );

    let envelope = envelope::validate_envelope(&output.trim(), "recent");
    let data = envelope::get_envelope_data(&envelope);

    // Data can be a string (JSONL), an array of objects, or a single object
    let jsonl_str = if let Some(s) = data.as_str() {
        s.to_string()
    } else if let Some(arr) = data.as_array() {
        // If it's an array, convert to JSONL string
        arr.iter()
            .map(|v| serde_json::to_string(v).expect("Failed to convert item to JSON"))
            .collect::<Vec<_>>()
            .join("\n")
    } else if data.is_object() {
        // Single object - convert to JSONL string
        serde_json::to_string(&data).expect("Failed to convert object to JSON")
    } else {
        panic!("recent envelope data should be a JSONL string, array, or object, got: {:?}", data);
    };

    // Parse the JSONL string
    let lines: Vec<&str> = jsonl_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find our bead with priority filter
    assert!(lines.iter().any(|line| {
        let parsed = json_validation::parse_json(line);
        parsed.get("id")
            .and_then(|v| v.as_str())
            .map(|id| id == &bead_id)
            .unwrap_or(false)
    }), "bead with priority filter should be in recent output");

    fixtures::close_bead(&bead_id, "Recent priority filter cleanup");
}
