use crate::format::{text::format_stats_text, ClaimResultOutput, Formatter, StatsOutput};
use crate::model::{Dependency, Issue};
use crate::velocity::VelocityStats;

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

    fn format_stats(&self, stats: &StatsOutput) -> String {
        format_stats_text(stats)
    }

    fn format_velocity(&self, stats: &[VelocityStats]) -> String {
        let mut s = String::new();
        for stat in stats {
            s.push_str(&format!("Model: {}\n", stat.model));
            s.push_str(&format!("Harness: {}\n", stat.harness));
            s.push_str(&format!("Type: {}\n", stat.issue_type));
            s.push_str(&format!("Samples: {}\n", stat.sample_count));
            if let Some(p50) = stat.p50_seconds {
                s.push_str(&format!("P50: {}s\n", p50));
            }
            if let Some(p90) = stat.p90_seconds {
                s.push_str(&format!("P90: {}s\n", p90));
            }
            if let Some(avg) = stat.avg_seconds {
                s.push_str(&format!("Avg: {:.1}s\n", avg));
            }
            s.push('\n');
        }
        s
    }

    fn format_with_envelope(&self, _kind: &str, data: &str) -> String {
        // Toon formatter doesn't support envelope wrapping
        // Return the data as-is
        data.to_string()
    }

    fn format_with_envelope_and_warning(
        &self,
        _kind: &str,
        data: &str,
        _warning: Option<&str>,
    ) -> String {
        // Toon formatter doesn't support envelope wrapping
        // Return the data as-is
        data.to_string()
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

/// Format dependencies as a text string for display.
///
/// This is the same implementation as text::format_dependencies since
/// both text and toon formatters use the same dependency format.
pub fn format_dependencies(dependencies: &[Dependency]) -> String {
    crate::format::text::format_dependencies(dependencies)
}
