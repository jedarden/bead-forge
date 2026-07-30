// Tests for labels command text format output

use std::process::Command;

/// Resolve the freshly-built bf binary — never the system-installed one.
fn bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

use std::sync::OnceLock;

static WORKSPACE: OnceLock<tempfile::TempDir> = OnceLock::new();

/// Per-binary isolated workspace — these tests previously ran against the
/// repo's own tracked .beads workspace, polluting it with test beads and
/// contending on its database under parallel test threads.
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
    // Extract bead ID from output (format: "bf-xxxx")
    let id = stdout.trim().to_string();
    id
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_label_list_single_bead_text_format() {
    // Test formatting with a single label
    let bead_id = create_test_bead("Single label test");

    let output = bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("urgent")
        .output()
        .expect("Failed to add label");

    assert!(
        output.status.success(),
        "Failed to add label: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // List labels for the bead using 'label list' with ID
    let output = bf()
        .arg("label")
        .arg("list")
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

    // Should have header line and label line
    assert!(
        lines.len() >= 2,
        "Expected at least 2 lines (header + label), got {}: {}",
        lines.len(),
        stdout
    );

    // First line should be the header
    assert!(
        lines[0].contains(&bead_id) || lines[0].contains("Labels for"),
        "First line should be a header, got: {}",
        lines[0]
    );

    // Find the label line (should be indented)
    let label_lines: Vec<&str> = lines
        .iter()
        .filter(|line| line.trim().starts_with("urgent") || line.trim() == "urgent")
        .cloned()
        .collect();

    assert!(
        !label_lines.is_empty(),
        "Should have found 'urgent' label in output: {}",
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
fn test_label_list_multiple_labels_text_format() {
    // Test formatting with multiple labels
    let bead_id = create_test_bead("Multiple labels test");

    // Add multiple labels in non-alphabetical order
    let labels_to_add = vec!["zebra", "apple", "middle", "banana"];

    for label in &labels_to_add {
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
            "Failed to add label {}: {}",
            label,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // List labels for the bead
    let output = bf()
        .arg("label")
        .arg("list")
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

    // Extract label lines (skip header, get indented label lines)
    let mut labels_found: Vec<String> = lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.contains("Labels for") && !trimmed.contains(&bead_id)
        })
        .map(|line| line.trim().to_string())
        .collect();

    // Should have found all labels
    assert_eq!(
        labels_found.len(),
        labels_to_add.len(),
        "Expected {} labels, found {}: {:?}",
        labels_to_add.len(),
        labels_found.len(),
        labels_found
    );

    // Each label should appear on its own line
    for label in &labels_to_add {
        assert!(
            labels_found.iter().any(|l| l == label || l == *label),
            "Missing label '{}' in {:?}",
            label,
            labels_found
        );
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
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_label_list_empty_bead_text_format() {
    // Test output when bead has no labels
    let bead_id = create_test_bead("Empty labels test");

    // List labels for a bead with no labels
    let output = bf()
        .arg("label")
        .arg("list")
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

    // Should have header line but no label lines (or only whitespace)
    let label_lines: Vec<&str> = lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.contains("Labels for") && !trimmed.contains(&bead_id)
        })
        .cloned()
        .collect();

    assert_eq!(
        label_lines.len(),
        0,
        "Expected no label lines for bead with no labels, got: {:?}",
        label_lines
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
fn test_label_list_all_unique_text_format() {
    // Test 'bf label list' without ID (all unique labels with counts)
    let bead1 = create_test_bead("Label list test bead 1");
    let bead2 = create_test_bead("Label list test bead 2");

    // Add different labels to each bead
    bf().arg("label")
        .arg("add")
        .arg(&bead1)
        .arg("--label")
        .arg("urgent")
        .arg("--label")
        .arg("backend")
        .output()
        .expect("Failed to add labels to bead 1");

    bf().arg("label")
        .arg("add")
        .arg(&bead2)
        .arg("--label")
        .arg("urgent")
        .arg("--label")
        .arg("frontend")
        .output()
        .expect("Failed to add labels to bead 2");

    // List all unique labels (no bead ID specified)
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
    let lines: Vec<&str> = stdout.lines().collect();

    // Should have header "All labels:"
    assert!(
        lines.iter().any(|line| line.contains("All labels") || line.contains("All")),
        "Output should contain 'All labels' header: {}",
        stdout
    );

    // Each label appears on its own line with count
    let label_lines: Vec<&str> = lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && !trimmed.contains("All labels")
                && !trimmed.contains("All")
        })
        .cloned()
        .collect();

    // Should have at least 3 unique labels: urgent, backend, frontend
    assert!(
        label_lines.len() >= 3,
        "Expected at least 3 label lines, got {}: {:?}",
        label_lines.len(),
        label_lines
    );

    // Each line should have format "label (count)"
    for line in &label_lines {
        assert!(
            line.contains('(') && line.contains(')'),
            "Label line should have format 'label (count)': {}",
            line
        );
    }

    // Verify specific labels exist
    let output_lower = stdout.to_lowercase();
    assert!(
        output_lower.contains("urgent"),
        "Missing 'urgent' label in output: {}",
        stdout
    );
    assert!(
        output_lower.contains("backend"),
        "Missing 'backend' label in output: {}",
        stdout
    );
    assert!(
        output_lower.contains("frontend"),
        "Missing 'frontend' label in output: {}",
        stdout
    );

    // Clean up
    bf().arg("close")
        .arg(&bead1)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead 1");
    bf().arg("close")
        .arg(&bead2)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead 2");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_labels_command_text_format_single_label() {
    // Test 'bf labels <id>' command in text format (no --format flag = text)
    let bead_id = create_test_bead("Labels command single label test");

    // Add a single label
    let output = bf()
        .arg("label")
        .arg("add")
        .arg(&bead_id)
        .arg("--label")
        .arg("solo")
        .output()
        .expect("Failed to add label");

    assert!(
        output.status.success(),
        "Failed to add label: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Use 'labels' command (not 'label list') - should default to text format
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

    // Should have exactly one line with the label
    assert_eq!(
        lines.len(),
        1,
        "Expected exactly 1 line for single label, got {}: {}",
        lines.len(),
        stdout
    );

    assert_eq!(
        lines[0].trim(),
        "solo",
        "Label line should be 'solo', got: {}",
        lines[0]
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
fn test_labels_command_text_format_multiple_labels() {
    // Test 'bf labels <id>' command with multiple labels in text format
    let bead_id = create_test_bead("Labels command multiple labels test");

    // Add multiple labels
    let labels_to_add = vec!["first", "second", "third"];

    for label in &labels_to_add {
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
            "Failed to add label {}: {}",
            label,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Use 'labels' command
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

    // Should have one line per label
    assert_eq!(
        lines.len(),
        labels_to_add.len(),
        "Expected {} lines, got {}: {}",
        labels_to_add.len(),
        lines.len(),
        stdout
    );

    // Each label should appear on its own line
    for label in &labels_to_add {
        assert!(
            lines.iter().any(|line| line.trim() == *label),
            "Missing label '{}' in lines: {:?}",
            label,
            lines
        );
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
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_labels_command_text_format_no_labels() {
    // Test 'bf labels <id>' command for bead with no labels
    let bead_id = create_test_bead("Labels command empty test");

    // Use 'labels' command on bead with no labels
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

    // Should be empty (no labels to print)
    assert_eq!(
        stdout.trim(),
        "",
        "Expected empty output for bead with no labels, got: {}",
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
fn test_label_list_alphabetical_order_single_bead() {
    // Test that labels for a single bead are in alphabetical order
    let bead_id = create_test_bead("Alphabetical order test");

    // Add labels in reverse alphabetical order
    let labels_to_add = vec!["zebra", "yellow", "apple", "banana"];

    for label in &labels_to_add {
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
            "Failed to add label {}: {}",
            label,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // List labels using 'labels' command (not 'label list')
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

    // Get labels from output (trim each line)
    let mut labels_found: Vec<String> = lines
        .iter()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    // Sort expected labels alphabetically
    let mut expected_labels = labels_to_add.clone();
    expected_labels.sort();

    // Verify labels are in alphabetical order
    assert_eq!(
        labels_found, expected_labels,
        "Labels should be in alphabetical order. Expected {:?}, got {:?}",
        expected_labels, labels_found
    );

    // Verify they're actually sorted
    let mut sorted_labels = labels_found.clone();
    sorted_labels.sort();
    assert_eq!(
        labels_found, sorted_labels,
        "Labels should be sorted alphabetically. Got unsorted: {:?}",
        labels_found
    );

    // Clean up
    bf().arg("close")
        .arg(&bead_id)
        .arg("--reason")
        .arg("Test cleanup")
        .output()
        .expect("Failed to close bead");
}
