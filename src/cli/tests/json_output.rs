//! Test infrastructure and helpers for JSON output testing
//!
//! This module provides reusable test utilities for validating JSON output
//! from bead-forge CLI commands. It includes helpers for:
//!
//! - JSON validation and parsing
//! - Test fixture creation (beads, labels, dependencies)
//! - CLI output capture and assertion
//! - Envelope wrapping validation
//! - JSONL (JSON Lines) format validation

use std::process::Command;
use std::sync::OnceLock;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Test workspace isolation - ensures tests don't share state
static TEST_WORKSPACE: OnceLock<TempDir> = OnceLock::new();

/// Get or create the shared test workspace
pub fn test_workspace() -> &'static Path {
    TEST_WORKSPACE.get_or_init(|| {
        let dir = tempfile::tempdir().expect("Failed to create temp dir for tests");
        let beads_dir = dir.path().join(".beads");
        std::fs::create_dir(&beads_dir).expect("Failed to create .beads directory");

        // Initialize workspace with default config
        crate::config::init_workspace(&beads_dir, "bf-test")
            .expect("Failed to initialize test workspace");

        // Create database upfront to avoid race conditions in parallel tests
        let metadata = crate::config::load_metadata(&beads_dir)
            .expect("Failed to load metadata");
        let _ = crate::Storage::open(&beads_dir.join(&metadata.database))
            .expect("Failed to create database");

        dir
    }).path()
}

/// Get the path to the bf binary, preferring CARGO_BIN_EXE for test consistency
pub fn bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf")
        .unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Create a Command builder for bf with workspace already configured
pub fn bf_command() -> Command {
    let workspace = test_workspace();
    let beads_dir = workspace.join(".beads");

    let mut cmd = Command::new(bf_binary());
    cmd.arg("-w").arg(&beads_dir);
    cmd.current_dir(workspace);
    cmd
}

/// JSON validation helpers
pub mod json_validation {
    use serde_json::{Value, from_str};

    /// Parse a JSON string and panic if invalid
    pub fn parse_json(json: &str) -> Value {
        from_str(json).unwrap_or_else(|e| {
            panic!("Failed to parse JSON: {}\nJSON was: {}", e, json)
        })
    }

    /// Parse a JSONL string (newline-delimited JSON) into a Vec of values
    pub fn parse_jsonl(jsonl: &str) -> Vec<Value> {
        jsonl
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| parse_json(line))
            .collect()
    }

    /// Assert that a JSON string is valid
    pub fn assert_valid_json(json: &str) {
        parse_json(json);
    }

    /// Assert that JSONL output is valid (each line is valid JSON)
    pub fn assert_valid_jsonl(jsonl: &str) {
        for (i, line) in jsonl.lines().enumerate() {
            if !line.trim().is_empty() {
                // parse_json already panics on error, so we just call it
                parse_json(line);
            }
        }
    }

    /// Check if JSON has a specific field
    pub fn has_field(json: &Value, field: &str) -> bool {
        json.get(field).is_some()
    }

    /// Get a string field from JSON, panic if missing or not a string
    pub fn get_string(json: &Value, field: &str) -> String {
        json.get(field)
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("Field '{}' is not a string or is missing: {}", field, json))
            .to_string()
    }

    /// Compare two JSON values for equality, with better error messages
    pub fn assert_json_eq(actual: &Value, expected: &Value) {
        if actual != expected {
            panic!(
                "JSON mismatch\nExpected: {}\nActual:   {}",
                expected, actual
            );
        }
    }
}

/// Test fixture creation helpers
pub mod fixtures {
    use std::process::Command;

    /// Create a test bead with the given title
    pub fn create_bead(title: &str) -> String {
        let output = super::bf_command()
            .arg("create")
            .arg("--title")
            .arg(title)
            .arg("--type")
            .arg("task")
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

    /// Create a test bead with labels
    pub fn create_bead_with_labels(title: &str, labels: &[&str]) -> String {
        let bead_id = create_bead(title);

        for label in labels {
            let output = super::bf_command()
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

    /// Create a test bead with assignee
    pub fn create_bead_with_assignee(title: &str, assignee: &str) -> String {
        let output = super::bf_command()
            .arg("create")
            .arg("--title")
            .arg(title)
            .arg("--type")
            .arg("task")
            .arg("--priority")
            .arg("2")
            .arg("--assignee")
            .arg(assignee)
            .output()
            .expect("Failed to execute bf create");

        if !output.status.success() {
            panic!(
                "Failed to create bead: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        stdout.trim().to_string()
    }

    /// Close a test bead
    pub fn close_bead(bead_id: &str, reason: &str) {
        let output = super::bf_command()
            .arg("close")
            .arg(bead_id)
            .arg("--reason")
            .arg(reason)
            .output()
            .expect("Failed to execute bf close");

        if !output.status.success() {
            panic!(
                "Failed to close bead: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    /// Add a dependency between two beads
    pub fn add_dependency(blocked: &str, blocker: &str) {
        let output = super::bf_command()
            .arg("dep")
            .arg("add")
            .arg("--blocker")
            .arg(blocker)
            .arg("--blocks")
            .arg(blocked)
            .output()
            .expect("Failed to execute bf dep add");

        if !output.status.success() {
            panic!(
                "Failed to add dependency: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    /// Claim a bead for testing
    pub fn claim_bead(assignee: &str) -> String {
        let output = super::bf_command()
            .arg("claim")
            .arg("--assignee")
            .arg(assignee)
            .output()
            .expect("Failed to execute bf claim");

        if !output.status.success() {
            panic!(
                "Failed to claim bead: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        // Extract bead_id from JSON output
        let json = super::json_validation::parse_json(&stdout);
        super::json_validation::get_string(&json, "bead_id")
    }
}

/// Envelope wrapping validation helpers
pub mod envelope {
    use super::json_validation::*;
    use serde_json::Value;

    /// Expected envelope structure: {version: 1, kind: "<command>", data: <payload>}
    pub fn validate_envelope(json: &str, expected_kind: &str) -> Value {
        let envelope = parse_json(json);

        // Check version field
        let version = envelope.get("version")
            .and_then(|v| v.as_i64())
            .expect("Envelope must have numeric 'version' field");
        assert_eq!(version, 1, "Envelope version must be 1");

        // Check kind field
        let kind = envelope.get("kind")
            .and_then(|k| k.as_str())
            .expect("Envelope must have string 'kind' field");
        assert_eq!(kind, expected_kind, "Envelope kind mismatch");

        // Check data field exists
        assert!(
            envelope.get("data").is_some(),
            "Envelope must have 'data' field"
        );

        envelope
    }

    /// Get the data field from an envelope
    pub fn get_envelope_data(envelope: &Value) -> Value {
        envelope.get("data")
            .cloned()
            .unwrap_or_else(|| panic!("Envelope missing 'data' field"))
    }

    /// Check if envelope has a warning field
    pub fn has_warning(envelope: &Value) -> bool {
        envelope.get("warning").is_some()
    }

    /// Get warning from envelope if present
    pub fn get_warning(envelope: &Value) -> Option<String> {
        envelope.get("warning")
            .and_then(|w| w.as_str())
            .map(|s| s.to_string())
    }
}

/// Command output capture helpers
pub mod capture {
    use std::process::Command;

    /// Capture stdout from a command as a string
    pub fn capture_stdout(cmd: &mut Command) -> String {
        let output = cmd.output().expect("Failed to execute command");

        if !output.status.success() {
            panic!(
                "Command failed: {}\nStderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        String::from_utf8(output.stdout).expect("Invalid UTF-8")
    }

    /// Capture stderr from a command as a string
    pub fn capture_stderr(cmd: &mut Command) -> String {
        let output = cmd.output().expect("Failed to execute command");
        String::from_utf8(output.stderr).expect("Invalid UTF-8")
    }

    /// Capture both stdout and stderr from a command
    pub fn capture_both(cmd: &mut Command) -> (String, String) {
        let output = cmd.output().expect("Failed to execute command");

        let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
        let stderr = String::from_utf8(output.stderr).expect("Invalid UTF-8");

        if !output.status.success() {
            panic!("Command failed.\nStdout: {}\nStderr: {}", stdout, stderr);
        }

        (stdout, stderr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_creation() {
        let workspace = test_workspace();
        assert!(workspace.exists(), "Test workspace should exist");

        let beads_dir = workspace.join(".beads");
        assert!(beads_dir.exists(), ".beads directory should exist");
    }

    #[test]
    fn test_bf_binary_resolution() {
        let binary = bf_binary();
        assert!(!binary.is_empty(), "Binary path should not be empty");
    }

    #[test]
    #[ignore] // This test requires the binary to be built first
    fn test_create_test_bead() {
        // Only run this test if the binary exists
        let binary = bf_binary();
        if !std::path::Path::new(&binary).exists() {
            eprintln!("Skipping test - binary not found at: {}", binary);
            return;
        }

        let bead_id = fixtures::create_bead("Test bead for infrastructure");
        assert!(!bead_id.is_empty(), "Bead ID should not be empty");
        assert!(bead_id.starts_with("bf-test-"), "Bead ID should have correct prefix");

        // Cleanup
        fixtures::close_bead(&bead_id, "Infrastructure test cleanup");
    }

    #[test]
    fn test_json_validation_helpers() {
        let valid_json = r#"{"id": "bf-test", "title": "Test"}"#;
        json_validation::assert_valid_json(valid_json);

        let parsed = json_validation::parse_json(valid_json);
        assert_eq!(json_validation::get_string(&parsed, "id"), "bf-test");
        assert_eq!(json_validation::get_string(&parsed, "title"), "Test");
    }

    #[test]
    fn test_jsonl_validation() {
        let jsonl = r#"{"id": "bf-1"}
{"id": "bf-2"}
{"id": "bf-3"}"#;

        json_validation::assert_valid_jsonl(jsonl);

        let parsed = json_validation::parse_jsonl(jsonl);
        assert_eq!(parsed.len(), 3, "Should parse 3 JSONL lines");
    }

    #[test]
    fn test_envelope_validation() {
        let envelope_str = r#"{"version": 1, "kind": "create", "data": {"id": "bf-test"}}"#;

        let envelope = envelope::validate_envelope(envelope_str, "create");
        let data = envelope::get_envelope_data(&envelope);

        assert_eq!(json_validation::get_string(&data, "id"), "bf-test");
        assert!(!envelope::has_warning(&envelope));
    }

    #[test]
    fn test_envelope_with_warning() {
        let envelope_str = r#"{"version": 1, "kind": "create", "data": {"id": "bf-test"}, "warning": "Test warning"}"#;

        let envelope = envelope::validate_envelope(envelope_str, "create");
        assert!(envelope::has_warning(&envelope));

        let warning = envelope::get_warning(&envelope);
        assert_eq!(warning, Some("Test warning".to_string()));
    }
}
