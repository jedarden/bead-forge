use crate::format::color::{format_status_colored, Color};
use crate::model::{Issue, Priority};
use std::cmp::max;

/// Table formatter for aligned column display of bead listings.
///
/// Provides table formatting with proper column alignment and color support.
/// Used by the text formatter for bead listings where visual alignment matters.
#[derive(Debug, Clone, Copy)]
pub struct TableFormatter {
    /// Use color coding for status display
    pub colored: bool,
}

impl TableFormatter {
    pub fn new() -> Self {
        Self { colored: Color::should_color() }
    }

    pub fn with_color(colored: bool) -> Self {
        Self { colored }
    }

    /// Format a list of issues as a table with aligned columns.
    ///
    /// Columns: ID, Title (truncated), Status, Priority, Type, Assignee (optional)
    pub fn format_issues(&self, issues: &[Issue]) -> String {
        if issues.is_empty() {
            return String::new();
        }

        // Calculate column widths based on content
        let max_id_width = issues.iter().map(|i| i.id.len()).max().unwrap_or(0);
        let max_title_width = issues.iter().map(|i| i.title.len()).max().unwrap_or(0);
        let max_status_width = issues.iter().map(|i| i.status.as_str().len()).max().unwrap_or(0);
        let max_type_width = issues.iter().map(|i| i.issue_type.as_str().len()).max().unwrap_or(0);
        let max_assignee_width = issues.iter().map(|i| {
            i.assignee.as_ref().map(|a| a.len()).unwrap_or(0)
        }).max().unwrap_or(0);

        // Set minimum widths for headers
        let id_width = max(max_id_width, 2);
        let title_width = min(max_title_width, 60); // Limit title width for readability
        let status_width = max(max_status_width, 6);
        let priority_width = 8; // "P0" to "P4" plus padding
        let type_width = max(max_type_width, 4);
        let assignee_width = max(max_assignee_width, 8);

        let mut result = String::new();

        // Header row
        result.push_str(&format!(
            "{:<id_width$} | {:<title_width$} | {:<status_width$} | {:<priority_width$} | {:<type_width$} | {:<assignee_width$}\n",
            "ID",
            "Title",
            "Status",
            "Priority",
            "Type",
            "Assignee",
            id_width = id_width,
            title_width = title_width,
            status_width = status_width,
            priority_width = priority_width,
            type_width = type_width,
            assignee_width = assignee_width
        ));

        // Separator row
        let separator = "-".repeat(id_width) + "-+-" +
            &"-".repeat(title_width) + "-+-" +
            &"-".repeat(status_width) + "-+-" +
            &"-".repeat(priority_width) + "-+-" +
            &"-".repeat(type_width) + "-+-" +
            &"-".repeat(assignee_width);
        result.push_str(&separator);
        result.push('\n');

        // Data rows
        for issue in issues {
            let title = if issue.title.len() > title_width {
                format!("{}...", &issue.title[..title_width - 3])
            } else {
                issue.title.clone()
            };

            let status_str = if self.colored {
                format_status_colored(&issue.status)
            } else {
                issue.status.to_string()
            };

            let priority_str = format_priority_colored(issue.priority, self.colored);

            let assignee_str = issue.assignee.as_deref().unwrap_or("");

            result.push_str(&format!(
                "{:<id_width$} | {:<title_width$} | {:<status_width$} | {:<priority_width$} | {:<type_width$} | {:<assignee_width$}\n",
                issue.id,
                title,
                status_str,
                priority_str,
                issue.issue_type.as_str(),
                assignee_str,
                id_width = id_width,
                title_width = title_width,
                status_width = status_width + if self.colored {
                    // ANSI codes add width, so we need to adjust
                    status_str.len() - issue.status.as_str().len()
                } else {
                    0
                },
                priority_width = priority_width,
                type_width = type_width,
                assignee_width = assignee_width
            ));
        }

        result
    }

    /// Format a single issue as a detailed view (for `bf show`).
    ///
    /// Shows all relevant fields in a clean, readable format.
    pub fn format_issue_detail(&self, issue: &Issue) -> String {
        let mut result = String::new();

        // Header with ID and title
        result.push_str(&format!("{}: {}\n", issue.id, issue.title));
        result.push_str(&format!("{}\n", "=".repeat(80)));

        // Core fields
        result.push_str(&format!("Status:      "));
        if self.colored {
            result.push_str(&format_status_colored(&issue.status));
        } else {
            result.push_str(&issue.status.to_string());
        }
        result.push('\n');

        result.push_str(&format!("Priority:    {}\n", format_priority_colored(issue.priority, self.colored)));
        result.push_str(&format!("Type:        {}\n", issue.issue_type));

        // Optional fields
        if let Some(desc) = &issue.description {
            result.push_str(&format!("\nDescription:\n{}\n", desc));
        }

        if let Some(design) = &issue.design {
            result.push_str(&format!("\nDesign:\n{}\n", design));
        }

        if let Some(criteria) = &issue.acceptance_criteria {
            result.push_str(&format!("\nAcceptance Criteria:\n{}\n", criteria));
        }

        if let Some(notes) = &issue.notes {
            result.push_str(&format!("\nNotes:\n{}\n", notes));
        }

        if let Some(assignee) = &issue.assignee {
            result.push_str(&format!("\nAssignee:     {}\n", assignee));
        }

        if let Some(owner) = &issue.owner {
            result.push_str(&format!("Owner:        {}\n", owner));
        }

        if let Some(external_ref) = &issue.external_ref {
            result.push_str(&format!("External Ref: {}\n", external_ref));
        }

        // Labels
        if !issue.labels.is_empty() {
            result.push_str(&format!("\nLabels:       {}\n", issue.labels.join(", ")));
        }

        // Annotations
        if !issue.annotations.is_empty() {
            result.push_str("\nAnnotations:\n");
            for (key, value) in &issue.annotations {
                result.push_str(&format!("  {}: {}\n", key, value));
            }
        }

        // Dependencies
        if !issue.dependencies.is_empty() {
            result.push_str("\nDependencies:\n");
            for dep in &issue.dependencies {
                let title = dep.title.as_deref().unwrap_or("Unknown");
                let blocking = if dep.dep_type.is_blocking() { " (blocks)" } else { "" };
                result.push_str(&format!("  {} - {}{}\n", dep.depends_on_id, title, blocking));
            }
        }

        // Comments
        if !issue.comments.is_empty() {
            result.push_str("\nComments:\n");
            for comment in &issue.comments {
                result.push_str(&format!(
                    "  [{} @ {}] {}\n",
                    comment.author,
                    comment.created_at.format("%Y-%m-%d %H:%M"),
                    comment.body
                ));
            }
        }

        // Timestamps
        result.push_str("\nTimestamps:\n");
        result.push_str(&format!("  Created:  {}\n", issue.created_at.format("%Y-%m-%d %H:%M:%S UTC")));
        result.push_str(&format!("  Updated:  {}\n", issue.updated_at.format("%Y-%m-%d %H:%M:%S UTC")));

        if let Some(closed_at) = &issue.closed_at {
            result.push_str(&format!("  Closed:   {}\n", closed_at.format("%Y-%m-%d %H:%M:%S UTC")));
        }

        if let Some(due_at) = &issue.due_at {
            result.push_str(&format!("  Due:      {}\n", due_at.format("%Y-%m-%d %H:%M:%S UTC")));
        }

        result
    }
}

impl Default for TableFormatter {
    fn default() -> Self {
        Self::new()
    }
}

/// Format priority with optional color coding
fn format_priority_colored(priority: Priority, colored: bool) -> String {
    let base = format!("{}", priority);
    if !colored {
        return base;
    }

    let color = match priority.0 {
        0 => Color::BrightRed,    // Critical
        1 => Color::Red,           // High
        2 => Color::Yellow,        // Medium
        3 => Color::Blue,          // Low
        4 => Color::BrightBlue,    // Backlog
        _ => Color::Reset,
    };

    crate::format::color::colorize(&base, color)
}

/// Limit a value to a maximum
fn min<T: Ord>(value: T, max: T) -> T {
    if value < max { value } else { max }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_format_empty_issues() {
        let formatter = TableFormatter::new();
        let result = formatter.format_issues(&[]);
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_single_issue() {
        let formatter = TableFormatter::new();
        let issue = Issue::new("bf-test".to_string(), "Test Issue".to_string(), ".".to_string());
        let result = formatter.format_issues(&[issue]);

        assert!(result.contains("ID"));
        assert!(result.contains("Title"));
        assert!(result.contains("Status"));
        assert!(result.contains("bf-test"));
        assert!(result.contains("Test Issue"));
    }

    #[test]
    fn test_format_multiple_issues() {
        let formatter = TableFormatter::new();
        let issue1 = Issue::new("bf-1".to_string(), "First Issue".to_string(), ".".to_string());
        let issue2 = Issue::new("bf-2".to_string(), "Second Issue".to_string(), ".".to_string());
        let result = formatter.format_issues(&[issue1, issue2]);

        assert!(result.contains("bf-1"));
        assert!(result.contains("bf-2"));
        assert!(result.contains("First Issue"));
        assert!(result.contains("Second Issue"));
    }

    #[test]
    fn test_format_issue_detail() {
        let formatter = TableFormatter::new();
        let mut issue = Issue::new("bf-test".to_string(), "Test Issue".to_string(), ".".to_string());
        issue.description = Some("This is a test description".to_string());

        let result = formatter.format_issue_detail(&issue);

        assert!(result.contains("bf-test"));
        assert!(result.contains("Test Issue"));
        assert!(result.contains("This is a test description"));
        assert!(result.contains("Status"));
        assert!(result.contains("Priority"));
    }

    #[test]
    fn test_format_with_color_disabled() {
        let formatter = TableFormatter::with_color(false);
        let issue = Issue::new("bf-test".to_string(), "Test Issue".to_string(), ".".to_string());
        let result = formatter.format_issues(&[issue]);

        // Should not contain ANSI codes when color is disabled
        assert!(!result.contains("\x1b["));
    }

    #[test]
    fn test_title_truncation() {
        let formatter = TableFormatter::new();
        let mut issue = Issue::new("bf-test".to_string(), "This is a very long title that should be truncated in the table view to maintain readability".to_string(), ".".to_string());

        let result = formatter.format_issues(&[issue]);

        // Long title should be truncated with "..."
        assert!(result.contains("..."));
    }

    // ==================== Separator formatting tests ====================

    #[test]
    fn test_separator_exact_equals_count() {
        let formatter = TableFormatter::new();
        let issue = Issue::new("bf-test".to_string(), "Test Issue".to_string(), ".".to_string());
        let result = formatter.format_issue_detail(&issue);

        // Find the separator line (should be exactly 80 equals signs)
        let lines: Vec<&str> = result.lines().collect();
        let separator_line = lines.get(1).expect("Should have a separator line");

        // Count the exact number of equals signs
        let equals_count = separator_line.chars().filter(|&c| c == '=').count();
        assert_eq!(equals_count, 80, "Separator should have exactly 80 equals signs, got {}", equals_count);

        // Verify it's all equals signs (no other characters except newline)
        assert!(separator_line.chars().all(|c| c == '='), "Separator should contain only equals signs");
    }

    #[test]
    fn test_separator_positioning_in_detail_output() {
        let formatter = TableFormatter::new();
        let issue = Issue::new("bf-abc123".to_string(), "Issue Title Here".to_string(), ".".to_string());
        let result = formatter.format_issue_detail(&issue);

        let lines: Vec<&str> = result.lines().collect();

        // Separator should be on line 2 (index 1) - between title and first field
        assert!(lines.len() > 1, "Should have at least 2 lines");
        let separator_line = lines[1];

        // Verify it's a separator
        assert!(separator_line.chars().all(|c| c == '='), "Line 2 should be separator with only equals");

        // Verify line before separator contains ID and title
        let header_line = lines[0];
        assert!(header_line.contains("bf-abc123"), "Line 1 should contain ID");
        assert!(header_line.contains("Issue Title Here"), "Line 1 should contain title");

        // Verify line after separator starts with "Status:"
        let first_field_line = lines.get(2).expect("Should have content after separator");
        assert!(first_field_line.starts_with("Status:"), "Line 3 should start with 'Status:'");
    }

    #[test]
    fn test_table_separator_construction() {
        let formatter = TableFormatter::new();
        let issue1 = Issue::new("bf-1".to_string(), "First".to_string(), ".".to_string());
        let issue2 = Issue::new("bf-2".to_string(), "Second".to_string(), ".".to_string());
        let result = formatter.format_issues(&[issue1, issue2]);

        let lines: Vec<&str> = result.lines().collect();

        // Find separator line (should be after header)
        let separator_line = lines.get(1).expect("Should have separator after header");

        // Verify separator pattern: dashes separated by "+-+"
        assert!(separator_line.contains("-+-"), "Separator should contain '-+-' separators");

        // Verify it starts and ends with dashes
        assert!(separator_line.starts_with('-'), "Separator should start with dashes");
        assert!(separator_line.ends_with('-'), "Separator should end with dashes");

        // Verify separator contains only dashes and plus signs
        assert!(separator_line.chars().all(|c| c == '-' || c == '+'),
                "Separator should contain only dashes and plus signs");
    }

    #[test]
    fn test_separator_width_matches_column_widths() {
        let formatter = TableFormatter::new();

        // Create issues with specific ID lengths to test separator width adaptation
        let issue1 = Issue::new("bf-short".to_string(), "Title".to_string(), ".".to_string());
        let issue2 = Issue::new("bf-very-long-id".to_string(), "Another Title".to_string(), ".".to_string());

        let result = formatter.format_issues(&[issue1, issue2]);
        let lines: Vec<&str> = result.lines().collect();

        let header_line = lines[0];
        let separator_line = lines[1];

        // Split both lines by " | " to get column widths
        let header_columns: Vec<&str> = header_line.split(" | ").collect();
        let separator_parts: Vec<&str> = separator_line.split("-+-").collect();

        // Verify number of separator parts matches number of header columns
        assert_eq!(separator_parts.len(), header_columns.len(),
                   "Separator parts count should match header columns count");

        // Verify each separator part matches corresponding header width
        for (i, (sep_part, header_col)) in separator_parts.iter().zip(header_columns.iter()).enumerate() {
            assert_eq!(sep_part.len(), header_col.len(),
                      "Separator part {} width should match header column width", i);
        }
    }

    #[test]
    fn test_separator_with_varied_content_widths() {
        let formatter = TableFormatter::new();

        // Create issues with varying field widths
        let issue1 = Issue::new("bf-a".to_string(), "Short".to_string(), ".".to_string());
        let issue2 = Issue::new("bf-very-long-issue-id".to_string(), "This is a much longer title".to_string(), ".".to_string());

        let result = formatter.format_issues(&[issue1, issue2]);
        let lines: Vec<&str> = result.lines().collect();

        let separator_line = lines[1];

        // Count separator components
        let dash_count = separator_line.chars().filter(|&c| c == '-').count();
        let plus_count = separator_line.chars().filter(|&c| c == '+').count();

        // Should have exactly 5 plus signs (6 columns -> 5 separators)
        assert_eq!(plus_count, 5, "Should have 5 plus signs for 6-column table");

        // Should have more dashes than minimum due to long content
        assert!(dash_count > 20, "Should have substantial dash count for column widths");
    }

    #[test]
    fn test_no_separator_for_empty_issues() {
        let formatter = TableFormatter::new();
        let result = formatter.format_issues(&[]);

        // Empty result should have no separator
        assert_eq!(result, "", "Empty issues should produce empty output with no separator");
    }

    #[test]
    fn test_single_issue_has_separator() {
        let formatter = TableFormatter::new();
        let issue = Issue::new("bf-single".to_string(), "Single Issue".to_string(), ".".to_string());
        let result = formatter.format_issues(&[issue]);

        let lines: Vec<&str> = result.lines().collect();

        // Should have header and separator at minimum
        assert!(lines.len() >= 2, "Should have header and separator");

        let separator_line = lines[1];
        assert!(separator_line.contains("-+-"), "Single issue should still have separator");
    }
}
