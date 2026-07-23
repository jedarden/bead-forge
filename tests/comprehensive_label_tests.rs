// Comprehensive label tests for bead-forge
// Tests all label operations including edge cases, persistence, and integration

use std::process::Command;
use std::sync::OnceLock;

/// Resolve the freshly-built bf binary — never the system-installed one.
fn bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

static WORKSPACE: OnceLock<tempfile::TempDir> = OnceLock::new();

/// Per-binary isolated workspace — these tests run against an isolated workspace
/// to avoid polluting the repo's tracked .beads directory.
fn workspace_dir() -> &'static std::path::Path {
    WORKSPACE
        .get_or_init(|| {
            let dir = tempfile::tempdir().unwrap();
            let beads = dir.path().join(".beads");
            std::fs::create_dir(&beads).unwrap();
            bead_forge::config::init_workspace(&beads, "bf").unwrap();
            // Create the database up front (WAL mode, schema applied) so
            // parallel test threads never stampede a cold-start conversion.
            let metadata = bead_forge::config::load_metadata(&beads).unwrap();
            let _ = bead_forge::Storage::open(&beads.join(&metadata.database)).unwrap();
            dir
        })
        .path()
}

fn bf() -> Command {
    let mut cmd = Command::new(bf_binary());
    cmd.arg("-w")
        .arg(workspace_dir().join(".beads"))
        .current_dir(workspace_dir());
    cmd
}

/// Create a test bead with optional labels
fn create_test_bead(title: &str, labels: &[&str]) -> String {
    let mut cmd = bf();
    cmd.arg("create")
        .arg("--title")
        .arg(title)
        .arg("--type")
        .arg("task")
        .arg("--priority")
        .arg("2");

    for label in labels {
        cmd.arg("--label").arg(label);
    }

    let output = cmd.output().expect("Failed to create bead");

    assert!(
        output.status.success(),
        "Failed to create bead: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    stdout.trim().to_string()
}

/// Close a test bead
fn close_test_bead(bead_id: &str) {
    let output = bf()
        .arg("close")
        .arg(bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");

    assert!(
        output.status.success(),
        "Failed to close bead: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

mod basic_label_operations {
    use super::*;

    #[test]
    fn test_label_add_single() {
        let bead_id = create_test_bead("test single label add", &[]);

        // Add a single label
        let output = bf()
            .args(["label", "add", &bead_id, "--label", "urgent"])
            .output()
            .expect("Failed to add label");

        assert!(
            output.status.success(),
            "Failed to add label: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Verify label was added
        let output = bf()
            .args(["labels", &bead_id, "--format", "json"])
            .output()
            .expect("Failed to list labels");

        let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

        assert_eq!(labels.len(), 1);
        assert!(labels.contains(&"urgent".to_string()));

        close_test_bead(&bead_id);
    }

    #[test]
    fn test_label_add_multiple() {
        let bead_id = create_test_bead("test multiple label add", &[]);

        // Add multiple labels at once
        let output = bf()
            .args([
                "label",
                "add",
                &bead_id,
                "--label",
                "urgent",
                "--label",
                "backend",
                "--label",
                "phase-1",
            ])
            .output()
            .expect("Failed to add labels");

        assert!(
            output.status.success(),
            "Failed to add labels: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Verify all labels were added
        let output = bf()
            .args(["labels", &bead_id, "--format", "json"])
            .output()
            .expect("Failed to list labels");

        let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

        assert_eq!(labels.len(), 3);
        assert!(labels.contains(&"urgent".to_string()));
        assert!(labels.contains(&"backend".to_string()));
        assert!(labels.contains(&"phase-1".to_string()));

        close_test_bead(&bead_id);
    }

    #[test]
    fn test_label_add_duplicate_idempotent() {
        let bead_id = create_test_bead("test duplicate label add", &[]);

        // Add the same label twice
        let output = bf()
            .args(["label", "add", &bead_id, "--label", "urgent", "--label", "urgent"])
            .output()
            .expect("Failed to add labels");

        assert!(
            output.status.success(),
            "Failed to add labels: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Verify only one instance exists
        let output = bf()
            .args(["labels", &bead_id, "--format", "json"])
            .output()
            .expect("Failed to list labels");

        let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

        assert_eq!(labels.len(), 1);
        assert!(labels.contains(&"urgent".to_string()));

        close_test_bead(&bead_id);
    }

    #[test]
    fn test_label_remove_single() {
        let bead_id = create_test_bead("test single label remove", &["urgent", "backend", "bug"]);

        // Remove one label
        let output = bf()
            .args(["label", "remove", &bead_id, "--label", "urgent"])
            .output()
            .expect("Failed to remove label");

        assert!(
            output.status.success(),
            "Failed to remove label: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Verify label was removed
        let output = bf()
            .args(["labels", &bead_id, "--format", "json"])
            .output()
            .expect("Failed to list labels");

        let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

        assert_eq!(labels.len(), 2);
        assert!(!labels.contains(&"urgent".to_string()));
        assert!(labels.contains(&"backend".to_string()));
        assert!(labels.contains(&"bug".to_string()));

        close_test_bead(&bead_id);
    }

    #[test]
    fn test_label_remove_multiple() {
        let bead_id = create_test_bead(
            "test multiple label remove",
            &["urgent", "backend", "bug", "phase-1"],
        );

        // Remove multiple labels
        let output = bf()
            .args([
                "label",
                "remove",
                &bead_id,
                "--label",
                "urgent",
                "--label",
                "bug",
            ])
            .output()
            .expect("Failed to remove labels");

        assert!(
            output.status.success(),
            "Failed to remove labels: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Verify labels were removed
        let output = bf()
            .args(["labels", &bead_id, "--format", "json"])
            .output()
            .expect("Failed to list labels");

        let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

        assert_eq!(labels.len(), 2);
        assert!(!labels.contains(&"urgent".to_string()));
        assert!(!labels.contains(&"bug".to_string()));
        assert!(labels.contains(&"backend".to_string()));
        assert!(labels.contains(&"phase-1".to_string()));

        close_test_bead(&bead_id);
    }

    #[test]
    fn test_label_remove_nonexistent_idempotent() {
        let bead_id = create_test_bead("test remove nonexistent label", &["backend"]);

        // Try to remove a label that doesn't exist (should succeed)
        let output = bf()
            .args(["label", "remove", &bead_id, "--label", "nonexistent"])
            .output()
            .expect("Failed to attempt removal");

        assert!(
            output.status.success(),
            "Removing nonexistent label should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Verify the original label is still there
        let output = bf()
            .args(["labels", &bead_id, "--format", "json"])
            .output()
            .expect("Failed to list labels");

        let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

        assert_eq!(labels.len(), 1);
        assert!(labels.contains(&"backend".to_string()));

        close_test_bead(&bead_id);
    }

    #[test]
    fn test_label_remove_idempotent() {
        let bead_id = create_test_bead("test idempotent label removal", &["urgent"]);

        // Remove the label twice (should succeed both times)
        let output1 = bf()
            .args(["label", "remove", &bead_id, "--label", "urgent"])
            .output()
            .expect("Failed to remove label first time");

        assert!(
            output1.status.success(),
            "First removal failed: {}",
            String::from_utf8_lossy(&output1.stderr)
        );

        let output2 = bf()
            .args(["label", "remove", &bead_id, "--label", "urgent"])
            .output()
            .expect("Failed to remove label second time");

        assert!(
            output2.status.success(),
            "Second removal should succeed (idempotent): {}",
            String::from_utf8_lossy(&output2.stderr)
        );

        // Verify no labels remain
        let output = bf()
            .args(["labels", &bead_id, "--format", "json"])
            .output()
            .expect("Failed to list labels");

        let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

        assert_eq!(labels.len(), 0);

        close_test_bead(&bead_id);
    }
}

mod label_list_operations {
    use super::*;

    #[test]
    fn test_label_list_empty_bead() {
        let bead_id = create_test_bead("test empty label list", &[]);

        // List labels for a bead with no labels
        let output = bf()
            .args(["labels", &bead_id, "--format", "json"])
            .output()
            .expect("Failed to list labels");

        assert!(
            output.status.success(),
            "Failed to list labels: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

        assert_eq!(labels.len(), 0);

        close_test_bead(&bead_id);
    }

    #[test]
    fn test_label_list_all_unique() {
        let bead1 = create_test_bead("label list bead 1", &["urgent", "backend"]);
        let bead2 = create_test_bead("label list bead 2", &["urgent", "frontend"]);

        // List all unique labels (no bead ID)
        let output = bf().args(["label", "list"]).output().expect("Failed to list all labels");

        assert!(
            output.status.success(),
            "Failed to list all labels: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        // Output format is "label (count)" per line
        let labels: Vec<String> = stdout
            .lines()
            .filter(|line| !line.is_empty() && !line.contains("All labels:"))
            .map(|line| line.trim().split('(').next().unwrap().trim().to_string())
            .collect();

        // Should have at least 3 unique labels: urgent, backend, frontend
        assert!(
            labels.len() >= 3,
            "Expected at least 3 unique labels, got {:?}: {}",
            labels,
            stdout
        );
        assert!(labels.contains(&"urgent".to_string()));
        assert!(labels.contains(&"backend".to_string()));
        assert!(labels.contains(&"frontend".to_string()));

        close_test_bead(&bead1);
        close_test_bead(&bead2);
    }

    #[test]
    fn test_label_list_with_bead_id() {
        let bead_id = create_test_bead("test label list with id", &["urgent", "backend"]);

        // List labels for a specific bead using `label list`
        let output = bf()
            .args(["label", "list", &bead_id])
            .output()
            .expect("Failed to list labels");

        assert!(
            output.status.success(),
            "Failed to list labels: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        let labels: Vec<String> = stdout
            .lines()
            .filter(|line| !line.is_empty() && !line.contains("Labels for"))
            .map(|s| s.trim().to_string())
            .collect();

        assert_eq!(labels.len(), 2, "Expected 2 labels, got: {:?}", labels);

        close_test_bead(&bead_id);
    }
}

mod label_integration {
    use super::*;

    #[test]
    fn test_create_with_labels() {
        // Create a bead with labels via `create --label`
        let bead_id = create_test_bead("test create with labels", &["urgent", "backend"]);

        // Verify labels were added during creation
        let output = bf()
            .args(["labels", &bead_id, "--format", "json"])
            .output()
            .expect("Failed to list labels");

        let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

        assert_eq!(labels.len(), 2);
        assert!(labels.contains(&"urgent".to_string()));
        assert!(labels.contains(&"backend".to_string()));

        close_test_bead(&bead_id);
    }

    #[test]
    fn test_show_includes_labels() {
        let bead_id = create_test_bead("test show labels", &["urgent", "backend"]);

        // Check that `bf show` includes labels
        let output = bf()
            .args(["show", &bead_id, "--format", "json"])
            .output()
            .expect("Failed to show bead");

        assert!(
            output.status.success(),
            "Failed to show bead: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        // show --format json returns a single-element array: [{...}]
        let bead_array: Vec<serde_json::Value> =
            serde_json::from_str(&json_output).expect("Failed to parse show JSON");

        // Labels should be present in the output
        assert!(!bead_array.is_empty(), "Show output should contain at least one bead");
        let bead = &bead_array[0];

        assert!(
            bead.get("labels").is_some(),
            "Show output should include labels field, got keys: {:?}",
            bead.as_object().map(|o| o.keys().collect::<Vec<_>>())
        );

        if let Some(labels) = bead.get("labels") {
            assert!(
                labels.is_array(),
                "Labels should be an array in show output"
            );
            let labels_arr = labels.as_array().unwrap();
            assert_eq!(labels_arr.len(), 2, "Expected 2 labels, got: {}", labels_arr.len());
        }

        close_test_bead(&bead_id);
    }

    #[test]
    fn test_search_by_label() {
        let bead1 = create_test_bead("search test bead 1", &["urgent", "backend"]);
        let bead2 = create_test_bead("search test bead 2", &["frontend"]);

        // Search for beads with the "urgent" label
        let output = bf()
            .args(["search", "--label", "urgent", "--format", "json"])
            .output()
            .expect("Failed to search by label");

        assert!(
            output.status.success(),
            "Failed to search: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        // search --format json returns JSONL (newline-delimited JSON objects)
        let results: Vec<serde_json::Value> = json_output
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_str(line).expect("Failed to parse search result line"))
            .collect();

        // Should find at least bead1
        assert!(
            results.len() >= 1,
            "Expected at least 1 result for label search, got {}",
            results.len()
        );

        let found_bead1 = results
            .iter()
            .any(|b| b.get("id").and_then(|v| v.as_str()) == Some(&bead1));
        assert!(found_bead1, "Search should have found bead1 with 'urgent' label");

        close_test_bead(&bead1);
        close_test_bead(&bead2);
    }

    #[test]
    fn test_search_by_multiple_labels() {
        let bead1 = create_test_bead("multi-label search 1", &["urgent", "backend"]);
        let bead2 = create_test_bead("multi-label search 2", &["urgent"]);
        let bead3 = create_test_bead("multi-label search 3", &["backend"]);

        // Search for beads with EITHER "urgent" OR "backend" label
        let output = bf()
            .args(["search", "--label", "urgent", "--label", "backend", "--format", "json"])
            .output()
            .expect("Failed to search by multiple labels");

        assert!(
            output.status.success(),
            "Failed to search: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        // search --format json returns JSONL (newline-delimited JSON objects)
        let results: Vec<serde_json::Value> = json_output
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_str(line).expect("Failed to parse search result line"))
            .collect();

        // Should find all three beads (OR logic for multiple --label flags)
        assert!(
            results.len() >= 3,
            "Expected at least 3 results for multi-label search, got {}",
            results.len()
        );

        close_test_bead(&bead1);
        close_test_bead(&bead2);
        close_test_bead(&bead3);
    }
}

mod label_output_formats {
    use super::*;

    #[test]
    fn test_labels_shortcut_text_format() {
        let bead_id = create_test_bead("test labels shortcut text", &["urgent", "backend"]);

        // Test the `bf labels` shortcut in text format (default)
        let output = bf()
            .args(["labels", &bead_id])
            .output()
            .expect("Failed to list labels");

        assert!(
            output.status.success(),
            "Failed to list labels: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        let labels: Vec<&str> = stdout.lines().filter(|line| !line.is_empty()).collect();

        assert_eq!(labels.len(), 2);

        close_test_bead(&bead_id);
    }

    #[test]
    fn test_labels_shortcut_json_format() {
        let bead_id = create_test_bead("test labels shortcut json", &["urgent", "backend"]);

        // Test the `bf labels` shortcut in JSON format
        let output = bf()
            .args(["labels", &bead_id, "--format", "json"])
            .output()
            .expect("Failed to list labels");

        assert!(
            output.status.success(),
            "Failed to list labels: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

        assert_eq!(labels.len(), 2);

        close_test_bead(&bead_id);
    }
}

mod label_persistence {
    use super::*;

    #[test]
    fn test_labels_persist_through_sync() {
        let bead_id = create_test_bead("test label persistence", &[]);

        // Add labels
        bf()
            .args([
                "label",
                "add",
                &bead_id,
                "--label",
                "urgent",
                "--label",
                "backend",
            ])
            .output()
            .expect("Failed to add labels");

        // Sync to JSONL
        let output = bf().args(["sync", "--flush-only"]).output().expect("Failed to sync");

        assert!(
            output.status.success(),
            "Failed to sync: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Verify labels are still present
        let output = bf()
            .args(["labels", &bead_id, "--format", "json"])
            .output()
            .expect("Failed to list labels");

        let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

        assert_eq!(labels.len(), 2);
        assert!(labels.contains(&"urgent".to_string()));
        assert!(labels.contains(&"backend".to_string()));

        close_test_bead(&bead_id);
    }
}

mod edge_cases {
    use super::*;

    #[test]
    fn test_label_with_special_characters() {
        let bead_id = create_test_bead("test special char labels", &[]);

        // Add labels with special characters
        let special_labels = vec!["bug:critical", "feature/auth", "ui-component"];
        for label in &special_labels {
            let output = bf()
                .args(["label", "add", &bead_id, "--label", label])
                .output()
                .expect("Failed to add label");

            assert!(
                output.status.success(),
                "Failed to add label '{}': {}",
                label,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Verify all labels were added
        let output = bf()
            .args(["labels", &bead_id, "--format", "json"])
            .output()
            .expect("Failed to list labels");

        let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

        assert_eq!(labels.len(), 3);
        for label in &special_labels {
            assert!(labels.contains(&label.to_string()));
        }

        close_test_bead(&bead_id);
    }

    #[test]
    fn test_empty_label_behavior() {
        let bead_id = create_test_bead("test empty label behavior", &[]);

        // Test current behavior with empty labels
        // The CLI currently accepts empty strings as labels
        let output = bf()
            .args(["label", "add", &bead_id, "--label", ""])
            .output()
            .expect("Failed to add empty label");

        // Verify the operation succeeds (current behavior)
        assert!(
            output.status.success(),
            "Empty label add should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Document that empty labels are currently stored
        let labels_output = bf()
            .args(["labels", &bead_id, "--format", "json"])
            .output()
            .expect("Failed to list labels");
        let json_output = String::from_utf8(labels_output.stdout).expect("Invalid UTF-8");
        let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

        // Current behavior: empty label is added
        // This test documents the actual behavior for future reference
        assert!(labels.contains(&"".to_string()), "Empty label is currently stored");

        close_test_bead(&bead_id);
    }

    #[test]
    fn test_large_number_of_labels() {
        let bead_id = create_test_bead("test many labels", &[]);

        // Add a large number of labels
        let labels: Vec<String> = (0..50).map(|i| format!("label-{}", i)).collect();

        for label in &labels {
            bf()
                .args(["label", "add", &bead_id, "--label", label])
                .output()
                .expect("Failed to add label");
        }

        // Verify all labels were added
        let output = bf()
            .args(["labels", &bead_id, "--format", "json"])
            .output()
            .expect("Failed to list labels");

        let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        let result_labels: Vec<String> =
            serde_json::from_str(&json_output).expect("Failed to parse labels JSON");

        assert_eq!(result_labels.len(), 50);

        close_test_bead(&bead_id);
    }
}
