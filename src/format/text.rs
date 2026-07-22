use crate::format::{ClaimResultOutput, Formatter, StatsOutput};
use crate::model::Issue;
use crate::velocity::VelocityStats;

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

    fn format_stats(&self, stats: &StatsOutput) -> String {
        format_stats_text(stats)
    }

    fn format_velocity(&self, stats: &[VelocityStats]) -> String {
        format_velocity_text(stats)
    }
}

/// Human-readable rendering of a `stats` result shared by the text and toon
/// formatters — toon has no distinct art for aggregate counts, so it renders
/// the same lines as text (matching how `velocity --format toon` behaves).
pub fn format_stats_text(stats: &StatsOutput) -> String {
    let mut s = String::new();
    s.push_str(&format!("Total beads: {}\n", stats.total));
    s.push_str(&format!("  Open: {}\n", stats.open));
    s.push_str(&format!("  In Progress: {}\n", stats.in_progress));
    s.push_str(&format!("  Closed: {}\n", stats.closed));

    if let Some(by_type) = &stats.by_type {
        s.push_str("\nBy type:\n");
        for (issue_type, count) in by_type {
            s.push_str(&format!("  {} ({})\n", issue_type, count));
        }
    }
    if let Some(by_priority) = &stats.by_priority {
        s.push_str("\nBy priority:\n");
        for (priority, count) in by_priority {
            s.push_str(&format!("  P{} ({})\n", priority, count));
        }
    }
    if let Some(by_assignee) = &stats.by_assignee {
        s.push_str("\nBy assignee:\n");
        if by_assignee.is_empty() {
            s.push_str("  (no assigned beads)\n");
        } else {
            for (assignee, count) in by_assignee {
                s.push_str(&format!("  {} ({})\n", assignee, count));
            }
        }
    }
    if let Some(by_label) = &stats.by_label {
        s.push_str("\nBy label:\n");
        for (label, count) in by_label {
            s.push_str(&format!("  {} ({})\n", label, count));
        }
    }

    s
}

/// Human-readable rendering of velocity statistics.
///
/// Emits the fixed-width table the `velocity` command has always printed
/// (header + 85-char rule + one row per `(model, harness, issue_type)` cohort),
/// or the two-line "no statistics yet" message when there are no cohorts. This
/// is the text shape the `Formatter` trait renders for `velocity`, the same way
/// `format_stats_text` renders `StatsOutput` for `stats`.
pub fn format_velocity_text(stats: &[VelocityStats]) -> String {
    let mut s = String::new();
    if stats.is_empty() {
        s.push_str("No velocity statistics available yet.\n");
        s.push_str("Velocity data accumulates as beads are claimed and closed.\n");
        return s;
    }

    s.push_str("Velocity Statistics:\n");
    s.push('\n');
    s.push_str(&format!(
        "{:<20} {:<15} {:<10} {:<8} {:<8} {:<8} {:<8}\n",
        "Model", "Harness", "Type", "Samples", "P50(s)", "P90(s)", "Avg(s)"
    ));
    s.push_str(&format!("{}\n", "-".repeat(85)));
    for stat in stats {
        s.push_str(&format!(
            "{:<20} {:<15} {:<10} {:<8} {:<8} {:<8} {:<8.1}\n",
            stat.model,
            stat.harness,
            stat.issue_type,
            stat.sample_count,
            stat.p50_seconds
                .map(|s| s.to_string())
                .unwrap_or_else(|| "-".to_string()),
            stat.p90_seconds
                .map(|s| s.to_string())
                .unwrap_or_else(|| "-".to_string()),
            stat.avg_seconds.unwrap_or(0.0),
        ));
    }

    s
}
