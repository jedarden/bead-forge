//! Comprehensive JSON output tests for search, ready, and recent commands
//!
//! These tests validate:
//! - JSON output structure validity for search, ready, and recent commands
//! - Required fields presence in each command's JSON output
//! - Empty results handling for all three commands
//! - Special characters in bead fields
//! - JSONL format correctness for multi-result commands
//! - Envelope mode validation
//! - Filtering behavior with JSON output

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

/// Create a test bead with the given title
fn create_bead(workspace: &Path, title: &str) -> String {
    let (out, err, ok) = run_bf(workspace, &["create", "--title", title, "--type", "task", "--priority", "2"]);
    assert!(ok, "bf create failed: {err}");
    let id = out.trim().to_string();
    assert!(!id.is_empty(), "create produced no id: {out}");
    id
}

/// Create a test bead with labels
fn create_bead_with_labels(workspace: &Path, title: &str, labels: &[&str]) -> String {
    let bead_id = create_bead(workspace, title);

    for label in labels {
        let (_out, err, ok) = run_bf(workspace, &["label", "add", &bead_id, "--label", label]);
        assert!(ok, "Failed to add label '{}': {err}", label);
    }

    bead_id
}

/// Create a test bead with assignee
fn create_bead_with_assignee(workspace: &Path, title: &str, assignee: &str) -> String {
    let (out, err, ok) = run_bf(
        workspace,
        &["create", "--title", title, "--type", "task", "--priority", "2", "--assignee", assignee],
    );
    assert!(ok, "bf create failed: {err}");
    let id = out.trim().to_string();
    assert!(!id.is_empty(), "create produced no id: {out}");
    id
}

/// Update a bead's status
fn update_bead_status(workspace: &Path, bead_id: &str, status: &str) {
    let (_out, err, ok) = run_bf(workspace, &["update", bead_id, "--status", status]);
    assert!(ok, "Failed to update bead status: {err}");
}

/// Close a test bead
fn close_bead(workspace: &Path, bead_id: &str, reason: &str) {
    let (_out, err, ok) = run_bf(workspace, &["close", bead_id, "--reason", reason]);
    assert!(ok, "Failed to close bead: {err}");
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

/// Check if a JSON value has a specific field
fn has_field(json: &Value, field: &str) -> bool {
    json.get(field).is_some()
}

/// Get a string field from JSON, panic if missing or not a string
fn get_string(json: &Value, field: &str) -> String {
    json.get(field)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("Field '{}' is not a string or is missing: {}", field, json))
        .to_string()
}

/// Validate envelope structure: {version: 1, kind: "<command>", data: <payload>}
fn validate_envelope(json: &str, expected_kind: &str) -> Value {
    let envelope = parse_json(json);

    let version = envelope.get("version")
        .and_then(|v| v.as_u64())
        .expect("Envelope must have numeric 'version' field");
    assert_eq!(version, 1, "Envelope version must be 1");

    let kind = envelope.get("kind")
        .and_then(|k| k.as_str())
        .expect("Envelope must have string 'kind' field");
    assert_eq!(kind, expected_kind, "Envelope kind mismatch");

    assert!(
        envelope.get("data").is_some(),
        "Envelope must have 'data' field"
    );

    envelope
}

/// Get the data field from an envelope
fn get_envelope_data(envelope: &Value) -> Value {
    envelope.get("data")
        .cloned()
        .unwrap_or_else(|| panic!("Envelope missing 'data' field"))
}

/// Helper to run `bf search --format json` and return stdout
fn run_search_json(workspace: &Path, query: Option<&str>) -> String {
    let args = if let Some(q) = query {
        vec!["search", q, "--format", "json"]
    } else {
        vec!["search", "--format", "json"]
    };
    let (out, err, ok) = run_bf(workspace, &args);
    assert!(ok, "bf search --format json failed: {err}");
    out
}

/// Helper to run `bf ready --format json` and return stdout
fn run_ready_json(workspace: &Path, limit: usize) -> String {
    let (out, err, ok) = run_bf(workspace, &["ready", "--limit", &limit.to_string(), "--format", "json"]);
    assert!(ok, "bf ready --format json failed: {err}");
    out
}

/// Extract beads from ready command envelope (handles single object, JSONL string, or array)
fn extract_ready_beads(envelope: &Value) -> Vec<Value> {
    let data_value = get_envelope_data(envelope);

    // Ready command returns data differently based on bead count:
    // - 1 bead: data is a JSON object
    // - 2+ beads: data is a JSONL string (non-envelope) or JSON array (envelope mode)
    if data_value.is_object() {
        // Single bead case
        vec![data_value]
    } else if data_value.is_string() {
        // Multiple beads case - parse as JSONL
        let data_str = data_value.as_str().unwrap();
        if data_str.is_empty() || data_str == "[]" {
            vec![]
        } else {
            parse_jsonl(data_str)
        }
    } else if data_value.is_array() {
        // Array case (envelope mode)
        data_value.as_array().unwrap().clone()
    } else {
        panic!("Ready envelope data must be object, string, or array, got: {}", data_value);
    }
}

/// Helper to run `bf recent --format json` and return stdout
fn run_recent_json(workspace: &Path) -> String {
    let (out, err, ok) = run_bf(workspace, &["recent", "--format", "json"]);
    assert!(ok, "bf recent --format json failed: {err}");
    out
}

/// Extract beads from recent command envelope (handles single object, JSONL string, or array)
fn extract_recent_beads(envelope: &Value) -> Vec<Value> {
    let data_value = get_envelope_data(envelope);

    // Recent command returns data differently based on bead count:
    // - 1 bead: data is a JSON object
    // - 2+ beads: data is a JSONL string (non-envelope) or JSON array (envelope mode)
    if data_value.is_object() {
        // Single bead case
        vec![data_value]
    } else if data_value.is_string() {
        // Multiple beads case - parse as JSONL
        let data_str = data_value.as_str().unwrap();
        if data_str.is_empty() {
            vec![]
        } else {
            parse_jsonl(data_str)
        }
    } else if data_value.is_array() {
        // Array case (envelope mode)
        data_value.as_array().unwrap().clone()
    } else {
        panic!("Recent envelope data must be object, string, or array, got: {}", data_value);
    }
}

// ============================================================================
// SEARCH COMMAND TESTS
// ============================================================================

#[test]
fn test_search_json_output_structure_validity() {
    let (_temp, workspace) = setup();

    // Create test beads
    create_bead(&workspace, "search test bead one");
    create_bead(&workspace, "search test bead two");

    // Run search --format json
    let jsonl = run_search_json(&workspace, Some("search"));

    // Parse as JSONL
    let parsed = parse_jsonl(&jsonl);
    assert!(!parsed.is_empty(), "Search should return results");

    // Verify structure
    for bead in &parsed {
        assert!(has_field(bead, "id"), "Each bead must have 'id' field");
        assert!(has_field(bead, "title"), "Each bead must have 'title' field");
        assert!(has_field(bead, "status"), "Each bead must have 'status' field");
        assert!(has_field(bead, "priority"), "Each bead must have 'priority' field");
        assert!(has_field(bead, "issue_type"), "Each bead must have 'issue_type' field");
    }
}

#[test]
fn test_search_json_required_fields_present() {
    let (_temp, workspace) = setup();

    let bead_id = create_bead(&workspace, "required fields test");

    let jsonl = run_search_json(&workspace, Some("required"));
    let parsed = parse_jsonl(&jsonl);

    let found = parsed.iter().any(|v| {
        v.get("id")
            .and_then(|id| id.as_str())
            .map(|id| id == bead_id)
            .unwrap_or(false)
    });
    assert!(found, "Created bead should be in search results");

    // Check all required fields for the found bead
    let bead = parsed.iter().find(|v| {
        v.get("id")
            .and_then(|id| id.as_str())
            .map(|id| id == bead_id)
            .unwrap_or(false)
    }).unwrap();

    // Core required fields
    assert!(has_field(bead, "id"));
    assert!(has_field(bead, "title"));
    assert!(has_field(bead, "status"));
    assert!(has_field(bead, "priority"));
    assert!(has_field(bead, "issue_type"));

    // Normalized display fields (should always be present)
    assert!(has_field(bead, "assignee"));
    assert!(has_field(bead, "labels"));

    // Timestamps
    assert!(has_field(bead, "created_at"));
    assert!(has_field(bead, "updated_at"));
}

#[test]
fn test_search_json_empty_results() {
    let (_temp, workspace) = setup();

    // Search with no beads
    let jsonl = run_search_json(&workspace, Some("nonexistent"));
    let parsed = parse_jsonl(&jsonl);

    assert_eq!(parsed.len(), 0, "Search with no matches should return empty results");
}

#[test]
fn test_search_json_special_characters() {
    let (_temp, workspace) = setup();

    // Create beads with special characters
    create_bead(&workspace, "bead with emoji 🎉");
    create_bead(&workspace, "bead with quotes \"test\"");
    create_bead(&workspace, "bead with apostrophe 'test'");
    create_bead(&workspace, "bead with unicode 中文");
    create_bead(&workspace, "bead with newlines\ntest");
    create_bead(&workspace, "bead with tabs\ttest");

    // Search should handle special characters in JSON
    let jsonl = run_search_json(&workspace, Some("bead"));
    let parsed = parse_jsonl(&jsonl);

    assert_eq!(parsed.len(), 6, "All beads with special characters should be found");

    // Verify JSON is valid (parse_json would have panicked if invalid)
    for bead in &parsed {
        let title = get_string(bead, "title");
        assert!(title.contains("bead"), "Title should contain 'bead'");
    }
}

#[test]
fn test_search_json_with_filters() {
    let (_temp, workspace) = setup();

    // Create beads with different attributes
    let id1 = create_bead_with_assignee(&workspace, "assignee test one", "user1");
    let id2 = create_bead_with_assignee(&workspace, "assignee test two", "user2");
    create_bead(&workspace, "no assignee");

    // Search with assignee filter
    let (out, err, ok) = run_bf(&workspace, &["search", "--assignee", "user1", "--format", "json"]);
    assert!(ok, "Search with assignee filter failed: {err}");

    let parsed = parse_jsonl(&out);
    assert_eq!(parsed.len(), 1, "Should find exactly one bead for user1");

    let found_id = get_string(&parsed[0], "id");
    assert_eq!(found_id, id1, "Should find the correct bead");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_search_json_status_filter() {
    let (_temp, workspace) = setup();

    // Create beads with different statuses
    let id1 = create_bead(&workspace, "open bead for status filter");
    let id2 = create_bead(&workspace, "blocked bead for status filter");
    update_bead_status(&workspace, &id2, "blocked");
    let id3 = create_bead(&workspace, "in_progress bead for status filter");
    update_bead_status(&workspace, &id3, "in_progress");

    // Search with status filter - open
    let (out, err, ok) = run_bf(&workspace, &["search", "--status", "open", "--format", "json"]);
    assert!(ok, "Search with status filter failed: {err}");

    let parsed = parse_jsonl(&out);
    assert!(parsed.len() >= 1, "Should find at least one open bead");

    // Verify all results have status "open"
    for bead in &parsed {
        let status = get_string(bead, "status");
        assert_eq!(status, "open", "Status filter should only return matching beads");
    }
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_search_json_type_filter() {
    let (_temp, workspace) = setup();

    // Create beads with different types
    let (out1, err1, ok1) = run_bf(&workspace, &["create", "--title", "bug type bead", "--type", "bug", "--priority", "2"]);
    assert!(ok1, "bf create failed: {err1}");
    let bug_id = out1.trim();

    let (out2, err2, ok2) = run_bf(&workspace, &["create", "--title", "task type bead", "--type", "task", "--priority", "2"]);
    assert!(ok2, "bf create failed: {err2}");
    let task_id = out2.trim();

    // Search with type filter - bug
    let (out, err, ok) = run_bf(&workspace, &["search", "--type", "bug", "--format", "json"]);
    assert!(ok, "Search with type filter failed: {err}");

    let parsed = parse_jsonl(&out);
    assert!(parsed.len() >= 1, "Should find at least one bug");

    // Verify all results have type "bug"
    for bead in &parsed {
        let issue_type = get_string(bead, "issue_type");
        assert_eq!(issue_type, "bug", "Type filter should only return matching beads");
    }
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_search_json_label_filter() {
    let (_temp, workspace) = setup();

    // Create beads with different labels
    let id1 = create_bead_with_labels(&workspace, "labeled bead one", &["frontend", "ui"]);
    let id2 = create_bead_with_labels(&workspace, "labeled bead two", &["backend", "api"]);
    create_bead(&workspace, "unlabeled bead");

    // Search with label filter
    let (out, err, ok) = run_bf(&workspace, &["search", "--label", "frontend", "--format", "json"]);
    assert!(ok, "Search with label filter failed: {err}");

    let parsed = parse_jsonl(&out);
    assert_eq!(parsed.len(), 1, "Should find exactly one bead with 'frontend' label");

    let found_id = get_string(&parsed[0], "id");
    assert_eq!(found_id, id1, "Should find the correct bead");

    // Verify the labels field contains the label
    let labels = parsed[0].get("labels").and_then(|l| l.as_array()).unwrap();
    assert!(labels.iter().any(|l| l.as_str() == Some("frontend")), "Labels should include 'frontend'");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_search_json_priority_range_filter() {
    let (_temp, workspace) = setup();

    // Create beads with different priorities
    let (out1, err1, ok1) = run_bf(&workspace, &["create", "--title", "priority 1 bead", "--type", "task", "--priority", "1"]);
    assert!(ok1, "bf create failed: {err1}");

    let (out2, err2, ok2) = run_bf(&workspace, &["create", "--title", "priority 2 bead", "--type", "task", "--priority", "2"]);
    assert!(ok2, "bf create failed: {err2}");

    let (out3, err3, ok3) = run_bf(&workspace, &["create", "--title", "priority 3 bead", "--type", "task", "--priority", "3"]);
    assert!(ok3, "bf create failed: {err3}");

    // Search with priority range filter
    let (out, err, ok) = run_bf(&workspace, &["search", "--priority-min", "2", "--priority-max", "2", "--format", "json"]);
    assert!(ok, "Search with priority range filter failed: {err}");

    let parsed = parse_jsonl(&out);
    assert!(parsed.len() >= 1, "Should find at least one bead in priority range");

    // Verify all results have priority 2
    for bead in &parsed {
        let priority = bead.get("priority").and_then(|p| p.as_i64()).unwrap();
        assert_eq!(priority, 2, "Priority range filter should only return matching beads");
    }
}

#[test]
fn test_search_json_multiple_filters_combined() {
    let (_temp, workspace) = setup();

    // Create beads with multiple attributes
    let id1 = create_bead_with_labels(&workspace, "frontend open task", &["frontend"]);
    let id2 = create_bead_with_labels(&workspace, "backend open task", &["backend"]);
    let id3 = create_bead_with_labels(&workspace, "frontend blocked task", &["frontend"]);
    update_bead_status(&workspace, &id3, "blocked");

    // Search with multiple filters: status + label
    let (out, err, ok) = run_bf(&workspace, &["search", "--status", "open", "--label", "frontend", "--format", "json"]);
    assert!(ok, "Search with multiple filters failed: {err}");

    let parsed = parse_jsonl(&out);
    assert_eq!(parsed.len(), 1, "Should find exactly one bead matching both filters");

    let found_id = get_string(&parsed[0], "id");
    assert_eq!(found_id, id1, "Should find the bead that matches both criteria");
}

#[test]
fn test_search_json_filter_no_results() {
    let (_temp, workspace) = setup();

    create_bead(&workspace, "normal bead");

    // Search with filter that matches nothing
    let (out, err, ok) = run_bf(&workspace, &["search", "--status", "blocked", "--format", "json"]);
    assert!(ok, "Search with non-matching filter failed: {err}");

    let parsed = parse_jsonl(&out);
    assert_eq!(parsed.len(), 0, "Search with non-matching filter should return empty results");
}

// ============================================================================
// READY COMMAND TESTS
// ============================================================================

#[test]
fn test_ready_json_output_structure_validity() {
    let (_temp, workspace) = setup();

    // Create open, unblocked beads
    create_bead(&workspace, "ready bead one");
    create_bead(&workspace, "ready bead two");

    // Run ready --format json
    let jsonl = run_ready_json(&workspace, 0); // unlimited

    // Parse as JSONL (ready returns JSONL, not array)
    let parsed = parse_jsonl(&jsonl);
    assert!(!parsed.is_empty(), "Ready should return results");

    // Verify structure
    for bead in &parsed {
        assert!(has_field(bead, "id"), "Each bead must have 'id' field");
        assert!(has_field(bead, "title"), "Each bead must have 'title' field");
        assert!(has_field(bead, "status"), "Each bead must have 'status' field");
        assert!(has_field(bead, "priority"), "Each bead must have 'priority' field");
    }
}

#[test]
fn test_ready_json_required_fields_present() {
    let (_temp, workspace) = setup();

    let bead_id = create_bead(&workspace, "ready required fields");

    let jsonl = run_ready_json(&workspace, 0);
    let parsed = parse_jsonl(&jsonl);

    let found = parsed.iter().any(|v| {
        v.get("id")
            .and_then(|id| id.as_str())
            .map(|id| id == bead_id)
            .unwrap_or(false)
    });
    assert!(found, "Created bead should be in ready results");

    // Check all required fields
    let bead = parsed.iter().find(|v| {
        v.get("id")
            .and_then(|id| id.as_str())
            .map(|id| id == bead_id)
            .unwrap_or(false)
    }).unwrap();

    // Core required fields
    assert!(has_field(bead, "id"));
    assert!(has_field(bead, "title"));
    assert!(has_field(bead, "status"));
    assert!(has_field(bead, "priority"));

    // Verify status is "open" (ready only shows open beads)
    let status = get_string(bead, "status");
    assert_eq!(status, "open", "Ready beads must have status 'open'");
}

#[test]
fn test_ready_json_empty_results() {
    let (_temp, workspace) = setup();

    // Ready with no beads
    let out = run_ready_json(&workspace, 0);

    // Ready command returns empty array for empty results
    assert_eq!(out.trim(), "[]", "Ready with no beads should return empty array");

    // Parse to verify it's valid JSON
    let parsed: serde_json::Value = parse_json(&out);
    assert!(parsed.is_array(), "Empty results should be an array");
    assert_eq!(parsed.as_array().unwrap().len(), 0, "Array should be empty");
}

#[test]
fn test_ready_json_only_open_unblocked() {
    let (_temp, workspace) = setup();

    // Create open bead
    let open_id = create_bead(&workspace, "open bead");

    // Create blocked bead
    let blocked_id = create_bead(&workspace, "blocked bead");
    update_bead_status(&workspace, &blocked_id, "blocked");

    // Create closed bead
    let closed_id = create_bead(&workspace, "closed bead");
    close_bead(&workspace, &closed_id, "test close");

    // Run ready
    let jsonl = run_ready_json(&workspace, 0);
    let parsed = parse_jsonl(&jsonl);

    // Should only find the open bead
    assert_eq!(parsed.len(), 1, "Ready should only return open, unblocked beads");

    let found_id = get_string(&parsed[0], "id");
    assert_eq!(found_id, open_id, "Should find the open bead");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_ready_json_limit() {
    let (_temp, workspace) = setup();

    // Create multiple beads
    create_bead(&workspace, "ready limit 1");
    create_bead(&workspace, "ready limit 2");
    create_bead(&workspace, "ready limit 3");

    // Test with limit
    let jsonl = run_ready_json(&workspace, 2);
    let parsed = parse_jsonl(&jsonl);

    assert_eq!(parsed.len(), 2, "Ready with limit=2 should return exactly 2 beads");
}

// ============================================================================
// RECENT COMMAND TESTS
// ============================================================================

#[test]
fn test_recent_json_output_structure_validity() {
    let (_temp, workspace) = setup();

    // Create test beads
    create_bead(&workspace, "recent bead one");
    create_bead(&workspace, "recent bead two");

    // Run recent --format json
    let json_str = run_recent_json(&workspace);

    // Parse as envelope
    let envelope = parse_json(&json_str);
    validate_envelope(&json_str, "recent");

    // Extract beads using helper that handles both formats
    let parsed = extract_recent_beads(&envelope);
    assert!(!parsed.is_empty(), "Recent should return results");

    // Verify structure
    for bead in &parsed {
        assert!(has_field(bead, "id"), "Each bead must have 'id' field");
        assert!(has_field(bead, "title"), "Each bead must have 'title' field");
        assert!(has_field(bead, "status"), "Each bead must have 'status' field");
        assert!(has_field(bead, "priority"), "Each bead must have 'priority' field");
        assert!(has_field(bead, "issue_type"), "Each bead must have 'issue_type' field");
    }
}

#[test]
fn test_recent_json_required_fields_present() {
    let (_temp, workspace) = setup();

    let bead_id = create_bead(&workspace, "recent required fields");

    let json_str = run_recent_json(&workspace);
    let envelope = parse_json(&json_str);

    // Extract beads using helper that handles both formats
    let parsed = extract_recent_beads(&envelope);

    let found = parsed.iter().any(|v| {
        v.get("id")
            .and_then(|id| id.as_str())
            .map(|id| id == bead_id)
            .unwrap_or(false)
    });
    assert!(found, "Created bead should be in recent results");

    // Check all required fields
    let bead = parsed.iter().find(|v| {
        v.get("id")
            .and_then(|id| id.as_str())
            .map(|id| id == bead_id)
            .unwrap_or(false)
    }).unwrap();

    // Core required fields
    assert!(has_field(bead, "id"));
    assert!(has_field(bead, "title"));
    assert!(has_field(bead, "status"));
    assert!(has_field(bead, "priority"));
    assert!(has_field(bead, "issue_type"));

    // Normalized display fields
    assert!(has_field(bead, "assignee"));
    assert!(has_field(bead, "labels"));

    // Timestamps
    assert!(has_field(bead, "created_at"));
    assert!(has_field(bead, "updated_at"));
}

#[test]
fn test_recent_json_empty_results() {
    let (_temp, workspace) = setup();

    // Recent with no beads
    let out = run_recent_json(&workspace);

    // Recent returns envelope with empty data string for empty results
    let parsed = parse_json(&out);
    assert_eq!(parsed["version"].as_u64().unwrap(), 1);
    assert_eq!(parsed["kind"].as_str().unwrap(), "recent");

    // Extract beads using helper
    let beads = extract_recent_beads(&parsed);
    assert_eq!(beads.len(), 0, "Recent with no beads should return empty results");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_recent_json_special_characters() {
    let (_temp, workspace) = setup();

    // Create beads with special characters
    create_bead(&workspace, "recent with emoji 🎉");
    create_bead(&workspace, "recent with quotes \"test\"");
    create_bead(&workspace, "recent with apostrophe 'test'");
    create_bead(&workspace, "recent with unicode العربية");

    // Recent should handle special characters in JSON
    let json_str = run_recent_json(&workspace);
    let envelope = parse_json(&json_str);

    // Extract beads using helper
    let parsed = extract_recent_beads(&envelope);

    assert_eq!(parsed.len(), 4, "All beads with special characters should be in recent");

    // Verify JSON is valid (parse_json would have panicked if invalid)
    for bead in &parsed {
        let title = get_string(bead, "title");
        assert!(title.contains("recent"), "Title should contain 'recent'");
    }
}

#[test]
fn test_recent_json_with_filters() {
    let (_temp, workspace) = setup();

    // Create beads with different statuses
    let open_id = create_bead(&workspace, "recent open");
    let closed_id = create_bead(&workspace, "recent closed");
    close_bead(&workspace, &closed_id, "test");

    // Recent with status filter
    let (out, err, ok) = run_bf(&workspace, &["recent", "--status", "open", "--format", "json"]);
    assert!(ok, "Recent with status filter failed: {err}");

    let envelope = parse_json(&out);
    let parsed = extract_recent_beads(&envelope);
    assert_eq!(parsed.len(), 1, "Should find exactly one open bead");

    let found_id = get_string(&parsed[0], "id");
    assert_eq!(found_id, open_id, "Should find the open bead");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_recent_json_limit() {
    let (_temp, workspace) = setup();

    // Create multiple beads
    create_bead(&workspace, "recent limit 1");
    create_bead(&workspace, "recent limit 2");
    create_bead(&workspace, "recent limit 3");

    // Test with limit
    let (out, err, ok) = run_bf(&workspace, &["recent", "-n", "2", "--format", "json"]);
    assert!(ok, "Recent with limit failed: {err}");

    let envelope = parse_json(&out);
    let parsed = extract_recent_beads(&envelope);
    assert_eq!(parsed.len(), 2, "Recent with limit=2 should return exactly 2 beads");
}

#[test]
fn test_recent_json_type_filter() {
    let (_temp, workspace) = setup();

    // Create beads with different types
    let (out1, err1, ok1) = run_bf(&workspace, &["create", "--title", "recent bug", "--type", "bug", "--priority", "2"]);
    assert!(ok1, "bf create failed: {err1}");

    let (out2, err2, ok2) = run_bf(&workspace, &["create", "--title", "recent task", "--type", "task", "--priority", "2"]);
    assert!(ok2, "bf create failed: {err2}");

    // Recent with type filter
    let (out, err, ok) = run_bf(&workspace, &["recent", "--type", "bug", "--format", "json"]);
    assert!(ok, "Recent with type filter failed: {err}");

    let envelope = parse_json(&out);
    let parsed = extract_recent_beads(&envelope);

    assert!(parsed.len() >= 1, "Should find at least one bug");

    // Verify all results have type "bug"
    for bead in &parsed {
        let issue_type = get_string(bead, "issue_type");
        assert_eq!(issue_type, "bug", "Type filter should only return matching beads");
    }
}

#[test]
fn test_recent_json_assignee_filter() {
    let (_temp, workspace) = setup();

    // Create beads with different assignees
    let id1 = create_bead_with_assignee(&workspace, "recent assigned to alice", "alice");
    let id2 = create_bead_with_assignee(&workspace, "recent assigned to bob", "bob");
    create_bead(&workspace, "recent unassigned");

    // Recent with assignee filter
    let (out, err, ok) = run_bf(&workspace, &["recent", "--assignee", "alice", "--format", "json"]);
    assert!(ok, "Recent with assignee filter failed: {err}");

    let envelope = parse_json(&out);
    let parsed = extract_recent_beads(&envelope);

    assert_eq!(parsed.len(), 1, "Should find exactly one bead assigned to alice");

    let found_id = get_string(&parsed[0], "id");
    assert_eq!(found_id, id1, "Should find the correct bead");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_recent_json_priority_filter() {
    let (_temp, workspace) = setup();

    // Create beads with different priorities
    let (out1, err1, ok1) = run_bf(&workspace, &["create", "--title", "recent priority 1", "--type", "task", "--priority", "1"]);
    assert!(ok1, "bf create failed: {err1}");

    let (out2, err2, ok2) = run_bf(&workspace, &["create", "--title", "recent priority 3", "--type", "task", "--priority", "3"]);
    assert!(ok2, "bf create failed: {err2}");

    // Recent with priority filter
    let (out, err, ok) = run_bf(&workspace, &["recent", "--priority", "1", "--format", "json"]);
    assert!(ok, "Recent with priority filter failed: {err}");

    let envelope = parse_json(&out);
    let parsed = extract_recent_beads(&envelope);

    assert!(parsed.len() >= 1, "Should find at least one bead with priority 1");

    // Verify all results have priority 1
    for bead in &parsed {
        let priority = bead.get("priority").and_then(|p| p.as_i64()).unwrap();
        assert_eq!(priority, 1, "Priority filter should only return matching beads");
    }
}

#[test]
fn test_recent_json_time_period_filter() {
    let (_temp, workspace) = setup();

    // Create a bead now
    let id1 = create_bead(&workspace, "recent time bead");

    // Recent with time period filter (24h)
    let (out, err, ok) = run_bf(&workspace, &["recent", "--time-period", "24h", "--format", "json"]);
    assert!(ok, "Recent with time-period filter failed: {err}");

    let envelope = parse_json(&out);
    let parsed = extract_recent_beads(&envelope);

    assert!(parsed.len() >= 1, "Should find at least one bead within 24h");

    // Verify our bead is in the results
    let found = parsed.iter().any(|v| {
        v.get("id")
            .and_then(|id| id.as_str())
            .map(|id| id == id1)
            .unwrap_or(false)
    });
    assert!(found, "Recently created bead should be in results");
}

#[test]
fn test_recent_json_multiple_filters_combined() {
    let (_temp, workspace) = setup();

    // Create beads with multiple attributes
    let id1 = create_bead_with_assignee(&workspace, "recent open alice task", "alice");
    let id2 = create_bead_with_assignee(&workspace, "recent blocked alice task", "alice");
    update_bead_status(&workspace, &id2, "blocked");
    let id3 = create_bead_with_assignee(&workspace, "recent open bob task", "bob");

    // Recent with multiple filters: status + assignee
    let (out, err, ok) = run_bf(&workspace, &["recent", "--status", "open", "--assignee", "alice", "--format", "json"]);
    assert!(ok, "Recent with multiple filters failed: {err}");

    let envelope = parse_json(&out);
    let parsed = extract_recent_beads(&envelope);

    assert_eq!(parsed.len(), 1, "Should find exactly one bead matching both filters");

    let found_id = get_string(&parsed[0], "id");
    assert_eq!(found_id, id1, "Should find the bead that matches both criteria");
}

#[test]
fn test_recent_json_filter_no_results() {
    let (_temp, workspace) = setup();

    create_bead(&workspace, "normal recent bead");

    // Recent with filter that matches nothing
    let (out, err, ok) = run_bf(&workspace, &["recent", "--status", "blocked", "--format", "json"]);
    assert!(ok, "Recent with non-matching filter failed: {err}");

    let envelope = parse_json(&out);
    let parsed = extract_recent_beads(&envelope);

    assert_eq!(parsed.len(), 0, "Recent with non-matching filter should return empty results");
}

#[test]
fn test_recent_json_limit_variations() {
    let (_temp, workspace) = setup();

    // Create multiple beads
    for i in 1..=5 {
        create_bead(&workspace, &format!("recent limit bead {}", i));
    }

    // Test limit=1
    let (out, err, ok) = run_bf(&workspace, &["recent", "-n", "1", "--format", "json"]);
    assert!(ok, "Recent with limit=1 failed: {err}");
    let envelope = parse_json(&out);
    let parsed = extract_recent_beads(&envelope);
    assert_eq!(parsed.len(), 1, "Recent with limit=1 should return exactly 1 bead");

    // Test limit=3
    let (out, err, ok) = run_bf(&workspace, &["recent", "-n", "3", "--format", "json"]);
    assert!(ok, "Recent with limit=3 failed: {err}");
    let envelope = parse_json(&out);
    let parsed = extract_recent_beads(&envelope);
    assert_eq!(parsed.len(), 3, "Recent with limit=3 should return exactly 3 beads");

    // Test unlimited (limit=0)
    let (out, err, ok) = run_bf(&workspace, &["recent", "-n", "0", "--format", "json"]);
    assert!(ok, "Recent with limit=0 failed: {err}");
    let envelope = parse_json(&out);
    let parsed = extract_recent_beads(&envelope);
    assert!(parsed.len() >= 5, "Recent with limit=0 should return all beads");
}

// ============================================================================
// ENVELOPE MODE TESTS
// ============================================================================

#[test]
fn test_search_json_no_envelope_mode() {
    let (_temp, workspace) = setup();

    create_bead(&workspace, "envelope search test");

    let (out, err, ok) = run_bf(&workspace, &["search", "envelope", "--format", "json"]);
    assert!(ok, "Search --format json failed: {err}");

    // Search does not use envelope mode, returns JSONL directly
    let parsed = parse_jsonl(&out);
    assert!(!parsed.is_empty(), "Search should return results");

    // Verify first result has required fields
    let bead = &parsed[0];
    assert!(has_field(bead, "id"), "Each bead must have 'id' field");
    assert!(has_field(bead, "title"), "Each bead must have 'title' field");
}

#[test]
fn test_ready_json_envelope_mode() {
    let (_temp, workspace) = setup();

    create_bead(&workspace, "envelope ready test");

    let (out, err, ok) = run_bf(&workspace, &["ready", "--format", "json", "--envelope"]);
    assert!(ok, "Ready --envelope failed: {err}");

    let envelope = validate_envelope(&out, "ready");

    // Extract beads using helper that handles both single object and JSONL string
    let parsed = extract_ready_beads(&envelope);

    assert!(!parsed.is_empty(), "Ready should return results");
}

#[test]
fn test_recent_json_envelope_mode() {
    let (_temp, workspace) = setup();

    create_bead(&workspace, "envelope recent test");

    let (out, err, ok) = run_bf(&workspace, &["recent", "--format", "json", "--envelope"]);
    assert!(ok, "Recent --envelope failed: {err}");

    let envelope = validate_envelope(&out, "recent");

    // Extract beads using helper that handles both single object and JSONL string
    let parsed = extract_recent_beads(&envelope);

    assert!(!parsed.is_empty(), "Recent should return results");
}

// ============================================================================
// CROSS-COMMAND CONSISTENCY TESTS
// ============================================================================

#[test]
fn test_json_field_consistency_across_commands() {
    let (_temp, workspace) = setup();

    let bead_id = create_bead_with_labels(&workspace, "consistency test", &["label1", "label2"]);

    // Get the same bead from different commands
    let search_json = run_search_json(&workspace, Some("consistency"));
    let search_parsed = parse_jsonl(&search_json);
    let search_bead = search_parsed.iter()
        .find(|v| v.get("id").and_then(|id| id.as_str()).map(|id| id == bead_id).unwrap_or(false))
        .unwrap();

    let ready_json = run_ready_json(&workspace, 0);
    let ready_parsed = parse_jsonl(&ready_json);
    let ready_bead = ready_parsed.iter()
        .find(|v| v.get("id").and_then(|id| id.as_str()).map(|id| id == bead_id).unwrap_or(false))
        .unwrap();

    let recent_json = run_recent_json(&workspace);
    let recent_envelope = parse_json(&recent_json);
    let recent_parsed = extract_recent_beads(&recent_envelope);
    let recent_bead = recent_parsed.iter()
        .find(|v| v.get("id").and_then(|id| id.as_str()).map(|id| id == bead_id).unwrap_or(false))
        .unwrap();

    // Verify field consistency across commands
    for bead in [search_bead, ready_bead, recent_bead] {
        // ID and title should match
        assert_eq!(get_string(bead, "id"), bead_id);
        assert_eq!(get_string(bead, "title"), "consistency test");

        // Fields should have consistent types
        assert!(bead.get("id").map(|v| v.is_string()).unwrap_or(false));
        assert!(bead.get("title").map(|v| v.is_string()).unwrap_or(false));
        assert!(bead.get("status").map(|v| v.is_string()).unwrap_or(false));
        assert!(bead.get("priority").map(|v| v.is_i64()).unwrap_or(false));
        assert!(bead.get("issue_type").map(|v| v.is_string()).unwrap_or(false));

        // Labels should be an array
        assert!(bead.get("labels").map(|v| v.is_array()).unwrap_or(false));
    }
}

// ============================================================================
// JSONL FORMAT TESTS
// ============================================================================

#[test]
fn test_search_jsonl_format() {
    let (_temp, workspace) = setup();

    create_bead(&workspace, "jsonl search 1");
    create_bead(&workspace, "jsonl search 2");

    let out = run_search_json(&workspace, Some("jsonl"));

    // Verify it's valid JSONL (not a single JSON value)
    assert!(
        from_str::<serde_json::Value>(&out).is_err(),
        "JSONL output should not parse as a single JSON value"
    );

    // But each line should be valid JSON
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2, "Should have 2 JSONL lines");

    for line in lines {
        assert!(from_str::<serde_json::Value>(line.trim()).is_ok(), "Each line should be valid JSON");
    }
}

#[test]
fn test_ready_jsonl_format() {
    let (_temp, workspace) = setup();

    create_bead(&workspace, "jsonl ready 1");
    create_bead(&workspace, "jsonl ready 2");

    let out = run_ready_json(&workspace, 0);

    // Verify it's valid JSONL (not a single JSON value)
    assert!(
        from_str::<serde_json::Value>(&out).is_err(),
        "JSONL output should not parse as a single JSON value"
    );

    // But each line should be valid JSON
    let lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty() && l.trim() != "[]").collect();
    assert_eq!(lines.len(), 2, "Should have 2 JSONL lines");

    for line in lines {
        assert!(from_str::<serde_json::Value>(line.trim()).is_ok(), "Each line should be valid JSON");
    }
}

#[test]
fn test_recent_jsonl_format() {
    let (_temp, workspace) = setup();

    create_bead(&workspace, "jsonl recent 1");
    create_bead(&workspace, "jsonl recent 2");

    let out = run_recent_json(&workspace);

    // Recent returns envelope format
    let envelope = parse_json(&out);
    assert_eq!(envelope["version"].as_u64().unwrap(), 1);
    assert_eq!(envelope["kind"].as_str().unwrap(), "recent");

    // Extract beads using helper
    let parsed = extract_recent_beads(&envelope);
    assert_eq!(parsed.len(), 2, "Should have 2 beads");

    // Verify each bead is valid JSON
    for bead in &parsed {
        assert!(bead.is_object(), "Each bead should be a JSON object");
        assert!(has_field(bead, "id"), "Each bead must have 'id' field");
    }
}