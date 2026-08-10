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
    use crate::model::{Dependency, DependencyType, IssueType, Priority, Status};
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

    // ==================== format_error tests ====================

    #[test]
    fn format_error_basic() {
        let formatter = TextFormatter;
        let result = formatter.format_error("Something went wrong");
        assert_eq!(result, "Error: Something went wrong\n");
    }

    #[test]
    fn format_error_empty() {
        let formatter = TextFormatter;
        let result = formatter.format_error("");
        assert_eq!(result, "Error: \n");
    }

    #[test]
    fn format_error_with_special_characters() {
        let formatter = TextFormatter;
        let result = formatter.format_error("Error: file not found: <test> & 'quotes'");
        assert_eq!(result, "Error: file not found: <test> & 'quotes'\n");
    }

    #[test]
    fn format_error_long_message() {
        let formatter = TextFormatter;
        let long_msg = "A".repeat(1000);
        let result = formatter.format_error(&long_msg);
        assert!(result.starts_with("Error: "));
        assert!(result.ends_with('\n'));
        assert_eq!(result.len(), 1008); // "Error: " + 1000 chars + "\n"
    }

    #[test]
    fn format_error_with_newlines() {
        let formatter = TextFormatter;
        let result = formatter.format_error("Line 1\nLine 2\nLine 3");
        assert_eq!(result, "Error: Line 1\nLine 2\nLine 3\n");
    }

    #[test]
    fn format_error_unicode() {
        let formatter = TextFormatter;
        let result = formatter.format_error("Error: emoji test 🚀 🔥 💻");
        assert_eq!(result, "Error: emoji test 🚀 🔥 💻\n");
    }

    // ==================== format_issues tests ====================

    #[test]
    fn format_issues_empty() {
        let formatter = TextFormatter;
        let issues: Vec<Issue> = vec![];
        let result = formatter.format_issues(&issues);
        assert_eq!(result, "");
    }

    #[test]
    fn format_issues_single() {
        let formatter = TextFormatter;
        let issue = create_test_issue("bf-abc123", "Test issue");
        let result = formatter.format_issues(&[issue]);

        assert!(result.contains("[bf-abc123]"));
        assert!(result.contains("Test issue"));
        assert!(result.contains("open"));
        assert!(result.contains("MEDIUM"));
    }

    #[test]
    fn format_issues_multiple() {
        let formatter = TextFormatter;
        let issues = vec![
            create_test_issue("bf-001", "First issue"),
            create_test_issue("bf-002", "Second issue"),
            create_test_issue("bf-003", "Third issue"),
        ];
        let result = formatter.format_issues(&issues);

        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("[bf-001]"));
        assert!(lines[1].contains("[bf-002]"));
        assert!(lines[2].contains("[bf-003]"));
    }

    #[test]
    fn format_issues_with_different_statuses() {
        let formatter = TextFormatter;
        let mut issues = vec![
            create_test_issue("bf-001", "Open issue"),
            create_test_issue("bf-002", "In progress issue"),
        ];
        issues[1].status = Status::InProgress;

        let result = formatter.format_issues(&issues);

        assert!(result.contains("open"));
        assert!(result.contains("in_progress"));
    }

    #[test]
    fn format_issues_with_different_priorities() {
        let formatter = TextFormatter;
        let mut issues = vec![
            create_test_issue("bf-001", "High priority"),
            create_test_issue("bf-002", "Low priority"),
        ];
        issues[0].priority = Priority::HIGH;
        issues[1].priority = Priority::LOW;

        let result = formatter.format_issues(&issues);

        assert!(result.contains("HIGH"));
        assert!(result.contains("LOW"));
    }

    #[test]
    fn format_issues_special_characters() {
        let formatter = TextFormatter;
        let issue = Issue {
            id: "bf-特殊".to_string(),
            title: "Title with <quotes> & \"double\" & 'single'".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            description: None,
            assignee: None,
            labels: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
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
        };

        let result = formatter.format_issues(&[issue]);
        assert!(result.contains("bf-特殊"));
        assert!(result.contains("<quotes>"));
    }

    #[test]
    fn format_issues_long_title() {
        let formatter = TextFormatter;
        let long_title = "A".repeat(500);
        let issue = create_test_issue("bf-long", &long_title);
        let result = formatter.format_issues(&[issue]);

        assert!(result.contains("bf-long"));
        assert!(result.contains(&long_title));
    }

    // ==================== format_issue tests ====================

    #[test]
    fn format_issue_basic() {
        let formatter = TextFormatter;
        let issue = create_test_issue("bf-basic", "Basic test issue");
        let result = formatter.format_issue(&issue);

        assert!(result.contains("ID: bf-basic"));
        assert!(result.contains("Title: Basic test issue"));
        assert!(result.contains("Status: open"));
        assert!(result.contains("Priority: MEDIUM"));
        assert!(result.contains("Type: task"));
        assert!(result.contains("Created at:"));
        assert!(result.contains("Updated at:"));
    }

    #[test]
    fn format_issue_with_description() {
        let formatter = TextFormatter;
        let mut issue = create_test_issue("bf-desc", "Issue with description");
        issue.description = Some("This is a detailed description".to_string());
        let result = formatter.format_issue(&issue);

        assert!(result.contains("ID: bf-desc"));
        assert!(result.contains("Description: This is a detailed description"));
    }

    #[test]
    fn format_issue_with_assignee() {
        let formatter = TextFormatter;
        let mut issue = create_test_issue("bf-assignee", "Issue with assignee");
        issue.assignee = Some("john.doe".to_string());
        let result = formatter.format_issue(&issue);

        assert!(result.contains("Assignee: john.doe"));
    }

    #[test]
    fn format_issue_with_labels() {
        let formatter = TextFormatter;
        let mut issue = create_test_issue("bf-labels", "Issue with labels");
        issue.labels = vec!["bug".to_string(), "urgent".to_string(), "frontend".to_string()];
        let result = formatter.format_issue(&issue);

        assert!(result.contains("Labels: bug, urgent, frontend"));
    }

    #[test]
    fn format_issue_with_all_optional_fields() {
        let formatter = TextFormatter;
        let mut issue = create_test_issue("bf-full", "Issue with all fields");
        issue.description = Some("Complete description".to_string());
        issue.assignee = Some("jane.smith".to_string());
        issue.labels = vec!["feature".to_string(), "backend".to_string()];
        let result = formatter.format_issue(&issue);

        assert!(result.contains("ID: bf-full"));
        assert!(result.contains("Description: Complete description"));
        assert!(result.contains("Assignee: jane.smith"));
        assert!(result.contains("Labels: feature, backend"));
        assert!(result.contains("Created at:"));
        assert!(result.contains("Updated at:"));
    }

    #[test]
    fn format_issue_empty_description() {
        let formatter = TextFormatter;
        let mut issue = create_test_issue("bf-empty-desc", "Issue with empty description");
        issue.description = Some("".to_string());
        let result = formatter.format_issue(&issue);

        // Empty description should still be included
        assert!(result.contains("Description:"));
    }

    #[test]
    fn format_issue_empty_assignee() {
        let formatter = TextFormatter;
        let mut issue = create_test_issue("bf-empty-assignee", "Issue with empty assignee");
        issue.assignee = Some("".to_string());
        let result = formatter.format_issue(&issue);

        // Empty assignee should still be included
        assert!(result.contains("Assignee:"));
    }

    #[test]
    fn format_issue_special_characters_in_fields() {
        let formatter = TextFormatter;
        let mut issue = create_test_issue("bf-special", "Issue with special chars: <>&'");
        issue.description = Some("Description with \"quotes\" & 'apostrophes'".to_string());
        issue.assignee = Some("user@example.com".to_string());
        issue.labels = vec!["tag-with-dash".to_string(), "tag_with_underscore".to_string()];
        let result = formatter.format_issue(&issue);

        assert!(result.contains("<>&'"));
        assert!(result.contains("\"quotes\""));
        assert!(result.contains("user@example.com"));
        assert!(result.contains("tag-with-dash"));
    }

    #[test]
    fn format_issue_long_title() {
        let formatter = TextFormatter;
        let long_title = "A".repeat(1000);
        let issue = create_test_issue("bf-long-title", &long_title);
        let result = formatter.format_issue(&issue);

        assert!(result.contains(&long_title));
        assert!(result.contains("ID: bf-long-title"));
    }

    #[test]
    fn format_issue_unicode_characters() {
        let formatter = TextFormatter;
        let mut issue = create_test_issue("bf-unicode", "Issue with emoji 🚀");
        issue.description = Some("Description with unicode: café, naïve, 日本語".to_string());
        issue.assignee = Some("user@example.com".to_string());
        issue.labels = vec!["unicode-tag-中文".to_string()];
        let result = formatter.format_issue(&issue);

        assert!(result.contains("🚀"));
        assert!(result.contains("café"));
        assert!(result.contains("日本語"));
        assert!(result.contains("中文"));
    }

    #[test]
    fn format_issue_empty_labels() {
        let formatter = TextFormatter;
        let issue = create_test_issue("bf-no-labels", "Issue without labels");
        let result = formatter.format_issue(&issue);

        // Empty labels should NOT be included
        assert!(!result.contains("Labels:"));
    }

    #[test]
    fn format_issue_no_description_or_assignee() {
        let formatter = TextFormatter;
        let issue = create_test_issue("bf-minimal", "Minimal issue");
        let result = formatter.format_issue(&issue);

        // These fields should NOT be present when None
        assert!(!result.contains("Description:"));
        assert!(!result.contains("Assignee:"));

        // But required fields should be present
        assert!(result.contains("ID: bf-minimal"));
        assert!(result.contains("Title: Minimal issue"));
        assert!(result.contains("Status: open"));
        assert!(result.contains("Priority: MEDIUM"));
        assert!(result.contains("Type: task"));
    }

    #[test]
    fn format_issue_newlines_in_description() {
        let formatter = TextFormatter;
        let mut issue = create_test_issue("bf-newlines", "Issue with newlines");
        issue.description = Some("Line 1\nLine 2\nLine 3".to_string());
        let result = formatter.format_issue(&issue);

        assert!(result.contains("Line 1\nLine 2\nLine 3"));
    }

    #[test]
    fn format_issue_different_status() {
        let formatter = TextFormatter;
        let mut issue = create_test_issue("bf-status", "Issue with custom status");
        issue.status = Status::InProgress;
        let result = formatter.format_issue(&issue);

        assert!(result.contains("Status: in_progress"));
    }

    #[test]
    fn format_issue_different_priority() {
        let formatter = TextFormatter;
        let mut issue = create_test_issue("bf-priority", "Issue with high priority");
        issue.priority = Priority::HIGH;
        let result = formatter.format_issue(&issue);

        assert!(result.contains("Priority: HIGH"));
    }

    #[test]
    fn format_issue_different_type() {
        let formatter = TextFormatter;
        let mut issue = create_test_issue("bf-type", "Bug issue");
        issue.issue_type = IssueType::Bug;
        let result = formatter.format_issue(&issue);

        assert!(result.contains("Type: bug"));
    }

    // ==================== Separator formatting tests ====================

    #[test]
    fn test_velocity_separator_exact_dash_count() {
        let stats = vec![
            VelocityStats {
                model: "claude-sonnet-5".to_string(),
                harness: "needle".to_string(),
                issue_type: "task".to_string(),
                sample_count: 10,
                p50_seconds: Some(120.0),
                p90_seconds: Some(300.0),
                avg_seconds: Some(150.0),
            },
        ];

        let result = format_velocity_text(&stats);
        let lines: Vec<&str> = result.lines().collect();

        // Find the separator line (should be after header)
        let separator_line = lines.iter()
            .find(|line| line.chars().all(|c| c == '-'))
            .expect("Should have a separator line with only dashes");

        // Count exact number of dashes
        let dash_count = separator_line.chars().count();
        assert_eq!(dash_count, 85, "Velocity separator should have exactly 85 dashes, got {}", dash_count);
    }

    #[test]
    fn test_velocity_separator_positioning() {
        let stats = vec![
            VelocityStats {
                model: "claude-sonnet-5".to_string(),
                harness: "needle".to_string(),
                issue_type: "task".to_string(),
                sample_count: 10,
                p50_seconds: Some(120.0),
                p90_seconds: Some(300.0),
                avg_seconds: Some(150.0),
            },
        ];

        let result = format_velocity_text(&stats);
        let lines: Vec<&str> = result.lines().collect();

        // Find header and separator lines
        let header_line = lines.iter().find(|line| line.contains("Model")).expect("Should have header");
        let separator_line = lines.iter()
            .find(|line| line.chars().all(|c| c == '-'))
            .expect("Should have separator line");

        let header_idx = lines.iter().position(|l| *l == header_line).unwrap();
        let separator_idx = lines.iter().position(|l| *l == separator_line).unwrap();

        // Separator should come immediately after header
        assert_eq!(separator_idx, header_idx + 1, "Separator should be immediately after header");

        // There should be content after separator
        assert!(lines.len() > separator_idx + 1, "Should have content after separator");
    }

    #[test]
    fn test_velocity_separator_with_no_stats() {
        let stats: Vec<VelocityStats> = vec![];
        let result = format_velocity_text(&stats);

        // When no stats, should show message but no separator
        assert!(result.contains("No velocity statistics available yet"));
        assert!(!result.chars().any(|c| c == '-'), "Should have no separator when no stats");
    }

    #[test]
    fn test_velocity_separator_matches_header_width() {
        let stats = vec![
            VelocityStats {
                model: "claude-sonnet-5".to_string(),
                harness: "needle".to_string(),
                issue_type: "task".to_string(),
                sample_count: 10,
                p50_seconds: Some(120.0),
                p90_seconds: Some(300.0),
                avg_seconds: Some(150.0),
            },
        ];

        let result = format_velocity_text(&stats);
        let lines: Vec<&str> = result.lines().collect();

        let header_line = lines.iter().find(|line| line.contains("Model")).expect("Should have header");
        let separator_line = lines.iter()
            .find(|line| line.chars().all(|c| c == '-'))
            .expect("Should have separator line");

        // Both should have same width
        assert_eq!(header_line.len(), separator_line.len(),
                   "Separator width should match header width");
    }

    #[test]
    fn test_velocity_separator_with_multiple_stats() {
        let stats = vec![
            VelocityStats {
                model: "claude-sonnet-5".to_string(),
                harness: "needle".to_string(),
                issue_type: "task".to_string(),
                sample_count: 10,
                p50_seconds: Some(120.0),
                p90_seconds: Some(300.0),
                avg_seconds: Some(150.0),
            },
            VelocityStats {
                model: "claude-opus-5".to_string(),
                harness: "claude-code".to_string(),
                issue_type: "bug".to_string(),
                sample_count: 5,
                p50_seconds: Some(90.0),
                p90_seconds: Some(180.0),
                avg_seconds: Some(110.0),
            },
        ];

        let result = format_velocity_text(&stats);
        let lines: Vec<&str> = result.lines().collect();

        // Count separator lines - should be exactly 1
        let separator_count = lines.iter()
            .filter(|line| line.chars().all(|c| c == '-'))
            .count();

        assert_eq!(separator_count, 1, "Should have exactly one separator line");

        // All separators (if any) should have 85 dashes
        for line in lines.iter() {
            if line.chars().all(|c| c == '-') {
                assert_eq!(line.len(), 85, "Each separator should be exactly 85 characters");
            }
        }
    }

    // ==================== format_dependencies_display tests ====================

    #[test]
    fn test_format_dependencies_display_empty() {
        let deps: Vec<crate::storage::sqlite::DependencyDisplay> = vec![];
        let result = format_dependencies_display(&deps);
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_dependencies_display_single_blocking() {
        let deps = vec![crate::storage::sqlite::DependencyDisplay {
            dep_type: "blocks".to_string(),
            bead_id: "bf-blocker".to_string(),
            title: "Blocker task".to_string(),
        }];

        let result = format_dependencies_display(&deps);
        assert_eq!(result, "Depends: bf-blocker (Blocker task) (blocks)");
    }

    #[test]
    fn test_format_dependencies_display_single_non_blocking() {
        let deps = vec![crate::storage::sqlite::DependencyDisplay {
            dep_type: "related".to_string(),
            bead_id: "bf-related".to_string(),
            title: "Related task".to_string(),
        }];

        let result = format_dependencies_display(&deps);
        assert_eq!(result, "Depends: bf-related (Related task)");
    }

    #[test]
    fn test_format_dependencies_display_multiple_mixed() {
        let deps = vec![
            crate::storage::sqlite::DependencyDisplay {
                dep_type: "blocks".to_string(),
                bead_id: "bf-blocker".to_string(),
                title: "Blocker task".to_string(),
            },
            crate::storage::sqlite::DependencyDisplay {
                dep_type: "related".to_string(),
                bead_id: "bf-related".to_string(),
                title: "Related task".to_string(),
            },
        ];

        let result = format_dependencies_display(&deps);
        assert_eq!(result, "Depends: bf-blocker (Blocker task) (blocks), bf-related (Related task)");
    }

    #[test]
    fn test_format_dependencies_display_special_characters() {
        let deps = vec![
            crate::storage::sqlite::DependencyDisplay {
                dep_type: "blocks".to_string(),
                bead_id: "bf-001".to_string(),
                title: "Task with <quotes> & \"double\" & 'single'".to_string(),
            },
            crate::storage::sqlite::DependencyDisplay {
                dep_type: "related".to_string(),
                bead_id: "bf-002".to_string(),
                title: "Task with emoji 🚀 🔥".to_string(),
            },
        ];

        let result = format_dependencies_display(&deps);
        assert!(result.contains("<quotes>"));
        assert!(result.contains("🚀"));
        assert!(result.contains("(blocks)"));
    }

    #[test]
    fn test_format_dependencies_display_multiple_blocking() {
        let deps = vec![
            crate::storage::sqlite::DependencyDisplay {
                dep_type: "blocks".to_string(),
                bead_id: "bf-blocker1".to_string(),
                title: "First blocker".to_string(),
            },
            crate::storage::sqlite::DependencyDisplay {
                dep_type: "blocks".to_string(),
                bead_id: "bf-blocker2".to_string(),
                title: "Second blocker".to_string(),
            },
        ];

        let result = format_dependencies_display(&deps);
        assert_eq!(result, "Depends: bf-blocker1 (First blocker) (blocks), bf-blocker2 (Second blocker) (blocks)");
    }

    #[test]
    fn test_format_dependencies_display_long_title() {
        let long_title = "A".repeat(500);
        let deps = vec![crate::storage::sqlite::DependencyDisplay {
            dep_type: "blocks".to_string(),
            bead_id: "bf-long".to_string(),
            title: long_title.clone(),
        }];

        let result = format_dependencies_display(&deps);
        assert!(result.contains("bf-long"));
        assert!(result.contains(&long_title));
    }

    #[test]
    fn test_format_dependencies_display_unicode_title() {
        let deps = vec![crate::storage::sqlite::DependencyDisplay {
            dep_type: "blocks".to_string(),
            bead_id: "bf-unicode".to_string(),
            title: "Tâsk with spëcial çharacters 日本語 中文".to_string(),
        }];

        let result = format_dependencies_display(&deps);
        assert!(result.contains("Tâsk"));
        assert!(result.contains("日本語"));
        assert!(result.contains("中文"));
    }

    #[test]
    fn test_format_dependencies_display_multiple_non_blocking() {
        let deps = vec![
            crate::storage::sqlite::DependencyDisplay {
                dep_type: "related".to_string(),
                bead_id: "bf-related1".to_string(),
                title: "First related".to_string(),
            },
            crate::storage::sqlite::DependencyDisplay {
                dep_type: "related".to_string(),
                bead_id: "bf-related2".to_string(),
                title: "Second related".to_string(),
            },
        ];

        let result = format_dependencies_display(&deps);
        assert_eq!(result, "Depends: bf-related1 (First related), bf-related2 (Second related)");
    }

    #[test]
    fn test_format_dependencies_display_empty_title() {
        let deps = vec![crate::storage::sqlite::DependencyDisplay {
            dep_type: "blocks".to_string(),
            bead_id: "bf-empty".to_string(),
            title: "".to_string(),
        }];

        let result = format_dependencies_display(&deps);
        assert_eq!(result, "Depends: bf-empty () (blocks)");
    }

    #[test]
    fn test_format_dependencies_display_newlines_in_title() {
        let deps = vec![crate::storage::sqlite::DependencyDisplay {
            dep_type: "blocks".to_string(),
            bead_id: "bf-newlines".to_string(),
            title: "Line 1\nLine 2\nLine 3".to_string(),
        }];

        let result = format_dependencies_display(&deps);
        assert!(result.contains("Line 1\nLine 2\nLine 3"));
    }

    #[test]
    fn test_format_dependencies_display_three_dependencies() {
        let deps = vec![
            crate::storage::sqlite::DependencyDisplay {
                dep_type: "blocks".to_string(),
                bead_id: "bf-1".to_string(),
                title: "First".to_string(),
            },
            crate::storage::sqlite::DependencyDisplay {
                dep_type: "related".to_string(),
                bead_id: "bf-2".to_string(),
                title: "Second".to_string(),
            },
            crate::storage::sqlite::DependencyDisplay {
                dep_type: "blocks".to_string(),
                bead_id: "bf-3".to_string(),
                title: "Third".to_string(),
            },
        ];

        let result = format_dependencies_display(&deps);
        assert_eq!(result, "Depends: bf-1 (First) (blocks), bf-2 (Second), bf-3 (Third) (blocks)");
    }
}
