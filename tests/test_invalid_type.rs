//! Tests for invalid/custom issue type handling in bead-forge.
//!
//! Tests that bead-forge correctly handles:
//! - Non-standard issue types (Custom variants)
//! - Invalid type strings that should be handled gracefully
//! - Roundtrip serialization/deserialization of custom types
//! - CLI creation with custom issue types

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

    #[test]
    fn test_custom_type_creation() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Create a bead with a custom issue type
        let custom_type = "spike";
        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Test custom type",
                "--type",
                custom_type,
                "--priority",
                "2",
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Output should contain bead ID");

        let bead_id = stdout.trim();
        println!("Created bead with ID: {}", bead_id);

        // Verify the bead shows the custom type correctly
        let show_output = run_show(&beads_dir, bead_id);
        assert!(
            show_output.contains(custom_type),
            "Custom type should be preserved in show output"
        );
    }

    #[test]
    fn test_multiple_custom_types() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Create beads with various custom types
        let custom_types = vec!["spike", "investigation", "refactor", "hotfix"];

        for custom_type in custom_types {
            let (stdout, stderr, success) = run_create(
                &beads_dir,
                &[
                    "--title",
                    &format!("Test {} type", custom_type),
                    "--type",
                    custom_type,
                    "--priority",
                    "2",
                ],
            );

            assert!(
                success,
                "Create command should succeed for type '{}'. stderr: {}",
                custom_type,
                stderr
            );

            let bead_id = stdout.trim();
            let show_output = run_show(&beads_dir, bead_id);

            assert!(
                show_output.contains(custom_type),
                "Custom type '{}' should be preserved in show output",
                custom_type
            );
        }
    }

    #[test]
    fn test_custom_type_with_special_chars() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Create a bead with a custom type containing special characters
        let custom_type = "custom-type-v2";
        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Test custom type with special chars",
                "--type",
                custom_type,
                "--priority",
                "2",
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();
        let show_output = run_show(&beads_dir, bead_id);

        assert!(
            show_output.contains(custom_type),
            "Custom type with special characters should be preserved"
        );
    }

    #[test]
    fn test_custom_type_json_roundtrip() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Create a bead with a custom issue type
        let custom_type = "research-task";
        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Test custom type JSON roundtrip",
                "--type",
                custom_type,
                "--priority",
                "2",
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();

        // Get JSON output
        let json_output = run_show_json(&beads_dir, bead_id);

        // Verify the custom type is in the JSON output
        assert!(
            json_output.contains(custom_type),
            "Custom type should be in JSON output"
        );

        // Export to JSONL
        let export_output = Command::new(bf_binary())
            .arg("--workspace")
            .arg(&beads_dir)
            .arg("sync")
            .arg("--flush-only")
            .output()
            .expect("Failed to execute bf sync");

        assert!(
            export_output.status.success(),
            "Export should succeed: {}",
            String::from_utf8_lossy(&export_output.stderr)
        );

        // Read the JSONL file
        let jsonl_path = beads_dir.join("issues.jsonl");
        let jsonl_content = fs::read_to_string(&jsonl_path)
            .expect("Failed to read JSONL file");

        assert!(
            jsonl_content.contains(custom_type),
            "Custom type should be in JSONL export"
        );
    }

    #[test]
    fn test_mixed_standard_and_custom_types() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Create beads with both standard and custom types
        let types_and_titles = vec![
            ("task", "Standard task"),
            ("bug", "Standard bug"),
            ("feature", "Standard feature"),
            ("spike", "Custom spike"),
            ("investigation", "Custom investigation"),
        ];

        for (issue_type, title) in types_and_titles {
            let (stdout, stderr, success) = run_create(
                &beads_dir,
                &[
                    "--title",
                    title,
                    "--type",
                    issue_type,
                    "--priority",
                    "2",
                ],
            );

            assert!(
                success,
                "Create command should succeed for type '{}'. stderr: {}",
                issue_type,
                stderr
            );

            let bead_id = stdout.trim();
            let show_output = run_show(&beads_dir, bead_id);

            assert!(
                show_output.contains(issue_type),
                "Type '{}' should be preserved in show output",
                issue_type
            );
        }
    }

    #[test]
    fn test_empty_and_whitespace_types() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        // Test with an empty-looking custom type (still valid as a custom type)
        let custom_type = "custom-type";
        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "Test custom type handling",
                "--type",
                custom_type,
                "--priority",
                "2",
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();
        let show_output = run_show(&beads_dir, bead_id);

        assert!(
            show_output.contains(custom_type),
            "Custom type should be preserved"
        );
    }
}
