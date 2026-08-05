//! Round-trip integration test for description and acceptance-criteria updates
//!
//! Tests the full create -> update -> show workflow to verify that:
//! - Description updates (both inline and file-based) are persisted correctly
//! - Acceptance-criteria updates are persisted correctly
//! - Show --json reflects the updated values
//! - Regression beads_rust#386 is prevented (update without touching update handler)
//!
//! Acceptance criteria:
//! - Test creates an issue
//! - Test updates description via --description-file
//! - Test updates acceptance-criteria via --acceptance-criteria
//! - Test verifies show --json reflects the new description and acceptance-criteria
//! - Test covers both inline and file-based description updates
//! - Test prevents the beads_rust#386 regression (update without touching update handler)

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
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

    /// Create a test bead via CLI
    fn create_test_bead(workspace: impl AsRef<std::path::Path>, title: &str) -> String {
        let bf_path = get_bf_binary();
        let result = std::process::Command::new(&bf_path)
            .arg("create")
            .arg("--title")
            .arg(title)
            .arg("--type")
            .arg("task")
            .arg("--priority")
            .arg("2")
            .current_dir(&workspace.as_ref())
            .output()
            .expect("Failed to create bead");

        assert!(
            result.status.success(),
            "bf create failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        String::from_utf8(result.stdout).unwrap().trim().to_string()
    }

    #[test]
    fn test_roundtrip_update_description_inline() {
        let (_temp, beads_dir) = setup_test_workspace();
        let workspace = beads_dir.parent().unwrap();
        let bf_path = get_bf_binary();

        // Create a test bead
        let bead_id = create_test_bead(workspace, "Test description update inline");

        // Update description with inline text
        let update_result = std::process::Command::new(&bf_path)
            .arg("update")
            .arg(&bead_id)
            .arg("--description")
            .arg("Initial inline description")
            .current_dir(workspace)
            .output()
            .expect("Failed to update bead");

        assert!(
            update_result.status.success(),
            "bf update failed: {}",
            String::from_utf8_lossy(&update_result.stderr)
        );

        // Show the bead in JSON format to verify the update
        let show_result = std::process::Command::new(&bf_path)
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

        let output = String::from_utf8(show_result.stdout).unwrap();
        let beads: Vec<serde_json::Value> =
            serde_json::from_str(&output).expect("Failed to parse JSON output");

        assert_eq!(beads.len(), 1, "Should return exactly one bead");
        let bead = &beads[0];
        assert_eq!(
            bead["description"], "Initial inline description",
            "Description should match the updated value"
        );
    }

    #[test]
    fn test_roundtrip_update_description_from_file() {
        let (_temp, beads_dir) = setup_test_workspace();
        let workspace = beads_dir.parent().unwrap();
        let bf_path = get_bf_binary();

        // Create a test bead
        let bead_id = create_test_bead(workspace, "Test description update from file");

        // Create a file with description content
        let desc_file = workspace.join("description.txt");
        let desc_content = "Multi-line description\nfrom file\nwith multiple lines";
        fs::write(&desc_file, desc_content).unwrap();

        // Update description from file
        let update_result = std::process::Command::new(&bf_path)
            .arg("update")
            .arg(&bead_id)
            .arg("--description-file")
            .arg(&desc_file)
            .current_dir(workspace)
            .output()
            .expect("Failed to update bead");

        assert!(
            update_result.status.success(),
            "bf update --description-file failed: {}",
            String::from_utf8_lossy(&update_result.stderr)
        );

        // Show the bead in JSON format to verify the update
        let show_result = std::process::Command::new(&bf_path)
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

        let output = String::from_utf8(show_result.stdout).unwrap();
        let beads: Vec<serde_json::Value> =
            serde_json::from_str(&output).expect("Failed to parse JSON output");

        assert_eq!(beads.len(), 1, "Should return exactly one bead");
        let bead = &beads[0];
        assert_eq!(
            bead["description"], desc_content,
            "Description should match the file content"
        );
    }

    #[test]
    fn test_roundtrip_update_acceptance_criteria() {
        let (_temp, beads_dir) = setup_test_workspace();
        let workspace = beads_dir.parent().unwrap();
        let bf_path = get_bf_binary();

        // Create a test bead
        let bead_id = create_test_bead(workspace, "Test acceptance criteria update");

        // Update acceptance criteria
        let ac_content = "AC 1: Should work\nAC 2: Should also work\nAC 3: Edge case handling";
        let update_result = std::process::Command::new(&bf_path)
            .arg("update")
            .arg(&bead_id)
            .arg("--acceptance-criteria")
            .arg(ac_content)
            .current_dir(workspace)
            .output()
            .expect("Failed to update bead");

        assert!(
            update_result.status.success(),
            "bf update --acceptance-criteria failed: {}",
            String::from_utf8_lossy(&update_result.stderr)
        );

        // Show the bead in JSON format to verify the update
        let show_result = std::process::Command::new(&bf_path)
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

        let output = String::from_utf8(show_result.stdout).unwrap();
        let beads: Vec<serde_json::Value> =
            serde_json::from_str(&output).expect("Failed to parse JSON output");

        assert_eq!(beads.len(), 1, "Should return exactly one bead");
        let bead = &beads[0];
        assert_eq!(
            bead["acceptance_criteria"], ac_content,
            "Acceptance criteria should match the updated value"
        );
    }

    #[test]
    fn test_roundtrip_update_both_fields() {
        let (_temp, beads_dir) = setup_test_workspace();
        let workspace = beads_dir.parent().unwrap();
        let bf_path = get_bf_binary();

        // Create a test bead
        let bead_id = create_test_bead(workspace, "Test both fields update");

        // Create description file
        let desc_file = workspace.join("desc.txt");
        let desc_content = "Comprehensive description from file";
        fs::write(&desc_file, desc_content).unwrap();

        // Update both description and acceptance criteria in one command
        let ac_content = "AC 1: Feature works\nAC 2: Tests pass";
        let update_result = std::process::Command::new(&bf_path)
            .arg("update")
            .arg(&bead_id)
            .arg("--description-file")
            .arg(&desc_file)
            .arg("--acceptance-criteria")
            .arg(ac_content)
            .current_dir(workspace)
            .output()
            .expect("Failed to update bead");

        assert!(
            update_result.status.success(),
            "bf update with both fields failed: {}",
            String::from_utf8_lossy(&update_result.stderr)
        );

        // Show the bead in JSON format to verify both updates
        let show_result = std::process::Command::new(&bf_path)
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

        let output = String::from_utf8(show_result.stdout).unwrap();
        let beads: Vec<serde_json::Value> =
            serde_json::from_str(&output).expect("Failed to parse JSON output");

        assert_eq!(beads.len(), 1, "Should return exactly one bead");
        let bead = &beads[0];
        assert_eq!(
            bead["description"], desc_content,
            "Description should match the file content"
        );
        assert_eq!(
            bead["acceptance_criteria"], ac_content,
            "Acceptance criteria should match the updated value"
        );
    }

    #[test]
    fn test_regression_beads_rust_386_update_persists_to_database() {
        // Regression test for beads_rust#386: ensure that updates actually
        // persist to the database and are reflected in subsequent show commands.
        // The bug was that updates could complete without touching the update
        // handler, leaving the database unchanged.
        let (_temp, beads_dir) = setup_test_workspace();
        let workspace = beads_dir.parent().unwrap();
        let bf_path = get_bf_binary();

        // Create a test bead
        let bead_id = create_test_bead(workspace, "Test regression beads_rust#386");

        // Update description
        let desc_content = "Regression test description";
        let update_result = std::process::Command::new(&bf_path)
            .arg("update")
            .arg(&bead_id)
            .arg("--description")
            .arg(desc_content)
            .current_dir(workspace)
            .output()
            .expect("Failed to update bead");

        assert!(
            update_result.status.success(),
            "bf update failed: {}",
            String::from_utf8_lossy(&update_result.stderr)
        );

        // Verify via CLI show (this catches the regression if the update handler was bypassed)
        let show_result = std::process::Command::new(&bf_path)
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

        let cli_output = String::from_utf8(show_result.stdout).unwrap();
        let cli_beads: Vec<serde_json::Value> =
            serde_json::from_str(&cli_output).expect("Failed to parse CLI JSON output");

        // Also verify directly against the database to double-check
        let db_path = beads_dir.join("beads.db");
        let storage = bead_forge::storage::Storage::open(&db_path).unwrap();
        let db_issue = storage
            .get_issue(&bead_id)
            .expect("Failed to query database")
            .expect("Issue should exist in database");

        // Both CLI output and database should have the updated description
        assert_eq!(
            cli_beads[0]["description"], desc_content,
            "CLI show output should have updated description"
        );
        assert_eq!(
            db_issue.description,
            Some(desc_content.to_string()),
            "Database should have updated description"
        );

        // Update acceptance criteria
        let ac_content = "AC: Regression test passes";
        let update_result2 = std::process::Command::new(&bf_path)
            .arg("update")
            .arg(&bead_id)
            .arg("--acceptance-criteria")
            .arg(ac_content)
            .current_dir(workspace)
            .output()
            .expect("Failed to update bead");

        assert!(
            update_result2.status.success(),
            "bf update (acceptance criteria) failed: {}",
            String::from_utf8_lossy(&update_result2.stderr)
        );

        // Verify both fields after second update
        let show_result2 = std::process::Command::new(&bf_path)
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
            .current_dir(workspace)
            .output()
            .expect("Failed to run bf show");

        let cli_output2 = String::from_utf8(show_result2.stdout).unwrap();
        let cli_beads2: Vec<serde_json::Value> =
            serde_json::from_str(&cli_output2).expect("Failed to parse CLI JSON output");

        let db_issue2 = storage.get_issue(&bead_id).unwrap().unwrap();

        assert_eq!(
            cli_beads2[0]["description"], desc_content,
            "Description should persist after second update"
        );
        assert_eq!(
            cli_beads2[0]["acceptance_criteria"], ac_content,
            "Acceptance criteria should be updated"
        );
        assert_eq!(
            db_issue2.description,
            Some(desc_content.to_string()),
            "Database description should persist"
        );
        assert_eq!(
            db_issue2.acceptance_criteria,
            Some(ac_content.to_string()),
            "Database acceptance criteria should be updated"
        );
    }

    #[test]
    fn test_roundtrip_sequential_updates_override_previous() {
        let (_temp, beads_dir) = setup_test_workspace();
        let workspace = beads_dir.parent().unwrap();
        let bf_path = get_bf_binary();

        // Create a test bead
        let bead_id = create_test_bead(workspace, "Test sequential updates");

        // First update
        let update1 = std::process::Command::new(&bf_path)
            .arg("update")
            .arg(&bead_id)
            .arg("--description")
            .arg("First description")
            .arg("--acceptance-criteria")
            .arg("First AC")
            .current_dir(workspace)
            .output()
            .expect("Failed to run first update");

        assert!(update1.status.success(), "First update failed");

        // Second update (should override)
        let update2 = std::process::Command::new(&bf_path)
            .arg("update")
            .arg(&bead_id)
            .arg("--description")
            .arg("Second description")
            .arg("--acceptance-criteria")
            .arg("Second AC")
            .current_dir(workspace)
            .output()
            .expect("Failed to run second update");

        assert!(update2.status.success(), "Second update failed");

        // Verify final state
        let show_result = std::process::Command::new(&bf_path)
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
            .current_dir(workspace)
            .output()
            .expect("Failed to run bf show");

        let output = String::from_utf8(show_result.stdout).unwrap();
        let beads: Vec<serde_json::Value> =
            serde_json::from_str(&output).expect("Failed to parse JSON output");

        let bead = &beads[0];
        assert_eq!(bead["description"], "Second description");
        assert_eq!(bead["acceptance_criteria"], "Second AC");
    }

    #[test]
    fn test_description_file_and_inline_conflict() {
        // Verify that --description and --description-file are mutually exclusive
        // (clap's conflicts_with should enforce this)
        let (_temp, beads_dir) = setup_test_workspace();
        let workspace = beads_dir.parent().unwrap();
        let bf_path = get_bf_binary();

        let bead_id = create_test_bead(workspace, "Test conflict handling");

        let desc_file = workspace.join("desc.txt");
        fs::write(&desc_file, "File content").unwrap();

        // Attempting to use both flags should fail
        let result = std::process::Command::new(&bf_path)
            .arg("update")
            .arg(&bead_id)
            .arg("--description")
            .arg("Inline content")
            .arg("--description-file")
            .arg(&desc_file)
            .current_dir(workspace)
            .output()
            .expect("Failed to run command");

        // Should fail due to argument conflict
        assert!(
            !result.status.success(),
            "Should fail when both --description and --description-file are provided"
        );
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            stderr.contains("conflict")
                || stderr.contains("cannot be used with")
                || stderr.contains("one of"),
            "Error should mention argument conflict"
        );
    }
}
