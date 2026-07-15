// Basic workflow test for bead-forge
// This test verifies the fundamental create/list/show workflow against the
// freshly-built binary in an isolated temp workspace — never against the
// developer machine's installed bf or real workspaces.

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    /// Resolve the freshly-built bf binary — never the system-installed one.
    fn bf_binary() -> String {
        std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
    }

    fn setup_test_workspace() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir(&beads_dir).unwrap();

        // Initialize empty issues.jsonl
        let issues_path = beads_dir.join("issues.jsonl");
        fs::write(&issues_path, "").unwrap();

        // Create minimal config (real schema: issue_prefixes)
        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            "# Test workspace config\nissue_prefixes:\n- test\n",
        )
        .unwrap();

        temp_dir
    }

    /// Create a bead in the workspace and return its ID (create prints the bare ID).
    fn create_bead(workspace: &TempDir, title: &str) -> String {
        let output = Command::new(bf_binary())
            .args(["create", "--title", title])
            .current_dir(workspace.path())
            .output()
            .expect("Failed to run bf create");
        assert!(
            output.status.success(),
            "bf create failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    #[test]
    fn test_bead_forge_cli_exists() {
        // Verify the built bf binary exists and is executable.
        let output = Command::new(bf_binary())
            .arg("--version")
            .output()
            .expect("Failed to execute the built bf binary");

        assert!(
            output.status.success(),
            "bf --version failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        println!(
            "bf binary runs: {}",
            String::from_utf8_lossy(&output.stdout).trim()
        );
    }

    #[test]
    fn test_bead_forge_version() {
        // Verify bf --help works.
        let output = Command::new(bf_binary())
            .arg("--help")
            .output()
            .expect("Failed to run 'bf --help'");

        // Help may land on stdout (clap --help, exit 0) or stderr depending
        // on invocation; accept either stream.
        let help_text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
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
        // Create an isolated workspace with one bead, then list it as JSON.
        let workspace = setup_test_workspace();
        let bead_id = create_bead(&workspace, "Workspace list smoke bead");

        let output = Command::new(bf_binary())
            .arg("list")
            .arg("--format")
            .arg("json")
            .current_dir(workspace.path())
            .output()
            .expect("Failed to run 'bf list'");

        assert!(
            output.status.success(),
            "bf list failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let json_output = String::from_utf8_lossy(&output.stdout);

        // bf list --format json outputs JSONL (one JSON object per line)
        let mut bead_count = 0;
        let mut found_created = false;
        for line in json_output.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let parsed: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|_| panic!("Invalid JSON on line: {}", line));
            assert!(parsed.is_object(), "Each line should be a JSON object");
            if parsed["id"] == bead_id.as_str() {
                found_created = true;
            }
            bead_count += 1;
        }
        println!("Workspace contains {} beads", bead_count);
        assert!(bead_count > 0, "Workspace should contain at least one bead");
        assert!(found_created, "Created bead should appear in list output");
    }

    #[test]
    fn test_bead_show_by_id() {
        // Create a bead in an isolated workspace, then show it by its returned ID.
        let workspace = setup_test_workspace();
        let title = "Complete test bead";
        let bead_id = create_bead(&workspace, title);

        let output = Command::new(bf_binary())
            .args(["show", &bead_id])
            .current_dir(workspace.path())
            .output()
            .unwrap_or_else(|e| panic!("Failed to run 'bf show {}': {}", bead_id, e));

        assert!(
            output.status.success(),
            "bf show {} failed: {}",
            bead_id,
            String::from_utf8_lossy(&output.stderr)
        );

        let show_output = String::from_utf8_lossy(&output.stdout);
        assert!(
            show_output.contains(&bead_id),
            "Show output should contain bead ID"
        );
        assert!(
            show_output.contains(title),
            "Show output should contain bead title"
        );

        println!("bf show {} output verified", bead_id);
    }
}
