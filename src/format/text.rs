use crate::format::{ClaimResultOutput, Formatter, StatsOutput};
use crate::model::{Dependency, Issue};
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
        s.push_str(&format!("Created at: {}\n", issue.created_at.format("%Y-%m-%d %H:%M:%S UTC")));
        s.push_str(&format!("Updated at: {}\n", issue.updated_at.format("%Y-%m-%d %H:%M:%S UTC")));
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

    fn format_with_envelope(&self, _kind: &str, data: &str) -> String {
        // Text formatter doesn't support envelope wrapping
        // Return the data as-is
        data.to_string()
    }

    fn format_with_envelope_and_warning(
        &self,
        _kind: &str,
        data: &str,
        _warning: Option<&str>,
    ) -> String {
        // Text formatter doesn't support envelope wrapping
        // Return the data as-is
        data.to_string()
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

/// Format dependencies as a text string for display.
///
/// # Arguments
/// * `dependencies` - Slice of Dependency objects to format
///
/// # Returns
/// A formatted string in the format "Depends: bf-xxx (Title) (blocks), bf-yyy (Title)"
/// or an empty string if there are no dependencies. The "(blocks)" suffix is only
/// added for blocking dependency types (those that affect ready work).
///
/// # Examples
/// ```
/// // Blocking and non-blocking dependencies
/// // Output: "Depends: bf-abc (Some task) (blocks), bf-def (Another task)"
/// ```
pub fn format_dependencies(dependencies: &[Dependency]) -> String {
    if dependencies.is_empty() {
        return String::new();
    }

    let parts: Vec<String> = dependencies
        .iter()
        .map(|dep| {
            let title = dep.title.as_deref().unwrap_or("Unknown");
            if dep.dep_type.is_blocking() {
                format!("{} ({}) (blocks)", dep.depends_on_id, title)
            } else {
                format!("{} ({})", dep.depends_on_id, title)
            }
        })
        .collect();

    format!("Depends: {}", parts.join(", "))
}

/// Format dependencies from storage DependencyDisplay results.
///
/// This is a convenience function that formats dependencies loaded from storage
/// via `get_dependencies_display()`, which includes bead titles from a JOIN
/// with the issues table. The DependencyDisplay format includes:
/// - dep_type: The dependency type (e.g., "blocks", "related")
/// - bead_id: The ID of the dependency bead
/// - title: The title of the dependency bead
///
/// # Arguments
/// * `dependencies` - Slice of DependencyDisplay objects from storage
///
/// # Returns
/// A formatted string in the format "Depends: bf-xxx (Title) (blocks), bf-yyy (Title)"
/// or an empty string if there are no dependencies.
pub fn format_dependencies_display(dependencies: &[crate::storage::sqlite::DependencyDisplay]) -> String {
    if dependencies.is_empty() {
        return String::new();
    }

    let parts: Vec<String> = dependencies
        .iter()
        .map(|dep| {
            if dep.dep_type == "blocks" {
                format!("{} ({}) (blocks)", dep.bead_id, dep.title)
            } else {
                format!("{} ({})", dep.bead_id, dep.title)
            }
        })
        .collect();

    format!("Depends: {}", parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Dependency, DependencyType};
    use chrono::Utc;

    #[test]
    fn test_format_dependencies_empty() {
        let deps: Vec<Dependency> = vec![];
        let result = format_dependencies(&deps);
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_dependencies_blocking() {
        let deps = vec![Dependency {
            issue_id: "bf-parent".to_string(),
            depends_on_id: "bf-blocker".to_string(),
            dep_type: DependencyType::Blocks,
            created_at: Utc::now(),
            created_by: None,
            metadata: None,
            thread_id: None,
            title: Some("Blocker task".to_string()),
        }];

        let result = format_dependencies(&deps);
        assert_eq!(result, "Depends: bf-blocker (Blocker task) (blocks)");
    }

    #[test]
    fn test_format_dependencies_non_blocking() {
        let deps = vec![Dependency {
            issue_id: "bf-parent".to_string(),
            depends_on_id: "bf-related".to_string(),
            dep_type: DependencyType::Related,
            created_at: Utc::now(),
            created_by: None,
            metadata: None,
            thread_id: None,
            title: Some("Related task".to_string()),
        }];

        let result = format_dependencies(&deps);
        assert_eq!(result, "Depends: bf-related (Related task)");
    }

    #[test]
    fn test_format_dependencies_mixed() {
        let deps = vec![
            Dependency {
                issue_id: "bf-parent".to_string(),
                depends_on_id: "bf-blocker".to_string(),
                dep_type: DependencyType::Blocks,
                created_at: Utc::now(),
                created_by: None,
                metadata: None,
                thread_id: None,
                title: Some("Blocker task".to_string()),
            },
            Dependency {
                issue_id: "bf-parent".to_string(),
                depends_on_id: "bf-related".to_string(),
                dep_type: DependencyType::Related,
                created_at: Utc::now(),
                created_by: None,
                metadata: None,
                thread_id: None,
                title: Some("Related task".to_string()),
            },
        ];

        let result = format_dependencies(&deps);
        assert_eq!(result, "Depends: bf-blocker (Blocker task) (blocks), bf-related (Related task)");
    }

    #[test]
    fn test_format_dependencies_unknown_title() {
        let deps = vec![Dependency {
            issue_id: "bf-parent".to_string(),
            depends_on_id: "bf-unknown".to_string(),
            dep_type: DependencyType::Blocks,
            created_at: Utc::now(),
            created_by: None,
            metadata: None,
            thread_id: None,
            title: None,
        }];

        let result = format_dependencies(&deps);
        assert_eq!(result, "Depends: bf-unknown (Unknown) (blocks)");
    }

    #[test]
    fn test_format_dependencies_multiple_blocking() {
        let deps = vec![
            Dependency {
                issue_id: "bf-parent".to_string(),
                depends_on_id: "bf-blocker1".to_string(),
                dep_type: DependencyType::Blocks,
                created_at: Utc::now(),
                created_by: None,
                metadata: None,
                thread_id: None,
                title: Some("First blocker".to_string()),
            },
            Dependency {
                issue_id: "bf-parent".to_string(),
                depends_on_id: "bf-blocker2".to_string(),
                dep_type: DependencyType::ParentChild,
                created_at: Utc::now(),
                created_by: None,
                metadata: None,
                thread_id: None,
                title: Some("Second blocker".to_string()),
            },
        ];

        let result = format_dependencies(&deps);
        assert_eq!(result, "Depends: bf-blocker1 (First blocker) (blocks), bf-blocker2 (Second blocker) (blocks)");
    }

    // ==================== Helper functions ====================

    fn create_test_issue(id: &str, title: &str) -> Issue {
        Issue {
            id: id.to_string(),
            title: title.to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            description: None,
            assignee: None,
            labels: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            // Default/None for all other fields
            content_hash: None,
            design: None,
            acceptance_criteria: None,
            notes: None,
            owner: None,
            estimated_minutes: None,
            created_by: None,
            closed_at: None,
            close_reason: None,
            closed_by_session: None,
            due_at: None,
            defer_until: None,
            external_ref: None,
            source_system: None,
            source_repo: None,
            deleted_at: None,
            deleted_by: None,
            delete_reason: None,
            original_type: None,
            compaction_level: None,
            compacted_at: None,
            compacted_at_commit: None,
            original_size: None,
            sender: None,
            ephemeral: false,
            pinned: false,
            is_template: false,
            dependencies: vec![],
            comments: vec![],
            events: vec![],
            annotations: std::collections::BTreeMap::new(),
        }
    }
}
