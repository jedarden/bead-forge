//! Tests for special character handling in bead-forge.
//!
//! Tests that bead-forge correctly handles:
//! - Special characters in titles: @#$%^&*()
//! - HTML tags and entities in descriptions
//! - Unicode and emoji characters
//! - Quotes and backslashes

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::TempDir;

    /// Create a temporary test workspace with bf configuration
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
        let mut cmd = Command::new("bf");
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
        let output = Command::new("bf")
            .arg("--workspace")
            .arg(beads_dir)
            .arg("show")
            .arg(bead_id)
            .output()
            .expect("Failed to execute bf show");

        String::from_utf8_lossy(&output.stdout).to_string()
    }

    #[test]
    fn test_special_chars_in_title() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let special_title = "Test special chars: @#$%^&*()";
        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                special_title,
                "--type",
                "task",
                "--priority",
                "2",
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);
        assert!(!stdout.is_empty(), "Output should contain bead ID");

        let bead_id = stdout.trim();
        println!("Created bead with ID: {}", bead_id);

        // Verify the bead shows the special characters correctly
        let show_output = run_show(&beads_dir, bead_id);
        assert!(
            show_output.contains("@#$%^&*()"),
            "Special characters should be preserved in show output"
        );
        assert!(
            show_output.contains("Test special chars:"),
            "Title prefix should be preserved"
        );
    }

    #[test]
    fn test_html_entities_in_description() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let description = "Description with <html> tags &amp; entities";
        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                "HTML test",
                "--type",
                "task",
                "--priority",
                "2",
                "--description",
                description,
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();
        let show_output = run_show(&beads_dir, bead_id);

        // HTML entities should be preserved
        assert!(
            show_output.contains("<html>"),
            "HTML tags should be preserved"
        );
        assert!(
            show_output.contains("&amp;"),
            "HTML entities should be preserved"
        );
    }

    #[test]
    fn test_combined_special_chars_and_html() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let title = "Test special chars: @#$%^&*()";
        let description = "Description with <html> tags &amp; entities";
        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                title,
                "--type",
                "task",
                "--priority",
                "2",
                "--description",
                description,
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();
        let show_output = run_show(&beads_dir, bead_id);

        assert!(
            show_output.contains("@#$%^&*()"),
            "Special chars in title should be preserved"
        );
        assert!(
            show_output.contains("<html>"),
            "HTML tags in description should be preserved"
        );
        assert!(
            show_output.contains("&amp;"),
            "HTML entities in description should be preserved"
        );
    }

    #[test]
    fn test_unicode_emoji_characters() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let title = "Test with emoji 🎉 and unicode: café, naïve";
        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                title,
                "--type",
                "task",
                "--priority",
                "2",
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();
        let show_output = run_show(&beads_dir, bead_id);

        assert!(
            show_output.contains("🎉"),
            "Emoji should be preserved"
        );
        assert!(
            show_output.contains("café"),
            "Unicode characters should be preserved"
        );
    }

    #[test]
    fn test_quotes_and_backslashes() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let title = "Test with \"quotes\" and 'apostrophes'";
        let description = "Path: C:\\Users\\test and backslash \\";
        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                title,
                "--type",
                "task",
                "--priority",
                "2",
                "--description",
                description,
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();
        let show_output = run_show(&beads_dir, bead_id);

        assert!(
            show_output.contains("\"quotes\""),
            "Double quotes should be preserved"
        );
        assert!(
            show_output.contains("'apostrophes'"),
            "Single quotes should be preserved"
        );
        assert!(
            show_output.contains("C:\\Users\\test"),
            "Backslashes should be preserved"
        );
    }

    #[test]
    fn test_json_roundtrip_with_special_chars() {
        let (_temp_dir, beads_dir) = setup_test_workspace();

        let title = "Test special chars: @#$%^&*()";
        let description = "Description with <html> tags &amp; entities";
        let (stdout, stderr, success) = run_create(
            &beads_dir,
            &[
                "--title",
                title,
                "--type",
                "task",
                "--priority",
                "2",
                "--description",
                description,
            ],
        );

        assert!(success, "Create command should succeed. stderr: {}", stderr);

        let bead_id = stdout.trim();

        // Export to JSONL
        let export_output = Command::new("bf")
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
            jsonl_content.contains("@#$%^&*()"),
            "Special characters should be in JSONL export"
        );
        assert!(
            jsonl_content.contains("<html>"),
            "HTML tags should be in JSONL export"
        );
        assert!(
            jsonl_content.contains("&amp;"),
            "HTML entities should be in JSONL export"
        );
    }
}
