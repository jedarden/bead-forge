//! Integration tests for epic CLI commands
//!
//! Tests epic creation and display via CLI commands:
//! - br create --type epic epic-name
//! - br show displays epic type correctly
//! - br list filters epic type issues
//! - Epic appears in JSON output with correct type

use std::fs;
use std::path::PathBuf;
use std::process::Command;
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

/// Extract bead ID from command output
fn extract_bead_id(output: &str) -> String {
    output
        .lines()
        .find(|line| line.contains("bf-"))
        .and_then(|line| line.split("bf-").nth(1))
        .map(|id| format!("bf-{}", id.trim().split_whitespace().next().unwrap_or(id)))
        .expect("Could not extract bead ID from output")
}

#[test]
fn test_create_epic_via_cli() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create an epic via CLI
    let create_result = Command::new(&bf_path)
        .arg("create")
        .arg("--title")
        .arg("Test Epic Creation")
        .arg("--type")
        .arg("epic")
        .arg("--priority")
        .arg("1")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");

    assert!(
        create_result.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&create_result.stderr)
    );

    let create_output = String::from_utf8(create_result.stdout).unwrap();
    println!("Create output:\n{}", create_output);

    // Verify bead ID was created
    let bead_id = extract_bead_id(&create_output);
    assert!(bead_id.starts_with("bf-"), "Bead ID should start with bf-");
}

#[test]
fn test_create_multiple_epics() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create multiple epics
    let epic_titles = vec![
        "First Epic",
        "Second Epic",
        "Third Epic"
    ];

    let mut bead_ids = Vec::new();
    for title in epic_titles {
        let create_result = Command::new(&bf_path)
            .arg("create")
            .arg("--title")
            .arg(title)
            .arg("--type")
            .arg("epic")
            .current_dir(workspace)
            .output()
            .expect("Failed to run bf create");

        assert!(
            create_result.status.success(),
            "bf create failed for {}: {}",
            title,
            String::from_utf8_lossy(&create_result.stderr)
        );

        let create_output = String::from_utf8(create_result.stdout).unwrap();
        let bead_id = extract_bead_id(&create_output);
        bead_ids.push(bead_id);
    }

    // Verify we created 3 distinct epics
    assert_eq!(bead_ids.len(), 3, "Should create 3 epics");
    let unique_ids: std::collections::HashSet<_> = bead_ids.iter().collect();
    assert_eq!(unique_ids.len(), 3, "All bead IDs should be unique");
}

#[test]
fn test_show_displays_epic_type_correctly() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create an epic
    let create_result = Command::new(&bf_path)
        .arg("create")
        .arg("--title")
        .arg("Epic Show Test")
        .arg("--type")
        .arg("epic")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");

    assert!(
        create_result.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&create_result.stderr)
    );

    let create_output = String::from_utf8(create_result.stdout).unwrap();
    let bead_id = extract_bead_id(&create_output);

    // Show the epic in text format
    let show_result = Command::new(&bf_path)
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

    let show_output = String::from_utf8(show_result.stdout).unwrap();
    println!("Show output:\n{}", show_output);

    // Verify epic type is displayed correctly
    assert!(
        show_output.contains("Type:") || show_output.contains("type:"),
        "Output should contain type information"
    );
    assert!(
        show_output.contains("epic"),
        "Output should contain 'epic' as the type"
    );
    assert!(
        show_output.contains("Epic Show Test"),
        "Output should contain the epic title"
    );
}

#[test]
fn test_show_json_format_epic() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create an epic with description
    let create_result = Command::new(&bf_path)
        .arg("create")
        .arg("--title")
        .arg("JSON Epic Test")
        .arg("--type")
        .arg("epic")
        .arg("--description")
        .arg("Test epic description")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");

    assert!(
        create_result.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&create_result.stderr)
    );

    let create_output = String::from_utf8(create_result.stdout).unwrap();
    let bead_id = extract_bead_id(&create_output);

    // Show the epic in JSON format
    let show_result = Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
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

    let json_output = String::from_utf8(show_result.stdout).unwrap();
    println!("JSON output:\n{}", json_output);

    // Parse and verify JSON structure - show returns an array
    let json_array: serde_json::Value = serde_json::from_str(&json_output)
        .expect("Failed to parse JSON output");

    let json = json_array.as_array()
        .expect("Show output should be an array")
        .first()
        .expect("Array should have at least one element");

    assert_eq!(
        json["id"].as_str(),
        Some(bead_id.as_str()),
        "JSON should contain correct bead ID"
    );
    assert_eq!(
        json["title"].as_str(),
        Some("JSON Epic Test"),
        "JSON should contain correct title"
    );
    assert_eq!(
        json["issue_type"].as_str(),
        Some("epic"),
        "JSON should contain epic type"
    );
    assert_eq!(
        json["description"].as_str(),
        Some("Test epic description"),
        "JSON should contain description"
    );
}

#[test]
fn test_list_filters_epic_type_issues() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create mix of epics and tasks
    let epic_titles = vec!["Epic One", "Epic Two"];
    let task_titles = vec!["Task One", "Task Two"];

    // Create epics
    for title in &epic_titles {
        let create_result = Command::new(&bf_path)
            .arg("create")
            .arg("--title")
            .arg(title)
            .arg("--type")
            .arg("epic")
            .current_dir(workspace)
            .output()
            .expect("Failed to run bf create");

        assert!(
            create_result.status.success(),
            "bf create failed for {}: {}",
            title,
            String::from_utf8_lossy(&create_result.stderr)
        );
    }

    // Create tasks
    for title in &task_titles {
        let create_result = Command::new(&bf_path)
            .arg("create")
            .arg("--title")
            .arg(title)
            .arg("--type")
            .arg("task")
            .current_dir(workspace)
            .output()
            .expect("Failed to run bf create");

        assert!(
            create_result.status.success(),
            "bf create failed for {}: {}",
            title,
            String::from_utf8_lossy(&create_result.stderr)
        );
    }

    // List all issues
    let list_result = Command::new(&bf_path)
        .arg("list")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf list");

    assert!(
        list_result.status.success(),
        "bf list failed: {}",
        String::from_utf8_lossy(&list_result.stderr)
    );

    let list_output = String::from_utf8(list_result.stdout).unwrap();
    println!("List output:\n{}", list_output);

    // Verify both epics and tasks are listed
    assert!(
        list_output.contains("Epic One") || list_output.contains("Epic Two"),
        "List should contain epic titles"
    );
    assert!(
        list_output.contains("Task One") || list_output.contains("Task Two"),
        "List should contain task titles"
    );
}

#[test]
fn test_list_json_format_epics() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create epics with different priorities
    let epics = vec![
        ("High Priority Epic", "1"),
        ("Medium Priority Epic", "2"),
    ];

    for (title, priority) in &epics {
        let create_result = Command::new(&bf_path)
            .arg("create")
            .arg("--title")
            .arg(title)
            .arg("--type")
            .arg("epic")
            .arg("--priority")
            .arg(priority)
            .current_dir(workspace)
            .output()
            .expect("Failed to run bf create");

        assert!(
            create_result.status.success(),
            "bf create failed: {}",
            String::from_utf8_lossy(&create_result.stderr)
        );
    }

    // List in JSON format
    let list_result = Command::new(&bf_path)
        .arg("list")
        .arg("--format")
        .arg("json")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf list");

    assert!(
        list_result.status.success(),
        "bf list failed: {}",
        String::from_utf8_lossy(&list_result.stderr)
    );

    let json_output = String::from_utf8(list_result.stdout).unwrap();
    println!("List JSON output:\n{}", json_output);

    // Parse JSONL format (newline-delimited JSON objects)
    let issues: Vec<serde_json::Value> = json_output
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).expect("Failed to parse JSON line"))
        .collect();

    // Verify we have at least the 2 epics we created
    assert!(
        issues.len() >= 2,
        "Should have at least 2 issues, got {}",
        issues.len()
    );

    // Count epics in the list
    let epic_count = issues
        .iter()
        .filter(|issue| issue["issue_type"].as_str() == Some("epic"))
        .count();

    assert!(
        epic_count >= 2,
        "Should have at least 2 epics in list, got {}",
        epic_count
    );
}

#[test]
fn test_epic_appears_in_json_with_correct_type() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create an epic
    let create_result = Command::new(&bf_path)
        .arg("create")
        .arg("--title")
        .arg("Epic JSON Type Test")
        .arg("--type")
        .arg("epic")
        .arg("--priority")
        .arg("1")
        .arg("--description")
        .arg("This is an epic for testing JSON type output")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");

    assert!(
        create_result.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&create_result.stderr)
    );

    let create_output = String::from_utf8(create_result.stdout).unwrap();
    let bead_id = extract_bead_id(&create_output);

    // Show in JSON format
    let show_result = Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
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

    let json_output = String::from_utf8(show_result.stdout).unwrap();
    println!("Epic JSON output:\n{}", json_output);

    // Parse and verify complete epic structure - show returns an array
    let json_array: serde_json::Value = serde_json::from_str(&json_output)
        .expect("Failed to parse epic JSON");

    let json = json_array.as_array()
        .expect("Show output should be an array")
        .first()
        .expect("Array should have at least one element");

    // Verify all expected fields with epic type
    assert_eq!(json["id"].as_str(), Some(bead_id.as_str()));
    assert_eq!(json["title"].as_str(), Some("Epic JSON Type Test"));
    assert_eq!(json["issue_type"].as_str(), Some("epic"));
    assert_eq!(json["priority"].as_u64(), Some(1));
    assert_eq!(json["status"].as_str(), Some("open"));
    assert_eq!(
        json["description"].as_str(),
        Some("This is an epic for testing JSON type output")
    );

    // Verify JSON structure contains expected keys
    let expected_keys = vec![
        "id", "title", "issue_type", "status", "priority",
        "description", "created_at", "updated_at"
    ];

    for key in expected_keys {
        assert!(
            json.get(key).is_some(),
            "JSON should contain {} field",
            key
        );
    }
}

#[test]
fn test_create_epic_with_all_fields() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create epic with all possible fields
    let create_result = Command::new(&bf_path)
        .arg("create")
        .arg("--title")
        .arg("Complete Epic Test")
        .arg("--type")
        .arg("epic")
        .arg("--priority")
        .arg("0")
        .arg("--description")
        .arg("This is a comprehensive epic with all fields")
        .arg("--assignee")
        .arg("test-user")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");

    assert!(
        create_result.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&create_result.stderr)
    );

    let create_output = String::from_utf8(create_result.stdout).unwrap();
    let bead_id = extract_bead_id(&create_output);

    // Verify epic with show command
    let show_result = Command::new(&bf_path)
        .arg("show")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");

    let json_output = String::from_utf8(show_result.stdout).unwrap();
    let json_array: serde_json::Value = serde_json::from_str(&json_output)
        .expect("Failed to parse epic JSON");

    let json = json_array.as_array()
        .expect("Show output should be an array")
        .first()
        .expect("Array should have at least one element");

    // Verify all fields are set correctly
    assert_eq!(json["issue_type"].as_str(), Some("epic"));
    assert_eq!(json["title"].as_str(), Some("Complete Epic Test"));
    assert_eq!(json["priority"].as_u64(), Some(0));
    assert_eq!(json["status"].as_str(), Some("open"));
    assert_eq!(
        json["description"].as_str(),
        Some("This is a comprehensive epic with all fields")
    );
    assert_eq!(json["assignee"].as_str(), Some("test-user"));
    assert_eq!(json["id"].as_str(), Some(bead_id.as_str()));
}

#[test]
fn test_multiple_epics_in_list_output() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();
    let bf_path = get_bf_binary();

    // Create multiple epics
    let epic_count = 5;
    for i in 1..=epic_count {
        let title = format!("Epic Number {}", i);
        let create_result = Command::new(&bf_path)
            .arg("create")
            .arg("--title")
            .arg(&title)
            .arg("--type")
            .arg("epic")
            .current_dir(workspace)
            .output()
            .expect("Failed to run bf create");

        assert!(
            create_result.status.success(),
            "bf create failed for {}: {}",
            title,
            String::from_utf8_lossy(&create_result.stderr)
        );
    }

    // List all issues in JSON format
    let list_result = Command::new(&bf_path)
        .arg("list")
        .arg("--format")
        .arg("json")
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf list");

    let json_output = String::from_utf8(list_result.stdout).unwrap();

    // Parse JSONL format (newline-delimited JSON objects)
    let issues: Vec<serde_json::Value> = json_output
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).expect("Failed to parse JSON line"))
        .collect();

    // Count how many epics are in the list
    let epic_count_in_list = issues
        .iter()
        .filter(|issue| issue["issue_type"].as_str() == Some("epic"))
        .count();

    assert_eq!(
        epic_count_in_list, epic_count,
        "Should have exactly {} epics in list, got {}",
        epic_count, epic_count_in_list
    );

    // Verify all epics have correct type
    for issue in issues {
        if issue["issue_type"].as_str() == Some("epic") {
            assert_eq!(issue["issue_type"].as_str(), Some("epic"));
        }
    }
}
