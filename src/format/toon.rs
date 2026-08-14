use crate::format::{ClaimResultOutput, Formatter, StatsOutput};
use crate::model::{Dependency, Issue};
use crate::velocity::VelocityStats;

#[derive(Debug, Clone, Copy)]
pub struct ToonFormatter;

impl Formatter for ToonFormatter {
    fn format_issue(&self, issue: &Issue) -> String {
        // Token-optimized single-line format:
        // id|status|priority|type|assignee|labels|title_truncated
        let mut parts = vec![
            issue.id.clone(),
            issue.status.as_str().to_string(),
            format!("P{}", issue.priority.0),
            issue.issue_type.as_str().to_string(),
        ];

        // Add assignee if present
        parts.push(issue.assignee.as_deref().unwrap_or("-").to_string());

        // Add labels if present
        if issue.labels.is_empty() {
            parts.push("-".to_string());
        } else {
            parts.push(issue.labels.join(","));
        }

        // Truncate title to 60 chars for token efficiency
        let title = if issue.title.len() > 60 {
            format!("{}...", &issue.title[..57])
        } else {
            issue.title.clone()
        };
        parts.push(title);

        parts.join("|")
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
        // Token-optimized error format
        format!("E: {}\n", message)
    }

    fn format_claim_result(&self, result: &ClaimResultOutput) -> String {
        if result.dry_run == Some(true) {
            format!(
                "{} p={} i={} w={}",
                result.bead_id,
                result.priority.unwrap_or(0),
                result.downstream_impact.unwrap_or(0),
                result.workspace.as_deref().unwrap_or(""),
            )
        } else if let Some(workspace) = &result.workspace {
            format!("{} w={}", result.bead_id, workspace)
        } else {
            result.bead_id.clone()
        }
    }

    fn format_no_claim(&self) -> String {
        "no beads".to_string()
    }

    fn format_stats(&self, stats: &StatsOutput) -> String {
        // Token-optimized stats format
        let mut s = String::new();
        s.push_str(&format!("T:{} O:{} IP:{} C:{}", stats.total, stats.open, stats.in_progress, stats.closed));

        // Add breakdowns in compact form if present
        if let Some(by_type) = &stats.by_type {
            let type_parts: Vec<String> = by_type
                .iter()
                .map(|(t, c)| format!("{}={}", t, c))
                .collect();
            s.push_str(&format!(" Ty:{}", type_parts.join(",")));
        }

        if let Some(by_priority) = &stats.by_priority {
            let prio_parts: Vec<String> = by_priority
                .iter()
                .map(|(p, c)| format!("P{}={}", p, c))
                .collect();
            s.push_str(&format!(" Pr:{}", prio_parts.join(",")));
        }

        if let Some(by_assignee) = &stats.by_assignee {
            let assignee_parts: Vec<String> = by_assignee
                .iter()
                .map(|(a, c)| format!("{}={}", a, c))
                .collect();
            s.push_str(&format!(" As:{}", assignee_parts.join(",")));
        }

        s.push('\n');
        s
    }

    fn format_velocity(&self, stats: &[VelocityStats]) -> String {
        // Token-optimized velocity format: one line per stat
        let mut s = String::new();
        for stat in stats {
            let parts = vec![
                stat.model.clone(),
                stat.harness.clone(),
                stat.issue_type.clone(),
                stat.sample_count.to_string(),
                stat.p50_seconds.map(|s| s.to_string()).unwrap_or("-".to_string()),
                stat.p90_seconds.map(|s| s.to_string()).unwrap_or("-".to_string()),
                stat.avg_seconds.map(|s| format!("{:.1}", s)).unwrap_or("-".to_string()),
            ];
            s.push_str(&parts.join("|"));
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
    // Token-optimized single-line format for lists
    // id status priority type assignee labels title
    let status_short = shorten_status(issue.status.as_str());
    let type_short = shorten_type(issue.issue_type.as_str());
    let assignee = issue.assignee.as_deref().unwrap_or("-");
    let labels = if issue.labels.is_empty() {
        "-".to_string()
    } else {
        issue.labels.join(",")
    };

    format!(
        "{} {} {} {} {} {} {}",
        issue.id,
        status_short,
        format!("P{}", issue.priority.0),
        type_short,
        assignee,
        labels,
        issue.title
    )
}

fn shorten_status(status: &str) -> &str {
    match status {
        "open" => "o",
        "in_progress" => "ip",
        "blocked" => "blk",
        "deferred" => "def",
        "draft" => "drf",
        "closed" => "cls",
        "tombstone" => "tmb",
        "pinned" => "pin",
        s => s,
    }
}

fn shorten_type(issue_type: &str) -> &str {
    match issue_type {
        "task" => "tsk",
        "bug" => "bug",
        "feature" => "feat",
        "epic" => "epic",
        "chore" => "chore",
        "docs" => "doc",
        "question" => "?",
        t => t,
    }
}

/// Format a ready bead for token-optimized output.
pub fn format_ready_bead(id: &str, title: &str, priority: i32, impact: i64, float: f32) -> String {
    format!(
        "{} P{} i{} f{} {}",
        id, priority, impact, float, title
    )
}

/// Format dependencies in token-optimized format.
pub fn format_dependencies(dependencies: &[Dependency]) -> String {
    if dependencies.is_empty() {
        return String::new();
    }

    let parts: Vec<String> = dependencies
        .iter()
        .map(|dep| {
            let title = dep.title.as_deref().unwrap_or("?");
            if dep.dep_type.is_blocking() {
                format!("{}({})!", dep.depends_on_id, title)
            } else {
                format!("{}({})", dep.depends_on_id, title)
            }
        })
        .collect();

    format!("D:{}", parts.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Dependency, DependencyType, IssueType, Priority, Status};
    use chrono::Utc;

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

    #[test]
    fn test_format_issue_token_optimized() {
        let formatter = ToonFormatter;
        let issue = create_test_issue("bf-123", "Test issue with very long title that should be truncated");

        let result = formatter.format_issue(&issue);

        // Should be pipe-delimited with minimal separators
        assert!(result.contains("bf-123"));
        assert!(result.contains("P2")); // MEDIUM = 2
        assert!(!result.contains("ID:")); // No verbose labels
        assert!(!result.contains("Title:")); // No verbose labels
    }

    #[test]
    fn test_format_issue_with_long_title_truncation() {
        let formatter = ToonFormatter;
        let long_title = "A".repeat(100);
        let issue = create_test_issue("bf-trunc", &long_title);

        let result = formatter.format_issue(&issue);

        // Title should be truncated to 60 chars with "..." suffix
        assert!(result.len() < long_title.len() + 20); // Much shorter
        assert!(result.contains("...")); // Has truncation marker
    }

    #[test]
    fn test_format_issues_shortens_status_and_type() {
        let formatter = ToonFormatter;
        let mut issue = create_test_issue("bf-abc", "Test issue");
        issue.status = Status::InProgress;
        issue.issue_type = IssueType::Feature;

        let result = formatter.format_issues(&[issue]);

        assert!(result.contains("ip")); // in_progress shortened
        assert!(result.contains("feat")); // feature shortened
        assert!(!result.contains("in_progress")); // Not full form
        assert!(!result.contains("feature")); // Not full form
    }

    #[test]
    fn test_format_issues_compact_format() {
        let formatter = ToonFormatter;
        let issue = create_test_issue("bf-compact", "Compact format test");

        let result = formatter.format_issues(&[issue]);

        // Should have no verbose labels, just space-separated fields
        assert!(!result.contains("ID:"));
        assert!(!result.contains("Status:"));
        assert!(!result.contains("Priority:"));
        // Should contain the essential data
        assert!(result.contains("bf-compact"));
        assert!(result.contains("o")); // open shortened
        assert!(result.contains("P2"));
    }

    #[test]
    fn test_format_error_short() {
        let formatter = ToonFormatter;
        let result = formatter.format_error("test error");

        assert_eq!(result, "E: test error\n");
        assert!(result.contains("E:")); // Short error prefix
    }

    #[test]
    fn test_format_claim_result_compact() {
        let formatter = ToonFormatter;
        let mut result = ClaimResultOutput::new("bf-claim", "agent");
        result.priority = Some(1);
        result.downstream_impact = Some(5);
        result.workspace = Some("/repo".to_string());
        result.dry_run = Some(true);

        let formatted = formatter.format_claim_result(&result);

        // Should use short field names (p=, i=, w=)
        assert!(formatted.contains("p=1"));
        assert!(formatted.contains("i=5"));
        assert!(formatted.contains("w=/repo"));
        assert!(!formatted.contains("priority=")); // Not verbose
    }

    #[test]
    fn test_format_stats_compact() {
        let formatter = ToonFormatter;
        let stats = StatsOutput::new(100, 50, 30, 20);

        let result = formatter.format_stats(&stats);

        // Should use compact format: T: O: IP: C:
        assert!(result.contains("T:100"));
        assert!(result.contains("O:50"));
        assert!(result.contains("IP:30"));
        assert!(result.contains("C:20"));
        assert!(!result.contains("Total")); // No verbose labels
    }

    #[test]
    fn test_format_velocity_compact() {
        let formatter = ToonFormatter;
        let stats = vec![
            VelocityStats {
                model: "claude-sonnet-5".to_string(),
                harness: "needle".to_string(),
                issue_type: "task".to_string(),
                sample_count: 10,
                p50_seconds: Some(120),
                p90_seconds: Some(300),
                avg_seconds: Some(150.0),
                last_updated: Some("2024-01-01T00:00:00Z".to_string()),
            },
        ];

        let result = formatter.format_velocity(&stats);

        // Should be pipe-delimited single line
        assert!(result.contains("|"));
        assert!(result.contains("claude-sonnet-5"));
        assert!(!result.contains("Model:")); // No verbose labels
    }

    #[test]
    fn test_format_ready_bead_compact() {
        let result = format_ready_bead("bf-ready", "Ready bead", 2, 5, 0.95);

        // Should use compact format with short field names
        assert!(result.contains("P2"));
        assert!(result.contains("i5"));
        assert!(result.contains("f0.95"));
        assert!(!result.contains("priority=")); // Not verbose
    }

    #[test]
    fn test_format_dependencies_compact() {
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
        ];

        let result = format_dependencies(&deps);

        // Should use compact format D: with ! for blocking
        assert!(result.contains("D:"));
        assert!(result.contains("bf-blocker(Blocker task)!"));
        assert!(!result.contains("Depends:")); // No verbose label
    }

    #[test]
    fn test_shorten_status_all_variants() {
        assert_eq!(shorten_status("open"), "o");
        assert_eq!(shorten_status("in_progress"), "ip");
        assert_eq!(shorten_status("blocked"), "blk");
        assert_eq!(shorten_status("deferred"), "def");
        assert_eq!(shorten_status("draft"), "drf");
        assert_eq!(shorten_status("closed"), "cls");
        assert_eq!(shorten_status("tombstone"), "tmb");
        assert_eq!(shorten_status("pinned"), "pin");
        // Custom status passes through
        assert_eq!(shorten_status("custom"), "custom");
    }

    #[test]
    fn test_shorten_type_all_variants() {
        assert_eq!(shorten_type("task"), "tsk");
        assert_eq!(shorten_type("bug"), "bug");
        assert_eq!(shorten_type("feature"), "feat");
        assert_eq!(shorten_type("epic"), "epic");
        assert_eq!(shorten_type("chore"), "chore");
        assert_eq!(shorten_type("docs"), "doc");
        assert_eq!(shorten_type("question"), "?");
        // Custom type passes through
        assert_eq!(shorten_type("spike"), "spike");
    }

    #[test]
    fn test_format_issue_with_labels_compact() {
        let formatter = ToonFormatter;
        let mut issue = create_test_issue("bf-labels", "Issue with labels");
        issue.labels = vec!["urgent".to_string(), "backend".to_string()];

        let result = formatter.format_issue(&issue);

        // Labels should be comma-separated in compact format
        assert!(result.contains("urgent,backend"));
        assert!(!result.contains("Labels:")); // No verbose label
    }

    #[test]
    fn test_format_issue_with_assignee_compact() {
        let formatter = ToonFormatter;
        let mut issue = create_test_issue("bf-assignee", "Issue with assignee");
        issue.assignee = Some("agent-001".to_string());

        let result = formatter.format_issue(&issue);

        // Assignee should be present without verbose label
        assert!(result.contains("agent-001"));
        assert!(!result.contains("Assignee:")); // No verbose label
    }

    #[test]
    fn test_format_no_claim_short() {
        let formatter = ToonFormatter;
        let result = formatter.format_no_claim();

        assert_eq!(result, "no beads");
        assert!(!result.contains("No beads available")); // Not verbose
    }
}
