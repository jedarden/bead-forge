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
    use rusqlite::Connection;
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

    /// Helper function to query database directly and verify bead storage
    ///
    /// This function opens the SQLite database directly and queries all fields
    /// of a bead to ensure it was stored correctly.
    fn verify_bead_in_database(beads_dir: &PathBuf, bead_id: &str, expected_fields: &BeadFields) {
        let db_path = beads_dir.join("beads.db");
        let conn = Connection::open(&db_path).expect("Failed to open database for verification");

        // Query the main issue record
        let mut stmt = conn
            .prepare(
                "SELECT id, title, description, status, priority, issue_type, assignee
                 FROM issues WHERE id = ?1",
            )
            .expect("Failed to prepare statement");

        let mut rows = stmt.query(&[bead_id]).expect("Failed to execute query");

        let row = rows
            .next()
            .expect("Failed to get row")
            .expect("No bead found with given ID");

        // Verify all fields match expected values
        let id: String = row.get(0).expect("Failed to get id");
        assert_eq!(id, bead_id, "Bead ID should match");

        let title: String = row.get(1).expect("Failed to get title");
        assert_eq!(title, expected_fields.title, "Title should match");

        let description: String = row.get(2).expect("Failed to get description");
        assert_eq!(
            description, expected_fields.description,
            "Description should match"
        );

        let status: String = row.get(3).expect("Failed to get status");
        assert_eq!(status, expected_fields.status, "Status should match");

        let priority: i32 = row.get(4).expect("Failed to get priority");
        assert_eq!(priority, expected_fields.priority, "Priority should match");

        let issue_type: String = row.get(5).expect("Failed to get issue_type");
        assert_eq!(
            issue_type, expected_fields.issue_type,
            "Issue type should match"
        );

        let assignee: Option<String> = row.get(6).expect("Failed to get assignee");
        assert_eq!(assignee, expected_fields.assignee, "Assignee should match");

        // Verify labels in bead_labels table
        let mut stmt = conn
            .prepare("SELECT label FROM bead_labels WHERE bead_id = ?1 ORDER BY label")
            .expect("Failed to prepare labels query");

        let labels: Vec<String> = stmt
            .query_map(&[bead_id], |row| row.get(0))
            .expect("Failed to query labels")
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to collect labels");

        assert_eq!(
            labels, expected_fields.labels,
            "Labels should match (sorted for comparison)"
        );
    }

    /// Struct to hold expected bead field values for verification
    struct BeadFields {
        title: String,
        description: String,
        status: String,
        priority: i32,
        issue_type: String,
        assignee: Option<String>,
        labels: Vec<String>,
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
        assert!(
            list_output.contains("test-user"),
            "Assignee should be in list"
        );
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
        assert!(
            list_output.contains("urgent"),
            "Third label should be in list"
        );
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
            &[
                "--title",
                "Feature bead",
                "--type",
                "feature",
                "--priority",
                "2",
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");
    }

    #[test]
    fn test_create_priority_critical() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Critical bead",
                "--type",
                "task",
                "--priority",
                "0",
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");
    }

    #[test]
    fn test_create_priority_high() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "High priority bead",
                "--type",
                "task",
                "--priority",
                "1",
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");
    }

    #[test]
    fn test_create_priority_medium() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Medium priority bead",
                "--type",
                "task",
                "--priority",
                "2",
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");
    }

    #[test]
    fn test_create_priority_low() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Low priority bead",
                "--type",
                "task",
                "--priority",
                "3",
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Should output bead ID");
    }

    #[test]
    fn test_create_priority_backlog() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Backlog bead",
                "--type",
                "task",
                "--priority",
                "4",
            ],
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
            &[
                "--title",
                "Second bead",
                "--type",
                "task",
                "--priority",
                "2",
            ],
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
            &[
                "--title",
                "Prefixed bead",
                "--type",
                "task",
                "--priority",
                "2",
            ],
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
        let (stdout, stderr, success) =
            run_create(&beads_dir, &["--type", "task", "--priority", "2"]);

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
        println!(
            "Empty title test - success: {}, stdout: {}, stderr: {}",
            success, stdout, stderr
        );
    }

    #[test]
    fn test_create_json_output() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "JSON test bead",
                "--type",
                "task",
                "--priority",
                "2",
                "--json",
            ],
        );

        assert!(
            success,
            "Create command with --json should succeed. stderr: {}",
            stderr
        );
        assert!(!stdout.is_empty(), "JSON output should not be empty");

        // Parse the JSON output (wrapped in envelope)
        let json_output: serde_json::Value =
            serde_json::from_str(&stdout).expect("Output should be valid JSON");

        // Verify envelope structure
        assert!(
            json_output.get("version").is_some(),
            "JSON output should contain 'version' field"
        );
        assert!(
            json_output.get("kind").is_some(),
            "JSON output should contain 'kind' field"
        );
        assert!(
            json_output.get("data").is_some(),
            "JSON output should contain 'data' field"
        );

        // Access the data field which contains the actual issue data
        let data = &json_output["data"];

        // Verify required fields exist in data
        assert!(
            data.get("id").is_some(),
            "JSON data should contain 'id' field"
        );
        assert!(
            data.get("title").is_some(),
            "JSON data should contain 'title' field"
        );
        assert!(
            data.get("type").is_some(),
            "JSON data should contain 'type' field"
        );
        assert!(
            data.get("priority").is_some(),
            "JSON data should contain 'priority' field"
        );
        assert!(
            data.get("status").is_some(),
            "JSON data should contain 'status' field"
        );

        // Verify field values
        assert_eq!(
            data["title"].as_str(),
            Some("JSON test bead"),
            "Title should match input"
        );
        assert_eq!(
            data["type"].as_str(),
            Some("task"),
            "Type should match input"
        );
        assert_eq!(
            data["priority"].as_u64(),
            Some(2),
            "Priority should match input"
        );
        assert_eq!(
            data["status"].as_str(),
            Some("open"),
            "Status should be 'open' for newly created bead"
        );

        // Verify ID is a string and follows expected format
        let bead_id = data["id"].as_str();
        assert!(bead_id.is_some(), "ID should be a string");
        assert!(
            bead_id.unwrap().starts_with("test-"),
            "ID should start with configured prefix"
        );

        // Verify envelope kind
        assert_eq!(
            json_output["kind"].as_str(),
            Some("create"),
            "Envelope kind should be 'create'"
        );

        println!("JSON output validation passed: {}", stdout);
    }

    #[test]
    fn test_create_verify_in_database() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Create a bead with all fields populated
        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Database verification bead",
                "--type",
                "bug",
                "--priority",
                "1",
                "--description",
                "This is a test description for database verification",
                "--assignee",
                "test-developer",
                "--label",
                "database-test",
                "--label",
                "priority-high",
                "--label",
                "verification",
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Output should contain bead ID");

        let bead_id = stdout.trim();
        println!("Created bead with ID: {}", bead_id);

        // Expected field values for verification
        let expected = BeadFields {
            title: "Database verification bead".to_string(),
            description: "This is a test description for database verification".to_string(),
            status: "open".to_string(), // Newly created beads have status "open"
            priority: 1,
            issue_type: "bug".to_string(),
            assignee: Some("test-developer".to_string()),
            labels: vec![
                "database-test".to_string(),
                "priority-high".to_string(),
                "verification".to_string(),
            ],
        };

        // Verify the bead was stored correctly in the database
        verify_bead_in_database(&beads_dir, bead_id, &expected);
    }
}
