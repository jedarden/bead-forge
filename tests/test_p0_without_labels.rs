// Test P0 priority beads work correctly without labels
// This test verifies that P0 (Critical priority) beads do not require labels
// and function correctly in all operations.

use std::process::Command;

fn bf(args: &[&str]) -> String {
    let output = Command::new("bf")
        .args(args)
        .current_dir(".")
        .output()
        .expect("Failed to execute bf command");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        eprintln!("bf command failed: {:?}", args);
        eprintln!("stdout: {}", stdout);
        eprintln!("stderr: {}", stderr);
        panic!("bf command exited with non-zero status");
    }

    stdout
}

fn bf_json(args: &[&str]) -> serde_json::Value {
    let output = Command::new("bf")
        .args(args)
        .current_dir(".")
        .output()
        .expect("Failed to execute bf command");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        eprintln!("bf command failed: {:?}", args);
        eprintln!("stdout: {}", stdout);
        eprintln!("stderr: {}", stderr);
        panic!("bf command exited with non-zero status");
    }

    serde_json::from_str(&stdout).expect("Failed to parse JSON output")
}

#[test]
fn test_p0_without_labels() {
    // Test 1: Create a P0 bead without labels
    let create_output = bf(&[
        "create",
        "--title", "P0 Test Bead - No Labels",
        "--priority", "0",
        "--type", "task"
    ]);

    // bf create outputs just the bead ID
    let bead_id = create_output.trim().to_string();
    assert!(bead_id.starts_with("bf-"), "Expected bead ID to start with 'bf-', got: {}", bead_id);

    println!("Created P0 bead: {}", bead_id);

    // Test 2: Verify the bead has P0 priority via show
    let show_output = bf(&["show", &bead_id, "--format", "json"]);
    let bead_json: serde_json::Value = serde_json::from_str(&show_output)
        .expect("Failed to parse bead JSON");

    assert_eq!(bead_json[0]["priority"], 0);
    // When there are no labels, the field is omitted entirely
    assert!(bead_json[0]["labels"].is_null() || bead_json[0]["labels"].as_array().map_or(true, |arr| arr.is_empty()));

    // Test 3: Verify the bead appears in list queries with P0 priority
    let list_output = bf(&["list", "--priority", "0", "--format", "json"]);
    let list_beads: Vec<serde_json::Value> = list_output
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();

    assert!(list_beads.iter().any(|bead| bead["id"] == bead_id));

    // Test 4: Update the bead without adding labels
    bf(&[
        "update",
        &bead_id,
        "--status", "in_progress"
    ]);

    // Verify update worked
    let updated_output = bf(&["show", &bead_id, "--format", "json"]);
    let updated_bead: serde_json::Value = serde_json::from_str(&updated_output)
        .expect("Failed to parse updated bead JSON");

    assert_eq!(updated_bead[0]["status"], "in_progress");
    assert_eq!(updated_bead[0]["priority"], 0);
    assert!(updated_bead[0]["labels"].is_null() || updated_bead[0]["labels"].as_array().map_or(true, |arr| arr.is_empty()));

    // Test 5: Close the bead without labels
    bf(&[
        "close",
        &bead_id,
        "--reason", "Test complete - P0 works without labels"
    ]);

    // Verify close worked
    let closed_output = bf(&["show", &bead_id, "--format", "json"]);
    let closed_bead: serde_json::Value = serde_json::from_str(&closed_output)
        .expect("Failed to parse closed bead JSON");

    assert_eq!(closed_bead[0]["status"], "closed");
    assert_eq!(closed_bead[0]["priority"], 0);
    assert!(closed_bead[0]["labels"].is_null() || closed_bead[0]["labels"].as_array().map_or(true, |arr| arr.is_empty()));

    println!("✓ P0 bead works correctly without labels");
}

#[test]
fn test_multiple_p0_without_labels() {
    // Test creating multiple P0 beads without labels
    let bead_ids = vec![
        create_p0_bead("First P0 task"),
        create_p0_bead("Second P0 task"),
        create_p0_bead("Third P0 task"),
    ];

    // Verify all P0 beads appear in list
    let list_output = bf(&["list", "--priority", "0", "--format", "json"]);
    let list_beads: Vec<serde_json::Value> = list_output
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();

    for bead_id in &bead_ids {
        assert!(list_beads.iter().any(|bead| bead["id"] == *bead_id));
    }

    // Verify all have no labels
    for bead_id in &bead_ids {
        let show_output = bf(&["show", bead_id, "--format", "json"]);
        let bead_json: serde_json::Value = serde_json::from_str(&show_output)
            .expect("Failed to parse bead JSON");
        assert!(bead_json[0]["labels"].is_null() || bead_json[0]["labels"].as_array().map_or(true, |arr| arr.is_empty()));
    }

    println!("✓ Multiple P0 beads work correctly without labels");
}

#[test]
fn test_p0_ready_without_labels() {
    // Test that P0 beads without labels appear in ready queue
    let bead_id = create_p0_bead("Ready queue test");

    // Verify it appears in ready list (use higher limit in case there are many ready beads)
    let ready_output = bf(&["ready", "--limit", "100", "--format", "json"]);
    let ready_beads: Vec<serde_json::Value> = if ready_output.trim().is_empty() || ready_output.trim() == "[]" {
        vec![]
    } else {
        // bf ready outputs JSONL (one JSON object per line)
        ready_output
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    };

    assert!(ready_beads.iter().any(|bead| bead["id"] == bead_id), "Bead {} not found in ready list", bead_id);
}

fn create_p0_bead(title: &str) -> String {
    let create_output = bf(&[
        "create",
        "--title", title,
        "--priority", "0",
        "--type", "task"
    ]);

    // bf create outputs just the bead ID
    create_output.trim().to_string()
}
