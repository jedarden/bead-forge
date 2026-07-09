// Smoke test for bead-forge (Test bead 1: bf-52is)
// This test verifies the bead-forge system works end-to-end

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use std::fs;
    use std::process::Command;

    fn setup_test_workspace() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir(&beads_dir).unwrap();

        // Initialize empty issues.jsonl
        let issues_path = beads_dir.join("issues.jsonl");
        fs::write(&issues_path, "").unwrap();

        // Create minimal config
        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            r#"issue_prefixes:
  - bf
default_priority: 2
default_type: task
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

        temp_dir
    }

    #[test]
    fn test_bead_forge_smoke() {
        // This is a smoke test to verify bead-forge works end-to-end
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        // Step 1: Create a bead
        let create_output = Command::new("bf")
            .arg("create")
            .arg("--title")
            .arg("Smoke test bead")
            .arg("--type")
            .arg("task")
            .current_dir(workspace)
            .output()
            .expect("Failed to execute bf create");

        assert!(
            create_output.status.success(),
            "bf create failed: {}",
            String::from_utf8_lossy(&create_output.stderr)
        );

        let create_text = String::from_utf8_lossy(&create_output.stdout);
        assert!(
            create_text.contains("bf-"),
            "Create output should contain bead ID"
        );

        // Extract bead ID
        let bead_id = create_text
            .lines()
            .find(|line| line.contains("bf-"))
            .and_then(|line| line.split("bf-").nth(1))
            .map(|id| format!("bf-{}", id.trim().split_whitespace().next().unwrap_or(id)))
            .expect("Could not extract bead ID");

        println!("Created bead: {}", bead_id);

        // Step 2: Show the bead
        let show_output = Command::new("bf")
            .arg("show")
            .arg(&bead_id)
            .current_dir(workspace)
            .output()
            .expect("Failed to execute bf show");

        assert!(
            show_output.status.success(),
            "bf show failed: {}",
            String::from_utf8_lossy(&show_output.stderr)
        );

        let show_text = String::from_utf8_lossy(&show_output.stdout);
        assert!(
            show_text.contains(&bead_id) && show_text.contains("Smoke test bead"),
            "Show output should contain bead ID and title"
        );

        // Step 3: Update bead status
        let update_output = Command::new("bf")
            .arg("update")
            .arg(&bead_id)
            .arg("--status")
            .arg("in_progress")
            .current_dir(workspace)
            .output()
            .expect("Failed to execute bf update");

        assert!(
            update_output.status.success(),
            "bf update failed: {}",
            String::from_utf8_lossy(&update_output.stderr)
        );

        // Step 4: List beads
        let list_output = Command::new("bf")
            .arg("list")
            .arg("--format")
            .arg("json")
            .current_dir(workspace)
            .output()
            .expect("Failed to execute bf list");

        assert!(
            list_output.status.success(),
            "bf list failed: {}",
            String::from_utf8_lossy(&list_output.stderr)
        );

        let list_text = String::from_utf8_lossy(&list_output.stdout);
        assert!(
            list_text.contains(&bead_id),
            "List output should contain created bead"
        );

        // Step 5: Close the bead
        let close_output = Command::new("bf")
            .arg("close")
            .arg(&bead_id)
            .arg("--reason")
            .arg("Smoke test completed successfully")
            .current_dir(workspace)
            .output()
            .expect("Failed to execute bf close");

        assert!(
            close_output.status.success(),
            "bf close failed: {}",
            String::from_utf8_lossy(&close_output.stderr)
        );

        println!("Bead-forge smoke test passed!");
    }

    #[test]
    fn test_bead_forge_cli_help() {
        // Verify bf CLI help works
        let help_output = Command::new("bf")
            .arg("--help")
            .output()
            .expect("Failed to execute bf --help");

        let help_text = String::from_utf8_lossy(&help_output.stderr);
        assert!(
            help_text.contains("bead-forge") || help_text.contains("beads"),
            "Help should mention bead-forge or beads"
        );

        println!("bf --help verified");
    }

    #[test]
    fn test_bead_forge_version() {
        // Verify bf version output
        let version_output = Command::new("bf")
            .arg("--version")
            .output()
            .expect("Failed to execute bf --version");

        // --version might output to stdout or stderr depending on clap
        let version_text = String::from_utf8_lossy(&version_output.stdout);
        let version_text_err = String::from_utf8_lossy(&version_output.stderr);
        let combined = format!("{}{}", version_text, version_text_err);

        assert!(
            !combined.trim().is_empty(),
            "Version output should not be empty"
        );

        println!("bf version: {}", combined.trim());
    }
}
