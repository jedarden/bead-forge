use crate::config::{find_beads_dir, get_default_prefix, load_config};
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
        /// The bead being blocked (must close after blocker closes)
        #[serde(alias = "child")]
        id: String,
        /// The bead that blocks (must close before id can close)
        #[serde(alias = "parent")]
        blocker: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Allowed field names for each operation type (for validation)
fn get_allowed_fields(op_name: &str) -> &'static [&'static str] {
    match op_name {
        "create" => &[
            "op",
            "title",
            "type",
            "priority",
            "description",
            "assignee",
            "labels",
        ],
        "dep_add_blocker" => &["op", "id", "blocker", "parent", "child"],
        "close" => &["op", "id", "reason"],
        _ => &[],
    }
}

/// Validate operation object keys before parsing (loud validation for unknown fields)
///
/// This pre-validation step catches typos and unknown fields early with clear error messages.
/// serde's `deny_unknown_fields` doesn't work with internally tagged enums, so we validate
/// at the Value level before typed parsing.
fn validate_op_fields(value: &serde_json::Value) -> Result<()> {
    let obj = value
        .as_object()
        .ok_or_else(|| anyhow!("Operation must be a JSON object"))?;

    let op_name = obj
        .get("op")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Operation must have an 'op' field"))?;

    let allowed = get_allowed_fields(op_name);

    for key in obj.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(anyhow!(
                "Unknown field '{}' in operation '{}'. Allowed fields: {}",
                key,
                op_name,
                allowed.join(", ")
            ));
        }
    }

    Ok(())
}

pub fn execute_batch(
    storage: &Storage,
    ops: Vec<BatchOp>,
    workspace_dir: &std::path::Path,
) -> Result<Vec<BatchResult>> {
    let config = load_config(
        &find_beads_dir(workspace_dir).ok_or_else(|| anyhow!("No .beads directory found"))?,
    )?;

    storage.with_immediate_transaction(|tx| {
        let mut results = Vec::new();
        let mut created_ids = Vec::new();

        for (idx, op) in ops.iter().enumerate() {
            let result = match op {
                BatchOp::Create {
                    title,
                    type_,
                    priority,
                    description,
                    assignee,
                    labels,
                } => {
                    match execute_create(
                        tx,
                        title,
                        type_,
                        *priority,
                        description,
                        assignee,
                        labels,
                        &config,
                        &mut created_ids,
                    ) {
                        Ok(id) => BatchResult {
                            op: idx,
                            status: "ok".to_string(),
                            id: Some(id.clone()),
                            error: None,
                            message: Some(format!("Created bead {}", id)),
                        },
                        Err(e) => BatchResult {
                            op: idx,
                            status: "error".to_string(),
                            id: None,
                            error: Some(e.to_string()),
                            message: None,
                        },
                    }
                }
                BatchOp::DepAddBlocker { id, blocker } => {
                    let id_resolved = resolve_reference(id, &created_ids);
                    let blocker_resolved = resolve_reference(blocker, &created_ids);
                    match execute_dep_add_blocker(tx, &id_resolved, &blocker_resolved) {
                        Ok(_) => BatchResult {
                            op: idx,
                            status: "ok".to_string(),
                            id: None,
                            error: None,
                            message: Some(format!(
                                "ok: {} blocked by {}",
                                id_resolved, blocker_resolved
                            )),
                        },
                        Err(e) => BatchResult {
                            op: idx,
                            status: "error".to_string(),
                            id: None,
                            error: Some(e.to_string()),
                            message: None,
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
                            message: Some(format!("Closed bead {}", id_resolved)),
                        },
                        Err(e) => BatchResult {
                            op: idx,
                            status: "error".to_string(),
                            id: None,
                            error: Some(e.to_string()),
                            message: None,
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

    let mut issue = Issue::new(String::new(), title.to_string(), ".".to_string());
    issue.issue_type = IssueType::from_str(type_).map_err(|e| anyhow!("Invalid type: {}", e))?;
    issue.priority = Priority(priority);
    issue.description = description.clone().or_else(|| Some(String::new()));
    issue.assignee = assignee.clone();
    issue.labels = labels.to_vec();

    // Insert issue. Short IDs are sized for ~1% collision probability by
    // design, so re-roll the ID on a colliding INSERT instead of failing.
    let mut id = String::new();
    let mut inserted = false;
    let mut last_err: Option<rusqlite::Error> = None;
    for _ in 0..5 {
        id = crate::id::generate_id(prefix, count as usize);
        issue.id = id.clone();
        let insert_result = tx.execute(
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
                issue.description.as_deref().unwrap_or(""),
                issue.design.as_deref().unwrap_or(""),
                issue.acceptance_criteria.as_deref().unwrap_or(""),
                issue.notes.as_deref().unwrap_or(""),
                issue.status.to_string(),
                &issue.priority,
                &issue.issue_type.to_string(),
                &issue.assignee,
                &issue.owner,
                &issue.estimated_minutes,
                issue.created_at.to_rfc3339(),
                &issue.created_by,
                issue.updated_at.to_rfc3339(),
                issue.closed_at.map(|d| d.to_rfc3339()),
                issue.close_reason.as_deref().unwrap_or(""),
                issue.closed_by_session.as_deref().unwrap_or(""),
                issue.due_at.map(|d| d.to_rfc3339()),
                issue.defer_until.map(|d| d.to_rfc3339()),
                issue.external_ref.as_deref(),
                issue.source_system.as_deref().unwrap_or(""),
                &issue.source_repo,
                issue.deleted_at.map(|d| d.to_rfc3339()),
                issue.deleted_by.as_deref().unwrap_or(""),
                issue.delete_reason.as_deref().unwrap_or(""),
                issue.original_type.as_deref().unwrap_or(""),
                &issue.compaction_level,
                issue.compacted_at.map(|d| d.to_rfc3339()),
                issue.compacted_at_commit.as_deref().unwrap_or(""),
                &issue.original_size,
                issue.sender.as_deref().unwrap_or(""),
                if issue.ephemeral { 1 } else { 0 },
                if issue.pinned { 1 } else { 0 },
                if issue.is_template { 1 } else { 0 },
            ],
        );
        match insert_result {
            Ok(_) => {
                inserted = true;
                break;
            }
            Err(e)
                if e.to_string()
                    .contains("UNIQUE constraint failed: issues.id") =>
            {
                last_err = Some(e);
            }
            Err(e) => return Err(e.into()),
        }
    }
    if !inserted {
        return Err(anyhow!(
            "ID collision retries exhausted: {}",
            last_err.map(|e| e.to_string()).unwrap_or_default()
        ));
    }

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

/// Execute a dep_add_blocker operation (unified with CLI dep add command)
///
/// # Arguments
/// * `id` - The bead being blocked (must close after blocker closes)
/// * `blocker` - The bead that blocks (must close before id can close)
///
/// # Returns
/// * `Ok(())` if the dependency was added successfully
/// * `Err(...)` if validation fails (cycle, duplicate, missing beads)
///
/// # Direction
/// Creates: id depends on blocker (blocker blocks id)
/// This matches: `bf dep add <blocker> --blocks <id>`
fn execute_dep_add_blocker(tx: &Connection, id: &str, blocker: &str) -> Result<()> {
    // Verify both beads exist
    let id_exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
        &[id],
        |row| row.get(0),
    )?;

    let blocker_exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
        &[blocker],
        |row| row.get(0),
    )?;

    if !id_exists {
        return Err(anyhow!("Bead not found: {}", id));
    }
    if !blocker_exists {
        return Err(anyhow!("Bead not found: {}", blocker));
    }

    // Check for duplicate dependency
    let exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM dependencies WHERE issue_id = ?1 AND depends_on_id = ?2)",
        &[id, blocker],
        |row| row.get(0),
    )?;

    if exists {
        return Err(anyhow!(
            "Dependency already exists: {} depends on {}",
            id,
            blocker
        ));
    }

    // Check for circular dependency (id -> blocker and blocker -> id)
    let reverse_exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM dependencies WHERE issue_id = ?1 AND depends_on_id = ?2)",
        &[blocker, id],
        |row| row.get(0),
    )?;

    if reverse_exists {
        return Err(anyhow!(
            "Circular dependency detected: {} <-> {}",
            id,
            blocker
        ));
    }

    // Add dependency (id depends on blocker, so blocker blocks id)
    let now = Utc::now();
    tx.execute(
        "INSERT INTO dependencies (issue_id, depends_on_id, type, created_at, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            id,
            blocker,
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

    // Check if already closed for idempotence
    let current_status: Option<String> = tx
        .query_row(
            "SELECT status FROM issues WHERE id = ?1",
            &[id],
            |row| row.get(0),
        )
        .ok();

    if current_status.as_deref() == Some("closed") {
        // Already closed - idempotent, return success
        return Ok(());
    }

    let now = Utc::now();
    tx.execute(
        "UPDATE issues SET status = 'closed', closed_at = ?, close_reason = ?, updated_at = ?
         WHERE id = ?",
        rusqlite::params![now.to_rfc3339(), reason, now.to_rfc3339(), id],
    )?;

    tx.execute(
        "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, created_at)
         VALUES (?1, 'closed', '', '', ?2, ?3)",
        rusqlite::params![id, reason, now.to_rfc3339()],
    )?;

    // Cascade status transition: find dependents that should move from blocked->open
    // A dependent should be unblocked if it has no remaining blockers in non-terminal status
    let dependents: Vec<(String, String)> = tx
        .prepare(
            "SELECT d.issue_id, i.status
             FROM dependencies d
             INNER JOIN issues i ON i.id = d.issue_id
             WHERE d.depends_on_id = ?1
             AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')",
        )?
        .query_map(&[id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    for (dep_id, dep_status) in dependents {
        // Only transition beads currently at status='blocked'
        if dep_status != "blocked" {
            continue;
        }

        // Check if the dependent has any OTHER remaining blockers not in terminal status
        let remaining_blockers: i64 = tx
            .query_row(
                r#"
                SELECT COUNT(DISTINCT d.depends_on_id)
                FROM dependencies d
                INNER JOIN issues i ON i.id = d.depends_on_id
                WHERE d.issue_id = ?1
                AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
                AND i.status NOT IN ('closed', 'tombstone', 'done', 'completed')
                "#,
                &[&dep_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // If no remaining blockers, transition to 'open'
        if remaining_blockers == 0 {
            let now = Utc::now();
            tx.execute(
                "UPDATE issues SET status = 'open', updated_at = ?1 WHERE id = ?2",
                rusqlite::params![now.to_rfc3339(), &dep_id],
            )?;
            tx.execute(
                "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, created_at) VALUES (?1, 'status_changed', 'system', 'blocked', 'open', ?2)",
                rusqlite::params![&dep_id, now.to_rfc3339()],
            )?;
        }
    }

    // Rebuild blocked_issues_cache to reflect the new blocker state
    tx.execute("DELETE FROM blocked_issues_cache", [])?;
    tx.execute(
        "INSERT INTO blocked_issues_cache (issue_id, blocked_by, blocked_at)
         SELECT d.issue_id, '[' || GROUP_CONCAT('\"' || d.depends_on_id || '\"') || ']' AS blocked_by, ?1
         FROM dependencies d
         INNER JOIN issues i ON i.id = d.depends_on_id
         WHERE d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
         AND i.status NOT IN ('closed', 'tombstone', 'done', 'completed')
         GROUP BY d.issue_id",
        rusqlite::params![Utc::now().to_rfc3339()],
    )?;

    // Invalidate critical path cache: closing a bead can unblock dependents
    crate::critical_path::invalidate_cache(tx)?;
    crate::critical_path::compute_all_critical_paths(tx)?;

    Ok(())
}

/// Mitosis: split a parent bead into multiple child beads atomically.
///
/// This function constructs a batch of operations that:
/// 1. Creates N child beads
/// 2. Adds dependencies (each child blocks the parent) for each child
/// 3. Closes the parent bead
///
/// All operations run in a single BEGIN IMMEDIATE transaction, so there's no
/// risk of orphaned children if the process crashes midway.
///
/// # Arguments
/// * `parent_id` - The ID of the parent bead to split
/// * `children` - Vector of (title, type_, priority) tuples for each child
/// * `close_reason` - Reason for closing the parent (default: "Split into children")
///
/// # Returns
/// * `Ok(Vec<BatchOp>)` - Batch operations ready for execute_batch()
///
/// # Example
/// ```ignore
/// let ops = mitosis("bf-123", vec![
///     ("Child 1".to_string(), "task".to_string(), 2),
///     ("Child 2".to_string(), "bug".to_string(), 0),
/// ], None)?;
/// let results = execute_batch(&storage, ops, &workspace_dir)?;
/// ```
pub fn mitosis(
    parent_id: &str,
    children: Vec<(String, String, i32)>,
    close_reason: Option<String>,
) -> Result<Vec<BatchOp>> {
    let mut ops = Vec::new();

    // Create child beads
    for (title, type_, priority) in &children {
        ops.push(BatchOp::Create {
            title: title.clone(),
            type_: type_.clone(),
            priority: *priority,
            description: None,
            assignee: None,
            labels: Vec::new(),
        });
    }

    // Add dependencies: each child blocks the parent
    // Reference children by placeholder (@0, @1, etc.)
    for (idx, _) in children.iter().enumerate() {
        ops.push(BatchOp::DepAddBlocker {
            id: parent_id.to_string(),    // parent is blocked
            blocker: format!("@{}", idx), // child blocks parent
        });
    }

    // Close the parent
    ops.push(BatchOp::Close {
        id: parent_id.to_string(),
        reason: close_reason.unwrap_or_else(|| "Split into children".to_string()),
    });

    Ok(ops)
}

/// Mitosis with extended options (description, assignee, labels).
///
/// Same as mitosis() but allows full control over child bead properties.
pub fn mitosis_ex(
    parent_id: &str,
    children: Vec<MitosisChild>,
    close_reason: Option<String>,
) -> Result<Vec<BatchOp>> {
    let mut ops = Vec::new();

    for child in &children {
        ops.push(BatchOp::Create {
            title: child.title.clone(),
            type_: child.type_.clone(),
            priority: child.priority,
            description: child.description.clone(),
            assignee: child.assignee.clone(),
            labels: child.labels.clone(),
        });
    }

    for (idx, _) in children.iter().enumerate() {
        ops.push(BatchOp::DepAddBlocker {
            id: parent_id.to_string(),    // parent is blocked
            blocker: format!("@{}", idx), // child blocks parent
        });
    }

    ops.push(BatchOp::Close {
        id: parent_id.to_string(),
        reason: close_reason.unwrap_or_else(|| "Split into children".to_string()),
    });

    Ok(ops)
}

/// Extended child bead definition for mitosis_ex.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MitosisChild {
    pub title: String,
    #[serde(default = "default_type")]
    pub type_: String,
    #[serde(default = "default_priority")]
    pub priority: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
}

pub fn parse_stdin() -> Result<Vec<BatchOp>> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;

    // Try JSON first with validation
    if let Ok(json_values) = serde_json::from_str::<Vec<serde_json::Value>>(&input) {
        // Validate each operation's fields before parsing
        for value in &json_values {
            validate_op_fields(value)?;
        }
        // Now parse the validated JSON
        return serde_json::from_str::<Vec<BatchOp>>(&input)
            .map_err(|e| anyhow!("JSON parse error: {}", e));
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
        return Err(anyhow!(
            "dep add-blocker requires <id> <blocker>. Usage: dep add-blocker <blocked-bead> <blocking-bead>"
        ));
    }
    Ok(BatchOp::DepAddBlocker {
        id: parts[0].to_string(),      // bead being blocked
        blocker: parts[1].to_string(), // bead that blocks
    })
}

fn parse_close(input: &str) -> Result<BatchOp> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    let id = parts
        .first()
        .ok_or_else(|| anyhow!("Missing ID for close operation"))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::generate_id;
    use crate::model::Status;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_reference_placeholder() {
        let created_ids = vec![
            "bf-001".to_string(),
            "bf-002".to_string(),
            "bf-003".to_string(),
        ];
        assert_eq!(resolve_reference("@0", &created_ids), "bf-001");
        assert_eq!(resolve_reference("@1", &created_ids), "bf-002");
        assert_eq!(resolve_reference("@2", &created_ids), "bf-003");
    }

    #[test]
    fn test_resolve_reference_passthrough() {
        let created_ids = vec!["bf-001".to_string()];
        assert_eq!(resolve_reference("bf-parent", &created_ids), "bf-parent");
        assert_eq!(resolve_reference("literal-id", &created_ids), "literal-id");
    }

    #[test]
    fn test_resolve_reference_out_of_bounds() {
        let created_ids = vec!["bf-001".to_string()];
        // Out-of-bounds @-ref returns the reference as-is
        assert_eq!(resolve_reference("@5", &created_ids), "@5");
    }

    #[test]
    fn test_resolve_reference_empty_created_ids() {
        let created_ids: Vec<String> = vec![];
        assert_eq!(resolve_reference("@0", &created_ids), "@0");
    }

    #[test]
    fn test_serde_alias_compatibility() {
        // Test that old {parent, child} syntax maps to {blocker, id}
        // parent -> blocker (the bead that blocks)
        // child -> id (the bead being blocked)

        // Old syntax (deprecated but still supported via alias)
        let old_json = r#"{"op":"dep_add_blocker","parent":"bf-blocker","child":"bf-blocked"}"#;
        let op_old: BatchOp = serde_json::from_str(old_json).unwrap();

        // New canonical syntax
        let new_json = r#"{"op":"dep_add_blocker","id":"bf-blocked","blocker":"bf-blocker"}"#;
        let op_new: BatchOp = serde_json::from_str(new_json).unwrap();

        // Both should parse to the same struct
        match (op_old, op_new) {
            (
                BatchOp::DepAddBlocker {
                    id: id1,
                    blocker: blocker1,
                },
                BatchOp::DepAddBlocker {
                    id: id2,
                    blocker: blocker2,
                },
            ) => {
                assert_eq!(id1, "bf-blocked");
                assert_eq!(blocker1, "bf-blocker");
                assert_eq!(id1, id2);
                assert_eq!(blocker1, blocker2);
            }
            _ => panic!("Both should parse to DepAddBlocker"),
        }
    }

    #[test]
    fn test_validate_op_fields_rejects_unknown_field() {
        let bad_json = serde_json::json!({"op": "dep_add_blocker", "id": "bf-1", "typo": "value"});
        let result = validate_op_fields(&bad_json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Unknown field 'typo'"));
        assert!(err.to_string().contains("Allowed fields"));
    }

    #[test]
    fn test_validate_op_fields_accepts_aliases() {
        // Both old and new syntax should validate
        let new_json =
            serde_json::json!({"op": "dep_add_blocker", "id": "bf-1", "blocker": "bf-2"});
        assert!(validate_op_fields(&new_json).is_ok());

        let old_json =
            serde_json::json!({"op": "dep_add_blocker", "parent": "bf-2", "child": "bf-1"});
        assert!(validate_op_fields(&old_json).is_ok());
    }

    #[test]
    fn test_validate_op_fields_allows_all_create_fields() {
        let valid_json = serde_json::json!({
            "op": "create",
            "title": "Test",
            "type": "bug",
            "priority": 0,
            "description": "A test bead",
            "assignee": "worker-1",
            "labels": ["urgent", "bug"]
        });
        assert!(validate_op_fields(&valid_json).is_ok());
    }

    #[test]
    fn test_execute_dep_add_blocker_direction_parity_with_cli() {
        // Create a temporary workspace
        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir_all(&beads_dir).unwrap();

        // Initialize storage
        let db_path = beads_dir.join("beads.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create two beads
        storage.with_immediate_transaction(|tx| {
            // Create blocker bead
            tx.execute(
                "INSERT INTO issues (id, content_hash, title, status, priority, issue_type, created_at, created_by, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    "bf-blocker", "hash1", "Blocker", "open", 2, "task", Utc::now().to_rfc3339(), "test", Utc::now().to_rfc3339()
                ],
            )?;

            // Create blocked bead
            tx.execute(
                "INSERT INTO issues (id, content_hash, title, status, priority, issue_type, created_at, created_by, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    "bf-blocked", "hash2", "Blocked", "open", 2, "task", Utc::now().to_rfc3339(), "test", Utc::now().to_rfc3339()
                ],
            )?;

            Ok(())
        }).unwrap();

        // Add dependency via batch function (should match CLI direction)
        storage
            .with_immediate_transaction(|tx| {
                execute_dep_add_blocker(tx, "bf-blocked", "bf-blocker").unwrap();
                Ok(())
            })
            .unwrap();

        // Verify the dependency row matches what CLI would create
        let deps = storage
            .with_immediate_transaction(|tx| {
                let mut stmt = tx.prepare(
                "SELECT issue_id, depends_on_id, type FROM dependencies WHERE issue_id = ?1"
            ).unwrap();
                let deps: Vec<(String, String, String)> = stmt
                    .query_map(["bf-blocked"], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    })
                    .unwrap()
                    .filter_map(|r| r.ok())
                    .collect();
                Ok(deps)
            })
            .unwrap();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "bf-blocked"); // issue_id
        assert_eq!(deps[0].1, "bf-blocker"); // depends_on_id
        assert_eq!(deps[0].2, "blocks"); // type

        // This matches: storage.add_dependency("bf-blocked", "bf-blocker", ...)
        // which CLI uses via: bf dep add bf-blocker --blocks bf-blocked
    }

    #[test]
    fn test_execute_dep_add_blocker_detects_cycles() {
        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir_all(&beads_dir).unwrap();

        let db_path = beads_dir.join("beads.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create two beads and a dependency
        storage.with_immediate_transaction(|tx| {
            tx.execute(
                "INSERT INTO issues (id, content_hash, title, status, priority, issue_type, created_at, created_by, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    "bf-a", "hash1", "A", "open", 2, "task", Utc::now().to_rfc3339(), "test", Utc::now().to_rfc3339()
                ],
            )?;
            tx.execute(
                "INSERT INTO issues (id, content_hash, title, status, priority, issue_type, created_at, created_by, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    "bf-b", "hash2", "B", "open", 2, "task", Utc::now().to_rfc3339(), "test", Utc::now().to_rfc3339()
                ],
            )?;
            Ok(())
        }).unwrap();

        // Add dependency: B blocks A
        storage
            .with_immediate_transaction(|tx| {
                execute_dep_add_blocker(tx, "bf-a", "bf-b").unwrap();
                Ok(())
            })
            .unwrap();

        // Attempting to add reverse dependency should fail (cycle detection)
        let result =
            storage.with_immediate_transaction(|tx| execute_dep_add_blocker(tx, "bf-b", "bf-a"));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Circular dependency"));
    }

    #[test]
    fn test_execute_dep_add_blocker_detects_duplicates() {
        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir_all(&beads_dir).unwrap();

        let db_path = beads_dir.join("beads.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create two beads
        storage.with_immediate_transaction(|tx| {
            tx.execute(
                "INSERT INTO issues (id, content_hash, title, status, priority, issue_type, created_at, created_by, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    "bf-a", "hash1", "A", "open", 2, "task", Utc::now().to_rfc3339(), "test", Utc::now().to_rfc3339()
                ],
            )?;
            tx.execute(
                "INSERT INTO issues (id, content_hash, title, status, priority, issue_type, created_at, created_by, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    "bf-b", "hash2", "B", "open", 2, "task", Utc::now().to_rfc3339(), "test", Utc::now().to_rfc3339()
                ],
            )?;
            Ok(())
        }).unwrap();

        // Add dependency once
        storage
            .with_immediate_transaction(|tx| {
                execute_dep_add_blocker(tx, "bf-a", "bf-b").unwrap();
                Ok(())
            })
            .unwrap();

        // Attempting to add same dependency again should fail
        let result =
            storage.with_immediate_transaction(|tx| execute_dep_add_blocker(tx, "bf-a", "bf-b"));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Dependency already exists"));
    }

    #[test]
    fn test_mitosis_placeholder_references_end_to_end() {
        // Create a temporary workspace
        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir_all(&beads_dir).unwrap();

        // Create config and metadata
        let config_path = beads_dir.join("config.yaml");
        fs::write(&config_path, "issue_prefixes: [bf]\ndefault_priority: 2\ndefault_type: task\nclaim_ttl_minutes: 30\n").unwrap();

        let metadata_path = beads_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
        )
        .unwrap();

        // Initialize storage
        let db_path = beads_dir.join("beads.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create a parent bead
        let parent_id = generate_id("bf", 0);
        let mut parent = Issue::new(
            parent_id.clone(),
            "Parent task to split".to_string(),
            ".".to_string(),
        );
        parent.issue_type = IssueType::Task;
        parent.priority = Priority(1);

        storage
            .with_immediate_transaction(|tx| {
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
                        &parent.id,
                        &parent.content_hash,
                        &parent.title,
                        parent.description.as_deref().unwrap_or(""),
                        parent.design.as_deref().unwrap_or(""),
                        parent.acceptance_criteria.as_deref().unwrap_or(""),
                        parent.notes.as_deref().unwrap_or(""),
                        parent.status.to_string(),
                        &parent.priority,
                        parent.issue_type.to_string(),
                        &parent.assignee,
                        &parent.owner,
                        &parent.estimated_minutes,
                        parent.created_at.to_rfc3339(),
                        &parent.created_by,
                        parent.updated_at.to_rfc3339(),
                        parent.closed_at.map(|d| d.to_rfc3339()),
                        parent.close_reason.as_deref().unwrap_or(""),
                        parent.closed_by_session.as_deref().unwrap_or(""),
                        parent.due_at.map(|d| d.to_rfc3339()),
                        parent.defer_until.map(|d| d.to_rfc3339()),
                        parent.external_ref.as_deref(),
                        parent.source_system.as_deref().unwrap_or(""),
                        &parent.source_repo,
                        parent.deleted_at.map(|d| d.to_rfc3339()),
                        parent.deleted_by.as_deref().unwrap_or(""),
                        parent.delete_reason.as_deref().unwrap_or(""),
                        parent.original_type.as_deref().unwrap_or(""),
                        &parent.compaction_level,
                        parent.compacted_at.map(|d| d.to_rfc3339()),
                        parent.compacted_at_commit.as_deref().unwrap_or(""),
                        &parent.original_size,
                        parent.sender.as_deref().unwrap_or(""),
                        if parent.ephemeral { 1 } else { 0 },
                        if parent.pinned { 1 } else { 0 },
                        if parent.is_template { 1 } else { 0 },
                    ],
                )?;
                Ok(())
            })
            .unwrap();

        // Build mitosis batch operations manually to test placeholder resolution
        let mut ops = vec![
            // Create two children
            BatchOp::Create {
                title: "Child 1".to_string(),
                type_: "task".to_string(),
                priority: 2,
                description: None,
                assignee: None,
                labels: vec![],
            },
            BatchOp::Create {
                title: "Child 2".to_string(),
                type_: "bug".to_string(),
                priority: 0,
                description: None,
                assignee: None,
                labels: vec![],
            },
            // Add dependencies using placeholder references
            // Each child blocks the parent
            BatchOp::DepAddBlocker {
                id: parent_id.clone(),     // parent is blocked
                blocker: "@0".to_string(), // first child blocks parent
            },
            BatchOp::DepAddBlocker {
                id: parent_id.clone(),     // parent is blocked
                blocker: "@1".to_string(), // second child blocks parent
            },
            // Close the parent
            BatchOp::Close {
                id: parent_id.clone(),
                reason: "Split into children".to_string(),
            },
        ];

        // Execute the batch
        let results = execute_batch(&storage, ops, &temp_dir.path()).unwrap();

        // Verify all operations succeeded
        assert_eq!(results.len(), 5);
        for result in &results {
            assert_eq!(result.status, "ok");
        }

        // Extract child IDs from results
        let child_0_id = results[0].id.as_ref().unwrap();
        let child_1_id = results[1].id.as_ref().unwrap();

        // Verify children exist
        let child_0 = storage.get_issue(child_0_id).unwrap().unwrap();
        let child_1 = storage.get_issue(child_1_id).unwrap().unwrap();
        assert_eq!(child_0.title, "Child 1");
        assert_eq!(child_1.title, "Child 2");

        // Verify dependencies were created correctly
        // Parent should depend on both children (children block parent)
        let parent_deps = storage
            .with_immediate_transaction(|tx| {
                let mut stmt = tx.prepare(
                "SELECT depends_on_id FROM dependencies WHERE issue_id = ?1 AND type = 'blocks'"
            ).unwrap();
                let deps: Vec<String> = stmt
                    .query_map([&parent_id], |row| row.get(0))
                    .unwrap()
                    .filter_map(|r| r.ok())
                    .collect();
                Ok(deps)
            })
            .unwrap();

        assert_eq!(parent_deps.len(), 2);
        assert!(parent_deps.contains(child_0_id));
        assert!(parent_deps.contains(child_1_id));

        // Verify parent was closed
        let parent_after = storage.get_issue(&parent_id).unwrap().unwrap();
        assert_eq!(parent_after.status, Status::Closed);
        assert_eq!(
            parent_after.close_reason.as_deref().unwrap(),
            "Split into children"
        );
    }

    #[test]
    fn test_mitosis_function() {
        // Test the mitosis() helper function
        let parent_id = "bf-parent";

        let ops = mitosis(
            parent_id,
            vec![
                ("Child 1".to_string(), "task".to_string(), 2),
                ("Child 2".to_string(), "bug".to_string(), 0),
            ],
            Some("Test split".to_string()),
        )
        .unwrap();

        // Should have 5 operations: 2 creates, 2 deps, 1 close
        assert_eq!(ops.len(), 5);

        // Verify operation order
        assert!(matches!(ops[0], BatchOp::Create { .. }));
        assert!(matches!(ops[1], BatchOp::Create { .. }));
        assert!(matches!(ops[2], BatchOp::DepAddBlocker { .. }));
        assert!(matches!(ops[3], BatchOp::DepAddBlocker { .. }));
        assert!(matches!(ops[4], BatchOp::Close { .. }));

        // Verify placeholder references (new canonical schema)
        if let BatchOp::DepAddBlocker { id, blocker } = &ops[2] {
            assert_eq!(id, parent_id); // parent is blocked
            assert_eq!(blocker, "@0"); // first child blocks parent
        } else {
            panic!("Expected DepAddBlocker");
        }

        if let BatchOp::DepAddBlocker { id, blocker } = &ops[3] {
            assert_eq!(id, parent_id); // parent is blocked
            assert_eq!(blocker, "@1"); // second child blocks parent
        } else {
            panic!("Expected DepAddBlocker");
        }

        if let BatchOp::Close { id, reason } = &ops[4] {
            assert_eq!(id, parent_id);
            assert_eq!(reason, "Test split");
        } else {
            panic!("Expected Close");
        }
    }

    #[test]
    fn test_mitosis_ex_function() {
        // Test the mitosis_ex() helper function with extended options
        let parent_id = "bf-parent";

        let children = vec![
            MitosisChild {
                title: "Child 1".to_string(),
                type_: "task".to_string(),
                priority: 2,
                description: Some("First child".to_string()),
                assignee: Some("worker-1".to_string()),
                labels: vec!["urgent".to_string()],
            },
            MitosisChild {
                title: "Child 2".to_string(),
                type_: "bug".to_string(),
                priority: 0,
                description: None,
                assignee: None,
                labels: vec![],
            },
        ];

        let ops = mitosis_ex(parent_id, children, Some("Extended split".to_string())).unwrap();

        // Should have 5 operations
        assert_eq!(ops.len(), 5);

        // Verify extended attributes are in the create operations
        if let BatchOp::Create {
            title,
            description,
            assignee,
            labels,
            ..
        } = &ops[0]
        {
            assert_eq!(title, "Child 1");
            assert_eq!(description, &Some("First child".to_string()));
            assert_eq!(assignee, &Some("worker-1".to_string()));
            assert_eq!(labels, &vec!["urgent".to_string()]);
        } else {
            panic!("Expected Create with extended attributes");
        }
    }
}
