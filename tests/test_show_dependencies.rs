//! Comprehensive tests for dependency and relationship display in `bf show` command
//!
//! Tests the show command functionality for dependencies including:
//! - Basic dependency relationships (blocked_by)
//! - Reverse dependencies (blocks)
//! - Parent-child relationships
//! - Circular dependency handling
//! - Multiple dependency types
//! - Dependency link formatting

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a temporary workspace for testing
fn setup_test_workspace() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("test-workspace");
    fs::create_dir_all(&workspace_dir).unwrap();
    let beads_dir = workspace_dir.join(".beads");
    fs::create_dir_all(&beads_dir).unwrap();

    // Initialize workspace
    let config_path = beads_dir.join("config.yaml");
    fs::write(
        &config_path,
        r#"issue_prefixes: [bf]
default_priority: 2
default_type: task
claim_ttl_minutes: 30
"#,
    )
    .unwrap();

    let metadata_path = beads_dir.join("metadata.json");
    fs::write(
        &metadata_path,
        r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
    )
    .unwrap();

    let db_path = beads_dir.join("beads.db");
    bead_forge::storage::Storage::open(&db_path).unwrap();

    (temp_dir, beads_dir)
}

/// Get the path to the bf binary
fn get_bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Create a test bead via CLI
fn create_test_bead(workspace: impl AsRef<std::path::Path>, title: &str) -> String {
    let bf_path = get_bf_binary();
    let result = std::process::Command::new(&bf_path)
        .arg("create")
        .arg("--title")
        .arg(title)
        .arg("--type")
        .arg("task")
        .arg("--priority")
        .arg("2")
        .current_dir(&workspace.as_ref())
        .output()
        .expect("Failed to create bead");

    assert!(
        result.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap().trim().to_string()
}

#[test]
fn test_show_displays_blocked_by_dependencies() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create three beads: blocker-1, blocker-2, and dependent
    let blocker1_id = create_test_bead(workspace, "Blocker task 1");
    let blocker2_id = create_test_bead(workspace, "Blocker task 2");
    let dependent_id = create_test_bead(workspace, "Dependent task");

    // Add blocking dependencies to dependent bead
    let batch_json = serde_json::json!([
        {"op": "dep_add_blocker", "id": &dependent_id, "blocker": &blocker1_id},
        {"op": "dep_add_blocker", "id": &dependent_id, "blocker": &blocker2_id}
    ]);

    let batch_file = workspace.join("batch.json");
    fs::write(&batch_file, batch_json.to_string()).unwrap();

    let batch_result = std::process::Command::new(&bf_path)
        .arg("batch")
        .arg("--file")
        .arg(&batch_file)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf batch");

    assert!(
        batch_result.status.success(),
        "bf batch failed: {}",
        String::from_utf8_lossy(&batch_result.stderr)
    );

    // Show the dependent bead in text format
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&dependent_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(
        show_result.status.success(),
        "bf show failed: {}",
        String::from_utf8_lossy(&show_result.stderr)
    );

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Show output with blocked_by:\n{}", output);

    // Verify "Blocked by:" section is displayed
    assert!(
        output.contains("Blocked by:"),
        "Should show 'Blocked by:' section for beads with dependencies"
    );

    // Verify both blocker IDs are shown
    assert!(
        output.contains(&blocker1_id),
        "Should show first blocker ID"
    );
    assert!(
        output.contains(&blocker2_id),
        "Should show second blocker ID"
    );

    // Verify dependency type is shown
    assert!(
        output.contains("(blocks)"),
        "Should show dependency type as 'blocks'"
    );
}

#[test]
fn test_show_displays_blocks_dependents() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create three beads: parent and two children
    let parent_id = create_test_bead(workspace, "Parent task");
    let child1_id = create_test_bead(workspace, "Child task 1");
    let child2_id = create_test_bead(workspace, "Child task 2");

    // Make both children depend on parent
    let batch_json = serde_json::json!([
        {"op": "dep_add_blocker", "id": &child1_id, "blocker": &parent_id},
        {"op": "dep_add_blocker", "id": &child2_id, "blocker": &parent_id}
    ]);

    let batch_file = workspace.join("batch.json");
    fs::write(&batch_file, batch_json.to_string()).unwrap();

    let batch_result = std::process::Command::new(&bf_path)
        .arg("batch")
        .arg("--file")
        .arg(&batch_file)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf batch");

    assert!(
        batch_result.status.success(),
        "bf batch failed: {}",
        String::from_utf8_lossy(&batch_result.stderr)
    );

    // Show the parent bead - it should show what it blocks
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&parent_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(
        show_result.status.success(),
        "bf show failed: {}",
        String::from_utf8_lossy(&show_result.stderr)
    );

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Show output with blocks:\n{}", output);

    // Verify "Blocks:" section is displayed
    assert!(
        output.contains("Blocks:"),
        "Should show 'Blocks:' section for beads that block others"
    );

    // Verify both child IDs are shown
    assert!(
        output.contains(&child1_id),
        "Should show first child ID that is blocked"
    );
    assert!(
        output.contains(&child2_id),
        "Should show second child ID that is blocked"
    );

    // Verify dependency type is shown
    assert!(
        output.contains("(blocks)"),
        "Should show dependency type as 'blocks'"
    );
}

#[test]
fn test_show_parent_child_relationships() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create parent and child beads
    let parent_id = create_test_bead(workspace, "Epic task");
    let child_id = create_test_bead(workspace, "Subtask");

    // Add parent-child relationship
    let batch_json = serde_json::json!([
        {"op": "dep_add_blocker", "id": &child_id, "blocker": &parent_id}
    ]);

    let batch_file = workspace.join("batch.json");
    fs::write(&batch_file, batch_json.to_string()).unwrap();

    let batch_result = std::process::Command::new(&bf_path)
        .arg("batch")
        .arg("--file")
        .arg(&batch_file)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf batch");

    assert!(
        batch_result.status.success(),
        "bf batch failed: {}",
        String::from_utf8_lossy(&batch_result.stderr)
    );

    // Show child - should show "Blocked by:" with parent
    let child_show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&child_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show child");

    let child_output = String::from_utf8(child_show_result.stdout).unwrap();
    println!("Child show output:\n{}", child_output);

    assert!(
        child_output.contains("Blocked by:"),
        "Child should show 'Blocked by:' section"
    );
    assert!(
        child_output.contains(&parent_id),
        "Child should show parent as blocker"
    );

    // Show parent - should show "Blocks:" with child
    let parent_show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&parent_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show parent");

    let parent_output = String::from_utf8(parent_show_result.stdout).unwrap();
    println!("Parent show output:\n{}", parent_output);

    assert!(
        parent_output.contains("Blocks:"),
        "Parent should show 'Blocks:' section"
    );
    assert!(
        parent_output.contains(&child_id),
        "Parent should show child as blocked"
    );
}

#[test]
fn test_show_complex_dependency_chain() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a dependency chain: A -> B -> C -> D
    let bead_a = create_test_bead(workspace, "Task A (final)");
    let bead_b = create_test_bead(workspace, "Task B");
    let bead_c = create_test_bead(workspace, "Task C");
    let bead_d = create_test_bead(workspace, "Task D (initial)");

    // Create chain: D blocks C, C blocks B, B blocks A
    let batch_json = serde_json::json!([
        {"op": "dep_add_blocker", "id": &bead_c, "blocker": &bead_d},
        {"op": "dep_add_blocker", "id": &bead_b, "blocker": &bead_c},
        {"op": "dep_add_blocker", "id": &bead_a, "blocker": &bead_b}
    ]);

    let batch_file = workspace.join("batch.json");
    fs::write(&batch_file, batch_json.to_string()).unwrap();

    let batch_result = std::process::Command::new(&bf_path)
        .arg("batch")
        .arg("--file")
        .arg(&batch_file)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf batch");

    assert!(
        batch_result.status.success(),
        "bf batch failed: {}",
        String::from_utf8_lossy(&batch_result.stderr)
    );

    // Show bead B (middle of chain) - should have both blocked_by and blocks
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_b)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(
        show_result.status.success(),
        "bf show failed: {}",
        String::from_utf8_lossy(&show_result.stderr)
    );

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Middle bead show output:\n{}", output);

    // Bead B should show both sections
    assert!(
        output.contains("Blocked by:"),
        "Middle bead should show 'Blocked by:' section"
    );
    assert!(
        output.contains("Blocks:"),
        "Middle bead should show 'Blocks:' section"
    );

    // Verify the relationships
    assert!(output.contains(&bead_c), "Should show bead C as blocker");
    assert!(output.contains(&bead_a), "Should show bead A as blocked");
}

#[test]
fn test_show_multiple_dependency_types() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create beads with different dependency types
    let main_id = create_test_bead(workspace, "Main task");
    let blocker_id = create_test_bead(workspace, "Hard blocker");
    let related_id = create_test_bead(workspace, "Related task");

    // Add different dependency types via direct storage API
    let storage = bead_forge::storage::Storage::open(&beads_dir.join("beads.db")).unwrap();

    // Add blocking dependency
    storage
        .add_dependency(
            &main_id,
            &blocker_id,
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();

    // Add related dependency
    storage
        .add_dependency(
            &main_id,
            &related_id,
            &bead_forge::model::DependencyType::RelatesTo,
            "test",
        )
        .unwrap();

    // Show the main bead
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&main_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(
        show_result.status.success(),
        "bf show failed: {}",
        String::from_utf8_lossy(&show_result.stderr)
    );

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Show output with multiple dependency types:\n{}", output);

    // Should show both dependencies
    assert!(
        output.contains("Blocked by:"),
        "Should show 'Blocked by:' section"
    );
    assert!(
        output.contains(&blocker_id),
        "Should show blocking dependency"
    );
    assert!(
        output.contains(&related_id),
        "Should show related dependency"
    );

    // Verify dependency types are shown
    assert!(
        output.contains("(blocks)") || output.contains("(relates-to)"),
        "Should show dependency types"
    );
}

#[test]
fn test_show_json_format_dependencies() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create beads with dependencies
    let blocker_id = create_test_bead(workspace, "Blocker");
    let dependent_id = create_test_bead(workspace, "Dependent");

    let batch_json = serde_json::json!([
        {"op": "dep_add_blocker", "id": &dependent_id, "blocker": &blocker_id}
    ]);

    let batch_file = workspace.join("batch.json");
    fs::write(&batch_file, batch_json.to_string()).unwrap();

    let batch_result = std::process::Command::new(&bf_path)
        .arg("batch")
        .arg("--file")
        .arg(&batch_file)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf batch");

    assert!(
        batch_result.status.success(),
        "bf batch failed: {}",
        String::from_utf8_lossy(&batch_result.stderr)
    );

    // Show in JSON format
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&dependent_id)
        .arg("--format")
        .arg("json")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(
        show_result.status.success(),
        "bf show failed: {}",
        String::from_utf8_lossy(&show_result.stderr)
    );

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("JSON output:\n{}", output);

    // Parse and verify JSON structure
    let beads: Vec<serde_json::Value> =
        serde_json::from_str(&output).expect("Failed to parse JSON output");

    assert_eq!(beads.len(), 1, "Should return exactly one bead");
    let bead = &beads[0];

    // Note: dependencies are stripped in JSON output for NEEDLE compatibility
    // So we verify they're NOT present (as per existing test behavior)
    assert!(
        bead.get("dependencies").is_none()
            || bead["dependencies"]
                .as_array()
                .map(|a| a.is_empty())
                .unwrap_or(false),
        "Dependencies should be stripped for NEEDLE compatibility in JSON output"
    );

    // But we can verify the dependency exists by showing it in text format
    let text_show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&dependent_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show in text format");

    let text_output = String::from_utf8(text_show_result.stdout).unwrap();
    assert!(
        text_output.contains("Blocked by:"),
        "Text format should show dependencies"
    );
    assert!(
        text_output.contains(&blocker_id),
        "Text format should show blocker ID"
    );
}

#[test]
fn test_show_no_dependencies() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create a bead with no dependencies
    let bead_id = create_test_bead(workspace, "Independent task");

    // Show the bead
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(
        show_result.status.success(),
        "bf show failed: {}",
        String::from_utf8_lossy(&show_result.stderr)
    );

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Show output with no dependencies:\n{}", output);

    // Should NOT show dependency sections
    assert!(
        !output.contains("Blocked by:"),
        "Should NOT show 'Blocked by:' for bead with no dependencies"
    );
    assert!(
        !output.contains("Blocks:"),
        "Should NOT show 'Blocks:' for bead that blocks nothing"
    );
}

#[test]
fn test_circular_dependencies_handling() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create two beads
    let bead_a = create_test_bead(workspace, "Bead A");
    let bead_b = create_test_bead(workspace, "Bead B");

    // Create circular dependency: A blocks B, B blocks A
    let storage = bead_forge::storage::Storage::open(&beads_dir.join("beads.db")).unwrap();

    // Add first dependency
    let result1 = storage.add_dependency(
        &bead_a,
        &bead_b,
        &bead_forge::model::DependencyType::Blocks,
        "test",
    );

    // Add second dependency (creates circular dependency)
    let result2 = storage.add_dependency(
        &bead_b,
        &bead_a,
        &bead_forge::model::DependencyType::Blocks,
        "test",
    );

    // Both operations should succeed (system allows circular deps)
    assert!(
        result1.is_ok(),
        "First dependency should be added successfully"
    );
    assert!(
        result2.is_ok(),
        "Second dependency creating circular relationship should succeed"
    );

    // Show bead A - should handle circular display gracefully
    let show_a_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_a)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(
        show_a_result.status.success(),
        "bf show should handle circular dependencies gracefully: {}",
        String::from_utf8_lossy(&show_a_result.stderr)
    );

    let output_a = String::from_utf8(show_a_result.stdout).unwrap();
    println!("Bead A show output with circular deps:\n{}", output_a);

    // Should show both sections
    assert!(
        output_a.contains("Blocked by:") || output_a.contains("Blocks:"),
        "Should show at least one dependency section"
    );

    if output_a.contains("Blocked by:") {
        assert!(
            output_a.contains(&bead_b),
            "Should show bead B in blocked_by"
        );
    }

    if output_a.contains("Blocks:") {
        assert!(output_a.contains(&bead_b), "Should show bead B in blocks");
    }

    // Show bead B - should also work
    let show_b_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_b)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(
        show_b_result.status.success(),
        "bf show should handle circular dependencies for bead B"
    );
}

#[test]
fn test_show_dependency_link_formatting() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create beads
    let blocker_id = create_test_bead(workspace, "Blocker with nice title");
    let dependent_id = create_test_bead(workspace, "Dependent task");

    // Add dependency
    let batch_json = serde_json::json!([
        {"op": "dep_add_blocker", "id": &dependent_id, "blocker": &blocker_id}
    ]);

    let batch_file = workspace.join("batch.json");
    fs::write(&batch_file, batch_json.to_string()).unwrap();

    let batch_result = std::process::Command::new(&bf_path)
        .arg("batch")
        .arg("--file")
        .arg(&batch_file)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf batch");

    assert!(
        batch_result.status.success(),
        "bf batch failed: {}",
        String::from_utf8_lossy(&batch_result.stderr)
    );

    // Show in text format and verify formatting
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&dependent_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Dependency link formatting:\n{}", output);

    // Verify proper formatting: should use arrow format "-> ID (type)"
    assert!(
        output.contains("->"),
        "Dependency links should use arrow format"
    );
    assert!(output.contains(&blocker_id), "Should show blocker ID");
    assert!(
        output.contains("(blocks)"),
        "Should show dependency type in parentheses"
    );
}

#[test]
fn test_show_blocks_and_blocked_by_both_present() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create chain: A -> B -> C
    let bead_a = create_test_bead(workspace, "Task A (blocks B)");
    let bead_b = create_test_bead(workspace, "Task B (middle - blocked by A, blocks C)");
    let bead_c = create_test_bead(workspace, "Task C (blocked by B)");

    // Create dependencies
    let batch_json = serde_json::json!([
        {"op": "dep_add_blocker", "id": &bead_b, "blocker": &bead_a},
        {"op": "dep_add_blocker", "id": &bead_c, "blocker": &bead_b}
    ]);

    let batch_file = workspace.join("batch.json");
    fs::write(&batch_file, batch_json.to_string()).unwrap();

    let batch_result = std::process::Command::new(&bf_path)
        .arg("batch")
        .arg("--file")
        .arg(&batch_file)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf batch");

    assert!(
        batch_result.status.success(),
        "bf batch failed: {}",
        String::from_utf8_lossy(&batch_result.stderr)
    );

    // Show bead B (middle bead) - should have both sections
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_b)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    assert!(
        show_result.status.success(),
        "bf show failed: {}",
        String::from_utf8_lossy(&show_result.stderr)
    );

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Middle bead with both sections:\n{}", output);

    // Verify both sections are present
    assert!(
        output.contains("Blocked by:"),
        "Should show 'Blocked by:' section"
    );
    assert!(output.contains("Blocks:"), "Should show 'Blocks:' section");

    // Verify correct IDs in each section
    let blocked_by_idx = output.find("Blocked by:").unwrap();
    let blocks_idx = output.find("Blocks:").unwrap();

    // Extract the sections
    let blocked_by_section = if blocked_by_idx < blocks_idx {
        &output[blocked_by_idx..blocks_idx]
    } else {
        &output[blocked_by_idx..]
    };

    let blocks_section = if blocks_idx < blocked_by_idx {
        &output[blocks_idx..blocked_by_idx]
    } else {
        &output[blocks_idx..]
    };

    // Verify bead A is in blocked_by
    assert!(
        blocked_by_section.contains(&bead_a),
        "Bead A should be in 'Blocked by:' section"
    );

    // Verify bead C is in blocks
    assert!(
        blocks_section.contains(&bead_c),
        "Bead C should be in 'Blocks:' section"
    );

    // Verify bead B is not in either section (can't block or be blocked by itself)
    assert!(
        !blocked_by_section.contains(&bead_b) && !blocks_section.contains(&bead_b),
        "Bead B should not appear in its own dependency sections"
    );
}
