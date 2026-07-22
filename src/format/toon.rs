use crate::format::{ClaimResultOutput, Formatter};
use crate::model::Issue;

#[derive(Debug, Clone, Copy)]
pub struct ToonFormatter;

impl Formatter for ToonFormatter {
    fn format_issue(&self, issue: &Issue) -> String {
        let mut parts = vec![
            format!("ID: {}", issue.id),
            format!("Title: {}", issue.title),
            format!("Status: {}", issue.status),
            format!("Priority: {}", issue.priority),
            format!("Type: {}", issue.issue_type),
        ];

        if let Some(desc) = &issue.description {
            parts.push(format!("Description: {}", desc));
        }
        if let Some(assignee) = &issue.assignee {
            parts.push(format!("Assignee: {}", assignee));
        }
        if !issue.labels.is_empty() {
            parts.push(format!("Labels: {}", issue.labels.join(", ")));
        }

        parts.join("\n")
    }

    fn format_issues(&self, issues: &[Issue]) -> String {
        let mut s = String::new();
        for issue in issues {
            s.push_str(&format_toon_issue_line(issue));
            s.push('\n');
        }
        s
    }

    fn format_error(&self, message: &str) -> String {
        format!("Error: {}\n", message)
    }

    fn format_claim_result(&self, result: &ClaimResultOutput) -> String {
        if result.dry_run == Some(true) {
            format!(
                "{} (priority={}, impact={}, workspace={})",
                result.bead_id,
                result.priority.unwrap_or(0),
                result.downstream_impact.unwrap_or(0),
                result.workspace.as_deref().unwrap_or(""),
            )
        } else if let Some(workspace) = &result.workspace {
            format!("{} (workspace: {})", result.bead_id, workspace)
        } else {
            result.bead_id.clone()
        }
    }

    fn format_no_claim(&self) -> String {
        "No beads available to claim".to_string()
    }
}

fn format_toon_issue_line(issue: &Issue) -> String {
    format!(
        "[{}] {} - {} ({})",
        issue.id,
        issue.title,
        issue.status,
        format_priority(issue.priority.0)
    )
}

fn format_priority(p: i32) -> String {
    format!("P{}", p)
}

pub fn format_ready_bead(id: &str, title: &str, priority: i32, impact: i64, float: f64) -> String {
    format!(
        "[{}] {} (priority={}, impact={}, float={})",
        id, title, priority, impact, float
    )
}
