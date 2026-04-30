use crate::config::{find_beads_dir, load_config, get_default_prefix};
use crate::model::{DependencyType, Issue, IssueType, Priority};
use crate::storage::Storage;
use anyhow::{anyhow, Result};
use chrono::Utc;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum BatchOp {
    #[serde(rename = "create")]
    Create {
        title: String,
        #[serde(default = "default_type")]
        type_: String,
        #[serde(default = "default_priority")]
        priority: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        assignee: Option<String>,
        #[serde(default)]
        labels: Vec<String>,
    },
    #[serde(rename = "dep_add_blocker")]
    DepAddBlocker {
        parent: String,
        child: String,
    },
    #[serde(rename = "close")]
    Close {
        id: String,
        #[serde(default = "default_close_reason")]
        reason: String,
    },
}

fn default_type() -> String {
    "task".to_string()
}

fn default_priority() -> i32 {
    2
}

fn default_close_reason() -> String {
    "Completed".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub op: usize,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn execute_batch(
    storage: &Storage,
    ops: Vec<BatchOp>,
    workspace_dir: &std::path::Path,
) -> Result<Vec<BatchResult>> {
    let config = load_config(&find_beads_dir(workspace_dir)
        .ok_or_else(|| anyhow!("No .beads directory found"))?)?;

    storage.with_immediate_transaction(|tx| {
        let mut results = Vec::new();
        let mut created_ids = Vec::new();

        for (idx, op) in ops.iter().enumerate() {
            let result = match op {
                BatchOp::Create { title, type_, priority, description, assignee, labels } => {
                    match execute_create(tx, title, type_, *priority, description, assignee, labels, &config, &mut created_ids) {
                        Ok(id) => BatchResult {
                            op: idx,
                            status: "ok".to_string(),
                            id: Some(id.clone()),
                            error: None,
                        },
                        Err(e) => BatchResult {
                            op: idx,
                            status: "error".to_string(),
                            id: None,
                            error: Some(e.to_string()),
                        },
                    }
                }
                BatchOp::DepAddBlocker { parent, child } => {
                    let parent_resolved = resolve_reference(parent, &created_ids);
                    let child_resolved = resolve_reference(child, &created_ids);
                    match execute_dep_add_blocker(tx, &parent_resolved, &child_resolved) {
                        Ok(_) => BatchResult {
                            op: idx,
                            status: "ok".to_string(),
                            id: None,
                            error: None,
                        },
                        Err(e) => BatchResult {
                            op: idx,
                            status: "error".to_string(),
                            id: None,
                            error: Some(e.to_string()),
                        },
                    }
                }
                BatchOp::Close { id, reason } => {
                    let id_resolved = resolve_reference(id, &created_ids);
                    match execute_close(tx, &id_resolved, reason) {
                        Ok(_) => BatchResult {
                            op: idx,
                            status: "ok".to_string(),
                            id: None,
                            error: None,
                        },
                        Err(e) => BatchResult {
                            op: idx,
                            status: "error".to_string(),
                            id: None,
                            error: Some(e.to_string()),
                        },
                    }
                }
            };

            // Fail fast on error
            if result.status == "error" {
                return Err(anyhow!("{}", result.error.unwrap_or_default()));
            }

            results.push(result);
        }

        Ok(results)
    })
}

/// Resolve placeholder references like @0, @1 to actual created IDs
/// If the input is not a placeholder reference, return it as-is
fn resolve_reference(reference: &str, created_ids: &[String]) -> String {
    if let Some(rest) = reference.strip_prefix('@') {
        if let Ok(idx) = rest.parse::<usize>() {
            if idx < created_ids.len() {
                return created_ids[idx].clone();
            }
        }
    }
    reference.to_string()
}

fn execute_create(
    tx: &Connection,
    title: &str,
    type_: &str,
    priority: i32,
    description: &Option<String>,
    assignee: &Option<String>,
    labels: &[String],
    config: &crate::config::Config,
    created_ids: &mut Vec<String>,
) -> Result<String> {
    // Get count to generate ID
    let count: i64 = tx.query_row("SELECT COUNT(*) FROM issues", [], |row| row.get(0))?;
    let prefix = get_default_prefix(config);
    let id = crate::id::generate_id(prefix, count as usize);

    let now = Utc::now();
    let mut issue = Issue::new(id.clone(), title.to_string(), ".".to_string());
    issue.issue_type = IssueType::from_str(type_)
        .map_err(|e| anyhow!("Invalid type: {}", e))?;
    issue.priority = Priority(priority);
    issue.description = description.clone();
    issue.assignee = assignee.clone();
    issue.labels = labels.to_vec();

    // Insert issue
    tx.execute(
        "INSERT INTO issues (
            id, content_hash, title, description, design, acceptance_criteria, notes,
            status, priority, issue_type, assignee, owner, estimated_minutes,
            created_at, created_by, updated_at, closed_at, close_reason,
            closed_by_session, due_at, defer_until, external_ref, source_system,
            source_repo, deleted_at, deleted_by, delete_reason, original_type,
            compaction_level, compacted_at, compacted_at_commit, original_size,
            sender, ephemeral, pinned, is_template
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                  ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27,
                  ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36)",
        rusqlite::params![
            &issue.id,
            &issue.content_hash,
            &issue.title,
            &issue.description,
            &issue.design,
            &issue.acceptance_criteria,
            &issue.notes,
            issue.status.to_string(),
            &issue.priority,
            issue.issue_type.to_string(),
            &issue.assignee,
            &issue.owner,
            &issue.estimated_minutes,
            issue.created_at.to_rfc3339(),
            &issue.created_by,
            issue.updated_at.to_rfc3339(),
            issue.closed_at.map(|d| d.to_rfc3339()),
            &issue.close_reason,
            &issue.closed_by_session,
            issue.due_at.map(|d| d.to_rfc3339()),
            issue.defer_until.map(|d| d.to_rfc3339()),
            &issue.external_ref,
            &issue.source_system,
            &issue.source_repo,
            issue.deleted_at.map(|d| d.to_rfc3339()),
            &issue.deleted_by,
            &issue.delete_reason,
            &issue.original_type,
            &issue.compaction_level,
            issue.compacted_at.map(|d| d.to_rfc3339()),
            &issue.compacted_at_commit,
            &issue.original_size,
            &issue.sender,
            if issue.ephemeral { 1 } else { 0 },
            if issue.pinned { 1 } else { 0 },
            if issue.is_template { 1 } else { 0 },
        ],
    )?;

    // Insert labels
    for label in labels {
        tx.execute(
            "INSERT INTO labels (issue_id, label) VALUES (?1, ?2)",
            rusqlite::params![&id, label],
        )?;
    }

    created_ids.push(id.clone());
    Ok(id)
}

fn execute_dep_add_blocker(tx: &Connection, parent: &str, child: &str) -> Result<()> {
    // Verify both beads exist
    let parent_exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
        &[parent],
        |row| row.get(0),
    )?;

    let child_exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
        &[child],
        |row| row.get(0),
    )?;

    if !parent_exists {
        return Err(anyhow!("Parent bead not found: {}", parent));
    }
    if !child_exists {
        return Err(anyhow!("Child bead not found: {}", child));
    }

    // Add dependency (parent blocks child)
    let now = Utc::now();
    tx.execute(
        "INSERT INTO dependencies (issue_id, depends_on_id, type, created_at, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            child,
            parent,
            DependencyType::Blocks.to_string(),
            now.to_rfc3339(),
            "batch",
        ],
    )?;

    Ok(())
}

fn execute_close(tx: &Connection, id: &str, reason: &str) -> Result<()> {
    // Verify bead exists
    let exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
        &[id],
        |row| row.get(0),
    )?;

    if !exists {
        return Err(anyhow!("Bead not found: {}", id));
    }

    let now = Utc::now();
    tx.execute(
        "UPDATE issues SET status = 'closed', closed_at = ?, close_reason = ?, updated_at = ?
         WHERE id = ?",
        rusqlite::params![now.to_rfc3339(), reason, now.to_rfc3339(), id],
    )?;

    tx.execute(
        "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, created_at)
         VALUES (?1, 'closed', NULL, NULL, ?2, ?3)",
        rusqlite::params![id, reason, now.to_rfc3339()],
    )?;

    Ok(())
}

pub fn parse_stdin() -> Result<Vec<BatchOp>> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;

    // Try JSON first
    if let Ok(ops) = serde_json::from_str::<Vec<BatchOp>>(&input) {
        return Ok(ops);
    }

    // Fall back to CLI-style parsing (one op per line)
    let mut ops = Vec::new();
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Simple parsing for: create --title "X" --type Y
        if let Some(rest) = line.strip_prefix("create ") {
            ops.push(parse_create(rest)?);
        } else if let Some(rest) = line.strip_prefix("dep add-blocker ") {
            ops.push(parse_dep_add(rest)?);
        } else if let Some(rest) = line.strip_prefix("close ") {
            ops.push(parse_close(rest)?);
        } else {
            return Err(anyhow!("Unknown operation: {}", line));
        }
    }

    Ok(ops)
}

fn parse_create(input: &str) -> Result<BatchOp> {
    let mut title = None;
    let mut type_ = "task".to_string();
    let mut priority = 2;
    let mut description = None;

    let parts = shell_words::split(input)?;
    let mut i = 0;
    while i < parts.len() {
        match parts[i].as_str() {
            "--title" => {
                i += 1;
                if i < parts.len() {
                    title = Some(parts[i].clone());
                }
            }
            "--type" => {
                i += 1;
                if i < parts.len() {
                    type_ = parts[i].clone();
                }
            }
            "--priority" => {
                i += 1;
                if i < parts.len() {
                    priority = parts[i].parse().unwrap_or(2);
                }
            }
            "--description" => {
                i += 1;
                if i < parts.len() {
                    description = Some(parts[i].clone());
                }
            }
            _ => {
                if title.is_none() {
                    title = Some(parts[i].clone());
                }
            }
        }
        i += 1;
    }

    let title = title.ok_or_else(|| anyhow!("Missing title for create operation"))?;

    Ok(BatchOp::Create {
        title,
        type_,
        priority,
        description,
        assignee: None,
        labels: Vec::new(),
    })
}

fn parse_dep_add(input: &str) -> Result<BatchOp> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(anyhow!("dep add-blocker requires parent and child IDs"));
    }
    Ok(BatchOp::DepAddBlocker {
        parent: parts[0].to_string(),
        child: parts[1].to_string(),
    })
}

fn parse_close(input: &str) -> Result<BatchOp> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    let id = parts.first().ok_or_else(|| anyhow!("Missing ID for close operation"))?;
    let reason = if parts.len() > 1 {
        parts[1..].join(" ")
    } else {
        "Completed".to_string()
    };
    Ok(BatchOp::Close {
        id: id.to_string(),
        reason,
    })
}
