// Integration test for bf update flags
// This test verifies that all update flags work correctly

#[test]
fn test_update_flags() {
    // Create a temporary workspace for testing
    let temp_dir = tempfile::tempdir().unwrap();
    let workspace = temp_dir.path();
    let _beads_dir = workspace.join(".beads");

    // Initialize workspace
    let bf_path =
        std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string());
    let init_result = std::process::Command::new(&bf_path)
        .arg("init")
        .arg("--prefix")
        .arg("test")
        .current_dir(workspace)
        .output()
        .expect("Failed to initialize workspace");

    assert!(init_result.status.success(), "bf init failed");

    // Create a test bead
    let create_result = std::process::Command::new(&bf_path)
        .arg("create")
        .arg("--title")
        .arg("Test Update Flags")
        .arg("--type")
        .arg("task")
        .arg("--priority")
        .arg("2")
        .arg("--description")
        .arg("Original description")
        .arg("--assignee")
        .arg("test-user")
        .current_dir(workspace)
        .output()
        .expect("Failed to create bead");

    assert!(create_result.status.success(), "bf create failed");

    let bead_id = String::from_utf8(create_result.stdout)
        .unwrap()
        .trim()
        .to_string();
    println!("Created test bead: {}", bead_id);

    // Test 1: Update title
    let result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--title")
        .arg("Updated title")
        .current_dir(workspace)
        .output()
        .expect("Failed to update title");

    assert!(result.status.success(), "bf update --title failed");
    println!("✓ Update --title works");

    // Test 2: Update status
    let result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--status")
        .arg("in_progress")
        .current_dir(workspace)
        .output()
        .expect("Failed to update status");

    assert!(result.status.success(), "bf update --status failed");
    println!("✓ Update --status works");

    // Test 3: Update priority
    let result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--priority")
        .arg("0")
        .current_dir(workspace)
        .output()
        .expect("Failed to update priority");

    assert!(result.status.success(), "bf update --priority failed");
    println!("✓ Update --priority works");

    // Test 4: Update assignee
    let result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--assignee")
        .arg("new-user")
        .current_dir(workspace)
        .output()
        .expect("Failed to update assignee");

    assert!(result.status.success(), "bf update --assignee failed");
    println!("✓ Update --assignee works");

    // Test 5: Update description
    let result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--description")
        .arg("Updated description")
        .current_dir(workspace)
        .output()
        .expect("Failed to update description");

    assert!(result.status.success(), "bf update --description failed");
    println!("✓ Update --description works");

    // Test 6: Update acceptance criteria
    let result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--acceptance-criteria")
        .arg("Updated acceptance criteria")
        .current_dir(workspace)
        .output()
        .expect("Failed to update acceptance criteria");

    assert!(
        result.status.success(),
        "bf update --acceptance-criteria failed"
    );
    println!("✓ Update --acceptance-criteria works");

    // Test 7: Update notes
    let result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--notes")
        .arg("Updated notes")
        .current_dir(workspace)
        .output()
        .expect("Failed to update notes");

    assert!(result.status.success(), "bf update --notes failed");
    println!("✓ Update --notes works");

    // Test 8: Update design
    let result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--design")
        .arg("Updated design")
        .current_dir(workspace)
        .output()
        .expect("Failed to update design");

    assert!(result.status.success(), "bf update --design failed");
    println!("✓ Update --design works");

    // Test 9: Update due_at with RFC3339 format
    let result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--due-at")
        .arg("2025-12-31T23:59:59Z")
        .current_dir(workspace)
        .output()
        .expect("Failed to update due_at");

    assert!(result.status.success(), "bf update --due-at failed");
    println!("✓ Update --due-at works");

    // Test 10: Multiple updates at once
    let result = std::process::Command::new(&bf_path)
        .arg("update")
        .arg(&bead_id)
        .arg("--title")
        .arg("Final title")
        .arg("--status")
        .arg("open")
        .arg("--priority")
        .arg("1")
        .current_dir(workspace)
        .output()
        .expect("Failed to update multiple fields");

    assert!(
        result.status.success(),
        "bf update with multiple flags failed"
    );
    println!("✓ Update with multiple flags works");

    // Verify final state
    let show_result = std::process::Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .current_dir(workspace)
        .output()
        .expect("Failed to show bead");

    assert!(show_result.status.success(), "bf show failed");

    let output = String::from_utf8(show_result.stdout).unwrap();
    println!("Final bead state: {}", output);

    // Parse JSON and verify values
    let beads: Vec<serde_json::Value> =
        serde_json::from_str(&output).expect("Failed to parse JSON");
    let bead = &beads[0];

    assert_eq!(bead["title"], "Final title");
    assert_eq!(bead["status"], "open");
    assert_eq!(bead["priority"], 1);

    println!("✓ All update flags tested successfully!");
}
