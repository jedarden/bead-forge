// Test batch operations with P0 (critical priority) beads
// This test verifies that batch operations work correctly with P0 beads,
// including creation, updates, dependencies, and closing.

use std::io::Write;
use std::process::{Command, Stdio};

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

fn bf_batch_stdin(batch_input: &str) -> (String, String, bool) {
    let mut child = Command::new("bf")
        .args(&["batch", "--stdin", "--no-auto-flush"])
        .current_dir(".")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute bf batch command");

    // Write to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(batch_input.as_bytes()).expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read output");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    (stdout, stderr, success)
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

/// Parse bead IDs from batch output format: "[op 0] ok: bf-xxx"
fn parse_bead_ids_from_batch_output(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|line| {
            if let Some(rest) = line.strip_prefix("[op ") {
                if let Some(id_part) = rest.split("] ok: ").nth(1) {
                    Some(id_part.trim().to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn test_batch_create_multiple_p0_beads() {
    // Test creating multiple P0 beads in a single batch operation
    let batch_input = r#"[
        {"op": "create", "title": "First P0 critical task", "priority": 0, "type": "task"},
        {"op": "create", "title": "Second P0 critical task", "priority": 0, "type": "bug"},
        {"op": "create", "title": "Third P0 critical task", "priority": 0, "type": "task", "labels": ["urgent", "critical"]}
    ]"#;

    let (stdout, stderr, success) = bf_batch_stdin(batch_input);

    assert!(success, "Batch command failed: stdout={}, stderr={}", stdout, stderr);

    let bead_ids = parse_bead_ids_from_batch_output(&stdout);
    assert_eq!(bead_ids.len(), 3, "Expected 3 bead IDs from output");

    // Verify all beads have P0 priority
    for bead_id in &bead_ids {
        let bead = bf_json(&["show", bead_id, "--format", "json"]);
        assert_eq!(bead[0]["priority"], 0, "Bead {} should have P0 priority", bead_id);
    }

    println!("✓ Batch created 3 P0 beads successfully");
}

#[test]
fn test_batch_update_p0_beads() {
    // Create P0 beads first
    let bead1 = create_p0_bead("P0 update test 1");
    let bead2 = create_p0_bead("P0 update test 2");

    // Update both in a batch
    let batch_input = format!(r#"[
        {{"op": "update", "id": "{}", "status": "in_progress", "assignee": "worker-1"}},
        {{"op": "update", "id": "{}", "status": "in_progress", "priority": 0, "assignee": "worker-2"}}
    ]"#, bead1, bead2);

    let (stdout, stderr, success) = bf_batch_stdin(&batch_input);

    assert!(success, "Batch update failed: stdout={}, stderr={}", stdout, stderr);

    // Verify updates
    let bead1_updated = bf_json(&["show", &bead1, "--format", "json"]);
    assert_eq!(bead1_updated[0]["status"], "in_progress");
    assert_eq!(bead1_updated[0]["assignee"], "worker-1");
    assert_eq!(bead1_updated[0]["priority"], 0); // Still P0

    let bead2_updated = bf_json(&["show", &bead2, "--format", "json"]);
    assert_eq!(bead2_updated[0]["status"], "in_progress");
    assert_eq!(bead2_updated[0]["assignee"], "worker-2");
    assert_eq!(bead2_updated[0]["priority"], 0); // Still P0

    println!("✓ Batch updated 2 P0 beads successfully");
}

#[test]
fn test_batch_p0_with_dependencies() {
    // Test creating P0 beads with blocker dependencies in a batch
    let batch_input = r#"[
        {"op": "create", "title": "P0 Blocker task", "priority": 0, "type": "task"},
        {"op": "create", "title": "P0 Blocked task", "priority": 0, "type": "task"},
        {"op": "dep_add_blocker", "id": "@1", "blocker": "@0"}
    ]"#;

    let (stdout, stderr, success) = bf_batch_stdin(batch_input);

    assert!(success, "Batch with dependencies failed: stdout={}, stderr={}", stdout, stderr);

    let bead_ids = parse_bead_ids_from_batch_output(&stdout);
    assert_eq!(bead_ids.len(), 2, "Expected 2 bead IDs (2 creates, dep doesn't create bead)");

    let blocker_id = &bead_ids[0];
    let blocked_id = &bead_ids[1];

    // Verify both beads are P0
    let blocker = bf_json(&["show", blocker_id, "--format", "json"]);
    let blocked = bf_json(&["show", blocked_id, "--format", "json"]);

    assert_eq!(blocker[0]["priority"], 0);
    assert_eq!(blocked[0]["priority"], 0);

    // Verify dependency exists
    let blocked_bead = bf_json(&["show", blocked_id, "--format", "json"]);
    assert!(blocked_bead[0]["dependencies"].as_array().map_or(false, |deps| !deps.is_empty()));

    println!("✓ Batch created P0 beads with dependencies successfully");
}

#[test]
fn test_batch_close_p0_beads() {
    // Create P0 beads
    let bead1 = create_p0_bead("P0 close test 1");
    let bead2 = create_p0_bead("P0 close test 2");

    // Close both in a batch
    let batch_input = format!(r#"[
        {{"op": "close", "id": "{}", "reason": "P0 task completed"}},
        {{"op": "close", "id": "{}", "reason": "P0 task done"}}
    ]"#, bead1, bead2);

    let (stdout, stderr, success) = bf_batch_stdin(&batch_input);

    assert!(success, "Batch close failed: stdout={}, stderr={}", stdout, stderr);

    // Verify both are closed
    let bead1_closed = bf_json(&["show", &bead1, "--format", "json"]);
    let bead2_closed = bf_json(&["show", &bead2, "--format", "json"]);

    assert_eq!(bead1_closed[0]["status"], "closed");
    assert_eq!(bead2_closed[0]["status"], "closed");
    assert_eq!(bead1_closed[0]["priority"], 0); // Still P0 even when closed
    assert_eq!(bead2_closed[0]["priority"], 0);

    println!("✓ Batch closed 2 P0 beads successfully");
}

#[test]
fn test_batch_mixed_priority_with_p0() {
    // Test batch with mixed priorities including P0
    let batch_input = r#"[
        {"op": "create", "title": "P0 critical", "priority": 0, "type": "task"},
        {"op": "create", "title": "P1 high", "priority": 1, "type": "task"},
        {"op": "create", "title": "P2 medium", "priority": 2, "type": "task"},
        {"op": "create", "title": "P0 another critical", "priority": 0, "type": "bug"}
    ]"#;

    let (stdout, stderr, success) = bf_batch_stdin(batch_input);

    assert!(success, "Batch with mixed priorities failed: stdout={}, stderr={}", stdout, stderr);

    let bead_ids = parse_bead_ids_from_batch_output(&stdout);
    assert_eq!(bead_ids.len(), 4);

    // Verify priorities
    let p0_count = bead_ids.iter()
        .filter(|id| {
            let bead = bf_json(&["show", id, "--format", "json"]);
            bead[0]["priority"].as_i64() == Some(0)
        })
        .count();

    assert_eq!(p0_count, 2, "Expected 2 P0 beads");

    println!("✓ Batch with mixed priorities including P0 succeeded");
}

#[test]
fn test_batch_p0_label_operations() {
    // Create a P0 bead
    let bead_id = create_p0_bead("P0 label test");

    // Add and remove labels in batch
    let batch_input = format!(r#"[
        {{"op": "label_add", "id": "{}", "labels": ["urgent", "critical", "backend"]}},
        {{"op": "label_remove", "id": "{}", "labels": ["urgent"]}}
    ]"#, bead_id, bead_id);

    let (_stdout, stderr, success) = bf_batch_stdin(&batch_input);

    assert!(success, "Batch label operations failed: stderr={}", stderr);

    // Verify final labels
    let bead = bf_json(&["show", &bead_id, "--format", "json"]);
    let labels: Vec<String> = bead[0]["labels"].as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"critical".to_string()));
    assert!(labels.contains(&"backend".to_string()));
    assert!(!labels.contains(&"urgent".to_string()));

    // Verify still P0
    assert_eq!(bead[0]["priority"], 0);

    println!("✓ Batch label operations on P0 bead succeeded");
}

#[test]
fn test_batch_p0_comment_operations() {
    // Create a P0 bead
    let bead_id = create_p0_bead("P0 comment test");

    // Add multiple comments in batch
    let batch_input = format!(r#"[
        {{"op": "comment", "id": "{}", "author": "tester", "text": "First comment on P0"}},
        {{"op": "comment", "id": "{}", "author": "reviewer", "text": "Second comment on P0"}}
    ]"#, bead_id, bead_id);

    let (_stdout, stderr, success) = bf_batch_stdin(&batch_input);

    assert!(success, "Batch comment operations failed: stderr={}", stderr);

    // Verify comments were added
    let bead = bf_json(&["show", &bead_id, "--format", "json"]);
    let comments = bead[0]["comments"].as_array().unwrap();
    assert_eq!(comments.len(), 2);

    println!("✓ Batch comment operations on P0 bead succeeded");
}

#[test]
fn test_batch_p0_error_handling() {
    // Test that batch operations fail fast on error with P0 beads
    // Create a valid bead first
    let valid_bead = create_p0_bead("Valid P0 bead");

    // Attempt batch with one invalid operation
    let batch_input = format!(r#"[
        {{"op": "update", "id": "{}", "status": "in_progress"}},
        {{"op": "update", "id": "bf-nonexistent", "status": "in_progress"}}
    ]"#, valid_bead);

    let (_stdout, stderr, success) = bf_batch_stdin(&batch_input);

    // Should fail due to non-existent bead
    assert!(!success, "Batch should fail on error");
    assert!(stderr.contains("Bead not found") || stderr.contains("error"),
            "Expected error message, got: {}", stderr);

    println!("✓ Batch error handling with P0 beads works correctly");
}

#[test]
fn test_batch_complex_p0_workflow() {
    // Complex workflow: create P0 beads, add deps, update, close
    let batch_input = r#"[
        {"op": "create", "title": "P0 Infrastructure fix", "priority": 0, "type": "bug", "labels": ["infrastructure", "critical"]},
        {"op": "create", "title": "P0 Security patch", "priority": 0, "type": "bug", "labels": ["security", "critical"]},
        {"op": "create", "title": "P0 Data recovery", "priority": 0, "type": "task", "assignee": "dba-team"},
        {"op": "dep_add_blocker", "id": "@2", "blocker": "@0"},
        {"op": "dep_add_blocker", "id": "@2", "blocker": "@1"},
        {"op": "update", "id": "@0", "status": "in_progress", "assignee": "infra-team"},
        {"op": "label_add", "id": "@2", "labels": ["database"]},
        {"op": "comment", "id": "@2", "author": "lead", "text": "Critical data recovery task"}
    ]"#;

    let (stdout, stderr, success) = bf_batch_stdin(batch_input);

    assert!(success, "Complex batch failed: stdout={}, stderr={}", stdout, stderr);

    let bead_ids = parse_bead_ids_from_batch_output(&stdout);
    assert_eq!(bead_ids.len(), 3, "Expected 3 bead IDs");

    let infra_id = &bead_ids[0];
    let security_id = &bead_ids[1];
    let data_id = &bead_ids[2];

    // Verify all are P0
    for bead_id in &[infra_id, security_id, data_id] {
        let bead = bf_json(&["show", bead_id, "--format", "json"]);
        assert_eq!(bead[0]["priority"], 0);
    }

    // Verify data recovery has 2 blockers
    let data_bead = bf_json(&["show", data_id, "--format", "json"]);
    assert!(data_bead[0]["dependencies"].as_array().map_or(false, |deps| deps.len() >= 2));

    // Verify infra bead was updated
    let infra_bead = bf_json(&["show", infra_id, "--format", "json"]);
    assert_eq!(infra_bead[0]["status"], "in_progress");
    assert_eq!(infra_bead[0]["assignee"], "infra-team");

    // Verify data bead has database label
    let data_final = bf_json(&["show", data_id, "--format", "json"]);
    let labels: Vec<String> = data_final[0]["labels"].as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    assert!(labels.contains(&"database".to_string()));

    println!("✓ Complex P0 batch workflow succeeded");
}

// Helper function to create a P0 bead
fn create_p0_bead(title: &str) -> String {
    let create_output = bf(&[
        "create",
        "--title", title,
        "--priority", "0",
        "--type", "task"
    ]);

    create_output.trim().to_string()
}
