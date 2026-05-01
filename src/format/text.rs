use crate::format::Formatter;
use crate::model::Issue;

#[derive(Debug, Clone, Copy)]
pub struct TextFormatter;

impl Formatter for TextFormatter {
    fn format_issue(&self, issue: &Issue) -> String {
        let mut s = String::new();
        s.push_str(&format!("ID: {}\n", issue.id));
        s.push_str(&format!("Title: {}\n", issue.title));
        s.push_str(&format!("Status: {}\n", issue.status));
        s.push_str(&format!("Priority: {}\n", issue.priority));
        s.push_str(&format!("Type: {}\n", issue.issue_type));

        if let Some(desc) = &issue.description {
            s.push_str(&format!("Description: {}\n", desc));
        }
        if let Some(assignee) = &issue.assignee {
            s.push_str(&format!("Assignee: {}\n", assignee));
        }
        if !issue.labels.is_empty() {
            s.push_str(&format!("Labels: {}\n", issue.labels.join(", ")));
        }

        s
    }

    fn format_issues(&self, issues: &[Issue]) -> String {
        let mut s = String::new();
        for issue in issues {
            s.push_str(&format!(
                "[{}] {} - {} ({})\n",
                issue.id, issue.title, issue.status, issue.priority
            ));
        }
        s
    }

    fn format_error(&self, message: &str) -> String {
        format!("Error: {}\n", message)
    }
}
