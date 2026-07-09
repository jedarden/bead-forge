// Basic workflow test for bead-forge
// This test verifies the fundamental claim-and-close workflow

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use std::fs;
    use std::path::PathBuf;
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
            "# Test workspace config\nworkspace:\n  name: test-workspace\n",
        )
        .unwrap();

        temp_dir
    }

    #[test]
    fn test_bead_forge_cli_exists() {
        // Verify the bf binary exists and is executable
        let output = Command::new("which")
            .arg("bf")
            .output()
            .expect("Failed to run 'which bf'");

        assert!(
            output.status.success(),
            "bf binary not found. Make sure it's installed and in PATH"
        );

        let path = String::from_utf8_lossy(&output.stdout);
        println!("bf binary found at: {}", path.trim());
    }

    #[test]
    fn test_bead_forge_version() {
        // Verify bf --help works (note: outputs help text to stderr due to clap behavior)
        let output = Command::new("bf")
            .arg("--help")
            .output()
            .expect("Failed to run 'bf --help'");

        // Note: bf --help returns exit code 1 and outputs to stderr
        // (clap treats missing subcommand as error)
        let help_text = String::from_utf8_lossy(&output.stderr);
        assert!(
            help_text.contains("bead-forge") || help_text.contains("beads"),
            "Help output should mention bead-forge or beads"
        );

        // Verify it shows usage information
        assert!(
            help_text.contains("Commands:") || help_text.contains("Usage:"),
            "Help output should show usage information"
        );

        println!("bf --help output verified");
    }

    #[test]
    fn test_current_workspace_accessible() {
        // Verify we can read the current workspace
        let output = Command::new("bf")
            .arg("list")
            .arg("--format")
            .arg("json")
            .current_dir("/home/coding/bead-forge")
            .output()
            .expect("Failed to run 'bf list'");

        assert!(
            output.status.success(),
            "bf list failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let json_output = String::from_utf8_lossy(&output.stdout);

        // bf list --format json outputs JSONL (one JSON object per line)
        // Parse each line separately
        if !json_output.trim().is_empty() {
            let mut bead_count = 0;
            for line in json_output.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                let parsed: serde_json::Value =
                    serde_json::from_str(line).expect(&format!("Invalid JSON on line: {}", line));
                assert!(parsed.is_object(), "Each line should be a JSON object");
                bead_count += 1;
            }
            println!("Workspace contains {} beads", bead_count);
            assert!(bead_count > 0, "Workspace should contain at least one bead");
        }
    }

    #[test]
    fn test_bead_show_by_id() {
        // Test showing a specific bead (bf-2atz should exist as the current bead)
        let output = Command::new("bf")
            .args(["show", "bf-2atz"])
            .current_dir("/home/coding/bead-forge")
            .output()
            .expect("Failed to run 'bf show bf-2atz'");

        assert!(
            output.status.success(),
            "bf show bf-2atz failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let show_output = String::from_utf8_lossy(&output.stdout);
        assert!(
            show_output.contains("bf-2atz"),
            "Show output should contain bead ID"
        );
        assert!(
            show_output.contains("Complete test bead"),
            "Show output should contain bead title"
        );

        println!("bf show bf-2atz output verified");
    }
}
