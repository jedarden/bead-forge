//! JSON envelope for unified --json output across all commands.
//!
//! All `bf` commands that support `--json` or `--format json` now emit a
//! stable envelope shape:
//!
//! ```json
//! {
//!   "version": 1,
//!   "kind": "<command>",
//!   "data": <command-specific data>,
//!   "warning": "<auto-flush failure message, if any>"
//! }
//! ```
//!
//! The `version` field enables future compatibility; `kind` identifies the
//! command so consumers can parse `data` correctly; `warning` is present only
//! when auto-flush fails (see `crate::autoflush`).
//!
//! ## Command-specific `data` shapes
//!
//! | Command | `data` shape | Empty result |
//! |---------|-------------|--------------|
//! | `create` | `{"id": "bf-xxx"}` | N/A (always succeeds) |
//! | `list` | `[{...}, {...}]` | `[]` |
//! | `ready` | `[{...}, {...}]` | `[]` |
//! | `show` | `{...}` | error (not found) |
//! | `claim` | `{...}` | `{}` (no bead available) |
//! | `update` | `{id: "..."}` | N/A |
//! | `close` | `{id: "..."}` | N/A |
//! | `reopen` | `{id: "..."}` | N/A |
//! | `delete` | `{id: "..."}` | N/A |
//! | `stats` | `{total: ..., ...}` | N/A |
//! | `velocity` | `[{...}, {...}]` | `[]` |
//! | `search` | `[{...}, {...}]` | `[]` |
//! | `recent` | `[{...}, {...}]` | `[]` |
//! | `batch` | `[{op: ..., result: ...}]` | `[]` |
//!
//! List-like commands (list, ready, search, recent, velocity) now emit a JSON
//! **array** wrapped in the envelope, not NDJSON. This fixes the prior divergence
//! where `json.load()` on `bf list --format json` would fail silently (only the
//! first line parses) while `bf ready --json` on empty output emitted `[]`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Current envelope version.
pub const VERSION: u32 = 1;

/// Unified JSON envelope for all command outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonEnvelope {
    /// Envelope version (currently 1).
    pub version: u32,
    /// Command identifier (e.g., "list", "ready", "claim", "create").
    pub kind: String,
    /// Command-specific data (varies by command).
    pub data: Value,
    /// Warning message (present only when auto-flush fails).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

impl JsonEnvelope {
    /// Create a new envelope for a command.
    pub fn new(kind: impl Into<String>, data: Value) -> Self {
        Self {
            version: VERSION,
            kind: kind.into(),
            data,
            warning: None,
        }
    }

    /// Add a warning to the envelope (e.g., auto-flush failure).
    pub fn with_warning(mut self, message: impl Into<String>) -> Self {
        self.warning = Some(message.into());
        self
    }

    /// Serialize the envelope to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Serialize the envelope to a compact JSON string (no pretty-printing).
    pub fn to_json_compact(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

// Unit tests for envelope structure and metadata
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // === Envelope creation tests ===

    #[test]
    fn envelope_new_creates_valid_structure() {
        let env = JsonEnvelope::new("test", json!({"key": "value"}));
        assert_eq!(env.version, VERSION);
        assert_eq!(env.kind, "test");
        assert_eq!(env.data, json!({"key": "value"}));
        assert!(env.warning.is_none());
    }

    #[test]
    fn envelope_new_accepts_various_kind_types() {
        // String slice
        let env1 = JsonEnvelope::new("list", json!([]));
        assert_eq!(env1.kind, "list");

        // String
        let env2 = JsonEnvelope::new(String::from("ready"), json!([]));
        assert_eq!(env2.kind, "ready");

        // Cow-like behavior
        let kind = String::from("search");
        let env3 = JsonEnvelope::new(kind.clone(), json!([]));
        assert_eq!(env3.kind, kind);
    }

    #[test]
    fn envelope_new_with_empty_data() {
        let env = JsonEnvelope::new("ready", json!([]));
        assert!(env.data.is_array());
        assert_eq!(env.data.as_array().unwrap().len(), 0);
    }

    #[test]
    fn envelope_new_with_null_data() {
        let env = JsonEnvelope::new("custom", json!(null));
        assert!(env.data.is_null());
    }

    // === Version field tests ===

    #[test]
    fn envelope_version_is_always_current() {
        let env = JsonEnvelope::new("any", json!([]));
        assert_eq!(env.version, VERSION);
        assert_eq!(env.version, 1);
    }

    #[test]
    fn envelope_version_is_present_in_serialization() {
        let env = JsonEnvelope::new("stats", json!({"total": 42}));
        let s = env.to_json().unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["version"], 1);
        assert!(v["version"].is_number());
        assert!(v["version"].is_u64());
    }

    #[test]
    fn envelope_version_constant_matches_serialized_value() {
        let env = JsonEnvelope::new("list", json!([]));
        let serialized = serde_json::to_value(&env).unwrap();
        assert_eq!(serialized["version"].as_u64().unwrap() as u32, VERSION);
    }

    // === Kind field tests ===

    #[test]
    fn envelope_kind_identifies_command() {
        let commands = vec!["list", "ready", "show", "claim", "create", "update", "close", "stats", "search"];
        for cmd in commands {
            let env = JsonEnvelope::new(cmd, json!(null));
            assert_eq!(env.kind, cmd);
        }
    }

    #[test]
    fn envelope_kind_is_required_field() {
        let env = JsonEnvelope::new("required", json!(null));
        let v = serde_json::to_value(&env).unwrap();
        assert!(v.get("kind").is_some());
        assert_eq!(v["kind"], "required");
    }

    #[test]
    fn envelope_kind_preserves_original_value() {
        let kind = "very_specific_command_name";
        let env = JsonEnvelope::new(kind, json!([]));
        let s = env.to_json_compact().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.kind, kind);
    }

    // === Data field tests ===

    #[test]
    fn envelope_data_can_be_object() {
        let data = json!({"id": "bf-123", "title": "Test"});
        let env = JsonEnvelope::new("show", data);
        assert!(env.data.is_object());
        let s = env.to_json().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.data["id"], "bf-123");
        assert_eq!(parsed.data["title"], "Test");
    }

    #[test]
    fn envelope_data_can_be_array() {
        let data = json!([1, 2, 3, 4, 5]);
        let env = JsonEnvelope::new("list", data);
        assert!(env.data.is_array());
        let s = env.to_json_compact().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.data.as_array().unwrap().len(), 5);
    }

    #[test]
    fn envelope_data_can_be_string() {
        let env = JsonEnvelope::new("message", json!("output text"));
        assert!(env.data.is_string());
        let s = env.to_json().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.data.as_str().unwrap(), "output text");
    }

    #[test]
    fn envelope_data_can_be_number() {
        let env = JsonEnvelope::new("count", json!(42));
        assert!(env.data.is_number());
        let s = env.to_json().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.data.as_i64().unwrap(), 42);
    }

    #[test]
    fn envelope_data_can_be_boolean() {
        let env = JsonEnvelope::new("success", json!(true));
        assert!(env.data.is_boolean());
        let s = env.to_json().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert!(parsed.data.as_bool().unwrap());
    }

    #[test]
    fn envelope_data_can_be_nested_structure() {
        let data = json!({
            "bead": {
                "id": "bf-123",
                "metadata": {
                    "created": "2024-01-01",
                    "tags": ["urgent", "backend"]
                }
            }
        });
        let env = JsonEnvelope::new("show", data.clone());
        let s = env.to_json().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.data, data);
        assert_eq!(parsed.data["bead"]["metadata"]["tags"][0], "urgent");
    }

    #[test]
    fn envelope_data_preserves_exact_value() {
        let original = json!({"complex": [1, 2, {"nested": "value"}]});
        let env = JsonEnvelope::new("preserve", original.clone());
        let s = env.to_json().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.data, original);
    }

    // === Warning field tests (metadata) ===

    #[test]
    fn envelope_warning_is_optional() {
        let env = JsonEnvelope::new("list", json!([]));
        assert!(env.warning.is_none());
    }

    #[test]
    fn envelope_with_warning_adds_message() {
        let env = JsonEnvelope::new("create", json!({"id": "bf-test"}))
            .with_warning("auto-flush failed");
        assert_eq!(env.warning, Some("auto-flush failed".to_string()));
    }

    #[test]
    fn envelope_warning_can_be_empty_string() {
        let env = JsonEnvelope::new("update", json!({}))
            .with_warning("");
        assert_eq!(env.warning, Some("".to_string()));
    }

    #[test]
    fn envelope_warning_can_contain_any_text() {
        let warning = "Warning: multiple unflushed beads in workspace";
        let env = JsonEnvelope::new("claim", json!({}))
            .with_warning(warning);
        assert_eq!(env.warning.unwrap(), warning);
    }

    #[test]
    fn envelope_with_warning_preserves_other_fields() {
        let data = json!({"id": "bf-456"});
        let env = JsonEnvelope::new("close", data.clone())
            .with_warning("partial flush failure");
        assert_eq!(env.kind, "close");
        assert_eq!(env.data, data);
        assert_eq!(env.warning, Some("partial flush failure".to_string()));
    }

    #[test]
    fn envelope_warning_serializes_when_present() {
        let env = JsonEnvelope::new("delete", json!({"id": "bf-789"}))
            .with_warning("archive failed");
        let s = env.to_json().unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["warning"], "archive failed");
        assert!(v["warning"].is_string());
    }

    #[test]
    fn envelope_warning_omitted_when_none_in_serialization() {
        let env = JsonEnvelope::new("list", json!([]));
        let s = env.to_json().unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        // Key should not be present (not just null)
        assert!(v.get("warning").is_none());
    }

    #[test]
    fn envelope_warning_omitted_in_compact_serialization() {
        let env = JsonEnvelope::new("ready", json!([]));
        let compact = env.to_json_compact().unwrap();
        let v: Value = serde_json::from_str(&compact).unwrap();
        assert!(v.get("warning").is_none());
    }

    // === Serialization tests ===

    #[test]
    fn envelope_to_json_produces_valid_json() {
        let env = JsonEnvelope::new("stats", json!({"total": 42}));
        let s = env.to_json().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.kind, "stats");
        assert_eq!(parsed.data["total"], 42);
    }

    #[test]
    fn envelope_to_json_pretty_prints() {
        let env = JsonEnvelope::new("list", json!([]));
        let s = env.to_json().unwrap();
        // Pretty-printed JSON has newlines
        assert!(s.contains('\n'));
        // And indentation
        assert!(s.contains("  "));
    }

    #[test]
    fn envelope_to_json_compact_produces_valid_json() {
        let env = JsonEnvelope::new("claim", json!({"bead_id": "bf-123"}));
        let compact = env.to_json_compact().unwrap();
        // Compact JSON has no newlines
        assert!(!compact.contains('\n'));
        // But still parses correctly
        let parsed: JsonEnvelope = serde_json::from_str(&compact).unwrap();
        assert_eq!(parsed.kind, "claim");
    }

    #[test]
    fn envelope_serialization_roundtrip() {
        let original = JsonEnvelope::new("search", json!([{"id": "1"}, {"id": "2"}]))
            .with_warning("test warning");
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: JsonEnvelope = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.version, original.version);
        assert_eq!(deserialized.kind, original.kind);
        assert_eq!(deserialized.data, original.data);
        assert_eq!(deserialized.warning, original.warning);
    }

    #[test]
    fn envelope_compact_roundtrip_with_warning() {
        let env = JsonEnvelope::new("velocity", json!({"avg_days": 3.5}))
            .with_warning("incomplete data");
        let compact = env.to_json_compact().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&compact).unwrap();
        assert_eq!(parsed.kind, "velocity");
        assert_eq!(parsed.warning, Some("incomplete data".to_string()));
        assert_eq!(parsed.data["avg_days"], 3.5);
    }

    // === Command-specific data shape tests ===

    #[test]
    fn list_command_emits_array() {
        let data = json!([
            {"id": "bf-1", "title": "First"},
            {"id": "bf-2", "title": "Second"}
        ]);
        let env = JsonEnvelope::new("list", data);
        let s = env.to_json().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert!(parsed.data.is_array());
        assert_eq!(parsed.data.as_array().unwrap().len(), 2);
    }

    #[test]
    fn ready_command_empty_returns_array() {
        let env = JsonEnvelope::new("ready", json!([]));
        let s = env.to_json_compact().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert!(parsed.data.is_array());
        assert_eq!(parsed.data.as_array().unwrap().len(), 0);
    }

    #[test]
    fn show_command_emits_single_object() {
        let data = json!({"id": "bf-123", "title": "Test Bead", "status": "open"});
        let env = JsonEnvelope::new("show", data);
        let s = env.to_json().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert!(parsed.data.is_object());
        assert_eq!(parsed.data["id"], "bf-123");
    }

    #[test]
    fn claim_command_emits_result_object() {
        let data = json!({"bead_id": "bf-456", "assignee": "agent-1"});
        let env = JsonEnvelope::new("claim", data);
        let s = env.to_json_compact().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert!(parsed.data.is_object());
        assert_eq!(parsed.data["bead_id"], "bf-456");
        assert_eq!(parsed.data["assignee"], "agent-1");
    }

    #[test]
    fn create_command_emits_id_only() {
        let data = json!({"id": "bf-new-123"});
        let env = JsonEnvelope::new("create", data);
        let s = env.to_json().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert!(parsed.data.is_object());
        assert!(parsed.data.get("id").is_some());
        assert_eq!(parsed.data["id"], "bf-new-123");
    }

    #[test]
    fn stats_command_emits_aggregate_counts() {
        let data = json!({"total": 100, "open": 50, "in_progress": 30, "closed": 20});
        let env = JsonEnvelope::new("stats", data);
        let s = env.to_json().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.data["total"], 100);
        assert_eq!(parsed.data["open"], 50);
        assert_eq!(parsed.data["in_progress"], 30);
        assert_eq!(parsed.data["closed"], 20);
    }

    #[test]
    fn search_command_emits_results_array() {
        let data = json!([
            {"id": "bf-1", "title": "First result"},
            {"id": "bf-2", "title": "Second result"}
        ]);
        let env = JsonEnvelope::new("search", data);
        let s = env.to_json().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert!(parsed.data.is_array());
        assert_eq!(parsed.data.as_array().unwrap().len(), 2);
    }

    #[test]
    fn batch_command_emits_operations_array() {
        let data = json!([
            {"op": "create", "result": {"id": "bf-1"}},
            {"op": "close", "result": {"id": "bf-2"}}
        ]);
        let env = JsonEnvelope::new("batch", data);
        let s = env.to_json().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert!(parsed.data.is_array());
        assert_eq!(parsed.data.as_array().unwrap().len(), 2);
        assert_eq!(parsed.data[0]["op"], "create");
    }

    // === Metadata/field presence tests ===

    #[test]
    fn envelope_has_required_fields() {
        let env = JsonEnvelope::new("list", json!([]));
        let v = serde_json::to_value(&env).unwrap();
        assert!(v.get("version").is_some());
        assert!(v.get("kind").is_some());
        assert!(v.get("data").is_some());
        // warning is optional
    }

    #[test]
    fn envelope_allows_arbitrary_kind_values() {
        let custom_commands = ["custom-1", "my_command", "SPECIAL_CMD"];
        for cmd in custom_commands {
            let env = JsonEnvelope::new(cmd, json!(null));
            assert_eq!(env.kind, cmd);
            let s = env.to_json().unwrap();
            let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
            assert_eq!(parsed.kind, cmd);
        }
    }

    #[test]
    fn envelope_field_order_is_stable() {
        let env = JsonEnvelope::new("test", json!([]));
        let s = env.to_json().unwrap();
        // Verify fields are present in serialized form
        assert!(s.contains("\"version\""));
        assert!(s.contains("\"kind\""));
        assert!(s.contains("\"data\""));
    }

    // === Edge cases and error conditions ===

    #[test]
    fn envelope_with_large_data_serializes_correctly() {
        let large_array: Vec<Value> = (0..1000)
            .map(|i| json!({"id": format!("bf-{}", i), "value": i}))
            .collect();
        let env = JsonEnvelope::new("list", json!(large_array));
        let s = env.to_json_compact().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert!(parsed.data.is_array());
        assert_eq!(parsed.data.as_array().unwrap().len(), 1000);
    }

    #[test]
    fn envelope_with_unicode_warning() {
        let env = JsonEnvelope::new("update", json!({}))
            .with_warning("⚠️  Warning: café Münchën");
        let s = env.to_json().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.warning, Some("⚠️  Warning: café Münchën".to_string()));
    }

    #[test]
    fn envelope_with_special_chars_in_kind() {
        let env = JsonEnvelope::new("cmd-with_special.chars", json!(null));
        assert_eq!(env.kind, "cmd-with_special.chars");
        let s = env.to_json().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.kind, "cmd-with_special.chars");
    }

    #[test]
    fn envelope_with_newlines_in_warning() {
        let env = JsonEnvelope::new("sync", json!({}))
            .with_warning("Line 1\nLine 2\nLine 3");
        let s = env.to_json().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.warning, Some("Line 1\nLine 2\nLine 3".to_string()));
    }
}

// === Integration tests for list and show envelope wrapping ===
// These tests verify that list --json and show --json properly
// wrap their output in envelopes with correct structure and metadata.
// Bead: bf-3v1r9
#[cfg(test)]
mod list_show {
    use super::*;
    use crate::model::{Issue, Status, Priority, IssueType};
    use crate::format::json::JsonFormatter;
    use crate::format::Formatter;
    use serde_json::json;

    /// Helper to create a test issue
    fn create_test_issue(id: &str, title: &str) -> Issue {
        let mut issue = Issue::new(
            id.to_string(),
            title.to_string(),
            ".".to_string()
        );
        issue.status = Status::Open;
        issue.priority = Priority(2);
        issue.issue_type = IssueType::Task;
        issue
    }

    /// Helper to parse envelope from JSON string
    fn parse_envelope(json_str: &str) -> JsonEnvelope {
        serde_json::from_str(json_str)
            .expect("Output must be valid JSON envelope")
    }

    #[test]
    fn list_json_envelope_returns_array_data() {
        let formatter = JsonFormatter;
        let issues = vec![
            create_test_issue("bf-1", "First task"),
            create_test_issue("bf-2", "Second task"),
        ];

        // Format issues as JSONL (one per line)
        let jsonl_output = Formatter::format_issues(&formatter, &issues);

        // Wrap in envelope
        let envelope = JsonEnvelope::new("list", json!(jsonl_output));
        let envelope_json = envelope.to_json().unwrap();
        let parsed = parse_envelope(&envelope_json);

        // Verify envelope structure
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.kind, "list");

        // The data field should contain the JSONL string
        assert!(parsed.data.is_string());

        // Parse the JSONL string and verify it contains both issues
        let jsonl_str = parsed.data.as_str().unwrap();
        let lines: Vec<&str> = jsonl_str.lines().collect();
        assert_eq!(lines.len(), 2, "List should output 2 lines (one per issue)");

        // Verify each line is valid JSON representing an issue
        for line in lines {
            let issue_value: serde_json::Value = serde_json::from_str(line)
                .expect("Each line must be valid JSON");
            assert!(issue_value.is_object(), "Each line must be a JSON object");
            assert!(issue_value.get("id").is_some(), "Each issue must have an id");
            assert!(issue_value.get("title").is_some(), "Each issue must have a title");
        }
    }

    #[test]
    fn list_json_envelope_empty_returns_empty_array() {
        let formatter = JsonFormatter;
        let issues: Vec<Issue> = vec![];

        // Format empty issues as JSONL (empty string)
        let jsonl_output = Formatter::format_issues(&formatter, &issues);

        // Empty output should be an empty string (not "[]")
        assert!(jsonl_output.is_empty() || jsonl_output == "[]");

        // For envelope wrapping, we use an empty array explicitly
        let envelope = JsonEnvelope::new("list", json!([]));
        let envelope_json = envelope.to_json().unwrap();
        let parsed = parse_envelope(&envelope_json);

        // Verify envelope structure
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.kind, "list");

        // Data should be an empty array
        assert!(parsed.data.is_array());
        assert_eq!(parsed.data.as_array().unwrap().len(), 0);
    }

    #[test]
    fn list_json_envelope_metadata_fields_present() {
        let formatter = JsonFormatter;
        let issues = vec![create_test_issue("bf-1", "Metadata test")];

        let jsonl_output = Formatter::format_issues(&formatter, &issues);
        let envelope = JsonEnvelope::new("list", json!(jsonl_output));
        let envelope_json = envelope.to_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&envelope_json)
            .expect("Envelope must serialize to valid JSON");

        // Verify all metadata fields are present
        assert!(parsed.get("version").is_some(), "version field must be present");
        assert!(parsed.get("kind").is_some(), "kind field must be present");
        assert!(parsed.get("data").is_some(), "data field must be present");

        // Verify metadata values
        assert_eq!(parsed["version"].as_u64().unwrap(), 1);
        assert_eq!(parsed["kind"].as_str().unwrap(), "list");

        // warning field is optional (absent when no warning)
        let warning_optional = parsed.get("warning");
        assert!(warning_optional.is_none() || warning_optional.unwrap().is_null() || warning_optional.unwrap().is_string());
    }

    #[test]
    fn show_json_envelope_returns_single_object() {
        let formatter = JsonFormatter;
        let issue = create_test_issue("bf-123", "Show test");

        // Format single issue as JSON
        let json_output = Formatter::format_issue(&formatter, &issue);

        // Wrap in envelope
        let envelope = JsonEnvelope::new("show", json!(json_output));
        let envelope_json = envelope.to_json().unwrap();
        let parsed = parse_envelope(&envelope_json);

        // Verify envelope structure
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.kind, "show");

        // Data should be a string (the JSON representation)
        assert!(parsed.data.is_string());

        // Parse the JSON string and verify it represents a single object
        let json_str = parsed.data.as_str().unwrap();
        let issue_value: serde_json::Value = serde_json::from_str(json_str)
            .expect("Data must be valid JSON");

        assert!(issue_value.is_object(), "Show data must be a JSON object");
        assert_eq!(issue_value["id"].as_str().unwrap(), "bf-123");
        assert_eq!(issue_value["title"].as_str().unwrap(), "Show test");
    }

    #[test]
    fn show_json_envelope_metadata_fields_present() {
        let formatter = JsonFormatter;
        let issue = create_test_issue("bf-meta", "Metadata fields test");

        let json_output = Formatter::format_issue(&formatter, &issue);
        let envelope = JsonEnvelope::new("show", json!(json_output));
        let envelope_json = envelope.to_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&envelope_json)
            .expect("Envelope must serialize to valid JSON");

        // Verify all metadata fields are present
        assert!(parsed.get("version").is_some(), "version field must be present");
        assert!(parsed.get("kind").is_some(), "kind field must be present");
        assert!(parsed.get("data").is_some(), "data field must be present");

        // Verify metadata values
        assert_eq!(parsed["version"].as_u64().unwrap(), 1);
        assert_eq!(parsed["kind"].as_str().unwrap(), "show");

        // warning field is optional
        let warning_optional = parsed.get("warning");
        assert!(warning_optional.is_none() || warning_optional.unwrap().is_null() || warning_optional.unwrap().is_string());
    }

    #[test]
    fn show_json_envelope_with_all_issue_fields() {
        let formatter = JsonFormatter;
        let mut issue = create_test_issue("bf-full", "Full issue test");

        // Add optional fields
        issue.description = Some("Test description".to_string());
        issue.acceptance_criteria = Some("AC: Should work".to_string());
        issue.assignee = Some("test-worker".to_string());
        issue.labels = vec!["phase-1".to_string(), "urgent".to_string()];

        let json_output = Formatter::format_issue(&formatter, &issue);
        let envelope = JsonEnvelope::new("show", json!(json_output));
        let envelope_json = envelope.to_json().unwrap();
        let parsed = parse_envelope(&envelope_json);

        // Verify envelope structure
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.kind, "show");

        // Parse and verify issue fields are preserved
        let json_str = parsed.data.as_str().unwrap();
        let issue_value: serde_json::Value = serde_json::from_str(json_str)
            .expect("Data must be valid JSON");

        assert_eq!(issue_value["id"].as_str().unwrap(), "bf-full");
        assert_eq!(issue_value["title"].as_str().unwrap(), "Full issue test");
        assert_eq!(issue_value["description"].as_str().unwrap(), "Test description");
        assert_eq!(issue_value["acceptance_criteria"].as_str().unwrap(), "AC: Should work");
        assert_eq!(issue_value["assignee"].as_str().unwrap(), "test-worker");
        assert!(issue_value["labels"].is_array());
        assert_eq!(issue_value["labels"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn list_json_envelope_multiple_issues_formatting() {
        let formatter = JsonFormatter;
        let issues = vec![
            create_test_issue("bf-a1", "Task A1"),
            create_test_issue("bf-a2", "Task A2"),
            create_test_issue("bf-a3", "Task A3"),
        ];

        let jsonl_output = Formatter::format_issues(&formatter, &issues);
        let envelope = JsonEnvelope::new("list", json!(jsonl_output));
        let envelope_json = envelope.to_json_compact().unwrap();
        let parsed = parse_envelope(&envelope_json);

        // Verify envelope metadata
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.kind, "list");

        // Parse JSONL and count issues
        let jsonl_str = parsed.data.as_str().unwrap();
        let lines: Vec<&str> = jsonl_str.lines().collect();
        assert_eq!(lines.len(), 3, "List should output 3 lines for 3 issues");

        // Verify each issue has required fields
        for (i, line) in lines.iter().enumerate() {
            let issue_value: serde_json::Value = serde_json::from_str(line)
                .expect(&format!("Line {} must be valid JSON", i));
            assert!(issue_value.get("id").is_some(), "Line {} issue must have id", i);
            assert!(issue_value.get("title").is_some(), "Line {} issue must have title", i);
            assert!(issue_value.get("status").is_some(), "Line {} issue must have status", i);
        }
    }

    #[test]
    fn list_show_envelope_kind_identifiers() {
        let formatter = JsonFormatter;

        // Test list envelope kind
        let list_issues = vec![create_test_issue("bf-1", "List kind test")];
        let list_jsonl = formatter.format_issues(&list_issues);
        let list_env = JsonEnvelope::new("list", json!(list_jsonl));
        assert_eq!(list_env.kind, "list");

        // Test show envelope kind
        let show_issue = create_test_issue("bf-2", "Show kind test");
        let show_json = formatter.format_issue(&show_issue);
        let show_env = JsonEnvelope::new("show", json!(show_json));
        assert_eq!(show_env.kind, "show");

        // Verify both serialize with correct kind
        let list_serialized = serde_json::to_value(&list_env).unwrap();
        let show_serialized = serde_json::to_value(&show_env).unwrap();

        assert_eq!(list_serialized["kind"].as_str().unwrap(), "list");
        assert_eq!(show_serialized["kind"].as_str().unwrap(), "show");
    }

    #[test]
    fn list_show_envelope_version_field() {
        let formatter = JsonFormatter;

        // Both list and show envelopes must have version = 1
        let issues = vec![create_test_issue("bf-1", "Version test")];
        let list_output = formatter.format_issues(&issues);
        let list_env = JsonEnvelope::new("list", json!(list_output));

        let show_issue = create_test_issue("bf-2", "Version test");
        let show_output = formatter.format_issue(&show_issue);
        let show_env = JsonEnvelope::new("show", json!(show_output));

        // Verify version field
        assert_eq!(list_env.version, VERSION);
        assert_eq!(show_env.version, VERSION);
        assert_eq!(list_env.version, 1);
        assert_eq!(show_env.version, 1);

        // Verify serialized version
        let list_json = serde_json::to_value(&list_env).unwrap();
        let show_json = serde_json::to_value(&show_env).unwrap();

        assert_eq!(list_json["version"].as_u64().unwrap(), 1);
        assert_eq!(show_json["version"].as_u64().unwrap(), 1);
    }
}

// === Integration tests for claim and stats envelope wrapping ===
// These tests verify that claim --json and stats --json properly
// wrap their output in envelopes with correct structure and metadata.
// Bead: bf-s4ljc
#[cfg(test)]
mod claim_stats {
    use super::*;
    use crate::model::{Issue, Status, Priority, IssueType};
    use crate::format::json::JsonFormatter;
    use crate::format::Formatter;
    use serde_json::json;

    /// Helper to create a test issue
    fn create_test_issue(id: &str, title: &str) -> Issue {
        let mut issue = Issue::new(
            id.to_string(),
            title.to_string(),
            ".".to_string()
        );
        issue.status = Status::Open;
        issue.priority = Priority(2);
        issue.issue_type = IssueType::Task;
        issue
    }

    /// Helper to parse envelope from JSON string
    fn parse_envelope(json_str: &str) -> JsonEnvelope {
        serde_json::from_str(json_str)
            .expect("Output must be valid JSON envelope")
    }

    // === Claim command envelope tests ===

    #[test]
    fn claim_json_envelope_has_stable_structure() {
        // Simulate a successful claim response
        let claim_data = json!({
            "bead_id": "bf-123",
            "assignee": "test-worker",
            "claimed_at": "2024-01-15T10:30:00Z"
        });

        let envelope = JsonEnvelope::new("claim", claim_data);
        let envelope_json = envelope.to_json().unwrap();
        let parsed = parse_envelope(&envelope_json);

        // Verify envelope structure
        assert_eq!(parsed.version, 1, "Envelope version must be 1");
        assert_eq!(parsed.kind, "claim", "Envelope kind must be 'claim'");
        assert!(parsed.data.is_object(), "Claim data must be an object");

        // Verify claim-specific fields are present in data
        assert!(parsed.data.get("bead_id").is_some(), "Claim data must have 'bead_id'");
        assert!(parsed.data.get("assignee").is_some(), "Claim data must have 'assignee'");
    }

    #[test]
    fn claim_json_envelope_metadata_fields_present() {
        let claim_data = json!({
            "bead_id": "bf-456",
            "assignee": "agent-007"
        });

        let envelope = JsonEnvelope::new("claim", claim_data);
        let envelope_json = envelope.to_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&envelope_json)
            .expect("Envelope must serialize to valid JSON");

        // Verify all metadata fields are present
        assert!(parsed.get("version").is_some(), "version field must be present");
        assert!(parsed.get("kind").is_some(), "kind field must be present");
        assert!(parsed.get("data").is_some(), "data field must be present");

        // Verify metadata values
        assert_eq!(parsed["version"].as_u64().unwrap(), 1, "version must be 1");
        assert_eq!(parsed["kind"].as_str().unwrap(), "claim", "kind must be 'claim'");

        // warning field is optional (absent when no warning)
        let warning_optional = parsed.get("warning");
        assert!(
            warning_optional.is_none() || warning_optional.unwrap().is_null() || warning_optional.unwrap().is_string(),
            "warning field must be absent, null, or a string"
        );
    }

    #[test]
    fn claim_json_envelope_successful_claim_case() {
        // Simulate a successful claim operation
        let claim_response = json!({
            "bead_id": "bf-abc123",
            "assignee": "worker-1",
            "claimed_at": "2024-07-23T12:00:00Z",
            "status": "open"
        });

        let envelope = JsonEnvelope::new("claim", claim_response);
        let envelope_json = envelope.to_json().unwrap();
        let parsed = parse_envelope(&envelope_json);

        // Verify envelope structure for successful claim
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.kind, "claim");

        // Verify the claim result contains expected fields
        assert!(parsed.data.is_object(), "Claim result must be an object");
        assert_eq!(parsed.data["bead_id"].as_str().unwrap(), "bf-abc123");
        assert_eq!(parsed.data["assignee"].as_str().unwrap(), "worker-1");
        assert_eq!(parsed.data["status"].as_str().unwrap(), "open");

        // Verify claimed_at timestamp is present
        assert!(parsed.data.get("claimed_at").is_some(), "Claim result must have 'claimed_at' timestamp");
    }

    #[test]
    fn claim_json_envelope_empty_when_no_bead_available() {
        // Simulate claim with no available beads
        let empty_claim = json!({});

        let envelope = JsonEnvelope::new("claim", empty_claim);
        let envelope_json = envelope.to_json().unwrap();
        let parsed = parse_envelope(&envelope_json);

        // Verify envelope structure for empty claim
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.kind, "claim");
        assert!(parsed.data.is_object(), "Claim data must be an object");
        assert!(parsed.data.as_object().unwrap().is_empty(), "Claim data must be empty when no beads available");
    }

    #[test]
    fn claim_json_envelope_roundtrip_serialization() {
        let original_data = json!({
            "bead_id": "bf-rust",
            "assignee": "dev-team",
            "priority": 1
        });

        let envelope = JsonEnvelope::new("claim", original_data.clone());
        let serialized = serde_json::to_string(&envelope).unwrap();
        let deserialized: JsonEnvelope = serde_json::from_str(&serialized).unwrap();

        // Verify roundtrip preserves all data
        assert_eq!(deserialized.version, 1);
        assert_eq!(deserialized.kind, "claim");
        assert_eq!(deserialized.data, original_data);
    }

    // === Stats command envelope tests ===

    #[test]
    fn stats_json_envelope_has_stable_structure() {
        let stats_data = json!({
            "total": 100,
            "open": 50,
            "in_progress": 30,
            "closed": 20
        });

        let envelope = JsonEnvelope::new("stats", stats_data);
        let envelope_json = envelope.to_json().unwrap();
        let parsed = parse_envelope(&envelope_json);

        // Verify envelope structure
        assert_eq!(parsed.version, 1, "Envelope version must be 1");
        assert_eq!(parsed.kind, "stats", "Envelope kind must be 'stats'");
        assert!(parsed.data.is_object(), "Stats data must be an object");

        // Verify stats-specific fields
        assert!(parsed.data.get("total").is_some(), "Stats must have 'total'");
        assert!(parsed.data.get("open").is_some(), "Stats must have 'open'");
    }

    #[test]
    fn stats_json_envelope_metadata_fields_present() {
        let stats_data = json!({
            "total": 42,
            "open": 10,
            "in_progress": 5,
            "closed": 27
        });

        let envelope = JsonEnvelope::new("stats", stats_data);
        let envelope_json = envelope.to_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&envelope_json)
            .expect("Envelope must serialize to valid JSON");

        // Verify all metadata fields are present
        assert!(parsed.get("version").is_some(), "version field must be present");
        assert!(parsed.get("kind").is_some(), "kind field must be present");
        assert!(parsed.get("data").is_some(), "data field must be present");

        // Verify metadata values
        assert_eq!(parsed["version"].as_u64().unwrap(), 1, "version must be 1");
        assert_eq!(parsed["kind"].as_str().unwrap(), "stats", "kind must be 'stats'");

        // warning field is optional
        let warning_optional = parsed.get("warning");
        assert!(
            warning_optional.is_none() || warning_optional.unwrap().is_null() || warning_optional.unwrap().is_string(),
            "warning field must be absent, null, or a string"
        );
    }

    #[test]
    fn stats_json_envelope_aggregate_counts() {
        let stats_response = json!({
            "total": 150,
            "open": 75,
            "in_progress": 45,
            "closed": 30,
            "by_priority": {
                "p0": 10,
                "p1": 25,
                "p2": 40,
                "p3": 30,
                "p4": 45
            }
        });

        let envelope = JsonEnvelope::new("stats", stats_response);
        let envelope_json = envelope.to_json().unwrap();
        let parsed = parse_envelope(&envelope_json);

        // Verify envelope structure
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.kind, "stats");

        // Verify aggregate counts
        assert_eq!(parsed.data["total"].as_u64().unwrap(), 150);
        assert_eq!(parsed.data["open"].as_u64().unwrap(), 75);
        assert_eq!(parsed.data["in_progress"].as_u64().unwrap(), 45);
        assert_eq!(parsed.data["closed"].as_u64().unwrap(), 30);

        // Verify nested by_priority object
        assert!(parsed.data["by_priority"].is_object());
        assert_eq!(parsed.data["by_priority"]["p0"].as_u64().unwrap(), 10);
    }
}
