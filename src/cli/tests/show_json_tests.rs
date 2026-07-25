//! JSON output tests for `bf show` command
//!
//! Comprehensive tests for show command JSON output including:
//! - JSON structure validation
//! - Required fields presence and types
//! - Special character handling in all text fields
//! - Error case (non-existent bead)
//! - Different bead types and statuses

use std::process::Command;
use tempfile::TempDir;

// Import test infrastructure helpers from sibling module
use super::json_output::{
    test_workspace, bf_binary, bf_command,
    json_validation, format_detection, fixtures, capture,
};

// Import items made available in parent scope
use super::*;

/// Create an isolated test workspace
fn create_isolated_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let beads_dir = temp_dir.path().join(".beads");
    std::fs::create_dir(&beads_dir).expect("Failed to create .beads directory");

    // Initialize workspace
    crate::config::init_workspace(&beads_dir, "bf-show-test")
        .expect("Failed to initialize test workspace");

    let metadata = crate::config::load_metadata(&beads_dir)
        .expect("Failed to load metadata");
    let _ = crate::Storage::open(&beads_dir.join(&metadata.database))
        .expect("Failed to create database");

    temp_dir
}

// ============================================================================
// Structure validation tests
// ============================================================================

#[test]
fn test_show_json_structure_validity() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create a test bead
    let bead_id = fixtures::create_bead("Structure test bead");

    // Get show JSON output
    let output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    // Verify it's an array with one element (NEEDLE contract)
    let json_str = output.trim();
    assert!(json_str.starts_with('['), "show output should start with '['");
    assert!(json_str.ends_with(']'), "show output should end with ']'");

    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("show output should be a JSON array");
    assert_eq!(array.len(), 1, "show should return exactly one bead");

    let bead = &array[0];
    assert!(bead.is_object(), "bead should be a JSON object");

    // Verify required fields are present
    json_validation::assert_required_fields(
        bead,
        &["id", "title", "status", "priority", "issue_type", "created_at", "updated_at"],
        "show command"
    );

    // Cleanup
    fixtures::close_bead(&bead_id, "Structure test cleanup");
}

#[test]
fn test_show_json_is_parseable() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Parseable test");

    let output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    // Should parse as valid JSON
    let json_str = output.trim();
    json_validation::assert_valid_json(json_str);

    // Should be an array with one element
    let parsed = json_validation::parse_json(json_str);
    assert!(parsed.is_array(), "Output must be a JSON array");
    let array = parsed.as_array().unwrap();
    assert_eq!(array.len(), 1, "Array should contain exactly one element");

    fixtures::close_bead(&bead_id, "Parseable test cleanup");
}

// ============================================================================
// Required fields tests
// ============================================================================

#[test]
fn test_show_json_required_fields_types() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Field types test");

    let output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("Should be array");
    let bead = &array[0];

    // id must be a string matching created bead
    let id_val = json_validation::get_string(bead, "id");
    assert_eq!(id_val, bead_id);
    assert!(id_val.len() > 0, "id must not be empty");

    // title must be a string
    let title = json_validation::get_string(bead, "title");
    assert_eq!(title, "Field types test");

    // status must be a string with valid value
    let status = json_validation::get_string(bead, "status");
    assert!(matches!(status.as_str(), "open" | "in_progress" | "blocked" | "closed"));

    // priority must be a number (0-4)
    let priority = json_validation::get_int(bead, "priority");
    assert!((0..=4).contains(&priority), "priority must be between 0 and 4");

    // issue_type must be a string
    let issue_type = json_validation::get_string(bead, "issue_type");
    assert!(!issue_type.is_empty(), "issue_type must not be empty");

    // created_at must be ISO 8601 format
    let created_at = json_validation::get_string(bead, "created_at");
    assert!(created_at.contains('T'), "created_at must be in ISO 8601 format");

    // updated_at must be ISO 8601 format
    let updated_at = json_validation::get_string(bead, "updated_at");
    assert!(updated_at.contains('T'), "updated_at must be in ISO 8601 format");

    // labels must be an array (get_array already validates this)
    let labels = json_validation::get_array(bead, "labels");
    // Successful call to get_array proves it's an array

    fixtures::close_bead(&bead_id, "Field types test cleanup");
}

#[test]
fn test_show_json_all_optional_fields_present() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Optional fields test");

    let output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("Should be array");
    let bead = &array[0];

    // Optional fields should always be present (even if null)
    let optional_fields = [
        "description",
        "assignee",
        "labels",
        "acceptance_criteria",
        "notes",
        "design",
    ];

    for field in &optional_fields {
        assert!(
            bead.get(*field).is_some(),
            "Optional field '{}' must be present in output",
            field
        );
    }

    // dependencies and comments should be stripped for NEEDLE compatibility
    assert!(
        bead.get("dependencies").is_none(),
        "dependencies should be stripped"
    );
    assert!(
        bead.get("comments").is_none(),
        "comments should be stripped"
    );

    fixtures::close_bead(&bead_id, "Optional fields test cleanup");
}

// ============================================================================
// Special character handling tests
// ============================================================================

#[test]
fn test_show_json_special_characters_in_title() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let special_title = "Test \"quotes\" and 'apostrophes' & <symbols> \\n\\t";
    let bead_id = fixtures::create_bead(special_title);

    let output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    // First verify it's valid JSON (proper escaping)
    let json_str = output.trim();
    json_validation::assert_valid_json(json_str);

    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("Should be array");
    let bead = &array[0];

    let title = json_validation::get_string(bead, "title");

    // Verify special characters are preserved
    assert!(title.contains("quotes"), "title should contain 'quotes'");
    assert!(title.contains("apostrophes"), "title should contain 'apostrophes'");
    assert!(title.contains("&"), "title should contain '&'");
    assert!(title.contains("<symbols>"), "title should contain '<symbols>'");

    fixtures::close_bead(&bead_id, "Special chars title cleanup");
}

#[test]
fn test_show_json_special_characters_in_description() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Desc special chars");

    // Update with special description
    let special_desc = "Multi-line\ndescription\nwith\ttabs\nUnicode: 你好 🎉 🚀\nEscape: \\\" \\n \\t";
    let mut cmd = bf_command();
    cmd.arg("update")
        .arg(&bead_id)
        .arg("--description")
        .arg(special_desc);
    let update_output = cmd.output().expect("Failed to update");
    assert!(update_output.status.success(), "Update should succeed");

    let output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    json_validation::assert_valid_json(json_str);

    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("Should be array");
    let bead = &array[0];

    let desc = json_validation::get_string(bead, "description");

    // Verify special characters are preserved
    assert!(desc.contains("Multi-line"), "description should contain multi-line text");
    assert!(desc.contains("你好"), "description should contain Chinese characters");
    assert!(desc.contains("🎉"), "description should contain emoji");
    assert!(desc.contains("🚀"), "description should contain another emoji");

    fixtures::close_bead(&bead_id, "Special chars description cleanup");
}

#[test]
fn test_show_json_special_characters_in_assignee() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Assignee test");

    // Update with special assignee
    let special_assignee = "user+test@example.com <admin>";
    let mut cmd = bf_command();
    cmd.arg("update")
        .arg(&bead_id)
        .arg("--assignee")
        .arg(special_assignee);
    let update_output = cmd.output().expect("Failed to update");
    assert!(update_output.status.success(), "Update should succeed");

    let output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    json_validation::assert_valid_json(json_str);

    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("Should be array");
    let bead = &array[0];

    let assignee = json_validation::get_string(bead, "assignee");
    assert!(assignee.contains("user+test"), "assignee should preserve special characters");
    assert!(assignee.contains("<admin>"), "assignee should preserve angle brackets");

    fixtures::close_bead(&bead_id, "Special chars assignee cleanup");
}

#[test]
fn test_show_json_unicode_emoji_in_all_text_fields() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let unicode_title = "🎉 Unicode title with emoji 🚀";
    let unicode_desc = "Description: 你好 مرحبا היי 🌟";
    let bead_id = fixtures::create_bead(unicode_title);

    // Update with unicode description
    let mut cmd = bf_command();
    cmd.arg("update")
        .arg(&bead_id)
        .arg("--description")
        .arg(unicode_desc);
    let update_output = cmd.output().expect("Failed to update");
    assert!(update_output.status.success(), "Update should succeed");

    let output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    json_validation::assert_valid_json(json_str);

    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("Should be array");
    let bead = &array[0];

    let title = json_validation::get_string(bead, "title");
    assert!(title.contains("🎉"), "title should contain party emoji");
    assert!(title.contains("🚀"), "title should contain rocket emoji");

    let desc = json_validation::get_string(bead, "description");
    assert!(desc.contains("你好"), "description should contain Chinese");
    assert!(desc.contains("مرحبا"), "description should contain Arabic");
    assert!(desc.contains("היי"), "description should contain Hebrew");
    assert!(desc.contains("🌟"), "description should contain star emoji");

    fixtures::close_bead(&bead_id, "Unicode emoji cleanup");
}

#[test]
fn test_show_json_special_characters_in_labels() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead_with_labels(
        "Label test",
        &["special/label", "label-with-dash", "label_with_underscore", "label.with.dots"]
    );

    let output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    json_validation::assert_valid_json(json_str);

    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("Should be array");
    let bead = &array[0];

    let labels = json_validation::get_array(bead, "labels");
    let label_strs: Vec<&str> = labels.iter()
        .filter_map(|l| l.as_str())
        .collect();

    assert!(label_strs.contains(&"special/label"), "labels should contain slashes");
    assert!(label_strs.contains(&"label-with-dash"), "labels should contain dashes");
    assert!(label_strs.contains(&"label_with_underscore"), "labels should contain underscores");
    assert!(label_strs.contains(&"label.with.dots"), "labels should contain dots");

    fixtures::close_bead(&bead_id, "Special chars labels cleanup");
}

// ============================================================================
// Error case tests
// ============================================================================

#[test]
fn test_show_json_nonexistent_bead_errors() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let (stdout, stderr, success) = capture::capture_failed_command(
        bf_command()
            .arg("show")
            .arg("bf-nonexistent-12345")
            .arg("--format")
            .arg("json")
    );

    // Should fail
    assert!(!success, "show should fail for non-existent bead");

    // Stderr should mention bead not found
    assert!(
        stderr.contains("not found") || stderr.contains("Bead not found"),
        "Error should indicate bead not found, got: {}",
        stderr
    );

    // Stdout should be empty (no JSON output on error)
    assert!(
        stdout.trim().is_empty(),
        "stdout should be empty for non-existent bead, got: {}",
        stdout
    );
}

// ============================================================================
// Additional edge case tests
// ============================================================================

#[test]
fn test_show_json_with_closed_bead() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Close test");
    fixtures::close_bead(&bead_id, "Test close reason");

    let output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("Should be array");
    let bead = &array[0];

    assert_eq!(json_validation::get_string(bead, "status"), "closed");
    assert_eq!(json_validation::get_string(bead, "close_reason"), "Test close reason");

    // closed_at should be present and valid
    let closed_at = json_validation::get_string(bead, "closed_at");
    assert!(closed_at.contains('T'), "closed_at should be in ISO 8601 format");
}

#[test]
fn test_show_json_timestamps_are_valid_rfc3339() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Timestamp test");

    let output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("Should be array");
    let bead = &array[0];

    // Check created_at is valid RFC3339
    let created_at = json_validation::get_string(bead, "created_at");
    let parsed_created = chrono::DateTime::parse_from_rfc3339(&created_at);
    assert!(parsed_created.is_ok(), "created_at should be valid RFC3339: {}", created_at);

    // Check updated_at is valid RFC3339
    let updated_at = json_validation::get_string(bead, "updated_at");
    let parsed_updated = chrono::DateTime::parse_from_rfc3339(&updated_at);
    assert!(parsed_updated.is_ok(), "updated_at should be valid RFC3339: {}", updated_at);

    // updated_at should be >= created_at
    assert!(
        parsed_updated.unwrap() >= parsed_created.unwrap(),
        "updated_at should be after or equal to created_at"
    );

    fixtures::close_bead(&bead_id, "Timestamp test cleanup");
}

#[test]
fn test_show_json_empty_fields_serialize_correctly() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Empty fields");

    let output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("Should be array");
    let bead = &array[0];

    // Empty description should be present but can be null or empty string
    match bead.get("description") {
        Some(serde_json::Value::String(s)) => {
            assert_eq!(s, "");
        }
        Some(serde_json::Value::Null) => {
            // Null is also fine
        }
        Some(other) => {
            panic!("description should be null or empty string, got: {:?}", other);
        }
        None => {
            panic!("description field must be present");
        }
    }

    // Assignee should be null when not set
    assert!(bead.get("assignee").is_some(), "assignee field must be present");
    assert!(
        bead.get("assignee").unwrap().is_null() ||
        bead.get("assignee").and_then(|v| v.as_str()).map(|s| s.is_empty()).unwrap_or(false),
        "assignee should be null or empty string when not set"
    );

    fixtures::close_bead(&bead_id, "Empty fields test cleanup");
}
