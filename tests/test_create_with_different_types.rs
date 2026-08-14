//! Test creating beads with different types (task, epic, bug, story, spike, genesis)
//!
//! This test verifies that the `bf create` command properly handles all standard
//! bead types as specified in the acceptance criteria for bead bf-4p1st5.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

// ============================================================================
// Test Fixtures and Helpers
// ============================================================================

/// Test workspace with isolated environment
pub struct TypeTestWorkspace {
    pub temp_dir: TempDir,
    pub workspace_dir: PathBuf,
    pub beads_dir: PathBuf,
}

impl TypeTestWorkspace {
    /// Create a new isolated test workspace
    pub fn new() -> anyhow::Result<Self> {
        let temp_dir = TempDir::new()?;
        let workspace_dir = temp_dir.path().join("test-workspace");
        fs::create_dir_all(&workspace_dir)?;

        let beads_dir = workspace_dir.join(".beads");
        fs::create_dir_all(&beads_dir)?;

        // Initialize bf config
        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            r#"issue_prefixes: [bf]
default_priority: 2
default_type: task
claim_ttl_minutes: 30
"#,
        )?;

        // Initialize metadata
        let metadata_path = beads_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
        )?;

        // Create empty database
        let db_path = beads_dir.join("beads.db");
        let _storage = bead_forge::storage::Storage::open(&db_path)?;

        Ok(TypeTestWorkspace {
            temp_dir,
            workspace_dir,
            beads_dir,
        })
    }
}

/// Test helper to create a bead and verify its type
fn test_create_type(workspace: &TypeTestWorkspace, title: &str, type_: &str) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_bf"));
    cmd.current_dir(&workspace.workspace_dir)
        .arg("create")
        .arg("--title")
        .arg(title)
        .arg("--type")
        .arg(type_);

    let output = cmd.output().expect("Failed to execute bf create");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify command succeeded
    assert!(output.status.success(), "create command failed: {:?}", output);

    // Extract bead ID from output (first line)
    let bead_id = stdout.lines().next().unwrap().trim();
    assert!(bead_id.starts_with("bf-"), "Invalid bead ID format: {}", bead_id);

    // Verify the bead was created with correct type using show --json
    let mut show_cmd = Command::new(env!("CARGO_BIN_EXE_bf"));
    show_cmd
        .current_dir(&workspace.workspace_dir)
        .arg("show")
        .arg("--format")
        .arg("json")
        .arg(bead_id);

    let show_output = show_cmd.output().expect("Failed to execute bf show");
    let show_json = String::from_utf8_lossy(&show_output.stdout);

    // Parse JSON and verify issue_type matches expected
    let json: serde_json::Value = serde_json::from_str(&show_json)
        .expect("Failed to parse JSON output");

    let issue_type = json[0]["issue_type"]
        .as_str()
        .expect("issue_type field missing or not a string");

    // IssueType::from_str is case-insensitive, so normalize for comparison
    assert_eq!(issue_type.to_lowercase(), type_.to_lowercase(),
               "Type mismatch: expected '{}', got '{}'", type_, issue_type);

    bead_id.to_string()
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_create_with_different_types() {
    // Create a temporary workspace for testing
    let workspace = TypeTestWorkspace::new().expect("Failed to create test workspace");

    // Test all required types from acceptance criteria
    let types = vec![
        ("task", "Test Task"),
        ("epic", "Test Epic"),
        ("bug", "Test Bug"),
        ("story", "Test Story"),
        ("spike", "Test Spike"),
        ("genesis", "Test Genesis"),
    ];

    for (type_, title) in types {
        test_create_type(&workspace, title, type_);
    }
}

#[test]
fn test_create_default_type_is_task() {
    // Create a temporary workspace for testing
    let workspace = TypeTestWorkspace::new().expect("Failed to create test workspace");

    // Create bead without --type flag (should default to task)
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_bf"));
    cmd.current_dir(&workspace.workspace_dir)
        .arg("create")
        .arg("--title")
        .arg("Default Type Test");

    let output = cmd.output().expect("Failed to execute bf create");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bead_id = stdout.lines().next().unwrap().trim();

    // Verify default type is task
    let mut show_cmd = Command::new(env!("CARGO_BIN_EXE_bf"));
    show_cmd
        .current_dir(&workspace.workspace_dir)
        .arg("show")
        .arg("--format")
        .arg("json")
        .arg(bead_id);

    let show_output = show_cmd.output().expect("Failed to execute bf show");
    let show_json = String::from_utf8_lossy(&show_output.stdout);
    let json: serde_json::Value = serde_json::from_str(&show_json)
        .expect("Failed to parse JSON output");

    let issue_type = json[0]["issue_type"]
        .as_str()
        .expect("issue_type field missing");

    assert_eq!(issue_type, "task", "Default type should be 'task', got '{}'", issue_type);
}

#[test]
fn test_create_type_validation_rejects_invalid_types() {
    // Create a temporary workspace for testing
    let workspace = TypeTestWorkspace::new().expect("Failed to create test workspace");

    // Attempt to create bead with invalid type
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_bf"));
    cmd.current_dir(&workspace.workspace_dir)
        .arg("create")
        .arg("--title")
        .arg("Invalid Type Test")
        .arg("--type")
        .arg("invalid_type");

    let output = cmd.output().expect("Failed to execute bf create");

    // Command should fail
    assert!(!output.status.success(), "create should reject invalid type");

    // Verify error message contains "Invalid type"
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid type") || stderr.contains("invalid"),
            "Error message should mention invalid type, got: {}", stderr);
}

#[test]
fn test_create_type_case_insensitive() {
    // Create a temporary workspace for testing
    let workspace = TypeTestWorkspace::new().expect("Failed to create test workspace");

    // Test case-insensitive type parsing
    let cases = vec![
        ("TASK", "task"),
        ("EPIC", "epic"),
        ("BuG", "bug"),
        ("StOrY", "story"),
        ("SPIKE", "spike"),
        ("GeNeSiS", "genesis"),
    ];

    for (input_type, expected_normalized) in cases {
        let bead_id = test_create_type(&workspace, &format!("Case Test {}", input_type), input_type);

        // Verify the type was normalized to lowercase in storage
        let mut show_cmd = Command::new(env!("CARGO_BIN_EXE_bf"));
        show_cmd
            .current_dir(&workspace.workspace_dir)
            .arg("show")
            .arg("--format")
            .arg("json")
            .arg(&bead_id);

        let show_output = show_cmd.output().expect("Failed to execute bf show");
        let show_json = String::from_utf8_lossy(&show_output.stdout);
        let json: serde_json::Value = serde_json::from_str(&show_json)
            .expect("Failed to parse JSON output");

        let issue_type = json[0]["issue_type"]
            .as_str()
            .expect("issue_type field missing");

        assert_eq!(issue_type, expected_normalized,
                   "Type should be normalized to '{}', got '{}'", expected_normalized, issue_type);
    }
}
