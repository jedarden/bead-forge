// P0 Epic CLI Integration Tests
// Tests the complete CLI workflow for creating and managing P0 (critical priority) epics with labels
// This covers the full user journey from creation to verification via CLI commands

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Get the path to the freshly-built bf binary
fn bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf")
        .unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Setup a temporary test workspace with proper bf configuration
fn setup_test_workspace() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");
    fs::create_dir_all(&beads_dir).unwrap();

    // Initialize config
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

    // Create metadata.json
    let metadata_path = beads_dir.join("metadata.json");
    fs::write(
        &metadata_path,
        r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
    )
    .unwrap();

    // Initialize empty issues.jsonl
    let issues_path = beads_dir.join("issues.jsonl");
    fs::write(&issues_path, "").unwrap();

    temp_dir
}

/// Extract bead ID from CLI output
fn extract_bead_id(output: &str) -> String {
    output
        .lines()
        .find(|line| line.contains("bf-"))
        .and_then(|line| line.split("bf-").nth(1))
        .map(|id| {
            format!(
                "bf-{}",
                id.trim()
                    .split_whitespace()
                    .next()
                    .unwrap_or(id)
                    .trim_end_matches('.')
            )
        })
        .expect("Could not extract bead ID from output")
}

#[test]
fn test_create_p0_epic_with_single_label() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();

    // Create P0 epic with single label
    let create_output = Command::new(bf_binary())
        .arg("create")
        .arg("--title")
        .arg("Critical Infrastructure Epic")
        .arg("--type")
        .arg("epic")
        .arg("--priority")
        .arg("0")
        .arg("--label")
        .arg("critical")
        .current_dir(workspace)
        .output()
        .expect("Failed to create P0 epic");

    assert!(
        create_output.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&create_output.stderr)
    );

    let create_text = String::from_utf8_lossy(&create_output.stdout);
    let bead_id = extract_bead_id(&create_text);

    // Verify the epic was created with P0 priority and label
    let show_output = Command::new(bf_binary())
        .arg("show")
        .arg(&bead_id)
        .arg("--json")
        .current_dir(workspace)
        .output()
        .expect("Failed to show epic");

    assert!(show_output.status.success());
    let show_json = String::from_utf8_lossy(&show_output.stdout);

    // Verify it's an epic with P0 priority
    assert!(show_json.contains(r#""issue_type":"epic""#));
    assert!(show_json.contains(r#""priority":0"#));
    assert!(show_json.contains(r#""critical""#));
}

#[test]
fn test_create_p0_epic_with_multiple_labels() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();

    // Create P0 epic with multiple labels
    let create_output = Command::new(bf_binary())
        .arg("create")
        .arg("--title")
        .arg("Multi-label Critical Epic")
        .arg("--type")
        .arg("epic")
        .arg("--priority")
        .arg("0")
        .arg("--label")
        .arg("critical")
        .arg("--label")
        .arg("urgent")
        .arg("--label")
        .arg("security")
        .current_dir(workspace)
        .output()
        .expect("Failed to create P0 epic with multiple labels");

    assert!(
        create_output.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&create_output.stderr)
    );

    let bead_id = extract_bead_id(&String::from_utf8_lossy(&create_output.stdout));

    // Verify all labels are present
    let show_output = Command::new(bf_binary())
        .arg("show")
        .arg(&bead_id)
        .arg("--json")
        .current_dir(workspace)
        .output()
        .expect("Failed to show epic");

    assert!(show_output.status.success());
    let show_json = String::from_utf8_lossy(&show_output.stdout);

    assert!(show_json.contains(r#""issue_type":"epic""#));
    assert!(show_json.contains(r#""priority":0"#));
    assert!(show_json.contains(r#""critical""#));
    assert!(show_json.contains(r#""urgent""#));
    assert!(show_json.contains(r#""security""#));
}

#[test]
fn test_p0_epic_display_in_list() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();

    // Create P0 epic
    let create_output = Command::new(bf_binary())
        .arg("create")
        .arg("--title")
        .arg("Test P0 Epic for List")
        .arg("--type")
        .arg("epic")
        .arg("--priority")
        .arg("0")
        .arg("--label")
        .arg("critical")
        .current_dir(workspace)
        .output()
        .expect("Failed to create epic");

    assert!(create_output.status.success());

    // List all epics
    let list_output = Command::new(bf_binary())
        .arg("list")
        .arg("--type")
        .arg("epic")
        .arg("--json")
        .current_dir(workspace)
        .output()
        .expect("Failed to list epics");

    assert!(list_output.status.success());
    let list_json = String::from_utf8_lossy(&list_output.stdout);

    // Verify P0 epic appears in list
    assert!(list_json.contains(r#""issue_type":"epic""#));
    assert!(list_json.contains(r#""priority":0"#));
    assert!(list_json.contains("Test P0 Epic for List"));
}

#[test]
fn test_p0_epic_priority_variations() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();

    // Test creating P0 epic with different priority specifications
    let priorities = vec!["0", "P0", "critical"];

    for (i, priority_spec) in priorities.iter().enumerate() {
        let create_output = Command::new(bf_binary())
            .arg("create")
            .arg("--title")
            .arg(&format!("Priority Test Epic {}", i))
            .arg("--type")
            .arg("epic")
            .arg("--priority")
            .arg(priority_spec)
            .arg("--label")
            .arg("critical")
            .current_dir(workspace)
            .output()
            .expect("Failed to create epic");

        assert!(
            create_output.status.success(),
            "Failed with priority spec {}: {}",
            priority_spec,
            String::from_utf8_lossy(&create_output.stderr)
        );
    }

    // Verify all epics have P0 priority
    let list_output = Command::new(bf_binary())
        .arg("list")
        .arg("--type")
        .arg("epic")
        .arg("--json")
        .current_dir(workspace)
        .output()
        .expect("Failed to list epics");

    let list_json = String::from_utf8_lossy(&list_output.stdout);

    // Count occurrences of priority 0 in the output
    let p0_count = list_json.matches(r#""priority":0"#).count();
    assert_eq!(p0_count, 3, "Should have 3 epics with P0 priority");
}

#[test]
fn test_p0_epic_with_description_and_labels() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();

    // Create P0 epic with description and labels
    let create_output = Command::new(bf_binary())
        .arg("create")
        .arg("--title")
        .arg("Critical Security Fix Epic")
        .arg("--type")
        .arg("epic")
        .arg("--priority")
        .arg("0")
        .arg("--description")
        .arg("This epic tracks critical security fixes for authentication system")
        .arg("--label")
        .arg("critical")
        .arg("--label")
        .arg("security")
        .arg("--label")
        .arg("auth")
        .current_dir(workspace)
        .output()
        .expect("Failed to create epic with description");

    assert!(
        create_output.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&create_output.stderr)
    );

    let bead_id = extract_bead_id(&String::from_utf8_lossy(&create_output.stdout));

    // Verify description and labels
    let show_output = Command::new(bf_binary())
        .arg("show")
        .arg(&bead_id)
        .arg("--json")
        .current_dir(workspace)
        .output()
        .expect("Failed to show epic");

    assert!(show_output.status.success());
    let show_json = String::from_utf8_lossy(&show_output.stdout);

    assert!(show_json.contains(r#""issue_type":"epic""#));
    assert!(show_json.contains(r#""priority":0"#));
    assert!(show_json.contains("authentication system"));
    assert!(show_json.contains(r#""critical""#));
    assert!(show_json.contains(r#""security""#));
    assert!(show_json.contains(r#""auth""#));
}

#[test]
fn test_p0_epic_label_add_remove_via_cli() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();

    // Create P0 epic
    let create_output = Command::new(bf_binary())
        .arg("create")
        .arg("--title")
        .arg("Epic for Label Operations")
        .arg("--type")
        .arg("epic")
        .arg("--priority")
        .arg("0")
        .arg("--label")
        .arg("critical")
        .current_dir(workspace)
        .output()
        .expect("Failed to create epic");

    let bead_id = extract_bead_id(&String::from_utf8_lossy(&create_output.stdout));

    // Add additional labels
    let add_output = Command::new(bf_binary())
        .arg("label")
        .arg(&bead_id)
        .arg("add")
        .arg("urgent")
        .current_dir(workspace)
        .output()
        .expect("Failed to add label");

    assert!(
        add_output.status.success(),
        "label add failed: {}",
        String::from_utf8_lossy(&add_output.stderr)
    );

    // Verify label was added
    let show_output = Command::new(bf_binary())
        .arg("show")
        .arg(&bead_id)
        .arg("--json")
        .current_dir(workspace)
        .output()
        .expect("Failed to show epic");

    let show_json = String::from_utf8_lossy(&show_output.stdout);
    assert!(show_json.contains(r#""critical""#));
    assert!(show_json.contains(r#""urgent""#));

    // Remove a label
    let remove_output = Command::new(bf_binary())
        .arg("label")
        .arg(&bead_id)
        .arg("remove")
        .arg("critical")
        .current_dir(workspace)
        .output()
        .expect("Failed to remove label");

    assert!(
        remove_output.status.success(),
        "label remove failed: {}",
        String::from_utf8_lossy(&remove_output.stderr)
    );

    // Verify label was removed but priority is still P0
    let show_after_remove = Command::new(bf_binary())
        .arg("show")
        .arg(&bead_id)
        .arg("--json")
        .current_dir(workspace)
        .output()
        .expect("Failed to show epic after label removal");

    let show_json_after = String::from_utf8_lossy(&show_after_remove.stdout);
    assert!(!show_json_after.contains(r#""critical""#));
    assert!(show_json_after.contains(r#""urgent""#)); // Other label still there
    assert!(show_json_after.contains(r#""priority":0"#)); // Priority unchanged
}

#[test]
fn test_p0_epic_update_preserves_priority_and_labels() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();

    // Create P0 epic
    let create_output = Command::new(bf_binary())
        .arg("create")
        .arg("--title")
        .arg("Original Title")
        .arg("--type")
        .arg("epic")
        .arg("--priority")
        .arg("0")
        .arg("--label")
        .arg("critical")
        .current_dir(workspace)
        .output()
        .expect("Failed to create epic");

    let bead_id = extract_bead_id(&String::from_utf8_lossy(&create_output.stdout));

    // Update title without changing priority
    let update_output = Command::new(bf_binary())
        .arg("update")
        .arg(&bead_id)
        .arg("--title")
        .arg("Updated Epic Title")
        .current_dir(workspace)
        .output()
        .expect("Failed to update epic");

    assert!(
        update_output.status.success(),
        "update failed: {}",
        String::from_utf8_lossy(&update_output.stderr)
    );

    // Verify priority and labels are preserved
    let show_output = Command::new(bf_binary())
        .arg("show")
        .arg(&bead_id)
        .arg("--json")
        .current_dir(workspace)
        .output()
        .expect("Failed to show epic");

    let show_json = String::from_utf8_lossy(&show_output.stdout);
    assert!(show_json.contains("Updated Epic Title"));
    assert!(show_json.contains(r#""priority":0"#)); // Priority still P0
    assert!(show_json.contains(r#""critical""#)); // Labels still present
}

#[test]
fn test_p0_epic_json_output_format() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();

    // Create P0 epic
    let create_output = Command::new(bf_binary())
        .arg("create")
        .arg("--title")
        .arg("JSON Format Test Epic")
        .arg("--type")
        .arg("epic")
        .arg("--priority")
        .arg("0")
        .arg("--label")
        .arg("critical")
        .arg("--label")
        .arg("test")
        .current_dir(workspace)
        .output()
        .expect("Failed to create epic");

    let bead_id = extract_bead_id(&String::from_utf8_lossy(&create_output.stdout));

    // Get JSON output
    let show_output = Command::new(bf_binary())
        .arg("show")
        .arg(&bead_id)
        .arg("--json")
        .current_dir(workspace)
        .output()
        .expect("Failed to show epic");

    let show_json = String::from_utf8_lossy(&show_output.stdout);

    // Parse and verify JSON structure
    let parsed: serde_json::Value = serde_json::from_str(&show_json)
        .expect("Failed to parse JSON output");

    assert_eq!(parsed["issue_type"], "epic");
    assert_eq!(parsed["priority"], 0);
    assert!(parsed["labels"].is_array());

    let labels = parsed["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 2);
    assert!(labels.iter().any(|l| l == "critical"));
    assert!(labels.iter().any(|l| l == "test"));
}

#[test]
fn test_multiple_p0_epics_with_different_labels() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();

    // Create multiple P0 epics with different label combinations
    let epics = vec![
        ("Epic 1", vec!["critical"]),
        ("Epic 2", vec!["critical", "urgent"]),
        ("Epic 3", vec!["critical", "security", "auth"]),
        ("Epic 4", vec!["critical", "frontend", "performance"]),
    ];

    for (title, labels) in &epics {
        let mut cmd = Command::new(bf_binary());
        cmd.arg("create")
            .arg("--title")
            .arg(title)
            .arg("--type")
            .arg("epic")
            .arg("--priority")
            .arg("0");

        for label in labels {
            cmd.arg("--label").arg(label);
        }

        let output = cmd
            .current_dir(workspace)
            .output()
            .expect("Failed to create epic");

        assert!(
            output.status.success(),
            "Failed to create '{}': {}",
            title,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // List all P0 epics
    let list_output = Command::new(bf_binary())
        .arg("list")
        .arg("--type")
        .arg("epic")
        .arg("--priority")
        .arg("0")
        .arg("--json")
        .current_dir(workspace)
        .output()
        .expect("Failed to list epics");

    assert!(list_output.status.success());
    let list_json = String::from_utf8_lossy(&list_output.stdout);

    // Verify all P0 epics are present
    for (title, _) in &epics {
        assert!(list_json.contains(title));
    }

    // Verify all have P0 priority
    let p0_count = list_json.matches(r#""priority":0"#).count();
    assert_eq!(p0_count, epics.len());
}

#[test]
fn test_p0_epic_ready_list() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();

    // Create P0 epic
    let create_output = Command::new(bf_binary())
        .arg("create")
        .arg("--title")
        .arg("Ready Test Epic")
        .arg("--type")
        .arg("epic")
        .arg("--priority")
        .arg("0")
        .arg("--label")
        .arg("critical")
        .current_dir(workspace)
        .output()
        .expect("Failed to create epic");

    assert!(create_output.status.success());

    // Check ready list
    let ready_output = Command::new(bf_binary())
        .arg("ready")
        .arg("--json")
        .current_dir(workspace)
        .output()
        .expect("Failed to list ready beads");

    assert!(ready_output.status.success());
    let ready_json = String::from_utf8_lossy(&ready_output.stdout);

    // P0 epic should appear in ready list (it's unblocked)
    assert!(ready_json.contains("Ready Test Epic"));
}

#[test]
fn test_p0_epic_close_reopen() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();

    // Create P0 epic
    let create_output = Command::new(bf_binary())
        .arg("create")
        .arg("--title")
        .arg("Close Reopen Test")
        .arg("--type")
        .arg("epic")
        .arg("--priority")
        .arg("0")
        .arg("--label")
        .arg("critical")
        .current_dir(workspace)
        .output()
        .expect("Failed to create epic");

    let bead_id = extract_bead_id(&String::from_utf8_lossy(&create_output.stdout));

    // Close the epic
    let close_output = Command::new(bf_binary())
        .arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Completed successfully")
        .current_dir(workspace)
        .output()
        .expect("Failed to close epic");

    assert!(
        close_output.status.success(),
        "close failed: {}",
        String::from_utf8_lossy(&close_output.stderr)
    );

    // Verify it's closed
    let show_after_close = Command::new(bf_binary())
        .arg("show")
        .arg(&bead_id)
        .arg("--json")
        .current_dir(workspace)
        .output()
        .expect("Failed to show epic");

    let show_json = String::from_utf8_lossy(&show_after_close.stdout);
    assert!(show_json.contains(r#""status":"closed""#));

    // Reopen the epic
    let reopen_output = Command::new(bf_binary())
        .arg("reopen")
        .arg(&bead_id)
        .current_dir(workspace)
        .output()
        .expect("Failed to reopen epic");

    assert!(
        reopen_output.status.success(),
        "reopen failed: {}",
        String::from_utf8_lossy(&reopen_output.stderr)
    );

    // Verify it's open again with P0 priority preserved
    let show_after_reopen = Command::new(bf_binary())
        .arg("show")
        .arg(&bead_id)
        .arg("--json")
        .current_dir(workspace)
        .output()
        .expect("Failed to show epic after reopen");

    let show_json = String::from_utf8_lossy(&show_after_reopen.stdout);
    assert!(show_json.contains(r#""status":"open""#));
    assert!(show_json.contains(r#""priority":0"#)); // P0 preserved
    assert!(show_json.contains(r#""critical""#)); // Labels preserved
}

#[test]
fn test_p0_epic_search_by_label() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();

    // Create P0 epic with specific label
    let create_output = Command::new(bf_binary())
        .arg("create")
        .arg("--title")
        .arg("Searchable Critical Epic")
        .arg("--type")
        .arg("epic")
        .arg("--priority")
        .arg("0")
        .arg("--label")
        .arg("critical")
        .arg("--label")
        .arg("search-test")
        .current_dir(workspace)
        .output()
        .expect("Failed to create epic");

    assert!(create_output.status.success());

    // Search by label
    let search_output = Command::new(bf_binary())
        .arg("search")
        .arg("search-test")
        .arg("--json")
        .current_dir(workspace)
        .output()
        .expect("Failed to search");

    assert!(search_output.status.success());
    let search_json = String::from_utf8_lossy(&search_output.stdout);

    assert!(search_json.contains("Searchable Critical Epic"));
    assert!(search_json.contains(r#""priority":0"#));
}
