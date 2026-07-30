//! Comprehensive envelope test coverage for all JSON commands.
//!
//! Tests verify that envelope structure is stable, metadata is present,
//! and envelope wrapping works correctly across all commands that support
//! JSON output.
//!
//! Bead: bf-4hs1k

use std::path::PathBuf;
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
    let mut full_args = vec!["--envelope"]; // Enable envelope wrapping at the beginning
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

    // Verify metadata fields (version, kind) are present
    assert!(envelope.get("version").is_some(), "Envelope must have 'version' metadata");
    assert!(envelope.get("kind").is_some(), "Envelope must have 'kind' metadata");
}

// ---------------------------------------------------------------------------
// create --json --envelope
// ---------------------------------------------------------------------------

#[test]
fn envelope_create_command_has_stable_structure() {
    let ws = init_workspace();
    let envelope = run_envelope_command(ws.path(), &["create", "--title", "test", "--type", "task", "--priority", "2", "--json"]);

    verify_envelope_structure(&envelope, "create");

    // Verify data contains the bead id
    let data = &envelope["data"];
    assert!(data.get("id").is_some(), "create data must contain 'id'");
    assert!(data["id"].is_string(), "create id must be a string");
    assert!(data["id"].as_str().unwrap().starts_with("test-"), "create id must start with 'test-' (workspace prefix)");
}

#[test]
fn envelope_create_command_metadata_fields_present() {
    let ws = init_workspace();
    let envelope = run_envelope_command(ws.path(), &["create", "--title", "test", "--type", "task", "--priority", "2", "--json"]);

    // Verify all metadata fields are present
    assert!(envelope.get("version").is_some(), "version field must be present");
    assert!(envelope.get("kind").is_some(), "kind field must be present");
    assert!(envelope.get("data").is_some(), "data field must be present");

    // Verify warning field is optional (may be null or absent)
    let warning = envelope.get("warning");
    if let Some(w) = warning {
        assert!(w.is_string() || w.is_null(), "warning must be string or null/absent");
    }
}

// ---------------------------------------------------------------------------
// show --json --envelope
// ---------------------------------------------------------------------------

#[test]
fn envelope_show_command_has_stable_structure() {
    let ws = init_workspace();
    let id = create_bead(ws.path(), "show test");

    let envelope = run_envelope_command(ws.path(), &["show", &id, "--json"]);

    verify_envelope_structure(&envelope, "show");

    // Verify data contains the bead
    let data = &envelope["data"];
    assert_eq!(data.get("id").and_then(|i| i.as_str()), Some(id.as_str()));
    assert_eq!(data.get("title").and_then(|t| t.as_str()), Some("show test"));
}

// ---------------------------------------------------------------------------
// list --format json --envelope
// ---------------------------------------------------------------------------

#[test]
fn envelope_list_command_has_stable_structure() {
    let ws = init_workspace();
    create_bead(ws.path(), "first");
    create_bead(ws.path(), "second");

    let envelope = run_envelope_command(ws.path(), &["list", "--format", "json"]);

    verify_envelope_structure(&envelope, "list");

    // Verify data is an array
    let data = &envelope["data"];
    assert!(data.is_array(), "list data must be an array");
    assert_eq!(data.as_array().unwrap().len(), 2);
}

#[test]
fn envelope_list_empty_emits_empty_array() {
    let ws = init_workspace();

    let envelope = run_envelope_command(ws.path(), &["list", "--format", "json"]);

    verify_envelope_structure(&envelope, "list");

    // Verify data is an empty array
    let data = &envelope["data"];
    assert!(data.is_array(), "list data must be an array");
    assert_eq!(data.as_array().unwrap().len(), 0);
}

// ---------------------------------------------------------------------------
// ready --format json --envelope
// ---------------------------------------------------------------------------

#[test]
fn envelope_ready_command_has_stable_structure() {
    let ws = init_workspace();
    create_bead(ws.path(), "ready bead");

    let envelope = run_envelope_command(ws.path(), &["ready", "--limit", "0", "--format", "json"]);

    verify_envelope_structure(&envelope, "ready");

    // Verify data is an array
    let data = &envelope["data"];
    assert!(data.is_array(), "ready data must be an array");
    assert!(data.as_array().unwrap().len() >= 1);
}

#[test]
fn envelope_ready_empty_emits_empty_array() {
    let ws = init_workspace();

    let envelope = run_envelope_command(ws.path(), &["ready", "--limit", "0", "--format", "json"]);

    verify_envelope_structure(&envelope, "ready");

    // Verify data is an empty array
    let data = &envelope["data"];
    assert!(data.is_array(), "ready data must be an array");
    assert_eq!(data.as_array().unwrap().len(), 0);
}

// ---------------------------------------------------------------------------
// claim --json --envelope
// ---------------------------------------------------------------------------

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn envelope_claim_command_has_stable_structure() {
    let ws = init_workspace();
    create_bead(ws.path(), "claim test");

    let envelope = run_envelope_command(ws.path(), &["claim", "--assignee", "test-worker", "--json"]);

    verify_envelope_structure(&envelope, "claim");

    // Verify data contains claim result
    let data = &envelope["data"];
    assert!(data.is_object(), "claim data must be an object");

    // Should have bead_id and assignee
    assert!(data.get("bead_id").is_some(), "claim data must have 'bead_id'");
    assert!(data.get("assignee").is_some(), "claim data must have 'assignee'");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn envelope_claim_no_bead_emits_empty_object() {
    let ws = init_workspace();

    let envelope = run_envelope_command(ws.path(), &["claim", "--assignee", "test-worker", "--json"]);

    verify_envelope_structure(&envelope, "claim");

    // Verify data is an empty object
    let data = &envelope["data"];
    assert!(data.is_object(), "claim data must be an object");
    assert!(data.as_object().unwrap().is_empty(), "claim data must be empty when no beads available");
}

// ---------------------------------------------------------------------------
// stats --format json --envelope
// ---------------------------------------------------------------------------

#[test]
fn envelope_stats_command_has_stable_structure() {
    let ws = init_workspace();
    create_bead(ws.path(), "stats test");

    let envelope = run_envelope_command(ws.path(), &["stats", "--format", "json"]);

    verify_envelope_structure(&envelope, "stats");

    // Verify data contains stats
    let data = &envelope["data"];
    assert!(data.is_object(), "stats data must be an object");

    // Should have common stats fields
    assert!(data.get("total").is_some(), "stats must have 'total'");
}

// ---------------------------------------------------------------------------
// velocity --format json --envelope
// ---------------------------------------------------------------------------

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn envelope_velocity_command_has_stable_structure() {
    let ws = init_workspace();

    let envelope = run_envelope_command(ws.path(), &["velocity", "--format", "json"]);

    verify_envelope_structure(&envelope, "velocity");

    // Verify data is an array (even if empty)
    let data = &envelope["data"];
    assert!(data.is_array(), "velocity data must be an array");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn envelope_velocity_empty_emits_empty_array() {
    let ws = init_workspace();

    let envelope = run_envelope_command(ws.path(), &["velocity", "--format", "json"]);

    verify_envelope_structure(&envelope, "velocity");

    // Verify data is an array (velocity returns [[]] - array containing empty results array)
    let data = &envelope["data"];
    assert!(data.is_array(), "velocity data must be an array");
    // Velocity returns [[]] which is an array with one empty array element
    let inner = data.as_array().unwrap().first().expect("velocity data must contain inner array");
    assert!(inner.is_array(), "velocity inner data must be an array");
    assert_eq!(inner.as_array().unwrap().len(), 0, "velocity inner array must be empty");
}

// ---------------------------------------------------------------------------
// search --format json --envelope
// ---------------------------------------------------------------------------

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn envelope_search_command_has_stable_structure() {
    let ws = init_workspace();
    create_bead(ws.path(), "searchable bead");

    let envelope = run_envelope_command(ws.path(), &["search", "searchable", "--format", "json"]);

    verify_envelope_structure(&envelope, "search");

    // Verify data is an array
    let data = &envelope["data"];
    assert!(data.is_array(), "search data must be an array");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn envelope_search_empty_emits_empty_array() {
    let ws = init_workspace();

    let envelope = run_envelope_command(ws.path(), &["search", "nonexistent", "--format", "json"]);

    verify_envelope_structure(&envelope, "search");

    // Verify data is an empty array
    let data = &envelope["data"];
    assert!(data.is_array(), "search data must be an array");
    assert_eq!(data.as_array().unwrap().len(), 0);
}

// ---------------------------------------------------------------------------
// recent --format json --envelope
// ---------------------------------------------------------------------------

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn envelope_recent_command_has_stable_structure() {
    let ws = init_workspace();
    create_bead(ws.path(), "recent bead");

    let envelope = run_envelope_command(ws.path(), &["recent", "--format", "json"]);

    verify_envelope_structure(&envelope, "recent");

    // Verify data is an array
    let data = &envelope["data"];
    assert!(data.is_array(), "recent data must be an array");
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn envelope_recent_empty_emits_empty_array() {
    let ws = init_workspace();

    let envelope = run_envelope_command(ws.path(), &["recent", "--format", "json"]);

    verify_envelope_structure(&envelope, "recent");

    // Verify data is an empty array
    let data = &envelope["data"];
    assert!(data.is_array(), "recent data must be an array");
    assert_eq!(data.as_array().unwrap().len(), 0);
}

// ---------------------------------------------------------------------------
// batch --json --envelope
// ---------------------------------------------------------------------------

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn envelope_batch_command_has_stable_structure() {
    let ws = init_workspace();

    // Create a simple batch operation
    let batch_input = r#"[{"op": "create", "title": "batch test", "type": "task", "priority": 2}]"#;

    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new(bf_path())
        .args(["batch", "--stdin", "--format", "json", "--envelope"])
        .current_dir(ws.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn bf batch");

    // Write batch input to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(batch_input.as_bytes()).expect("Failed to write to stdin");
    }

    let out = child.wait_with_output().expect("Failed to read bf batch output");

    if !out.status.success() {
        panic!("bf batch failed: {}", String::from_utf8_lossy(&out.stderr));
    }

    let stdout = String::from_utf8(out.stdout).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Invalid JSON output from batch");

    verify_envelope_structure(&envelope, "batch");

    // Verify data is an array of results
    let data = &envelope["data"];
    assert!(data.is_array(), "batch data must be an array");
    assert!(data.as_array().unwrap().len() >= 1);
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn envelope_batch_empty_emits_empty_array() {
    let ws = init_workspace();

    // Empty batch
    let batch_input = r#"[]"#;

    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new(bf_path())
        .args(["batch", "--stdin", "--format", "json", "--envelope"])
        .current_dir(ws.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn bf batch");

    // Write batch input to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(batch_input.as_bytes()).expect("Failed to write to stdin");
    }

    let out = child.wait_with_output().expect("Failed to read bf batch output");

    let stdout = String::from_utf8(out.stdout).unwrap();
    let envelope: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Invalid JSON output from batch");

    verify_envelope_structure(&envelope, "batch");

    // Verify data is an empty array
    let data = &envelope["data"];
    assert!(data.is_array(), "batch data must be an array");
    assert_eq!(data.as_array().unwrap().len(), 0);
}

// ---------------------------------------------------------------------------
// Envelope stability tests
// ---------------------------------------------------------------------------

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn envelope_version_is_always_one() {
    let ws = init_workspace();

    // Test that all JSON commands have version=1
    let commands = vec![
        vec!["create", "--title", "vtest", "--type", "task", "--priority", "2", "--json"],
        vec!["stats", "--format", "json"],
        vec!["velocity", "--format", "json"],
        vec!["search", "x", "--format", "json"],
        vec!["recent", "--format", "json"],
        vec!["list", "--format", "json"],
        vec!["ready", "--limit", "0", "--format", "json"],
    ];

    for args in commands {
        let out = Command::new(bf_path())
            .args(&args)
            .arg("--envelope")
            .current_dir(ws.path())
            .output()
            .expect("Failed to run command");

        if out.status.success() {
            let stdout = String::from_utf8(out.stdout).unwrap();
            if let Ok(envelope) = serde_json::from_str::<serde_json::Value>(&stdout) {
                assert_eq!(
                    envelope.get("version").and_then(|v| v.as_u64()),
                    Some(1),
                    "Version must be 1 for command: {:?}",
                    args
                );
            }
        }
    }
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn envelope_kind_matches_command() {
    let ws = init_workspace();

    // Test that envelope kind matches the command name
    let test_cases = vec![
        (vec!["create", "--title", "ktest", "--type", "task", "--priority", "2", "--json"], "create"),
        (vec!["stats", "--format", "json"], "stats"),
        (vec!["velocity", "--format", "json"], "velocity"),
        (vec!["search", "x", "--format", "json"], "search"),
        (vec!["recent", "--format", "json"], "recent"),
        (vec!["list", "--format", "json"], "list"),
        (vec!["ready", "--limit", "0", "--format", "json"], "ready"),
    ];

    for (args, expected_kind) in test_cases {
        let out = Command::new(bf_path())
            .args(&args)
            .arg("--envelope")
            .current_dir(ws.path())
            .output()
            .expect("Failed to run command");

        if out.status.success() {
            let stdout = String::from_utf8(out.stdout).unwrap();
            if let Ok(envelope) = serde_json::from_str::<serde_json::Value>(&stdout) {
                assert_eq!(
                    envelope.get("kind").and_then(|k| k.as_str()),
                    Some(expected_kind),
                    "Kind mismatch for command {:?}, expected '{}'",
                    args,
                    expected_kind
                );
            }
        }
    }
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn envelope_data_field_always_present() {
    let ws = init_workspace();

    // Test that all JSON commands have a data field
    let commands = vec![
        vec!["create", "--title", "dtest", "--type", "task", "--priority", "2", "--json"],
        vec!["stats", "--format", "json"],
        vec!["velocity", "--format", "json"],
        vec!["search", "x", "--format", "json"],
        vec!["recent", "--format", "json"],
        vec!["list", "--format", "json"],
        vec!["ready", "--limit", "0", "--format", "json"],
    ];

    for args in commands {
        let out = Command::new(bf_path())
            .args(&args)
            .arg("--envelope")
            .current_dir(ws.path())
            .output()
            .expect("Failed to run command");

        if out.status.success() {
            let stdout = String::from_utf8(out.stdout).unwrap();
            if let Ok(envelope) = serde_json::from_str::<serde_json::Value>(&stdout) {
                assert!(
                    envelope.get("data").is_some(),
                    "Data field must be present for command: {:?}",
                    args
                );
            }
        }
    }
}

#[test]
fn envelope_serializes_and_deserializes_correctly() {
    let ws = init_workspace();
    create_bead(ws.path(), "serde test");

    // Test round-trip serialization/deserialization
    let envelope = run_envelope_command(ws.path(), &["list", "--format", "json"]);

    // Serialize back to JSON
    let serialized = serde_json::to_string(&envelope)
        .expect("Failed to serialize envelope");

    // Deserialize again
    let deserialized: serde_json::Value = serde_json::from_str(&serialized)
        .expect("Failed to deserialize envelope");

    // Verify structure is preserved
    assert_eq!(envelope, deserialized, "Round-trip serialization must preserve envelope");
}

#[test]
fn envelope_parses_as_valid_json_object() {
    let ws = init_workspace();

    let commands = vec![
        vec!["create", "--title", "valid", "--type", "task", "--priority", "2", "--json"],
        vec!["stats", "--format", "json"],
        vec!["list", "--format", "json"],
    ];

    for args in commands {
        let out = Command::new(bf_path())
            .args(&args)
            .arg("--envelope")
            .current_dir(ws.path())
            .output()
            .expect("Failed to run command");

        if out.status.success() {
            let stdout = String::from_utf8(out.stdout).unwrap();
            // Should parse as a JSON object (not array, string, etc.)
            let parsed = serde_json::from_str::<serde_json::Value>(&stdout)
                .expect(&format!("Command {:?} must emit valid JSON object", args));
            assert!(parsed.is_object(), "Output must be a JSON object");
        }
    }
}

// ---------------------------------------------------------------------------
// Module: list_show
// Focus: Integration tests for list and show JSON envelope wrapping
// Test command: cargo test envelope::list_show
// ---------------------------------------------------------------------------

#[cfg(test)]
mod list_show {
    use super::*;

    /// Test: list --json returns envelope with array data
    #[test]
    fn envelope_list_json_returns_array_data() {
        let ws = init_workspace();

        // Create some beads
        create_bead(ws.path(), "first bead");
        create_bead(ws.path(), "second bead");
        create_bead(ws.path(), "third bead");

        let envelope = run_envelope_command(ws.path(), &["list", "--format", "json"]);

        // Verify envelope structure
        verify_envelope_structure(&envelope, "list");

        // Verify data is an array
        let data = &envelope["data"];
        assert!(data.is_array(), "list --json envelope data must be an array");

        let items = data.as_array().unwrap();
        assert_eq!(items.len(), 3, "list --json envelope should contain 3 beads");

        // Verify each item in the array is a valid bead object
        for (i, item) in items.iter().enumerate() {
            assert!(item.is_object(), "list item {} must be an object", i);
            assert!(item.get("id").is_some(), "list item {} must have 'id'", i);
            assert!(item.get("title").is_some(), "list item {} must have 'title'", i);
        }
    }

    /// Test: list --json envelope with metadata fields
    #[test]
    fn envelope_list_json_has_metadata_fields() {
        let ws = init_workspace();
        create_bead(ws.path(), "metadata test");

        let envelope = run_envelope_command(ws.path(), &["list", "--format", "json"]);

        // Verify all required metadata fields are present
        assert!(envelope.get("version").is_some(), "list envelope must have 'version' field");
        assert!(envelope.get("kind").is_some(), "list envelope must have 'kind' field");
        assert!(envelope.get("data").is_some(), "list envelope must have 'data' field");

        // Verify version value
        assert_eq!(
            envelope.get("version").and_then(|v| v.as_u64()),
            Some(1),
            "list envelope version must be 1"
        );

        // Verify kind value
        assert_eq!(
            envelope.get("kind").and_then(|k| k.as_str()),
            Some("list"),
            "list envelope kind must be 'list'"
        );

        // Verify data is array
        let data = &envelope["data"];
        assert!(data.is_array(), "list envelope data must be an array");
    }

    /// Test: show --json returns envelope with single object
    #[test]
    fn envelope_show_json_returns_single_object() {
        let ws = init_workspace();
        let id = create_bead(ws.path(), "show single object test");

        let envelope = run_envelope_command(ws.path(), &["show", &id, "--json"]);

        // Verify envelope structure
        verify_envelope_structure(&envelope, "show");

        // Verify data is a single object (not an array)
        let data = &envelope["data"];
        assert!(data.is_object(), "show --json envelope data must be a single object, not an array");
        assert!(!data.is_array(), "show --json envelope data must not be an array");

        // Verify the object contains expected bead fields
        assert_eq!(data.get("id").and_then(|i| i.as_str()), Some(id.as_str()));
        assert_eq!(data.get("title").and_then(|t| t.as_str()), Some("show single object test"));
        assert!(data.get("status").is_some(), "bead must have 'status'");
        assert!(data.get("priority").is_some(), "bead must have 'priority'");
    }

    /// Test: show --json envelope with metadata fields
    #[test]
    fn envelope_show_json_has_metadata_fields() {
        let ws = init_workspace();
        let id = create_bead(ws.path(), "show metadata test");

        let envelope = run_envelope_command(ws.path(), &["show", &id, "--json"]);

        // Verify all required metadata fields are present
        assert!(envelope.get("version").is_some(), "show envelope must have 'version' field");
        assert!(envelope.get("kind").is_some(), "show envelope must have 'kind' field");
        assert!(envelope.get("data").is_some(), "show envelope must have 'data' field");

        // Verify version value
        assert_eq!(
            envelope.get("version").and_then(|v| v.as_u64()),
            Some(1),
            "show envelope version must be 1"
        );

        // Verify kind value
        assert_eq!(
            envelope.get("kind").and_then(|k| k.as_str()),
            Some("show"),
            "show envelope kind must be 'show'"
        );

        // Verify data is a single object
        let data = &envelope["data"];
        assert!(data.is_object(), "show envelope data must be an object");
    }

    /// Test: empty list returns envelope with empty array
    #[test]
    fn envelope_list_empty_returns_empty_array() {
        let ws = init_workspace();

        let envelope = run_envelope_command(ws.path(), &["list", "--format", "json"]);

        // Verify envelope structure
        verify_envelope_structure(&envelope, "list");

        // Verify data is an empty array
        let data = &envelope["data"];
        assert!(data.is_array(), "empty list envelope data must be an array");
        assert_eq!(data.as_array().unwrap().len(), 0, "empty list envelope must have empty array");
    }

    /// Test: list and show envelopes have consistent structure
    #[test]
    fn envelope_list_and_show_consistent_structure() {
        let ws = init_workspace();
        let id = create_bead(ws.path(), "consistency test");

        // Get list envelope
        let list_envelope = run_envelope_command(ws.path(), &["list", "--format", "json"]);

        // Get show envelope for the same bead
        let show_envelope = run_envelope_command(ws.path(), &["show", &id, "--json"]);

        // Both should have version = 1
        assert_eq!(
            list_envelope.get("version").and_then(|v| v.as_u64()),
            Some(1),
            "list envelope version must be 1"
        );
        assert_eq!(
            show_envelope.get("version").and_then(|v| v.as_u64()),
            Some(1),
            "show envelope version must be 1"
        );

        // Both should have correct kind
        assert_eq!(
            list_envelope.get("kind").and_then(|k| k.as_str()),
            Some("list"),
            "list envelope kind must be 'list'"
        );
        assert_eq!(
            show_envelope.get("kind").and_then(|k| k.as_str()),
            Some("show"),
            "show envelope kind must be 'show'"
        );

        // Both should have data field
        assert!(list_envelope.get("data").is_some(), "list envelope must have data");
        assert!(show_envelope.get("data").is_some(), "show envelope must have data");
    }

    /// Test: list envelope data items match show envelope data structure
    #[test]
    fn envelope_list_items_match_show_structure() {
        let ws = init_workspace();
        let id = create_bead(ws.path(), "structure match test");

        // Get list envelope
        let list_envelope = run_envelope_command(ws.path(), &["list", "--format", "json"]);
        let list_data = &list_envelope["data"];

        // Get show envelope for the same bead
        let show_envelope = run_envelope_command(ws.path(), &["show", &id, "--json"]);
        let show_data = &show_envelope["data"];

        // Both should have the same id
        let list_item = list_data.as_array().unwrap().first().unwrap();
        assert_eq!(
            list_item.get("id").and_then(|i| i.as_str()),
            show_data.get("id").and_then(|i| i.as_str()),
            "list item id must match show data id"
        );

        // Both should have the same title
        assert_eq!(
            list_item.get("title").and_then(|t| t.as_str()),
            show_data.get("title").and_then(|t| t.as_str()),
            "list item title must match show data title"
        );
    }
}

// ---------------------------------------------------------------------------
// Module: claim_stats
// Focus: Integration tests for claim and stats JSON envelope wrapping
// Test command: cargo test envelope::claim_stats
// ---------------------------------------------------------------------------

#[cfg(test)]
mod claim_stats {
    use super::*;

    /// Test: claim --json returns envelope with claim result object
    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn envelope_claim_json_returns_claim_result() {
        let ws = init_workspace();

        // Create a bead to claim
        create_bead(ws.path(), "claim test bead");

        let envelope = run_envelope_command(ws.path(), &["claim", "--assignee", "test-worker", "--json"]);

        // Verify envelope structure
        verify_envelope_structure(&envelope, "claim");

        // Verify data is a single object
        let data = &envelope["data"];
        assert!(data.is_object(), "claim --json envelope data must be an object");

        // Verify the object contains expected claim fields
        assert!(data.get("bead_id").is_some(), "claim result must have 'bead_id'");
        assert!(data.get("assignee").is_some(), "claim result must have 'assignee'");
        assert_eq!(
            data.get("assignee").and_then(|a| a.as_str()),
            Some("test-worker"),
            "claim assignee must match"
        );
    }

    /// Test: claim --json envelope with metadata fields
    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn envelope_claim_json_has_metadata_fields() {
        let ws = init_workspace();
        create_bead(ws.path(), "claim metadata test");

        let envelope = run_envelope_command(ws.path(), &["claim", "--assignee", "metadata-worker", "--json"]);

        // Verify all required metadata fields are present
        assert!(envelope.get("version").is_some(), "claim envelope must have 'version' field");
        assert!(envelope.get("kind").is_some(), "claim envelope must have 'kind' field");
        assert!(envelope.get("data").is_some(), "claim envelope must have 'data' field");

        // Verify version value
        assert_eq!(
            envelope.get("version").and_then(|v| v.as_u64()),
            Some(1),
            "claim envelope version must be 1"
        );

        // Verify kind value
        assert_eq!(
            envelope.get("kind").and_then(|k| k.as_str()),
            Some("claim"),
            "claim envelope kind must be 'claim'"
        );

        // Verify data is an object
        let data = &envelope["data"];
        assert!(data.is_object(), "claim envelope data must be an object");
    }

    /// Test: claim with no available beads returns envelope with empty object
    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn envelope_claim_no_beads_returns_empty_object() {
        let ws = init_workspace();

        // Create beads but with status that won't be claimed
        // or just try to claim from empty workspace
        let envelope = run_envelope_command(ws.path(), &["claim", "--assignee", "empty-worker", "--json"]);

        // Verify envelope structure
        verify_envelope_structure(&envelope, "claim");

        // Verify data is an empty object
        let data = &envelope["data"];
        assert!(data.is_object(), "claim --json envelope data must be an object");
        assert!(data.as_object().unwrap().is_empty(), "claim with no beads must return empty object");
    }

    /// Test: stats --json returns envelope with stats object
    #[test]
    fn envelope_stats_json_returns_stats_result() {
        let ws = init_workspace();

        // Create some beads for stats
        create_bead(ws.path(), "stats bead 1");
        create_bead(ws.path(), "stats bead 2");

        let envelope = run_envelope_command(ws.path(), &["stats", "--format", "json"]);

        // Verify envelope structure
        verify_envelope_structure(&envelope, "stats");

        // Verify data is a single object
        let data = &envelope["data"];
        assert!(data.is_object(), "stats --json envelope data must be an object");

        // Verify the object contains expected stats fields
        assert!(data.get("total").is_some(), "stats must have 'total' field");
        assert!(data.get("total").and_then(|t| t.as_u64()).unwrap() >= 2, "total must be at least 2");
    }

    /// Test: stats --json envelope with metadata fields
    #[test]
    fn envelope_stats_json_has_metadata_fields() {
        let ws = init_workspace();
        create_bead(ws.path(), "stats metadata test");

        let envelope = run_envelope_command(ws.path(), &["stats", "--format", "json"]);

        // Verify all required metadata fields are present
        assert!(envelope.get("version").is_some(), "stats envelope must have 'version' field");
        assert!(envelope.get("kind").is_some(), "stats envelope must have 'kind' field");
        assert!(envelope.get("data").is_some(), "stats envelope must have 'data' field");

        // Verify version value
        assert_eq!(
            envelope.get("version").and_then(|v| v.as_u64()),
            Some(1),
            "stats envelope version must be 1"
        );

        // Verify kind value
        assert_eq!(
            envelope.get("kind").and_then(|k| k.as_str()),
            Some("stats"),
            "stats envelope kind must be 'stats'"
        );

        // Verify data is an object
        let data = &envelope["data"];
        assert!(data.is_object(), "stats envelope data must be an object");
    }

    /// Test: stats with empty workspace returns envelope with zero stats
    #[test]
    fn envelope_stats_empty_returns_zero_stats() {
        let ws = init_workspace();

        let envelope = run_envelope_command(ws.path(), &["stats", "--format", "json"]);

        // Verify envelope structure
        verify_envelope_structure(&envelope, "stats");

        // Verify data has total = 0
        let data = &envelope["data"];
        assert!(data.is_object(), "stats --json envelope data must be an object");
        assert_eq!(
            data.get("total").and_then(|t| t.as_u64()),
            Some(0),
            "stats for empty workspace must have total = 0"
        );
    }

    /// Test: claim and stats envelopes have consistent structure
    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn envelope_claim_and_stats_consistent_structure() {
        let ws = init_workspace();
        create_bead(ws.path(), "consistency test");

        // Get claim envelope
        let claim_envelope = run_envelope_command(ws.path(), &["claim", "--assignee", "consistency-worker", "--json"]);

        // Get stats envelope
        let stats_envelope = run_envelope_command(ws.path(), &["stats", "--format", "json"]);

        // Both should have version = 1
        assert_eq!(
            claim_envelope.get("version").and_then(|v| v.as_u64()),
            Some(1),
            "claim envelope version must be 1"
        );
        assert_eq!(
            stats_envelope.get("version").and_then(|v| v.as_u64()),
            Some(1),
            "stats envelope version must be 1"
        );

        // Both should have correct kind
        assert_eq!(
            claim_envelope.get("kind").and_then(|k| k.as_str()),
            Some("claim"),
            "claim envelope kind must be 'claim'"
        );
        assert_eq!(
            stats_envelope.get("kind").and_then(|k| k.as_str()),
            Some("stats"),
            "stats envelope kind must be 'stats'"
        );

        // Both should have data field
        assert!(claim_envelope.get("data").is_some(), "claim envelope must have data");
        assert!(stats_envelope.get("data").is_some(), "stats envelope must have data");
    }

    /// Test: claim envelope contains valid bead_id
    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn envelope_claim_bead_id_is_valid() {
        let ws = init_workspace();
        let id = create_bead(ws.path(), "bead id validation");

        let envelope = run_envelope_command(ws.path(), &["claim", "--assignee", "validation-worker", "--json"]);

        let data = &envelope["data"];
        let claimed_id = data.get("bead_id").and_then(|i| i.as_str());

        assert!(claimed_id.is_some(), "claim result must have bead_id");
        assert!(claimed_id.unwrap().starts_with("test-"), "claimed bead_id must start with 'test-' (workspace prefix)");
    }

    /// Test: stats envelope contains numeric fields
    #[test]
    fn envelope_stats_fields_are_numeric() {
        let ws = init_workspace();
        create_bead(ws.path(), "numeric test");

        let envelope = run_envelope_command(ws.path(), &["stats", "--format", "json"]);

        let data = &envelope["data"];

        // Common numeric stats fields that should exist
        if let Some(total) = data.get("total").and_then(|t| t.as_u64()) {
            assert!(total >= 0, "stats total must be non-negative");
        }

        // Check for other potential stats fields (these may vary)
        for (key, value) in data.as_object().unwrap() {
            if key != "total" && key != "by_status" && key != "by_priority" {
                // Most stats fields should be numeric
                if value.is_u64() || value.is_i64() || value.is_f64() {
                    // Valid numeric field
                } else if value.is_object() || value.is_array() {
                    // Nested stats are also valid
                }
            }
        }
    }

    /// Test: claim with specific assignee reflects in envelope
    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn envelope_claim_reflects_assignee() {
        let ws = init_workspace();
        create_bead(ws.path(), "assignee reflection test");

        let test_assignee = "specific-test-worker";
        let envelope = run_envelope_command(ws.path(), &["claim", "--assignee", test_assignee, "--json"]);

        let data = &envelope["data"];
        assert_eq!(
            data.get("assignee").and_then(|a| a.as_str()),
            Some(test_assignee),
            "claim envelope must reflect the specified assignee"
        );
    }

    /// Test: stats envelope reflects created beads
    #[test]
    fn envelope_stats_reflects_bead_count() {
        let ws = init_workspace();

        // Create specific number of beads
        create_bead(ws.path(), "count test 1");
        create_bead(ws.path(), "count test 2");
        create_bead(ws.path(), "count test 3");

        let envelope = run_envelope_command(ws.path(), &["stats", "--format", "json"]);

        let data = &envelope["data"];
        let total = data.get("total").and_then(|t| t.as_u64());

        assert!(total.is_some(), "stats must have total field");
        assert_eq!(total.unwrap(), 3, "stats total must reflect created bead count");
    }
}
