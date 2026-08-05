//! Tests for bf show and bf list commands
//!
//! Acceptance Criteria:
//! - bf show displays all bead fields (id, title, description, status, type, priority, assignee, created_at, updated_at)
//! - bf list shows beads in table format by default
//! - bf list --status filters by status (open, closed, blocked)
//! - bf list --type filters by issue type
//! - bf list --assignee filters by assignee
//! - bf list --priority filters by priority level
//! - Both commands support --json output format

mod common;

use std::process::Command;

/// Get the path to the bf binary
fn bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Create a Command builder for bf with workspace configured
fn bf_command(workspace: &common::TempWorkspace) -> Command {
    let mut cmd = Command::new(&bf_binary());
    cmd.arg("-w").arg(&workspace.beads_dir);
    cmd.current_dir(workspace.workspace_path());
    cmd
}

/// Test that bf show displays all required bead fields
#[test]
fn test_show_displays_all_required_fields() {
    let ws = common::TempWorkspace::new().unwrap();

    // Create a test bead with all fields populated
    let bead_id = "bf-show-test";
    ws.create_bead(bead_id, "Test bead for show command").unwrap();

    // Update with additional fields
    let storage = ws.storage().unwrap();
    let changes = bead_forge::IssueChanges {
        description: Some("Test description".to_string()),
        assignee: Some("test-assignee".to_string()),
        ..Default::default()
    };
    storage.update_issue(bead_id, &changes).unwrap();

    // Get text output (default format)
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success(), "bf show should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Verify all required fields are present in output
    assert!(stdout.contains(&format!("ID: {}", bead_id)), "Should show ID");
    assert!(stdout.contains("Title:"), "Should show title");
    assert!(stdout.contains("Status:"), "Should show status");
    assert!(stdout.contains("Priority:"), "Should show priority");
    assert!(stdout.contains("Type:"), "Should show type");
    assert!(stdout.contains("Description:"), "Should show description");
    assert!(stdout.contains("Assignee:"), "Should show assignee");
    assert!(stdout.contains("Created at:"), "Should show created_at");
    assert!(stdout.contains("Updated at:"), "Should show updated_at");
}

/// Test that bf list shows beads in table format by default
#[test]
fn test_list_table_format_by_default() {
    let ws = common::TempWorkspace::new().unwrap();

    // Create test beads
    ws.create_bead("bf-list-1", "First bead").unwrap();
    ws.create_bead("bf-list-2", "Second bead").unwrap();

    // Get list output (no format specified)
    let output = bf_command(&ws)
        .arg("list")
        .output()
        .expect("Failed to execute bf list");

    assert!(output.status.success(), "bf list should succeed");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Verify default text format: [ID] Title - Status (Priority)
    assert!(stdout.contains("[bf-list-1]"), "Should show first bead in default format");
    assert!(stdout.contains("[bf-list-2]"), "Should show second bead in default format");
    assert!(stdout.contains("First bead"), "Should show first bead title");
    assert!(stdout.contains("Second bead"), "Should show second bead title");
    assert!(stdout.contains("open"), "Should show status");
    assert!(stdout.contains("P2"), "Should show priority");
}

/// Test that bf list --status filters by status
#[test]
fn test_list_status_filter() {
    let ws = common::TempWorkspace::new().unwrap();

    // Create beads with different statuses
    ws.create_bead("bf-open", "Open bead").unwrap();

    // Create a closed bead
    let closed = bead_forge::Issue {
        id: "bf-closed".to_string(),
        title: "Closed bead".to_string(),
        status: bead_forge::Status::Closed,
        closed_at: Some(chrono::Utc::now()),
        close_reason: Some("Test".to_string()),
        ..Default::default()
    };
    ws.create_issue(&closed).unwrap();

    // Create a blocked bead
    let blocked = bead_forge::Issue {
        id: "bf-blocked".to_string(),
        title: "Blocked bead".to_string(),
        status: bead_forge::Status::Blocked,
        ..Default::default()
    };
    ws.create_issue(&blocked).unwrap();

    // Test filtering by open status
    let output = bf_command(&ws)
        .arg("list")
        .arg("--status")
        .arg("open")
        .output()
        .expect("Failed to execute bf list");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(stdout.contains("bf-open"), "Should show open bead");
    assert!(!stdout.contains("bf-closed"), "Should not show closed bead");

    // Test filtering by closed status
    let output = bf_command(&ws)
        .arg("list")
        .arg("--status")
        .arg("closed")
        .output()
        .expect("Failed to execute bf list");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(stdout.contains("bf-closed"), "Should show closed bead");
    assert!(!stdout.contains("bf-open"), "Should not show open bead");

    // Test filtering by blocked status
    let output = bf_command(&ws)
        .arg("list")
        .arg("--status")
        .arg("blocked")
        .output()
        .expect("Failed to execute bf list");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(stdout.contains("bf-blocked"), "Should show blocked bead");
}

/// Test that bf list --type filters by issue type
#[test]
fn test_list_type_filter() {
    let ws = common::TempWorkspace::new().unwrap();

    // Create beads with different types
    let bug = bead_forge::Issue {
        id: "bf-bug".to_string(),
        title: "Bug bead".to_string(),
        issue_type: bead_forge::IssueType::Bug,
        ..Default::default()
    };
    ws.create_issue(&bug).unwrap();

    let feature = bead_forge::Issue {
        id: "bf-feature".to_string(),
        title: "Feature bead".to_string(),
        issue_type: bead_forge::IssueType::Feature,
        ..Default::default()
    };
    ws.create_issue(&feature).unwrap();

    // Test filtering by bug type
    let output = bf_command(&ws)
        .arg("list")
        .arg("--type")
        .arg("bug")
        .output()
        .expect("Failed to execute bf list");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(stdout.contains("bf-bug"), "Should show bug");
    assert!(!stdout.contains("bf-feature"), "Should not show feature");

    // Test filtering by feature type
    let output = bf_command(&ws)
        .arg("list")
        .arg("--type")
        .arg("feature")
        .output()
        .expect("Failed to execute bf list");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(stdout.contains("bf-feature"), "Should show feature");
    assert!(!stdout.contains("bf-bug"), "Should not show bug");
}

/// Test that bf list --assignee filters by assignee
#[test]
fn test_list_assignee_filter() {
    let ws = common::TempWorkspace::new().unwrap();

    // Create beads with different assignees
    ws.create_bead("bf-unassigned", "Unassigned bead").unwrap();

    let storage = ws.storage().unwrap();

    // Create bead with assignee
    let assigned = bead_forge::Issue {
        id: "bf-assigned".to_string(),
        title: "Assigned bead".to_string(),
        assignee: Some("alice".to_string()),
        ..Default::default()
    };
    ws.create_issue(&assigned).unwrap();

    // Test filtering by assignee
    let output = bf_command(&ws)
        .arg("list")
        .arg("--assignee")
        .arg("alice")
        .output()
        .expect("Failed to execute bf list");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(stdout.contains("bf-assigned"), "Should show assigned bead");
    assert!(!stdout.contains("bf-unassigned"), "Should not show unassigned bead");
}

/// Test that bf list --priority filters by priority level
#[test]
fn test_list_priority_filter() {
    let ws = common::TempWorkspace::new().unwrap();

    // Create beads with different priorities
    let critical = bead_forge::Issue {
        id: "bf-critical".to_string(),
        title: "Critical bead".to_string(),
        priority: bead_forge::Priority::CRITICAL, // P0
        ..Default::default()
    };
    ws.create_issue(&critical).unwrap();

    let medium = bead_forge::Issue {
        id: "bf-medium".to_string(),
        title: "Medium bead".to_string(),
        priority: bead_forge::Priority::MEDIUM, // P2
        ..Default::default()
    };
    ws.create_issue(&medium).unwrap();

    // Test filtering by P0 priority
    let output = bf_command(&ws)
        .arg("list")
        .arg("--priority")
        .arg("0")
        .output()
        .expect("Failed to execute bf list");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(stdout.contains("bf-critical"), "Should show P0 bead");
    assert!(!stdout.contains("bf-medium"), "Should not show P2 bead");

    // Test filtering by P2 priority
    let output = bf_command(&ws)
        .arg("list")
        .arg("--priority")
        .arg("2")
        .output()
        .expect("Failed to execute bf list");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(stdout.contains("bf-medium"), "Should show P2 bead");
    assert!(!stdout.contains("bf-critical"), "Should not show P0 bead");
}

/// Test that bf show supports --json output format
#[test]
fn test_show_json_output() {
    let ws = common::TempWorkspace::new().unwrap();

    let bead_id = "bf-json-show";
    ws.create_bead(bead_id, "JSON test bead").unwrap();

    // Get JSON output
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .arg("--json")
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Verify output is valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&stdout.trim())
        .expect("Output should be valid JSON");

    // Verify it's an array (NEEDLE contract)
    let array = parsed.as_array().expect("Show output should be array");
    assert_eq!(array.len(), 1, "Should return single bead");

    let bead = &array[0];

    // Verify required fields are present in JSON
    assert!(bead.get("id").is_some(), "Should have id field");
    assert!(bead.get("title").is_some(), "Should have title field");
    assert!(bead.get("status").is_some(), "Should have status field");
    assert!(bead.get("priority").is_some(), "Should have priority field");
    assert!(bead.get("issue_type").is_some(), "Should have issue_type field");
    assert!(bead.get("created_at").is_some(), "Should have created_at field");
    assert!(bead.get("updated_at").is_some(), "Should have updated_at field");
}

/// Test that bf list supports --json output format
#[test]
fn test_list_json_output() {
    let ws = common::TempWorkspace::new().unwrap();

    ws.create_bead("bf-json-list-1", "JSON list test 1").unwrap();
    ws.create_bead("bf-json-list-2", "JSON list test 2").unwrap();

    // Get JSON output
    let output = bf_command(&ws)
        .arg("list")
        .arg("--json")
        .output()
        .expect("Failed to execute bf list");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Verify output is JSONL (one JSON object per line)
    for line in stdout.lines() {
        if !line.trim().is_empty() {
            let parsed: serde_json::Value = serde_json::from_str(line)
                .expect("Each line should be valid JSON");

            // Verify required fields
            assert!(parsed.get("id").is_some(), "Should have id field");
            assert!(parsed.get("title").is_some(), "Should have title field");
            assert!(parsed.get("status").is_some(), "Should have status field");
        }
    }
}

/// Test that bf show displays description field
#[test]
fn test_show_displays_description() {
    let ws = common::TempWorkspace::new().unwrap();

    let bead_id = "bf-show-desc";
    ws.create_bead(bead_id, "Test bead with description").unwrap();

    // Add description
    let storage = ws.storage().unwrap();
    let changes = bead_forge::IssueChanges {
        description: Some("This is a test description\nwith multiple lines".to_string()),
        ..Default::default()
    };
    storage.update_issue(bead_id, &changes).unwrap();

    // Get text output
    let output = bf_command(&ws)
        .arg("show")
        .arg(bead_id)
        .output()
        .expect("Failed to execute bf show");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(stdout.contains("Description:"), "Should show description label");
    assert!(stdout.contains("This is a test description"), "Should show description content");
}
