//! Tests for empty candidates scenarios in `bf ready`
//!
//! This test module ensures that cmd_ready handles all edge cases with empty candidates.
//!
//! Test Cases:
//! - Claim all available beads, verify `bf ready` returns []
//! - Empty database (no beads at all), verify output
//! - JSON format: verify output is valid [] and exit code 0
//! - Text format: verify appropriate message and exit code 0
//! - All beads claimed but some exist (not ready state)
//! - All beads blocked (not ready state)

mod common;
use std::process::Command;

/// Get the path to the bf binary
fn bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf")
        .unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Create a Command builder for bf with workspace configured
fn bf_command(workspace: &common::TempWorkspace) -> Command {
    let mut cmd = Command::new(&bf_binary());
    cmd.arg("-w")
        .arg(&workspace.beads_dir)
        .current_dir(workspace.workspace_path());
    cmd
}

/// Parse JSON string
fn parse_json(json: &str) -> serde_json::Value {
    serde_json::from_str(json)
        .unwrap_or_else(|e| panic!("Failed to parse JSON: {}\nJSON was: {}", e, json))
}

// ============================================================================
// Test 1: Empty database (no beads at all)
// ============================================================================

#[test]
fn test_ready_empty_database() {
    let ws = common::TempWorkspace::new().unwrap();

    // Run bf ready on empty database
    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    // Should succeed with exit code 0
    assert!(
        output.status.success(),
        "bf ready should succeed with exit code 0 on empty database"
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should return [] for empty candidates
    assert_eq!(
        trimmed, "[]",
        "Empty ready should return '[]', got: {}",
        trimmed
    );
}

// ============================================================================
// Test 2: Empty database with text format
// ============================================================================

#[test]
fn test_ready_empty_database_text_format() {
    let ws = common::TempWorkspace::new().unwrap();

    // Run bf ready on empty database with text format (default)
    let output = bf_command(&ws)
        .arg("ready")
        .output()
        .expect("Failed to execute bf ready");

    // Should succeed with exit code 0
    assert!(
        output.status.success(),
        "bf ready should succeed with exit code 0 on empty database"
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should return "No ready candidates" message
    assert_eq!(
        trimmed, "No ready candidates",
        "Empty ready in text format should return 'No ready candidates', got: '{}'",
        trimmed
    );
}

// ============================================================================
// Test 3: All beads claimed (in_progress status)
// ============================================================================

#[test]
fn test_ready_all_beads_claimed() {
    let ws = common::TempWorkspace::new().unwrap();

    // Create some beads
    ws.create_bead("bf-task-1", "First task").unwrap();
    ws.create_bead("bf-task-2", "Second task").unwrap();
    ws.create_bead("bf-task-3", "Third task").unwrap();

    // Claim all beads by setting them to in_progress
    let storage = ws.storage().unwrap();

    for id in &["bf-task-1", "bf-task-2", "bf-task-3"] {
        let changes = bead_forge::IssueChanges {
            status: Some(bead_forge::Status::InProgress),
            assignee: Some("test-worker".to_string()),
            ..Default::default()
        };
        storage.update_issue(id, &changes).unwrap();
    }

    // Run bf ready - should return no candidates
    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(
        output.status.success(),
        "bf ready should succeed with exit code 0 when all beads are claimed"
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should return [] since all beads are claimed
    assert_eq!(
        trimmed, "[]",
        "Ready with all beads claimed should return '[]', got: {}",
        trimmed
    );
}

// ============================================================================
// Test 4: All beads blocked
// ============================================================================

#[test]
fn test_ready_all_beads_blocked() {
    let ws = common::TempWorkspace::new().unwrap();

    // Create a dependency chain where all beads are blocked
    ws.create_bead("bf-blocker-1", "Final task").unwrap();
    ws.create_bead("bf-blocked-1", "Blocked task 1").unwrap();
    ws.create_bead("bf-blocked-2", "Blocked task 2").unwrap();

    // Create blocking dependencies
    let storage = ws.storage().unwrap();
    storage
        .add_dependency(
            "bf-blocked-1",
            "bf-blocker-1",
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();
    storage
        .add_dependency(
            "bf-blocked-2",
            "bf-blocked-1",
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();

    // Set blocker to in_progress (so it won't be ready)
    let changes = bead_forge::IssueChanges {
        status: Some(bead_forge::Status::InProgress),
        assignee: Some("test-worker".to_string()),
        ..Default::default()
    };
    storage
        .update_issue("bf-blocker-1", &changes)
        .unwrap();

    // Run bf ready - should return no candidates
    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(
        output.status.success(),
        "bf ready should succeed with exit code 0 when all beads are blocked"
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should return [] since all beads are either blocked or in_progress
    assert_eq!(
        trimmed, "[]",
        "Ready with all beads blocked should return '[]', got: {}",
        trimmed
    );
}

// ============================================================================
// Test 5: All beads closed
// ============================================================================

#[test]
fn test_ready_all_beads_closed() {
    let ws = common::TempWorkspace::new().unwrap();

    // Create beads and close them all
    for i in 1..=3 {
        let id = format!("bf-closed-{}", i);
        let title = format!("Closed task {}", i);
        ws.create_bead(&id, &title).unwrap();

        let storage = ws.storage().unwrap();
        let changes = bead_forge::IssueChanges {
            status: Some(bead_forge::Status::Closed),
            actor: Some("test".to_string()),
            ..Default::default()
        };
        storage.update_issue(&id, &changes).unwrap();
    }

    // Run bf ready - should return no candidates
    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(
        output.status.success(),
        "bf ready should succeed with exit code 0 when all beads are closed"
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should return [] since all beads are closed
    assert_eq!(
        trimmed, "[]",
        "Ready with all beads closed should return '[]', got: {}",
        trimmed
    );
}

// ============================================================================
// Test 6: JSON format validity check
// ============================================================================

#[test]
fn test_ready_json_format_valid_empty() {
    let ws = common::TempWorkspace::new().unwrap();

    // Run bf ready with JSON format on empty database
    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(
        output.status.success(),
        "bf ready should succeed with exit code 0"
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Verify it's valid JSON
    let json = parse_json(trimmed);

    // Verify it's an empty array
    assert!(
        json.is_array(),
        "JSON output should be an array, got: {:?}",
        json
    );
    let arr = json.as_array().unwrap();
    assert_eq!(
        arr.len(), 0,
        "JSON array should be empty, got {} elements",
        arr.len()
    );
}

// ============================================================================
// Test 7: Text format message check
// ============================================================================

#[test]
fn test_ready_text_format_message_empty() {
    let ws = common::TempWorkspace::new().unwrap();

    // Run bf ready with default text format on empty database
    let output = bf_command(&ws)
        .arg("ready")
        .output()
        .expect("Failed to execute bf ready");

    assert!(
        output.status.success(),
        "bf ready should succeed with exit code 0"
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Verify the message is appropriate
    assert_eq!(
        trimmed, "No ready candidates",
        "Text format should show 'No ready candidates', got: '{}'",
        trimmed
    );

    // Verify exit code is 0
    assert_eq!(
        output.status.code(), Some(0),
        "Exit code should be 0, got: {:?}",
        output.status.code()
    );
}

// ============================================================================
// Test 8: Toon format message check
// ============================================================================

#[test]
fn test_ready_toon_format_message_empty() {
    let ws = common::TempWorkspace::new().unwrap();

    // Run bf ready with toon format on empty database
    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("toon")
        .output()
        .expect("Failed to execute bf ready");

    assert!(
        output.status.success(),
        "bf ready should succeed with exit code 0"
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Toon format should also show "No ready candidates" message
    assert_eq!(
        trimmed, "No ready candidates",
        "Toon format should show 'No ready candidates', got: '{}'",
        trimmed
    );
}

// ============================================================================
// Test 9: Verify exit code 0 for all empty scenarios
// ============================================================================

#[test]
fn test_ready_exit_code_zero_all_empty_scenarios() {
    let ws = common::TempWorkspace::new().unwrap();

    // Test various empty scenarios
    let scenarios = vec![
        vec!["ready", "--format", "json"],
        vec!["ready", "--format", "text"],
        vec!["ready", "--format", "toon"],
        vec!["ready"],
        vec!["ready", "--limit", "0"],
        vec!["ready", "--limit", "10"],
    ];

    for args in scenarios {
        let output = bf_command(&ws)
            .args(&args)
            .output()
            .expect(&format!("Failed to execute bf ready with args: {:?}", args));

        assert!(
            output.status.success(),
            "bf ready {:?} should succeed with exit code 0",
            args
        );

        assert_eq!(
            output.status.code(), Some(0),
            "bf ready {:?} exit code should be 0, got: {:?}",
            args,
            output.status.code()
        );
    }
}

// ============================================================================
// Test 10: Mixed scenario - some ready, then all claimed
// ============================================================================

#[test]
fn test_ready_mixed_then_all_claimed() {
    let ws = common::TempWorkspace::new().unwrap();

    // Create some open beads
    ws.create_bead("bf-open-1", "Open task 1").unwrap();
    ws.create_bead("bf-open-2", "Open task 2").unwrap();

    // Initially should have ready candidates
    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Should NOT be empty initially
    assert_ne!(
        trimmed, "[]",
        "Should initially have ready candidates"
    );

    // Now claim all beads
    let storage = ws.storage().unwrap();
    for id in &["bf-open-1", "bf-open-2"] {
        let changes = bead_forge::IssueChanges {
            status: Some(bead_forge::Status::InProgress),
            assignee: Some("worker".to_string()),
            ..Default::default()
        };
        storage.update_issue(id, &changes).unwrap();
    }

    // Now should return empty
    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute bf ready");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    assert_eq!(
        trimmed, "[]",
        "Should return [] after all beads claimed"
    );
}

// ============================================================================
// Test 11: Envelope mode with empty candidates
// ============================================================================

#[test]
fn test_ready_envelope_empty_candidates() {
    let ws = common::TempWorkspace::new().unwrap();

    // Run bf ready with envelope on empty database
    let output = bf_command(&ws)
        .arg("ready")
        .arg("--format")
        .arg("json")
        .arg("--envelope")
        .output()
        .expect("Failed to execute bf ready");

    assert!(
        output.status.success(),
        "bf ready with envelope should succeed with exit code 0"
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let envelope = parse_json(&stdout);

    // Verify envelope structure
    assert_eq!(
        envelope["version"], 1,
        "Envelope version should be 1"
    );
    assert_eq!(
        envelope["kind"], "ready",
        "Envelope kind should be 'ready'"
    );

    // Verify data is an empty array
    let data = &envelope["data"];
    assert!(
        data.is_array(),
        "Envelope data should be an array, got: {:?}",
        data
    );
    let arr = data.as_array().unwrap();
    assert_eq!(
        arr.len(), 0,
        "Envelope data array should be empty, got {} elements",
        arr.len()
    );
}
