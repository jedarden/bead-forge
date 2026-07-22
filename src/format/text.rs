use crate::format::{ClaimResultOutput, Formatter};
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

    fn format_claim_result(&self, result: &ClaimResultOutput) -> String {
        if result.dry_run == Some(true) {
            // dry-run preview: "{id} (priority=N, impact=N, workspace=PATH)"
            format!(
                "{} (priority={}, impact={}, workspace={})",
                result.bead_id,
                result.priority.unwrap_or(0),
                result.downstream_impact.unwrap_or(0),
                result.workspace.as_deref().unwrap_or(""),
            )
        } else if let Some(workspace) = &result.workspace {
            // cross-workspace claim: "{id} (workspace: PATH)"
            format!("{} (workspace: {})", result.bead_id, workspace)
        } else {
            // single-workspace claim: just the id
            result.bead_id.clone()
        }
    }

    fn format_no_claim(&self) -> String {
        "No beads available to claim".to_string()
    }
}
