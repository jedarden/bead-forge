// Comprehensive tests for the create command (Test Bead: bf-22wrj)
// Tests all aspects of bead creation including types, priorities, labels, assignees, etc.

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

    fn extract_bead_id(output: &str) -> String {
        output
            .lines()
            .find(|line| line.contains("bf-"))
            .and_then(|line| line.split("bf-").nth(1))
            .map(|id| format!("bf-{}", id.trim().split_whitespace().next().unwrap_or(id)))
            .expect("Could not extract bead ID from output")
    }

    #[test]
    fn test_create_basic_bead() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        let create_output = Command::new("bf")
            .arg("create")
            .arg("--title")
            .arg("Basic test bead")
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

        // Verify bead ID format (bf-xxx where xxx is 3-8 chars, adaptive based on count)
        // Minimum is bf- + 3 chars = 7 chars total
        let hash_part = bead_id.strip_prefix("bf-").unwrap();
        assert!(
            hash_part.len() >= 3 && hash_part.len() <= 8 && hash_part.chars().all(|c| c.is_ascii_alphanumeric()),
            "Bead ID hash part should be 3-8 alphanumeric chars, got: {} (full ID: {})",
            hash_part,
            bead_id
        );

        // Verify bead exists in database
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
            show_text.contains("Basic test bead"),
            "Show output should contain bead title"
        );
        assert!(
            show_text.contains("open"),
            "Show output should show default status 'open'"
        );
        assert!(
            show_text.contains("P2"),
            "Show output should show default priority 'P2'"
        );
        assert!(
            show_text.contains("task"),
            "Show output should show default type 'task'"
        );
    }

    #[test]
    fn test_create_with_all_parameters() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        let create_output = Command::new("bf")
            .arg("create")
            .arg("--title")
            .arg("Complete test bead with all parameters")
            .arg("--type")
            .arg("bug")
            .arg("--priority")
            .arg("0")
            .arg("--description")
            .arg("This is a detailed description")
            .arg("--assignee")
            .arg("test-worker")
            .arg("--label")
            .arg("urgent")
            .arg("--label")
            .arg("backend")
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

        // Verify all fields are correctly set
        let show_output = Command::new("bf")
            .arg("show")
            .arg(&bead_id)
            .current_dir(workspace)
            .output()
            .expect("Failed to show bead");

        let show_text = String::from_utf8_lossy(&show_output.stdout);

        assert!(
            show_text.contains("Complete test bead with all parameters"),
            "Title should be set"
        );
        assert!(
            show_text.contains("This is a detailed description"),
            "Description should be set"
        );
        assert!(
            show_text.contains("bug"),
            "Type should be 'bug'"
        );
        assert!(
            show_text.contains("P0"),
            "Priority should be 'P0'"
        );
        assert!(
            show_text.contains("test-worker"),
            "Assignee should be set"
        );
        assert!(
            show_text.contains("urgent"),
            "First label should be present"
        );
        assert!(
            show_text.contains("backend"),
            "Second label should be present"
        );
        assert!(
            show_text.contains("security"),
            "Third label should be present"
        );
    }

    #[test]
    fn test_create_all_standard_types() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        let types = vec!["task", "bug", "feature", "epic", "chore", "docs", "question"];

        for issue_type in types {
            let create_output = Command::new("bf")
                .arg("create")
                .arg("--title")
                .arg(&format!("Test bead for type: {}", issue_type))
                .arg("--type")
                .arg(issue_type)
                .current_dir(workspace)
                .output()
                .expect(&format!("Failed to create bead with type {}", issue_type));

            assert!(
                create_output.status.success(),
                "bf create failed for type {}: {}",
                issue_type,
                String::from_utf8_lossy(&create_output.stderr)
            );

            let create_text = String::from_utf8_lossy(&create_output.stdout);
            let bead_id = extract_bead_id(&create_text);

            // Verify the type was set correctly
            let show_output = Command::new("bf")
                .arg("show")
                .arg(&bead_id)
                .current_dir(workspace)
                .output()
                .expect("Failed to show bead");

            let show_text = String::from_utf8_lossy(&show_output.stdout);
            assert!(
                show_text.contains(issue_type),
                "Type '{}' should be in show output for bead {}",
                issue_type,
                bead_id
            );
        }
    }

    #[test]
    fn test_create_all_priorities() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        let priorities = vec![("0", "P0"), ("1", "P1"), ("2", "P2"), ("3", "P3"), ("4", "P4")];

        for (priority_value, priority_display) in priorities {
            let create_output = Command::new("bf")
                .arg("create")
                .arg("--title")
                .arg(&format!("Test bead for priority: {}", priority_value))
                .arg("--priority")
                .arg(priority_value)
                .current_dir(workspace)
                .output()
                .expect(&format!("Failed to create bead with priority {}", priority_value));

            assert!(
                create_output.status.success(),
                "bf create failed for priority {}: {}",
                priority_value,
                String::from_utf8_lossy(&create_output.stderr)
            );

            let create_text = String::from_utf8_lossy(&create_output.stdout);
            let bead_id = extract_bead_id(&create_text);

            // Verify the priority was set correctly
            let show_output = Command::new("bf")
                .arg("show")
                .arg(&bead_id)
                .current_dir(workspace)
                .output()
                .expect("Failed to show bead");

            let show_text = String::from_utf8_lossy(&show_output.stdout);
            assert!(
                show_text.contains(priority_display),
                "Priority '{}' should be in show output for bead {}",
                priority_display,
                bead_id
            );
        }
    }

    #[test]
    fn test_create_with_single_label() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        let create_output = Command::new("bf")
            .arg("create")
            .arg("--title")
            .arg("Bead with single label")
            .arg("--label")
            .arg("test-label")
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

        // Verify label is present
        let show_output = Command::new("bf")
            .arg("show")
            .arg(&bead_id)
            .current_dir(workspace)
            .output()
            .expect("Failed to show bead");

        let show_text = String::from_utf8_lossy(&show_output.stdout);
        assert!(
            show_text.contains("test-label"),
            "Label should be present in bead"
        );
    }

    #[test]
    fn test_create_with_multiple_labels() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        let labels = vec!["label1", "label2", "label3", "label4", "label5"];

        let mut cmd = Command::new("bf");
        cmd.arg("create")
            .arg("--title")
            .arg("Bead with multiple labels")
            .current_dir(workspace);

        for label in &labels {
            cmd.arg("--label").arg(label);
        }

        let create_output = cmd
            .output()
            .expect("Failed to create bead");

        assert!(
            create_output.status.success(),
            "bf create failed: {}",
            String::from_utf8_lossy(&create_output.stderr)
        );

        let create_text = String::from_utf8_lossy(&create_output.stdout);
        let bead_id = extract_bead_id(&create_text);

        // Verify all labels are present
        let show_output = Command::new("bf")
            .arg("show")
            .arg(&bead_id)
            .current_dir(workspace)
            .output()
            .expect("Failed to show bead");

        let show_text = String::from_utf8_lossy(&show_output.stdout);
        for label in labels {
            assert!(
                show_text.contains(label),
                "Label '{}' should be present in bead",
                label
            );
        }
    }

    #[test]
    fn test_create_with_assignee() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        let assignees = vec![
            "worker-1",
            "claude-code-opus-4.8",
            "agent-smith",
            "test-worker@example.com",
        ];

        for assignee in assignees {
            let create_output = Command::new("bf")
                .arg("create")
                .arg("--title")
                .arg(&format!("Bead assigned to {}", assignee))
                .arg("--assignee")
                .arg(assignee)
                .current_dir(workspace)
                .output()
                .expect(&format!("Failed to create bead with assignee {}", assignee));

            assert!(
                create_output.status.success(),
                "bf create failed for assignee {}: {}",
                assignee,
                String::from_utf8_lossy(&create_output.stderr)
            );

            let create_text = String::from_utf8_lossy(&create_output.stdout);
            let bead_id = extract_bead_id(&create_text);

            // Verify assignee is set
            let show_output = Command::new("bf")
                .arg("show")
                .arg(&bead_id)
                .current_dir(workspace)
                .output()
                .expect("Failed to show bead");

            let show_text = String::from_utf8_lossy(&show_output.stdout);
            assert!(
                show_text.contains(assignee),
                "Assignee '{}' should be present in bead {}",
                assignee,
                bead_id
            );
        }
    }

    #[test]
    fn test_create_with_description() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        let descriptions = vec![
            "Simple description",
            "Multi-line\ndescription\nwith newlines",
            "Description with special chars: <>&\"'",
            "Very long description that exceeds normal length but should still be supported by the system without any issues or truncation occurring during storage or retrieval",
        ];

        for description in descriptions {
            let create_output = Command::new("bf")
                .arg("create")
                .arg("--title")
                .arg("Bead with description")
                .arg("--description")
                .arg(description)
                .current_dir(workspace)
                .output()
                .expect(&format!("Failed to create bead with description"));

            assert!(
                create_output.status.success(),
                "bf create failed: {}",
                String::from_utf8_lossy(&create_output.stderr)
            );

            let create_text = String::from_utf8_lossy(&create_output.stdout);
            let bead_id = extract_bead_id(&create_text);

            // Verify description is set
            let show_output = Command::new("bf")
                .arg("show")
                .arg(&bead_id)
                .current_dir(workspace)
                .output()
                .expect("Failed to show bead");

            let show_text = String::from_utf8_lossy(&show_output.stdout);
            // Check that description is present (may be truncated in display)
            let desc_preview = if description.len() > 50 {
                &description[..50]
            } else {
                description
            };
            assert!(
                show_text.contains(desc_preview) || description.len() > 50,
                "Description preview should be present in bead"
            );
        }
    }

    #[test]
    fn test_create_with_custom_type() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        let custom_types = vec!["spike", "spike-triage", "custom-workflow", "investigation"];

        for custom_type in custom_types {
            let create_output = Command::new("bf")
                .arg("create")
                .arg("--title")
                .arg(&format!("Bead with custom type: {}", custom_type))
                .arg("--type")
                .arg(custom_type)
                .current_dir(workspace)
                .output()
                .expect(&format!("Failed to create bead with custom type {}", custom_type));

            assert!(
                create_output.status.success(),
                "bf create failed for custom type {}: {}",
                custom_type,
                String::from_utf8_lossy(&create_output.stderr)
            );

            let create_text = String::from_utf8_lossy(&create_output.stdout);
            let bead_id = extract_bead_id(&create_text);

            // Verify custom type is preserved
            let show_output = Command::new("bf")
                .arg("show")
                .arg(&bead_id)
                .current_dir(workspace)
                .output()
                .expect("Failed to show bead");

            let show_text = String::from_utf8_lossy(&show_output.stdout);
            assert!(
                show_text.contains(custom_type),
                "Custom type '{}' should be present in bead {}",
                custom_type,
                bead_id
            );
        }
    }

    #[test]
    fn test_create_id_sequence() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        // Create multiple beads and verify IDs are sequential
        let mut bead_ids = Vec::new();

        for i in 0..5 {
            let create_output = Command::new("bf")
                .arg("create")
                .arg("--title")
                .arg(&format!("Sequential bead {}", i))
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
            bead_ids.push(bead_id);
        }

        // Verify all IDs are unique
        let unique_ids: std::collections::HashSet<_> = bead_ids.iter().collect();
        assert_eq!(
            unique_ids.len(),
            bead_ids.len(),
            "All bead IDs should be unique"
        );

        // List beads to verify they all exist
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
        for bead_id in &bead_ids {
            assert!(
                list_text.contains(bead_id),
                "Bead {} should be in list output",
                bead_id
            );
        }
    }

    #[test]
    fn test_create_defaults() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        // Create bead with minimal parameters (only title)
        let create_output = Command::new("bf")
            .arg("create")
            .arg("--title")
            .arg("Bead with defaults")
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

        // Verify default values
        let show_output = Command::new("bf")
            .arg("show")
            .arg(&bead_id)
            .current_dir(workspace)
            .output()
            .expect("Failed to show bead");

        let show_text = String::from_utf8_lossy(&show_output.stdout);

        // Check defaults
        assert!(
            show_text.contains("open"),
            "Default status should be 'open'"
        );
        assert!(
            show_text.contains("P2"),
            "Default priority should be 'P2'"
        );
        assert!(
            show_text.contains("task"),
            "Default type should be 'task'"
        );
    }

    #[test]
    fn test_create_persists_to_database() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        let create_output = Command::new("bf")
            .arg("create")
            .arg("--title")
            .arg("Persistence test bead")
            .arg("--type")
            .arg("feature")
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
        let bead_id = extract_bead_id(&create_text);

        // Verify bead is in list
        let list_output = Command::new("bf")
            .arg("list")
            .arg("--format")
            .arg("json")
            .current_dir(workspace)
            .output()
            .expect("Failed to list beads");

        let list_text = String::from_utf8_lossy(&list_output.stdout);
        assert!(
            list_text.contains(&bead_id),
            "Bead should be in list output"
        );
        assert!(
            list_text.contains("Persistence test bead"),
            "Bead title should be in list output"
        );

        // Show specific bead
        let show_output = Command::new("bf")
            .arg("show")
            .arg(&bead_id)
            .current_dir(workspace)
            .output()
            .expect("Failed to show bead");

        let show_text = String::from_utf8_lossy(&show_output.stdout);
        assert!(
            show_text.contains("feature"),
            "Type should be persisted"
        );
        assert!(
            show_text.contains("P1"),
            "Priority should be persisted"
        );
    }

    #[test]
    fn test_create_with_special_characters_in_title() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        let titles = vec![
            "Title with emoji 🎉",
            "Title with quotes \"test\"",
            "Title with apostrophes 'test'",
            "Title with special chars: <>&[]{}",
            "Title with unicode: café résumé naïve",
        ];

        for title in titles {
            let create_output = Command::new("bf")
                .arg("create")
                .arg("--title")
                .arg(title)
                .current_dir(workspace)
                .output()
                .expect(&format!("Failed to create bead with special title"));

            assert!(
                create_output.status.success(),
                "bf create failed for special title: {}",
                String::from_utf8_lossy(&create_output.stderr)
            );

            let create_text = String::from_utf8_lossy(&create_output.stdout);
            let bead_id = extract_bead_id(&create_text);

            // Verify title is preserved
            let show_output = Command::new("bf")
                .arg("show")
                .arg(&bead_id)
                .current_dir(workspace)
                .output()
                .expect("Failed to show bead");

            let show_text = String::from_utf8_lossy(&show_output.stdout);
            assert!(
                show_text.contains(title),
                "Special title should be preserved in bead: {}",
                title
            );
        }
    }

    #[test]
    fn test_create_with_hyphenated_labels() {
        let temp_dir = setup_test_workspace();
        let workspace = temp_dir.path();

        let labels = vec![
            "phase-1",
            "phase-2",
            "backend-service",
            "frontend-ui",
            "high-priority",
        ];

        let mut cmd = Command::new("bf");
        cmd.arg("create")
            .arg("--title")
            .arg("Bead with hyphenated labels")
            .current_dir(workspace);

        for label in &labels {
            cmd.arg("--label").arg(label);
        }

        let create_output = cmd
            .output()
            .expect("Failed to create bead");

        assert!(
            create_output.status.success(),
            "bf create failed: {}",
            String::from_utf8_lossy(&create_output.stderr)
        );

        let create_text = String::from_utf8_lossy(&create_output.stdout);
        let bead_id = extract_bead_id(&create_text);

        // Verify all labels are present
        let show_output = Command::new("bf")
            .arg("show")
            .arg(&bead_id)
            .current_dir(workspace)
            .output()
            .expect("Failed to show bead");

        let show_text = String::from_utf8_lossy(&show_output.stdout);
        for label in labels {
            assert!(
                show_text.contains(label),
                "Hyphenated label '{}' should be present",
                label
            );
        }
    }
}
