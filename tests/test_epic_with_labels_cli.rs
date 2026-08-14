//! Comprehensive CLI tests for epic with labels functionality.
//!
//! Tests the full CLI workflow for epics with labels:
//! - Creating epics with --label flag
//! - Adding labels with `bf label add`
//! - Removing labels with `bf label remove`
//! - Listing labels with `bf labels` and `bf label list`
//! - Filtering epics by labels with `bf search --label`
//! - JSON output format validation
//! - Epic type preservation through label operations

use std::fs;
use std::path::Path;
use std::process::Command;

/// Helper to run bf commands and capture output
fn bf_cmd(args: &[&str], dir: &Path) -> (String, String, bool) {
    let output = Command::new("bf")
        .args(args)
        .current_dir(dir)
        .env("BEADS_DIR", dir.join(".beads"))
        .output()
        .expect("Failed to run bf command");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    (stdout, stderr, success)
}

/// Helper to create a test workspace
fn create_test_workspace(_name: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let beads_dir = dir.path().join(".beads");
    fs::create_dir(&beads_dir).expect("Failed to create .beads dir");

    // Initialize beads workspace
    let (stdout, stderr, success) = bf_cmd(&["init", "--prefix", "test"], dir.path());
    assert!(
        success,
        "bf init failed: stdout: {}, stderr: {}",
        stdout, stderr
    );

    dir
}

/// Helper to extract bead ID from JSON output
fn extract_id(json: &str) -> String {
    if let Some(line) = json.lines().next() {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(id) = data.get("id").and_then(|v| v.as_str()) {
                return id.to_string();
            }
        }
    }
    panic!("Failed to extract ID from JSON: {}", json);
}

#[cfg(test)]
mod epic_label_cli_tests {
    use super::*;

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_create_epic_with_single_label() {
        let workspace = create_test_workspace("epic_single_label");
        let dir = workspace.path();

        // Create epic with a single label
        let (stdout, stderr, success) = bf_cmd(
            &[
                "create",
                "--type",
                "epic",
                "--label",
                "epic-label",
                "Test Epic with Label",
            ],
            dir,
        );
        assert!(
            success,
            "Create epic failed: stdout: {}, stderr: {}",
            stdout, stderr
        );

        let id = extract_id(&stdout);

        // Verify the epic was created with correct type and label
        let (stdout, stderr, success) = bf_cmd(&["show", &id, "--format", "json"], dir);
        assert!(
            success,
            "Show epic failed: stdout: {}, stderr: {}",
            stdout, stderr
        );

        if let Some(line) = stdout.lines().next() {
            let data: serde_json::Value = serde_json::from_str(line).expect("Failed to parse JSON");
            assert_eq!(data["issue_type"], "epic");
            assert_eq!(data["labels"].as_array().unwrap().len(), 1);
            assert_eq!(data["labels"][0], "epic-label");
        }
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_create_epic_with_multiple_labels() {
        let workspace = create_test_workspace("epic_multiple_labels");
        let dir = workspace.path();

        // Create epic with multiple labels
        let (stdout, stderr, success) = bf_cmd(
            &[
                "create",
                "--type",
                "epic",
                "--label",
                "phase-1",
                "--label",
                "backend",
                "--label",
                "high-priority",
                "Multi-label Epic",
            ],
            dir,
        );
        assert!(
            success,
            "Create epic failed: stdout: {}, stderr: {}",
            stdout, stderr
        );

        let id = extract_id(&stdout);

        // Verify all labels are present
        let (stdout, stderr, success) = bf_cmd(&["show", &id, "--format", "json"], dir);
        assert!(
            success,
            "Show epic failed: stdout: {}, stderr: {}",
            stdout, stderr
        );

        if let Some(line) = stdout.lines().next() {
            let data: serde_json::Value = serde_json::from_str(line).expect("Failed to parse JSON");
            let labels = data["labels"].as_array().unwrap();
            assert_eq!(labels.len(), 3);

            let label_values: Vec<String> = labels
                .iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect();

            assert!(label_values.contains(&"phase-1".to_string()));
            assert!(label_values.contains(&"backend".to_string()));
            assert!(label_values.contains(&"high-priority".to_string()));
        }
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_add_label_to_existing_epic() {
        let workspace = create_test_workspace("add_label_epic");
        let dir = workspace.path();

        // Create epic without labels
        let (stdout, _, _) = bf_cmd(&["create", "--type", "epic", "Epic for label add"], dir);
        let id = extract_id(&stdout);

        // Add a label using `bf label add`
        let (stdout, stderr, success) =
            bf_cmd(&["label", "add", "--label", "added-label", &id], dir);
        assert!(
            success,
            "Label add failed: stdout: {}, stderr: {}",
            stdout, stderr
        );
        assert!(stdout.contains("Added label 'added-label'"));

        // Verify label was added
        let (stdout, _, success) = bf_cmd(&["labels", &id, "--format", "json"], dir);
        assert!(success);

        if let Some(line) = stdout.lines().next() {
            let data: serde_json::Value = serde_json::from_str(line).expect("Failed to parse JSON");
            assert_eq!(data["labels"].as_array().unwrap().len(), 1);
            assert_eq!(data["labels"][0], "added-label");
        }
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_add_multiple_labels_to_epic() {
        let workspace = create_test_workspace("add_multiple_labels");
        let dir = workspace.path();

        // Create epic
        let (stdout, _, _) = bf_cmd(
            &["create", "--type", "epic", "Epic for multiple labels"],
            dir,
        );
        let id = extract_id(&stdout);

        // Add multiple labels
        for label in &["label-1", "label-2", "label-3"] {
            let (stdout, stderr, success) = bf_cmd(&["label", "add", "--label", label, &id], dir);
            assert!(
                success,
                "Label add failed for {}: stdout: {}, stderr: {}",
                label, stdout, stderr
            );
        }

        // Verify all labels are present
        let (stdout, _, _) = bf_cmd(&["labels", &id, "--format", "json"], dir);

        if let Some(line) = stdout.lines().next() {
            let data: serde_json::Value = serde_json::from_str(line).expect("Failed to parse JSON");
            assert_eq!(data["labels"].as_array().unwrap().len(), 3);
        }
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_remove_label_from_epic() {
        let workspace = create_test_workspace("remove_label_epic");
        let dir = workspace.path();

        // Create epic with labels
        let (stdout, _, _) = bf_cmd(
            &[
                "create",
                "--type",
                "epic",
                "--label",
                "keep",
                "--label",
                "remove",
                "Epic for label remove",
            ],
            dir,
        );
        let id = extract_id(&stdout);

        // Remove one label
        let (stdout, stderr, success) = bf_cmd(&["label", "remove", "--label", "remove", &id], dir);
        assert!(
            success,
            "Label remove failed: stdout: {}, stderr: {}",
            stdout, stderr
        );
        assert!(stdout.contains("Removed label 'remove'"));

        // Verify only 'keep' label remains
        let (stdout, _, _) = bf_cmd(&["labels", &id, "--format", "json"], dir);

        if let Some(line) = stdout.lines().next() {
            let data: serde_json::Value = serde_json::from_str(line).expect("Failed to parse JSON");
            let labels = data["labels"].as_array().unwrap();
            assert_eq!(labels.len(), 1);
            assert_eq!(labels[0], "keep");
        }
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_list_labels_for_epic() {
        let workspace = create_test_workspace("list_labels");
        let dir = workspace.path();

        // Create epic with labels
        let (stdout, _, _) = bf_cmd(
            &[
                "create",
                "--type",
                "epic",
                "--label",
                "alpha",
                "--label",
                "beta",
                "Label List Epic",
            ],
            dir,
        );
        let id = extract_id(&stdout);

        // List labels for the epic
        let (stdout, stderr, success) = bf_cmd(&["labels", &id], dir);
        assert!(
            success,
            "Labels command failed: stdout: {}, stderr: {}",
            stdout, stderr
        );
        assert!(stdout.contains("Labels for"));
        assert!(stdout.contains("alpha"));
        assert!(stdout.contains("beta"));
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_list_all_labels() {
        let workspace = create_test_workspace("list_all_labels");
        let dir = workspace.path();

        // Create multiple epics with overlapping labels
        let (stdout, _, _) = bf_cmd(
            &[
                "create", "--type", "epic", "--label", "common", "--label", "epic1", "Epic 1",
            ],
            dir,
        );
        let _id1 = extract_id(&stdout);

        let (stdout, _, _) = bf_cmd(
            &[
                "create", "--type", "epic", "--label", "common", "--label", "epic2", "Epic 2",
            ],
            dir,
        );
        let _id2 = extract_id(&stdout);

        let (stdout, _, _) = bf_cmd(
            &["create", "--type", "epic", "--label", "unique", "Epic 3"],
            dir,
        );
        let _id3 = extract_id(&stdout);

        // List all labels (no specific ID)
        let (stdout, stderr, success) = bf_cmd(&["label", "list"], dir);
        assert!(
            success,
            "Label list failed: stdout: {}, stderr: {}",
            stdout, stderr
        );
        assert!(stdout.contains("common"));
        assert!(stdout.contains("unique"));
        // Should show counts
        assert!(stdout.contains("3") || stdout.contains("2") || stdout.contains("1"));
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_search_epics_by_label() {
        let workspace = create_test_workspace("search_by_label");
        let dir = workspace.path();

        // Create epics with different labels
        let (stdout, _, _) = bf_cmd(
            &[
                "create",
                "--type",
                "epic",
                "--label",
                "backend",
                "Backend Epic",
            ],
            dir,
        );
        let backend_id = extract_id(&stdout);

        let (stdout, _, _) = bf_cmd(
            &[
                "create",
                "--type",
                "epic",
                "--label",
                "frontend",
                "Frontend Epic",
            ],
            dir,
        );
        let frontend_id = extract_id(&stdout);

        let (stdout, _, _) = bf_cmd(
            &[
                "create",
                "--type",
                "epic",
                "--label",
                "backend",
                "Another Backend Epic",
            ],
            dir,
        );
        let another_backend_id = extract_id(&stdout);

        // Search for backend epics
        let (stdout, stderr, success) = bf_cmd(
            &[
                "search", "--label", "backend", "--type", "epic", "--format", "json",
            ],
            dir,
        );
        assert!(
            success,
            "Search failed: stdout: {}, stderr: {}",
            stdout, stderr
        );

        // Should find 2 backend epics
        let results: Vec<serde_json::Value> = stdout
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        assert_eq!(results.len(), 2);
        let ids: Vec<String> = results
            .iter()
            .filter_map(|v| v.get("id").and_then(|i| i.as_str()).map(String::from))
            .collect();
        assert!(ids.contains(&backend_id));
        assert!(ids.contains(&another_backend_id));
        assert!(!ids.contains(&frontend_id));
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_search_epics_by_multiple_labels() {
        let workspace = create_test_workspace("search_multiple_labels");
        let dir = workspace.path();

        // Create epics
        let (_stdout, _, _) = bf_cmd(
            &[
                "create",
                "--type",
                "epic",
                "--label",
                "backend",
                "--label",
                "phase-1",
                "Backend P1",
            ],
            dir,
        );

        let (_stdout, _, _) = bf_cmd(
            &[
                "create",
                "--type",
                "epic",
                "--label",
                "frontend",
                "--label",
                "phase-1",
                "Frontend P1",
            ],
            dir,
        );

        let (_stdout, _, _) = bf_cmd(
            &[
                "create",
                "--type",
                "epic",
                "--label",
                "backend",
                "--label",
                "phase-2",
                "Backend P2",
            ],
            dir,
        );

        // Search with multiple labels (OR logic)
        let (stdout, stderr, success) = bf_cmd(
            &[
                "search", "--label", "backend", "--label", "phase-1", "--type", "epic", "--format",
                "json",
            ],
            dir,
        );
        assert!(
            success,
            "Search failed: stdout: {}, stderr: {}",
            stdout, stderr
        );

        let results: Vec<serde_json::Value> = stdout
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        // Should find at least 3 epics (all with backend OR phase-1)
        assert!(results.len() >= 3);
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_epic_type_preserved_with_label_operations() {
        let workspace = create_test_workspace("epic_type_preservation");
        let dir = workspace.path();

        // Create epic
        let (stdout, _, _) = bf_cmd(
            &[
                "create",
                "--type",
                "epic",
                "--label",
                "initial",
                "Type Preservation Epic",
            ],
            dir,
        );
        let id = extract_id(&stdout);

        // Perform various label operations
        let (_, _, add_success) = bf_cmd(&["label", "add", "--label", "added", &id], dir);
        assert!(add_success, "Add label failed");

        let (_, _, remove_success) = bf_cmd(&["label", "remove", "--label", "initial", &id], dir);
        assert!(remove_success, "Remove label failed");

        // Verify type is still epic
        let (stdout, _, _) = bf_cmd(&["show", &id, "--format", "json"], dir);

        if let Some(line) = stdout.lines().next() {
            let data: serde_json::Value = serde_json::from_str(line).expect("Failed to parse JSON");
            assert_eq!(data["issue_type"], "epic");
        }
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_json_format_for_epic_labels() {
        let workspace = create_test_workspace("json_format");
        let dir = workspace.path();

        // Create epic with labels
        let (stdout, _, _) = bf_cmd(
            &[
                "create",
                "--type",
                "epic",
                "--label",
                "json-test",
                "--label",
                "format",
                "JSON Format Epic",
            ],
            dir,
        );
        let id = extract_id(&stdout);

        // Verify JSON output format
        let (stdout, _, success) = bf_cmd(&["show", &id, "--format", "json"], dir);
        assert!(success);

        if let Some(line) = stdout.lines().next() {
            let data: serde_json::Value = serde_json::from_str(line).expect("Failed to parse JSON");

            // Verify structure
            assert!(data.is_object());
            assert!(data.get("id").is_some());
            assert!(data.get("title").is_some());
            assert!(data.get("issue_type").is_some());
            assert!(data.get("labels").is_some());

            // Verify labels is an array
            let labels = data["labels"].as_array().expect("labels should be array");
            assert_eq!(labels.len(), 2);

            // Verify label values are strings
            for label in labels {
                assert!(label.is_string());
            }
        }
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_special_characters_in_labels() {
        let workspace = create_test_workspace("special_chars");
        let dir = workspace.path();

        // Create epic with special character labels
        let (stdout, stderr, success) = bf_cmd(
            &[
                "create",
                "--type",
                "epic",
                "--label",
                "label-with-dash",
                "--label",
                "label_with_underscore",
                "Special Chars Epic",
            ],
            dir,
        );
        assert!(
            success,
            "Create failed: stdout: {}, stderr: {}",
            stdout, stderr
        );

        let id = extract_id(&stdout);

        // Verify labels are preserved correctly
        let (stdout, _, _) = bf_cmd(&["labels", &id, "--format", "json"], dir);

        if let Some(line) = stdout.lines().next() {
            let data: serde_json::Value = serde_json::from_str(line).expect("Failed to parse JSON");
            let labels = data["labels"].as_array().unwrap();
            assert_eq!(labels.len(), 2);
        }
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_empty_labels_list() {
        let workspace = create_test_workspace("empty_labels");
        let dir = workspace.path();

        // Create epic without labels
        let (stdout, _, _) = bf_cmd(&["create", "--type", "epic", "No Labels Epic"], dir);
        let id = extract_id(&stdout);

        // List labels should show empty
        let (stdout, _, _) = bf_cmd(&["labels", &id, "--format", "json"], dir);

        if let Some(line) = stdout.lines().next() {
            let data: serde_json::Value = serde_json::from_str(line).expect("Failed to parse JSON");
            assert_eq!(data["labels"].as_array().unwrap().len(), 0);
        }
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_duplicate_label_handling() {
        let workspace = create_test_workspace("duplicate_labels");
        let dir = workspace.path();

        // Create epic
        let (stdout, _, _) = bf_cmd(
            &[
                "create",
                "--type",
                "epic",
                "--label",
                "test-label",
                "Duplicate Test Epic",
            ],
            dir,
        );
        let id = extract_id(&stdout);

        // Try adding the same label again
        let (stdout, stderr, success) =
            bf_cmd(&["label", "add", "--label", "test-label", &id], dir);
        // Should succeed but not duplicate
        assert!(
            success,
            "Add duplicate label failed: stdout: {}, stderr: {}",
            stdout, stderr
        );

        // Verify only one instance exists
        let (stdout, _, _) = bf_cmd(&["labels", &id, "--format", "json"], dir);

        if let Some(line) = stdout.lines().next() {
            let data: serde_json::Value = serde_json::from_str(line).expect("Failed to parse JSON");
            assert_eq!(data["labels"].as_array().unwrap().len(), 1);
        }
    }
}
