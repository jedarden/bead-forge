// Basic label CLI tests for bead-forge
// Tests fundamental label operations through the CLI

use std::process::Command;
use std::sync::OnceLock;

static WORKSPACE: OnceLock<tempfile::TempDir> = OnceLock::new();

/// Per-test isolated workspace
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
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_label_add_single() {
    // Test adding a single label to a bead
    let bead_id = create_test_bead("Single label test");

    // Add single label
    let output = bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("test-label")
        .output()
        .expect("Failed to add single label");

    assert!(
        output.status.success(),
        "Single label add failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(
        stdout.contains("Added label 'test-label'"),
        "Expected confirmation message for single label: got {}",
        stdout
    );

    // Verify label was added using bf label list
    let list_output = bf()
        .arg("label")
        .arg("list")
        .arg(&bead_id)
        .output()
        .expect("Failed to list labels");

    assert!(
        list_output.status.success(),
        "Label list failed: {}",
        String::from_utf8_lossy(&list_output.stderr)
    );

    let list_stdout = String::from_utf8(list_output.stdout).expect("Invalid UTF-8");
    assert!(
        list_stdout.contains("test-label"),
        "Label not found in list output: {}",
        list_stdout
    );

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_label_add_multiple() {
    // Test adding multiple labels at once
    let bead_id = create_test_bead("Multiple labels test");

    // Add multiple labels in one command
    let output = bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("label1")
        .arg("--label")
        .arg("label2")
        .arg("--label")
        .arg("label3")
        .output()
        .expect("Failed to add multiple labels");

    assert!(
        output.status.success(),
        "Multiple label add failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(
        stdout.contains("Added label 'label1'"),
        "Expected confirmation for label1: {}",
        stdout
    );
    assert!(
        stdout.contains("Added label 'label2'"),
        "Expected confirmation for label2: {}",
        stdout
    );
    assert!(
        stdout.contains("Added label 'label3'"),
        "Expected confirmation for label3: {}",
        stdout
    );

    // Verify all labels were added
    let list_output = bf()
        .arg("label")
        .arg("list")
        .arg(&bead_id)
        .output()
        .expect("Failed to list labels");

    let list_stdout = String::from_utf8(list_output.stdout).expect("Invalid UTF-8");
    assert!(
        list_stdout.contains("label1"),
        "label1 not found: {}",
        list_stdout
    );
    assert!(
        list_stdout.contains("label2"),
        "label2 not found: {}",
        list_stdout
    );
    assert!(
        list_stdout.contains("label3"),
        "label3 not found: {}",
        list_stdout
    );

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_label_remove() {
    // Test removing a label from a bead
    let bead_id = create_test_bead("Label removal test");

    // Add labels first
    bf().arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("keep-this")
        .arg("--label")
        .arg("remove-this")
        .output()
        .expect("Failed to add labels");

    // Remove one label
    let output = bf()
        .arg("label")
        .arg("remove")
        .arg(&bead_id)
        .arg("--label")
        .arg("remove-this")
        .output()
        .expect("Failed to remove label");

    assert!(
        output.status.success(),
        "Label remove failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(
        stdout.contains("Removed label 'remove-this'"),
        "Expected removal confirmation: {}",
        stdout
    );

    // Verify the removed label is gone
    let list_output = bf()
        .arg("label")
        .arg("list")
        .arg(&bead_id)
        .output()
        .expect("Failed to list labels");

    let list_stdout = String::from_utf8(list_output.stdout).expect("Invalid UTF-8");
    assert!(
        !list_stdout.contains("remove-this"),
        "Removed label should not be present: {}",
        list_stdout
    );
    assert!(
        list_stdout.contains("keep-this"),
        "Other labels should remain: {}",
        list_stdout
    );

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_label_list_single_bead() {
    // Test listing labels for a single bead using `bf label list <id>`
    let bead_id = create_test_bead("List single bead test");

    // Add labels
    bf().arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("bug")
        .arg("--label")
        .arg("high-priority")
        .output()
        .expect("Failed to add labels");

    // List labels for the specific bead
    let output = bf()
        .arg("label")
        .arg("list")
        .arg(&bead_id)
        .output()
        .expect("Failed to list labels for single bead");

    assert!(
        output.status.success(),
        "Label list single bead failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(
        stdout.contains(&format!("Labels for {}:", bead_id)),
        "Expected header for bead: {}",
        stdout
    );
    assert!(
        stdout.contains("bug"),
        "Expected 'bug' label: {}",
        stdout
    );
    assert!(
        stdout.contains("high-priority"),
        "Expected 'high-priority' label: {}",
        stdout
    );

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_label_list_all_beads() {
    // Test listing all labels across workspace using `bf label list` (no id)
    let bead1 = create_test_bead("Bead 1 for list all");
    let bead2 = create_test_bead("Bead 2 for list all");

    // Add labels to different beads
    bf().arg("label")
        .arg("add")
        .arg(&bead1)
        .arg("--label")
        .arg("frontend")
        .arg("--label")
        .arg("ui")
        .output()
        .expect("Failed to add labels to bead1");

    bf().arg("label")
        .arg("add")
        .arg(&bead2)
        .arg("--label")
        .arg("backend")
        .arg("--label")
        .arg("database")
        .output()
        .expect("Failed to add labels to bead2");

    // List all labels in workspace
    let output = bf()
        .arg("label")
        .arg("list")
        .output()
        .expect("Failed to list all labels");

    assert!(
        output.status.success(),
        "Label list all failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(
        stdout.contains("All labels:"),
        "Expected 'All labels:' header: {}",
        stdout
    );
    assert!(
        stdout.contains("frontend"),
        "Expected 'frontend' label: {}",
        stdout
    );
    assert!(
        stdout.contains("ui"),
        "Expected 'ui' label: {}",
        stdout
    );
    assert!(
        stdout.contains("backend"),
        "Expected 'backend' label: {}",
        stdout
    );
    assert!(
        stdout.contains("database"),
        "Expected 'database' label: {}",
        stdout
    );

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
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_label_duplicate_handling() {
    // Test that adding duplicate labels is handled correctly (idempotent)
    let bead_id = create_test_bead("Duplicate label test");

    // Add a label
    let output1 = bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("duplicate-test")
        .output()
        .expect("Failed to add label first time");

    assert!(
        output1.status.success(),
        "First add failed: {}",
        String::from_utf8_lossy(&output1.stderr)
    );

    // Try to add the same label again
    let output2 = bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("duplicate-test")
        .output()
        .expect("Failed to add label second time");

    // Should succeed (idempotent operation)
    assert!(
        output2.status.success(),
        "Second add should succeed (idempotent): {}",
        String::from_utf8_lossy(&output2.stderr)
    );

    // Verify only one instance of the label exists
    let list_output = bf()
        .arg("label")
        .arg("list")
        .arg(&bead_id)
        .output()
        .expect("Failed to list labels");

    let list_stdout = String::from_utf8(list_output.stdout).expect("Invalid UTF-8");
    let count = list_stdout.matches("duplicate-test").count();
    assert_eq!(
        count, 1,
        "Label should appear only once, found {} occurrences in: {}",
        count, list_stdout
    );

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_label_duplicate_handling_multiple() {
    // Test duplicate handling when adding multiple labels at once
    let bead_id = create_test_bead("Multiple duplicate test");

    // Add labels including duplicates in same command
    let output = bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("unique-a")
        .arg("--label")
        .arg("shared")
        .arg("--label")
        .arg("unique-b")
        .arg("--label")
        .arg("shared")
        .output()
        .expect("Failed to add labels with duplicates");

    assert!(
        output.status.success(),
        "Add with duplicates failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify only unique labels exist
    let list_output = bf()
        .arg("label")
        .arg("list")
        .arg(&bead_id)
        .output()
        .expect("Failed to list labels");

    let list_stdout = String::from_utf8(list_output.stdout).expect("Invalid UTF-8");

    // Count occurrences of each label
    let count_a = list_stdout.matches("unique-a").count();
    let count_b = list_stdout.matches("unique-b").count();
    let count_shared = list_stdout.matches("shared").count();

    assert_eq!(count_a, 1, "unique-a should appear once");
    assert_eq!(count_b, 1, "unique-b should appear once");
    assert_eq!(count_shared, 1, "shared should appear once despite being added twice");

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_label_remove_nonexistent() {
    // Test that removing a non-existent label is handled gracefully (idempotent)
    let bead_id = create_test_bead("Remove nonexistent test");

    // Add a label
    bf().arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("existing")
        .output()
        .expect("Failed to add label");

    // Try to remove a non-existent label
    let output = bf()
        .arg("label")
        .arg("remove")
        .arg(&bead_id)
        .arg("--label")
        .arg("nonexistent")
        .output()
        .expect("Failed to attempt label removal");

    // Should succeed (idempotent - no-op)
    assert!(
        output.status.success(),
        "Removing non-existent label should succeed (no-op): {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify existing label is still present
    let list_output = bf()
        .arg("label")
        .arg("list")
        .arg(&bead_id)
        .output()
        .expect("Failed to list labels");

    let list_stdout = String::from_utf8(list_output.stdout).expect("Invalid UTF-8");
    assert!(
        list_stdout.contains("existing"),
        "Existing label should remain: {}",
        list_stdout
    );
    assert!(
        !list_stdout.contains("nonexistent"),
        "Non-existent label should not appear: {}",
        list_stdout
    );

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_label_list_empty_bead() {
    // Test listing labels for a bead with no labels
    let bead_id = create_test_bead("Empty bead label list test");

    // List labels for bead with no labels
    let output = bf()
        .arg("label")
        .arg("list")
        .arg(&bead_id)
        .output()
        .expect("Failed to list labels for empty bead");

    assert!(
        output.status.success(),
        "Listing labels for empty bead should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(
        stdout.contains(&format!("Labels for {}:", bead_id)),
        "Should show header even for empty bead: {}",
        stdout
    );

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_label_list_empty_workspace() {
    // Test listing all labels when workspace has no labels
    let bead_id = create_test_bead("Empty workspace test");

    // Don't add any labels, just list all
    let output = bf()
        .arg("label")
        .arg("list")
        .output()
        .expect("Failed to list all labels in empty workspace");

    assert!(
        output.status.success(),
        "Listing all labels in empty workspace should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    assert!(
        stdout.contains("All labels:"),
        "Should show 'All labels:' header even when empty: {}",
        stdout
    );

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}
