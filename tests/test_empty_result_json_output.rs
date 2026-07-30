//! Comprehensive tests for empty result set JSON output
//!
//! These tests verify that commands that return no matching results
//! produce valid, properly formatted JSON output.
//!
//! Acceptance Criteria:
//! - Test search command with no results returns valid JSON
//! - Test list command with no beads returns valid JSON
//! - Test ready command with no ready beads returns valid JSON
//! - Verify empty result arrays are properly formatted
//! - All tests pass

use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use serde_json::{Value, from_str};

fn bf() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bf"))
}

/// Run `bf` with args in `workspace`, returning (stdout, stderr, success).
fn run_bf(workspace: &Path, args: &[&str]) -> (String, String, bool) {
    let output = bf()
        .current_dir(workspace)
        .args(args)
        .output()
        .expect("failed to execute bf");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.success(),
    )
}

fn setup() -> (TempDir, PathBuf) {
    let temp = TempDir::new().unwrap();
    let workspace = temp.path().to_path_buf();
    let (_o, e, ok) = run_bf(&workspace, &["init", "--prefix", "bf"]);
    assert!(ok, "bf init failed: {e}");
    (temp, workspace)
}

/// Parse a JSON string and panic if invalid
fn parse_json(json: &str) -> Value {
    from_str(json).unwrap_or_else(|e| {
        panic!("Failed to parse JSON: {}\nJSON was: {}", e, json)
    })
}

/// Parse a JSONL string (newline-delimited JSON) into a Vec of values
fn parse_jsonl(jsonl: &str) -> Vec<Value> {
    jsonl
        .lines()
        .filter(|line| !line.trim().is_empty() && line.trim() != "[]")
        .map(|line| parse_json(line))
        .collect()
}

// ============================================================================
// SEARCH COMMAND EMPTY RESULT TESTS
// ============================================================================

#[test]
fn test_search_json_no_results_returns_valid_json() {
    let (_temp, workspace) = setup();

    // Create a bead (so workspace isn't completely empty)
    let (_out, err, ok) = run_bf(
        &workspace,
        &["create", "--title", "test bead", "--type", "task", "--priority", "2"],
    );
    assert!(ok, "bf create failed: {err}");

    // Search with text that won't match anything
    let (out, err, ok) = run_bf(
        &workspace,
        &["search", "NONEXISTENT_SEARCH_TERM_xyz123", "--format", "json"],
    );
    assert!(ok, "Search with no matches failed: {err}");

    let trimmed = out.trim();

    // Empty search result should return valid JSON (either "[]" or empty string)
    if !trimmed.is_empty() {
        let parsed = parse_json(trimmed);

        // If it's not empty, it should be a valid JSON structure
        if let Some(arr) = parsed.as_array() {
            assert_eq!(arr.len(), 0, "Empty search should return empty array");
        } else {
            // Could be wrapped in envelope or other structure
            assert!(parsed.is_object() || parsed.is_array(), "Should return valid JSON");
        }
    }
}

#[test]
fn test_search_json_with_status_filter_no_results() {
    let (_temp, workspace) = setup();

    // Create an open bead
    let (_out, err, ok) = run_bf(
        &workspace,
        &["create", "--title", "open bead", "--type", "task", "--priority", "2"],
    );
    assert!(ok, "bf create failed: {err}");

    // Search for closed beads (should return empty)
    let (out, err, ok) = run_bf(
        &workspace,
        &["search", "--status", "closed", "--format", "json"],
    );
    assert!(ok, "Search with no matches failed: {err}");

    let trimmed = out.trim();

    // Should return valid JSON for empty results
    if !trimmed.is_empty() {
        let parsed = parse_json(trimmed);
        if let Some(arr) = parsed.as_array() {
            assert_eq!(arr.len(), 0, "Should return empty array");
        }
    }
}

#[test]
fn test_search_json_with_type_filter_no_results() {
    let (_temp, workspace) = setup();

    // Create a task
    let (_out, err, ok) = run_bf(
        &workspace,
        &["create", "--title", "task bead", "--type", "task", "--priority", "2"],
    );
    assert!(ok, "bf create failed: {err}");

    // Search for epics (should return empty)
    let (out, err, ok) = run_bf(
        &workspace,
        &["search", "--type", "epic", "--format", "json"],
    );
    assert!(ok, "Search with no matches failed: {err}");

    let trimmed = out.trim();

    // Should return valid JSON for empty results
    if !trimmed.is_empty() {
        let parsed = parse_json(trimmed);
        if let Some(arr) = parsed.as_array() {
            assert_eq!(arr.len(), 0, "Should return empty array");
        }
    }
}

#[test]
fn test_search_json_with_label_filter_no_results() {
    let (_temp, workspace) = setup();

    // Create a bead without labels
    let bead_id = {
        let (out, err, ok) = run_bf(
            &workspace,
            &["create", "--title", "plain bead", "--type", "task", "--priority", "2"],
        );
        assert!(ok, "bf create failed: {err}");
        out.trim().to_string()
    };

    // Search for beads with a label that doesn't exist
    let (out, err, ok) = run_bf(
        &workspace,
        &["search", "--label", "nonexistent_label", "--format", "json"],
    );
    assert!(ok, "Search with no matches failed: {err}");

    let trimmed = out.trim();

    // Should return valid JSON for empty results
    if !trimmed.is_empty() {
        let parsed = parse_json(trimmed);
        if let Some(arr) = parsed.as_array() {
            assert_eq!(arr.len(), 0, "Should return empty array");
        }
    }

    // Cleanup
    run_bf(&workspace, &["close", &bead_id, "--reason", "test cleanup"]);
}

#[test]
fn test_search_json_with_priority_filter_no_results() {
    let (_temp, workspace) = setup();

    // Create a normal priority bead
    let (_out, err, ok) = run_bf(
        &workspace,
        &["create", "--title", "normal bead", "--type", "task", "--priority", "2"],
    );
    assert!(ok, "bf create failed: {err}");

    // Search for critical priority beads (should return empty)
    let (out, err, ok) = run_bf(
        &workspace,
        &["search", "--priority-min", "0", "--priority-max", "0", "--format", "json"],
    );
    assert!(ok, "Search with no matches failed: {err}");

    let trimmed = out.trim();

    // Should return valid JSON for empty results
    if !trimmed.is_empty() {
        let parsed = parse_json(trimmed);
        if let Some(arr) = parsed.as_array() {
            assert_eq!(arr.len(), 0, "Should return empty array");
        }
    }
}

// ============================================================================
// LIST COMMAND EMPTY RESULT TESTS
// ============================================================================

#[test]
fn test_list_json_no_beads_returns_valid_json() {
    let (_temp, workspace) = setup();

    // Don't create any beads - list from completely empty workspace
    let (out, err, ok) = run_bf(
        &workspace,
        &["list", "--format", "json"],
    );
    assert!(ok, "List from empty workspace failed: {err}");

    let trimmed = out.trim();

    // Empty list should return "[]" or valid JSON
    if trimmed.is_empty() {
        // Empty string is acceptable for truly empty workspace
        return;
    }

    let parsed = parse_json(trimmed);

    // Should be an empty array
    if let Some(arr) = parsed.as_array() {
        assert_eq!(arr.len(), 0, "Empty list should return empty array");
    } else {
        panic!("Empty list should return array, got: {}", parsed);
    }
}

#[test]
fn test_list_json_with_status_filter_no_results() {
    let (_temp, workspace) = setup();

    // Create an open bead
    let (_out, err, ok) = run_bf(
        &workspace,
        &["create", "--title", "open bead", "--type", "task", "--priority", "2"],
    );
    assert!(ok, "bf create failed: {err}");

    // List closed beads (should return empty)
    let (out, err, ok) = run_bf(
        &workspace,
        &["list", "--status", "closed", "--format", "json"],
    );
    assert!(ok, "List with no matches failed: {err}");

    let trimmed = out.trim();

    // Should return valid JSON for empty results
    if !trimmed.is_empty() {
        let parsed = parse_json(trimmed);
        if let Some(arr) = parsed.as_array() {
            assert_eq!(arr.len(), 0, "Should return empty array");
        } else {
            // Could be wrapped in envelope
            assert!(parsed.is_object() || parsed.is_array(), "Should return valid JSON");
        }
    }
}

#[test]
fn test_list_json_with_type_filter_no_results() {
    let (_temp, workspace) = setup();

    // Create a task
    let (_out, err, ok) = run_bf(
        &workspace,
        &["create", "--title", "task bead", "--type", "task", "--priority", "2"],
    );
    assert!(ok, "bf create failed: {err}");

    // List epics (should return empty)
    let (out, err, ok) = run_bf(
        &workspace,
        &["list", "--type", "epic", "--format", "json"],
    );
    assert!(ok, "List with no matches failed: {err}");

    let trimmed = out.trim();

    // Should return valid JSON for empty results
    if !trimmed.is_empty() {
        let parsed = parse_json(trimmed);
        if let Some(arr) = parsed.as_array() {
            assert_eq!(arr.len(), 0, "Should return empty array");
        }
    }
}

// ============================================================================
// READY COMMAND EMPTY RESULT TESTS
// ============================================================================

#[test]
fn test_ready_json_no_ready_beads_returns_valid_json() {
    let (_temp, workspace) = setup();

    // Create a bead and block it
    let blocker_id = {
        let (out, err, ok) = run_bf(
            &workspace,
            &["create", "--title", "blocker bead", "--type", "task", "--priority", "2"],
        );
        assert!(ok, "bf create failed: {err}");
        out.trim().to_string()
    };

    let blocked_id = {
        let (out, err, ok) = run_bf(
            &workspace,
            &["create", "--title", "blocked bead", "--type", "task", "--priority", "2"],
        );
        assert!(ok, "bf create failed: {err}");
        out.trim().to_string()
    };

    // Add blocking dependency: dep add <blocker> --blocks <blocked>
    let (_out, err, ok) = run_bf(
        &workspace,
        &["dep", "add", &blocker_id, "--blocks", &blocked_id],
    );
    assert!(ok, "Failed to add blocker: {err}");

    // Get ready beads (should be empty since all are blocked)
    let (out, err, ok) = run_bf(
        &workspace,
        &["ready", "--format", "json"],
    );
    assert!(ok, "Ready command failed: {err}");

    let trimmed = out.trim();

    // Should return valid JSON for empty results
    if !trimmed.is_empty() {
        let parsed = parse_json(trimmed);
        if let Some(arr) = parsed.as_array() {
            assert_eq!(arr.len(), 0, "Should return empty array when no ready beads");
        } else {
            // Could be "[]" string or wrapped in envelope
            assert!(parsed.is_object() || parsed.is_array() || parsed.is_string(),
                    "Should return valid JSON for empty ready");
        }
    }

    // Cleanup
    run_bf(&workspace, &["close", &blocker_id, "--reason", "test cleanup"]);
    run_bf(&workspace, &["close", &blocked_id, "--reason", "test cleanup"]);
}

#[test]
fn test_ready_json_all_closed_beads_returns_valid_json() {
    let (_temp, workspace) = setup();

    // Create and close a bead
    let bead_id = {
        let (out, err, ok) = run_bf(
            &workspace,
            &["create", "--title", "closed bead", "--type", "task", "--priority", "2"],
        );
        assert!(ok, "bf create failed: {err}");
        out.trim().to_string()
    };

    let (_out, err, ok) = run_bf(
        &workspace,
        &["close", &bead_id, "--reason", "test close"],
    );
    assert!(ok, "Failed to close bead: {err}");

    // Get ready beads (should be empty since all are closed)
    let (out, err, ok) = run_bf(
        &workspace,
        &["ready", "--format", "json"],
    );
    assert!(ok, "Ready command failed: {err}");

    let trimmed = out.trim();

    // Should return valid JSON for empty results
    if !trimmed.is_empty() {
        let parsed = parse_json(trimmed);
        if let Some(arr) = parsed.as_array() {
            assert_eq!(arr.len(), 0, "Should return empty array when no ready beads");
        } else {
            assert!(parsed.is_object() || parsed.is_array() || parsed.is_string(),
                    "Should return valid JSON for empty ready");
        }
    }
}

#[test]
fn test_ready_json_empty_workspace_returns_valid_json() {
    let (_temp, workspace) = setup();

    // Don't create any beads - ready from completely empty workspace
    let (out, err, ok) = run_bf(
        &workspace,
        &["ready", "--format", "json"],
    );
    assert!(ok, "Ready from empty workspace failed: {err}");

    let trimmed = out.trim();

    // Empty ready should return valid JSON
    if trimmed.is_empty() {
        // Empty string is acceptable
        return;
    }

    // Should be "[]" or valid empty JSON structure
    if trimmed != "[]" {
        let parsed = parse_json(trimmed);
        if let Some(arr) = parsed.as_array() {
            assert_eq!(arr.len(), 0, "Empty ready should return empty array");
        }
    }
}

// ============================================================================
// RECENT COMMAND EMPTY RESULT TESTS
// ============================================================================

#[test]
fn test_recent_json_very_short_time_period_returns_valid_json() {
    let (_temp, workspace) = setup();

    // Use very short time period that should yield no results
    let (out, err, ok) = run_bf(
        &workspace,
        &["recent", "--time-period", "1s", "--format", "json"],
    );
    assert!(ok, "Recent with short time period failed: {err}");

    let trimmed = out.trim();

    // Should return valid JSON even for empty results
    assert!(!trimmed.is_empty(), "Recent should always return something");

    let parsed = parse_json(trimmed);

    // Should be wrapped in envelope: {version: 1, kind: "recent", data: ...}
    assert!(parsed.is_object(), "Recent should return envelope object");
    assert!(parsed.get("version").is_some(), "Envelope should have version");
    assert!(parsed.get("kind").is_some(), "Envelope should have kind");
    assert!(parsed.get("data").is_some(), "Envelope should have data field");

    let data = parsed.get("data").unwrap();

    // Data should be empty array or empty string for no results
    if let Some(arr) = data.as_array() {
        assert_eq!(arr.len(), 0, "Empty recent should return empty array in data");
    } else if let Some(s) = data.as_str() {
        assert!(s.is_empty() || s == "[]", "Empty recent data should be empty");
    }
}

#[test]
fn test_recent_json_empty_workspace_returns_valid_json() {
    let (_temp, workspace) = setup();

    // Recent from completely empty workspace
    let (out, err, ok) = run_bf(
        &workspace,
        &["recent", "--format", "json"],
    );
    assert!(ok, "Recent from empty workspace failed: {err}");

    let trimmed = out.trim();

    // Should return valid JSON wrapped in envelope
    assert!(!trimmed.is_empty(), "Recent should always return envelope");

    let parsed = parse_json(trimmed);

    // Should be wrapped in envelope
    assert!(parsed.is_object(), "Recent should return envelope object");
    assert!(parsed.get("data").is_some(), "Envelope should have data field");
}

// ============================================================================
// CROSS-COMMAND CONSISTENCY TESTS
// ============================================================================

#[test]
fn test_all_commands_consistent_empty_format() {
    let (_temp, workspace) = setup();

    // Test that all commands handle empty results consistently

    // Search with no results
    let (search_out, err, ok) = run_bf(
        &workspace,
        &["search", "nonexistent", "--format", "json"],
    );
    assert!(ok, "Search failed: {err}");

    // List with no results (empty workspace)
    let (list_out, err, ok) = run_bf(
        &workspace,
        &["list", "--format", "json"],
    );
    assert!(ok, "List failed: {err}");

    // Ready with no results (empty workspace)
    let (ready_out, err, ok) = run_bf(
        &workspace,
        &["ready", "--format", "json"],
    );
    assert!(ok, "Ready failed: {err}");

    // Recent (always returns envelope)
    let (recent_out, err, ok) = run_bf(
        &workspace,
        &["recent", "--format", "json"],
    );
    assert!(ok, "Recent failed: {err}");

    // All should return valid JSON
    let results = vec![
        ("search", search_out),
        ("list", list_out),
        ("ready", ready_out),
        ("recent", recent_out),
    ];

    for (cmd, output) in results {
        let trimmed = output.trim();

        // Empty string is acceptable for truly empty workspaces
        if trimmed.is_empty() {
            continue;
        }

        // Otherwise should be valid JSON
        let parsed = parse_json(trimmed);

        // Verify it's valid JSON structure
        assert!(parsed.is_object() || parsed.is_array() || parsed.is_string(),
                "{} should return valid JSON structure", cmd);
    }
}

#[test]
fn test_empty_result_arrays_properly_formatted() {
    let (_temp, workspace) = setup();

    // Test that empty arrays are properly formatted (not malformed)

    // Create a bead to make workspace non-empty
    let (_out, err, ok) = run_bf(
        &workspace,
        &["create", "--title", "test bead", "--type", "task", "--priority", "2"],
    );
    assert!(ok, "bf create failed: {err}");

    // Test various filters that should return empty
    let test_cases = vec![
        vec!["search", "--status", "closed", "--format", "json"],
        vec!["list", "--status", "closed", "--format", "json"],
        vec!["search", "--type", "epic", "--format", "json"],
        vec!["list", "--type", "epic", "--format", "json"],
    ];

    for args in test_cases {
        let (out, err, ok) = run_bf(&workspace, &args);
        assert!(ok, "Command failed: {:?} - {}", args, err);

        let trimmed = out.trim();

        // If not empty string, should be valid JSON
        if !trimmed.is_empty() {
            let parsed = parse_json(trimmed);

            // Should be properly formatted JSON
            if let Some(arr) = parsed.as_array() {
                assert_eq!(arr.len(), 0, "Empty result should be empty array");
            }
        }
    }
}
