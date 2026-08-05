// Test: P0 Priority Bead Creation with Multiple Labels
// Test Bead: bf-3u25fp
//
// This test verifies that the `bf create` command correctly handles:
// - P0 (Critical) priority creation
// - Multiple labels specified via repeated --label flags
// - Proper storage and retrieval of both priority and labels

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

        // Create minimal config
        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            r#"issue_prefixes:
  - bf
default_priority: 2
default_type: task
claim_ttl_minutes: 30
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

    /// Extract bead ID from command output
    fn extract_bead_id(output: &str) -> String {
        output
            .lines()
            .find(|line| line.contains("bf-"))
            .and_then(|line| line.split("bf-").nth(1))
            .map(|id| format!("bf-{}", id.trim().split_whitespace().next().unwrap_or(id)))
            .expect("Could not extract bead ID from output")
    }

    #[test]
    fn test_p0_creation_with_single_label() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        // Create P0 bead with single label
        let create_output = Command::new(bf_binary())
            .arg("create")
            .arg("--title")
            .arg("P0 test with single label")
            .arg("--type")
            .arg("task")
            .arg("--priority")
            .arg("0")
            .arg("--label")
            .arg("critical")
            .current_dir(workspace)
            .output()
            .expect("Failed to create bead");

        assert!(
            create_output.status.success(),
            "bf create failed: {}",
            String::from_utf8_lossy(&create_output.stderr)
        );

        let create_text = String::from_utf8_lossy(&create_output.stdout);
        let bead_id = extract_bead_id(&create_text);

        // Verify bead exists and has correct priority and label
        let show_output = Command::new(bf_binary())
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
            .current_dir(workspace)
            .output()
            .expect("Failed to show bead");

        assert!(
            show_output.status.success(),
            "bf show failed: {}",
            String::from_utf8_lossy(&show_output.stderr)
        );

        let show_json = String::from_utf8_lossy(&show_output.stdout);
        let bead: serde_json::Value = serde_json::from_str(&show_json)
            .expect("Failed to parse show output as JSON");

        // Verify priority is P0 (0)
        let priority = bead.get("priority").and_then(|p| p.as_i64());
        assert_eq!(priority, Some(0), "Priority should be 0 (P0)");

        // Verify label is present
        let labels = bead.get("labels").and_then(|l| l.as_array());
        assert!(labels.is_some(), "Labels field should be present");
        let labels = labels.unwrap();
        assert_eq!(labels.len(), 1, "Should have exactly 1 label");
        assert_eq!(labels[0].as_str(), Some("critical"), "Label should be 'critical'");
    }

    #[test]
    fn test_p0_creation_with_multiple_labels() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        // Create P0 bead with multiple labels
        let create_output = Command::new(bf_binary())
            .arg("create")
            .arg("--title")
            .arg("P0 test with multiple labels")
            .arg("--type")
            .arg("task")
            .arg("--priority")
            .arg("0")
            .arg("--label")
            .arg("critical")
            .arg("--label")
            .arg("urgent")
            .arg("--label")
            .arg("security")
            .current_dir(workspace)
            .output()
            .expect("Failed to create bead");

        assert!(
            create_output.status.success(),
            "bf create failed: {}",
            String::from_utf8_lossy(&create_output.stderr)
        );

        let create_text = String::from_utf8_lossy(&create_output.stdout);
        let bead_id = extract_bead_id(&create_text);

        // Verify bead exists and has correct priority and all labels
        let show_output = Command::new(bf_binary())
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
            .current_dir(workspace)
            .output()
            .expect("Failed to show bead");

        assert!(
            show_output.status.success(),
            "bf show failed: {}",
            String::from_utf8_lossy(&show_output.stderr)
        );

        let show_json = String::from_utf8_lossy(&show_output.stdout);
        let bead: serde_json::Value = serde_json::from_str(&show_json)
            .expect("Failed to parse show output as JSON");

        // Verify priority is P0 (0)
        let priority = bead.get("priority").and_then(|p| p.as_i64());
        assert_eq!(priority, Some(0), "Priority should be 0 (P0)");

        // Verify all labels are present
        let labels = bead.get("labels").and_then(|l| l.as_array());
        assert!(labels.is_some(), "Labels field should be present");
        let labels = labels.unwrap();
        assert_eq!(labels.len(), 3, "Should have exactly 3 labels");

        let label_set: std::collections::HashSet<_> = labels
            .iter()
            .filter_map(|l| l.as_str())
            .collect();

        assert!(label_set.contains("critical"), "Should contain 'critical' label");
        assert!(label_set.contains("urgent"), "Should contain 'urgent' label");
        assert!(label_set.contains("security"), "Should contain 'security' label");
    }

    #[test]
    fn test_p0_epic_with_multiple_labels() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        // Create P0 epic with multiple labels
        let create_output = Command::new(bf_binary())
            .arg("create")
            .arg("--title")
            .arg("P0 epic with multiple labels")
            .arg("--type")
            .arg("epic")
            .arg("--priority")
            .arg("0")
            .arg("--label")
            .arg("critical")
            .arg("--label")
            .arg("high-priority")
            .arg("--label")
            .arg("feature")
            .arg("--label")
            .arg("backend")
            .current_dir(workspace)
            .output()
            .expect("Failed to create bead");

        assert!(
            create_output.status.success(),
            "bf create failed: {}",
            String::from_utf8_lossy(&create_output.stderr)
        );

        let create_text = String::from_utf8_lossy(&create_output.stdout);
        let bead_id = extract_bead_id(&create_text);

        // Verify bead exists and has correct properties
        let show_output = Command::new(bf_binary())
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
            .current_dir(workspace)
            .output()
            .expect("Failed to show bead");

        assert!(
            show_output.status.success(),
            "bf show failed: {}",
            String::from_utf8_lossy(&show_output.stderr)
        );

        let show_json = String::from_utf8_lossy(&show_output.stdout);
        let bead: serde_json::Value = serde_json::from_str(&show_json)
            .expect("Failed to parse show output as JSON");

        // Verify type is epic
        let issue_type = bead.get("type").and_then(|t| t.as_str());
        assert_eq!(issue_type, Some("epic"), "Type should be 'epic'");

        // Verify priority is P0 (0)
        let priority = bead.get("priority").and_then(|p| p.as_i64());
        assert_eq!(priority, Some(0), "Priority should be 0 (P0)");

        // Verify all labels are present
        let labels = bead.get("labels").and_then(|l| l.as_array());
        assert!(labels.is_some(), "Labels field should be present");
        let labels = labels.unwrap();
        assert_eq!(labels.len(), 4, "Should have exactly 4 labels");

        let label_set: std::collections::HashSet<_> = labels
            .iter()
            .filter_map(|l| l.as_str())
            .collect();

        assert!(label_set.contains("critical"), "Should contain 'critical' label");
        assert!(label_set.contains("high-priority"), "Should contain 'high-priority' label");
        assert!(label_set.contains("feature"), "Should contain 'feature' label");
        assert!(label_set.contains("backend"), "Should contain 'backend' label");
    }

    #[test]
    fn test_p0_creation_json_output() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        // Create P0 bead with multiple labels using JSON output
        let create_output = Command::new(bf_binary())
            .arg("create")
            .arg("--title")
            .arg("P0 JSON test")
            .arg("--priority")
            .arg("0")
            .arg("--label")
            .arg("critical")
            .arg("--label")
            .arg("urgent")
            .arg("--json")
            .current_dir(workspace)
            .output()
            .expect("Failed to create bead");

        assert!(
            create_output.status.success(),
            "bf create failed: {}",
            String::from_utf8_lossy(&create_output.stderr)
        );

        let create_json = String::from_utf8_lossy(&create_output.stdout);
        let result: serde_json::Value = serde_json::from_str(&create_json)
            .expect("Failed to parse create output as JSON");

        // Verify JSON output structure
        let bead_id = result.get("id").and_then(|i| i.as_str());
        assert!(bead_id.is_some(), "JSON output should contain 'id' field");
        assert!(bead_id.unwrap().starts_with("bf-"), "ID should start with 'bf-'");

        let priority = result.get("priority").and_then(|p| p.as_i64());
        assert_eq!(priority, Some(0), "JSON output should show priority as 0");

        let labels = result.get("labels").and_then(|l| l.as_array());
        assert!(labels.is_some(), "JSON output should contain 'labels' field");
        let labels = labels.unwrap();
        assert_eq!(labels.len(), 2, "JSON output should show 2 labels");

        let label_set: std::collections::HashSet<_> = labels
            .iter()
            .filter_map(|l| l.as_str())
            .collect();

        assert!(label_set.contains("critical"), "Should contain 'critical' label");
        assert!(label_set.contains("urgent"), "Should contain 'urgent' label");
    }

    #[test]
    fn test_p0_with_special_character_labels() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        // Create P0 bead with special character labels
        let create_output = Command::new(bf_binary())
            .arg("create")
            .arg("--title")
            .arg("P0 with special labels")
            .arg("--priority")
            .arg("0")
            .arg("--label")
            .arg("high-priority")
            .arg("--label")
            .arg("needs-review")
            .arg("--label")
            .arg("API:breaking")
            .arg("--label")
            .arg("bug:security")
            .current_dir(workspace)
            .output()
            .expect("Failed to create bead");

        assert!(
            create_output.status.success(),
            "bf create failed: {}",
            String::from_utf8_lossy(&create_output.stderr)
        );

        let create_text = String::from_utf8_lossy(&create_output.stdout);
        let bead_id = extract_bead_id(&create_text);

        // Verify special character labels are preserved
        let show_output = Command::new(bf_binary())
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
            .current_dir(workspace)
            .output()
            .expect("Failed to show bead");

        let show_json = String::from_utf8_lossy(&show_output.stdout);
        let bead: serde_json::Value = serde_json::from_str(&show_json)
            .expect("Failed to parse show output as JSON");

        let labels = bead.get("labels").and_then(|l| l.as_array()).unwrap();
        assert_eq!(labels.len(), 4, "Should have 4 special character labels");

        let label_set: std::collections::HashSet<_> = labels
            .iter()
            .filter_map(|l| l.as_str())
            .collect();

        assert!(label_set.contains("high-priority"), "Should contain 'high-priority'");
        assert!(label_set.contains("needs-review"), "Should contain 'needs-review'");
        assert!(label_set.contains("API:breaking"), "Should contain 'API:breaking'");
        assert!(label_set.contains("bug:security"), "Should contain 'bug:security'");
    }

    #[test]
    fn test_p0_label_persistence() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        // Create P0 bead with labels
        let create_output = Command::new(bf_binary())
            .arg("create")
            .arg("--title")
            .arg("P0 persistence test")
            .arg("--priority")
            .arg("0")
            .arg("--label")
            .arg("critical")
            .arg("--label")
            .arg("test-label")
            .current_dir(workspace)
            .output()
            .expect("Failed to create bead");

        let create_text = String::from_utf8_lossy(&create_output.stdout);
        let bead_id = extract_bead_id(&create_text);

        // Verify labels persist
        let show_output = Command::new(bf_binary())
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
            .current_dir(workspace)
            .output()
            .expect("Failed to show bead");

        let show_json = String::from_utf8_lossy(&show_output.stdout);
        let bead: serde_json::Value = serde_json::from_str(&show_json)
            .expect("Failed to parse show output as JSON");

        let labels = bead.get("labels").and_then(|l| l.as_array()).unwrap();
        assert_eq!(labels.len(), 2, "Labels should persist");

        // Update bead to ensure labels are not lost
        let update_output = Command::new(bf_binary())
            .arg("update")
            .arg(&bead_id)
            .arg("--title")
            .arg("Updated P0 persistence test")
            .current_dir(workspace)
            .output()
            .expect("Failed to update bead");

        assert!(
            update_output.status.success(),
            "bf update failed: {}",
            String::from_utf8_lossy(&update_output.stderr)
        );

        // Verify labels still exist after update
        let show_output2 = Command::new(bf_binary())
            .arg("show")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
            .current_dir(workspace)
            .output()
            .expect("Failed to show bead after update");

        let show_json2 = String::from_utf8_lossy(&show_output2.stdout);
        let bead2: serde_json::Value = serde_json::from_str(&show_json2)
            .expect("Failed to parse show output after update");

        let labels2 = bead2.get("labels").and_then(|l| l.as_array()).unwrap();
        assert_eq!(labels2.len(), 2, "Labels should persist after update");

        // Verify priority is still P0
        let priority2 = bead2.get("priority").and_then(|p| p.as_i64());
        assert_eq!(priority2, Some(0), "Priority should still be 0 after update");
    }
}
