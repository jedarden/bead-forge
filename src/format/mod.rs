pub mod envelope;
pub mod json;
pub mod text;
pub mod toon;
pub mod warning;

pub use envelope::{JsonEnvelope, VERSION as ENVELOPE_VERSION};
pub use json::JsonFormatter;
pub use text::{format_dependencies, format_dependencies_display, TextFormatter};
pub use toon::ToonFormatter;
pub use warning::{warn_stderr, with_warning};

use crate::model::Issue;
use crate::velocity::VelocityStats;
use serde::Serialize;
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
