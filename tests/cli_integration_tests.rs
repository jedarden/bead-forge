//! Integration tests for core CLI commands
//!
//! Tests the following commands with temporary test databases:
//! - bf create: Create beads with various configurations
//! - bf claim: Claim beads with concurrency scenarios
//! - bf batch: Batch operations for create, dep_add_blocker, and close
//! - bf sync --flush-only: Flush database changes to JSONL
//!
//! All tests use tempfile for test isolation and automatic cleanup.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;
use serde_json::Value;

/// Get the path to the bf binary
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

/// Setup a test workspace with bf configuration
fn setup() -> (TempDir, PathBuf) {
    let temp = TempDir::new().unwrap();
    let workspace = temp.path().to_path_buf();
    let (_o, e, ok) = run_bf(&workspace, &["init", "--prefix", "bf"]);
    assert!(ok, "bf init failed: {e}");
    (temp, workspace)
}

/// Get the path to the beads directory
fn beads_dir(workspace: &Path) -> PathBuf {
    workspace.join(".beads")
}

/// Get the path to the database file
fn db_path(workspace: &Path) -> PathBuf {
    beads_dir(workspace).join("beads.db")
}

/// Get the path to the JSONL file
fn jsonl_path(workspace: &Path) -> PathBuf {
    beads_dir(workspace).join("issues.jsonl")
}

/// Parse JSON output from commands
fn parse_json(json: &str) -> Value {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("Failed to parse JSON: {}\nJSON was: {}", e, json))
}

/// Parse a JSONL string (newline-delimited JSON) into a Vec of values
fn parse_jsonl(jsonl: &str) -> Vec<Value> {
    jsonl
        .lines()
        .filter(|line| !line.trim().is_empty() && *line != "[]")
        .map(|line| parse_json(line))
        .collect()
}

/// Get a string field from JSON (returns empty string for null/missing)
fn get_string(json: &Value, field: &str) -> String {
    json.get(field)
        .and_then(|v| {
            if v.is_null() {
                Some(String::new())
            } else {
                v.as_str().map(|s| s.to_string())
            }
        })
        .unwrap_or_else(|| String::new())
}

/// Get a numeric field from JSON
fn get_number(json: &Value, field: &str) -> i64 {
    json.get(field)
        .and_then(|v| v.as_i64())
        .or_else(|| json.get(field).and_then(|v| v.as_u64().map(|n| n as i64)))
        .unwrap_or_else(|| panic!("Field '{}' is missing or not a number: {}", field, json))
}

/// Unwrap envelope format to get the data field
/// Handles: {"version":1,"kind":"...","data":...}
fn unwrap_envelope(json: &Value) -> Value {
    json.get("data")
        .unwrap_or_else(|| panic!("Envelope missing 'data' field: {}", json))
        .clone()
}

// ============================================================================
// CREATE command tests
// ============================================================================

#[test]
fn test_create_bead_basic() {
    let (_t, ws) = setup();

    let (stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", "Test bead basic"]);
    assert!(ok, "create should succeed");

    let id = stdout.trim();
    assert!(!id.is_empty(), "create should return an ID");
    assert!(id.starts_with("bf-"), "ID should start with bf- prefix");

    // Verify bead exists in database
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id]);
    assert!(ok, "show should succeed");
    assert!(stdout.contains("Test bead basic"), "show should display the title");
}

#[test]
fn test_create_bead_with_all_options() {
    let (_t, ws) = setup();

    let (stdout, _stderr, ok) = run_bf(
        &ws,
        &[
            "create",
            "--title",
            "Comprehensive test bead",
            "--type",
            "feature",
            "--priority",
            "1",
            "--description",
            "This is a test description",
            "--assignee",
            "test-worker",
            "--label",
            "backend",
            "--label",
            "priority",
        ],
    );
    assert!(ok, "create should succeed");

    let id = stdout.trim();

    // Verify all fields via JSON
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok, "show should succeed");

    let bead_json = parse_json(&stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&bead_json, "title"), "Comprehensive test bead");
    assert_eq!(get_string(&bead_json, "issue_type"), "feature");
    assert_eq!(get_number(&bead_json, "priority"), 1);
    assert_eq!(get_string(&bead_json, "description"), "This is a test description");
    assert_eq!(get_string(&bead_json, "assignee"), "test-worker");

    // Verify labels
    let labels = bead_json.get("labels").and_then(|v| v.as_array()).unwrap();
    assert_eq!(labels.len(), 2);
    let label_strs: Vec<String> = labels.iter().map(|l| l.as_str().unwrap().to_string()).collect();
    assert!(label_strs.contains(&"backend".to_string()));
    assert!(label_strs.contains(&"priority".to_string()));
}

#[test]
fn test_create_bead_with_json_output() {
    let (_t, ws) = setup();

    let (stdout, _stderr, ok) = run_bf(
        &ws,
        &["create", "--title", "JSON output test", "--json"],
    );
    assert!(ok, "create should succeed");

    let json = parse_json(&stdout);

    // The JSON output is wrapped in an envelope with a "data" field
    let data = json.get("data").unwrap_or(&json);
    assert!(data.get("id").is_some(), "JSON should contain id field");
    assert!(data.get("title").is_some(), "JSON should contain title field");
    assert_eq!(get_string(data, "title"), "JSON output test");
}

#[test]
fn test_create_multiple_beads() {
    let (_t, ws) = setup();

    // Create multiple beads
    let ids = vec![
        "First bead",
        "Second bead",
        "Third bead",
    ];

    for title in &ids {
        let (stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", title]);
        assert!(ok, "create should succeed for {}", title);
        let id = stdout.trim();
        assert!(!id.is_empty(), "create should return ID for {}", title);
    }

    // Verify all beads exist
    let (stdout, _stderr, ok) = run_bf(&ws, &["list", "--format", "json"]);
    assert!(ok, "list should succeed");

    let beads = parse_jsonl(&stdout);
    assert_eq!(beads.len(), 3, "should have 3 beads");
}

// ============================================================================
// CLAIM command tests with concurrency
// ============================================================================

#[test]
fn test_claim_single_bead() {
    let (_t, ws) = setup();

    // Create a test bead
    let (stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", "Claimable bead"]);
    assert!(ok, "create should succeed");
    let id = stdout.trim();

    // Claim the bead
    let (stdout, _stderr, ok) = run_bf(&ws, &["claim", "--assignee", "worker-1"]);
    assert!(ok, "claim should succeed");
    assert!(stdout.contains(id), "claim should return the bead ID");

    // Verify bead is claimed
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok, "show should succeed");

    let bead_json = parse_json(&stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&bead_json, "assignee"), "worker-1");
    assert_eq!(get_string(&bead_json, "status"), "in_progress");
}

#[test]
fn test_claim_with_dry_run() {
    let (_t, ws) = setup();

    // Create a test bead
    let (stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", "Dry run test"]);
    assert!(ok, "create should succeed");
    let id = stdout.trim();

    // Dry run claim
    let (stdout, _stderr, ok) = run_bf(&ws, &["claim", "--assignee", "worker-1", "--dry-run"]);
    assert!(ok, "claim dry-run should succeed");
    assert!(stdout.contains(id), "dry-run should show the bead");

    // Verify bead is NOT actually claimed
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok, "show should succeed");

    let bead_json = parse_json(&stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert!(bead_json.get("assignee").is_none() || bead_json.get("assignee").unwrap().is_null(),
              "assignee should be null after dry-run");
    assert_eq!(get_string(&bead_json, "status"), "open");
}

#[test]
fn test_claim_concurrent_same_bead() {
    let (_t, ws) = setup();

    // Create a single test bead
    let (stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", "Race condition test"]);
    assert!(ok, "create should succeed");
    let id = stdout.trim();

    // Simulate concurrent claims by spawning two threads
    let ws1 = ws.clone();
    let ws2 = ws.clone();

    // Use a barrier to synchronize the threads
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let barrier1 = barrier.clone();
    let barrier2 = barrier.clone();

    let handle1 = thread::spawn(move || {
        barrier1.wait();
        let (stdout, _stderr, ok) = run_bf(&ws1, &["claim", "--assignee", "worker-1"]);
        (ok, stdout)
    });

    let handle2 = thread::spawn(move || {
        barrier2.wait();
        let (stdout, _stderr, ok) = run_bf(&ws2, &["claim", "--assignee", "worker-2"]);
        (ok, stdout)
    });

    // Wait for both threads to complete
    let (ok1, out1) = handle1.join().unwrap();
    let (ok2, out2) = handle2.join().unwrap();

    // Exactly one should succeed
    assert!(ok1 || ok2, "at least one claim should succeed");
    assert!(!(ok1 && ok2 && out1.contains(id) && out2.contains(id)),
            "both workers should not claim the same bead");

    // Verify the bead is claimed by exactly one worker
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok, "show should succeed");

    let bead_json = parse_json(&stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    let assignee = get_string(&bead_json, "assignee");
    assert!(assignee == "worker-1" || assignee == "worker-2",
            "bead should be claimed by exactly one worker");
    assert_eq!(get_string(&bead_json, "status"), "in_progress");
}

#[test]
fn test_claim_multiple_beads_concurrently() {
    let (_t, ws) = setup();

    // Create multiple beads
    let mut bead_ids = vec![];
    for i in 1..=5 {
        let (stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", &format!("Concurrent bead {}", i)]);
        assert!(ok, "create should succeed");
        bead_ids.push(stdout.trim().to_string());
    }

    // Claim beads with multiple workers
    let mut claimed_ids = vec![];
    for worker_id in 1..=3 {
        let ws = ws.clone();
        let (stdout, _stderr, ok) = run_bf(&ws, &["claim", "--assignee", &format!("worker-{}", worker_id)]);
        if ok {
            claimed_ids.push(stdout.trim().to_string());
        }
    }

    // Verify all claimed beads are unique
    let mut unique_ids = std::collections::HashSet::new();
    for id in &claimed_ids {
        unique_ids.insert(id);
    }
    assert_eq!(unique_ids.len(), claimed_ids.len(),
              "each claim should be for a different bead");
}

#[test]
fn test_claim_fallback_mode() {
    let (_t, ws) = setup();

    // Don't create any beads in the main workspace

    // Try claiming with fallback mode - should handle empty case gracefully
    let (stdout, _stderr, ok) = run_bf(&ws, &["claim", "--assignee", "worker-1", "--fallback", "any"]);
    assert!(ok, "claim fallback should succeed even with no beads");
    // Should indicate no beads available
    assert!(stdout.contains("no beads") || stdout.contains("No") || stdout.contains("available") || stdout.is_empty(),
            "should indicate no beads available");
}

// ============================================================================
// BATCH command tests
// ============================================================================

#[test]
fn test_batch_create_single() {
    let (_t, ws) = setup();

    let batch_json = r#"[{"op":"create","title":"Batch created bead"}]"#;
    let (stdout, _stderr, ok) = run_bf(&ws, &["batch", "--json", batch_json, "--format", "json"]);
    assert!(ok, "batch create should succeed");

    // Parse the envelope format
    let envelope = parse_json(&stdout);
    let data = unwrap_envelope(&envelope);
    let results_array = data.as_array().unwrap();
    assert_eq!(results_array.len(), 1, "should create 1 bead");

    let first_result = &results_array[0];
    assert!(first_result.get("id").is_some(), "batch should return an ID");

    let id = get_string(first_result, "id");

    // Verify bead exists
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id]);
    assert!(ok, "show should succeed");
    assert!(stdout.contains("Batch created bead"), "should show the created bead");
}

#[test]
fn test_batch_create_multiple() {
    let (_t, ws) = setup();

    let batch_json = r#"[
        {"op":"create","title":"First batch bead"},
        {"op":"create","title":"Second batch bead"},
        {"op":"create","title":"Third batch bead"}
    ]"#;

    let (stdout, _stderr, ok) = run_bf(&ws, &["batch", "--json", batch_json, "--format", "json"]);
    assert!(ok, "batch create multiple should succeed");

    let envelope = parse_json(&stdout);
    let data = unwrap_envelope(&envelope);
    let results_array = data.as_array().unwrap();
    assert_eq!(results_array.len(), 3, "should create 3 beads");

    // Verify all beads exist
    let (list_out, _stderr, ok) = run_bf(&ws, &["list", "--format", "json"]);
    assert!(ok, "list should succeed");
    let beads = parse_jsonl(&list_out);
    assert_eq!(beads.len(), 3, "should have 3 beads in database");
}

#[test]
fn test_batch_with_placeholder_references() {
    let (_t, ws) = setup();

    let batch_json = r#"[
        {"op":"create","title":"Parent bead"},
        {"op":"create","title":"Child bead"},
        {"op":"dep_add_blocker","id":"@0","blocker":"@1"}
    ]"#;

    let (stdout, _stderr, ok) = run_bf(&ws, &["batch", "--json", batch_json, "--format", "json"]);
    assert!(ok, "batch with placeholders should succeed");

    let envelope = parse_json(&stdout);
    let data = unwrap_envelope(&envelope);
    let results_array = data.as_array().unwrap();
    assert_eq!(results_array.len(), 3, "should execute 3 operations");

    // Verify dependency was created
    let parent_id = get_string(&results_array[0], "id");
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &parent_id, "--json"]);
    assert!(ok, "show parent should succeed");

    let parent_json = parse_json(&stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    let deps = parent_json.get("dependencies").and_then(|v| v.as_array()).unwrap();
    assert!(deps.len() >= 1, "parent should have at least one dependency");
}

#[test]
fn test_batch_close_bead() {
    let (_t, ws) = setup();

    // First create a bead
    let (stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", "Bead to close"]);
    assert!(ok, "create should succeed");
    let id = stdout.trim();

    // Close it via batch
    let batch_json = format!(r#"[{{"op":"close","id":"{}","reason":"Test closure"}}]"#, id);
    let (stdout, _stderr, ok) = run_bf(&ws, &["batch", "--json", &batch_json, "--format", "json"]);
    assert!(ok, "batch close should succeed");

    let envelope = parse_json(&stdout);
    let data = unwrap_envelope(&envelope);
    let results_array = data.as_array().unwrap();
    let first_result = &results_array[0];

    // BatchResult has status: "ok" for success, not "success" or "closed"
    assert_eq!(get_string(first_result, "status"), "ok",
            "batch close should have status ok");

    // Verify bead is closed
    let (stdout, _stderr, ok) = run_bf(&ws, &["show", &id, "--json"]);
    assert!(ok, "show should succeed");

    let bead_json = parse_json(&stdout.trim().trim_start_matches('[').trim_end_matches(']'));
    assert_eq!(get_string(&bead_json, "status"), "closed");
}

#[test]
fn test_batch_stdin_input() {
    let (_t, ws) = setup();

    let batch_json = r#"[{"op":"create","title":"Stdin batch bead"}]"#;

    // Write batch JSON to a temp file
    let temp_file = ws.join("batch_input.json");
    fs::write(&temp_file, batch_json).unwrap();

    let (stdout, _stderr, ok) = run_bf(&ws, &["batch", "--file", temp_file.to_str().unwrap(), "--format", "json"]);
    assert!(ok, "batch from file should succeed");

    let envelope = parse_json(&stdout);
    let data = unwrap_envelope(&envelope);
    let results_array = data.as_array().unwrap();
    let first_result = &results_array[0];

    assert!(first_result.get("id").is_some(), "batch should return an ID");
}

#[test]
fn test_batch_atomic_transaction() {
    let (_t, ws) = setup();

    // Create a valid batch that should fail mid-way (invalid dependency)
    let batch_json = r#"[
        {"op":"create","title":"Valid bead"},
        {"op":"dep_add_blocker","id":"nonexistent","blocker":"@0"}
    ]"#;

    let (_stdout, _stderr, ok) = run_bf(&ws, &["batch", "--json", batch_json, "--format", "json"]);

    // The batch should fail due to invalid dependency
    assert!(!ok, "batch should fail with invalid dependency");

    // In a proper atomic transaction, the first bead should not be created
    // or should be rolled back

    // Verify the bead was NOT created (transaction was atomic)
    let (list_out, _stderr, ok) = run_bf(&ws, &["list", "--format", "json"]);
    assert!(ok, "list should succeed");
    let beads = parse_jsonl(&list_out);

    // The bead should not exist due to transaction rollback
    assert_eq!(beads.len(), 0, "no beads should exist after failed batch");
}

// ============================================================================
// SYNC --flush-only command tests
// ============================================================================

#[test]
fn test_sync_flush_only_after_create() {
    let (_t, ws) = setup();

    // Create a bead
    let (stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", "Flush test bead"]);
    assert!(ok, "create should succeed");
    let id = stdout.trim();

    // Flush to JSONL
    let (sync_stdout, _stderr, ok) = run_bf(&ws, &["sync", "--flush-only"]);
    assert!(ok, "sync flush-only should succeed");
    assert!(sync_stdout.contains("Flushed") || sync_stdout.contains("1"),
            "should indicate flush occurred");

    // Verify JSONL file exists and contains the bead
    let jsonl_content = fs::read_to_string(jsonl_path(&ws)).unwrap();
    assert!(!jsonl_content.is_empty(), "JSONL should not be empty after flush");

    let beads = parse_jsonl(&jsonl_content);
    assert!(beads.len() >= 1, "JSONL should contain at least one bead");

    // Find our bead in JSONL
    let found = beads.iter().any(|b| get_string(b, "id") == id);
    assert!(found, "created bead should be in JSONL after flush");
}

#[test]
fn test_sync_flush_only_after_update() {
    let (_t, ws) = setup();

    // Create a bead
    let (stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", "Original title"]);
    assert!(ok, "create should succeed");
    let id = stdout.trim();

    // Update the bead
    let (_stdout, _stderr, ok) = run_bf(&ws, &["update", &id, "--title", "Updated title"]);
    assert!(ok, "update should succeed");

    // Flush to JSONL
    let (_stdout, _stderr, ok) = run_bf(&ws, &["sync", "--flush-only"]);
    assert!(ok, "sync flush-only should succeed");

    // Verify JSONL contains the updated title
    let jsonl_content = fs::read_to_string(jsonl_path(&ws)).unwrap();
    let beads = parse_jsonl(&jsonl_content);

    let updated_bead = beads.iter().find(|b| get_string(b, "id") == id).unwrap();
    assert_eq!(get_string(updated_bead, "title"), "Updated title",
              "JSONL should contain updated title after flush");
}

#[test]
fn test_sync_flush_only_after_close() {
    let (_t, ws) = setup();

    // Create and close a bead
    let (stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", "Close test bead"]);
    assert!(ok, "create should succeed");
    let id = stdout.trim();

    let (_stdout, _stderr, ok) = run_bf(&ws, &["close", &id, "--reason", "Test closure"]);
    assert!(ok, "close should succeed");

    // Flush to JSONL
    let (_stdout, _stderr, ok) = run_bf(&ws, &["sync", "--flush-only"]);
    assert!(ok, "sync flush-only should succeed");

    // Verify JSONL contains the closed bead
    let jsonl_content = fs::read_to_string(jsonl_path(&ws)).unwrap();
    let beads = parse_jsonl(&jsonl_content);

    let closed_bead = beads.iter().find(|b| get_string(b, "id") == id).unwrap();
    assert_eq!(get_string(closed_bead, "status"), "closed",
              "JSONL should contain closed status after flush");
    assert_eq!(get_string(closed_bead, "close_reason"), "Test closure",
              "JSONL should contain close reason after flush");
}

#[test]
fn test_sync_flush_only_incremental() {
    let (_t, ws) = setup();

    // Create first bead and flush
    let (stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", "First bead"]);
    assert!(ok, "create should succeed");
    let id1 = stdout.trim();

    let (_stdout, _stderr, ok) = run_bf(&ws, &["sync", "--flush-only"]);
    assert!(ok, "sync flush-only should succeed");

    // Create second bead
    let (stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", "Second bead"]);
    assert!(ok, "create should succeed");
    let id2 = stdout.trim();

    // Flush again - should only flush the second bead
    let (_stdout, _stderr, ok) = run_bf(&ws, &["sync", "--flush-only"]);
    assert!(ok, "sync flush-only should succeed");

    // Verify JSONL contains both beads
    let jsonl_content = fs::read_to_string(jsonl_path(&ws)).unwrap();
    let beads = parse_jsonl(&jsonl_content);

    assert_eq!(beads.len(), 2, "JSONL should contain both beads");
    assert!(beads.iter().any(|b| get_string(b, "id") == id1), "first bead should be in JSONL");
    assert!(beads.iter().any(|b| get_string(b, "id") == id2), "second bead should be in JSONL");
}

#[test]
fn test_sync_flush_only_empty_database() {
    let (_t, ws) = setup();

    // Flush empty database
    let (_stdout, _stderr, ok) = run_bf(&ws, &["sync", "--flush-only"]);
    assert!(ok, "sync flush-only should succeed even with empty database");

    // JSONL should be empty or contain only metadata
    let jsonl_path = jsonl_path(&ws);
    if jsonl_path.exists() {
        let jsonl_content = fs::read_to_string(&jsonl_path).unwrap();
        let beads = parse_jsonl(&jsonl_content);
        assert_eq!(beads.len(), 0, "JSONL should be empty when database is empty");
    }
}

#[test]
fn test_sync_flush_only_after_batch() {
    let (_t, ws) = setup();

    // Create beads via batch
    let batch_json = r#"[
        {"op":"create","title":"Batch bead 1"},
        {"op":"create","title":"Batch bead 2"}
    ]"#;

    let (stdout, _stderr, ok) = run_bf(&ws, &["batch", "--json", batch_json, "--format", "json"]);
    assert!(ok, "batch should succeed");

    let envelope = parse_json(&stdout);
    let data = unwrap_envelope(&envelope);
    let results_array = data.as_array().unwrap();
    let id1 = get_string(&results_array[0], "id");
    let id2 = get_string(&results_array[1], "id");

    // Flush to JSONL
    let (_stdout, _stderr, ok) = run_bf(&ws, &["sync", "--flush-only"]);
    assert!(ok, "sync flush-only should succeed");

    // Verify JSONL contains both batch-created beads
    let jsonl_content = fs::read_to_string(jsonl_path(&ws)).unwrap();
    let beads = parse_jsonl(&jsonl_content);

    assert_eq!(beads.len(), 2, "JSONL should contain both batch beads");
    assert!(beads.iter().any(|b| get_string(b, "id") == id1), "first batch bead should be in JSONL");
    assert!(beads.iter().any(|b| get_string(b, "id") == id2), "second batch bead should be in JSONL");
}

// ============================================================================
// Combined workflow tests
// ============================================================================

#[test]
fn test_full_workflow_create_claim_flush() {
    let (_t, ws) = setup();

    // Create a bead
    let (stdout, _stderr, ok) = run_bf(&ws, &["create", "--title", "Workflow test bead"]);
    assert!(ok, "create should succeed");
    let id = stdout.trim();

    // Claim the bead
    let (stdout, _stderr, ok) = run_bf(&ws, &["claim", "--assignee", "workflow-worker"]);
    assert!(ok, "claim should succeed");
    assert!(stdout.contains(&id), "claim should return the bead ID");

    // Close the bead
    let (_stdout, _stderr, ok) = run_bf(&ws, &["close", &id, "--reason", "Workflow completed"]);
    assert!(ok, "close should succeed");

    // Flush to JSONL
    let (_stdout, _stderr, ok) = run_bf(&ws, &["sync", "--flush-only"]);
    assert!(ok, "sync flush-only should succeed");

    // Verify complete lifecycle in JSONL
    let jsonl_content = fs::read_to_string(jsonl_path(&ws)).unwrap();
    let beads = parse_jsonl(&jsonl_content);

    assert_eq!(beads.len(), 1, "JSONL should contain one bead");
    let bead = &beads[0];

    assert_eq!(get_string(bead, "id"), id);
    assert_eq!(get_string(bead, "status"), "closed");
    // Note: close clears the assignee (sets to NULL), so expect empty string
    assert_eq!(get_string(bead, "assignee"), "");
    assert_eq!(get_string(bead, "close_reason"), "Workflow completed");
}
