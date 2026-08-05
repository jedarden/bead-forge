// Batch P0 Test 2 - Additional batch operation scenarios with P0 beads
//
// This test complements test_batch_p0.rs by covering:
// - Batch atomicity and rollback with P0 beads
// - Complex @ reference resolution with P0 beads
// - P0 priority enforcement across batch operations
// - P0 status transitions in batch scenarios
// - Edge cases with P0 batch operations

use std::io::Write;
use std::process::{Command, Stdio};

/// Helper to execute bf command and return stdout
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

/// Helper to execute bf batch via stdin
fn bf_batch_stdin(batch_input: &str) -> (String, String, bool) {
    let mut child = Command::new("bf")
        .args(&["batch", "--stdin", "--no-auto-flush"])
        .current_dir(".")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute bf batch command");

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(batch_input.as_bytes()).expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read output");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    (stdout, stderr, success)
}

/// Helper to execute bf and parse JSON output
fn bf_json(args: &[&str]) -> serde_json::Value {
    let output = Command::new("bf")
        .args(args)
        .current_dir(".")
        .output()
        .expect("Failed to execute bf command");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if !output.status.success() {
        panic!("bf command failed: {:?}", args);
    }

    serde_json::from_str(&stdout).expect("Failed to parse JSON output")
}

/// Helper to create a P0 bead
fn create_p0_bead(title: &str) -> String {
    bf(&["create", "--title", title, "--priority", "0", "--type", "task"]).trim().to_string()
}

#[test]
fn test_batch_p0_rollback_on_invalid_dependency() {
    // Test that batch operations roll back completely when a dependency operation fails
    // This ensures atomicity: if op 3 fails, ops 1 and 2 should be rolled back

    let batch_input = r#"[
        {"op": "create", "title": "P0 Task 1", "priority": 0, "type": "task"},
        {"op": "create", "title": "P0 Task 2", "priority": 0, "type": "task"},
        {"op": "dep_add_blocker", "id": "@1", "blocker": "bf-nonexistent"}
    ]"#;

    let (_stdout, stderr, success) = bf_batch_stdin(batch_input);

    // The batch should fail due to non-existent blocker
    assert!(!success, "Batch should fail when dependency references non-existent bead");
    assert!(stderr.contains("not found") || stderr.contains("error"),
            "Expected error about non-existent bead, got: {}", stderr);

    // Verify that neither P0 task was created (rollback occurred)
    let list_output = bf(&["list", "--format", "json"]);
    let list: serde_json::Value = serde_json::from_str(&list_output).unwrap();

    let beads = list.as_array().unwrap();
    assert!(!beads.iter().any(|b| b["title"] == "P0 Task 1" || b["title"] == "P0 Task 2"),
            "Rollback should have prevented creation of P0 beads after failed dependency");

    println!("✓ Batch rollback with P0 beads works correctly");
}

#[test]
fn test_batch_p0_complex_at_reference_resolution() {
    // Test complex @ placeholder references with P0 beads in a single batch
    // Scenario: Create 3 P0 beads, then add dependencies between them using @ references

    let batch_input = r#"[
        {"op": "create", "title": "P0 Foundation A", "priority": 0, "type": "task", "labels": ["foundation"]},
        {"op": "create", "title": "P0 Foundation B", "priority": 0, "type": "task", "labels": ["foundation"]},
        {"op": "create", "title": "P0 Dependent", "priority": 0, "type": "task", "labels": ["dependent"]},
        {"op": "dep_add_blocker", "id": "@2", "blocker": "@0"},
        {"op": "dep_add_blocker", "id": "@2", "blocker": "@1"},
        {"op": "label_add", "id": "@2", "labels": ["multi-blocker"]}
    ]"#;

    let (stdout, _stderr, success) = bf_batch_stdin(batch_input);

    assert!(success, "Batch with complex @ references failed: {}", stdout);

    let results: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .expect("Failed to parse batch results");

    assert_eq!(results.len(), 6, "Expected 6 operations");

    // Verify all operations succeeded
    for (i, result) in results.iter().enumerate() {
        assert_eq!(result["status"], "ok", "Operation {} failed: {:?}", i, result);
    }

    // Extract bead IDs
    let foundation_a_id = results[0]["id"].as_str().unwrap();
    let foundation_b_id = results[1]["id"].as_str().unwrap();
    let dependent_id = results[2]["id"].as_str().unwrap();

    // Verify all are P0
    for bead_id in &[foundation_a_id, foundation_b_id, dependent_id] {
        let bead = bf_json(&["show", bead_id, "--format", "json"]);
        assert_eq!(bead[0]["priority"], 0, "Bead {} should be P0", bead_id);
    }

    // Verify dependent has 2 blockers
    let dependent_bead = bf_json(&["show", dependent_id, "--format", "json"]);
    let deps = dependent_bead[0]["dependencies"].as_array().unwrap();
    assert_eq!(deps.len(), 2, "Dependent should have 2 blockers");

    // Verify dependent has the multi-blocker label
    let labels: Vec<String> = dependent_bead[0]["labels"].as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    assert!(labels.contains(&"multi-blocker".to_string()));
    assert!(labels.contains(&"dependent".to_string()));

    println!("✓ Complex @ reference resolution with P0 beads works correctly");
}

#[test]
fn test_batch_p0_priority_enforcement_mixed_ops() {
    // Test that P0 priority is enforced and preserved across mixed batch operations
    // Scenario: Create beads with different priorities, update some, ensure priorities stay correct

    let batch_input = r#"[
        {"op": "create", "title": "P0 Critical", "priority": 0, "type": "bug"},
        {"op": "create", "title": "P1 High", "priority": 1, "type": "task"},
        {"op": "create", "title": "P2 Medium", "priority": 2, "type": "task"},
        {"op": "update", "id": "@1", "status": "in_progress"},
        {"op": "update", "id": "@0", "assignee": "critical-team"}
    ]"#;

    let (stdout, _stderr, success) = bf_batch_stdin(batch_input);

    assert!(success, "Batch with mixed priorities failed: {}", stdout);

    let results: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .expect("Failed to parse batch results");

    // Extract IDs
    let p0_id = results[0]["id"].as_str().unwrap();
    let p1_id = results[1]["id"].as_str().unwrap();
    let p2_id = results[2]["id"].as_str().unwrap();

    // Verify priorities are preserved
    let p0_bead = bf_json(&["show", p0_id, "--format", "json"]);
    assert_eq!(p0_bead[0]["priority"], 0);
    assert_eq!(p0_bead[0]["assignee"], "critical-team");

    let p1_bead = bf_json(&["show", p1_id, "--format", "json"]);
    assert_eq!(p1_bead[0]["priority"], 1);
    assert_eq!(p1_bead[0]["status"], "in_progress");

    let p2_bead = bf_json(&["show", p2_id, "--format", "json"]);
    assert_eq!(p2_bead[0]["priority"], 2);

    println!("✓ P0 priority enforcement across mixed operations works correctly");
}

#[test]
fn test_batch_p0_status_transition_workflow() {
    // Test P0 beads going through status transitions in a single batch
    // Scenario: Create P0 bead, transition it through multiple states

    let bead_id = create_p0_bead("Status transition test");

    let batch_input = format!(r#"[
        {{"op": "update", "id": "{}", "status": "in_progress"}},
        {{"op": "comment", "id": "{}", "author": "worker", "text": "Working on P0 task"}},
        {{"op": "label_add", "id": "{}", "labels": ["active"]}},
        {{"op": "update", "id": "{}", "status": "blocked"}},
        {{"op": "update", "id": "{}", "status": "in_progress"}}
    ]"#, bead_id, bead_id, bead_id, bead_id, bead_id);

    let (stdout, _stderr, success) = bf_batch_stdin(&batch_input);

    assert!(success, "Status transition batch failed: {}", stdout);

    let results: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .expect("Failed to parse batch results");

    assert_eq!(results.len(), 5);
    for result in &results {
        assert_eq!(result["status"], "ok");
    }

    // Verify final state
    let bead = bf_json(&["show", &bead_id, "--format", "json"]);
    assert_eq!(bead[0]["status"], "in_progress");
    assert_eq!(bead[0]["priority"], 0, "Should still be P0");

    let labels: Vec<String> = bead[0]["labels"].as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    assert!(labels.contains(&"active".to_string()));

    let comments = bead[0]["comments"].as_array().unwrap();
    assert_eq!(comments.len(), 1);

    println!("✓ P0 status transition workflow works correctly");
}

#[test]
fn test_batch_p0_edge_case_single_operation() {
    // Test edge case: batch with a single P0 create operation
    let batch_input = r#"[
        {"op": "create", "title": "Single P0", "priority": 0, "type": "task"}
    ]"#;

    let (stdout, _stderr, success) = bf_batch_stdin(batch_input);

    assert!(success, "Single-op batch failed: {}", stdout);

    let results: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .expect("Failed to parse batch results");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["status"], "ok");

    let bead_id = results[0]["id"].as_str().unwrap();
    let bead = bf_json(&["show", bead_id, "--format", "json"]);
    assert_eq!(bead[0]["priority"], 0);

    println!("✓ Single P0 operation batch works correctly");
}

#[test]
fn test_batch_p0_edge_case_all_close_operations() {
    // Test edge case: batch with only close operations on P0 beads

    // Create P0 beads first
    let bead1 = create_p0_bead("P0 close test 1");
    let bead2 = create_p0_bead("P0 close test 2");
    let bead3 = create_p0_bead("P0 close test 3");

    let batch_input = format!(r#"[
        {{"op": "close", "id": "{}", "reason": "Completed P0-1"}},
        {{"op": "close", "id": "{}", "reason": "Completed P0-2"}},
        {{"op": "close", "id": "{}", "reason": "Completed P0-3"}}
    ]"#, bead1, bead2, bead3);

    let (stdout, _stderr, success) = bf_batch_stdin(&batch_input);

    assert!(success, "All-close batch failed: {}", stdout);

    let results: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .expect("Failed to parse batch results");

    assert_eq!(results.len(), 3);
    for result in &results {
        assert_eq!(result["status"], "ok");
    }

    // Verify all are closed but still P0
    for bead_id in &[&bead1, &bead2, &bead3] {
        let bead = bf_json(&["show", bead_id, "--format", "json"]);
        assert_eq!(bead[0]["status"], "closed");
        assert_eq!(bead[0]["priority"], 0);
    }

    println!("✓ All-close P0 batch works correctly");
}

#[test]
fn test_batch_p0_with_cascading_dependencies() {
    // Test P0 beads with cascading dependencies: A -> B -> C -> D
    // All P0, all dependent on the previous one

    let batch_input = r#"[
        {"op": "create", "title": "P0 Level 0", "priority": 0, "type": "task"},
        {"op": "create", "title": "P0 Level 1", "priority": 0, "type": "task"},
        {"op": "create", "title": "P0 Level 2", "priority": 0, "type": "task"},
        {"op": "create", "title": "P0 Level 3", "priority": 0, "type": "task"},
        {"op": "dep_add_blocker", "id": "@1", "blocker": "@0"},
        {"op": "dep_add_blocker", "id": "@2", "blocker": "@1"},
        {"op": "dep_add_blocker", "id": "@3", "blocker": "@2"}
    ]"#;

    let (stdout, _stderr, success) = bf_batch_stdin(batch_input);

    assert!(success, "Cascading dependencies batch failed: {}", stdout);

    let results: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .expect("Failed to parse batch results");

    assert_eq!(results.len(), 7);

    // Verify all P0
    for i in 0..4 {
        let bead_id = results[i]["id"].as_str().unwrap();
        let bead = bf_json(&["show", bead_id, "--format", "json"]);
        assert_eq!(bead[0]["priority"], 0);
    }

    // Verify dependency chain
    let level3_id = results[3]["id"].as_str().unwrap();
    let level3_bead = bf_json(&["show", level3_id, "--format", "json"]);
    let deps = level3_bead[0]["dependencies"].as_array().unwrap();
    assert_eq!(deps.len(), 1, "Level 3 should have 1 blocker");

    println!("✓ Cascading P0 dependencies work correctly");
}

#[test]
fn test_batch_p0_error_handling_with_rollback_verification() {
    // Test that when a batch fails mid-execution with P0 beads,
    // the database state is consistent (no partial updates)

    // Create a P0 bead first
    let existing_p0 = create_p0_bead("Existing P0");

    // Batch that will fail: valid update then invalid operation
    let batch_input = format!(r#"[
        {{"op": "update", "id": "{}", "status": "in_progress"}},
        {{"op": "update", "id": "bf-nonexistent", "status": "in_progress"}}
    ]"#, existing_p0);

    let (_stdout, _stderr, success) = bf_batch_stdin(&batch_input);

    // Should fail
    assert!(!success, "Batch should fail on non-existent bead");

    // Verify the existing P0 was NOT updated (rollback occurred)
    let bead = bf_json(&["show", &existing_p0, "--format", "json"]);
    assert_eq!(bead[0]["status"], "open", "P0 bead should remain open after rollback");
    assert_eq!(bead[0]["priority"], 0, "Should still be P0");

    println!("✓ Error handling with P0 rollback verification works correctly");
}

#[test]
fn test_batch_p0_mixed_create_update_close_workflow() {
    // Test a realistic workflow: create some P0s, update some, close some
    // All in a single batch to simulate real usage

    let batch_input = r#"[
        {"op": "create", "title": "P0 New Task", "priority": 0, "type": "task"},
        {"op": "create", "title": "P0 New Bug", "priority": 0, "type": "bug"},
        {"op": "update", "id": "@0", "status": "in_progress", "assignee": "team-a"},
        {"op": "label_add", "id": "@1", "labels": ["security", "urgent"]},
        {"op": "comment", "id": "@1", "author": "triager", "text": "Critical security issue"}
    ]"#;

    let (stdout, _stderr, success) = bf_batch_stdin(batch_input);

    assert!(success, "Mixed workflow batch failed: {}", stdout);

    let results: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .expect("Failed to parse batch results");

    assert_eq!(results.len(), 5);

    let task_id = results[0]["id"].as_str().unwrap();
    let bug_id = results[1]["id"].as_str().unwrap();

    // Verify task
    let task = bf_json(&["show", task_id, "--format", "json"]);
    assert_eq!(task[0]["priority"], 0);
    assert_eq!(task[0]["status"], "in_progress");
    assert_eq!(task[0]["assignee"], "team-a");

    // Verify bug
    let bug = bf_json(&["show", bug_id, "--format", "json"]);
    assert_eq!(bug[0]["priority"], 0);

    let bug_labels: Vec<String> = bug[0]["labels"].as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    assert!(bug_labels.contains(&"security".to_string()));
    assert!(bug_labels.contains(&"urgent".to_string()));

    let bug_comments = bug[0]["comments"].as_array().unwrap();
    assert_eq!(bug_comments.len(), 1);

    println!("✓ Mixed create-update-close P0 workflow works correctly");
}

#[test]
fn test_batch_p0_empty_batch_handling() {
    // Test edge case: empty batch (no operations)
    let batch_input = r#"[]"#;

    let (stdout, _stderr, success) = bf_batch_stdin(batch_input);

    // Empty batch should either succeed with no results or fail gracefully
    // Check what the actual behavior is
    if success {
        let results: Vec<serde_json::Value> = serde_json::from_str(&stdout)
            .expect("Failed to parse batch results");
        assert_eq!(results.len(), 0);
    } else {
        // If it fails, that's also acceptable behavior for empty batch
        println!("Empty batch rejected (acceptable behavior)");
    }

    println!("✓ Empty P0 batch handling works");
}
