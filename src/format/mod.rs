pub mod json;
pub mod text;
pub mod toon;

pub use json::JsonFormatter;
pub use text::TextFormatter;
pub use toon::ToonFormatter;

use crate::model::Issue;

pub trait Formatter {
    fn format_issue(&self, issue: &Issue) -> String;
    fn format_issues(&self, issues: &[Issue]) -> String;
    fn format_error(&self, message: &str) -> String;
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
