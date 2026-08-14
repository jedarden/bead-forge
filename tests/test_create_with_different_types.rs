//! Tests for bead type validation and creation with different types.
//!
//! Tests all supported bead types (task, epic, bug, story, spike, genesis)
//! and validates that invalid types are rejected.

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::TempDir;

    /// Create a temporary test workspace with bf configuration
    fn bf_binary() -> String {
        std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
    }

    fn setup_test_workspace() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path();
        let beads_dir = workspace_dir.join(".beads");
        fs::create_dir(&beads_dir).unwrap();

        // Initialize workspace with proper bf config format
        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            "issue_prefixes: [test]\ndefault_priority: 2\ndefault_type: task\nclaim_ttl_minutes: 30\n",
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

    /// Verify bead was stored in database with expected type
    fn verify_bead_type(beads_dir: &PathBuf, bead_id: &str, expected_type: &str) {
        let db_path = beads_dir.join("beads.db");
        let conn = Connection::open(&db_path).expect("Failed to open database for verification");

        let mut stmt = conn
            .prepare("SELECT issue_type FROM issues WHERE id = ?1")
            .expect("Failed to prepare statement");

        let mut rows = stmt.query(&[bead_id]).expect("Failed to execute query");

        let row = rows
            .next()
            .expect("Failed to get row")
            .expect("No bead found with given ID");

        let issue_type: String = row.get(0).expect("Failed to get issue_type");
        assert_eq!(
            issue_type, expected_type,
            "Issue type should match expected type"
        );
    }

    #[test]
    fn test_create_with_task_type() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Task bead", "--type", "task", "--priority", "2"],
        );

        assert!(success, "Create with task type should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");

        let bead_id = stdout.trim();
        verify_bead_type(&beads_dir, bead_id, "task");
    }

    #[test]
    fn test_create_with_epic_type() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Epic bead", "--type", "epic", "--priority", "2"],
        );

        assert!(success, "Create with epic type should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");

        let bead_id = stdout.trim();
        verify_bead_type(&beads_dir, bead_id, "epic");
    }

    #[test]
    fn test_create_with_bug_type() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Bug bead", "--type", "bug", "--priority", "2"],
        );

        assert!(success, "Create with bug type should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");

        let bead_id = stdout.trim();
        verify_bead_type(&beads_dir, bead_id, "bug");
    }

    #[test]
    fn test_create_with_story_type() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Story bead", "--type", "story", "--priority", "2"],
        );

        assert!(success, "Create with story type should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");

        let bead_id = stdout.trim();
        verify_bead_type(&beads_dir, bead_id, "story");
    }

    #[test]
    fn test_create_with_spike_type() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Spike bead", "--type", "spike", "--priority", "2"],
        );

        assert!(success, "Create with spike type should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");

        let bead_id = stdout.trim();
        verify_bead_type(&beads_dir, bead_id, "spike");
    }

    #[test]
    fn test_create_with_genesis_type() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Genesis bead", "--type", "genesis", "--priority", "2"],
        );

        assert!(success, "Create with genesis type should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");

        let bead_id = stdout.trim();
        verify_bead_type(&beads_dir, bead_id, "genesis");
    }

    #[test]
    fn test_create_with_feature_type() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Feature bead", "--type", "feature", "--priority", "2"],
        );

        assert!(success, "Create with feature type should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");

        let bead_id = stdout.trim();
        verify_bead_type(&beads_dir, bead_id, "feature");
    }

    #[test]
    fn test_create_with_chore_type() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Chore bead", "--type", "chore", "--priority", "2"],
        );

        assert!(success, "Create with chore type should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");

        let bead_id = stdout.trim();
        verify_bead_type(&beads_dir, bead_id, "chore");
    }

    #[test]
    fn test_create_with_docs_type() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Docs bead", "--type", "docs", "--priority", "2"],
        );

        assert!(success, "Create with docs type should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");

        let bead_id = stdout.trim();
        verify_bead_type(&beads_dir, bead_id, "docs");
    }

    #[test]
    fn test_create_with_question_type() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Question bead", "--type", "question", "--priority", "2"],
        );

        assert!(success, "Create with question type should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");

        let bead_id = stdout.trim();
        verify_bead_type(&beads_dir, bead_id, "question");
    }

    #[test]
    fn test_create_with_invalid_type_rejected() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Invalid type test", "--type", "invalid_type_xyz", "--priority", "2"],
        );

        assert!(!success, "Create with invalid type should fail");
        assert!(
            stderr.contains("Invalid type") || stderr.contains("type"),
            "Error message should mention invalid type: {}",
            stderr
        );
        assert!(stdout.is_empty(), "No output should be produced on failure");
    }

    #[test]
    fn test_create_with_empty_type_rejected() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Empty type test", "--type", "", "--priority", "2"],
        );

        assert!(!success, "Create with empty type should fail");
        assert!(
            stderr.contains("type") || stderr.contains("empty"),
            "Error message should mention type: {}",
            stderr
        );
    }

    #[test]
    fn test_create_with_custom_type_rejected() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Custom type test", "--type", "custom", "--priority", "2"],
        );

        assert!(!success, "Create with custom type should fail");
        assert!(
            stderr.contains("Invalid type") || stderr.contains("type"),
            "Error message should mention invalid type: {}",
            stderr
        );
    }

    #[test]
    fn test_create_default_type_is_task() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Create without specifying type (should default to task)
        let (stdout, stderr, success) =
            run_create(&beads_dir, &["--title", "Default type test", "--priority", "2"]);

        assert!(success, "Create without type should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");

        let bead_id = stdout.trim();
        verify_bead_type(&beads_dir, bead_id, "task");
    }

    #[test]
    fn test_create_type_case_insensitive() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Test uppercase types
        let types = vec!["TASK", "EPIC", "BUG", "STORY", "SPIKE", "GENESIS"];

        for type_str in types {
            let (stdout, stderr, success) = run_create(
                &beads_dir,
                &[
                    "--title",
                    format!("{} test", type_str).as_str(),
                    "--type",
                    type_str,
                    "--priority",
                    "2",
                ],
            );

            assert!(
                success,
                "Create with uppercase type {} should succeed. stderr: {}",
                type_str,
                stderr
            );
            assert!(!stdout.is_empty(), "Should output bead ID for type {}", type_str);

            let bead_id = stdout.trim();
            let expected_lowercase = type_str.to_lowercase();
            verify_bead_type(&beads_dir, bead_id, &expected_lowercase);
        }
    }

    #[test]
    fn test_create_type_with_whitespace_trimmed() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Whitespace test", "--type", "  story  ", "--priority", "2"],
        );

        assert!(success, "Create with padded type should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");

        let bead_id = stdout.trim();
        verify_bead_type(&beads_dir, bead_id, "story");
    }

    #[test]
    fn test_create_all_types_in_sequence() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let types = vec!["task", "epic", "bug", "story", "spike", "genesis"];

        for (i, type_str) in types.iter().enumerate() {
            let (stdout, stderr, success) = run_create(
                &beads_dir,
                &[
                    "--title",
                    format!("Test bead {}: {}", i + 1, type_str).as_str(),
                    "--type",
                    type_str,
                    "--priority",
                    "2",
                ],
            );

            assert!(
                success,
                "Create with type {} should succeed. stderr: {}",
                type_str,
                stderr
            );
            assert!(!stdout.is_empty(), "Should output bead ID for type {}", type_str);

            let bead_id = stdout.trim();
            verify_bead_type(&beads_dir, bead_id, type_str);
        }
    }

    #[test]
    fn test_create_json_with_story_type() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Story JSON test",
                "--type",
                "story",
                "--priority",
                "2",
                "--json",
            ],
        );

        assert!(success, "Create with story type and --json should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "JSON output should not be empty");

        // Parse the JSON output
        let json_output: serde_json::Value =
            serde_json::from_str(&stdout).expect("Output should be valid JSON");

        // Access the data field
        let data = &json_output["data"];

        // Verify type field in JSON output
        assert_eq!(
            data["type"].as_str(),
            Some("story"),
            "Type should be 'story' in JSON output"
        );
    }

    #[test]
    fn test_create_json_with_spike_type() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Spike JSON test",
                "--type",
                "spike",
                "--priority",
                "2",
                "--json",
            ],
        );

        assert!(success, "Create with spike type and --json should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "JSON output should not be empty");

        // Parse the JSON output
        let json_output: serde_json::Value =
            serde_json::from_str(&stdout).expect("Output should be valid JSON");

        // Access the data field
        let data = &json_output["data"];

        // Verify type field in JSON output
        assert_eq!(
            data["type"].as_str(),
            Some("spike"),
            "Type should be 'spike' in JSON output"
        );
    }

    #[test]
    fn test_create_json_with_genesis_type() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Genesis JSON test",
                "--type",
                "genesis",
                "--priority",
                "2",
                "--json",
            ],
        );

        assert!(
            success,
            "Create with genesis type and --json should succeed. stderr: {}",
            stderr
        );
        assert!(!stdout.is_empty(), "JSON output should not be empty");

        // Parse the JSON output
        let json_output: serde_json::Value =
            serde_json::from_str(&stdout).expect("Output should be valid JSON");

        // Access the data field
        let data = &json_output["data"];

        // Verify type field in JSON output
        assert_eq!(
            data["type"].as_str(),
            Some("genesis"),
            "Type should be 'genesis' in JSON output"
        );
    }
}
