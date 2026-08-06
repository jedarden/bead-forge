//! CLI integration test for self-blocking prevention (bf-lsb3km)
//!
//! Tests the `bf dep add` command and batch operations to ensure they reject
//! self-blocking with informative error messages.

use std::process::Command;
use std::path::PathBuf;
use tempfile::TempDir;

/// Creates a temporary workspace with a test bead
fn setup_workspace_with_bead() -> (TempDir, String) {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path();

    // Initialize bf workspace
    let init_result = Command::new("cargo")
        .args(["run", "--", "init", "--prefix", "test"])
        .current_dir(workspace_dir)
        .output()
        .expect("Failed to run bf init");

    assert!(
        init_result.status.success(),
        "bf init failed: {}",
        String::from_utf8_lossy(&init_result.stderr)
    );

    // Create a test bead
    let create_result = Command::new("cargo")
        .args([
            "run", "--",
            "create",
            "--title", "Test Bead",
            "--type", "task",
            "--priority", "2",
        ])
        .current_dir(workspace_dir)
        .output()
        .expect("Failed to run bf create");

    assert!(
        create_result.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&create_result.stderr)
    );

    let bead_id = String::from_utf8(create_result.stdout)
        .expect("Invalid UTF-8 in output")
        .trim()
        .to_string();

    (temp_dir, bead_id)
}

#[test]
fn test_cli_dep_add_rejects_self_blocking() {
    let (_temp_dir, bead_id) = setup_workspace_with_bead();

    // Try to add self-blocking dependency via CLI
    let result = Command::new("cargo")
        .args([
            "run", "--",
            "dep", "add",
            &bead_id,
            "--blocks", &bead_id,
        ])
        .current_dir(_temp_dir.path())
        .output()
        .expect("Failed to run bf dep add");

    // Should fail
    assert!(
        !result.status.success(),
        "bf dep add should fail when trying to add self-blocking dependency"
    );

    let stderr = String::from_utf8_lossy(&result.stderr);
    let stdout = String::from_utf8_lossy(&result.stdout);
    let output = format!("{}\n{}", stderr, stdout);

    // Error message should be informative
    assert!(
        output.to_lowercase().contains("cannot") ||
        output.to_lowercase().contains("block itself") ||
        output.to_lowercase().contains("self-blocking"),
        "Error message should mention self-blocking prevention. Got: {}",
        output
    );

    // Should mention the bead ID
    assert!(
        output.contains(&bead_id) || output.contains("itself"),
        "Error message should reference the problematic bead. Got: {}",
        output
    );
}

#[test]
fn test_cli_dep_add_allows_different_beads() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path();

    // Initialize workspace
    let init_result = Command::new("cargo")
        .args(["run", "--", "init", "--prefix", "test"])
        .current_dir(workspace_dir)
        .output()
        .expect("Failed to run bf init");

    assert!(
        init_result.status.success(),
        "bf init failed: {}",
        String::from_utf8_lossy(&init_result.stderr)
    );

    // Create first bead
    let bead1_result = Command::new("cargo")
        .args([
            "run", "--",
            "create",
            "--title", "Blocker Bead",
            "--type", "task",
            "--priority", "2",
        ])
        .current_dir(workspace_dir)
        .output()
        .expect("Failed to run bf create");

    assert!(bead1_result.status.success());
    let bead1_id = String::from_utf8(bead1_result.stdout)
        .expect("Invalid UTF-8")
        .trim()
        .to_string();

    // Create second bead
    let bead2_result = Command::new("cargo")
        .args([
            "run", "--",
            "create",
            "--title", "Dependent Bead",
            "--type", "task",
            "--priority", "2",
        ])
        .current_dir(workspace_dir)
        .output()
        .expect("Failed to run bf create");

    assert!(bead2_result.status.success());
    let bead2_id = String::from_utf8(bead2_result.stdout)
        .expect("Invalid UTF-8")
        .trim()
        .to_string();

    // Add valid blocking dependency (bead2 depends on bead1)
    let dep_result = Command::new("cargo")
        .args([
            "run", "--",
            "dep", "add",
            &bead1_id,
            "--blocks", &bead2_id,
        ])
        .current_dir(workspace_dir)
        .output()
        .expect("Failed to run bf dep add");

    // Should succeed
    assert!(
        dep_result.status.success(),
        "bf dep add should succeed for different beads. Output: {}",
        String::from_utf8_lossy(&dep_result.stderr)
    );
}

#[test]
fn test_batch_dep_add_blocker_rejects_self_blocking() {
    let (_temp_dir, bead_id) = setup_workspace_with_bead();

    // Create batch operation that attempts self-blocking
    let batch_json = serde_json::json!([{
        "op": "dep_add_blocker",
        "id": bead_id,
        "blocker": bead_id
    }]);

    // Write batch to temp file
    let batch_file = _temp_dir.path().join("batch.json");
    std::fs::write(&batch_file, batch_json.to_string())
        .expect("Failed to write batch file");

    // Run batch operation
    let result = Command::new("cargo")
        .args([
            "run", "--",
            "batch",
            "--file", batch_file.to_str().unwrap(),
        ])
        .current_dir(_temp_dir.path())
        .output()
        .expect("Failed to run bf batch");

    // Should fail
    assert!(
        !result.status.success(),
        "bf batch should fail when trying to add self-blocking dependency"
    );

    let stderr = String::from_utf8_lossy(&result.stderr);
    let stdout = String::from_utf8_lossy(&result.stdout);
    let output = format!("{}\n{}", stderr, stdout);

    // Error message should be informative
    assert!(
        output.to_lowercase().contains("cannot") ||
        output.to_lowercase().contains("block itself") ||
        output.to_lowercase().contains("self-blocking"),
        "Error message should mention self-blocking prevention. Got: {}",
        output
    );
}

#[test]
fn test_batch_dep_add_blocker_allows_different_beads() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path();

    // Initialize workspace
    let init_result = Command::new("cargo")
        .args(["run", "--", "init", "--prefix", "test"])
        .current_dir(workspace_dir)
        .output()
        .expect("Failed to run bf init");

    assert!(
        init_result.status.success(),
        "bf init failed: {}",
        String::from_utf8_lossy(&init_result.stderr)
    );

    // Create first bead
    let bead1_result = Command::new("cargo")
        .args([
            "run", "--",
            "create",
            "--title", "Blocker Bead",
            "--type", "task",
            "--priority", "2",
        ])
        .current_dir(workspace_dir)
        .output()
        .expect("Failed to run bf create");

    assert!(bead1_result.status.success());
    let bead1_id = String::from_utf8(bead1_result.stdout)
        .expect("Invalid UTF-8")
        .trim()
        .to_string();

    // Create second bead
    let bead2_result = Command::new("cargo")
        .args([
            "run", "--",
            "create",
            "--title", "Dependent Bead",
            "--type", "task",
            "--priority", "2",
        ])
        .current_dir(workspace_dir)
        .output()
        .expect("Failed to run bf create");

    assert!(bead2_result.status.success());
    let bead2_id = String::from_utf8(bead2_result.stdout)
        .expect("Invalid UTF-8")
        .trim()
        .to_string();

    // Create batch operation with valid dependency (different beads)
    let batch_json = serde_json::json!([{
        "op": "dep_add_blocker",
        "id": bead2_id,
        "blocker": bead1_id
    }]);

    // Write batch to temp file
    let batch_file = temp_dir.path().join("batch.json");
    std::fs::write(&batch_file, batch_json.to_string())
        .expect("Failed to write batch file");

    // Run batch operation
    let result = Command::new("cargo")
        .args([
            "run", "--",
            "batch",
            "--file", batch_file.to_str().unwrap(),
        ])
        .current_dir(workspace_dir)
        .output()
        .expect("Failed to run bf batch");

    // Should succeed
    assert!(
        result.status.success(),
        "bf batch should succeed for different beads. Output: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn test_error_message_quality() {
    let (_temp_dir, bead_id) = setup_workspace_with_bead();

    // Test bf dep add error message
    let result = Command::new("cargo")
        .args([
            "run", "--",
            "dep", "add",
            &bead_id,
            "--blocks", &bead_id,
        ])
        .current_dir(_temp_dir.path())
        .output()
        .expect("Failed to run bf dep add");

    let stderr = String::from_utf8_lossy(&result.stderr);
    let stdout = String::from_utf8_lossy(&result.stdout);
    let output = format!("{}\n{}", stderr, stdout);

    // Error message should be clear and actionable
    assert!(
        output.len() > 20,
        "Error message should have reasonable length"
    );

    // Should mention the specific bead
    assert!(
        output.contains(&bead_id) || output.contains("itself"),
        "Error message should reference the problematic bead"
    );

    // Should contain "block" and some form of negation
    let output_lower = output.to_lowercase();
    assert!(
        output_lower.contains("block") &&
        (output_lower.contains("cannot") || output_lower.contains("not allowed") || output_lower.contains("self")),
        "Error message should mention blocking and why it's not allowed"
    );
}
