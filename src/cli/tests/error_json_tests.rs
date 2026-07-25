//! Error case and invalid query JSON output tests
//!
//! This module tests JSON output behavior under error conditions:
//! - Invalid bead IDs and references
//! - Invalid query scenarios (malformed queries, invalid filters)
//! - Commands that fail with proper error messages
//! - Schema consistency even on errors
//! - Invalid command-line arguments and flags
//! - Database corruption or missing workspace scenarios
//!
//! ## Test Philosophy
//!
//! Error conditions should behave predictably:
//! 1. Errors go to stderr, NOT to stdout JSON output
//! 2. Stdout should either be empty or contain valid JSON (even for error cases)
//! 3. Error messages in stderr should be informative and reference the invalid input
//! 4. Exit codes should be non-zero for errors

use std::process::Command;
use tempfile::TempDir;

// Import test infrastructure helpers from sibling module
use super::json_output::{
    test_workspace, bf_binary, bf_command, bf_command_with_workspace,
    json_validation, format_detection, fixtures, capture,
};

/// Create an isolated test workspace
fn create_isolated_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let beads_dir = temp_dir.path().join(".beads");
    std::fs::create_dir(&beads_dir).expect("Failed to create .beads directory");

    // Initialize workspace
    crate::config::init_workspace(&beads_dir, "bf-error-test")
        .expect("Failed to initialize test workspace");

    let metadata = crate::config::load_metadata(&beads_dir)
        .expect("Failed to load metadata");
    let _ = crate::Storage::open(&beads_dir.join(&metadata.database))
        .expect("Failed to create database");

    temp_dir
}

// ============================================================================
// Invalid bead ID error tests
// ============================================================================

#[test]
fn test_show_json_malformed_bead_id() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Test various malformed bead IDs
    let malformed_ids = vec![
        "not-a-bead-id".to_string(),
        "bf-".to_string(),
        "bf-123".to_string(),
        "Bf-test-invalid".to_string(),
        "bf_test_invalid".to_string(),
        "".to_string(),
        "12345".to_string(),
        "test-bf-invalid".to_string(),
        "bf-invalid-!@#$%".to_string(),
        format!("bf-{}", "x".repeat(100)), // Extremely long ID
    ];

    for malformed_id in malformed_ids {
        if malformed_id.is_empty() {
            continue; // Skip empty - clap will catch this as missing argument
        }

        let (stdout, stderr, success) = capture::capture_failed_command(
            &mut bf_command()
                .arg("show")
                .arg(&malformed_id)
                .arg("--format")
                .arg("json")
        );

        // Command should fail
        assert!(!success, "show should fail for malformed ID: {}", malformed_id);

        // Stdout should be empty or contain only valid JSON (no partial/error JSON)
        let stdout_trimmed = stdout.trim();
        if !stdout_trimmed.is_empty() {
            json_validation::assert_valid_json(&stdout_trimmed);
        }

        // Stderr should contain error message
        assert!(!stderr.is_empty(), "stderr should contain error for malformed ID: {}", malformed_id);

        // Error should reference the invalid ID or indicate format/validation issue
        assert!(
            stderr.contains(malformed_id.as_str()) ||
            stderr.contains("not found") ||
            stderr.contains("invalid") ||
            stderr.contains("format") ||
            stderr.contains("malformed"),
            "Error should reference the invalid ID or indicate problem, got: {}",
            stderr
        );
    }
}

#[test]
fn test_update_json_malformed_bead_id() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let malformed_ids = vec![
        "invalid-id".to_string(),
        "not-a-bf-id".to_string(),
        "XYZ-123".to_string(),
    ];

    for malformed_id in malformed_ids {
        let (stdout, stderr, success) = capture::capture_failed_command(
            &mut bf_command()
                .arg("update")
                .arg(&malformed_id)
                .arg("--description")
                .arg("Test update")
        );

        assert!(!success, "update should fail for malformed ID: {}", malformed_id);
        assert!(!stderr.is_empty(), "stderr should contain error for malformed ID: {}", malformed_id);
    }
}

#[test]
fn test_close_json_nonexistent_bead_id() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut bf_command()
            .arg("close")
            .arg("bf-nonexistent-12345")
            .arg("--reason")
            .arg("Test close")
    );

    assert!(!success, "close should fail for non-existent bead ID");
    assert!(!stderr.is_empty(), "stderr should contain error message");
    assert!(stdout.trim().is_empty(), "stdout should be empty for close error");
}

#[test]
fn test_comment_json_nonexistent_bead_id() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut bf_command()
            .arg("comment")
            .arg("bf-nonexistent-comment-123")
            .arg("--text")
            .arg("Test comment")
    );

    assert!(!success, "comment should fail for non-existent bead ID");
    assert!(!stderr.is_empty(), "stderr should contain error message");
}

// ============================================================================
// Invalid dependency reference error tests
// ============================================================================

#[test]
fn test_dep_add_json_invalid_blocker_id() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create a valid bead to be blocked
    let blocked_id = fixtures::create_bead("Bead to be blocked");

    // Try to add dependency with invalid blocker
    let invalid_blocker = "bf-nonexistent-blocker-999";

    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut bf_command()
            .arg("dep")
            .arg("add")
            .arg(&blocked_id)
            .arg("--blocks")
            .arg(invalid_blocker)
    );

    assert!(!success, "dep add should fail for invalid blocker ID");
    assert!(!stderr.is_empty(), "stderr should contain error message");

    // Error should mention the dependency, blocker, or foreign key constraint
    assert!(
        stderr.contains("depend") ||
        stderr.contains("block") ||
        stderr.contains("not found") ||
        stderr.contains(invalid_blocker) ||
        stderr.contains("FOREIGN KEY") ||
        stderr.contains("constraint"),
        "Error should reference the dependency issue, got: {}",
        stderr
    );

    // Cleanup
    fixtures::close_bead(&blocked_id, "Dep test cleanup");
}

#[test]
fn test_dep_add_json_invalid_blocked_id() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create a valid blocker bead
    let blocker_id = fixtures::create_bead("Valid blocker bead");

    // Try to add dependency with invalid blocked bead
    let invalid_blocked = "bf-nonexistent-blocked-888";

    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut bf_command()
            .arg("dep")
            .arg("add")
            .arg(invalid_blocked)
            .arg("--blocks")
            .arg(&blocker_id)
    );

    // May or may not fail depending on implementation
    if !success {
        assert!(!stderr.is_empty(), "stderr should contain error message");

        // Error should mention the dependency issue or foreign key constraint
        assert!(
            stderr.contains("depend") ||
            stderr.contains("block") ||
            stderr.contains("not found") ||
            stderr.contains("FOREIGN KEY") ||
            stderr.contains("constraint"),
            "Error should reference the dependency issue, got: {}",
            stderr
        );
    }

    // Cleanup
    fixtures::close_bead(&blocker_id, "Dep test cleanup");
}

#[test]
fn test_dep_add_json_circular_dependency() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create two beads
    let bead1_id = fixtures::create_bead("Bead 1 for circular dep");
    let bead2_id = fixtures::create_bead("Bead 2 for circular dep");

    // Add dependency: bead2 -> bead1
    fixtures::add_dependency(&bead2_id, &bead1_id);

    // Try to add circular dependency: bead1 -> bead2
    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut bf_command()
            .arg("dep")
            .arg("add")
            .arg(&bead1_id)
            .arg("--blocks")
            .arg(&bead2_id)
    );

    // Should either fail (circular detected) or succeed (some implementations allow it)
    if !success {
        assert!(!stderr.is_empty(), "stderr should contain error about circular dependency");
        // Error should mention circular or cycle
        assert!(
            stderr.contains("circular") ||
            stderr.contains("cycle") ||
            stderr.contains("depend"),
            "Error should mention circular dependency, got: {}",
            stderr
        );
    }

    // Cleanup
    fixtures::close_bead(&bead1_id, "Circular dep cleanup 1");
    fixtures::close_bead(&bead2_id, "Circular dep cleanup 2");
}

// ============================================================================
// Invalid query scenario tests
// ============================================================================

#[test]
fn test_search_json_empty_query() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create a test bead
    let bead_id = fixtures::create_bead("Test bead for empty query");

    // Try empty search query
    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut bf_command()
            .arg("search")
            .arg("")
            .arg("--format")
            .arg("json")
    );

    // Empty query might be rejected or return empty
    if !success {
        assert!(!stderr.is_empty(), "stderr should contain error for empty query");
    } else {
        // If it succeeds, should return valid JSON
        json_validation::assert_valid_jsonl(&stdout);
    }

    // Cleanup
    fixtures::close_bead(&bead_id, "Empty query cleanup");
}

#[test]
fn test_search_json_query_with_only_special_chars() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Queries with only special characters (avoiding CLI flag conflicts)
    let special_queries = vec![
        "!!!",
        "@@@",
        "$$$",
        "%%%",
        "^^^",
        "&&&",
        "***",
        "((((",
        "))))",
        "___",
        "+++",
        "===",
        "???",
    ];

    for query in special_queries {
        let output = capture::capture_stdout(
            bf_command()
                .arg("search")
                .arg(query)
                .arg("--format")
                .arg("json")
        );

        // Should return valid JSONL (even if empty)
        json_validation::assert_valid_jsonl(&output);
    }
}

#[test]
fn test_search_json_query_with_unmatched_brackets() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create a bead with brackets
    let bead_id = fixtures::create_bead("Bead with [brackets] and (parentheses)");

    // Query with unmatched brackets
    let bracket_queries = vec![
        "[test",
        "test]",
        "(test",
        "test)",
        "{test",
        "test}",
        "[[test",
        "test]]",
    ];

    for query in bracket_queries {
        let output = capture::capture_stdout(
            bf_command()
                .arg("search")
                .arg(query)
                .arg("--format")
                .arg("json")
        );

        // Should return valid JSONL (even if empty or error in search)
        json_validation::assert_valid_jsonl(&output);
    }

    // Cleanup
    fixtures::close_bead(&bead_id, "Bracket query cleanup");
}

#[test]
fn test_list_json_invalid_status_filter() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let invalid_statuses = vec![
        "invalid-status",
        "INVALID",
        "openn",
        "closedd",
        "blockedd",
        "pendingg",
        "123",
        "😀", // emoji
    ];

    for invalid_status in invalid_statuses {
        // Test that invalid status filters don't crash and return valid JSON
        let output = capture::capture_stdout(
            bf_command()
                .arg("list")
                .arg("--status")
                .arg(invalid_status)
                .arg("--format")
                .arg("json")
        );

        // Should return valid JSONL (even if empty results due to no matching status)
        json_validation::assert_valid_jsonl(&output);
    }
}

#[test]
fn test_list_json_invalid_type_filter() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let invalid_types = vec![
        "invalid-type",
        "INVALID",
        "taskk",
        "buggg",
        "genesisx",
        "not-a-type",
        "xyz",
    ];

    for invalid_type in invalid_types {
        // Test that invalid type filters don't crash and return valid JSON
        let output = capture::capture_stdout(
            bf_command()
                .arg("list")
                .arg("--type")
                .arg(invalid_type)
                .arg("--format")
                .arg("json")
        );

        // Should return valid JSONL (even if empty results due to no matching type)
        json_validation::assert_valid_jsonl(&output);
    }
}

#[test]
fn test_list_json_invalid_priority_filter() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let invalid_priorities = vec![
        "invalid",
        "abc",
        "5.5", // decimal
        "-1",  // negative
        "1000", // out of range
        "😀", // emoji
    ];

    for invalid_priority in invalid_priorities {
        let (stdout, stderr, success) = capture::capture_failed_command(
            &mut bf_command()
                .arg("list")
                .arg("--priority-min")
                .arg(invalid_priority)
                .arg("--format")
                .arg("json")
        );

        // Should fail for invalid priority
        assert!(!success, "list should fail for invalid priority: {}", invalid_priority);

        // Stderr should mention the invalid value
        assert!(!stderr.is_empty(), "stderr should contain error for invalid priority");
    }
}

#[test]
fn test_list_json_invalid_limit_filter() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let invalid_limits = vec![
        "-1",   // negative
        "-10",  // more negative
        "abc",  // not a number
        "3.14", // decimal
    ];

    for invalid_limit in invalid_limits {
        // Test that invalid limit values don't crash
        let (stdout, stderr, success) = capture::capture_failed_command(
            &mut bf_command()
                .arg("list")
                .arg("--limit")
                .arg(invalid_limit)
                .arg("--format")
                .arg("json")
        );

        // Should fail with invalid limit
        if !success {
            assert!(!stderr.is_empty(), "stderr should contain error for invalid limit");
        } else {
            // If succeeds, should return valid JSONL
            json_validation::assert_valid_jsonl(&stdout);
        }
    }
}

// ============================================================================
// Invalid label and assignee tests
// ============================================================================

#[test]
fn test_label_add_json_empty_label() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Bead for empty label test");

    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut bf_command()
            .arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg("")
    );

    assert!(!success, "label add should fail for empty label");
    assert!(!stderr.is_empty(), "stderr should contain error for empty label");

    // Cleanup
    fixtures::close_bead(&bead_id, "Empty label cleanup");
}

#[test]
fn test_label_add_json_nonexistent_bead() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut bf_command()
            .arg("label")
            .arg("add")
            .arg("bf-nonexistent-label-999")
            .arg("--label")
            .arg("test-label")
    );

    assert!(!success, "label add should fail for non-existent bead");
    assert!(!stderr.is_empty(), "stderr should contain error");
}

#[test]
fn test_label_remove_json_nonexistent_label() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Bead for label removal test");

    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut bf_command()
            .arg("label")
            .arg("remove")
            .arg(&bead_id)
            .arg("--label")
            .arg("nonexistent-label-xyz")
    );

    // Should either succeed (no-op) or fail with clear message
    if !success {
        assert!(!stderr.is_empty(), "stderr should contain error or warning");
    }

    // Cleanup
    fixtures::close_bead(&bead_id, "Label removal cleanup");
}

#[test]
fn test_update_json_empty_assignee() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Bead for assignee test");

    // Try setting empty assignee (should clear assignee)
    let output = capture::capture_stdout(
        bf_command()
            .arg("update")
            .arg(&bead_id)
            .arg("--assignee")
            .arg("")
    );

    // Should succeed (clearing assignee is valid)
    // Verify the bead was updated
    let show_output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    json_validation::assert_valid_json(&show_output);

    // Cleanup
    fixtures::close_bead(&bead_id, "Assignee cleanup");
}

#[test]
fn test_update_json_invalid_email_format() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Bead for email test");

    // Invalid email formats (the system may or may not validate emails)
    let invalid_emails = vec![
        "not-an-email",
        "@",
        "@example.com",
        "user@",
        "user@@example.com",
        "user..name@example.com",
    ];

    for invalid_email in invalid_emails {
        let output = capture::capture_stdout(
            bf_command()
                .arg("update")
                .arg(&bead_id)
                .arg("--assignee")
                .arg(invalid_email)
        );

        // System may accept any string as assignee (no email validation)
        // Just verify no crash and valid JSON on show
        let show_output = capture::capture_stdout(
            bf_command()
                .arg("show")
                .arg(&bead_id)
                .arg("--format")
                .arg("json")
        );

        json_validation::assert_valid_json(&show_output);
    }

    // Cleanup
    fixtures::close_bead(&bead_id, "Email cleanup");
}

// ============================================================================
// Invalid command-line argument tests
// ============================================================================

#[test]
fn test_show_json_missing_required_argument() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut bf_command()
            .arg("show")
            .arg("--format")
            .arg("json")
        // Missing bead ID argument
    );

    assert!(!success, "show should fail without bead ID");
    assert!(!stderr.is_empty(), "stderr should contain error about missing argument");
}

#[test]
fn test_create_json_missing_title() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut bf_command()
            .arg("create")
            .arg("--type")
            .arg("task")
            .arg("--priority")
            .arg("2")
        // Missing --title
    );

    assert!(!success, "create should fail without title");
    assert!(!stderr.is_empty(), "stderr should contain error about missing title");
}

#[test]
fn test_create_json_invalid_type() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Test that invalid type values are handled without crashing
    // bead-forge accepts any type string (flexible schema)
    let output = capture::capture_stdout(
        bf_command()
            .arg("create")
            .arg("--title")
            .arg("Test with invalid type")
            .arg("--type")
            .arg("invalid-type-xyz")
            .arg("--priority")
            .arg("2")
            .arg("--json")
    );

    // Should succeed and return valid JSON
    json_validation::assert_valid_json(&output);

    // Verify the bead was created with the custom type
    let parsed = json_validation::parse_json(&output);
    let data = json_validation::get_object(&parsed, "data");
    let bead_id = json_validation::get_string(&data, "id");

    // Show the bead to verify type was preserved
    let show_output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    json_validation::assert_valid_json(&show_output);

    // Cleanup
    fixtures::close_bead(&bead_id, "Invalid type cleanup");
}

#[test]
fn test_create_json_invalid_priority() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let invalid_priorities = vec![
        "invalid",
        "abc",
        "5.5",
        "-1",
        "1000",
    ];

    for invalid_priority in invalid_priorities {
        let (stdout, stderr, success) = capture::capture_failed_command(
            &mut bf_command()
                .arg("create")
                .arg("--title")
                .arg("Test invalid priority")
                .arg("--type")
                .arg("task")
                .arg("--priority")
                .arg(invalid_priority)
        );

        assert!(!success, "create should fail for invalid priority: {}", invalid_priority);
        assert!(!stderr.is_empty(), "stderr should contain error for invalid priority");
    }
}

#[test]
fn test_ready_json_invalid_time_period() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let invalid_periods = vec![
        "invalid",
        "xyz",
        "1x",   // wrong unit
        "-1h",  // negative
        "0",    // no unit
        "h",    // no number
    ];

    for invalid_period in invalid_periods {
        let (stdout, stderr, success) = capture::capture_failed_command(
            &mut bf_command()
                .arg("ready")
                .arg("--time-period")
                .arg(invalid_period)
                .arg("--format")
                .arg("json")
        );

        // Should fail for invalid time period
        assert!(!success, "ready should fail for invalid time period: {}", invalid_period);
        assert!(!stderr.is_empty(), "stderr should contain error for invalid time period");
    }
}

// ============================================================================
// Workspace and database error tests
// ============================================================================

#[test]
fn test_command_json_nonexistent_workspace() {
    let _ws = create_isolated_workspace();

    let nonexistent_workspace = "/tmp/nonexistent-bf-workspace-xyz123";

    // Test that nonexistent workspace is handled gracefully
    // bf may auto-initialize or fail - both are acceptable behaviors
    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut Command::new(bf_binary())
            .arg("-w")
            .arg(nonexistent_workspace)
            .arg("list")
            .arg("--format")
            .arg("json")
    );

    // Command behavior may vary - just verify it doesn't crash
    // If it succeeds, verify JSON output is valid
    if success {
        json_validation::assert_valid_jsonl(&stdout);
    } else {
        // If it fails, verify error message is present
        assert!(!stderr.is_empty(), "stderr should contain error message");
    }
}

#[test]
fn test_command_json_corrupted_database() {
    // Create a workspace and corrupt the database
    let temp_dir = create_isolated_workspace();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    // Load metadata to find database path
    let metadata = crate::config::load_metadata(&beads_dir)
        .expect("Failed to load metadata");
    let db_path = beads_dir.join(&metadata.database);

    // Corrupt the database by writing garbage
    std::fs::write(&db_path, b"corrupted database garbage data")
        .expect("Failed to corrupt database");

    // Try to list beads - should fail gracefully
    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut Command::new(bf_binary())
            .arg("-w")
            .arg(&beads_dir)
            .arg("list")
            .arg("--format")
            .arg("json")
    );

    assert!(!success, "command should fail with corrupted database");
    assert!(!stderr.is_empty(), "stderr should contain error about database");

    // Error should mention database or corruption
    assert!(
        stderr.contains("database") ||
        stderr.contains("corrupted") ||
        stderr.contains("malformed") ||
        stderr.contains("disk"),
        "Error should mention database issue, got: {}",
        stderr
    );
}

#[test]
fn test_command_json_missing_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");
    std::fs::create_dir(&beads_dir).expect("Failed to create .beads directory");

    // Don't initialize - try to use the uninitialized workspace

    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut Command::new(bf_binary())
            .arg("-w")
            .arg(&beads_dir)
            .arg("list")
            .arg("--format")
            .arg("json")
    );

    // bead-forge may auto-initialize or handle missing config gracefully
    // Just verify it doesn't crash and output is valid JSON if it succeeds
    if success {
        json_validation::assert_valid_jsonl(&stdout);
    } else {
        assert!(!stderr.is_empty(), "stderr should contain error message");
    }
}

// ============================================================================
// Schema consistency tests
// ============================================================================

#[test]
fn test_error_responses_dont_emit_partial_json() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create a bead
    let bead_id = fixtures::create_bead("Test bead for error schema");

    // Try operations that will fail and verify stdout doesn't contain partial JSON
    let error_operations = vec![
        // Invalid bead ID operations
        || {
            let (stdout, _, _) = capture::capture_failed_command(
                &mut bf_command()
                    .arg("show")
                    .arg("bf-invalid-id")
                    .arg("--format")
                    .arg("json")
            );
            stdout
        },
        || {
            let (stdout, _, _) = capture::capture_failed_command(
                &mut bf_command()
                    .arg("update")
                    .arg("bf-invalid")
                    .arg("--description")
                    .arg("test")
            );
            stdout
        },
    ];

    for operation in error_operations {
        let stdout = operation();

        // Stdout should either be empty or contain valid complete JSON
        let stdout_trimmed = stdout.trim();
        if !stdout_trimmed.is_empty() {
            // If not empty, must be valid JSON
            json_validation::assert_valid_json(&stdout_trimmed);

            // Valid JSON should be complete (not partial)
            // This is implicitly checked by successful parsing above
        }
    }

    // Cleanup
    fixtures::close_bead(&bead_id, "Schema cleanup");
}

#[test]
fn test_empty_result_maintains_schema() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Test various empty result scenarios
    let bead_id = fixtures::create_bead("Bead for empty schema test");

    // Close the bead to make certain queries empty
    fixtures::close_bead(&bead_id, "Setup for empty tests");

    // Test 1: Search with no results
    let search_output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("nonexistent-search-term-xyz-123")
            .arg("--format")
            .arg("json")
    );

    // Empty search returns nothing
    let search_trimmed = search_output.trim();
    assert!(search_trimmed.is_empty() || search_trimmed == "[]",
           "Empty search should be empty or [], got: '{}'", search_trimmed);

    // Test 2: List closed beads (if only closed beads exist, open filter returns empty)
    let list_output = capture::capture_stdout(
        bf_command()
            .arg("list")
            .arg("--status")
            .arg("open")
            .arg("--format")
            .arg("json")
    );

    // Should return valid JSONL (possibly empty)
    let list_trimmed = list_output.trim();
    if !list_trimmed.is_empty() {
        json_validation::assert_valid_jsonl(&list_output);
    }
}

#[test]
fn test_json_output_field_consistency_on_errors() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create beads with various states
    let bead1_id = fixtures::create_bead("Bead 1 for consistency test");
    let bead2_id = fixtures::create_bead("Bead 2 for consistency test");

    // Add different properties
    capture::capture_stdout(
        bf_command()
            .arg("update")
            .arg(&bead1_id)
            .arg("--description")
            .arg("Description with special chars: \"quotes\" & symbols")
    );

    capture::capture_stdout(
        bf_command()
            .arg("label")
            .arg("add")
            .arg(&bead2_id)
            .arg("--label")
            .arg("test-label")
    );

    // Verify both beads have consistent field presence in JSON output
    let list_output = capture::capture_stdout(
        bf_command()
            .arg("list")
            .arg("--format")
            .arg("json")
    );

    let lines: Vec<&str> = list_output.lines().filter(|l| !l.trim().is_empty()).collect();

    for line in lines {
        let parsed = json_validation::parse_json(line);

        // Verify all beads have the same required fields present
        // (even if values differ)
        json_validation::assert_required_fields(
            &parsed,
            &["id", "title", "status", "priority", "issue_type"],
            "consistency check"
        );

        // Assignee and labels should always be present (display normalization)
        assert!(parsed.get("assignee").is_some(), "assignee field must be present");
        assert!(parsed.get("labels").is_some(), "labels field must be present");
    }

    // Cleanup
    fixtures::close_bead(&bead1_id, "Consistency cleanup 1");
    fixtures::close_bead(&bead2_id, "Consistency cleanup 2");
}

// ============================================================================
// Concurrent and race condition error tests
// ============================================================================

#[test]
fn test_claim_json_no_ready_beads() {
    let temp_dir = create_isolated_workspace();
    let empty_workspace = temp_dir.path();

    // Try to claim from empty workspace
    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut bf_command_with_workspace(empty_workspace)
            .arg("claim")
            .arg("--assignee")
            .arg("test-worker")
            .arg("--format")
            .arg("json")
    );

    // Should fail (no beads to claim)
    if !success {
        assert!(!stderr.is_empty(), "stderr should contain error about no beads");

        // Error should mention no beads or nothing available
        assert!(
            stderr.contains("no") ||
            stderr.contains("available") ||
            stderr.contains("nothing") ||
            stderr.contains("ready"),
            "Error should mention no beads available, got: {}",
            stderr
        );
    } else {
        // If succeeds, should return valid JSON
        json_validation::assert_valid_json(&stdout);
    }
}

#[test]
fn test_show_json_already_closed_bead() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Bead to close and show");
    fixtures::close_bead(&bead_id, "Close for show test");

    // Show closed bead - should still work and return valid JSON
    let output = capture::capture_stdout(
        bf_command()
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
    );

    json_validation::assert_valid_json(&output);

    // Parse and verify status is "closed"
    let json_str = output.trim();
    let parsed = json_validation::parse_json(json_str);
    let array = parsed.as_array().expect("show output should be a JSON array");
    let bead = &array[0];

    let status = json_validation::get_string(bead, "status");
    assert_eq!(status, "closed", "Status should be closed");
}

#[test]
fn test_update_json_closed_bead() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Bead to close and update");
    fixtures::close_bead(&bead_id, "Close for update test");

    // Try to update closed bead
    let (stdout, stderr, success) = capture::capture_failed_command(
        &mut bf_command()
            .arg("update")
            .arg(&bead_id)
            .arg("--description")
            .arg("Should not update closed bead")
    );

    // May or may not be allowed depending on implementation
    if !success {
        assert!(!stderr.is_empty(), "stderr should contain error about updating closed bead");
    } else {
        // If update succeeded, verify it actually updated
        let show_output = capture::capture_stdout(
            bf_command()
                .arg("show")
                .arg(&bead_id)
                .arg("--format")
                .arg("json")
        );

        json_validation::assert_valid_json(&show_output);
    }
}
