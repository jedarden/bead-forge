//! Git history reconstruction for event log.
//!
//! Provides read-only access to git log history of .beads/issues.jsonl
//! to reconstruct events older than the SQLite events table retention window.

use crate::model::{Event, EventType, Issue, Status};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// A snapshot of the JSONL file at a specific commit.
pub struct JsonlSnapshot {
    pub commit_hash: String,
    pub commit_date: DateTime<Utc>,
    pub issues: HashMap<String, Issue>,
}

/// Parse git log to get snapshots of issues.jsonl at each commit.
///
/// Runs: git log --follow --date=iso-strict -- .beads/issues.jsonl
/// Then for each commit, runs: git show <commit>:path to get full contents
pub fn parse_git_log_snapshots(workspace: &Path, jsonl_path: &Path) -> Result<Vec<JsonlSnapshot>> {
    let relative_path = jsonl_path.strip_prefix(workspace).unwrap_or(jsonl_path);
    let path_str = relative_path.to_str().unwrap_or(".beads/issues.jsonl");

    // First, get list of commits that touched this file
    let output = Command::new("git")
        .args(["log", "--follow", "--format=%H|%ci", "--", path_str])
        .current_dir(workspace)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8(out.stdout)?;
            parse_snapshots_from_commits(workspace, path_str, &stdout)
        }
        _ => {
            // Not in git repo or file has no history - return empty
            Ok(Vec::new())
        }
    }
}

/// Parse git log output and create snapshots from each commit.
fn parse_snapshots_from_commits(
    workspace: &Path,
    path_str: &str,
    git_output: &str,
) -> Result<Vec<JsonlSnapshot>> {
    let mut snapshots = Vec::new();

    // Parse each commit and get the file contents at that commit
    for line in git_output.lines() {
        let parts: Vec<&str> = line.splitn(2, '|').collect();
        if parts.len() != 2 {
            continue;
        }

        let commit_hash = parts[0].to_string();
        let commit_date_str = parts[1];

        // Parse the date (git log --format=%ci gives "2024-01-01 12:00:00 +0000")
        let commit_date = parse_git_date(commit_date_str)?;

        // Get file contents at this commit
        let file_output = Command::new("git")
            .args(["show", &format!("{}:{}", commit_hash, path_str)])
            .current_dir(workspace)
            .output();

        let issues = match file_output {
            Ok(out) if out.status.success() => {
                let content = String::from_utf8_lossy(&out.stdout);
                parse_jsonl_issues(&content)?
            }
            _ => {
                // File didn't exist at this commit (or was empty)
                HashMap::new()
            }
        };

        snapshots.push(JsonlSnapshot {
            commit_hash,
            commit_date,
            issues,
        });
    }

    // git log returns newest first, we want oldest first
    snapshots.reverse();
    Ok(snapshots)
}

/// Parse git date format "2024-01-01 12:00:00 +0000" to DateTime<Utc>
fn parse_git_date(date_str: &str) -> Result<DateTime<Utc>> {
    use chrono::NaiveDateTime;
    // git log --format=%ci gives "2024-01-01 12:00:00 +0000"
    let trimmed = date_str.trim();
    // Parse "YYYY-MM-DD HH:MM:SS +ZZZZ" format
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S %z") {
        return Ok(DateTime::from_naive_utc_and_offset(dt, Utc));
    }

    // Fallback: try parsing as ISO-8601
    DateTime::parse_from_rfc3339(trimmed)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| anyhow!("Failed to parse git date: {}", date_str))
}

/// Parse JSONL content into a map of issues.
fn parse_jsonl_issues(content: &str) -> Result<HashMap<String, Issue>> {
    let mut issues = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(issue) = serde_json::from_str::<Issue>(line) {
            issues.insert(issue.id.clone(), issue);
        }
    }
    Ok(issues)
}

/// Reconstruct events from git log history of issues.jsonl.
///
/// Parses git log to get the JSONL file state at each commit, then
/// creates synthetic events for state transitions:
/// - New bead appearing = synthetic created event
/// - Status open->in_progress = synthetic claimed event
/// - Status->closed = synthetic closed event with duration_seconds
///
/// All synthetic events get actor="git-reconstructed".
pub fn reconstruct_events_from_git(
    workspace: &Path,
    jsonl_path: &Path,
    filter_issue_id: Option<&str>,
) -> Result<Vec<Event>> {
    let snapshots = parse_git_log_snapshots(workspace, jsonl_path)?;

    if snapshots.is_empty() {
        return Ok(Vec::new());
    }

    let mut events = Vec::new();
    let mut next_event_id: i64 = -1000; // Negative IDs to distinguish from SQLite events

    // Process snapshots chronologically (oldest to newest)
    for window in snapshots.windows(2) {
        let prev = &window[0];
        let curr = &window[1];

        // Find differences between snapshots and create synthetic events
        for (issue_id, current_issue) in &curr.issues {
            // Apply issue_id filter if provided
            if let Some(filter_id) = filter_issue_id {
                if issue_id != filter_id {
                    continue;
                }
            }

            let prev_issue = prev.issues.get(issue_id);

            match prev_issue {
                None => {
                    // New bead appeared - create synthetic created event
                    events.push(Event {
                        id: next_event_id,
                        issue_id: issue_id.clone(),
                        event_type: EventType::Created,
                        actor: "git-reconstructed".to_string(),
                        old_value: None,
                        new_value: None,
                        comment: None,
                        created_at: current_issue.created_at,
                    });
                    next_event_id -= 1;

                    // If already in_progress at creation, add claimed event
                    if current_issue.status == Status::InProgress {
                        events.push(Event {
                            id: next_event_id,
                            issue_id: issue_id.clone(),
                            event_type: EventType::StatusChanged,
                            actor: "git-reconstructed".to_string(),
                            old_value: Some(Status::Open.as_str().to_string()),
                            new_value: Some(Status::InProgress.as_str().to_string()),
                            comment: None,
                            created_at: current_issue.created_at,
                        });
                        next_event_id -= 1;
                    }

                    // If already closed at creation, add closed event
                    if current_issue.status == Status::Closed {
                        events.push(Event {
                            id: next_event_id,
                            issue_id: issue_id.clone(),
                            event_type: EventType::Closed,
                            actor: "git-reconstructed".to_string(),
                            old_value: None,
                            new_value: current_issue.close_reason.clone(),
                            comment: None,
                            created_at: current_issue.closed_at.unwrap_or(current_issue.created_at),
                        });
                        next_event_id -= 1;
                    }
                }
                Some(previous_issue) => {
                    // Existing bead - check for status changes
                    if previous_issue.status != current_issue.status {
                        match (previous_issue.status.clone(), current_issue.status.clone()) {
                            (Status::Open, Status::InProgress) => {
                                // open -> in_progress = claimed
                                events.push(Event {
                                    id: next_event_id,
                                    issue_id: issue_id.clone(),
                                    event_type: EventType::StatusChanged,
                                    actor: "git-reconstructed".to_string(),
                                    old_value: Some(Status::Open.as_str().to_string()),
                                    new_value: Some(Status::InProgress.as_str().to_string()),
                                    comment: None,
                                    created_at: curr.commit_date,
                                });
                                next_event_id -= 1;
                            }
                            (_, Status::Closed) => {
                                // any -> closed = closed event
                                events.push(Event {
                                    id: next_event_id,
                                    issue_id: issue_id.clone(),
                                    event_type: EventType::Closed,
                                    actor: "git-reconstructed".to_string(),
                                    old_value: None,
                                    new_value: current_issue.close_reason.clone(),
                                    comment: None,
                                    created_at: curr.commit_date,
                                });
                                next_event_id -= 1;
                            }
                            _ => {
                                // Other status changes - generic status_changed event
                                events.push(Event {
                                    id: next_event_id,
                                    issue_id: issue_id.clone(),
                                    event_type: EventType::StatusChanged,
                                    actor: "git-reconstructed".to_string(),
                                    old_value: Some(previous_issue.status.as_str().to_string()),
                                    new_value: Some(current_issue.status.as_str().to_string()),
                                    comment: None,
                                    created_at: curr.commit_date,
                                });
                                next_event_id -= 1;
                            }
                        }
                    }

                    // Check for assignee changes
                    if previous_issue.assignee != current_issue.assignee {
                        events.push(Event {
                            id: next_event_id,
                            issue_id: issue_id.clone(),
                            event_type: EventType::AssigneeChanged,
                            actor: "git-reconstructed".to_string(),
                            old_value: previous_issue.assignee.clone(),
                            new_value: current_issue.assignee.clone(),
                            comment: None,
                            created_at: curr.commit_date,
                        });
                        next_event_id -= 1;
                    }

                    // Check for priority changes
                    if previous_issue.priority != current_issue.priority {
                        events.push(Event {
                            id: next_event_id,
                            issue_id: issue_id.clone(),
                            event_type: EventType::PriorityChanged,
                            actor: "git-reconstructed".to_string(),
                            old_value: Some(previous_issue.priority.0.to_string()),
                            new_value: Some(current_issue.priority.0.to_string()),
                            comment: None,
                            created_at: curr.commit_date,
                        });
                        next_event_id -= 1;
                    }
                }
            }
        }

        // Check for deleted beads (tombstones)
        for issue_id in prev.issues.keys() {
            // Apply issue_id filter if provided
            if let Some(filter_id) = filter_issue_id {
                if issue_id != filter_id {
                    continue;
                }
            }

            if !curr.issues.contains_key(issue_id) {
                // Bead was deleted - create synthetic deleted event
                events.push(Event {
                    id: next_event_id,
                    issue_id: issue_id.clone(),
                    event_type: EventType::Deleted,
                    actor: "git-reconstructed".to_string(),
                    old_value: None,
                    new_value: None,
                    comment: None,
                    created_at: curr.commit_date,
                });
                next_event_id -= 1;
            }
        }
    }

    Ok(events)
}

/// Merge SQLite events with git-reconstructed events.
///
/// When git events are provided, this function:
/// 1. Combines both event lists
/// 2. Sorts by created_at timestamp
/// 3. Deduplicates based on timestamp and event type (SQLite takes precedence)
pub fn merge_events(sqlite_events: Vec<Event>, git_events: Vec<Event>) -> Vec<Event> {
    if git_events.is_empty() {
        return sqlite_events;
    }

    let mut all_events = Vec::with_capacity(sqlite_events.len() + git_events.len());

    // Add all SQLite events (positive IDs)
    for event in sqlite_events {
        all_events.push(event);
    }

    // Add git events, filtering out obvious duplicates
    // (same issue_id, event_type, and similar timestamp)
    for git_event in git_events {
        let is_duplicate = all_events.iter().any(|sqlite_event| {
            sqlite_event.issue_id == git_event.issue_id
                && sqlite_event.event_type == git_event.event_type
                && sqlite_event.created_at == git_event.created_at
        });

        if !is_duplicate {
            all_events.push(git_event);
        }
    }

    // Sort by created_at, then by id (git events have negative ids, so they come first for same timestamp)
    all_events.sort_by(|a, b| {
        a.created_at
            .cmp(&b.created_at)
            .then_with(|| a.id.cmp(&b.id))
    });

    all_events
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_git_date() {
        let date_str = "2024-01-01 12:00:00 +0000";
        let result = parse_git_date(date_str);
        assert!(result.is_ok());
        let dt = result.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 1);
    }

    #[test]
    fn test_parse_jsonl_issues() {
        let jsonl = r#"{"id":"bf-1","title":"Test","status":"open","priority":2,"issue_type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z"}
{"id":"bf-2","title":"Test 2","status":"closed","priority":2,"issue_type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z"}"#;
        let issues = parse_jsonl_issues(jsonl).unwrap();
        assert_eq!(issues.len(), 2);
        assert!(issues.contains_key("bf-1"));
        assert!(issues.contains_key("bf-2"));
    }

    #[test]
    fn test_merge_events() {
        use chrono::NaiveDateTime;

        let sqlite_events = vec![Event {
            id: 1,
            issue_id: "bf-1".to_string(),
            event_type: EventType::Created,
            actor: "sqlite".to_string(),
            old_value: None,
            new_value: None,
            comment: None,
            created_at: DateTime::from_naive_utc_and_offset(
                NaiveDateTime::from_timestamp_opt(1704067200, 0).unwrap(),
                Utc,
            ),
        }];

        let git_events = vec![Event {
            id: -1,
            issue_id: "bf-1".to_string(),
            event_type: EventType::StatusChanged,
            actor: "git-reconstructed".to_string(),
            old_value: Some("open".to_string()),
            new_value: Some("in_progress".to_string()),
            comment: None,
            created_at: DateTime::from_naive_utc_and_offset(
                NaiveDateTime::from_timestamp_opt(1704067200, 0).unwrap(),
                Utc,
            ),
        }];

        let merged = merge_events(sqlite_events, git_events);
        assert_eq!(merged.len(), 2);
    }
}
