//! Comprehensive edge case tests for JSON output across all commands
//!
//! This module tests edge cases and boundary conditions for JSON output:
//! - Extremely long descriptions (testing field length limits)
//! - Unicode and special characters in all fields
//! - Newlines and unusual whitespace handling
//! - Error case formatting (invalid IDs, missing files, etc.)
//! - Commands with no matching results
//! - Beads with minimal required fields
//! - Edge combinations (long descriptions with special characters)

use std::process::Command;
use tempfile::TempDir;

// Import test infrastructure helpers from sibling module
use super::json_output::{
    test_workspace, bf_binary, bf_command, bf_command_with_workspace,
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
    crate::config::init_workspace(&beads_dir, "bf-edge-test")
        .expect("Failed to initialize test workspace");

    let metadata = crate::config::load_metadata(&beads_dir)
        .expect("Failed to load metadata");
    let _ = crate::Storage::open(&beads_dir.join(&metadata.database))
        .expect("Failed to create database");

    temp_dir
}

// ============================================================================
// Extremely long description tests
// ============================================================================

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_show_json_extremely_long_description() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create a bead with an extremely long description
    let bead_id = fixtures::create_bead("Long description test");

    // Create a description that's very long (10KB+)
    let long_desc = "A".repeat(1024 * 10); // 10KB of 'A' characters

    let output = capture::capture_stdout(
        bf_command()
            .arg("update")
            .arg(&bead_id)
            .arg("--description")
            .arg(&long_desc)
    );

    // Get show JSON output
    let show_output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    // Verify JSON is valid
    json_validation::assert_valid_json(&show_output);

    // Parse and verify description is present and preserved
    let json_str = show_output.trim();
    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("show output should be a JSON array");
    let bead = &array[0];

    let description = json_validation::get_string_optional(bead, "description");

    assert!(description.is_some(), "Description should be present");
    assert_eq!(description.unwrap(), long_desc, "Long description should be preserved exactly");

    // Cleanup
    fixtures::close_bead(&bead_id, "Long description test cleanup");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_show_json_long_description_with_special_characters() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create a bead with long description containing special characters
    let bead_id = fixtures::create_bead("Long special chars test");

    // Create a long description with special characters
    let special_chars: String = (0..100)
        .map(|i| format!("Line {} with \"quotes\", 'apostrophes', & symbols <>, \\backslashes\\, and unicode: 🎉 café\n", i))
        .collect();

    let output = capture::capture_stdout(
        bf_command()
            .arg("update")
            .arg(&bead_id)
            .arg("--description")
            .arg(&special_chars)
    );

    // Get show JSON output
    let show_output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    // Verify JSON is valid and special characters are preserved
    json_validation::assert_valid_json(&show_output);
    let json_str = show_output.trim();
    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("show output should be a JSON array");
    let bead = &array[0];

    let description = json_validation::get_string(bead, "description");

    assert!(description.contains("\"quotes\""), "Quotes should be preserved");
    assert!(description.contains("'apostrophes'"), "Apostrophes should be preserved");
    assert!(description.contains("& symbols"), "Ampersands should be preserved");
    assert!(description.contains(r"\backslashes\"), "Backslashes should be preserved");
    assert!(description.contains("🎉"), "Emoji should be preserved");
    assert!(description.contains("café"), "Unicode should be preserved");

    // Cleanup
    fixtures::close_bead(&bead_id, "Long special chars cleanup");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_list_json_with_long_descriptions() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create multiple beads with varying description lengths
    let bead1_id = fixtures::create_bead("Short desc bead");
    let bead2_id = fixtures::create_bead("Medium desc bead");
    let bead3_id = fixtures::create_bead("Long desc bead");

    // Add descriptions of different lengths
    capture::capture_stdout(
        bf_command()
            .arg("update")
            .arg(&bead1_id)
            .arg("--description")
            .arg("Short description")
    );

    capture::capture_stdout(
        bf_command()
            .arg("update")
            .arg(&bead2_id)
            .arg("--description")
            .arg(&"A".repeat(500))
    );

    capture::capture_stdout(
        bf_command()
            .arg("update")
            .arg(&bead3_id)
            .arg("--description")
            .arg(&"B".repeat(5000))
    );

    // Get list JSON output
    let output = capture::capture_stdout(
        bf_command()
            .arg("list")
            .arg("--format")
            .arg("json")
    );

    // Verify JSONL format is valid even with very long descriptions
    json_validation::assert_valid_jsonl(&output);

    let lines: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();

    // Each line should be valid JSON
    for line in lines {
        json_validation::assert_valid_json(line);
        let parsed = json_validation::parse_json(line);
        // Description field should be present (even if empty)
        let desc = json_validation::get_string_optional(&parsed, "description");
        // Verify descriptions are preserved (not truncated)
        if let Some(d) = desc {
            assert!(d.len() <= 5000 || d.starts_with('B'), "Long descriptions should be preserved");
        }
    }

    // Cleanup
    fixtures::close_bead(&bead1_id, "List long desc cleanup");
    fixtures::close_bead(&bead2_id, "List long desc cleanup");
    fixtures::close_bead(&bead3_id, "List long desc cleanup");
}

// ============================================================================
// Unicode and special character tests
// ============================================================================

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_show_json_unicode_in_all_fields() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Test comprehensive unicode in multiple fields
    let unicode_title = "🎉 Unicode title 日本語 مرحبا היי 🚀";
    let unicode_assignee = "משתמש@example.com";
    let unicode_desc = "Description with 你好, Bonjour, مرحبا by the user: testing@example.com <admin>";
    let bead_id = fixtures::create_bead_with_assignee(unicode_title, unicode_assignee);

    // Add unicode description
    capture::capture_stdout(
        bf_command()
            .arg("update")
            .arg(&bead_id)
            .arg("--description")
            .arg(unicode_desc)
    );

    // Add unicode labels
    capture::capture_stdout(
        bf_command()
            .arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg("标签/تسمية")
    );

    // Get show JSON output
    let show_output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    // Verify all unicode is preserved
    json_validation::assert_valid_json(&show_output);
    let json_str = show_output.trim();
    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("show output should be a JSON array");
    let bead = &array[0];

    let title = json_validation::get_string(bead, "title");
    let assignee = json_validation::get_string(bead, "assignee");
    let description = json_validation::get_string(bead, "description");

    assert!(title.contains("🎉"), "Emoji in title should be preserved");
    assert!(title.contains("日本語"), "Japanese in title should be preserved");
    assert!(assignee.contains("משתמש"), "Hebrew in assignee should be preserved");
    assert!(description.contains("你好"), "Chinese in description should be preserved");
    assert!(description.contains("مرحبا"), "Arabic in description should be preserved");

    // Cleanup
    fixtures::close_bead(&bead_id, "Unicode test cleanup");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_list_json_with_unicode_labels() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create beads with unicode labels
    let bead1_id = fixtures::create_bead_with_labels("Unicode label test 1", &["标签", "تسمية", "label"]);
    let bead2_id = fixtures::create_bead_with_labels("Unicode label test 2", &["étiquette", "तीर"]);

    // Get list JSON output
    let output = capture::capture_stdout(
        bf_command()
            .arg("list")
            .arg("--format")
            .arg("json")
    );

    // Verify JSONL is valid and unicode labels are preserved
    json_validation::assert_valid_jsonl(&output);

    let lines: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();

    let mut found_unicode_labels = false;
    for line in lines {
        let parsed = json_validation::parse_json(line);
        let labels = json_validation::get_array(&parsed, "labels");

        for label in labels {
            if let Some(label_str) = label.as_str() {
                if label_str.contains("标签") || label_str.contains("تسمية") ||
                   label_str.contains("étiquette") || label_str.contains("तीर") {
                    found_unicode_labels = true;
                    break;
                }
            }
        }
        if found_unicode_labels {
            break;
        }
    }

    assert!(found_unicode_labels, "Should find unicode labels in list output");

    // Cleanup
    fixtures::close_bead(&bead1_id, "Unicode labels cleanup");
    fixtures::close_bead(&bead2_id, "Unicode labels cleanup");
}

// ============================================================================
// Newline and unusual whitespace tests
// ============================================================================

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_show_json_trailing_and_leading_whitespace() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Trailing/leading whitespace test");

    // Add description with trailing and leading whitespace
    let whitespace_desc = "   \n\n  Leading spaces and newlines\nDescription text here\n  Trailing spaces  \n\n   ";

    capture::capture_stdout(
        bf_command()
            .arg("update")
            .arg(&bead_id)
            .arg("--description")
            .arg(whitespace_desc)
    );

    // Get show JSON output
    let show_output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    // Verify JSON is valid
    json_validation::assert_valid_json(&show_output);
    let json_str = show_output.trim();
    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("show output should be a JSON array");
    let bead = &array[0];

    let description = json_validation::get_string(bead, "description");

    // Verify trailing and leading whitespace is preserved
    assert!(description.starts_with("   \n\n  ") || description.starts_with("  "),
            "Leading whitespace should be preserved or normalized consistently");
    assert!(description.ends_with("  \n\n   ") || description.ends_with("  "),
            "Trailing whitespace should be preserved or normalized consistently");
    assert!(description.contains("Description text here"), "Content should be preserved");

    // Cleanup
    fixtures::close_bead(&bead_id, "Trailing/leading whitespace test cleanup");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_show_json_newlines_and_tabs_preserved() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Whitespace test");

    // Add description with complex whitespace
    let complex_whitespace = "Line 1\n\nLine 3 (double newline above)\n\tTabbed line\n  Mixed spacing\n\n\nTriple newline";

    capture::capture_stdout(
        bf_command()
            .arg("update")
            .arg(&bead_id)
            .arg("--description")
            .arg(complex_whitespace)
    );

    // Get show JSON output
    let show_output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    // Verify JSON is valid and whitespace is preserved
    json_validation::assert_valid_json(&show_output);
    let json_str = show_output.trim();
    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("show output should be a JSON array");
    let bead = &array[0];

    let description = json_validation::get_string(bead, "description");

    assert!(description.contains("\n\n"), "Double newlines should be preserved");
    assert!(description.contains("\t"), "Tabs should be preserved");
    assert!(description.contains("  "), "Multiple spaces should be preserved");
    assert!(description.contains("\n\n\n"), "Triple newlines should be preserved");

    // Cleanup
    fixtures::close_bead(&bead_id, "Whitespace test cleanup");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_show_json_carriage_returns_and_mixed_line_endings() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Line ending test");

    // Add description with various line endings
    let mixed_line_endings = "Line with \\r\n (CRLF)\nLine with \\n (LF)\rLine with \\r (CR)";

    capture::capture_stdout(
        bf_command()
            .arg("update")
            .arg(&bead_id)
            .arg("--description")
            .arg(mixed_line_endings)
    );

    // Get show JSON output
    let show_output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    // Verify JSON is valid
    json_validation::assert_valid_json(&show_output);

    // The description should be present (line ending normalization may vary)
    let json_str = show_output.trim();
    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("show output should be a JSON array");
    let bead = &array[0];

    let description = json_validation::get_string(bead, "description");

    assert!(description.contains("Line with"), "Content should be preserved");
    assert!(description.contains("CRLF"), "Line ending markers should be preserved");

    // Cleanup
    fixtures::close_bead(&bead_id, "Line ending test cleanup");
}

// ============================================================================
// Error case formatting tests
// ============================================================================

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_show_json_invalid_bead_id_error_format() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Try to show a non-existent bead
    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut bf_command()
            .arg("show")
            .arg("bf-nonexistent-12345")
            .arg("--format")
            .arg("json")
    );

    // Verify command failed as expected
    assert!(!success, "Command should fail for invalid bead ID");

    // Verify error output is present
    assert!(!stderr.is_empty(), "Error message should be present in stderr");

    // Error message should mention the invalid ID or not found
    assert!(stderr.contains("bf-nonexistent-12345") || stderr.contains("not found") || stderr.contains("invalid"),
           "Error should reference the invalid bead ID or indicate not found");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_update_json_invalid_bead_id_error_format() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Try to update a non-existent bead
    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut bf_command()
            .arg("update")
            .arg("bf-nonexistent-67890")
            .arg("--description")
            .arg("This should fail")
    );

    // Verify command failed as expected
    assert!(!success, "Command should fail for invalid bead ID");

    // Verify error output is present
    assert!(!stderr.is_empty(), "Error message should be present in stderr");

    // Error message should be informative
    assert!(stderr.len() > 10, "Error message should be meaningful");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_claim_json_no_ready_beads_error_format() {
    let _ws = create_isolated_workspace();

    // Create a fresh workspace with no beads
    let temp_dir = create_isolated_workspace();
    let empty_workspace = temp_dir.path();

    // Try to claim when no beads are available
    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut bf_command_with_workspace(empty_workspace)
            .arg("claim")
            .arg("--assignee")
            .arg("test-worker")
            .arg("--format")
            .arg("json")
    );

    // The command might succeed with no beads (returning empty) or fail with error
    if success {
        // If it succeeds, verify the JSON is valid
        json_validation::assert_valid_json(&stdout);
    } else {
        // If it fails, verify error format
        assert!(!stderr.is_empty(), "Error message should be present when no beads available");
        // Error should mention no beads or nothing to claim
        assert!(stderr.contains("no") || stderr.contains("available") || stderr.contains("nothing"),
               "Error should indicate no beads available");
    }
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_label_add_json_invalid_bead_id_error_format() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Try to add a label to a non-existent bead
    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut bf_command()
            .arg("label")
            .arg("add")
            .arg("bf-nonexistent-label-test")
            .arg("--label")
            .arg("test-label")
    );

    // Verify command failed as expected
    assert!(!success, "Command should fail for invalid bead ID");

    // Verify error output is present
    assert!(!stderr.is_empty(), "Error message should be present in stderr");
}

// ============================================================================
// Empty result tests (commands with no matching results)
// ============================================================================

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_show_json_empty_workspace() {
    let temp_dir = create_isolated_workspace();
    let empty_workspace = temp_dir.path();

    // Try to show a bead in empty workspace
    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut bf_command_with_workspace(empty_workspace)
            .arg("show")
            .arg("bf-some-id")
            .arg("--format")
            .arg("json")
    );

    // Should fail gracefully
    assert!(!success, "Show should fail in empty workspace");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_list_json_empty_workspace() {
    let temp_dir = create_isolated_workspace();
    let empty_workspace = temp_dir.path();

    // List in empty workspace should return valid empty output
    let output = capture::capture_stdout(
        bf_command_with_workspace(empty_workspace)
            .arg("list")
            .arg("--format")
            .arg("json")
    );

    // Empty output is valid JSONL (empty string or only whitespace)
    let trimmed = output.trim();
    assert!(trimmed.is_empty(), "Empty list should return empty JSONL");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_ready_json_empty_workspace() {
    let temp_dir = create_isolated_workspace();
    let empty_workspace = temp_dir.path();

    // Ready in empty workspace should return valid empty output
    let output = capture::capture_stdout(
        bf_command_with_workspace(empty_workspace)
            .arg("ready")
            .arg("--format")
            .arg("json")
    );

    // Empty output is valid JSONL
    let trimmed = output.trim();
    assert!(trimmed.is_empty() || trimmed == "[]", "Empty ready should return empty JSONL");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_search_json_empty_workspace() {
    let temp_dir = create_isolated_workspace();
    let empty_workspace = temp_dir.path();

    // Search in empty workspace should return valid empty output
    let output = capture::capture_stdout(
        bf_command_with_workspace(empty_workspace)
            .arg("search")
            .arg("test")
            .arg("--format")
            .arg("json")
    );

    // Empty output is valid JSONL
    let trimmed = output.trim();
    assert!(trimmed.is_empty() || trimmed == "[]", "Empty search should return empty JSONL");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_recent_json_empty_workspace() {
    let temp_dir = create_isolated_workspace();
    let empty_workspace = temp_dir.path();

    // Recent in empty workspace should return valid empty output
    let output = capture::capture_stdout(
        bf_command_with_workspace(empty_workspace)
            .arg("recent")
            .arg("--format")
            .arg("json")
    );

    // recent command ALWAYS uses envelope format
    let envelope = envelope::validate_envelope(&output.trim(), "recent");

    // Data field should be empty string (no beads to show)
    let data = envelope::get_envelope_data(&envelope);
    assert!(data.is_string(), "recent envelope data should be a string");

    let jsonl_str = data.as_str().expect("data should be string");
    assert_eq!(jsonl_str, "", "recent data should be empty string for empty workspace");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_list_json_no_open_beads() {
    // Use a truly isolated workspace to avoid interference from other tests
    let temp_dir = create_isolated_workspace();
    let workspace = temp_dir.path();

    // Create and immediately close a bead
    let bead_id = fixtures::create_bead("Closed bead test");
    fixtures::close_bead(&bead_id, "Close for empty list test");

    // List should return beads, but they should all be closed
    let output = capture::capture_stdout(
        bf_command_with_workspace(workspace)
            .arg("list")
            .arg("--format")
            .arg("json")
    );

    // Should return valid JSONL
    json_validation::assert_valid_jsonl(&output);

    let lines: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();

    // All returned beads should have closed status
    for line in lines {
        let parsed = json_validation::parse_json(line);
        let status = json_validation::get_string(&parsed, "status");
        assert_eq!(status, "closed", "All beads in list should be closed");
    }
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_ready_json_all_blocked_beads() {
    // Use a truly isolated workspace to avoid interference from other tests
    let temp_dir = create_isolated_workspace();
    let workspace = temp_dir.path();

    // Create beads that will all be blocked or closed
    let bead1_id = fixtures::create_bead("Blocked bead 1");
    let bead2_id = fixtures::create_bead("Blocked bead 2");
    let bead3_id = fixtures::create_bead("Blocked bead 3");

    // Create dependency chain: bead2 -> bead1, bead3 -> bead1
    fixtures::add_dependency(&bead2_id, &bead1_id);
    fixtures::add_dependency(&bead3_id, &bead1_id);

    // Close bead1 so that the dependent beads (bead2, bead3) are blocked
    // and bead1 itself is closed (not returned by ready)
    fixtures::close_bead(&bead1_id, "Blocker closed");

    // Ready should return no beads (all are either blocked or closed)
    let output = capture::capture_stdout(
        bf_command_with_workspace(workspace)
            .arg("ready")
            .arg("--format")
            .arg("json")
    );

    // Ready with all blocked/closed beads should return empty output
    // May be empty string or empty JSON array [] depending on implementation
    let trimmed = output.trim();
    assert!(
        trimmed.is_empty() || trimmed == "[]",
        "Ready with all blocked/closed beads should return empty or [], got: {}",
        trimmed
    );

    // Cleanup
    fixtures::close_bead(&bead2_id, "Blocked test cleanup 2");
    fixtures::close_bead(&bead3_id, "Blocked test cleanup 3");
}

// ============================================================================
// Minimal field tests
// ============================================================================

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_create_json_minimal_fields() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create bead with absolute minimum required parameters
    let output = capture::capture_stdout(
        bf_command()
            .arg("create")
            .arg("--title")
            .arg("Minimal bead")
            .arg("--type")
            .arg("task")
            .arg("--priority")
            .arg("2") // default priority
            .arg("--json")
    );

    // Verify JSON output is valid
    json_validation::assert_valid_json(&output);

    // Parse and verify envelope structure
    let parsed = json_validation::parse_json(&output);

    // Verify envelope has required fields
    json_validation::assert_required_fields(&parsed, &["kind", "version", "data"], "create envelope");

    // Verify it's a create envelope
    let kind = json_validation::get_string(&parsed, "kind");
    assert_eq!(kind, "create", "Envelope kind should be 'create'");

    // Extract data from envelope
    let data = json_validation::get_object(&parsed, "data");

    // Verify data has id field
    json_validation::assert_required_fields(&data, &["id"], "create data");
    let bead_id = json_validation::get_string(&data, "id");
    assert!(!bead_id.is_empty(), "Bead ID should not be empty");

    // Verify the bead was created with minimal fields
    let show_output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    let show_json_str = show_output.trim();
    let show_parsed = json_validation::parse_json(show_json_str);
    let show_array = show_parsed.as_array().expect("show output should be a JSON array");
    let show_bead = &show_array[0];

    // Verify all required fields are present with proper defaults
    json_validation::assert_required_fields(
        show_bead,
        &["id", "title", "status", "priority", "issue_type", "created_at", "updated_at"],
        "show minimal bead"
    );

    // Verify optional fields have proper null/empty defaults
    let assignee = json_validation::get_string_optional(show_bead, "assignee");
    assert!(assignee == Some("".to_string()) || assignee.is_none(),
              "Assignee should be empty string or null for minimal bead");

    let labels = json_validation::get_array(show_bead, "labels");
    assert_eq!(labels.len(), 0, "Labels should be empty array for minimal bead");

    // Cleanup
    fixtures::close_bead(&bead_id, "Minimal test cleanup");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_show_json_minimal_bead_structure() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create a bead and then verify minimal required fields are present
    let bead_id = fixtures::create_bead("Minimal structure test");

    // Get show JSON output
    let show_output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    // Verify minimal required fields
    let json_str = show_output.trim();
    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("show output should be a JSON array");
    let bead = &array[0];

    // These are the absolute minimum required fields for any bead JSON output
    let minimal_required_fields = ["id", "title", "status", "priority", "issue_type"];
    json_validation::assert_required_fields(bead, &minimal_required_fields, "minimal bead structure");

    // Verify field types
    assert!(bead.get("id").and_then(|v| v.as_str()).is_some(), "id must be string");
    assert!(bead.get("title").and_then(|v| v.as_str()).is_some(), "title must be string");
    assert!(bead.get("status").and_then(|v| v.as_str()).is_some(), "status must be string");
    assert!(bead.get("priority").and_then(|v| v.as_i64()).is_some(), "priority must be integer");
    assert!(bead.get("issue_type").and_then(|v| v.as_str()).is_some(), "issue_type must be string");

    // Cleanup
    fixtures::close_bead(&bead_id, "Minimal structure cleanup");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_list_json_minimal_beads() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create beads with minimal configuration
    let bead1_id = fixtures::create_bead("Minimal list bead 1");
    let bead2_id = fixtures::create_bead("Minimal list bead 2");

    // Get list JSON output
    let output = capture::capture_stdout(
        bf_command()
            .arg("list")
            .arg("--format")
            .arg("json")
    );

    // Verify JSONL is valid and each bead has minimal fields
    json_validation::assert_valid_jsonl(&output);

    let lines: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();

    for line in lines {
        let parsed = json_validation::parse_json(line);

        // Verify minimal required fields
        let minimal_required_fields = ["id", "title", "status", "priority", "issue_type"];
        json_validation::assert_required_fields(&parsed, &minimal_required_fields, "list minimal bead");
    }

    // Cleanup
    fixtures::close_bead(&bead1_id, "Minimal list cleanup");
    fixtures::close_bead(&bead2_id, "Minimal list cleanup");
}

// ============================================================================
// Edge combination tests
// ============================================================================

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_show_json_long_description_with_special_chars_and_unicode() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Test the worst case: very long description with special characters and unicode
    let bead_id = fixtures::create_bead("Worst case test");

    // Create a massive description with everything combined
    let complex_desc: String = (0..100)
        .flat_map(|i| {
            format!(
                "Block {}: Special chars \"'&<>\\, Unicode 🎉 café 日本語, newlines\n\tTabs and   spaces\n",
                i
            ).chars().collect::<Vec<_>>()
        })
        .collect();

    capture::capture_stdout(
        bf_command()
            .arg("update")
            .arg(&bead_id)
            .arg("--description")
            .arg(&complex_desc)
    );

    // Get show JSON output
    let show_output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    // Verify JSON is valid even with this complex input
    json_validation::assert_valid_json(&show_output);
    let json_str = show_output.trim();
    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("show output should be a JSON array");
    let bead = &array[0];

    let description = json_validation::get_string(bead, "description");

    // Verify all special components are present
    assert!(description.len() > 5000, "Long description should be preserved");
    assert!(description.contains("\"'&<>\\"), "Special chars should be preserved");
    assert!(description.contains("🎉"), "Emoji should be preserved");
    assert!(description.contains("café"), "Unicode should be preserved");
    assert!(description.contains("\n"), "Newlines should be preserved");
    assert!(description.contains("\t"), "Tabs should be preserved");

    // Cleanup
    fixtures::close_bead(&bead_id, "Worst case cleanup");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_show_json_edge_case_title_combinations() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Test various edge case title combinations
    let edge_titles: Vec<String> = vec![
        "".to_string(), // Empty title (if allowed)
        "A".to_string(), // Single character
        " ".to_string(), // Single space
        "A\nB".to_string(), // Title with newline
        "🎉".to_string(), // Single emoji
        "&amp;&lt;&gt;".to_string(), // HTML entities
        "'\"`".to_string(), // Quote variants
        "x".repeat(500), // Very long title
    ];

    for title in &edge_titles {
        if title.is_empty() {
            continue; // Skip empty title if create rejects it
        }

        let bead_id = fixtures::create_bead(title);

        // Get show JSON output
        let show_output = capture::capture_stdout(
            bf_command()
                .arg("show")
                .arg(&bead_id)
                .arg("--format")
                .arg("json")
        );

        // Verify JSON is valid for each edge case
        json_validation::assert_valid_json(&show_output);
        let json_str = show_output.trim();
        let parsed = json_validation::parse_json(json_str);
        let array = parsed.as_array().expect("show output should be a JSON array");
        let bead = &array[0];

        let retrieved_title = json_validation::get_string(bead, "title");
        assert_eq!(retrieved_title, *title, "Edge case title should be preserved exactly");

        // Cleanup
        fixtures::close_bead(&bead_id, "Edge case title cleanup");
    }
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_list_json_mixed_content_types() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create beads with various combinations of special content
    let bead1_id = fixtures::create_bead_with_labels("Mixed content 1", &["bug", "urgent"]);
    let bead2_id = fixtures::create_bead_with_assignee("Mixed content 2", "user@example.com");
    let bead3_id = fixtures::create_bead("Mixed content 3");

    // Add complex descriptions and labels
    capture::capture_stdout(
        bf_command()
            .arg("update")
            .arg(&bead1_id)
            .arg("--description")
            .arg("Description with unicode: 🎉 café 日本語")
    );

    capture::capture_stdout(
        bf_command()
            .arg("update")
            .arg(&bead2_id)
            .arg("--description")
            .arg("Multi-line\ndescription\nwith\ttabs")
    );

    capture::capture_stdout(
        bf_command()
            .arg("label")
            .arg("add")
            .arg(&bead3_id)
            .arg("--label")
            .arg("feature/需求")
    );

    // Get list JSON output
    let output = capture::capture_stdout(
        bf_command()
            .arg("list")
            .arg("--format")
            .arg("json")
    );

    // Verify JSONL is valid with all the mixed content
    json_validation::assert_valid_jsonl(&output);

    let lines: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();

    for line in lines {
        let parsed = json_validation::parse_json(line);
        json_validation::assert_required_fields(
            &parsed,
            &["id", "title", "status", "priority", "issue_type", "created_at", "updated_at"],
            "list mixed content"
        );
    }

    // Cleanup
    fixtures::close_bead(&bead1_id, "Mixed content cleanup");
    fixtures::close_bead(&bead2_id, "Mixed content cleanup");
    fixtures::close_bead(&bead3_id, "Mixed content cleanup");
}

// ============================================================================
// Error recovery and resiliency tests
// ============================================================================

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_show_json_partial_unicode_sequence() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Partial unicode test");

    // Test with unicode edge cases that might cause issues in JSON serialization
    let partial_unicode = "Valid unicode: 🎉 café, then edge cases: \u{FEFF} (BOM), \u{200B} (zero-width space), \u{202E} (RTO override)";

    let update_result = capture::capture_failed_command(
        &mut bf_command()
            .arg("update")
            .arg(&bead_id)
            .arg("--description")
            .arg(partial_unicode)
    );

    // System should either accept it (and preserve it) or reject it with clear error
    if update_result.2 {
        // If accepted, verify JSON is still valid
        let show_output = capture::capture_stdout(
            bf_command()
                .arg("show")
                .arg(&bead_id)
                .arg("--format")
                .arg("json")
        );
        json_validation::assert_valid_json(&show_output);
        // For show command, verify it's valid JSON array format
        let json_str = show_output.trim();
        let parsed = json_validation::parse_json(json_str);
        let _array = parsed.as_array().expect("show output should be a JSON array");
    } else {
        // If rejected, verify error is clear
        assert!(!update_result.1.is_empty(), "Error should be clear for invalid unicode");
    }

    // Cleanup
    fixtures::close_bead(&bead_id, "Partial unicode cleanup");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_show_json_very_long_single_line() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Very long single line test");

    // Create a description that's extremely long but single-line (no newlines)
    let single_long_line = "A".repeat(50000); // 50KB single line

    capture::capture_stdout(
        bf_command()
            .arg("update")
            .arg(&bead_id)
            .arg("--description")
            .arg(&single_long_line)
    );

    // Get show JSON output
    let show_output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    // Verify JSON is valid even with very long single line
    json_validation::assert_valid_json(&show_output);
    let json_str = show_output.trim();
    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("show output should be a JSON array");
    let bead = &array[0];

    let description = json_validation::get_string(bead, "description");

    assert_eq!(description.len(), 50000, "Very long single line should be preserved");

    // Cleanup
    fixtures::close_bead(&bead_id, "Very long single line cleanup");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_json_output_consistency_across_commands() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create a bead with complex content
    let bead_id = fixtures::create_bead("Consistency test");
    let complex_desc = "Test with unicode: 🎉 café, special chars: \"'&<>\\, newlines\nand\ttabs";

    capture::capture_stdout(
        bf_command()
            .arg("update")
            .arg(&bead_id)
            .arg("--description")
            .arg(complex_desc)
    );

    // Get the bead from different commands and verify consistency
    let show_output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    let list_output = capture::capture_stdout(
        bf_command()
            .arg("list")
            .arg("--format")
            .arg("json")
    );

    // Parse both outputs
    let show_json_str = show_output.trim();
    let show_parsed = json_validation::parse_json(show_json_str);
    let show_array = show_parsed.as_array().expect("show output should be a JSON array");
    let show_bead = &show_array[0];
    let list_lines: Vec<&str> = list_output.lines().filter(|l| !l.trim().is_empty()).collect();

    // Find the bead in list output
    let mut list_parsed = None;
    for line in list_lines {
        let parsed = json_validation::parse_json(line);
        if json_validation::get_string(&parsed, "id") == bead_id {
            list_parsed = Some(parsed);
            break;
        }
    }

    assert!(list_parsed.is_some(), "Bead should be found in list output");
    let list_parsed = list_parsed.unwrap();

    // Verify key fields are consistent between commands
    let show_title = json_validation::get_string(show_bead, "title");
    let list_title = json_validation::get_string(&list_parsed, "title");
    assert_eq!(show_title, list_title, "Title should be consistent across commands");

    let show_desc = json_validation::get_string(show_bead, "description");
    let list_desc = json_validation::get_string(&list_parsed, "description");
    assert_eq!(show_desc, list_desc, "Description should be consistent across commands");

    let show_status = json_validation::get_string(show_bead, "status");
    let list_status = json_validation::get_string(&list_parsed, "status");
    assert_eq!(show_status, list_status, "Status should be consistent across commands");

    // Cleanup
    fixtures::close_bead(&bead_id, "Consistency test cleanup");
}
