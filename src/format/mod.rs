pub mod json;
pub mod text;
pub mod toon;
pub mod warning;

pub use json::JsonFormatter;
pub use text::TextFormatter;
pub use toon::ToonFormatter;
pub use warning::{warn_stderr, with_warning};

use crate::model::Issue;
use serde::Serialize;

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

pub trait Formatter {
    fn format_issue(&self, issue: &Issue) -> String;
    fn format_issues(&self, issues: &[Issue]) -> String;
    fn format_error(&self, message: &str) -> String;
    /// Render a single `claim` result object (see `ClaimResultOutput`).
    fn format_claim_result(&self, result: &ClaimResultOutput) -> String;
    /// Render the "no beads available" outcome of `claim` — `{}` for JSON,
    /// a human message for text/toon.
    fn format_no_claim(&self) -> String;
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
