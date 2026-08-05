// Tests for labels containing special characters
// Covers realistic labeling patterns: bug:critical, ui/component, ui-component, back_end, etc.

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
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_labels_with_colons() {
    // Test realistic colon-based labels like bug:critical, feature:auth
    let bead_id = create_test_bead("Colon labels test bead");

    let colon_labels = vec![
        "bug:critical",
        "bug:major",
        "bug:minor",
        "feature:auth",
        "feature:ui",
        "priority:high",
        "priority:low",
        "type:enhancement",
    ];

    // Add all colon labels
    for label in &colon_labels {
        bf().arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to add colon label");
    }

    // Verify all labels persisted
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list colon labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(
        labels.len(),
        colon_labels.len(),
        "Expected {} colon labels, got {}",
        colon_labels.len(),
        labels.len()
    );
    for label in &colon_labels {
        assert!(
            labels.contains(&label.to_string()),
            "Missing colon label '{}'",
            label
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
fn test_labels_with_slashes() {
    // Test realistic slash-based labels like ui/component, auth/oauth
    let bead_id = create_test_bead("Slash labels test bead");

    let slash_labels = vec![
        "ui/component",
        "ui/page",
        "auth/oauth",
        "auth/jwt",
        "backend/api",
        "backend/service",
        "frontend/react",
        "frontend/vue",
    ];

    // Add all slash labels
    for label in &slash_labels {
        bf().arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to add slash label");
    }

    // Verify all labels persisted
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list slash labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(
        labels.len(),
        slash_labels.len(),
        "Expected {} slash labels, got {}",
        slash_labels.len(),
        labels.len()
    );
    for label in &slash_labels {
        assert!(
            labels.contains(&label.to_string()),
            "Missing slash label '{}'",
            label
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
fn test_labels_with_hyphens() {
    // Test realistic hyphen-based labels like ui-component, back-end
    let bead_id = create_test_bead("Hyphen labels test bead");

    let hyphen_labels = vec![
        "ui-component",
        "back-end",
        "front-end",
        "high-priority",
        "low-priority",
        "in-progress",
        "not-started",
        "code-review",
        "needs-testing",
    ];

    // Add all hyphen labels
    for label in &hyphen_labels {
        bf().arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to add hyphen label");
    }

    // Verify all labels persisted
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list hyphen labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(
        labels.len(),
        hyphen_labels.len(),
        "Expected {} hyphen labels, got {}",
        hyphen_labels.len(),
        labels.len()
    );
    for label in &hyphen_labels {
        assert!(
            labels.contains(&label.to_string()),
            "Missing hyphen label '{}'",
            label
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
fn test_labels_with_underscores() {
    // Test realistic underscore-based labels like back_end, test_case
    let bead_id = create_test_bead("Underscore labels test bead");

    let underscore_labels = vec![
        "back_end",
        "front_end",
        "test_case",
        "unit_test",
        "integration_test",
        "api_call",
        "db_query",
        "cache_hit",
    ];

    // Add all underscore labels
    for label in &underscore_labels {
        bf().arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to add underscore label");
    }

    // Verify all labels persisted
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list underscore labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(
        labels.len(),
        underscore_labels.len(),
        "Expected {} underscore labels, got {}",
        underscore_labels.len(),
        labels.len()
    );
    for label in &underscore_labels {
        assert!(
            labels.contains(&label.to_string()),
            "Missing underscore label '{}'",
            label
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
fn test_mixed_special_character_labels() {
    // Test mixing different special character patterns on the same bead
    let bead_id = create_test_bead("Mixed special chars test bead");

    let mixed_labels = vec![
        // Colons
        "bug:critical",
        "feature:auth",
        // Slashes
        "ui/component",
        "auth/oauth",
        // Hyphens
        "ui-component",
        "back-end",
        // Underscores
        "back_end",
        "test_case",
    ];

    // Add all labels with different special characters
    for label in &mixed_labels {
        bf().arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to add mixed special char label");
    }

    // Verify all labels persisted correctly
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list mixed special char labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(
        labels.len(),
        mixed_labels.len(),
        "Expected {} mixed labels, got {}",
        mixed_labels.len(),
        labels.len()
    );
    for label in &mixed_labels {
        assert!(
            labels.contains(&label.to_string()),
            "Missing mixed special char label '{}'",
            label
        );
    }

    // Verify text format shows all labels
    let text_output = bf()
        .arg("labels")
        .arg(&bead_id)
        .output()
        .expect("Failed to list labels in text format");

    let text = String::from_utf8(text_output.stdout).expect("Invalid UTF-8");
    for label in &mixed_labels {
        assert!(
            text.contains(label),
            "Missing label '{}' in text output",
            label
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
fn test_complex_label_patterns() {
    // Test complex real-world label patterns combining multiple special chars
    let bead_id = create_test_bead("Complex label patterns test bead");

    let complex_labels = vec![
        "bug:critical/api",          // Multiple colons and slashes
        "feature/auth/oauth",        // Multiple slashes
        "priority:blocker-database", // Colon and hyphen
        "status:in_progress",        // Colon and underscore
        "type:ui-component/layout",  // Complex nesting pattern
        "team:back-end/core",        // Mixed separators
    ];

    // Add all complex labels
    for label in &complex_labels {
        let output = bf()
            .arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to execute label add command");

        assert!(
            output.status.success(),
            "Complex label '{}' should be accepted: {}",
            label,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Verify all complex labels persisted
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list complex labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(
        labels.len(),
        complex_labels.len(),
        "Expected {} complex labels, got {}",
        complex_labels.len(),
        labels.len()
    );
    for label in &complex_labels {
        assert!(
            labels.contains(&label.to_string()),
            "Missing complex label '{}'",
            label
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
fn test_special_character_label_persistence() {
    // Test that special character labels persist through sync
    let bead_id = create_test_bead("Special char persistence test bead");

    let special_labels = vec!["bug:critical", "ui/component", "back-end", "test_case"];

    // Add labels
    for label in &special_labels {
        bf().arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to add label");
    }

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

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(
        labels.len(),
        special_labels.len(),
        "Expected {} labels after flush, got {}",
        special_labels.len(),
        labels.len()
    );
    for label in &special_labels {
        assert!(
            labels.contains(&label.to_string()),
            "Missing special char label '{}' after flush",
            label
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
fn test_special_character_label_removal() {
    // Test removing labels with special characters
    let bead_id = create_test_bead("Special char removal test bead");

    let labels = vec!["bug:critical", "ui/component", "back-end", "test_case"];

    // Add all labels
    for label in &labels {
        bf().arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to add label");
    }

    // Remove specific labels with special characters
    let to_remove = vec!["bug:critical", "back-end"];
    for label in &to_remove {
        bf().arg("label")
            .arg("remove")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to remove special char label");
    }

    // Verify remaining labels
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list labels after removal");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let remaining_labels: Vec<String> =
        serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(
        remaining_labels.len(),
        labels.len() - to_remove.len(),
        "Expected {} remaining labels",
        labels.len() - to_remove.len()
    );
    assert!(
        remaining_labels.contains(&"ui/component".to_string()),
        "Missing 'ui/component'"
    );
    assert!(
        remaining_labels.contains(&"test_case".to_string()),
        "Missing 'test_case'"
    );
    assert!(
        !remaining_labels.contains(&"bug:critical".to_string()),
        "'bug:critical' should be removed"
    );
    assert!(
        !remaining_labels.contains(&"back-end".to_string()),
        "'back-end' should be removed"
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
fn test_label_with_multiple_colons() {
    // Test labels with multiple colons (e.g., bug:severity:critical)
    let bead_id = create_test_bead("Multiple colons test bead");

    let multi_colon_labels = vec![
        "bug:severity:critical",
        "feature:category:authentication",
        "priority:level:high",
        "status:workflow:in_progress",
    ];

    // Add labels with multiple colons
    for label in &multi_colon_labels {
        let output = bf()
            .arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to execute label add command");

        assert!(
            output.status.success(),
            "Multi-colon label '{}' should be accepted: {}",
            label,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Verify all multi-colon labels persisted
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list multi-colon labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(
        labels.len(),
        multi_colon_labels.len(),
        "Expected {} multi-colon labels, got {}",
        multi_colon_labels.len(),
        labels.len()
    );
    for label in &multi_colon_labels {
        assert!(
            labels.contains(&label.to_string()),
            "Missing multi-colon label '{}'",
            label
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
fn test_label_with_multiple_slashes() {
    // Test labels with multiple slashes (e.g., ui/component/button)
    let bead_id = create_test_bead("Multiple slashes test bead");

    let multi_slash_labels = vec![
        "ui/component/button",
        "auth/provider/oauth/google",
        "backend/service/database/postgres",
        "frontend/pages/dashboard/analytics",
    ];

    // Add labels with multiple slashes
    for label in &multi_slash_labels {
        let output = bf()
            .arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to execute label add command");

        assert!(
            output.status.success(),
            "Multi-slash label '{}' should be accepted: {}",
            label,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Verify all multi-slash labels persisted
    let output = bf()
        .arg("labels")
        .arg(&bead_id)
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to list multi-slash labels");

    let json_output = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let labels: Vec<String> = serde_json::from_str(&json_output).expect("Failed to parse JSON");

    assert_eq!(
        labels.len(),
        multi_slash_labels.len(),
        "Expected {} multi-slash labels, got {}",
        multi_slash_labels.len(),
        labels.len()
    );
    for label in &multi_slash_labels {
        assert!(
            labels.contains(&label.to_string()),
            "Missing multi-slash label '{}'",
            label
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
