//! Integration tests for stats command envelope wrapping.
//!
//! Tests verify that the stats command's JSON output is properly wrapped
//! in an envelope with correct structure and metadata fields.
//!
//! Test command: cargo test envelope::claim_stats::stats

use std::process::Command;
use tempfile::TempDir;

/// Resolve the freshly-built bf binary.
fn bf_path() -> String {
    std::env::var("CARGO_BIN_EXE_bf")
        .unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Create an isolated workspace via `bf init`.
fn init_workspace() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let out = Command::new(bf_path())
        .args(["init", "--prefix", "test"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to init workspace");
    assert!(
        out.status.success(),
        "bf init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    temp_dir
}

/// Create a task bead via the CLI, returning its printed id.
fn create_bead(workspace: &std::path::Path, title: &str) -> String {
    let out = Command::new(bf_path())
        .args(["create", "--title", title, "--type", "task", "--priority", "2"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(
        out.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Run a command with --envelope flag and return parsed envelope.
fn run_envelope_command(workspace: &std::path::Path, args: &[&str]) -> serde_json::Value {
    let mut full_args = vec!["--envelope"];
    full_args.extend_from_slice(args);

    let out = Command::new(bf_path())
        .args(&full_args)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf command");

    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();

    if !out.status.success() {
        panic!("Command failed: {:?}\nstdout: {}\nstderr: {}", full_args, stdout, stderr);
    }

    serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON output from {:?}: {}\nOutput: {}\nstderr: {}", full_args, e, stdout, stderr))
}

/// Verify envelope structure is stable and valid.
fn verify_envelope_structure(envelope: &serde_json::Value, expected_kind: &str) {
    // Verify envelope is an object
    assert!(envelope.is_object(), "Envelope must be an object");

    // Verify version field is present and equals 1
    assert_eq!(
        envelope.get("version").and_then(|v| v.as_u64()),
        Some(1),
        "Envelope version must be 1"
    );

    // Verify kind field matches expected
    assert_eq!(
        envelope.get("kind").and_then(|k| k.as_str()),
        Some(expected_kind),
        "Envelope kind must be '{}'",
        expected_kind
    );

    // Verify data field is present
    assert!(
        envelope.get("data").is_some(),
        "Envelope must have a 'data' field"
    );
}

/// Test: stats --json --envelope has stable envelope structure
#[test]
fn stats_envelope_has_stable_structure() {
    let ws = init_workspace();
    create_bead(ws.path(), "stats test");

    let envelope = run_envelope_command(ws.path(), &["stats", "--format", "json"]);

    // Verify envelope structure
    verify_envelope_structure(&envelope, "stats");

    // Verify data contains stats
    let data = &envelope["data"];
    assert!(data.is_object(), "stats data must be an object");
    assert!(data.get("total").is_some(), "stats must have 'total' field");
}

/// Test: stats --json --envelope has correct metadata fields
#[test]
fn stats_envelope_metadata_fields() {
    let ws = init_workspace();
    create_bead(ws.path(), "metadata test");

    let envelope = run_envelope_command(ws.path(), &["stats", "--format", "json"]);

    // Verify version field
    assert_eq!(
        envelope.get("version").and_then(|v| v.as_u64()),
        Some(1),
        "stats envelope version must be 1"
    );

    // Verify kind field is 'stats'
    assert_eq!(
        envelope.get("kind").and_then(|k| k.as_str()),
        Some("stats"),
        "stats envelope kind must be 'stats'"
    );

    // Verify data field is present
    assert!(
        envelope.get("data").is_some(),
        "stats envelope must have 'data' field"
    );

    // Verify data is an object
    let data = &envelope["data"];
    assert!(data.is_object(), "stats data must be an object");
}

/// Test: stats --json --envelope successful case returns valid stats
#[test]
fn stats_envelope_successful_case() {
    let ws = init_workspace();

    // Create multiple beads for meaningful stats
    create_bead(ws.path(), "first bead");
    create_bead(ws.path(), "second bead");
    create_bead(ws.path(), "third bead");

    let envelope = run_envelope_command(ws.path(), &["stats", "--format", "json"]);

    // Verify envelope structure
    verify_envelope_structure(&envelope, "stats");

    // Verify stats data is accurate
    let data = &envelope["data"];
    assert!(data.is_object(), "stats data must be an object");

    // Check total count
    let total = data.get("total").and_then(|t| t.as_u64());
    assert_eq!(total, Some(3), "stats total must reflect bead count");

    // Verify other stats fields are present
    // Common stats fields include: total, by_status, by_priority, etc.
    assert!(data.get("total").is_some(), "stats must have 'total'");
}

/// Test: stats --json --envelope with empty workspace
#[test]
fn stats_envelope_empty_workspace() {
    let ws = init_workspace();

    let envelope = run_envelope_command(ws.path(), &["stats", "--format", "json"]);

    // Verify envelope structure even when empty
    verify_envelope_structure(&envelope, "stats");

    // Verify total is 0
    let data = &envelope["data"];
    assert_eq!(
        data.get("total").and_then(|t| t.as_u64()),
        Some(0),
        "stats total must be 0 for empty workspace"
    );
}

/// Test: stats --json --envelope data contains expected fields
#[test]
fn stats_envelope_data_fields() {
    let ws = init_workspace();
    create_bead(ws.path(), "fields test");

    let envelope = run_envelope_command(ws.path(), &["stats", "--format", "json"]);

    let data = &envelope["data"];

    // Verify total field is numeric
    if let Some(total) = data.get("total") {
        assert!(total.is_number(), "stats 'total' must be numeric");
    }

    // Verify data is an object with at least total field
    assert!(data.is_object(), "stats data must be an object");
    assert!(data.as_object().unwrap().contains_key("total"), "stats data must contain 'total'");
}

/// Test: stats --json --envelope kind field matches command
#[test]
fn stats_envelope_kind_matches_command() {
    let ws = init_workspace();
    create_bead(ws.path(), "kind test");

    let envelope = run_envelope_command(ws.path(), &["stats", "--format", "json"]);

    // Verify kind field is exactly 'stats'
    assert_eq!(
        envelope.get("kind").and_then(|k| k.as_str()),
        Some("stats"),
        "stats envelope kind must be 'stats'"
    );
}

/// Test: stats --json --envelope version is always 1
#[test]
fn stats_envelope_version_always_one() {
    let ws = init_workspace();
    create_bead(ws.path(), "version test");

    let envelope = run_envelope_command(ws.path(), &["stats", "--format", "json"]);

    // Verify version is exactly 1
    assert_eq!(
        envelope.get("version").and_then(|v| v.as_u64()),
        Some(1),
        "stats envelope version must always be 1"
    );
}
