// Basic functionality test for bead-forge (Test Bead A: bf-23vs)
// This test verifies the fundamental CRUD operations work correctly

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
    fn test_basic_bead_lifecycle() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        // Test 1: Create a bead
        let create_output = Command::new("bf")
            .arg("create")
            .arg("--title")
            .arg("Test bead for basic functionality")
            .arg("--type")
            .arg("task")
            .arg("--priority")
            .arg("1")
            .current_dir(workspace)
            .output()
            .expect("Failed to create bead");

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

        // Extract the bead ID
        let bead_id = create_text
            .lines()
            .find(|line| line.contains("bf-"))
            .and_then(|line| line.split("bf-").nth(1))
            .map(|id| format!("bf-{}", id.trim().split_whitespace().next().unwrap_or(id)))
            .expect("Could not extract bead ID from create output");

        println!("Created bead: {}", bead_id);

        // Test 2: List beads
        let list_output = Command::new("bf")
            .arg("list")
            .arg("--format")
            .arg("json")
            .current_dir(workspace)
            .output()
            .expect("Failed to list beads");

        assert!(
            list_output.status.success(),
            "bf list failed: {}",
            String::from_utf8_lossy(&list_output.stderr)
        );

        let list_text = String::from_utf8_lossy(&list_output.stdout);
        assert!(
            list_text.contains(&bead_id),
            "List output should contain created bead ID"
        );
        assert!(
            list_text.contains("Test bead for basic functionality"),
            "List output should contain bead title"
        );

        // Test 3: Show specific bead
        let show_output = Command::new("bf")
            .arg("show")
            .arg(&bead_id)
            .current_dir(workspace)
            .output()
            .expect("Failed to show bead");

        assert!(
            show_output.status.success(),
            "bf show failed: {}",
            String::from_utf8_lossy(&show_output.stderr)
        );

        let show_text = String::from_utf8_lossy(&show_output.stdout);
        assert!(
            show_text.contains(&bead_id),
            "Show output should contain bead ID"
        );
        assert!(
            show_text.contains("Test bead for basic functionality"),
            "Show output should contain bead title"
        );

        // Test 4: Update bead status
        let update_output = Command::new("bf")
            .arg("update")
            .arg(&bead_id)
            .arg("--status")
            .arg("in_progress")
            .current_dir(workspace)
            .output()
            .expect("Failed to update bead");

        assert!(
            update_output.status.success(),
            "bf update failed: {}",
            String::from_utf8_lossy(&update_output.stderr)
        );

        // Verify the status change
        let show_after_update = Command::new("bf")
            .arg("show")
            .arg(&bead_id)
            .current_dir(workspace)
            .output()
            .expect("Failed to show bead after update");

        let show_text_after = String::from_utf8_lossy(&show_after_update.stdout);
        assert!(
            show_text_after.contains("in_progress"),
            "Show output should show updated status"
        );

        // Test 5: Close bead
        let close_output = Command::new("bf")
            .arg("close")
            .arg(&bead_id)
            .arg("--reason")
            .arg("Test completed successfully")
            .current_dir(workspace)
            .output()
            .expect("Failed to close bead");

        assert!(
            close_output.status.success(),
            "bf close failed: {}",
            String::from_utf8_lossy(&close_output.stderr)
        );

        // Verify the bead is closed
        let show_after_close = Command::new("bf")
            .arg("show")
            .arg(&bead_id)
            .current_dir(workspace)
            .output()
            .expect("Failed to show bead after close");

        let show_text_close = String::from_utf8_lossy(&show_after_close.stdout);
        assert!(
            show_text_close.contains("closed"),
            "Show output should show closed status"
        );
        assert!(
            show_text_close.contains("Test completed successfully"),
            "Show output should show close reason"
        );

        println!("Basic bead lifecycle test passed for bead: {}", bead_id);
    }

    #[test]
    fn test_bead_labels() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        // Create a bead with labels
        let create_output = Command::new("bf")
            .arg("create")
            .arg("--title")
            .arg("Labeled test bead")
            .arg("--label")
            .arg("test-label-1")
            .arg("--label")
            .arg("test-label-2")
            .current_dir(workspace)
            .output()
            .expect("Failed to create bead with labels");

        assert!(
            create_output.status.success(),
            "bf create with labels failed: {}",
            String::from_utf8_lossy(&create_output.stderr)
        );

        let create_text = String::from_utf8_lossy(&create_output.stdout);
        let bead_id = create_text
            .lines()
            .find(|line| line.contains("bf-"))
            .and_then(|line| line.split("bf-").nth(1))
            .map(|id| format!("bf-{}", id.trim().split_whitespace().next().unwrap_or(id)))
            .expect("Could not extract bead ID");

        // Show the bead to verify labels
        let show_output = Command::new("bf")
            .arg("show")
            .arg(&bead_id)
            .current_dir(workspace)
            .output()
            .expect("Failed to show bead");

        let show_text = String::from_utf8_lossy(&show_output.stdout);
        assert!(
            show_text.contains("test-label-1"),
            "Show output should contain first label"
        );
        assert!(
            show_text.contains("test-label-2"),
            "Show output should contain second label"
        );

        println!("Bead labels test passed for bead: {}", bead_id);
    }

    #[test]
    fn test_empty_workspace_list() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        // List should work even on empty workspace
        let list_output = Command::new("bf")
            .arg("list")
            .current_dir(workspace)
            .output()
            .expect("Failed to list beads in empty workspace");

        assert!(
            list_output.status.success(),
            "bf list should succeed on empty workspace: {}",
            String::from_utf8_lossy(&list_output.stderr)
        );

        println!("Empty workspace list test passed");
    }
}
