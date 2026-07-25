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

    /// Try to parse a JSON string, returning a Result
    pub fn try_parse_json(json: &str) -> Result<Value, serde_json::Error> {
        from_str(json)
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

    /// Check that multiple required fields exist in JSON, with better error messages
    pub fn assert_required_fields(json: &Value, fields: &[&str], context: &str) {
        for field in fields {
            if !has_field(json, field) {
                panic!(
                    "{}: Missing required field '{}'. JSON was:\n{}",
                    context, field, json
                );
            }
        }
    }

    /// Get an integer field from JSON, panic if missing or not an integer
    pub fn get_int(json: &Value, field: &str) -> i64 {
        json.get(field)
            .and_then(|v| v.as_i64())
            .unwrap_or_else(|| panic!("Field '{}' is not an integer or is missing: {}", field, json))
    }

    /// Get a boolean field from JSON, panic if missing or not a boolean
    pub fn get_bool(json: &Value, field: &str) -> bool {
        json.get(field)
            .and_then(|v| v.as_bool())
            .unwrap_or_else(|| panic!("Field '{}' is not a boolean or is missing: {}", field, json))
    }

    /// Get an array field from JSON, panic if missing or not an array
    pub fn get_array(json: &Value, field: &str) -> Vec<Value> {
        json.get(field)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_else(|| panic!("Field '{}' is not an array or is missing: {}", field, json))
    }

    /// Get an object field from JSON, panic if missing or not an object
    pub fn get_object(json: &Value, field: &str) -> Value {
        json.get(field)
            .and_then(|v| if v.is_object() { Some(v) } else { None })
            .cloned()
            .unwrap_or_else(|| panic!("Field '{}' is not an object or is missing: {}", field, json))
    }

    /// Get an optional string field from JSON, returning None if missing or not a string
    pub fn get_string_optional(json: &Value, field: &str) -> Option<String> {
        json.get(field)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Get an optional integer field from JSON, returning None if missing or not an integer
    pub fn get_int_optional(json: &Value, field: &str) -> Option<i64> {
        json.get(field)
            .and_then(|v| v.as_i64())
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

/// Format detection and validation helpers
pub mod format_detection {
    use super::json_validation::*;

    /// JSON output format types
    #[derive(Debug, Clone, PartialEq)]
    pub enum JsonFormat {
        /// Single JSON object (e.g., `{"key": "value"}`)
        SingleObject,
        /// JSON array (e.g., `[{"key": "value"}]`)
        Array,
        /// JSONL - newline-delimited JSON (e.g., `{"key": "value"}\n{"key2": "value2"}`)
        JsonL,
        /// Empty array (`[]`)
        EmptyArray,
        /// Empty string (no output)
        Empty,
    }

    /// Detect the format of JSON output
    pub fn detect_format(output: &str) -> JsonFormat {
        let trimmed = output.trim();

        if trimmed.is_empty() {
            return JsonFormat::Empty;
        }

        if trimmed == "[]" {
            return JsonFormat::EmptyArray;
        }

        // Check if it's a JSONL (multiple lines)
        let lines: Vec<&str> = trimmed.lines().filter(|l| !l.trim().is_empty()).collect();

        if lines.len() > 1 {
            // Verify each line is valid JSON
            for line in lines {
                parse_json(line);
            }
            return JsonFormat::JsonL;
        }

        // Single line - check if it's an array or object
        let parsed = parse_json(trimmed);

        if parsed.is_array() {
            JsonFormat::Array
        } else if parsed.is_object() {
            JsonFormat::SingleObject
        } else {
            panic!("Unexpected JSON format: {}", trimmed);
        }
    }

    /// Assert that output is in the expected format
    pub fn assert_format(output: &str, expected: JsonFormat) {
        let detected = detect_format(output);
        assert_eq!(
            detected, expected,
            "Format mismatch: expected {:?}, got {:?}",
            expected, detected
        );
    }

    /// Check if output is valid JSONL (may be empty)
    pub fn is_valid_jsonl(output: &str) -> bool {
        let trimmed = output.trim();

        if trimmed.is_empty() || trimmed == "[]" {
            return true;
        }

        // Try to parse each line as JSON
        for line in trimmed.lines() {
            if !line.trim().is_empty() {
                if try_parse_json(line).is_err() {
                    return false;
                }
            }
        }

        true
    }

    /// Check if output is a valid JSON object
    pub fn is_valid_json_object(output: &str) -> bool {
        let trimmed = output.trim();
        if trimmed.is_empty() {
            return false;
        }

        match try_parse_json(trimmed) {
            Ok(v) => v.is_object(),
            Err(_) => false,
        }
    }

    /// Check if output is a valid JSON array
    pub fn is_valid_json_array(output: &str) -> bool {
        let trimmed = output.trim();
        if trimmed.is_empty() {
            return false;
        }

        match try_parse_json(trimmed) {
            Ok(v) => v.is_array(),
            Err(_) => false,
        }
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
mod infrastructure_tests {
    use super::*;
    use super::json_validation::*;

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

    #[test]
    fn test_assert_required_fields() {
        let json_str = r#"{"id": "bf-test", "title": "Test", "status": "open"}"#;
        let parsed = parse_json(json_str);

        // Should succeed when all fields are present
        assert_required_fields(&parsed, &["id", "title", "status"], "test context");

        // Should panic when a field is missing
        let result = std::panic::catch_unwind(|| {
            assert_required_fields(&parsed, &["id", "title", "missing_field"], "test context");
        });
        assert!(result.is_err(), "Should panic when required field is missing");
    }

    #[test]
    fn test_get_int() {
        let json_str = r#"{"id": "bf-test", "priority": 2}"#;
        let parsed = parse_json(json_str);

        assert_eq!(get_int(&parsed, "priority"), 2);

        // Should panic when field is not an integer
        let result = std::panic::catch_unwind(|| {
            get_int(&parsed, "id");
        });
        assert!(result.is_err(), "Should panic when field is not an integer");
    }

    #[test]
    fn test_get_bool() {
        let json_str = r#"{"active": true, "inactive": false}"#;
        let parsed = parse_json(json_str);

        assert!(get_bool(&parsed, "active"));
        assert!(!get_bool(&parsed, "inactive"));

        // Should panic when field is not a boolean
        let result = std::panic::catch_unwind(|| {
            get_bool(&parsed, "id");
        });
        assert!(result.is_err(), "Should panic when field is not a boolean");
    }

    #[test]
    fn test_get_array() {
        let json_str = r#"{"labels": ["bug", "feature"], "empty": []}"#;
        let parsed = parse_json(json_str);

        let labels = get_array(&parsed, "labels");
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0].as_str(), Some("bug"));

        let empty = get_array(&parsed, "empty");
        assert_eq!(empty.len(), 0);

        // Should panic when field is not an array
        let result = std::panic::catch_unwind(|| {
            get_array(&parsed, "id");
        });
        assert!(result.is_err(), "Should panic when field is not an array");
    }

    #[test]
    fn test_get_object() {
        let json_str = r#"{"metadata": {"key": "value", "count": 5}}"#;
        let parsed = parse_json(json_str);

        let metadata = get_object(&parsed, "metadata");
        assert!(metadata.is_object());
        assert_eq!(metadata.get("key").and_then(|v| v.as_str()), Some("value"));

        // Should panic when field is not an object
        let result = std::panic::catch_unwind(|| {
            get_object(&parsed, "key");
        });
        assert!(result.is_err(), "Should panic when field is not an object");
    }

    #[test]
    fn test_get_string_optional() {
        let json_str = r#"{"id": "bf-test", "description": null, "title": "Test"}"#;
        let parsed = parse_json(json_str);

        assert_eq!(get_string_optional(&parsed, "id"), Some("bf-test".to_string()));
        assert_eq!(get_string_optional(&parsed, "description"), None);
        assert_eq!(get_string_optional(&parsed, "missing"), None);
    }

    #[test]
    fn test_get_int_optional() {
        let json_str = r#"{"priority": 2, "count": null, "status": "open"}"#;
        let parsed = parse_json(json_str);

        assert_eq!(get_int_optional(&parsed, "priority"), Some(2));
        assert_eq!(get_int_optional(&parsed, "count"), None);
        assert_eq!(get_int_optional(&parsed, "status"), None);
        assert_eq!(get_int_optional(&parsed, "missing"), None);
    }
}

#[cfg(test)]
mod format_detection_tests {
    use super::*;

    #[test]
    fn test_detect_single_object() {
        let json_str = r#"{"id": "bf-test", "title": "Test"}"#;
        let format = format_detection::detect_format(json_str);
        assert_eq!(format, format_detection::JsonFormat::SingleObject);
    }

    #[test]
    fn test_detect_array() {
        let json_str = r#"[{"id": "bf-1"}, {"id": "bf-2"}]"#;
        let format = format_detection::detect_format(json_str);
        assert_eq!(format, format_detection::JsonFormat::Array);
    }

    #[test]
    fn test_detect_jsonl() {
        let jsonl = r#"{"id": "bf-1"}
{"id": "bf-2"}
{"id": "bf-3"}"#;
        let format = format_detection::detect_format(jsonl);
        assert_eq!(format, format_detection::JsonFormat::JsonL);
    }

    #[test]
    fn test_detect_empty_array() {
        let json_str = r#"[]"#;
        let format = format_detection::detect_format(json_str);
        assert_eq!(format, format_detection::JsonFormat::EmptyArray);
    }

    #[test]
    fn test_detect_empty() {
        let format = format_detection::detect_format("");
        assert_eq!(format, format_detection::JsonFormat::Empty);

        let format = format_detection::detect_format("   ");
        assert_eq!(format, format_detection::JsonFormat::Empty);
    }

    #[test]
    fn test_assert_format() {
        format_detection::assert_format(
            r#"{"id": "test"}"#,
            format_detection::JsonFormat::SingleObject
        );

        format_detection::assert_format(
            r#"[]"#,
            format_detection::JsonFormat::EmptyArray
        );

        let result = std::panic::catch_unwind(|| {
            format_detection::assert_format(
                r#"{"id": "test"}"#,
                format_detection::JsonFormat::Array
            );
        });
        assert!(result.is_err(), "Should panic when format doesn't match");
    }

    #[test]
    fn test_is_valid_jsonl() {
        assert!(format_detection::is_valid_jsonl(
            r#"{"id": "bf-1"}
{"id": "bf-2"}"#
        ));

        assert!(format_detection::is_valid_jsonl(""));
        assert!(format_detection::is_valid_jsonl("[]"));

        // Invalid JSONL (one line is invalid JSON)
        assert!(!format_detection::is_valid_jsonl(
            r#"{"id": "bf-1"}
invalid json
{"id": "bf-2"}"#
        ));
    }

    #[test]
    fn test_is_valid_json_object() {
        assert!(format_detection::is_valid_json_object(r#"{"id": "test"}"#));
        assert!(!format_detection::is_valid_json_object(r#"[{"id": "test"}]"#));
        assert!(!format_detection::is_valid_json_object(r#"[]"#));
        assert!(!format_detection::is_valid_json_object(""));
    }

    #[test]
    fn test_is_valid_json_array() {
        assert!(format_detection::is_valid_json_array(r#"[{"id": "test"}]"#));
        assert!(format_detection::is_valid_json_array(r#"[]"#));
        assert!(!format_detection::is_valid_json_array(r#"{"id": "test"}"#));
        assert!(!format_detection::is_valid_json_array(""));
    }
}

/// Core command JSON output tests
///
/// Tests the JSON output format for core bead-forge commands:
/// - show: Single bead details
/// - list: Multiple beads in JSONL format
/// - search: Search results in JSONL format
/// - ready: Ready (unblocked) beads in JSONL format
/// - recent: Recently modified beads with envelope wrapping
#[cfg(test)]
mod command_json_output_tests {
    use super::*;

    /// Helper to check if binary exists, skip test if not
    fn require_binary() {
        let binary = bf_binary();
        if !std::path::Path::new(&binary).exists() {
            eprintln!("Skipping test - binary not found at: {}. Run 'cargo build' first.", binary);
            panic!("Binary not found");
        }
    }

    /// Helper to check required issue fields in JSON
    fn assert_issue_fields_present(json: &serde_json::Value, context: &str) {
        assert!(json.get("id").is_some(), "{}: Missing 'id' field", context);
        assert!(json.get("title").is_some(), "{}: Missing 'title' field", context);
        assert!(json.get("status").is_some(), "{}: Missing 'status' field", context);
        assert!(json.get("priority").is_some(), "{}: Missing 'priority' field", context);
        assert!(json.get("type").is_some(), "{}: Missing 'type' field", context);
        // These should always be present even if null/empty (display normalization)
        assert!(json.get("assignee").is_some(), "{}: Missing 'assignee' field", context);
        assert!(json.get("labels").is_some(), "{}: Missing 'labels' field", context);
    }

    #[test]
    #[ignore]
    fn test_show_command_json_structure() {
        require_binary();

        // Create a test bead
        let bead_id = fixtures::create_bead("Test bead for show command JSON");

        // Get JSON output
        let output = capture::capture_stdout(
            bf_command()
                .arg("show")
                .arg(&bead_id)
                .arg("--format")
                .arg("json")
        );

        // show command wraps output in array for NEEDLE compatibility: [{...}]
        let json_str = output.trim();
        assert!(json_str.starts_with('['), "show output should start with '['");
        assert!(json_str.ends_with(']'), "show output should end with ']'");

        // Parse as array
        let parsed = json_validation::parse_json(json_str);
        let array = parsed.as_array().expect("show output should be a JSON array");
        assert_eq!(array.len(), 1, "show should return exactly one issue");

        let issue_json = &array[0];
        assert_issue_fields_present(issue_json, "show command");
        assert_eq!(json_validation::get_string(issue_json, "id"), bead_id);

        // Cleanup
        fixtures::close_bead(&bead_id, "Show command test cleanup");
    }

    #[test]
    #[ignore]
    fn test_show_command_json_special_characters() {
        require_binary();

        // Create bead with special characters
        let special_title = "Test bead with \"quotes\" and 'apostrophes' & symbols <>";
        let bead_id = fixtures::create_bead(special_title);

        // Get JSON output
        let output = capture::capture_stdout(
            bf_command()
                .arg("show")
                .arg(&bead_id)
                .arg("--format")
                .arg("json")
        );

        // Parse and verify special characters are properly escaped
        let json_str = output.trim();
        let parsed = json_validation::parse_json(json_str);
        let array = parsed.as_array().expect("show output should be a JSON array");
        let issue_json = &array[0];

        let title = json_validation::get_string(issue_json, "title");
        assert!(title.contains("quotes"), "Title should contain escaped quotes");

        // Cleanup
        fixtures::close_bead(&bead_id, "Special characters test cleanup");
    }

    #[test]
    #[ignore]
    fn test_show_command_json_empty_dependencies_comments() {
        require_binary();

        let bead_id = fixtures::create_bead("Test bead for empty deps/comments");

        let output = capture::capture_stdout(
            bf_command()
                .arg("show")
                .arg(&bead_id)
                .arg("--format")
                .arg("json")
        );

        let json_str = output.trim();
        let parsed = json_validation::parse_json(json_str);
        let array = parsed.as_array().expect("show output should be a JSON array");
        let issue_json = &array[0];

        // Verify dependencies and comments are stripped/empty
        assert!(issue_json.get("dependencies").is_none(), "dependencies should be stripped from JSON output");
        assert!(issue_json.get("comments").is_none(), "comments should be stripped from JSON output");

        fixtures::close_bead(&bead_id, "Empty deps test cleanup");
    }

    #[test]
    #[ignore]
    fn test_list_command_json_structure() {
        require_binary();

        // Create multiple beads
        let bead1 = fixtures::create_bead("First bead for list test");
        let bead2 = fixtures::create_bead("Second bead for list test");
        let bead3 = fixtures::create_bead("Third bead for list test");

        // Get JSON output
        let output = capture::capture_stdout(
            bf_command()
                .arg("list")
                .arg("--format")
                .arg("json")
        );

        // list returns JSONL (one JSON object per line)
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines.len() >= 3, "list should return at least 3 beads");

        // Each line should be valid JSON with required fields
        for line in lines.iter().take(3) {
            let parsed = json_validation::parse_json(line);
            assert_issue_fields_present(&parsed, "list command");
        }

        // Cleanup
        fixtures::close_bead(&bead1, "List test cleanup 1");
        fixtures::close_bead(&bead2, "List test cleanup 2");
        fixtures::close_bead(&bead3, "List test cleanup 3");
    }

    #[test]
    #[ignore]
    fn test_list_command_json_empty_results() {
        require_binary();

        // List from empty workspace (or with status that yields no results)
        let output = capture::capture_stdout(
            bf_command()
                .arg("list")
                .arg("--status")
                .arg("closed")
                .arg("--format")
                .arg("json")
        );

        // Empty list returns "[]" (special case in cmd_list)
        let trimmed = output.trim();
        assert_eq!(trimmed, "[]", "Empty list should return '[]'");
    }

    #[test]
    #[ignore]
    fn test_list_command_json_filters() {
        require_binary();

        // Create beads with different properties
        let open_bead = fixtures::create_bead("Open bead for filter test");
        fixtures::close_bead(&open_bead, "Close for filter test");

        let active_bead = fixtures::create_bead("Active bead for filter test");

        // Test status filter
        let output = capture::capture_stdout(
            bf_command()
                .arg("list")
                .arg("--status")
                .arg("closed")
                .arg("--format")
                .arg("json")
        );

        let lines: Vec<&str> = output.lines().filter(|l| !l.is_empty() && *l != "[]").collect();
        assert!(lines.len() >= 1, "Should find at least one closed bead");

        // Verify the filtered result has correct status
        let parsed = json_validation::parse_json(lines[0]);
        assert_eq!(json_validation::get_string(&parsed, "status"), "closed");

        fixtures::close_bead(&active_bead, "Filter test cleanup");
    }

    #[test]
    #[ignore]
    fn test_list_command_json_ensure_fields_present() {
        require_binary();

        let bead_id = fixtures::create_bead("Test bead for field presence");

        let output = capture::capture_stdout(
            bf_command()
                .arg("list")
                .arg("--format")
                .arg("json")
        );

        // Find our bead in the output
        let lines: Vec<&str> = output.lines().collect();
        let our_bead = lines.iter()
            .find(|line| line.contains(&bead_id))
            .expect("Should find our bead in list output");

        let parsed = json_validation::parse_json(our_bead);

        // Verify display normalization: assignee and labels always present
        assert!(parsed.get("assignee").is_some(), "assignee must be present");
        assert!(parsed.get("labels").is_some(), "labels must be present");
        assert!(parsed.get("labels").unwrap().is_array(), "labels must be an array");

        fixtures::close_bead(&bead_id, "Field presence test cleanup");
    }

    #[test]
    #[ignore]
    fn test_search_command_json_structure() {
        require_binary();

        // Create test beads
        let bead1 = fixtures::create_bead("Searchable bead with unique keyword");
        let bead2 = fixtures::create_bead("Another searchable item");

        // Search for beads
        let output = capture::capture_stdout(
            bf_command()
                .arg("search")
                .arg("keyword")
                .arg("--format")
                .arg("json")
        );

        // search returns JSONL (one JSON object per line)
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines.len() >= 1, "search should find at least one bead");

        // Each line should be valid JSON with required fields
        for line in lines {
            let parsed = json_validation::parse_json(line);
            assert_issue_fields_present(&parsed, "search command");
        }

        // Cleanup
        fixtures::close_bead(&bead1, "Search test cleanup 1");
        fixtures::close_bead(&bead2, "Search test cleanup 2");
    }

    #[test]
    #[ignore]
    fn test_search_command_json_empty_results() {
        require_binary();

        // Search for something that doesn't exist
        let output = capture::capture_stdout(
            bf_command()
                .arg("search")
                .arg("nonexistent")
                .arg("--format")
                .arg("json")
        );

        // Empty search produces no output (different from list!)
        let trimmed = output.trim();
        assert_eq!(trimmed, "", "Empty search should produce no output");
    }

    #[test]
    #[ignore]
    fn test_search_command_json_with_filters() {
        require_binary();

        // Create beads with different properties
        let high_priority = fixtures::create_bead("High priority searchable bead");
        let low_priority = fixtures::create_bead("Low priority bead");

        // Search with priority filter
        let output = capture::capture_stdout(
            bf_command()
                .arg("search")
                .arg("bead")
                .arg("--priority-min")
                .arg("0")
                .arg("--priority-max")
                .arg("1")
                .arg("--format")
                .arg("json")
        );

        let lines: Vec<&str> = output.lines().collect();
        // Verify results are within priority range (if any returned)
        for line in lines {
            let parsed = json_validation::parse_json(line);
            if let Some(priority) = parsed.get("priority").and_then(|p| p.as_i64()) {
                assert!(priority >= 0 && priority <= 1, "Priority should be in range 0-1");
            }
        }

        fixtures::close_bead(&high_priority, "Search filter test cleanup 1");
        fixtures::close_bead(&low_priority, "Search filter test cleanup 2");
    }

    #[test]
    #[ignore]
    fn test_ready_command_json_structure() {
        require_binary();

        // Create some open beads (they should be ready)
        let bead1 = fixtures::create_bead("Ready bead 1");
        let bead2 = fixtures::create_bead("Ready bead 2");

        // Get ready beads
        let output = capture::capture_stdout(
            bf_command()
                .arg("ready")
                .arg("--format")
                .arg("json")
        );

        // ready returns JSONL, empty returns "[]"
        let trimmed = output.trim();
        if trimmed != "[]" {
            let lines: Vec<&str> = trimmed.lines().collect();
            assert!(lines.len() >= 1, "ready should return at least some beads");

            // Each line should be valid JSON with required fields
            for line in lines {
                let parsed = json_validation::parse_json(line);
                assert_issue_fields_present(&parsed, "ready command");
            }
        }

        fixtures::close_bead(&bead1, "Ready test cleanup 1");
        fixtures::close_bead(&bead2, "Ready test cleanup 2");
    }

    #[test]
    #[ignore]
    fn test_ready_command_json_empty_results() {
        require_binary();

        // If all beads are blocked or closed, ready should return []
        let output = capture::capture_stdout(
            bf_command()
                .arg("ready")
                .arg("--limit")
                .arg("0")
                .arg("--format")
                .arg("json")
        );

        let trimmed = output.trim();
        // Empty ready returns "[]" (special case in cmd_ready)
        assert!(trimmed == "[]" || trimmed.is_empty(), "Empty ready should return '[]' or empty string");
    }

    #[test]
    #[ignore]
    fn test_ready_command_json_limit() {
        require_binary();

        // Create multiple beads
        for i in 1..=5 {
            let bead = fixtures::create_bead(&format!("Ready bead {}", i));
            fixtures::close_bead(&bead, "Limit test setup");
        }

        // Test with limit
        let output = capture::capture_stdout(
            bf_command()
                .arg("ready")
                .arg("--limit")
                .arg("2")
                .arg("--format")
                .arg("json")
        );

        let trimmed = output.trim();
        if trimmed != "[]" && !trimmed.is_empty() {
            let lines: Vec<&str> = trimmed.lines().collect();
            assert!(lines.len() <= 2, "ready with --limit 2 should return at most 2 beads");
        }
    }

    #[test]
    #[ignore]
    fn test_recent_command_json_structure() {
        require_binary();

        // Create a test bead
        let bead_id = fixtures::create_bead("Recent bead for test");

        // Get recent beads (always uses envelope)
        let output = capture::capture_stdout(
            bf_command()
                .arg("recent")
                .arg("--format")
                .arg("json")
        );

        // recent always wraps output in envelope
        let envelope = envelope::validate_envelope(&output, "recent");
        let data = envelope::get_envelope_data(&envelope);

        // Data should be an array or JSONL string
        if let Some(array) = data.as_array() {
            // Verify array has our bead
            assert!(array.len() >= 1, "recent should return at least one bead");
            for issue_json in array {
                assert_issue_fields_present(issue_json, "recent command");
            }
        }

        fixtures::close_bead(&bead_id, "Recent test cleanup");
    }

    #[test]
    #[ignore]
    fn test_recent_command_json_time_period() {
        require_binary();

        let bead_id = fixtures::create_bead("Recent bead with time filter");

        // Test with time period
        let output = capture::capture_stdout(
            bf_command()
                .arg("recent")
                .arg("--time-period")
                .arg("1h")
                .arg("--format")
                .arg("json")
        );

        // Should still be wrapped in envelope
        let envelope = envelope::validate_envelope(&output, "recent");
        let data = envelope::get_envelope_data(&envelope);

        // Data should be present (even if empty array)
        assert!(data.is_array() || data.is_string(), "recent data should be array or string");

        fixtures::close_bead(&bead_id, "Time period test cleanup");
    }

    #[test]
    #[ignore]
    fn test_recent_command_json_empty_results() {
        require_binary();

        // Use very short time period that should yield no results
        let output = capture::capture_stdout(
            bf_command()
                .arg("recent")
                .arg("--time-period")
                .arg("1s")
                .arg("--format")
                .arg("json")
        );

        // Even empty results are wrapped in envelope
        let envelope = envelope::validate_envelope(&output, "recent");
        let data = envelope::get_envelope_data(&envelope);

        // Empty results should be empty array or empty string
        if let Some(array) = data.as_array() {
            assert_eq!(array.len(), 0, "Empty recent should return empty array");
        }
    }

    #[test]
    #[ignore]
    fn test_show_command_json_with_envelope() {
        require_binary();

        let bead_id = fixtures::create_bead("Test bead for envelope show");

        // Test with envelope flag
        let output = capture::capture_stdout(
            bf_command()
                .arg("show")
                .arg(&bead_id)
                .arg("--format")
                .arg("json")
                .arg("--envelope")
        );

        // Should be wrapped in envelope
        let envelope = envelope::validate_envelope(&output, "show");
        let data = envelope::get_envelope_data(&envelope);

        // Data should be an array with one element (show wraps in array)
        let array = data.as_array().expect("show data should be array");
        assert_eq!(array.len(), 1, "show should return one bead");

        let issue_json = &array[0];
        assert_issue_fields_present(issue_json, "show with envelope");
        assert_eq!(json_validation::get_string(issue_json, "id"), bead_id);

        fixtures::close_bead(&bead_id, "Envelope show test cleanup");
    }

    #[test]
    #[ignore]
    fn test_list_command_json_with_envelope() {
        require_binary();

        fixtures::create_bead("Test bead for envelope list");

        // Test with envelope flag
        let output = capture::capture_stdout(
            bf_command()
                .arg("list")
                .arg("--format")
                .arg("json")
                .arg("--envelope")
        );

        // Should be wrapped in envelope
        let envelope = envelope::validate_envelope(&output, "list");
        let data = envelope::get_envelope_data(&envelope);

        // Data should be an array of issues
        let array = data.as_array().expect("list data should be array");
        assert!(array.len() >= 1, "list should return at least one bead");

        for issue_json in array {
            assert_issue_fields_present(issue_json, "list with envelope");
        }
    }

    #[test]
    #[ignore]
    fn test_ready_command_json_with_envelope() {
        require_binary();

        fixtures::create_bead("Test bead for envelope ready");

        // Test with envelope flag
        let output = capture::capture_stdout(
            bf_command()
                .arg("ready")
                .arg("--format")
                .arg("json")
                .arg("--envelope")
        );

        // Should be wrapped in envelope
        let envelope = envelope::validate_envelope(&output, "ready");
        let data = envelope::get_envelope_data(&envelope);

        // Data should be an array or empty array
        let array = data.as_array().expect("ready data should be array");
        if !array.is_empty() {
            for issue_json in array {
                assert_issue_fields_present(issue_json, "ready with envelope");
            }
        }
    }

    #[test]
    #[ignore]
    fn test_search_command_json_with_envelope() {
        require_binary();

        fixtures::create_bead("Searchable envelope test bead");

        // Test with envelope flag
        let output = capture::capture_stdout(
            bf_command()
                .arg("search")
                .arg("envelope")
                .arg("--format")
                .arg("json")
                .arg("--envelope")
        );

        // Should be wrapped in envelope
        let envelope = envelope::validate_envelope(&output, "search");
        let data = envelope::get_envelope_data(&envelope);

        // Data should be an array or empty array
        if let Some(array) = data.as_array() {
            if !array.is_empty() {
                for issue_json in array {
                    assert_issue_fields_present(issue_json, "search with envelope");
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn test_json_output_handles_unicode() {
        require_binary();

        // Create bead with Unicode characters
        let unicode_title = "Test bead with emoji 🎉 and unicode Ñ";
        let bead_id = fixtures::create_bead(unicode_title);

        // Get JSON output from show command
        let output = capture::capture_stdout(
            bf_command()
                .arg("show")
                .arg(&bead_id)
                .arg("--format")
                .arg("json")
        );

        // Should be valid JSON
        json_validation::assert_valid_json(&output.trim());

        // Parse and verify Unicode is preserved
        let json_str = output.trim();
        let parsed = json_validation::parse_json(json_str);
        let array = parsed.as_array().expect("show output should be a JSON array");
        let issue_json = &array[0];

        let title = json_validation::get_string(issue_json, "title");
        assert!(title.contains("🎉"), "Unicode emoji should be preserved");
        assert!(title.contains("Ñ"), "Unicode character should be preserved");

        fixtures::close_bead(&bead_id, "Unicode test cleanup");
    }

    #[test]
    #[ignore]
    fn test_json_output_handles_newlines_in_description() {
        require_binary();

        let bead_id = fixtures::create_bead("Test bead with multiline description");

        // Update with description containing newlines
        let mut update_cmd = bf_command();
        update_cmd.arg("update")
            .arg(&bead_id)
            .arg("--description")
            .arg("Line 1\nLine 2\nLine 3");

        let _update_output = update_cmd.output().expect("Failed to update bead");

        // Get JSON output
        let output = capture::capture_stdout(
            bf_command()
                .arg("show")
                .arg(&bead_id)
                .arg("--format")
                .arg("json")
        );

        // Should be valid JSON despite newlines
        json_validation::assert_valid_json(&output.trim());

        fixtures::close_bead(&bead_id, "Newline test cleanup");
    }
}
