use crate::format::Formatter;
use crate::model::Issue;
use serde_json;

#[derive(Debug, Clone, Copy)]
pub struct JsonFormatter;

impl Formatter for JsonFormatter {
    fn format_issue(&self, issue: &Issue) -> String {
        // Clone and strip dependencies/comments for br compatibility
        let mut stripped = issue.clone();
        stripped.dependencies = vec![];
        stripped.comments = vec![];
        serde_json::to_string(&stripped).unwrap_or_else(|_| "{}".to_string())
    }

    fn format_issues(&self, issues: &[Issue]) -> String {
        issues
            .iter()
            .map(|issue| {
                let mut stripped = issue.clone();
                stripped.dependencies = vec![];
                stripped.comments = vec![];
                serde_json::to_string(&stripped)
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_default()
            .join("\n")
    }

    fn format_error(&self, message: &str) -> String {
        serde_json::json!({"error": message}).to_string()
    }
}
