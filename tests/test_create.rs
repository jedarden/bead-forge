//! Comprehensive tests for the `bf create` command.
//!
//! Tests all aspects of bead creation including:
//! - Basic bead creation with required fields
//! - Optional fields (description, assignee, labels)
//! - Type field with all valid values
//! - Priority field with all valid values
//! - Multiple labels
//! - ID generation and uniqueness
//! - Verification in database after creation

#[cfg(test)]
mod tests {
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

    /// Run bf list command to verify bead was created
    fn run_list(beads_dir: &PathBuf) -> String {
        let output = Command::new(bf_binary())
            .arg("--workspace")
            .arg(beads_dir)
            .arg("list")
            .arg("--format")
            .arg("json")
            .output()
            .expect("Failed to execute bf list");

        String::from_utf8_lossy(&output.stdout).to_string()
    }

    #[test]
    fn test_create_basic_bead() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Test bead", "--type", "task", "--priority", "2"],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Output should contain bead ID");

        let bead_id = stdout.trim();
        println!("Created bead with ID: {}", bead_id);

        // Verify the bead appears in list output
        let list_output = run_list(&beads_dir);
        assert!(list_output.contains(bead_id), "Bead should be in list");
        assert!(list_output.contains("Test bead"), "Title should be in list");
    }

    #[test]
    fn test_create_with_description() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Test bead with description",
                "--type",
                "task",
                "--priority",
                "2",
                "--description",
                "This is a detailed description",
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();
        let list_output = run_list(&beads_dir);
        assert!(
            list_output.contains("detailed description"),
            "Description should be in list"
        );
    }

    #[test]
    fn test_create_with_assignee() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Test bead with assignee",
                "--type",
                "task",
                "--priority",
                "2",
                "--assignee",
                "test-user",
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();
        let list_output = run_list(&beads_dir);
        assert!(list_output.contains("test-user"), "Assignee should be in list");
    }

    #[test]
    fn test_create_with_single_label() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Test bead with label",
                "--type",
                "task",
                "--priority",
                "2",
                "--label",
                "phase-1",
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();
        let list_output = run_list(&beads_dir);
        assert!(list_output.contains("phase-1"), "Label should be in list");
    }

    #[test]
    fn test_create_with_multiple_labels() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Test bead with multiple labels",
                "--type",
                "task",
                "--priority",
                "2",
                "--label",
                "phase-1",
                "--label",
                "feature-x",
                "--label",
                "urgent",
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();
        let list_output = run_list(&beads_dir);
        assert!(
            list_output.contains("phase-1"),
            "First label should be in list"
        );
        assert!(
            list_output.contains("feature-x"),
            "Second label should be in list"
        );
        assert!(list_output.contains("urgent"), "Third label should be in list");
    }

    #[test]
    fn test_create_type_task() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Task bead", "--type", "task", "--priority", "2"],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");
    }

    #[test]
    fn test_create_type_bug() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Bug bead", "--type", "bug", "--priority", "2"],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");
    }

    #[test]
    fn test_create_type_feature() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Feature bead", "--type", "feature", "--priority", "2"],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");
    }

    #[test]
    fn test_create_priority_critical() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Critical bead", "--type", "task", "--priority", "0"],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");
    }

    #[test]
    fn test_create_priority_high() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "High priority bead", "--type", "task", "--priority", "1"],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");
    }

    #[test]
    fn test_create_priority_medium() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Medium priority bead", "--type", "task", "--priority", "2"],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");
    }

    #[test]
    fn test_create_priority_low() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Low priority bead", "--type", "task", "--priority", "3"],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");
    }

    #[test]
    fn test_create_priority_backlog() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Backlog bead", "--type", "task", "--priority", "4"],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");
    }

    #[test]
    fn test_create_with_all_fields() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Complete test bead",
                "--type",
                "feature",
                "--priority",
                "1",
                "--description",
                "A comprehensive description",
                "--assignee",
                "developer-1",
                "--label",
                "phase-2",
                "--label",
                "ui",
                "--label",
                "high-impact",
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();
        let list_output = run_list(&beads_dir);

        assert!(list_output.contains(bead_id), "Bead ID should be in list");
        assert!(
            list_output.contains("Complete test bead"),
            "Title should be in list"
        );
        assert!(
            list_output.contains("comprehensive description"),
            "Description should be in list"
        );
        assert!(
            list_output.contains("developer-1"),
            "Assignee should be in list"
        );
        assert!(list_output.contains("phase-2"), "Label should be in list");
    }

    #[test]
    fn test_create_generates_unique_ids() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (id1, stderr, success1) = run_create(
            &beads_dir,
            &["--title", "First bead", "--type", "task", "--priority", "2"],
        );
        assert!(success1, "First create should succeed. stderr: {}", stderr);

        let (id2, stderr, success2) = run_create(
            &beads_dir,
            &["--title", "Second bead", "--type", "task", "--priority", "2"],
        );
        assert!(success2, "Second create should succeed. stderr: {}", stderr);

        let id1 = id1.trim();
        let id2 = id2.trim();

        assert_ne!(id1, id2, "Each bead should have a unique ID");
        println!("Generated unique IDs: {} and {}", id1, id2);
    }

    #[test]
    fn test_create_id_has_prefix() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "Prefixed bead", "--type", "task", "--priority", "2"],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();
        assert!(
            bead_id.starts_with("test-"),
            "Bead ID should start with configured prefix 'test-'"
        );
        println!("Bead ID with prefix: {}", bead_id);
    }

    #[test]
    fn test_create_long_description() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let long_description = "This is a very long description that spans multiple lines. \
            It should be stored correctly in the database and retrieved properly. \
            The create command should handle descriptions of arbitrary length. \
            This ensures that users can provide detailed context for their beads.";

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Long description bead",
                "--type",
                "task",
                "--priority",
                "2",
                "--description",
                long_description,
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();
        let list_output = run_list(&beads_dir);
        assert!(
            list_output.contains("very long description"),
            "Long description should be stored"
        );
    }

    #[test]
    fn test_create_missing_title() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Test that create command fails when title is not provided
        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--type", "task", "--priority", "2"],
        );

        assert!(!success, "Create command should fail when title is missing");
        assert!(
            stdout.is_empty() || stderr.contains("title"),
            "Error message should mention missing title"
        );
    }

    #[test]
    fn test_create_empty_title() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Test that create command handles empty string title
        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &["--title", "", "--type", "task", "--priority", "2"],
        );

        // Empty title should either be rejected or accepted (depending on implementation)
        // This test documents current behavior
        println!("Empty title test - success: {}, stdout: {}, stderr: {}", success, stdout, stderr);
    }
}
