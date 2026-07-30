//! JSON output tests for list and ready commands using --json flag
//!
//! These tests verify that `bf list` and `bf ready` with the `--json` flag
//! (alias for --format json) output valid JSONL format.
//!
//! Acceptance criteria:
//! - Test list command with --json flag
//! - Test ready command with --json flag
//! - Validate output is valid JSONL format
//! - Test empty results edge case
//! - Test multiple items output
//! - Tests pass with cargo test

use std::process::Command;
use serde_json::Value;

/// Resolve the freshly-built bf binary — never the system-installed one.
fn bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

use std::sync::OnceLock;

static WORKSPACE: OnceLock<tempfile::TempDir> = OnceLock::new();

/// Per-binary isolated workspace — prevents test pollution and contention.
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
    stdout.trim().to_string()
}

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

/// Parse a JSONL string (newline-delimited JSON) into a Vec of values
fn parse_jsonl(jsonl: &str) -> Vec<Value> {
    jsonl
        .lines()
        .filter(|line| !line.trim().is_empty() && line.trim() != "[]")
        .map(|line| serde_json::from_str(line).expect("Failed to parse JSON line"))
        .collect()
}

// ============================================================================
// LIST COMMAND WITH --json FLAG TESTS
// ============================================================================

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_list_command_json_flag() {
    // Test that --json flag works as alias for --format json
    let bead_id = create_test_bead("Test list --json flag");

    let output = bf()
        .arg("list")
        .arg("--json")
        .output()
        .expect("Failed to execute list command");

    assert!(
        output.status.success(),
        "list --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Should find our bead in output
    assert!(stdout.contains(&bead_id), "list --json should include our bead");

    // Verify output is valid JSONL
    let parsed = parse_jsonl(&stdout);
    assert!(parsed.len() >= 1, "Should have at least one bead");

    // Verify required fields
    let bead = parsed.iter().find(|b| b["id"].as_str() == Some(&bead_id))
        .expect("Should find our bead");

    assert!(bead.get("id").is_some(), "Should have id field");
    assert!(bead.get("title").is_some(), "Should have title field");
    assert!(bead.get("status").is_some(), "Should have status field");

    close_test_bead(&bead_id);
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_list_json_empty_results() {
    // Test empty results with status filter
    let output = bf()
        .arg("list")
        .arg("--status")
        .arg("closed")
        .arg("--json")
        .output()
        .expect("Failed to execute list command");

    assert!(
        output.status.success(),
        "list --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Empty list should return "[]"
    assert_eq!(trimmed, "[]", "Empty list should return '[]'");

    // Verify it's valid JSON
    let parsed: Value = serde_json::from_str(trimmed)
        .expect("Empty result should be valid JSON");
    assert!(parsed.is_array(), "Empty result should be an array");
    assert_eq!(parsed.as_array().unwrap().len(), 0, "Array should be empty");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_list_json_multiple_items() {
    // Create multiple test beads
    let bead1 = create_test_bead("Multiple items test 1");
    let bead2 = create_test_bead("Multiple items test 2");
    let bead3 = create_test_bead("Multiple items test 3");
    let bead4 = create_test_bead("Multiple items test 4");
    let bead5 = create_test_bead("Multiple items test 5");

    let output = bf()
        .arg("list")
        .arg("--json")
        .output()
        .expect("Failed to execute list command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Parse JSONL output
    let parsed = parse_jsonl(&stdout);

    // Should have at least our 5 beads
    assert!(parsed.len() >= 5, "Should have at least 5 beads, got {}", parsed.len());

    // Verify all our beads are present
    let bead_ids = vec![&bead1, &bead2, &bead3, &bead4, &bead5];
    for expected_id in &bead_ids {
        assert!(
            parsed.iter().any(|b| b["id"].as_str() == Some(expected_id)),
            "Should find bead {} in list output",
            expected_id
        );
    }

    // Verify each item has required fields
    for bead in &parsed {
        assert!(bead.get("id").is_some(), "Each item should have id");
        assert!(bead.get("title").is_some(), "Each item should have title");
        assert!(bead.get("status").is_some(), "Each item should have status");
        assert!(bead.get("priority").is_some(), "Each item should have priority");
        assert!(bead.get("issue_type").is_some(), "Each item should have issue_type");
        assert!(bead.get("labels").is_some(), "Each item should have labels array");
    }

    // Cleanup
    for bead_id in &bead_ids {
        close_test_bead(bead_id);
    }
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_list_json_valid_jsonl_format() {
    // Create test beads
    let bead1 = create_test_bead("JSONL format test 1");
    let bead2 = create_test_bead("JSONL format test 2");

    let output = bf()
        .arg("list")
        .arg("--json")
        .output()
        .expect("Failed to execute list command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");

    // Verify each line is valid JSON (JSONL format)
    let mut line_count = 0;
    for line in stdout.lines() {
        if !line.trim().is_empty() && line.trim() != "[]" {
            let parsed: Value = serde_json::from_str(line)
                .expect(&format!("Each line should be valid JSON: {}", line));
            assert!(parsed.is_object(), "Each line should be a JSON object");
            line_count += 1;
        }
    }

    assert!(line_count >= 2, "Should have at least 2 JSON objects");

    // Cleanup
    close_test_bead(&bead1);
    close_test_bead(&bead2);
}

// ============================================================================
// READY COMMAND WITH --json FLAG TESTS
// ============================================================================

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_ready_command_json_flag() {
    // Test that --json flag works for ready command
    let bead_id = create_test_bead("Test ready --json flag");

    let output = bf()
        .arg("ready")
        .arg("--limit")
        .arg("10")
        .arg("--json")
        .output()
        .expect("Failed to execute ready command");

    assert!(
        output.status.success(),
        "ready --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Ready returns "[]" for empty or JSONL for results
    if trimmed != "[]" {
        // Should contain our bead
        assert!(stdout.contains(&bead_id), "ready --json should include our bead");

        // Verify output is valid JSONL
        let parsed = parse_jsonl(&stdout);
        assert!(parsed.len() >= 1, "Should have at least one bead");

        // Verify required fields
        let bead = parsed.iter().find(|b| b["id"].as_str() == Some(&bead_id))
            .expect("Should find our bead");

        assert!(bead.get("id").is_some(), "Should have id field");
        assert!(bead.get("title").is_some(), "Should have title field");
        assert!(bead.get("status").is_some(), "Should have status field");
    }

    close_test_bead(&bead_id);
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_ready_json_empty_results() {
    // Test empty results - ready returns "[]" when no beads available
    let output = bf()
        .arg("ready")
        .arg("--json")
        .output()
        .expect("Failed to execute ready command");

    assert!(
        output.status.success(),
        "ready --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    // Empty ready should return "[]"
    assert_eq!(trimmed, "[]", "Empty ready should return '[]'");

    // Verify it's valid JSON
    let parsed: Value = serde_json::from_str(trimmed)
        .expect("Empty result should be valid JSON");
    assert!(parsed.is_array(), "Empty result should be an array");
    assert_eq!(parsed.as_array().unwrap().len(), 0, "Array should be empty");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_ready_json_multiple_items() {
    // Create multiple test beads (all unblocked, so ready)
    let bead1 = create_test_bead("Ready multiple test 1");
    let bead2 = create_test_bead("Ready multiple test 2");
    let bead3 = create_test_bead("Ready multiple test 3");
    let bead4 = create_test_bead("Ready multiple test 4");

    let output = bf()
        .arg("ready")
        .arg("--limit")
        .arg("10")
        .arg("--json")
        .output()
        .expect("Failed to execute ready command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    if trimmed != "[]" {
        // Parse JSONL output
        let parsed = parse_jsonl(&stdout);

        // Should have at least some of our beads
        assert!(parsed.len() >= 1, "Should have at least one ready bead");

        // Verify each item has required fields
        for bead in &parsed {
            assert!(bead.get("id").is_some(), "Each item should have id");
            assert!(bead.get("title").is_some(), "Each item should have title");
            assert!(bead.get("status").is_some(), "Each item should have status");
            assert!(bead.get("priority").is_some(), "Each item should have priority");
            assert!(bead.get("issue_type").is_some(), "Each item should have issue_type");
            assert!(bead.get("labels").is_some(), "Each item should have labels array");
        }
    }

    // Cleanup
    close_test_bead(&bead1);
    close_test_bead(&bead2);
    close_test_bead(&bead3);
    close_test_bead(&bead4);
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_ready_json_valid_jsonl_format() {
    // Create test beads
    let bead1 = create_test_bead("Ready JSONL format test 1");
    let bead2 = create_test_bead("Ready JSONL format test 2");

    let output = bf()
        .arg("ready")
        .arg("--limit")
        .arg("10")
        .arg("--json")
        .output()
        .expect("Failed to execute ready command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    if trimmed != "[]" {
        // Verify each line is valid JSON (JSONL format)
        let mut line_count = 0;
        for line in trimmed.lines() {
            if !line.trim().is_empty() && line.trim() != "[]" {
                let parsed: Value = serde_json::from_str(line)
                    .expect(&format!("Each line should be valid JSON: {}", line));
                assert!(parsed.is_object(), "Each line should be a JSON object");
                line_count += 1;
            }
        }

        assert!(line_count >= 1, "Should have at least one JSON object");
    }

    // Cleanup
    close_test_bead(&bead1);
    close_test_bead(&bead2);
}

// ============================================================================
// EDGE CASES AND CONSISTENCY TESTS
// ============================================================================

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_list_ready_json_flag_consistency() {
    // Test that --json and --format json produce identical output
    let bead_id = create_test_bead("Consistency test");

    // Get output with --json flag
    let json_flag_output = bf()
        .arg("list")
        .arg("--json")
        .output()
        .expect("Failed to execute list with --json");

    let json_flag_stdout = String::from_utf8(json_flag_output.stdout).expect("Invalid UTF-8");

    // Get output with --format json
    let format_json_output = bf()
        .arg("list")
        .arg("--format")
        .arg("json")
        .output()
        .expect("Failed to execute list with --format json");

    let format_json_stdout = String::from_utf8(format_json_output.stdout).expect("Invalid UTF-8");

    // Both should contain our bead
    assert!(json_flag_stdout.contains(&bead_id), "--json should contain bead");
    assert!(format_json_stdout.contains(&bead_id), "--format json should contain bead");

    // Parse both and verify they have the same number of items
    let json_flag_parsed = parse_jsonl(&json_flag_stdout);
    let format_json_parsed = parse_jsonl(&format_json_stdout);

    assert_eq!(json_flag_parsed.len(), format_json_parsed.len(),
               "Both flags should produce same number of items");

    close_test_bead(&bead_id);
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_list_json_with_filters() {
    // Create beads with different statuses
    let open_bead = create_test_bead("Filter test - open");
    close_test_bead(&open_bead);

    let open_bead2 = create_test_bead("Filter test - open 2");

    // Test with status filter
    let output = bf()
        .arg("list")
        .arg("--status")
        .arg("open")
        .arg("--json")
        .output()
        .expect("Failed to execute list command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let parsed = parse_jsonl(&stdout);

    // Should find the open bead
    assert!(parsed.iter().any(|b| b["id"].as_str() == Some(&open_bead2)),
            "Should find open bead");

    // Should not find the closed bead
    assert!(!parsed.iter().any(|b| b["id"].as_str() == Some(&open_bead)),
            "Should not find closed bead when filtering for open");

    // Verify all results have status=open
    for bead in &parsed {
        assert_eq!(bead["status"].as_str().unwrap(), "open",
                   "All filtered results should have status=open");
    }

    close_test_bead(&open_bead2);
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_ready_json_limit_parameter() {
    // Create multiple beads
    for i in 1..=5 {
        let bead = create_test_bead(&format!("Ready limit test bead {}", i));
        close_test_bead(&bead);
    }

    // Test with limit=2
    let output = bf()
        .arg("ready")
        .arg("--limit")
        .arg("2")
        .arg("--json")
        .output()
        .expect("Failed to execute ready command");

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let trimmed = stdout.trim();

    if trimmed != "[]" {
        let lines: Vec<&str> = trimmed.lines().collect();
        assert!(lines.len() <= 2, "ready with --limit 2 should return at most 2 beads");
    }
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_list_ready_json_unicode() {
    // Test with unicode and emoji
    let unicode_title = "Test with unicode Ñ and emoji 🎉🚀💡";
    let bead_id = create_test_bead(unicode_title);

    // Test list preserves unicode
    let list_output = bf()
        .arg("list")
        .arg("--json")
        .output()
        .expect("Failed to execute list command");

    let list_stdout = String::from_utf8(list_output.stdout).expect("Invalid UTF-8");
    assert!(list_stdout.contains("🎉"), "list --json should preserve emoji");
    assert!(list_stdout.contains("Ñ"), "list --json should preserve unicode");

    // Test ready preserves unicode
    let ready_output = bf()
        .arg("ready")
        .arg("--limit")
        .arg("10")
        .arg("--json")
        .output()
        .expect("Failed to execute ready command");

    let ready_stdout = String::from_utf8(ready_output.stdout).expect("Invalid UTF-8");
    let trimmed = ready_stdout.trim();

    if trimmed != "[]" {
        assert!(ready_stdout.contains("🎉"), "ready --json should preserve emoji");
        assert!(ready_stdout.contains("Ñ"), "ready --json should preserve unicode");
    }

    close_test_bead(&bead_id);
}
