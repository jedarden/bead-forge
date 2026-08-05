//! Error handling tests for epic creation
//!
//! Tests that bf create properly handles invalid inputs for epic creation:
//! - Invalid priority values (negative, non-numeric, out of range)
//! - Invalid type values (unknown types)
//! - Missing required parameters (title, type)

use std::process::Command;

/// Helper to run bf create with arguments and capture output
fn run_bf_create(args: &[&str]) -> (String, String, bool) {
    let output = Command::new(env!("CARGO_BIN_EXE_bf"))
        .args(args)
        .output()
        .expect("Failed to execute bf command");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    (stdout, stderr, success)
}

#[test]
fn test_negative_priority_fails() {
    let (stdout, stderr, success) = run_bf_create(&[
        "create",
        "--title", "Test Epic",
        "--type", "epic",
        "--priority=-1",
    ]);

    // Should fail
    assert!(!success, "bf create with negative priority should fail");

    // Should contain an error message about invalid priority
    let output = format!("{}\n{}", stdout, stderr);
    assert!(
        output.to_lowercase().contains("priority") || output.to_lowercase().contains("invalid"),
        "Error message should mention priority or invalid input. Got: {}",
        output
    );
}

#[test]
fn test_non_numeric_priority_fails() {
    let (stdout, stderr, success) = run_bf_create(&[
        "create",
        "--title", "Test Epic",
        "--type", "epic",
        "--priority", "abc",
    ]);

    // Should fail
    assert!(!success, "bf create with non-numeric priority should fail");

    // Should contain an error message about invalid priority
    let output = format!("{}\n{}", stdout, stderr);
    assert!(
        output.to_lowercase().contains("priority") ||
        output.to_lowercase().contains("invalid") ||
        output.to_lowercase().contains("number"),
        "Error message should mention priority or invalid input. Got: {}",
        output
    );
}

#[test]
fn test_priority_out_of_range_fails() {
    let (stdout, stderr, success) = run_bf_create(&[
        "create",
        "--title", "Test Epic",
        "--type", "epic",
        "--priority", "5",
    ]);

    // Should fail (priority must be 0-4)
    assert!(!success, "bf create with priority > 4 should fail");

    // Should contain an error message about invalid priority
    let output = format!("{}\n{}", stdout, stderr);
    assert!(
        output.to_lowercase().contains("priority") || output.to_lowercase().contains("invalid"),
        "Error message should mention priority or invalid input. Got: {}",
        output
    );
}

#[test]
fn test_priority_zero_succeeds() {
    let (stdout, stderr, success) = run_bf_create(&[
        "create",
        "--title", "Critical Epic",
        "--type", "epic",
        "--priority", "0",
    ]);

    // Should succeed (priority 0 is Critical, which is valid)
    assert!(success, "bf create with priority 0 should succeed. stdout: {}, stderr: {}", stdout, stderr);

    // Should output a bead ID
    assert!(
        !stdout.trim().is_empty(),
        "Should output a bead ID on success"
    );
}

#[test]
fn test_priority_four_succeeds() {
    let (stdout, stderr, success) = run_bf_create(&[
        "create",
        "--title", "Backlog Epic",
        "--type", "epic",
        "--priority", "4",
    ]);

    // Should succeed (priority 4 is Backlog, which is valid)
    assert!(success, "bf create with priority 4 should succeed. stdout: {}, stderr: {}", stdout, stderr);

    // Should output a bead ID
    assert!(
        !stdout.trim().is_empty(),
        "Should output a bead ID on success"
    );
}

#[test]
fn test_unknown_type_fails() {
    // Test with a clearly invalid type
    let (stdout, stderr, success) = run_bf_create(&[
        "create",
        "--title", "Test Epic",
        "--type", "unknown_invalid_type_xyz123",
        "--priority", "0",
    ]);

    // Currently unknown types are accepted as Custom types
    // This test documents current behavior - change to assert!(!success)
    // if type validation is added
    let output = format!("{}\n{}", stdout, stderr);

    // For now, verify it either succeeds (current behavior) or provides a clear error
    if success {
        // Current behavior: unknown types become Custom types
        assert!(
            !stdout.trim().is_empty(),
            "Should output a bead ID even for custom type"
        );
    } else {
        // Future behavior: type validation
        assert!(
            output.to_lowercase().contains("type") || output.to_lowercase().contains("invalid"),
            "Error message should mention type or invalid input. Got: {}",
            output
        );
    }
}

#[test]
fn test_valid_type_epic_succeeds() {
    let (stdout, stderr, success) = run_bf_create(&[
        "create",
        "--title", "Test Epic",
        "--type", "epic",
        "--priority", "2",
    ]);

    // Should succeed
    assert!(success, "bf create with valid type 'epic' should succeed. stdout: {}, stderr: {}", stdout, stderr);

    // Should output a bead ID
    assert!(
        !stdout.trim().is_empty(),
        "Should output a bead ID on success"
    );
}

#[test]
fn test_valid_type_task_succeeds() {
    let (stdout, stderr, success) = run_bf_create(&[
        "create",
        "--title", "Test Task",
        "--type", "task",
        "--priority", "2",
    ]);

    // Should succeed
    assert!(success, "bf create with valid type 'task' should succeed. stdout: {}, stderr: {}", stdout, stderr);

    // Should output a bead ID
    assert!(
        !stdout.trim().is_empty(),
        "Should output a bead ID on success"
    );
}

#[test]
fn test_missing_title_fails() {
    let (stdout, stderr, success) = run_bf_create(&[
        "create",
        "--type", "epic",
        "--priority", "2",
    ]);

    // Should fail (--title is required)
    assert!(!success, "bf create without --title should fail");

    // Should contain an error message about missing required argument
    let output = format!("{}\n{}", stdout, stderr);
    assert!(
        output.to_lowercase().contains("required") || output.to_lowercase().contains("missing") || output.to_lowercase().contains("title"),
        "Error message should mention required argument or title. Got: {}",
        output
    );
}

#[test]
fn test_empty_title_fails() {
    let (stdout, stderr, success) = run_bf_create(&[
        "create",
        "--title", "",
        "--type", "epic",
        "--priority", "2",
    ]);

    // Should fail (empty title is rejected)
    assert!(!success, "bf create with empty title should fail");

    // Should contain an error message about empty title
    let output = format!("{}\n{}", stdout, stderr);
    assert!(
        output.to_lowercase().contains("title") || output.to_lowercase().contains("empty") || output.to_lowercase().contains("whitespace"),
        "Error message should mention title or empty/whitespace. Got: {}",
        output
    );
}

#[test]
fn test_whitespace_only_title_fails() {
    let (stdout, stderr, success) = run_bf_create(&[
        "create",
        "--title", "   ",
        "--type", "epic",
        "--priority", "2",
    ]);

    // Should fail (whitespace-only title is rejected)
    assert!(!success, "bf create with whitespace-only title should fail");

    // Should contain an error message about empty title
    let output = format!("{}\n{}", stdout, stderr);
    assert!(
        output.to_lowercase().contains("title") || output.to_lowercase().contains("empty") || output.to_lowercase().contains("whitespace"),
        "Error message should mention title or empty/whitespace. Got: {}",
        output
    );
}

#[test]
fn test_missing_type_with_default_succeeds() {
    let (stdout, stderr, success) = run_bf_create(&[
        "create",
        "--title", "Test Bead",
        "--priority", "2",
    ]);

    // Should succeed (--type defaults to "task")
    assert!(success, "bf create without --type should succeed with default. stdout: {}, stderr: {}", stdout, stderr);

    // Should output a bead ID
    assert!(
        !stdout.trim().is_empty(),
        "Should output a bead ID on success"
    );
}

#[test]
fn test_empty_type_fails() {
    let (stdout, stderr, success) = run_bf_create(&[
        "create",
        "--title", "Test Epic",
        "--type", "",
        "--priority", "2",
    ]);

    // Should fail (empty type is rejected)
    assert!(!success, "bf create with empty type should fail");

    // Should contain an error message about empty type
    let output = format!("{}\n{}", stdout, stderr);
    assert!(
        output.to_lowercase().contains("type") || output.to_lowercase().contains("empty") || output.to_lowercase().contains("whitespace"),
        "Error message should mention type or empty/whitespace. Got: {}",
        output
    );
}

#[test]
fn test_whitespace_only_type_fails() {
    let (stdout, stderr, success) = run_bf_create(&[
        "create",
        "--title", "Test Epic",
        "--type", "   ",
        "--priority", "2",
    ]);

    // Should fail (whitespace-only type is rejected)
    assert!(!success, "bf create with whitespace-only type should fail");

    // Should contain an error message about empty type
    let output = format!("{}\n{}", stdout, stderr);
    assert!(
        output.to_lowercase().contains("type") || output.to_lowercase().contains("empty") || output.to_lowercase().contains("whitespace"),
        "Error message should mention type or empty/whitespace. Got: {}",
        output
    );
}
