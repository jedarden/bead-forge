// Comprehensive label functionality tests
// Tests labels command in both text and JSON formats, sync persistence, and edge cases

use std::process::Command;
use std::sync::OnceLock;

static WORKSPACE: OnceLock<tempfile::TempDir> = OnceLock::new();

/// Per-binary isolated workspace
fn workspace_dir() -> &'static std::path::Path {
    WORKSPACE
        .get_or_init(|| {
            let dir = tempfile::tempdir().unwrap();
            let beads = dir.path().join(".beads");
            std::fs::create_dir(&beads).unwrap();
            bead_forge::config::init_workspace(&beads, "bf").unwrap();
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

fn bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

fn create_test_bead(title: &str) -> String {
    let output = bf()
        .arg("create")
        .arg("--title")
        .arg(title)
        .arg("--type")
        .arg("task")
        .arg("--priority")
        .arg("2")
        .output()
        .expect("Failed to create bead");

    assert!(
        output.status.success(),
        "Failed to create bead: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    stdout.trim().to_string()
}

#[test]
fn test_labels_text_format_single_bead() {
    // Test labels command in text format for a single bead
    let bead_id = create_test_bead("Text format test bead");

    // Add labels
    bf().arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .arg("--label")
        .arg("backend")
        .output()
        .expect("Failed to add labels");

    // List labels in text format (default)
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .output()
        .expect("Failed to list labels");

    assert!(
        output.status.success(),
        "Failed to list labels: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let lines: Vec<&str> = stdout.lines().collect();

    // Should have one label per line
    assert_eq!(lines.len(), 2, "Expected 2 label lines, got {}: {:?}", lines.len(), stdout);

    // Labels should be present
    let text = stdout.to_lowercase();
    assert!(text.contains("urgent"), "Missing 'urgent' label in text output");
    assert!(text.contains("backend"), "Missing 'backend' label in text output");

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_labels_text_format_all_beads() {
    // Test labels command in text format showing all beads with labels
    let bead1 = create_test_bead("Bead one");
    let bead2 = create_test_bead("Bead two");
    let bead3 = create_test_bead("Bead three - no labels");

    // Add labels to bead1
    bf().arg("label")
        .arg("add")
        .arg(&bead1)
        .arg("--label")
        .arg("frontend")
        .arg("--label")
        .arg("ui")
        .output()
        .expect("Failed to add labels to bead1");

    // Add labels to bead2
    bf().arg("label")
        .arg("add")
        .arg(&bead2)
        .arg("--label")
        .arg("backend")
        .output()
        .expect("Failed to add labels to bead2");

    // List all beads with labels in text format (no bead ID specified)
    let output = bf()
        .arg("labels")
        .output()
        .expect("Failed to list all labels");

    assert!(
        output.status.success(),
        "Failed to list all labels: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should show all three beads
    assert!(stdout.contains(&bead1), "Missing bead1 in output");
    assert!(stdout.contains(&bead2), "Missing bead2 in output");
    assert!(stdout.contains(&bead3), "Missing bead3 in output");

    // Should show labels
    let text = stdout.to_lowercase();
    assert!(text.contains("frontend"), "Missing 'frontend' label");
    assert!(text.contains("ui"), "Missing 'ui' label");
    assert!(text.contains("backend"), "Missing 'backend' label");

    // Should indicate "(no labels)" for bead3
    assert!(stdout.contains("(no labels)"), "Missing '(no labels)' indicator");

    // Clean up
    bf().arg("close")
        .arg(&bead1)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead1");
    bf().arg("close")
        .arg(&bead2)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead2");
    bf().arg("close")
        .arg(&bead3)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead3");
}

#[test]
fn test_labels_json_format_single_bead() {
    // Test labels command in JSON format for a single bead
    let bead_id = create_test_bead("JSON format single bead test");

    // Add labels
    bf().arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("json-test")
        .arg("--label")
        .arg("validation")
        .output()
        .expect("Failed to add labels");

    // List labels in JSON format
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    assert!(
        output.status.success(),
        "Failed to list labels: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(labels.len(), 2, "Expected 2 labels, got {}", labels.len());
    assert!(labels.contains(&"json-test".to_string()), "Missing 'json-test' label");
    assert!(labels.contains(&"validation".to_string()), "Missing 'validation' label");

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_labels_json_format_all_beads() {
    // Test labels command in JSONL format showing all beads with labels
    let bead1 = create_test_bead("JSONL test bead 1");
    let bead2 = create_test_bead("JSONL test bead 2");

    // Add labels
    bf().arg("label")
        .arg("add")
        .arg(&bead1)
        .arg("--label")
        .arg("jsonl-test")
        .output()
        .expect("Failed to add labels to bead1");

    bf().arg("label")
        .arg("add")
        .arg(&bead2)
        .arg("--label")
        .arg("another-label")
        .output()
        .expect("Failed to add labels to bead2");

    // List all beads in JSON format (JSONL - one JSON object per line)
    let output = bf()
        .arg("labels")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list all labels in JSON");

    assert!(
        output.status.success(),
        "Failed to list all labels: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let lines: Vec<&str> = stdout.lines().collect();

    // Should have at least 2 lines (one per bead)
    assert!(lines.len() >= 2, "Expected at least 2 JSONL lines, got {}", lines.len());

    // Parse each line as JSON
    let mut found_bead1 = false;
    let mut found_bead2 = false;

    for line in lines {
        if line.trim().is_empty() || line.trim() == "[]" {
            continue;
        }
        if let Ok(obj) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                if id == bead1 {
                    found_bead1 = true;
                    let labels = obj.get("labels").and_then(|v| v.as_array());
                    assert!(labels.is_some(), "Missing labels array for bead1");
                    let labels = labels.unwrap();
                    assert!(
                        labels.iter().any(|l| l.as_str() == Some("jsonl-test")),
                        "Missing 'jsonl-test' label in bead1"
                    );
                } else if id == bead2 {
                    found_bead2 = true;
                    let labels = obj.get("labels").and_then(|v| v.as_array());
                    assert!(labels.is_some(), "Missing labels array for bead2");
                    let labels = labels.unwrap();
                    assert!(
                        labels.iter().any(|l| l.as_str() == Some("another-label")),
                        "Missing 'another-label' label in bead2"
                    );
                }
            }
        }
    }

    assert!(found_bead1, "Bead1 not found in JSONL output");
    assert!(found_bead2, "Bead2 not found in JSONL output");

    // Clean up
    bf().arg("close")
        .arg(&bead1)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead1");
    bf().arg("close")
        .arg(&bead2)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead2");
}

#[test]
fn test_label_persistence_through_sync_flush_only() {
    // Test that labels persist through sync --flush-only
    let bead_id = create_test_bead("Sync persistence test bead");

    // Add labels
    bf().arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("sync-test")
        .arg("--label")
        .arg("persistence")
        .output()
        .expect("Failed to add labels");

    // Flush to JSONL
    let flush_output = bf()
        .arg("sync")
        .arg("--flush-only")
        .output()
        .expect("Failed to flush");

    assert!(
        flush_output.status.success(),
        "Flush failed: {}",
        String::from_utf8_lossy(&flush_output.stderr)
    );

    // Verify labels are still in the database
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels after flush");

    assert!(
        output.status.success(),
        "Failed to list labels after flush: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(labels.len(), 2, "Expected 2 labels after flush, got {}", labels.len());
    assert!(labels.contains(&"sync-test".to_string()), "Missing 'sync-test' label after flush");
    assert!(labels.contains(&"persistence".to_string()), "Missing 'persistence' label after flush");

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_label_survival_after_removal_and_flush() {
    // Test that remaining labels survive after removing some labels and flushing
    let bead_id = create_test_bead("Label survival test bead");

    // Add multiple labels
    bf().arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("keep-this")
        .arg("--label")
        .arg("remove-this")
        .arg("--label")
        .arg("also-keep")
        .output()
        .expect("Failed to add labels");

    // Remove one label
    bf().arg("label")
        .arg("remove")
        .arg(&bead_id)
        .arg("--label")
        .arg("remove-this")
        .output()
        .expect("Failed to remove label");

    // Flush to JSONL
    let flush_output = bf()
        .arg("sync")
        .arg("--flush-only")
        .output()
        .expect("Failed to flush after removal");

    assert!(
        flush_output.status.success(),
        "Flush failed: {}",
        String::from_utf8_lossy(&flush_output.stderr)
    );

    // Verify remaining labels persist
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels after removal and flush");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(labels.len(), 2, "Expected 2 labels to survive, got {}", labels.len());
    assert!(labels.contains(&"keep-this".to_string()), "Missing 'keep-this' label");
    assert!(labels.contains(&"also-keep".to_string()), "Missing 'also-keep' label");
    assert!(!labels.contains(&"remove-this".to_string()), "'remove-this' should not be present");

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_label_with_special_characters() {
    // Test labels with special characters
    let bead_id = create_test_bead("Special chars label test");

    // Add labels with various special characters
    let special_labels = vec![
        "label-with-dash",
        "label_with_underscore",
        "label.with.dots",
        "label/with/slashes",
        "label:with:colons",
    ];

    for label in &special_labels {
        bf().arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to add label with special chars");
    }

    // Verify all labels persisted
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels with special chars");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(labels.len(), special_labels.len(), "Expected {} labels, got {}", special_labels.len(), labels.len());
    for label in &special_labels {
        assert!(labels.contains(&label.to_string()), "Missing label '{}'", label);
    }

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_label_with_unicode() {
    // Test labels with unicode characters
    let bead_id = create_test_bead("Unicode label test");

    let unicode_labels = vec![
        "日本語",      // Japanese
        "العربية",      // Arabic
        "unicode-test", // Mixed
    ];

    for label in &unicode_labels {
        bf().arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to add unicode label");
    }

    // Verify all unicode labels persisted
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list unicode labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(labels.len(), unicode_labels.len(), "Expected {} labels, got {}", unicode_labels.len(), labels.len());
    for label in &unicode_labels {
        assert!(labels.contains(&label.to_string()), "Missing unicode label '{}'", label);
    }

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_label_with_spaces() {
    // Test labels with spaces (should be supported)
    let bead_id = create_test_bead("Spaces in label test");

    // Add label with spaces
    bf().arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("label with spaces")
        .output()
        .expect("Failed to add label with spaces");

    // Verify label with spaces persisted
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels with spaces");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(labels.len(), 1, "Expected 1 label, got {}", labels.len());
    assert!(labels.contains(&"label with spaces".to_string()), "Missing label with spaces");

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_labels_empty_after_removal_sync() {
    // Test that a bead with no labels shows correctly after sync
    let bead_id = create_test_bead("Empty labels after sync test");

    // Add labels
    bf().arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("temporary")
        .output()
        .expect("Failed to add label");

    // Flush
    bf().arg("sync")
        .arg("--flush-only")
        .output()
        .expect("Failed to flush");

    // Remove all labels
    bf().arg("label")
        .arg("remove")
        .arg(&bead_id)
        .arg("--label")
        .arg("temporary")
        .output()
        .expect("Failed to remove label");

    // Flush again
    bf().arg("sync")
        .arg("--flush-only")
        .output()
        .expect("Failed to flush after removal");

    // Verify no labels remain
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(labels.len(), 0, "Expected 0 labels after removal, got {}", labels.len());

    // Also check in text format for all beads
    let output = bf()
        .arg("labels")
        .output()
        .expect("Failed to list all beads");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(stdout.contains("(no labels)"), "Missing '(no labels)' indicator for bead with no labels");

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_label_command_list_all_unique() {
    // Test `bf label list` (no bead ID) shows all unique labels with counts
    let bead1 = create_test_bead("Label list test bead 1");
    let bead2 = create_test_bead("Label list test bead 2");
    let bead3 = create_test_bead("Label list test bead 3");

    // Add labels with some overlap
    bf().arg("label")
        .arg("add")
        .arg(&bead1)
        .arg("--label")
        .arg("common")
        .arg("--label")
        .arg("unique-1")
        .output()
        .expect("Failed to add labels to bead1");

    bf().arg("label")
        .arg("add")
        .arg(&bead2)
        .arg("--label")
        .arg("common")
        .arg("--label")
        .arg("unique-2")
        .output()
        .expect("Failed to add labels to bead2");

    bf().arg("label")
        .arg("add")
        .arg(&bead3)
        .arg("--label")
        .arg("common")
        .output()
        .expect("Failed to add labels to bead3");

    // List all unique labels using `bf label list`
    let output = bf()
        .arg("label")
        .arg("list")
        .output()
        .expect("Failed to list all unique labels");

    assert!(
        output.status.success(),
        "Failed to list all labels: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should show "common (3)" since it's on 3 beads
    assert!(stdout.contains("common"), "Missing 'common' label");
    assert!(stdout.contains("3") || stdout.contains("(3)"), "Missing count for 'common' label");

    // Should show unique labels with count 1
    assert!(stdout.contains("unique-1"), "Missing 'unique-1' label");
    assert!(stdout.contains("unique-2"), "Missing 'unique-2' label");

    // Clean up
    bf().arg("close")
        .arg(&bead1)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead1");
    bf().arg("close")
        .arg(&bead2)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead2");
    bf().arg("close")
        .arg(&bead3)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead3");
}

#[test]
fn test_empty_label_handling() {
    // Test behavior when empty labels are provided
    let bead_id = create_test_bead("Empty label test bead");

    // Try to add an empty label
    let _output = bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("")
        .output();

    // Empty labels should either be rejected or result in no change
    // Verify that no empty label was added
    let list_output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    let json_output = String::from_utf8(list_output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    // Should have no labels or at least no empty strings
    assert!(!labels.contains(&"".to_string()), "Empty label should not be present");

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_label_deduplication() {
    // Test that duplicate labels are automatically deduplicated
    let bead_id = create_test_bead("Label deduplication test bead");

    // Add the same label multiple times
    for _ in 0..3 {
        bf().arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg("duplicate-test")
            .output()
            .expect("Failed to add label");
    }

    // Verify only one instance exists
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    // Count occurrences of "duplicate-test"
    let count = labels.iter().filter(|&l| l == "duplicate-test").count();
    assert_eq!(count, 1, "Expected 1 instance of label, found {}", count);

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_label_deduplication_multiple_labels() {
    // Test deduplication when adding multiple labels at once with duplicates
    let bead_id = create_test_bead("Multiple label dedup test bead");

    // Add multiple labels with duplicates in the same command
    bf().arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("label-a")
        .arg("--label")
        .arg("label-b")
        .arg("--label")
        .arg("label-a")  // Duplicate
        .arg("--label")
        .arg("label-c")
        .arg("--label")
        .arg("label-b")  // Duplicate
        .output()
        .expect("Failed to add labels");

    // Verify only unique labels exist
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    // Should have exactly 3 unique labels
    assert_eq!(labels.len(), 3, "Expected 3 unique labels, got {}", labels.len());
    assert!(labels.contains(&"label-a".to_string()), "Missing 'label-a'");
    assert!(labels.contains(&"label-b".to_string()), "Missing 'label-b'");
    assert!(labels.contains(&"label-c".to_string()), "Missing 'label-c'");

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_very_long_label_name() {
    // Test handling of very long label names
    let bead_id = create_test_bead("Long label test bead");

    // Create a very long label (1000 characters)
    let long_label = "a".repeat(1000);

    // Add the long label
    let output = bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg(&long_label)
        .output()
        .expect("Failed to execute label add command");

    // Either accept it or reject it - check the outcome
    if output.status.success() {
        // If accepted, verify it persisted correctly
        let list_output = bf()
            .arg("labels")
            .arg(&bead_id)
            .arg("--format")
            .arg("json")
            .output()
            .expect("Failed to list labels");

        let json_output = String::from_utf8(list_output.stdout).expect("Invalid UTF-8");
        let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

        assert!(labels.contains(&long_label), "Long label should be present");
        assert_eq!(labels.len(), 1, "Should have exactly 1 label");
    } else {
        // If rejected, that's also acceptable behavior
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("too long") || stderr.contains("invalid") || stderr.contains("error"),
            "Expected error message for long label, got: {}", stderr);
    }

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_label_whitespace_trimming() {
    // Test that leading and trailing whitespace is trimmed from labels
    let bead_id = create_test_bead("Label whitespace trimming test bead");

    // Add labels with extra whitespace
    bf().arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("  spaced-label  ")
        .arg("--label")
        .arg("\t tabbed-label\t")
        .arg("--label")
        .arg("  mixed-whitespace  ")
        .output()
        .expect("Failed to add labels with whitespace");

    // Verify labels are trimmed
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    // Labels should be trimmed (no leading/trailing whitespace)
    assert_eq!(labels.len(), 3, "Expected 3 labels, got {}", labels.len());
    assert!(labels.contains(&"spaced-label".to_string()), "Missing trimmed 'spaced-label'");
    assert!(labels.contains(&"tabbed-label".to_string()), "Missing trimmed 'tabbed-label'");
    assert!(labels.contains(&"mixed-whitespace".to_string()), "Missing trimmed 'mixed-whitespace'");

    // Verify no labels with whitespace exist
    for label in &labels {
        assert_eq!(label, label.trim(), "Label '{}' should be trimmed", label);
    }

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_label_deduplication_with_whitespace() {
    // Test that labels differing only by whitespace are treated as duplicates
    let bead_id = create_test_bead("Whitespace dedup test bead");

    // Add the same label with different whitespace
    bf().arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("test-label")
        .output()
        .expect("Failed to add first label");

    bf().arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("  test-label  ")
        .output()
        .expect("Failed to add label with spaces");

    bf().arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("\ttest-label\t")
        .output()
        .expect("Failed to add label with tabs");

    // Verify only one label exists (after trimming)
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(labels.len(), 1, "Expected 1 label after deduplication, got {}", labels.len());
    assert_eq!(labels[0], "test-label", "Label should be 'test-label'");

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_adding_many_labels() {
    // Test adding a large number of labels (50 labels)
    let bead_id = create_test_bead("Many labels test bead");

    // Create 50 unique labels
    let many_labels: Vec<String> = (0..50)
        .map(|i| format!("label-{:03}", i))
        .collect();

    // Add all labels
    for label in &many_labels {
        bf().arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to add label");
    }

    // Verify all labels persisted
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(labels.len(), 50, "Expected 50 labels, got {}", labels.len());

    // Verify all expected labels are present
    for label in &many_labels {
        assert!(labels.contains(label), "Missing label '{}'", label);
    }

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_remove_all_labels_from_bead() {
    // Test removing all labels from a bead with multiple labels
    let bead_id = create_test_bead("Remove all labels test bead");

    // Add multiple labels
    let labels_to_add = vec![
        "bug:critical",
        "feature/auth",
        "ui-component",
        "backend",
        "frontend",
        "high-priority",
    ];

    for label in &labels_to_add {
        bf().arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to add label");
    }

    // Verify all labels were added
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels before removal");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(labels.len(), labels_to_add.len(), "Expected {} labels before removal", labels_to_add.len());

    // Remove all labels one by one
    for label in &labels_to_add {
        bf().arg("label")
            .arg("remove")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to remove label");
    }

    // Verify no labels remain
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels after removal");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(labels.len(), 0, "Expected 0 labels after removing all, got {}", labels.len());

    // Also verify in text format shows "(no labels)"
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .output()
        .expect("Failed to list labels in text format");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(stdout.contains("(no labels)"), "Expected '(no labels)' indicator in text format");

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_remove_label_from_bead_with_no_labels() {
    // Test removing a label from a bead that has no labels
    let bead_id = create_test_bead("No labels test bead");

    // Verify bead has no labels initially
    let initial_output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list initial labels");

    let json_output = String::from_utf8(initial_output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(labels.len(), 0, "Expected 0 labels initially");

    // Attempt to remove a label from the empty bead
    let remove_output = bf()
        .arg("label")
        .arg("remove")
        .arg(&bead_id)
        .arg("--label")
        .arg("nonexistent-label")
        .output()
        .expect("Failed to run label remove command");

    // Should succeed (idempotent - no-op on non-existent label)
    assert!(
        remove_output.status.success(),
        "Removing from empty label list should succeed: {}",
        String::from_utf8_lossy(&remove_output.stderr)
    );

    // Verify still no labels
    let final_output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels after removal");

    let json_output = String::from_utf8(final_output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(labels.len(), 0, "Expected 0 labels after attempting to remove from empty list");

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_empty_label_rejection() {
    // Test that empty labels are explicitly rejected
    let bead_id = create_test_bead("Empty label rejection test bead");

    // Try to add an empty label - should fail or be no-op
    let _output = bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("")
        .output();

    // Command should either fail (rejected) or succeed with no change (idempotent no-op)
    // Either way, verify no empty label was actually added
    let list_output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    let json_output = String::from_utf8(list_output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    // Should have no labels, and definitely no empty strings
    assert_eq!(labels.len(), 0, "Should have no labels after attempting to add empty label");
    assert!(!labels.contains(&"".to_string()), "Empty string should not be in labels");

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_single_character_label() {
    // Test that single-character labels are accepted
    let bead_id = create_test_bead("Single char label test bead");

    // Test various single-character labels
    let single_char_labels = vec!["a", "Z", "0", "9", "-", "_", "."];

    for label in &single_char_labels {
        let output = bf()
            .arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to add label");

        assert!(
            output.status.success(),
            "Single-character label '{}' should be accepted: {}",
            label,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Verify all single-character labels persisted
    let list_output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    let json_output = String::from_utf8(list_output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(labels.len(), single_char_labels.len(), "Expected {} single-character labels", single_char_labels.len());
    for label in &single_char_labels {
        assert!(labels.contains(&label.to_string()), "Missing single-character label '{}'", label);
    }

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_whitespace_only_label() {
    // Test that whitespace-only labels are rejected or trimmed to empty
    let bead_id = create_test_bead("Whitespace-only label test bead");

    // Try to add various whitespace-only labels
    let whitespace_labels = vec!["   ", "\t", "\n", "  \t  ", "   \n   "];

    for label in &whitespace_labels {
        let _output = bf()
            .arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output();

        // Should either fail (rejected) or succeed with no actual label added
        // (trimmed to empty string, which is then rejected or ignored)
    }

    // Verify no whitespace-only labels were added
    let list_output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    let json_output = String::from_utf8(list_output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(labels.len(), 0, "Whitespace-only labels should not be added");

    // Verify none of the labels are empty or whitespace-only
    for label in &labels {
        assert!(!label.trim().is_empty(), "Label '{}' should not be whitespace-only", label);
    }

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
fn test_mixed_whitespace_content_label() {
    // Test labels that mix actual content with whitespace
    let bead_id = create_test_bead("Mixed whitespace content test bead");

    // Add labels with content surrounded by whitespace - should be trimmed
    let labels_with_whitespace = vec![
        ("  a  ", "a"),
        ("\tb\t", "b"),
        ("  c  ", "c"),
        ("\td\n", "d"),
    ];

    for (input, expected) in &labels_with_whitespace {
        let output = bf()
            .arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(input)
            .output()
            .expect("Failed to add label with whitespace");

        assert!(
            output.status.success(),
            "Label '{}' should be accepted and trimmed to '{}': {}",
            input,
            expected,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Verify labels were trimmed correctly
    let list_output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels");

    let json_output = String::from_utf8(list_output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(labels.len(), labels_with_whitespace.len(), "Expected {} labels after trimming", labels_with_whitespace.len());

    // Verify each expected trimmed label is present
    for (_input, expected) in &labels_with_whitespace {
        assert!(labels.contains(&expected.to_string()), "Missing trimmed label '{}'", expected);
    }

    // Verify no labels have leading/trailing whitespace
    for label in &labels {
        assert_eq!(label, label.trim(), "Label '{}' should have no leading/trailing whitespace", label);
    }

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}
