//! Ready command - list open beads in JSONL format
//!
//! This is the foundational ready command that lists all open (not closed) beads.
//! Dependency filtering will be added in a subsequent bead.

use crate::format::{get_formatter, OutputFormat};
use crate::model::{Issue, IssueFilter, Status};
use crate::storage::Storage;
use crate::Result;
use std::path::Path;

/// Run the ready command to list open beads.
///
/// # Arguments
/// * `beads_dir` - Path to the .beads directory
/// * `limit` - Maximum number of beads to return (0 = unlimited)
/// * `format` - Output format ("text", "json", "toon")
/// * `envelope` - Whether to wrap output in a JSON envelope
///
/// # Returns
/// * `Result<()>` - Success or error
///
/// # Output
/// For JSON format: outputs beads in JSONL format (one JSON object per line)
/// For other formats: uses the appropriate formatter
pub fn run_ready(
    beads_dir: &Path,
    limit: usize,
    format: &str,
    envelope: bool,
) -> Result<()> {
    // Load metadata and open storage
    let metadata = crate::config::load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;

    // Build filter to get all open (not closed) beads
    let mut filter = IssueFilter::default();
    filter.status = Some(Status::Open);

    // --limit 0 means unlimited (None in filter)
    if limit > 0 {
        filter.limit = Some(limit);
    }

    // Query for all open beads
    let issues = storage.list_issues(&filter)?;

    // Use the common formatter pattern for consistency with other commands
    let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
    let formatter = get_formatter(output_format);

    match output_format {
        OutputFormat::Json => {
            // Output beads in JSONL format (one JSON object per line)
            let jsonl = issues
                .iter()
                .map(|issue| {
                    let json_str = formatter.format_issue(issue);
                    // Resolve dependencies for proper JSON output
                    resolve_dependencies_for_json(&storage, issue, &json_str)
                })
                .collect::<Vec<_>>()
                .join("\n");

            if envelope {
                // Wrap in envelope with kind="ready"
                // Convert JSONL to JSON array for the envelope data field
                let data = if jsonl.is_empty() {
                    "[]".to_string()
                } else {
                    let objects: Vec<serde_json::Value> = jsonl
                        .lines()
                        .filter_map(|line| serde_json::from_str(line).ok())
                        .collect();
                    serde_json::to_string(&objects).unwrap_or_else(|_| "[]".to_string())
                };
                println!("{}", formatter.format_with_envelope("ready", &data));
            } else {
                // Raw JSONL output; empty ready prints `[]` as a special case
                if jsonl.is_empty() {
                    println!("[]");
                } else {
                    println!("{}", jsonl);
                }
            }
        }
        OutputFormat::Toon => {
            // Use the formatter to ensure consistent output
            let output = formatter.format_issues(&issues);
            if output.is_empty() {
                println!("No ready candidates");
            } else {
                print!("{}", output);
            }
        }
        OutputFormat::Text => {
            // Use the formatter to ensure consistent output
            let output = formatter.format_issues(&issues);
            if output.is_empty() {
                println!("No ready candidates");
            } else {
                print!("{}", output);
            }
        }
    }

    Ok(())
}

/// Resolve an issue's dependency edges to full bead details for JSON output.
///
/// This resolves each dependency from its edge representation (issue_id, depends_on_id, type)
/// to the target bead's full details (id, title, status, priority, dependency_type).
fn resolve_dependencies_for_json(storage: &Storage, issue: &Issue, json_line: &str) -> String {
    let mut value: serde_json::Value = match serde_json::from_str(json_line) {
        Ok(v) => v,
        Err(_) => return json_line.to_string(),
    };

    if let Some(obj) = value.as_object_mut() {
        let resolved: Vec<serde_json::Value> = issue
            .dependencies
            .iter()
            .filter_map(|dep| {
                storage.get_issue(&dep.depends_on_id).ok().flatten().map(|target| {
                    serde_json::json!({
                        "id": target.id,
                        "title": target.title,
                        "status": target.status,
                        "priority": target.priority,
                        "dependency_type": dep.dep_type,
                    })
                })
            })
            .collect();

        if resolved.is_empty() {
            obj.remove("dependencies");
        } else {
            obj.insert(
                "dependencies".to_string(),
                serde_json::Value::Array(resolved),
            );
        }
    }

    serde_json::to_string(&value).unwrap_or_else(|_| json_line.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Issue;
    use chrono::{DateTime, Utc};
    use std::path::PathBuf;

    #[test]
    fn test_resolve_dependencies_for_json_no_deps() {
        // Test with an issue that has no dependencies
        let issue = Issue {
            id: "bf-123".to_string(),
            title: "Test bead".to_string(),
            status: Status::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };

        let json_line = r#"{"id":"bf-123","title":"Test bead","status":"open"}"#;
        let result = resolve_dependencies_for_json(&storage_mock(), &issue, json_line);

        // Result should not include dependencies field
        assert!(result.contains("id"));
        assert!(!result.contains("dependencies"));
    }

    #[test]
    fn test_resolve_dependencies_for_json_with_deps() {
        // Test with an issue that has dependencies
        let mut issue = Issue {
            id: "bf-123".to_string(),
            title: "Test bead".to_string(),
            status: Status::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };

        // Add a dependency
        issue.dependencies.push(crate::model::Dependency {
            issue_id: "bf-123".to_string(),
            depends_on_id: "bf-456".to_string(),
            dep_type: crate::model::DependencyType::Blocks,
            created_at: Utc::now(),
            created_by: Some("test".to_string()),
            ..Default::default()
        });

        let json_line = r#"{"id":"bf-123","title":"Test bead","status":"open"}"#;
        let result = resolve_dependencies_for_json(&storage_mock(), &issue, json_line);

        // Result should include resolved dependencies
        assert!(result.contains("dependencies"));
        assert!(result.contains("bf-456"));
    }

    // Mock storage for testing
    fn storage_mock() -> Storage {
        // This is a placeholder - real tests would use a test database
        // For now, this just satisfies the type signature
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join("test_beads.db");
        Storage::open(&db_path).unwrap()
    }
}
