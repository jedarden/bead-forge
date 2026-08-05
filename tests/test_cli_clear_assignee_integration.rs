//! Integration tests for CLI assignee clearing behavior
//!
//! This test file validates the complete `bf update --clear-assignee` CLI flow:
//! 1. Parse args → update storage → verify assignee cleared
//! 2. Test that `bf update --clear-assignee <id>` clears the assignee field
//! 3. Test that the bead remains otherwise unchanged (title, description, status, etc.)
//! 4. Add tests that exercise the CLI command end-to-end
//!
//! These tests complement the existing tests in `stale_assignee_clearing_workflow.rs`
//! by providing more comprehensive field preservation verification.

use std::process::Command;
use tempfile::TempDir;

/// Helper function to run bf commands and return output
fn run_bf_command(args: &[&str], workspace_dir: &std::path::Path) -> (bool, String, String) {
    let output = Command::new("bf")
        .args(args)
        .current_dir(workspace_dir)
        .output()
        .expect("Failed to run bf command");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (success, stdout, stderr)
}

/// Helper function to create a test bead with comprehensive fields
fn create_test_bead_with_all_fields(
    workspace_dir: &std::path::Path,
    assignee: &str,
) -> String {
    let (success, stdout, stderr) = run_bf_command(
        &[
            "create",
            "--title",
            "Test bead for assignee clearing",
            "--description",
            "This is a test description for the bead",
            "--type",
            "task",
            "--priority",
            "2",
            "--assignee",
            assignee,
            "--json",
        ],
        workspace_dir,
    );

    assert!(success, "bf create failed: {}", stderr);

    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Failed to parse bf create JSON output");
    json["id"]
        .as_str()
        .expect("Bead ID not found in create output")
        .to_string()
}

/// Helper function to get bead data as JSON
fn get_bead_json(workspace_dir: &std::path::Path, bead_id: &str) -> serde_json::Value {
    let (success, stdout, stderr) = run_bf_command(
        &["show", bead_id, "--format", "json", "--envelope"],
        workspace_dir,
    );

    assert!(success, "bf show failed: {}", stderr);

    serde_json::from_str(&stdout).expect("Failed to parse bf show JSON output")
}

#[test]
fn test_cli_clear_assignee_clears_only_assignee_field() {
    // Test that --clear-assignee clears only the assignee field and nothing else
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace_dir = temp_dir.path().join("test-workspace-1");
    std::fs::create_dir_all(&workspace_dir).expect("Failed to create workspace");

    // Initialize bf workspace
    let (init_success, _, init_stderr) = run_bf_command(
        &["init", "--prefix", "test"],
        &workspace_dir,
    );
    assert!(
        init_success,
        "bf init failed: {}",
        init_stderr
    );

    // Create a test bead with all fields set
    let bead_id = create_test_bead_with_all_fields(&workspace_dir, "test-worker-1");

    // Get the initial bead state
    let initial_bead = get_bead_json(&workspace_dir, &bead_id);
    let initial_data = &initial_bead["data"];

    // Record all field values before clearing
    let initial_title = initial_data["title"].as_str().unwrap();
    let initial_description = initial_data["description"].as_str().unwrap();
    let initial_status = initial_data["status"].as_str().unwrap();
    let initial_priority = initial_data["priority"].as_i64().unwrap();
    let initial_assignee = initial_data["assignee"].as_str().unwrap();
    let initial_type = initial_data["type"].as_str().unwrap();

    // Verify initial state
    assert_eq!(initial_assignee, "test-worker-1", "Initial assignee should be set");

    // Clear the assignee using --clear-assignee
    let (update_success, update_stdout, update_stderr) = run_bf_command(
        &["update", &bead_id, "--clear-assignee"],
        &workspace_dir,
    );
    assert!(
        update_success,
        "bf update --clear-assignee failed: {}",
        update_stderr
    );

    // Get the bead state after clearing
    let updated_bead = get_bead_json(&workspace_dir, &bead_id);
    let updated_data = &updated_bead["data"];

    // Verify assignee is cleared
    let cleared_assignee = updated_data["assignee"].as_str();
    assert_eq!(
        cleared_assignee,
        None,
        "Assignee should be NULL (null in JSON) after --clear-assignee"
    );

    // Verify all other fields are unchanged
    assert_eq!(
        updated_data["title"].as_str().unwrap(),
        initial_title,
        "Title should remain unchanged"
    );
    assert_eq!(
        updated_data["description"].as_str().unwrap(),
        initial_description,
        "Description should remain unchanged"
    );
    assert_eq!(
        updated_data["status"].as_str().unwrap(),
        initial_status,
        "Status should remain unchanged"
    );
    assert_eq!(
        updated_data["priority"].as_i64().unwrap(),
        initial_priority,
        "Priority should remain unchanged"
    );
    assert_eq!(
        updated_data["type"].as_str().unwrap(),
        initial_type,
        "Type should remain unchanged"
    );
}

#[test]
fn test_cli_clear_assignee_with_concurrent_updates() {
    // Test that --clear-assignee works when combined with other field updates
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace_dir = temp_dir.path().join("test-workspace-2");
    std::fs::create_dir_all(&workspace_dir).expect("Failed to create workspace");

    // Initialize bf workspace
    let (init_success, _, init_stderr) = run_bf_command(
        &["init", "--prefix", "test"],
        &workspace_dir,
    );
    assert!(
        init_success,
        "bf init failed: {}",
        init_stderr
    );

    // Create a test bead
    let bead_id = create_test_bead_with_all_fields(&workspace_dir, "test-worker-2");

    // Get initial state
    let initial_bead = get_bead_json(&workspace_dir, &bead_id);
    let initial_data = &initial_bead["data"];

    // Clear assignee and update other fields in the same command
    let (update_success, _, update_stderr) = run_bf_command(
        &[
            "update",
            &bead_id,
            "--clear-assignee",
            "--title",
            "Updated title",
            "--status",
            "in_progress",
            "--priority",
            "1",
        ],
        &workspace_dir,
    );
    assert!(
        update_success,
        "bf update with multiple flags failed: {}",
        update_stderr
    );

    // Get updated state
    let updated_bead = get_bead_json(&workspace_dir, &bead_id);
    let updated_data = &updated_bead["data"];

    // Verify assignee is cleared
    assert_eq!(
        updated_data["assignee"].as_str(),
        None,
        "Assignee should be NULL when --clear-assignee is used"
    );

    // Verify other fields were updated correctly
    assert_eq!(
        updated_data["title"].as_str().unwrap(),
        "Updated title",
        "Title should be updated"
    );
    assert_eq!(
        updated_data["status"].as_str().unwrap(),
        "in_progress",
        "Status should be updated"
    );
    assert_eq!(
        updated_data["priority"].as_i64().unwrap(),
        1,
        "Priority should be updated"
    );

    // Verify description remains unchanged (we didn't update it)
    assert_eq!(
        updated_data["description"].as_str().unwrap(),
        initial_data["description"].as_str().unwrap(),
        "Description should remain unchanged"
    );
}

#[test]
fn test_cli_clear_assignee_idempotent() {
    // Test that clearing an already-unassigned bead is idempotent (no-op)
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace_dir = temp_dir.path().join("test-workspace-3");
    std::fs::create_dir_all(&workspace_dir).expect("Failed to create workspace");

    // Initialize bf workspace
    let (init_success, _, init_stderr) = run_bf_command(
        &["init", "--prefix", "test"],
        &workspace_dir,
    );
    assert!(
        init_success,
        "bf init failed: {}",
        init_stderr
    );

    // Create a bead without an assignee
    let (create_success, create_stdout, create_stderr) = run_bf_command(
        &[
            "create",
            "--title",
            "Test bead without assignee",
            "--type",
            "task",
            "--priority",
            "2",
            "--json",
        ],
        &workspace_dir,
    );
    assert!(
        create_success,
        "bf create failed: {}",
        create_stderr
    );

    let create_json: serde_json::Value = serde_json::from_str(&create_stdout)
        .expect("Failed to parse create JSON");
    let bead_id = create_json["id"]
        .as_str()
        .expect("No bead ID")
        .to_string();

    // Verify the bead has no assignee
    let initial_bead = get_bead_json(&workspace_dir, &bead_id);
    assert_eq!(
        initial_bead["data"]["assignee"].as_str(),
        None,
        "Bead should initially have no assignee"
    );

    // Try to clear the assignee (should be a no-op)
    let (update_success, _, update_stderr) = run_bf_command(
        &["update", &bead_id, "--clear-assignee"],
        &workspace_dir,
    );
    assert!(
        update_success,
        "bf update --clear-assignee on unassigned bead should succeed: {}",
        update_stderr
    );

    // Verify the bead still has no assignee
    let updated_bead = get_bead_json(&workspace_dir, &bead_id);
    assert_eq!(
        updated_bead["data"]["assignee"].as_str(),
        None,
        "Assignee should still be NULL after clearing an already-unassigned bead"
    );

    // Verify all other fields are unchanged
    assert_eq!(
        updated_bead["data"]["title"].as_str().unwrap(),
        initial_bead["data"]["title"].as_str().unwrap(),
        "Title should remain unchanged"
    );
    assert_eq!(
        updated_bead["data"]["status"].as_str().unwrap(),
        initial_bead["data"]["status"].as_str().unwrap(),
        "Status should remain unchanged"
    );
}

#[test]
fn test_cli_clear_assignee_with_json_output() {
    // Test that --clear-assignee works correctly with --json output flag
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace_dir = temp_dir.path().join("test-workspace-4");
    std::fs::create_dir_all(&workspace_dir).expect("Failed to create workspace");

    // Initialize bf workspace
    let (init_success, _, init_stderr) = run_bf_command(
        &["init", "--prefix", "test"],
        &workspace_dir,
    );
    assert!(
        init_success,
        "bf init failed: {}",
        init_stderr
    );

    // Create a bead with assignee
    let bead_id = create_test_bead_with_all_fields(&workspace_dir, "test-worker-json");

    // Clear assignee with --json output
    let (update_success, update_stdout, update_stderr) = run_bf_command(
        &["update", &bead_id, "--clear-assignee", "--json"],
        &workspace_dir,
    );
    assert!(
        update_success,
        "bf update --clear-assignee --json failed: {}",
        update_stderr
    );

    // Parse JSON output
    let update_json: serde_json::Value = serde_json::from_str(&update_stdout)
        .expect("Failed to parse update JSON output");

    // Verify the response structure
    assert_eq!(
        update_json["id"].as_str().unwrap(),
        bead_id,
        "JSON response should include the bead ID"
    );
    assert_eq!(
        update_json["updated"].as_bool().unwrap(),
        true,
        "JSON response should indicate successful update"
    );

    // Verify the actual storage change
    let bead = get_bead_json(&workspace_dir, &bead_id);
    assert_eq!(
        bead["data"]["assignee"].as_str(),
        None,
        "Assignee should be NULL in storage"
    );
}

#[test]
fn test_cli_clear_assignee_command_flow_from_parse_to_storage() {
    // Test the complete command flow: CLI parsing → execution → storage verification
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace_dir = temp_dir.path().join("test-workspace-5");
    std::fs::create_dir_all(&workspace_dir).expect("Failed to create workspace");

    // Initialize bf workspace
    let (init_success, _, init_stderr) = run_bf_command(
        &["init", "--prefix", "test"],
        &workspace_dir,
    );
    assert!(
        init_success,
        "bf init failed: {}",
        init_stderr
    );

    // Create multiple beads with different assignees
    let bead_ids = vec![
        create_test_bead_with_all_fields(&workspace_dir, "worker-a"),
        create_test_bead_with_all_fields(&workspace_dir, "worker-b"),
        create_test_bead_with_all_fields(&workspace_dir, "worker-c"),
    ];

    // Clear assignee for each bead sequentially
    for (index, bead_id) in bead_ids.iter().enumerate() {
        let expected_worker = format!("worker-{}", char::from_u32('a' as u32 + index as u32).unwrap());

        // Verify initial state
        let initial_bead = get_bead_json(&workspace_dir, bead_id);
        assert_eq!(
            initial_bead["data"]["assignee"].as_str().unwrap(),
            expected_worker,
            "Bead {} should have initial assignee",
            bead_id
        );

        // Clear assignee
        let (update_success, _, update_stderr) = run_bf_command(
            &["update", bead_id, "--clear-assignee"],
            &workspace_dir,
        );
        assert!(
            update_success,
            "bf update --clear-assignee failed for bead {}: {}",
            bead_id,
            update_stderr
        );

        // Verify storage was updated
        let updated_bead = get_bead_json(&workspace_dir, bead_id);
        assert_eq!(
            updated_bead["data"]["assignee"].as_str(),
            None,
            "Bead {} should have NULL assignee after update",
            bead_id
        );
    }

    // Final verification: all beads should now have NULL assignees
    for bead_id in &bead_ids {
        let final_bead = get_bead_json(&workspace_dir, bead_id);
        assert_eq!(
            final_bead["data"]["assignee"].as_str(),
            None,
            "Final verification failed for bead {}",
            bead_id
        );
    }
}

#[test]
fn test_cli_clear_assignee_error_handling() {
    // Test error handling: trying to clear assignee on non-existent bead
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace_dir = temp_dir.path().join("test-workspace-6");
    std::fs::create_dir_all(&workspace_dir).expect("Failed to create workspace");

    // Initialize bf workspace
    let (init_success, _, init_stderr) = run_bf_command(
        &["init", "--prefix", "test"],
        &workspace_dir,
    );
    assert!(
        init_success,
        "bf init failed: {}",
        init_stderr
    );

    // Try to clear assignee on a non-existent bead
    let (update_success, _, update_stderr) = run_bf_command(
        &["update", "bf-nonexistent-123", "--clear-assignee"],
        &workspace_dir,
    );

    // The command should fail gracefully
    assert!(
        !update_success,
        "bf update --clear-assignee should fail for non-existent bead"
    );
    assert!(
        update_stderr.contains("not found") || update_stderr.contains("does not exist"),
        "Error message should indicate bead was not found"
    );
}

#[test]
fn test_cli_clear_assignee_preserves_created_at_timestamp() {
    // Test that clearing assignee preserves the created_at timestamp
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workspace_dir = temp_dir.path().join("test-workspace-7");
    std::fs::create_dir_all(&workspace_dir).expect("Failed to create workspace");

    // Initialize bf workspace
    let (init_success, _, init_stderr) = run_bf_command(
        &["init", "--prefix", "test"],
        &workspace_dir,
    );
    assert!(
        init_success,
        "bf init failed: {}",
        init_stderr
    );

    // Create a bead with assignee
    let bead_id = create_test_bead_with_all_fields(&workspace_dir, "timestamp-test-worker");

    // Get initial bead state including created_at
    let initial_bead = get_bead_json(&workspace_dir, &bead_id);
    let initial_created_at = initial_bead["data"]["created_at"]
        .as_str()
        .expect("created_at should be present");

    // Clear assignee
    let (update_success, _, update_stderr) = run_bf_command(
        &["update", &bead_id, "--clear-assignee"],
        &workspace_dir,
    );
    assert!(
        update_success,
        "bf update --clear-assignee failed: {}",
        update_stderr
    );

    // Get updated bead state
    let updated_bead = get_bead_json(&workspace_dir, &bead_id);
    let updated_created_at = updated_bead["data"]["created_at"]
        .as_str()
        .expect("created_at should be present after update");

    // Verify created_at is unchanged
    assert_eq!(
        initial_created_at, updated_created_at,
        "created_at timestamp should be preserved when clearing assignee"
    );

    // Verify updated_at has changed (it should reflect the update time)
    let initial_updated_at = initial_bead["data"]["updated_at"]
        .as_str()
        .expect("updated_at should be present initially");
    let updated_updated_at = updated_bead["data"]["updated_at"]
        .as_str()
        .expect("updated_at should be present after update");

    assert_ne!(
        initial_updated_at, updated_updated_at,
        "updated_at timestamp should change when assignee is cleared"
    );
}
