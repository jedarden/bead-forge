pub mod color;
pub mod envelope;
pub mod json;
pub mod table;
pub mod text;
pub mod toon;
pub mod warning;

pub use color::{status_color, Color, format_status_colored, colorize};
pub use envelope::{JsonEnvelope, VERSION as ENVELOPE_VERSION};
pub use json::JsonFormatter;
pub use table::TableFormatter;
pub use text::{format_dependencies, format_dependencies_display, TextFormatter};
pub use toon::ToonFormatter;
pub use warning::{warn_stderr, with_warning};

use crate::model::Issue;
use crate::velocity::VelocityStats;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// JSON/serializable projection of a `claim` result.
///
/// `claim` emits a single object — never an `Issue` and never an array — that
/// mixes fields from `ScoredBead` (dry-run preview), `ClaimResult`
/// (`reclaimed`/`workspace`), and the caller's `assignee`. Only `bead_id` and
/// `assignee` are always present; every other field is optional and omitted
/// (via `skip_serializing_if`) when unset, so each claim branch emits exactly
/// the keys that apply to it. This is the shape the `Formatter` trait renders
/// for `claim`, the same way `format_issues` renders `&[Issue]` for
/// `list`/`ready`/`search`/`recent`.
#[derive(Debug, Clone, Serialize)]
pub struct ClaimResultOutput {
    pub bead_id: String,
    pub assignee: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reclaimed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downstream_impact: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
}

impl ClaimResultOutput {
    pub fn new(bead_id: impl Into<String>, assignee: impl Into<String>) -> Self {
        Self {
            bead_id: bead_id.into(),
            assignee: assignee.into(),
            reclaimed: None,
            workspace: None,
            title: None,
            priority: None,
            downstream_impact: None,
            dry_run: None,
        }
    }
}

/// JSON/serializable projection of a `stats` result.
///
/// `stats` emits a single object — never an `Issue` and never an array —
/// with the four aggregate counts and, optionally, breakdowns folded in as
/// nested maps. Only the four count fields are always present; each
/// breakdown is `Option` and omitted (via `skip_serializing_if`) when the
/// caller did not request it (`bf stats --by-type`, …). This is the shape
/// the `Formatter` trait renders for `stats`, the same way
/// `format_claim_result` renders `ClaimResultOutput` for `claim`.
///
/// Folding the breakdowns into the object (rather than appending them as
/// plain text after it) is what keeps `bf stats --format json --by-type`
/// valid JSON — the prior implementation printed the JSON object followed by
/// human-readable text, so the combined stdout could not be parsed.
///
/// Breakdown keys are strings because JSON object keys must be strings:
/// `by_priority` uses the raw priority number (`"0"`, `"1"`, …) and
/// `by_assignee` uses `"None"` for the unassigned bucket, matching the text
/// view. `BTreeMap` gives deterministic, sorted key order.
#[derive(Debug, Clone, Serialize)]
pub struct StatsOutput {
    pub total: usize,
    pub open: usize,
    pub in_progress: usize,
    pub closed: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_type: Option<BTreeMap<String, i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_priority: Option<BTreeMap<String, i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_assignee: Option<BTreeMap<String, i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_label: Option<BTreeMap<String, i64>>,
}

impl StatsOutput {
    pub fn new(total: usize, open: usize, in_progress: usize, closed: usize) -> Self {
        Self {
            total,
            open,
            in_progress,
            closed,
            by_type: None,
            by_priority: None,
            by_assignee: None,
            by_label: None,
        }
    }
}

/// JSON/serializable projection of labels output.
///
/// Represents the label output shape: `{"id": "...", "labels": ["label1", "label2"]}`.
/// Both `id` and `labels` are always present — `labels` is an empty array when no
/// labels are set, not omitted. This is the shape for formatting label data
/// across commands that need to emit bead IDs with their associated labels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelsOutput {
    pub id: String,
    pub labels: Vec<String>,
}

impl LabelsOutput {
    /// Create a new LabelsOutput from an ID and labels vector.
    pub fn new(id: impl Into<String>, labels: Vec<String>) -> Self {
        Self {
            id: id.into(),
            labels,
        }
    }

    /// Create a LabelsOutput from an Issue reference.
    pub fn from_issue(issue: &Issue) -> Self {
        Self {
            id: issue.id.clone(),
            labels: issue.labels.clone(),
        }
    }

    /// Format as a JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Format as a pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

pub trait Formatter {
    fn format_issue(&self, issue: &Issue) -> String;
    fn format_issues(&self, issues: &[Issue]) -> String;
    fn format_error(&self, message: &str) -> String;
    /// Render a single `claim` result object (see `ClaimResultOutput`).
    fn format_claim_result(&self, result: &ClaimResultOutput) -> String;
    /// Render the "no beads available" outcome of `claim` — `{}` for JSON,
    /// a human message for text/toon.
    fn format_no_claim(&self) -> String;
    /// Render a single `stats` result object (see `StatsOutput`).
    fn format_stats(&self, stats: &StatsOutput) -> String;
    /// Render velocity statistics — a JSON array for JSON, a table for text,
    /// a per-stat block for toon (mirrors how `format_stats` renders `StatsOutput`).
    fn format_velocity(&self, stats: &[VelocityStats]) -> String;
    /// Format data with envelope wrapping (JSON formatters only).
    /// Text and Toon formatters return the data as-is.
    fn format_with_envelope(&self, kind: &str, data: &str) -> String;
    /// Format data with envelope wrapping and optional warning (JSON formatters only).
    /// Text and Toon formatters return the data as-is.
    fn format_with_envelope_and_warning(
        &self,
        kind: &str,
        data: &str,
        warning: Option<&str>,
    ) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    Toon,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "text" => Some(OutputFormat::Text),
            "json" => Some(OutputFormat::Json),
            "toon" => Some(OutputFormat::Toon),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            OutputFormat::Text => "text",
            OutputFormat::Json => "json",
            OutputFormat::Toon => "toon",
        }
    }
}

pub fn get_formatter(format: OutputFormat) -> Box<dyn Formatter> {
    match format {
        OutputFormat::Text => Box::new(TextFormatter),
        OutputFormat::Json => Box::new(JsonFormatter),
        OutputFormat::Toon => Box::new(ToonFormatter),
    }
}

#[cfg(test)]
mod labels_output_tests {
    use super::*;
    use serde_json::Value;

    fn parse_json(json_str: &str) -> Value {
        serde_json::from_str(json_str).expect("LabelsOutput must produce valid JSON")
    }

    #[test]
    fn test_labels_output_basic_format() {
        let output = LabelsOutput::new("bf-test".to_string(), vec!["urgent".to_string(), "backend".to_string()]);
        let json = output.to_json();
        let parsed = parse_json(&json);

        assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("bf-test"));
        assert_eq!(
            parsed.get("labels").and_then(|v| v.as_array()),
            Some(&vec![
                serde_json::Value::String("urgent".to_string()),
                serde_json::Value::String("backend".to_string())
            ])
        );
    }

    #[test]
    fn test_labels_output_empty_list() {
        let output = LabelsOutput::new("bf-empty".to_string(), vec![]);
        let json = output.to_json();
        let parsed = parse_json(&json);

        assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("bf-empty"));
        assert_eq!(
            parsed.get("labels").and_then(|v| v.as_array()),
            Some(&vec![])
        );
        // Empty array should be present, not omitted
        assert!(parsed.get("labels").is_some());
    }

    #[test]
    fn test_labels_output_single_label() {
        let output = LabelsOutput::new("bf-single".to_string(), vec!["priority".to_string()]);
        let json = output.to_json();
        let parsed = parse_json(&json);

        assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("bf-single"));
        assert_eq!(
            parsed.get("labels").and_then(|v| v.as_array()),
            Some(&vec![serde_json::Value::String("priority".to_string())])
        );
    }

    #[test]
    fn test_labels_output_from_issue() {
        let mut issue = Issue::new("bf-from-issue".to_string(), "Test Issue".to_string(), ".".to_string());
        issue.labels = vec!["phase-1".to_string(), "model".to_string()];

        let output = LabelsOutput::from_issue(&issue);
        let json = output.to_json();
        let parsed = parse_json(&json);

        assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("bf-from-issue"));
        assert_eq!(
            parsed.get("labels").and_then(|v| v.as_array()),
            Some(&vec![
                serde_json::Value::String("phase-1".to_string()),
                serde_json::Value::String("model".to_string())
            ])
        );
    }

    #[test]
    fn test_labels_output_special_characters() {
        let output = LabelsOutput::new(
            "bf-special".to_string(),
            vec![
                "label-with-dashes".to_string(),
                "label_with_underscores".to_string(),
                "label.with.dots".to_string(),
                "label:with:colons".to_string(),
                "label/with/slashes".to_string(),
                "label with spaces".to_string(),
            ]
        );
        let json = output.to_json();
        let parsed = parse_json(&json);

        assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("bf-special"));

        let labels = parsed.get("labels").and_then(|v| v.as_array()).unwrap();
        assert_eq!(labels.len(), 6);
        assert_eq!(labels[0].as_str(), Some("label-with-dashes"));
        assert_eq!(labels[1].as_str(), Some("label_with_underscores"));
        assert_eq!(labels[2].as_str(), Some("label.with.dots"));
        assert_eq!(labels[3].as_str(), Some("label:with:colons"));
        assert_eq!(labels[4].as_str(), Some("label/with/slashes"));
        assert_eq!(labels[5].as_str(), Some("label with spaces"));
    }

    #[test]
    fn test_labels_output_unicode_characters() {
        let output = LabelsOutput::new(
            "bf-unicode".to_string(),
            vec![
                "日本語".to_string(),
                "العربية".to_string(),
                "emoji😀🎉".to_string(),
                "czech-čřž".to_string(),
            ]
        );
        let json = output.to_json();
        let parsed = parse_json(&json);

        assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("bf-unicode"));

        let labels = parsed.get("labels").and_then(|v| v.as_array()).unwrap();
        assert_eq!(labels.len(), 4);
        assert_eq!(labels[0].as_str(), Some("日本語"));
        assert_eq!(labels[1].as_str(), Some("العربية"));
        assert_eq!(labels[2].as_str(), Some("emoji😀🎉"));
        assert_eq!(labels[3].as_str(), Some("czech-čřž"));
    }

    #[test]
    fn test_labels_output_json_pretty() {
        let output = LabelsOutput::new("bf-pretty".to_string(), vec!["label1".to_string(), "label2".to_string()]);
        let json_pretty = output.to_json_pretty();

        // Pretty JSON should contain newlines and indentation
        assert!(json_pretty.contains('\n'));
        assert!(json_pretty.contains("  \""));

        let parsed = parse_json(&json_pretty);
        assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("bf-pretty"));
    }

    #[test]
    fn test_labels_output_valid_json_structure() {
        let output = LabelsOutput::new("bf-valid".to_string(), vec!["test".to_string()]);
        let json = output.to_json();

        // Should be valid JSON that can be parsed
        let parsed: Value = serde_json::from_str(&json).expect("must be valid JSON");

        // Should be an object with two keys
        assert!(parsed.is_object());
        assert_eq!(parsed.as_object().unwrap().len(), 2);
        assert!(parsed.get("id").is_some());
        assert!(parsed.get("labels").is_some());

        // id should be a string
        assert!(parsed.get("id").unwrap().is_string());

        // labels should be an array
        assert!(parsed.get("labels").unwrap().is_array());
    }

    #[test]
    fn test_labels_output_from_issue_with_empty_labels() {
        let issue = Issue::new("bf-empty-labels".to_string(), "Test Issue".to_string(), ".".to_string());
        // Don't add any labels

        let output = LabelsOutput::from_issue(&issue);
        let json = output.to_json();
        let parsed = parse_json(&json);

        assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("bf-empty-labels"));
        assert_eq!(
            parsed.get("labels").and_then(|v| v.as_array()),
            Some(&vec![])
        );
    }

    #[test]
    fn test_labels_output_escaped_characters() {
        let output = LabelsOutput::new(
            "bf-escape".to_string(),
            vec![
                "label\"quote".to_string(),
                "label\\backslash".to_string(),
                "label\nnewline".to_string(),
                "label\ttab".to_string(),
            ]
        );
        let json = output.to_json();
        let parsed = parse_json(&json);

        let labels = parsed.get("labels").and_then(|v| v.as_array()).unwrap();
        assert_eq!(labels.len(), 4);
        assert_eq!(labels[0].as_str(), Some("label\"quote"));
        assert_eq!(labels[1].as_str(), Some("label\\backslash"));
        assert_eq!(labels[2].as_str(), Some("label\nnewline"));
        assert_eq!(labels[3].as_str(), Some("label\ttab"));
    }

    #[test]
    fn test_labels_output_many_labels() {
        let many_labels: Vec<String> = (0..50).map(|i| format!("label-{}", i)).collect();
        let output = LabelsOutput::new("bf-many".to_string(), many_labels.clone());
        let json = output.to_json();
        let parsed = parse_json(&json);

        let labels = parsed.get("labels").and_then(|v| v.as_array()).unwrap();
        assert_eq!(labels.len(), 50);

        for (i, label) in labels.iter().enumerate() {
            assert_eq!(label.as_str(), Some(many_labels[i].as_str()));
        }
    }

    #[test]
    fn test_labels_output_long_label_names() {
        let long_label = "a".repeat(1000);
        let output = LabelsOutput::new("bf-long".to_string(), vec![long_label.clone()]);
        let json = output.to_json();
        let parsed = parse_json(&json);

        assert_eq!(
            parsed.get("labels").and_then(|v| v.as_array()).map(|arr| arr[0].as_str().unwrap().len()),
            Some(1000)
        );
    }

    #[test]
    fn test_labels_output_consistency_between_to_json_and_from_issue() {
        let mut issue = Issue::new("bf-consistency".to_string(), "Test".to_string(), ".".to_string());
        issue.labels = vec!["label1".to_string(), "label2".to_string()];

        let from_issue = LabelsOutput::from_issue(&issue);
        let manual = LabelsOutput::new("bf-consistency".to_string(), vec!["label1".to_string(), "label2".to_string()]);

        assert_eq!(from_issue.to_json(), manual.to_json());
    }

    #[test]
    fn test_labels_output_serialization_roundtrip() {
        let original = LabelsOutput::new("bf-roundtrip".to_string(), vec!["label1".to_string(), "label2".to_string()]);
        let json = original.to_json();
        let deserialized: LabelsOutput = serde_json::from_str(&json).expect("must deserialize");

        assert_eq!(original.id, deserialized.id);
        assert_eq!(original.labels, deserialized.labels);
    }
}
