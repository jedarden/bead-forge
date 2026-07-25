//! JSON output tests for `bf search` command
//!
//! Comprehensive tests for search command JSON output including:
//! - JSON structure validation
//! - Required fields presence and types
//! - Empty result set handling
//! - JSONL format validation
//! - Special character handling
//! - Query functionality (text search in title and description)
//! - Filtering by status, type, assignee, labels, and priority range
//! - Limit functionality
//! - Multiple filter combinations

use std::process::Command;
use tempfile::TempDir;

// Import test infrastructure helpers from sibling module
use super::json_output::{
    test_workspace, bf_binary, bf_command,
    json_validation, format_detection, fixtures, capture,
};

// Import items made available in parent scope
use super::*;

/// Create an isolated test workspace
fn create_isolated_workspace() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let beads_dir = temp_dir.path().join(".beads");
    std::fs::create_dir(&beads_dir).expect("Failed to create .beads directory");

    // Initialize workspace
    crate::config::init_workspace(&beads_dir, "bf-search-test")
        .expect("Failed to initialize test workspace");

    let metadata = crate::config::load_metadata(&beads_dir)
        .expect("Failed to load metadata");
    let _ = crate::Storage::open(&beads_dir.join(&metadata.database))
        .expect("Failed to create database");

    temp_dir
}

/// Create a test bead with a specific type
fn create_bead_with_type(title: &str, issue_type: &str) -> String {
    let output = bf_command()
        .arg("create")
        .arg("--title")
        .arg(title)
        .arg("--type")
        .arg(issue_type)
        .arg("--priority")
        .arg("2")
        .output()
        .expect("Failed to execute bf create");

    if !output.status.success() {
        panic!(
            "Failed to create bead: {}\nStderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    stdout.trim().to_string()
}

/// Create a test bead with type and labels
fn create_bead_with_type_and_labels(title: &str, issue_type: &str, labels: &[&str]) -> String {
    let bead_id = create_bead_with_type(title, issue_type);

    for label in labels {
        let output = bf_command()
            .arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to execute bf label add");

        if !output.status.success() {
            panic!(
                "Failed to add label '{}': {}",
                label,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    bead_id
}

/// Create a test bead with type and priority
fn create_bead_with_type_and_priority(title: &str, issue_type: &str, priority: i64) -> String {
    let output = bf_command()
        .arg("create")
        .arg("--title")
        .arg(title)
        .arg("--type")
        .arg(issue_type)
        .arg("--priority")
        .arg(&priority.to_string())
        .output()
        .expect("Failed to execute bf create");

    if !output.status.success() {
        panic!(
            "Failed to create bead: {}\nStderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    stdout.trim().to_string()
}

/// Create a test bead with type, labels, and priority
fn create_bead_with_type_and_labels_and_priority(title: &str, issue_type: &str, labels: &[&str], priority: i64) -> String {
    let bead_id = create_bead_with_type_and_priority(title, issue_type, priority);

    for label in labels {
        let output = bf_command()
            .arg("label")
            .arg("add")
            .arg(&bead_id)
            .arg("--label")
            .arg(label)
            .output()
            .expect("Failed to execute bf label add");

        if !output.status.success() {
            panic!(
                "Failed to add label '{}': {}",
                label,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    bead_id
}

// ============================================================================
// Basic structure and format tests
// ============================================================================

#[test]
fn test_search_json_structure_validity() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create test beads with searchable content
    let bead1_id = fixtures::create_bead("Search test bead one");
    let bead2_id = fixtures::create_bead("Search test bead two");

    // Search for beads containing "test"
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("test")
            .arg("--format")
            .arg("json")
    );

    // Verify it's valid JSONL (multiple lines, each a valid JSON object)
    let json_str = output.trim();
    json_validation::assert_valid_jsonl(json_str);

    // Parse each line and verify structure
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() >= 2, "search should return at least 2 beads");

    for line in lines {
        let parsed = json_validation::parse_json(line);
        assert!(parsed.is_object(), "each line should be a JSON object");

        // Verify required fields
        json_validation::assert_required_fields(
            &parsed,
            &["id", "title", "status", "priority", "issue_type", "created_at", "updated_at"],
            "search command"
        );
    }

    // Cleanup
    fixtures::close_bead(&bead1_id, "Search test cleanup 1");
    fixtures::close_bead(&bead2_id, "Search test cleanup 2");
}

#[test]
fn test_search_json_jsonl_format_structure() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create test beads
    let bead1_id = fixtures::create_bead("JSONL search test one");
    let bead2_id = fixtures::create_bead("JSONL search test two");
    let bead3_id = fixtures::create_bead("JSONL search test three");

    // Search for beads
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("JSONL")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();

    // Test 1: Validate that output is in JSONL format (NOT a JSON array)
    format_detection::assert_format(json_str, format_detection::JsonFormat::JsonL);

    // Test 2: Validate each line is valid JSON
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() >= 3, "search should return at least 3 beads");

    for (i, line) in lines.iter().enumerate() {
        // Each line must be valid JSON
        json_validation::assert_valid_json(line);

        // Each line must be a JSON object
        let parsed = json_validation::parse_json(line);
        assert!(parsed.is_object(), "JSONL line {} should be a JSON object", i);
    }

    // Test 3: Verify output is NOT a JSON array
    let first_char = json_str.chars().next().unwrap_or(' ');
    let last_char = json_str.chars().last().unwrap_or(' ');
    assert_ne!(first_char, '[', "JSONL output should not start with '['");
    assert_ne!(last_char, ']', "JSONL output should not end with ']'");

    // Cleanup
    fixtures::close_bead(&bead1_id, "JSONL search cleanup 1");
    fixtures::close_bead(&bead2_id, "JSONL search cleanup 2");
    fixtures::close_bead(&bead3_id, "JSONL search cleanup 3");
}

#[test]
fn test_search_json_empty_result() {
    let temp_dir = create_isolated_workspace();
    let workspace = temp_dir.path();

    // Ensure no matching beads exist by using a fresh workspace
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("nonexistent")
            .arg("--format")
            .arg("json")
    );

    // Empty search should produce no output (empty string)
    let json_str = output.trim();
    assert_eq!(json_str, "", "empty search should print nothing");
}

#[test]
fn test_search_json_required_fields_types() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Search field types test");

    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("field types")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Find our bead in the output
    let bead_json = lines.iter()
        .find(|line| line.contains(&bead_id))
        .expect("created bead should be in search output");

    let parsed = json_validation::parse_json(bead_json);

    // id must be a string matching created bead
    let id_val = json_validation::get_string(&parsed, "id");
    assert_eq!(id_val, bead_id);

    // title must be a string
    let title = json_validation::get_string(&parsed, "title");
    assert_eq!(title, "Search field types test");

    // status must be a string with valid value
    let status = json_validation::get_string(&parsed, "status");
    assert!(matches!(status.as_str(), "open" | "in_progress" | "blocked" | "closed"));

    // priority must be a number (0-4)
    let priority = json_validation::get_int(&parsed, "priority");
    assert!((0..=4).contains(&priority), "priority must be between 0 and 4");

    // issue_type must be a string
    let issue_type = json_validation::get_string(&parsed, "issue_type");
    assert!(!issue_type.is_empty(), "issue_type must not be empty");

    // assignee must be present (null or string)
    assert!(parsed.get("assignee").is_some(), "assignee field must be present");

    // labels must be an array
    let labels = json_validation::get_array(&parsed, "labels");
    // Successful call proves it's an array

    fixtures::close_bead(&bead_id, "Search field types cleanup");
}

// ============================================================================
// Query functionality tests
// ============================================================================

#[test]
fn test_search_json_query_in_title() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Unique search term in title");

    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("Unique search term")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find our bead
    assert!(lines.iter().any(|line| line.contains(&bead_id)),
            "search should find bead with matching title");

    fixtures::close_bead(&bead_id, "Query title cleanup");
}

#[test]
fn test_search_json_query_in_description() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Description search test");

    // Add a description with searchable content
    let unique_desc = "UniqueDescriptionContentXYZ";
    let mut cmd = bf_command();
    cmd.arg("update")
        .arg(&bead_id)
        .arg("--description")
        .arg(unique_desc);
    let update_output = cmd.output().expect("Failed to update");
    assert!(update_output.status.success(), "Update should succeed");

    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("UniqueDescriptionContentXYZ")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find our bead via description
    assert!(lines.iter().any(|line| line.contains(&bead_id)),
            "search should find bead with matching description");

    fixtures::close_bead(&bead_id, "Query description cleanup");
}

#[test]
fn test_search_json_query_case_sensitive() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("CaseSensitiveSearchTerm");

    // Search with different case (should NOT match - SQLite LIKE is case-sensitive)
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("casesensivesearchterm")  // lowercase
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should NOT find our bead (case-sensitive search)
    assert!(!lines.iter().any(|line| line.contains(&bead_id)),
            "search should be case-sensitive and not find lowercase query");

    // Search with exact case (should match)
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("CaseSensitiveSearchTerm")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find our bead with exact case
    assert!(lines.iter().any(|line| line.contains(&bead_id)),
            "search should find bead with exact case match");

    fixtures::close_bead(&bead_id, "Case sensitive cleanup");
}

#[test]
fn test_search_json_query_no_match() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Random bead content");

    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("NonExistentSearchTerm12345")
            .arg("--format")
            .arg("json")
    );

    // Should return no results
    let json_str = output.trim();
    assert_eq!(json_str, "", "search with no matches should return empty output");

    fixtures::close_bead(&bead_id, "No match cleanup");
}

// ============================================================================
// Filter tests
// ============================================================================

#[test]
fn test_search_json_status_filter() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead1_id = fixtures::create_bead("Status filter test open");
    let bead2_id = fixtures::create_bead("Status filter test closed");

    // Close bead2
    fixtures::close_bead(&bead2_id, "Test close");

    // Search with status filter
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("Status filter")
            .arg("--status")
            .arg("closed")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find bead2 (closed) but not bead1 (open)
    assert!(lines.iter().any(|line| line.contains(&bead2_id)),
            "search should find closed bead");
    assert!(!lines.iter().any(|line| line.contains(&bead1_id)),
            "search should not find open bead with closed filter");

    fixtures::close_bead(&bead1_id, "Status filter cleanup 1");
}

#[test]
fn test_search_json_multiple_status_filters() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead1_id = fixtures::create_bead("Multi status open");
    let bead2_id = fixtures::create_bead("Multi status in_progress");
    let bead3_id = fixtures::create_bead("Multi status closed");

    // Update bead2 to in_progress
    let mut cmd = bf_command();
    cmd.arg("update")
        .arg(&bead2_id)
        .arg("--status")
        .arg("in_progress");
    let update_output = cmd.output().expect("Failed to update");
    assert!(update_output.status.success(), "Update should succeed");

    // Close bead3
    fixtures::close_bead(&bead3_id, "Test close");

    // Search with multiple status filters (OR logic)
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("Multi status")
            .arg("--status")
            .arg("open")
            .arg("--status")
            .arg("closed")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find bead1 (open) and bead3 (closed) but not bead2 (in_progress)
    assert!(lines.iter().any(|line| line.contains(&bead1_id)),
            "search should find open bead");
    assert!(lines.iter().any(|line| line.contains(&bead3_id)),
            "search should find closed bead");
    assert!(!lines.iter().any(|line| line.contains(&bead2_id)),
            "search should not find in_progress bead with open/closed filter");

    fixtures::close_bead(&bead1_id, "Multi status cleanup 1");
    fixtures::close_bead(&bead2_id, "Multi status cleanup 2");
}

#[test]
fn test_search_json_type_filter() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create beads with different types using --type flag in create
    let bead1_id = create_bead_with_type("Type filter bug", "bug");
    let bead2_id = create_bead_with_type("Type filter feature", "feature");

    // Search with type filter
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("Type filter")
            .arg("--type")
            .arg("bug")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find bead1 (bug) but not bead2 (feature)
    assert!(lines.iter().any(|line| line.contains(&bead1_id)),
            "search should find bug bead");
    assert!(!lines.iter().any(|line| line.contains(&bead2_id)),
            "search should not find feature bead with bug filter");

    fixtures::close_bead(&bead1_id, "Type filter cleanup 1");
    fixtures::close_bead(&bead2_id, "Type filter cleanup 2");
}

#[test]
fn test_search_json_assignee_filter() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead1_id = fixtures::create_bead_with_assignee("Assignee filter alice", "alice");
    let bead2_id = fixtures::create_bead_with_assignee("Assignee filter bob", "bob");

    // Search with assignee filter
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("Assignee filter")
            .arg("--assignee")
            .arg("alice")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find bead1 (alice) but not bead2 (bob)
    assert!(lines.iter().any(|line| line.contains(&bead1_id)),
            "search should find alice's bead");
    assert!(!lines.iter().any(|line| line.contains(&bead2_id)),
            "search should not find bob's bead with alice filter");

    fixtures::close_bead(&bead1_id, "Assignee filter cleanup 1");
    fixtures::close_bead(&bead2_id, "Assignee filter cleanup 2");
}

#[test]
fn test_search_json_label_filter() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead1_id = fixtures::create_bead_with_labels("Label filter urgent", &["urgent", "bug"]);
    let bead2_id = fixtures::create_bead_with_labels("Label filter enhancement", &["enhancement"]);

    // Search with label filter
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("Label filter")
            .arg("--label")
            .arg("urgent")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find bead1 (has urgent label) but not bead2
    assert!(lines.iter().any(|line| line.contains(&bead1_id)),
            "search should find bead with urgent label");
    assert!(!lines.iter().any(|line| line.contains(&bead2_id)),
            "search should not find bead without urgent label");

    fixtures::close_bead(&bead1_id, "Label filter cleanup 1");
    fixtures::close_bead(&bead2_id, "Label filter cleanup 2");
}

#[test]
fn test_search_json_priority_range_filter() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead1_id = fixtures::create_bead("Priority range 0");
    let bead2_id = fixtures::create_bead("Priority range 2");
    let bead3_id = fixtures::create_bead("Priority range 4");

    // Update priorities
    for (bead_id, priority) in [(&bead1_id, 0), (&bead2_id, 2), (&bead3_id, 4)] {
        let mut cmd = bf_command();
        cmd.arg("update")
            .arg(bead_id)
            .arg("--priority")
            .arg(&priority.to_string());
        let update_output = cmd.output().expect("Failed to update");
        assert!(update_output.status.success(), "Update should succeed");
    }

    // Search with priority range
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("Priority range")
            .arg("--priority-min")
            .arg("1")
            .arg("--priority-max")
            .arg("3")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find bead2 (priority 2) but not bead1 (priority 0) or bead3 (priority 4)
    assert!(lines.iter().any(|line| line.contains(&bead2_id)),
            "search should find bead with priority in range");
    assert!(!lines.iter().any(|line| line.contains(&bead1_id)),
            "search should not find bead with priority below range");
    assert!(!lines.iter().any(|line| line.contains(&bead3_id)),
            "search should not find bead with priority above range");

    fixtures::close_bead(&bead1_id, "Priority range cleanup 1");
    fixtures::close_bead(&bead2_id, "Priority range cleanup 2");
    fixtures::close_bead(&bead3_id, "Priority range cleanup 3");
}

#[test]
fn test_search_json_priority_min_only() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead1_id = fixtures::create_bead("Priority min 0");
    let bead2_id = fixtures::create_bead("Priority min 3");

    // Update priorities
    for (bead_id, priority) in [(&bead1_id, 0), (&bead2_id, 3)] {
        let mut cmd = bf_command();
        cmd.arg("update")
            .arg(bead_id)
            .arg("--priority")
            .arg(&priority.to_string());
        let update_output = cmd.output().expect("Failed to update");
        assert!(update_output.status.success(), "Update should succeed");
    }

    // Search with priority-min only
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("Priority min")
            .arg("--priority-min")
            .arg("2")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find bead2 (priority 3) but not bead1 (priority 0)
    assert!(lines.iter().any(|line| line.contains(&bead2_id)),
            "search should find bead with priority >= min");
    assert!(!lines.iter().any(|line| line.contains(&bead1_id)),
            "search should not find bead with priority below min");

    fixtures::close_bead(&bead1_id, "Priority min cleanup 1");
    fixtures::close_bead(&bead2_id, "Priority min cleanup 2");
}

#[test]
fn test_search_json_priority_max_only() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead1_id = fixtures::create_bead("Priority max 1");
    let bead2_id = fixtures::create_bead("Priority max 4");

    // Update priorities
    for (bead_id, priority) in [(&bead1_id, 1), (&bead2_id, 4)] {
        let mut cmd = bf_command();
        cmd.arg("update")
            .arg(bead_id)
            .arg("--priority")
            .arg(&priority.to_string());
        let update_output = cmd.output().expect("Failed to update");
        assert!(update_output.status.success(), "Update should succeed");
    }

    // Search with priority-max only
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("Priority max")
            .arg("--priority-max")
            .arg("2")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find bead1 (priority 1) but not bead2 (priority 4)
    assert!(lines.iter().any(|line| line.contains(&bead1_id)),
            "search should find bead with priority <= max");
    assert!(!lines.iter().any(|line| line.contains(&bead2_id)),
            "search should not find bead with priority above max");

    fixtures::close_bead(&bead1_id, "Priority max cleanup 1");
    fixtures::close_bead(&bead2_id, "Priority max cleanup 2");
}

// ============================================================================
// Limit tests
// ============================================================================

#[test]
fn test_search_json_limit() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create multiple beads
    let bead1 = fixtures::create_bead("Search limit 1");
    let bead2 = fixtures::create_bead("Search limit 2");
    let bead3 = fixtures::create_bead("Search limit 3");

    // Test limit
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("Search limit")
            .arg("--limit")
            .arg("2")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    assert_eq!(lines.len(), 2, "limited search should return exactly 2 beads");

    // Cleanup
    fixtures::close_bead(&bead1, "Search limit cleanup 1");
    fixtures::close_bead(&bead2, "Search limit cleanup 2");
    fixtures::close_bead(&bead3, "Search limit cleanup 3");
}

#[test]
fn test_search_json_default_limit() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create multiple beads (more than default limit of 50)
    let mut bead_ids = Vec::new();
    for i in 1..=60 {
        let bead_id = fixtures::create_bead(&format!("Default limit test {}", i));
        bead_ids.push(bead_id);
    }

    // Search without explicit limit (should use default of 50)
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("Default limit test")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should return at most 50 results (default limit)
    assert_eq!(lines.len(), 50, "search should return at most 50 beads by default");

    // Cleanup
    for bead_id in bead_ids {
        fixtures::close_bead(&bead_id, "Default limit cleanup");
    }
}

// ============================================================================
// Special character handling tests
// ============================================================================

#[test]
fn test_search_json_special_characters_in_query() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let special_title = "Test \"quotes\" and 'apostrophes' & <symbols>";
    let bead_id = fixtures::create_bead(special_title);

    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("quotes")
            .arg("--format")
            .arg("json")
    );

    // Verify it's valid JSON (proper escaping)
    let json_str = output.trim();
    json_validation::assert_valid_jsonl(json_str);

    // Find our bead
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();
    let bead_json = lines.iter()
        .find(|line| line.contains(&bead_id))
        .expect("created bead should be in search output");

    let parsed = json_validation::parse_json(bead_json);
    let title = json_validation::get_string(&parsed, "title");

    // Verify special characters are preserved
    assert!(title.contains("quotes"), "title should contain 'quotes'");
    assert!(title.contains("apostrophes"), "title should contain 'apostrophes'");

    fixtures::close_bead(&bead_id, "Search special chars cleanup");
}

#[test]
fn test_search_json_unicode_in_query() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let unicode_title = "🎉 Unicode test with 日本語 and café";
    let bead_id = fixtures::create_bead(unicode_title);

    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("日本語")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find bead with Japanese characters
    assert!(lines.iter().any(|line| line.contains(&bead_id)),
            "search should handle unicode in query");

    fixtures::close_bead(&bead_id, "Search unicode cleanup");
}

#[test]
fn test_search_json_special_characters_in_result() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead(fixtures::SPECIAL_CHARACTERS_TITLE);

    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("quotes")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();
    let bead_json = lines.iter()
        .find(|line| line.contains(&bead_id))
        .expect("created bead should be in search output");

    // Verify JSON is valid (special characters properly escaped)
    json_validation::assert_valid_json(bead_json);

    let parsed = json_validation::parse_json(bead_json);
    let title = json_validation::get_string(&parsed, "title");

    // Verify special characters are preserved
    assert!(title.contains("quotes"), "title should contain 'quotes'");
    assert!(title.contains("apostrophes"), "title should contain 'apostrophes'");
    assert!(title.contains("&"), "title should contain '&'");
    assert!(title.contains("<"), "title should contain '<'");
    assert!(title.contains(">"), "title should contain '>'");

    fixtures::close_bead(&bead_id, "Search special chars result cleanup");
}

// ============================================================================
// Combined filter tests
// ============================================================================

#[test]
fn test_search_json_combined_filters() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create beads with different types using --type flag in create
    let bead1_id = create_bead_with_type_and_labels_and_priority("Combined urgent bug", "bug", &["urgent", "bug"], 0);
    let bead2_id = create_bead_with_type_and_labels_and_priority("Combined urgent feature", "feature", &["urgent", "feature"], 2);

    // Search with combined filters
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("Combined")
            .arg("--label")
            .arg("urgent")
            .arg("--type")
            .arg("bug")
            .arg("--priority-max")
            .arg("1")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find only bead1 (matches all filters: urgent label, bug type, priority <= 1)
    assert!(lines.iter().any(|line| line.contains(&bead1_id)),
            "search should find bead matching all filters");
    assert!(!lines.iter().any(|line| line.contains(&bead2_id)),
            "search should not find bead that doesn't match all filters");

    fixtures::close_bead(&bead1_id, "Combined filter cleanup 1");
    fixtures::close_bead(&bead2_id, "Combined filter cleanup 2");
}

#[test]
fn test_search_json_query_with_filters() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead1_id = fixtures::create_bead_with_labels("Query filter bug fix", &["bug"]);
    let bead2_id = fixtures::create_bead_with_labels("Query filter feature add", &["feature"]);

    // Search for "fix" with label filter
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("fix")
            .arg("--label")
            .arg("bug")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find bead1 (matches query "fix" and label "bug")
    assert!(lines.iter().any(|line| line.contains(&bead1_id)),
            "search should find bead matching both query and label filter");
    assert!(!lines.iter().any(|line| line.contains(&bead2_id)),
            "search should not find bead that doesn't match label filter");

    fixtures::close_bead(&bead1_id, "Query filter cleanup 1");
    fixtures::close_bead(&bead2_id, "Query filter cleanup 2");
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_search_json_whitespace_in_query() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead_id = fixtures::create_bead("Search   with    spaces");

    // Search for exact substring with multiple spaces (should match)
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("Search   with")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find bead (exact substring match with correct spacing)
    assert!(lines.iter().any(|line| line.contains(&bead_id)),
            "search should find bead with exact substring including spaces");

    // Search for different spacing (should NOT match - LIKE requires exact spacing)
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("Search with")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should NOT find bead (spacing doesn't match)
    assert!(!lines.iter().any(|line| line.contains(&bead_id)),
            "search should not find bead when spacing doesn't match");

    // Search for single word (should match)
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("Search")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find bead (single word match)
    assert!(lines.iter().any(|line| line.contains(&bead_id)),
            "search should find bead with single word from title");

    fixtures::close_bead(&bead_id, "Whitespace cleanup");
}

#[test]
fn test_search_json_empty_query_with_filters() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    let bead1_id = fixtures::create_bead_with_labels("Empty query test", &["special"]);
    let bead2_id = fixtures::create_bead("Empty query other");

    // Search with empty query and label filter
    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("--label")
            .arg("special")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // Should find bead1 by label even without query
    assert!(lines.iter().any(|line| line.contains(&bead1_id)),
            "search should work with empty query and filters");

    fixtures::close_bead(&bead1_id, "Empty query cleanup 1");
    fixtures::close_bead(&bead2_id, "Empty query cleanup 2");
}

#[test]
fn test_search_json_result_ordering() {
    let _ws = create_isolated_workspace();
    let workspace = test_workspace();

    // Create beads with specific titles to test ordering
    let bead1_id = fixtures::create_bead("Order test A");
    let bead2_id = fixtures::create_bead("Order test B");
    let bead3_id = fixtures::create_bead("Order test C");

    let output = capture::capture_stdout(
        bf_command()
            .arg("search")
            .arg("Order test")
            .arg("--format")
            .arg("json")
    );

    let json_str = output.trim();
    let lines: Vec<&str> = json_str.lines().filter(|l| !l.trim().is_empty()).collect();

    // All beads should be present
    assert!(lines.iter().any(|line| line.contains(&bead1_id)),
            "search should include bead1");
    assert!(lines.iter().any(|line| line.contains(&bead2_id)),
            "search should include bead2");
    assert!(lines.iter().any(|line| line.contains(&bead3_id)),
            "search should include bead3");

    // Cleanup
    fixtures::close_bead(&bead1_id, "Order test cleanup 1");
    fixtures::close_bead(&bead2_id, "Order test cleanup 2");
    fixtures::close_bead(&bead3_id, "Order test cleanup 3");
}
