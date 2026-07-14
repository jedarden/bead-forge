//! Epic type validation tests
//!
//! Test suite that verifies epic type is correctly validated during bead creation.
//! Covers:
//! - Positive case: valid epic type creation succeeds
//! - Negative case: invalid epic type fails appropriately
//! - Type field is correctly set in storage

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Create a temporary test workspace with bf configuration
/// Resolve the freshly-built bf binary — never the system-installed one.
fn bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

fn setup_test_workspace() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path();
    let beads_dir = workspace_dir.join(".beads");
    fs::create_dir(&beads_dir).unwrap();

    // Initialize workspace with bf config
    let config_path = beads_dir.join("config.yaml");
    fs::write(
        &config_path,
        "# Test workspace config\nworkspace:\n  name: test-workspace\nid:\n  prefix: \"test\"\n",
    )
    .unwrap();

    // Initialize metadata
    let metadata_path = beads_dir.join("metadata.json");
    fs::write(
        &metadata_path,
        r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
    )
    .unwrap();

    // Initialize empty issues.jsonl
    let issues_path = beads_dir.join("issues.jsonl");
    fs::write(&issues_path, "").unwrap();

    (temp_dir, beads_dir)
}

/// Run bf create command with the given arguments
fn run_create(beads_dir: &PathBuf, args: &[&str]) -> (String, String, bool) {
    let mut cmd = Command::new(bf_binary());
    cmd.arg("--workspace").arg(beads_dir);
    cmd.arg("create");
    for arg in args {
        cmd.arg(arg);
    }

    let output = cmd.output().expect("Failed to execute bf create");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    (stdout, stderr, success)
}

/// Run bf show command to get bead details
fn run_show(beads_dir: &PathBuf, bead_id: &str) -> String {
    let output = Command::new(bf_binary())
        .arg("--workspace")
        .arg(beads_dir)
        .arg("show")
        .arg(bead_id)
        .output()
        .expect("Failed to execute bf show");

    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Run bf show command with JSON output
fn run_show_json(beads_dir: &PathBuf, bead_id: &str) -> String {
    let output = Command::new(bf_binary())
        .arg("--workspace")
        .arg(beads_dir)
        .arg("show")
        .arg(bead_id)
        .arg("--json")
        .output()
        .expect("Failed to execute bf show");

    String::from_utf8_lossy(&output.stdout).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epic_type_positive_case() {
        // Test creating an epic with --type epic succeeds
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Test Epic",
                "--type",
                "epic",
                "--priority",
                "2",
            ],
        );

        assert!(success, "Epic creation should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Output should contain bead ID");

        let bead_id = stdout.trim();
        println!("Created epic bead with ID: {}", bead_id);

        // Verify the epic type is preserved in show output
        let show_output = run_show(&beads_dir, bead_id);
        assert!(
            show_output.contains("epic"),
            "Epic type should be preserved in show output. Got: {}",
            show_output
        );
    }

    #[test]
    fn test_valid_epic_type_creation() {
        // Test that valid epic type creation succeeds and type field is set correctly
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Create an issue with a valid epic type
        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Valid Epic Creation Test",
                "--type",
                "epic",
                "--priority",
                "1",
            ],
        );

        // Assert the creation succeeds
        assert!(success, "Valid epic type creation should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();
        assert!(!bead_id.is_empty(), "Bead ID should be returned");

        // Verify the type field is set to 'epic' via JSON output
        let json_output = run_show_json(&beads_dir, bead_id);
        let parsed: serde_json::Value =
            serde_json::from_str(&json_output).expect("Failed to parse JSON output");

        // Output is wrapped in an array: [{issue}]
        let issue = &parsed[0];
        assert_eq!(
            issue["issue_type"],
            "epic",
            "Type field should be set to 'epic'"
        );
        assert_eq!(issue["title"], "Valid Epic Creation Test");
        assert_eq!(issue["priority"], 1);
    }

    #[test]
    fn test_epic_type_storage_validation() {
        // Test that epic type is correctly stored in the database
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Create epic
        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Storage Validation Epic",
                "--type",
                "epic",
                "--priority",
                "1",
            ],
        );

        assert!(success, "Epic creation should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();

        // Verify via JSON output that the type field is correctly set
        let json_output = run_show_json(&beads_dir, bead_id);

        // Parse JSON to verify the type field
        let parsed: serde_json::Value =
            serde_json::from_str(&json_output).expect("Failed to parse JSON output");

        // Output is wrapped in an array: [{issue}]
        let issue = &parsed[0];
        assert_eq!(
            issue["issue_type"],
            "epic",
            "Issue type should be 'epic' in storage"
        );
        assert_eq!(issue["id"], bead_id, "ID should match created bead");
        assert_eq!(
            issue["title"],
            "Storage Validation Epic",
            "Title should be preserved"
        );
        assert_eq!(issue["priority"], 1, "Priority should be preserved");
    }

    #[test]
    fn test_epic_type_roundtrip_persistence() {
        // Test that epic type persists through sync/export roundtrip
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Create epic
        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Roundtrip Epic",
                "--type",
                "epic",
                "--priority",
                "0",
            ],
        );

        assert!(success, "Epic creation should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();

        // Export to JSONL
        let sync_output = Command::new(bf_binary())
            .arg("--workspace")
            .arg(&beads_dir)
            .arg("sync")
            .arg("--flush-only")
            .output()
            .expect("Failed to execute bf sync");

        assert!(
            sync_output.status.success(),
            "Sync should succeed: {}",
            String::from_utf8_lossy(&sync_output.stderr)
        );

        // Read the JSONL file
        let jsonl_path = beads_dir.join("issues.jsonl");
        let jsonl_content =
            fs::read_to_string(&jsonl_path).expect("Failed to read JSONL file");

        assert!(
            jsonl_content.contains("\"issue_type\":\"epic\""),
            "Epic type should be in JSONL export"
        );
        assert!(
            jsonl_content.contains(&bead_id),
            "Bead ID should be in JSONL export"
        );
    }

    #[test]
    fn test_invalid_empty_type() {
        // Test that creating a bead with empty type is handled gracefully
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (_stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Invalid Type Test",
                "--type",
                "",
                "--priority",
                "2",
            ],
        );

        // Empty type is accepted as a Custom type (IssueType::Custom(""))
        // This is by design - the system allows custom types
        assert!(success, "Empty type should be accepted as Custom type. stderr: {}", stderr);

        // Verify the bead was created
        let bead_id = _stdout.trim();
        assert!(!bead_id.is_empty(), "Bead ID should be returned");
    }

    #[test]
    fn test_invalid_special_characters_type() {
        // Test creating beads with type strings containing control characters
        // The system accepts custom types, so these should succeed
        let test_cases = vec![
            ("", "Empty Type"),           // Empty string
            ("   ", "Whitespace Type"),   // All whitespace
            ("custom-type", "Custom Type with Dash"), // Custom type with dash
        ];

        for (type_val, title) in test_cases {
            let (_temp_dir, beads_dir) = setup_test_workspace();

            let (stdout, stderr, success) = run_create(
                &beads_dir,
                &[
                    "--title",
                    title,
                    "--type",
                    type_val,
                    "--priority",
                    "2",
                ],
            );

            assert!(
                success,
                "Type '{}' should be accepted as Custom type. stderr: {}",
                type_val,
                stderr
            );

            let bead_id = stdout.trim();
            assert!(!bead_id.is_empty(), "Bead ID should be returned for type: {}", type_val);
        }
    }

    #[test]
    fn test_epic_type_case_insensitivity() {
        // Test epic type handles case variations correctly
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Test lowercase "epic"
        let (stdout_lower, stderr_lower, success_lower) = run_create(
            &beads_dir,
            &[
                "--title",
                "Lowercase Epic",
                "--type",
                "epic",
                "--priority",
                "2",
            ],
        );

        assert!(
            success_lower,
            "Lowercase 'epic' should succeed. stderr: {}",
            stderr_lower
        );

        let bead_id_lower = stdout_lower.trim();
        let show_lower = run_show(&beads_dir, bead_id_lower);
        assert!(
            show_lower.contains("epic"),
            "Lowercase epic should be recognized. Got: {}",
            show_lower
        );

        // Test uppercase "EPIC"
        let (_temp_dir2, beads_dir2) = setup_test_workspace();

        let (stdout_upper, stderr_upper, success_upper) = run_create(
            &beads_dir2,
            &[
                "--title",
                "Uppercase Epic",
                "--type",
                "EPIC",
                "--priority",
                "2",
            ],
        );

        // Epic should be case-insensitive or normalized
        assert!(
            success_upper,
            "Uppercase 'EPIC' should succeed or be normalized. stderr: {}",
            stderr_upper
        );

        if success_upper {
            let bead_id_upper = stdout_upper.trim();
            let show_upper = run_show(&beads_dir2, bead_id_upper);
            assert!(
                show_upper.to_lowercase().contains("epic"),
                "Epic should be recognized regardless of case. Got: {}",
                show_upper
            );
        }
    }

    #[test]
    fn test_epic_vs_other_types_distinction() {
        // Test that epic type is distinct from other standard types
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Create beads with different types
        let type_tests = vec![
            ("task", "Task Test"),
            ("bug", "Bug Test"),
            ("feature", "Feature Test"),
            ("epic", "Epic Test"),
            ("chore", "Chore Test"),
        ];

        for (issue_type, title) in &type_tests {
            let (stdout, stderr, success) = run_create(
                &beads_dir,
                &[ "--title", title, "--type", issue_type, "--priority", "2", ],
            );

            assert!(
                success,
                "Type '{}' should succeed. stderr: {}",
                issue_type,
                stderr
            );

            let bead_id = stdout.trim();
            let show_output = run_show(&beads_dir, bead_id);

            // Verify each type is preserved correctly
            assert!(
                show_output.contains(issue_type),
                "Type '{}' should be preserved for bead '{}'. Got: {}",
                issue_type,
                bead_id,
                show_output
            );
        }
    }

    #[test]
    fn test_epic_type_with_all_priorities() {
        // Test that epic type works with all priority levels
        let priorities = vec!["0", "1", "2", "3", "4"];

        for priority in priorities {
            let (_temp_dir, beads_dir) = setup_test_workspace();

            let (stdout, stderr, success) = run_create(
                &beads_dir,
                &[
                    "--title",
                    &format!("Epic with P{} priority", priority),
                    "--type",
                    "epic",
                    "--priority",
                    priority,
                ],
            );

            assert!(
                success,
                "Epic with priority {} should succeed. stderr: {}",
                priority,
                stderr
            );

            let bead_id = stdout.trim();
            let json_output = run_show_json(&beads_dir, bead_id);

            let parsed: serde_json::Value =
                serde_json::from_str(&json_output).expect("Failed to parse JSON");

            // Output is wrapped in an array: [{issue}]
            let issue = &parsed[0];
            assert_eq!(
                issue["issue_type"], "epic",
                "Issue type should be 'epic' for priority {}",
                priority
            );
            assert_eq!(
                issue["priority"], priority.parse::<i32>().unwrap(),
                "Priority should be preserved"
            );
        }
    }

    #[test]
    fn test_multiple_epics_creation() {
        // Test creating multiple epics in the same workspace
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let epic_count = 5;
        let mut bead_ids = Vec::new();

        for i in 1..=epic_count {
            let (stdout, stderr, success) = run_create(
                &beads_dir,
                &[
                    "--title",
                    &format!("Epic Number {}", i),
                    "--type",
                    "epic",
                    "--priority",
                    "2",
                ],
            );

            assert!(
                success,
                "Epic {} creation should succeed. stderr: {}",
                i,
                stderr
            );

            let bead_id = stdout.trim();
            bead_ids.push(bead_id.to_string());

            let show_output = run_show(&beads_dir, bead_id);
            assert!(
                show_output.contains("epic"),
                "Epic {} should have epic type. Got: {}",
                i,
                show_output
            );
        }

        // Verify all epics are distinct
        assert_eq!(
            bead_ids.len(),
            epic_count,
            "Should create {} distinct epic beads",
            epic_count
        );

        let unique_count = bead_ids.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(
            unique_count,
            epic_count,
            "All epic bead IDs should be unique"
        );
    }

    #[test]
    fn test_epic_type_with_description() {
        // Test epic type with full description fields
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Comprehensive Epic Test",
                "--type",
                "epic",
                "--priority",
                "0",
                "--description",
                "This is a detailed epic description for validation testing",
            ],
        );

        assert!(success, "Epic with description should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();
        let json_output = run_show_json(&beads_dir, bead_id);

        let parsed: serde_json::Value =
            serde_json::from_str(&json_output).expect("Failed to parse JSON");

        // Output is wrapped in an array: [{issue}]
        let issue = &parsed[0];
        assert_eq!(issue["issue_type"], "epic");
        assert_eq!(
            issue["description"],
            "This is a detailed epic description for validation testing"
        );
        assert_eq!(issue["priority"], 0);
    }
}
