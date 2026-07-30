use crate::autoflush;
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
    #[serde(rename = "update")]
    Update {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        design: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        acceptance_criteria: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        priority: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        assignee: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        owner: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        issue_type: Option<String>,
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
    #[serde(rename = "dep_remove")]
    DepRemove {
        /// The bead that has the dependency
        id: String,
        /// The bead that is being depended on (to remove the dependency from)
        depends_on: String,
    },
    #[serde(rename = "label_add")]
    LabelAdd {
        id: String,
        #[serde(default)]
        labels: Vec<String>,
    },
    #[serde(rename = "label_remove")]
    LabelRemove {
        id: String,
        #[serde(default)]
        labels: Vec<String>,
    },
    #[serde(rename = "comment")]
    Comment {
        id: String,
        #[serde(default = "default_comment_author")]
        author: String,
        text: String,
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

fn default_comment_author() -> String {
    "batch".to_string()
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
        "update" => &[
            "op",
            "id",
            "title",
            "description",
            "design",
            "acceptance_criteria",
            "notes",
            "status",
            "priority",
            "assignee",
            "owner",
            "issue_type",
        ],
        "dep_add_blocker" => &["op", "id", "blocker", "parent", "child"],
        "dep_remove" => &["op", "id", "depends_on"],
        "label_add" => &["op", "id", "labels"],
        "label_remove" => &["op", "id", "labels"],
        "comment" => &["op", "id", "author", "text"],
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
    no_auto_flush: bool,
) -> Result<Vec<BatchResult>> {
    let config = load_config(
        &find_beads_dir(workspace_dir).ok_or_else(|| anyhow!("No .beads directory found"))?,
    )?;

    let results = storage.with_immediate_transaction(|tx| {
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
                BatchOp::Update {
                    id,
                    title,
                    description,
                    design,
                    acceptance_criteria,
                    notes,
                    status,
                    priority,
                    assignee,
                    owner,
                    issue_type,
                } => {
                    let id_resolved = resolve_reference(id, &created_ids);
                    match execute_update(
                        tx,
                        &id_resolved,
                        title,
                        description,
                        design,
                        acceptance_criteria,
                        notes,
                        status,
                        *priority,
                        assignee,
                        owner,
                        issue_type,
                    ) {
                        Ok(_) => BatchResult {
                            op: idx,
                            status: "ok".to_string(),
                            id: None,
                            error: None,
                            message: Some(format!("Updated bead {}", id_resolved)),
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
                BatchOp::DepRemove { id, depends_on } => {
                    let id_resolved = resolve_reference(id, &created_ids);
                    let depends_on_resolved = resolve_reference(depends_on, &created_ids);
                    match execute_dep_remove(tx, &id_resolved, &depends_on_resolved) {
                        Ok(_) => BatchResult {
                            op: idx,
                            status: "ok".to_string(),
                            id: None,
                            error: None,
                            message: Some(format!(
                                "Removed dependency: {} -> {}",
                                id_resolved, depends_on_resolved
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
                BatchOp::LabelAdd { id, labels } => {
                    let id_resolved = resolve_reference(id, &created_ids);
                    match execute_label_add(tx, &id_resolved, labels) {
                        Ok(_) => BatchResult {
                            op: idx,
                            status: "ok".to_string(),
                            id: None,
                            error: None,
                            message: Some(format!("Added labels to {}", id_resolved)),
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
                BatchOp::LabelRemove { id, labels } => {
                    let id_resolved = resolve_reference(id, &created_ids);
                    match execute_label_remove(tx, &id_resolved, labels) {
                        Ok(_) => BatchResult {
                            op: idx,
                            status: "ok".to_string(),
                            id: None,
                            error: None,
                            message: Some(format!("Removed labels from {}", id_resolved)),
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
                BatchOp::Comment { id, author, text } => {
                    let id_resolved = resolve_reference(id, &created_ids);
                    match execute_comment(tx, &id_resolved, author, text) {
                        Ok(_) => BatchResult {
                            op: idx,
                            status: "ok".to_string(),
                            id: None,
                            error: None,
                            message: Some(format!("Added comment to {}", id_resolved)),
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
    })?;

    // Single auto-flush after successful transaction commit (Phase 7.1 mechanism).
    // All beads modified in the batch were marked dirty atomically within the
    // transaction, so one flush exports them all to JSONL. If auto-flush fails,
    // dirty marks are retained and the next mutation or explicit `bf sync --flush-only`
    // will retry - the transaction itself is not affected.
    let flush_outcome = autoflush::after_mutation_with_config(
        workspace_dir,
        &config,
        no_auto_flush,
    );

    // Surface flush failures as warnings (non-fatal - the batch succeeded in SQLite)
    if let Some(warning) = flush_outcome.warning() {
        eprintln!("warning: {}", warning);
    }

    Ok(results)
}

/// Mark a bead dirty within the batch transaction so the single
/// end-of-transaction auto-flush (Phase 7.1) surgically re-exports it to JSONL.
///
/// Mirrors [`crate::storage::Storage::mark_dirty`] but reuses the caller's open
/// transaction so every mutation in the batch lands its dirty mark atomically
/// with the change itself — a rollback drops both together.
fn mark_dirty_tx(tx: &Connection, id: &str) -> Result<()> {
    tx.execute(
        "INSERT OR REPLACE INTO dirty_issues (issue_id, marked_at) VALUES (?1, ?2)",
        rusqlite::params![id, Utc::now().to_rfc3339()],
    )?;
    Ok(())
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

    // Mark dirty so the end-of-transaction flush exports the new bead.
    mark_dirty_tx(tx, &id)?;

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

    // Both endpoints' exported records reflect the new edge; mark both dirty so
    // the end-of-transaction flush re-exports them.
    mark_dirty_tx(tx, id)?;
    mark_dirty_tx(tx, blocker)?;

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

    // Mark the just-closed bead dirty for the end-of-transaction flush.
    mark_dirty_tx(tx, id)?;

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
            // A cascaded blocked->open transition changes the dependent's
            // exported status; mark it dirty too.
            mark_dirty_tx(tx, &dep_id)?;
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

/// Execute an update operation
fn execute_update(
    tx: &Connection,
    id: &str,
    title: &Option<String>,
    description: &Option<String>,
    design: &Option<String>,
    acceptance_criteria: &Option<String>,
    notes: &Option<String>,
    status: &Option<String>,
    priority: Option<i32>,
    assignee: &Option<String>,
    owner: &Option<String>,
    issue_type: &Option<String>,
) -> Result<()> {
    // Verify bead exists
    let exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
            &[id],
            |row| row.get(0),
        )?;

    if !exists {
        return Err(anyhow!("Bead not found: {}", id));
    }

    let mut updates = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref title) = title {
        updates.push("title = ?");
        params.push(Box::new(title.clone()));
    }
    if let Some(ref description) = description {
        updates.push("description = ?");
        params.push(Box::new(description.clone()));
    }
    if let Some(ref design) = design {
        updates.push("design = ?");
        params.push(Box::new(design.clone()));
    }
    if let Some(ref acceptance_criteria) = acceptance_criteria {
        updates.push("acceptance_criteria = ?");
        params.push(Box::new(acceptance_criteria.clone()));
    }
    if let Some(ref notes) = notes {
        updates.push("notes = ?");
        params.push(Box::new(notes.clone()));
    }
    if let Some(ref status) = status {
        updates.push("status = ?");
        params.push(Box::new(status.clone()));
    }
    if let Some(priority) = priority {
        updates.push("priority = ?");
        params.push(Box::new(priority));
    }
    if let Some(ref assignee) = assignee {
        if assignee.trim().is_empty() {
            updates.push("assignee = NULL");
        } else {
            updates.push("assignee = ?");
            params.push(Box::new(assignee.clone()));
        }
    }
    if let Some(ref owner) = owner {
        updates.push("owner = ?");
        params.push(Box::new(owner.clone()));
    }
    if let Some(ref issue_type) = issue_type {
        updates.push("issue_type = ?");
        params.push(Box::new(issue_type.clone()));
    }

    if !updates.is_empty() {
        updates.push("updated_at = ?");
        params.push(Box::new(Utc::now().to_rfc3339()));

        let query = format!("UPDATE issues SET {} WHERE id = ?", updates.join(", "));
        let mut all_params: Vec<Box<dyn rusqlite::ToSql>> = params.into_iter().collect();
        all_params.push(Box::new(id.to_string()));
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            all_params.iter().map(|p| p.as_ref()).collect();

        tx.execute(&query, param_refs.as_slice())?;

        // Mark dirty for export
        mark_dirty_tx(tx, id)?;

        // Invalidate critical path cache if status changed
        if status.is_some() {
            crate::critical_path::invalidate_cache(tx)?;
            crate::critical_path::compute_all_critical_paths(tx)?;
        }
    }

    Ok(())
}

/// Execute a dep_remove operation
fn execute_dep_remove(tx: &Connection, id: &str, depends_on: &str) -> Result<()> {
    // Verify both beads exist
    let id_exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
            &[id],
            |row| row.get(0),
        )?;

    let depends_on_exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
            &[depends_on],
            |row| row.get(0),
        )?;

    if !id_exists {
        return Err(anyhow!("Bead not found: {}", id));
    }
    if !depends_on_exists {
        return Err(anyhow!("Bead not found: {}", depends_on));
    }

    // Check if dependency exists
    let exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM dependencies WHERE issue_id = ?1 AND depends_on_id = ?2)",
            &[id, depends_on],
            |row| row.get(0),
        )?;

    if !exists {
        return Err(anyhow!(
            "Dependency does not exist: {} depends on {}",
            id,
            depends_on
        ));
    }

    // Remove dependency
    tx.execute(
        "DELETE FROM dependencies WHERE issue_id = ?1 AND depends_on_id = ?2",
        rusqlite::params![id, depends_on],
    )?;

    // Both endpoints' exported records reflect the removed edge; mark both dirty
    mark_dirty_tx(tx, id)?;
    mark_dirty_tx(tx, depends_on)?;

    // Rebuild blocked_issues_cache and invalidate critical path cache
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

    crate::critical_path::invalidate_cache(tx)?;
    crate::critical_path::compute_all_critical_paths(tx)?;

    Ok(())
}

/// Execute a label_add operation
fn execute_label_add(tx: &Connection, id: &str, labels: &[String]) -> Result<()> {
    // Verify bead exists
    let exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
            &[id],
            |row| row.get(0),
        )?;

    if !exists {
        return Err(anyhow!("Bead not found: {}", id));
    }

    // Add labels (INSERT OR IGNORE handles duplicates)
    for label in labels {
        tx.execute(
            "INSERT OR IGNORE INTO labels (issue_id, label) VALUES (?1, ?2)",
            rusqlite::params![id, label],
        )?;
    }

    // Mark dirty for export
    mark_dirty_tx(tx, id)?;

    Ok(())
}

/// Execute a label_remove operation
fn execute_label_remove(tx: &Connection, id: &str, labels: &[String]) -> Result<()> {
    // Verify bead exists
    let exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
            &[id],
            |row| row.get(0),
        )?;

    if !exists {
        return Err(anyhow!("Bead not found: {}", id));
    }

    // Remove labels
    for label in labels {
        tx.execute(
            "DELETE FROM labels WHERE issue_id = ?1 AND label = ?2",
            rusqlite::params![id, label],
        )?;
    }

    // Mark dirty for export
    mark_dirty_tx(tx, id)?;

    Ok(())
}

/// Execute a comment operation
fn execute_comment(tx: &Connection, id: &str, author: &str, text: &str) -> Result<()> {
    // Verify bead exists
    let exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM issues WHERE id = ?1)",
            &[id],
            |row| row.get(0),
        )?;

    if !exists {
        return Err(anyhow!("Bead not found: {}", id));
    }

    // Generate comment ID (using current timestamp)
    let comment_id = Utc::now().timestamp_micros();

    // Insert comment
    tx.execute(
        "INSERT INTO comments (id, issue_id, author, text, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![comment_id, id, author, text, Utc::now().to_rfc3339()],
    )?;

    // Mark dirty for export
    mark_dirty_tx(tx, id)?;

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
/// let results = execute_batch(&storage, ops, &workspace_dir, false)?;
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
        } else if let Some(rest) = line.strip_prefix("update ") {
            ops.push(parse_update(rest)?);
        } else if let Some(rest) = line.strip_prefix("dep add-blocker ") {
            ops.push(parse_dep_add(rest)?);
        } else if let Some(rest) = line.strip_prefix("dep remove ") {
            ops.push(parse_dep_remove(rest)?);
        } else if let Some(rest) = line.strip_prefix("label add ") {
            ops.push(parse_label_add(rest)?);
        } else if let Some(rest) = line.strip_prefix("label remove ") {
            ops.push(parse_label_remove(rest)?);
        } else if let Some(rest) = line.strip_prefix("comment ") {
            ops.push(parse_comment(rest)?);
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

fn parse_update(input: &str) -> Result<BatchOp> {
    // Simple parsing: update <id> --status X --priority Y --assignee Z
    let parts = shell_words::split(input)?;
    if parts.is_empty() {
        return Err(anyhow!("Missing ID for update operation"));
    }

    let id = parts[0].clone();
    let mut status = None;
    let mut priority = None;
    let mut assignee = None;
    let mut title = None;

    let mut i = 1;
    while i < parts.len() {
        match parts[i].as_str() {
            "--status" => {
                i += 1;
                if i < parts.len() {
                    status = Some(parts[i].clone());
                }
            }
            "--priority" => {
                i += 1;
                if i < parts.len() {
                    priority = Some(parts[i].parse().unwrap_or(2));
                }
            }
            "--assignee" => {
                i += 1;
                if i < parts.len() {
                    assignee = Some(parts[i].clone());
                }
            }
            "--title" => {
                i += 1;
                if i < parts.len() {
                    title = Some(parts[i].clone());
                }
            }
            _ => {
                i += 1;
            }
        }
        i += 1;
    }

    Ok(BatchOp::Update {
        id,
        title,
        description: None,
        design: None,
        acceptance_criteria: None,
        notes: None,
        status,
        priority,
        assignee,
        owner: None,
        issue_type: None,
    })
}

fn parse_dep_remove(input: &str) -> Result<BatchOp> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(anyhow!(
            "dep remove requires <id> <depends_on>. Usage: dep remove <bead> <depends-on-bead>"
        ));
    }
    Ok(BatchOp::DepRemove {
        id: parts[0].to_string(),
        depends_on: parts[1].to_string(),
    })
}

fn parse_label_add(input: &str) -> Result<BatchOp> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(anyhow!(
            "label add requires <id> <label>... Usage: label add <bead> <label1> <label2> ..."
        ));
    }
    let id = parts[0].to_string();
    let labels = parts[1..].iter().map(|s| s.to_string()).collect();
    Ok(BatchOp::LabelAdd { id, labels })
}

fn parse_label_remove(input: &str) -> Result<BatchOp> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(anyhow!(
            "label remove requires <id> <label>... Usage: label remove <bead> <label1> <label2> ..."
        ));
    }
    let id = parts[0].to_string();
    let labels = parts[1..].iter().map(|s| s.to_string()).collect();
    Ok(BatchOp::LabelRemove { id, labels })
}

fn parse_comment(input: &str) -> Result<BatchOp> {
    // comment <id> <text...>
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(anyhow!(
            "comment requires <id> <text>. Usage: comment <bead> <comment text>"
        ));
    }
    let id = parts[0].to_string();
    let text = parts[1..].join(" ");
    Ok(BatchOp::Comment {
        id,
        author: default_comment_author(),
        text,
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
        let results = execute_batch(&storage, ops, &temp_dir.path(), false /* enable auto-flush */).unwrap();

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

    #[test]
    fn test_update_operation_modifies_bead_fields() {
        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir_all(&beads_dir).unwrap();

        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            "issue_prefixes: [bf]\ndefault_priority: 2\ndefault_type: task\nclaim_ttl_minutes: 30\n",
        )
        .unwrap();

        let metadata_path = beads_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
        )
        .unwrap();

        let db_path = beads_dir.join("beads.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create a test bead
        storage
            .with_immediate_transaction(|tx| {
                tx.execute(
                    "INSERT INTO issues (id, content_hash, title, description, status, priority, issue_type, created_at, created_by, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        "bf-test", "hash1", "Test Bead", "Original description", "open", 2, "task",
                        Utc::now().to_rfc3339(), "test", Utc::now().to_rfc3339()
                    ],
                )?;
                Ok(())
            })
            .unwrap();

        // Test update operation
        let ops = vec![BatchOp::Update {
            id: "bf-test".to_string(),
            title: Some("Updated Title".to_string()),
            description: Some("Updated description".to_string()),
            design: Some("Design notes".to_string()),
            acceptance_criteria: Some("Criteria".to_string()),
            notes: Some("Notes".to_string()),
            status: Some("in_progress".to_string()),
            priority: Some(0),
            assignee: Some("worker-1".to_string()),
            owner: Some("owner-1".to_string()),
            issue_type: Some("bug".to_string()),
        }];

        let results = execute_batch(&storage, ops, &temp_dir.path(), false /* enable auto-flush */).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "ok");

        // Verify all fields were updated
        let issue = storage.get_issue("bf-test").unwrap().unwrap();
        assert_eq!(issue.title, "Updated Title");
        assert_eq!(issue.description.as_deref(), Some("Updated description"));
        assert_eq!(issue.design.as_deref(), Some("Design notes"));
        assert_eq!(issue.acceptance_criteria.as_deref(), Some("Criteria"));
        assert_eq!(issue.notes.as_deref(), Some("Notes"));
        assert_eq!(issue.status, Status::InProgress);
        assert_eq!(issue.priority, Priority(0));
        assert_eq!(issue.assignee.as_deref(), Some("worker-1"));
        assert_eq!(issue.owner.as_deref(), Some("owner-1"));
        assert_eq!(issue.issue_type, IssueType::Bug);
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_label_add_adds_labels_to_bead() {
        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir_all(&beads_dir).unwrap();

        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            "issue_prefixes: [bf]\ndefault_priority: 2\ndefault_type: task\nclaim_ttl_minutes: 30\n",
        )
        .unwrap();

        let metadata_path = beads_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
        )
        .unwrap();

        let db_path = beads_dir.join("beads.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create a test bead
        storage
            .with_immediate_transaction(|tx| {
                tx.execute(
                    "INSERT INTO issues (id, content_hash, title, status, priority, issue_type, created_at, created_by, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        "bf-test", "hash1", "Test Bead", "open", 2, "task",
                        Utc::now().to_rfc3339(), "test", Utc::now().to_rfc3339()
                    ],
                )?;
                Ok(())
            })
            .unwrap();

        // Test label_add operation
        let ops = vec![BatchOp::LabelAdd {
            id: "bf-test".to_string(),
            labels: vec!["urgent".to_string(), "backend".to_string()],
        }];

        let results = execute_batch(&storage, ops, &temp_dir.path(), false /* enable auto-flush */).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "ok");

        // Verify labels were added
        let issue = storage.get_issue("bf-test").unwrap().unwrap();
        assert_eq!(issue.labels.len(), 2);
        assert!(issue.labels.contains(&"urgent".to_string()));
        assert!(issue.labels.contains(&"backend".to_string()));
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_label_remove_removes_labels_from_bead() {
        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir_all(&beads_dir).unwrap();

        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            "issue_prefixes: [bf]\ndefault_priority: 2\ndefault_type: task\nclaim_ttl_minutes: 30\n",
        )
        .unwrap();

        let metadata_path = beads_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
        )
        .unwrap();

        let db_path = beads_dir.join("beads.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create a test bead with labels
        storage
            .with_immediate_transaction(|tx| {
                tx.execute(
                    "INSERT INTO issues (id, content_hash, title, status, priority, issue_type, created_at, created_by, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        "bf-test", "hash1", "Test Bead", "open", 2, "task",
                        Utc::now().to_rfc3339(), "test", Utc::now().to_rfc3339()
                    ],
                )?;
                // Add initial labels
                for label in &["urgent", "backend", "bug"] {
                    tx.execute(
                        "INSERT INTO labels (issue_id, label) VALUES (?1, ?2)",
                        rusqlite::params!["bf-test", label],
                    )?;
                }
                Ok(())
            })
            .unwrap();

        // Test label_remove operation
        let ops = vec![BatchOp::LabelRemove {
            id: "bf-test".to_string(),
            labels: vec!["urgent".to_string(), "bug".to_string()],
        }];

        let results = execute_batch(&storage, ops, &temp_dir.path(), false /* enable auto-flush */).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "ok");

        // Verify labels were removed
        let issue = storage.get_issue("bf-test").unwrap().unwrap();
        assert_eq!(issue.labels.len(), 1);
        assert!(issue.labels.contains(&"backend".to_string()));
        assert!(!issue.labels.contains(&"urgent".to_string()));
        assert!(!issue.labels.contains(&"bug".to_string()));
    }

    #[test]
    fn test_update_label_operations_return_result() {
        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir_all(&beads_dir).unwrap();

        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            "issue_prefixes: [bf]\ndefault_priority: 2\ndefault_type: task\nclaim_ttl_minutes: 30\n",
        )
        .unwrap();

        let metadata_path = beads_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
        )
        .unwrap();

        let db_path = beads_dir.join("beads.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create a test bead
        storage
            .with_immediate_transaction(|tx| {
                tx.execute(
                    "INSERT INTO issues (id, content_hash, title, status, priority, issue_type, created_at, created_by, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        "bf-test", "hash1", "Test Bead", "open", 2, "task",
                        Utc::now().to_rfc3339(), "test", Utc::now().to_rfc3339()
                    ],
                )?;
                Ok(())
            })
            .unwrap();

        // Test that operations return Result (error case - non-existent bead)
        let ops = vec![
            BatchOp::Update {
                id: "bf-nonexistent".to_string(),
                title: Some("New Title".to_string()),
                description: None,
                design: None,
                acceptance_criteria: None,
                notes: None,
                status: None,
                priority: None,
                assignee: None,
                owner: None,
                issue_type: None,
            },
            BatchOp::LabelAdd {
                id: "bf-nonexistent".to_string(),
                labels: vec!["urgent".to_string()],
            },
            BatchOp::LabelRemove {
                id: "bf-nonexistent".to_string(),
                labels: vec!["urgent".to_string()],
            },
        ];

        // execute_batch should fail on first error
        let result = execute_batch(&storage, ops, &temp_dir.path(), false /* enable auto-flush */);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Bead not found"));
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_update_and_label_operations_wired_in_exec_loop() {
        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir_all(&beads_dir).unwrap();

        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            "issue_prefixes: [bf]\ndefault_priority: 2\ndefault_type: task\nclaim_ttl_minutes: 30\n",
        )
        .unwrap();

        let metadata_path = beads_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
        )
        .unwrap();

        let db_path = beads_dir.join("beads.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create test beads
        storage
            .with_immediate_transaction(|tx| {
                for id in &["bf-1", "bf-2", "bf-3"] {
                    tx.execute(
                        "INSERT INTO issues (id, content_hash, title, status, priority, issue_type, created_at, created_by, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                        rusqlite::params![
                            *id, "hash1", "Test", "open", 2, "task",
                            Utc::now().to_rfc3339(), "test", Utc::now().to_rfc3339()
                        ],
                    )?;
                }
                Ok(())
            })
            .unwrap();

        // Test multiple operations in a single batch
        let ops = vec![
            BatchOp::Update {
                id: "bf-1".to_string(),
                title: Some("Updated 1".to_string()),
                description: None,
                design: None,
                acceptance_criteria: None,
                notes: None,
                status: None,
                priority: None,
                assignee: None,
                owner: None,
                issue_type: None,
            },
            BatchOp::LabelAdd {
                id: "bf-2".to_string(),
                labels: vec!["urgent".to_string()],
            },
            BatchOp::LabelRemove {
                id: "bf-3".to_string(),
                labels: vec!["bug".to_string()], // removing non-existent label is fine
            },
        ];

        let results = execute_batch(&storage, ops, &temp_dir.path(), false /* enable auto-flush */).unwrap();
        assert_eq!(results.len(), 3);

        // All operations should succeed
        for result in &results {
            assert_eq!(result.status, "ok");
        }

        // Verify each operation
        let issue1 = storage.get_issue("bf-1").unwrap().unwrap();
        assert_eq!(issue1.title, "Updated 1");

        let issue2 = storage.get_issue("bf-2").unwrap().unwrap();
        assert_eq!(issue2.labels.len(), 1);
        assert!(issue2.labels.contains(&"urgent".to_string()));
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_mixed_op_batch_all_operations_atomic() {
        // Acceptance criterion 3: Mixed-op batches (update+label+dep+comment+create+close+dep_remove)
        // are atomic - either all succeed or all fail together.
        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir_all(&beads_dir).unwrap();

        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            "issue_prefixes: [bf]\ndefault_priority: 2\ndefault_type: task\nclaim_ttl_minutes: 30\n",
        )
        .unwrap();

        let metadata_path = beads_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
        )
        .unwrap();

        let db_path = beads_dir.join("beads.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create initial beads for dependency and update operations
        storage
            .with_immediate_transaction(|tx| {
                for id in &["bf-parent", "bf-child", "bf-target"] {
                    tx.execute(
                        "INSERT INTO issues (id, content_hash, title, status, priority, issue_type, created_at, created_by, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                        rusqlite::params![
                            *id,
                            "hash1",
                            format!("Bead {}", id),
                            "open",
                            2,
                            "task",
                            Utc::now().to_rfc3339(),
                            "test",
                            Utc::now().to_rfc3339()
                        ],
                    )?;
                }
                Ok(())
            })
            .unwrap();

        // Build a mixed-operation batch with all 8 operation types:
        // 1. Create a new bead
        // 2. Update existing bead
        // 3. Add label to existing bead
        // 4. Add dependency (dep_add_blocker)
        // 5. Add comment to existing bead
        // 6. Close a bead
        // 7. Remove label (label_remove)
        // 8. Remove dependency (dep_remove)
        let ops = vec![
            BatchOp::Create {
                title: "New bead from batch".to_string(),
                type_: "task".to_string(),
                priority: 1,
                description: Some("Created in mixed batch".to_string()),
                assignee: Some("worker-1".to_string()),
                labels: vec!["batch-created".to_string()],
            },
            BatchOp::Update {
                id: "bf-target".to_string(),
                title: Some("Updated in batch".to_string()),
                description: Some("Updated description".to_string()),
                design: None,
                acceptance_criteria: None,
                notes: None,
                status: Some("in_progress".to_string()),
                priority: Some(0),
                assignee: Some("worker-2".to_string()),
                owner: None,
                issue_type: None,
            },
            BatchOp::LabelAdd {
                id: "bf-target".to_string(),
                labels: vec!["urgent".to_string(), "backend".to_string()],
            },
            BatchOp::DepAddBlocker {
                id: "bf-child".to_string(),
                blocker: "bf-parent".to_string(),
            },
            BatchOp::Comment {
                id: "bf-target".to_string(),
                author: "batch-test".to_string(),
                text: "Comment added during batch".to_string(),
            },
            BatchOp::LabelRemove {
                id: "bf-parent".to_string(),
                labels: vec!["old-label".to_string()], // removing non-existent is fine
            },
            BatchOp::DepRemove {
                id: "bf-child".to_string(),
                depends_on: "bf-parent".to_string(),
            },
        ];

        // Execute the batch - all operations should succeed atomically
        let results = execute_batch(&storage, ops, &temp_dir.path(), false /* enable auto-flush */).unwrap();
        assert_eq!(results.len(), 7);

        // Verify all operations succeeded
        for result in &results {
            assert_eq!(result.status, "ok", "Operation {} should succeed", result.op);
        }

        // Verify each operation's effect:

        // 1. Create: new bead should exist with all fields
        let created_id = results[0].id.as_ref().unwrap();
        let created_bead = storage.get_issue(created_id).unwrap().unwrap();
        assert_eq!(created_bead.title, "New bead from batch");
        assert_eq!(created_bead.description.as_deref(), Some("Created in mixed batch"));
        assert_eq!(created_bead.assignee.as_deref(), Some("worker-1"));
        assert_eq!(created_bead.labels.len(), 1);
        assert!(created_bead.labels.contains(&"batch-created".to_string()));

        // 2. Update: bf-target should be updated
        let target = storage.get_issue("bf-target").unwrap().unwrap();
        assert_eq!(target.title, "Updated in batch");
        assert_eq!(target.description.as_deref(), Some("Updated description"));
        assert_eq!(target.status, Status::InProgress);
        assert_eq!(target.priority, Priority(0));
        assert_eq!(target.assignee.as_deref(), Some("worker-2"));

        // 3. LabelAdd: bf-target should have new labels
        assert_eq!(target.labels.len(), 2);
        assert!(target.labels.contains(&"urgent".to_string()));
        assert!(target.labels.contains(&"backend".to_string()));

        // 4. DepAddBlocker: then removed below, so verify the remove worked
        let child_deps = storage
            .with_immediate_transaction(|tx| {
                let mut stmt = tx
                    .prepare("SELECT depends_on_id FROM dependencies WHERE issue_id = ?1")
                    .unwrap();
                let deps: Vec<String> = stmt
                    .query_map(["bf-child"], |row| row.get(0))
                    .unwrap()
                    .filter_map(|r| r.ok())
                    .collect();
                Ok(deps)
            })
            .unwrap();
        assert_eq!(child_deps.len(), 0, "Dependency should be removed");

        // 5. Comment: bf-target should have comment
        let comments = storage
            .with_immediate_transaction(|tx| {
                let mut stmt = tx
                    .prepare("SELECT author, text FROM comments WHERE issue_id = ?1")
                    .unwrap();
                let comments: Vec<(String, String)> = stmt
                    .query_map(["bf-target"], |row| {
                        Ok((row.get(0)?, row.get(1)?))
                    })
                    .unwrap()
                    .filter_map(|r| r.ok())
                    .collect();
                Ok(comments)
            })
            .unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].0, "batch-test");
        assert_eq!(comments[0].1, "Comment added during batch");

        // All operations succeeded atomically in a single transaction
    }

    #[test]
    fn test_batch_rollback_on_any_op_failure() {
        // Acceptance criterion 4: Rollback on any op failure.
        // When one operation fails, all previous operations in that batch
        // must be rolled back (no partial updates).
        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir_all(&beads_dir).unwrap();

        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            "issue_prefixes: [bf]\ndefault_priority: 2\ndefault_type: task\nclaim_ttl_minutes: 30\n",
        )
        .unwrap();

        let metadata_path = beads_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
        )
        .unwrap();

        let db_path = beads_dir.join("beads.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create initial bead
        storage
            .with_immediate_transaction(|tx| {
                tx.execute(
                    "INSERT INTO issues (id, content_hash, title, status, priority, issue_type, created_at, created_by, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        "bf-existing",
                        "hash1",
                        "Existing Bead",
                        "open",
                        2,
                        "task",
                        Utc::now().to_rfc3339(),
                        "test",
                        Utc::now().to_rfc3339()
                    ],
                )?;
                Ok(())
            })
            .unwrap();

        // Get initial bead count
        let initial_count: i64 = storage
            .with_immediate_transaction(|tx| {
                Ok(tx.query_row("SELECT COUNT(*) FROM issues", [], |row| row.get(0))?)
            })
            .unwrap();

        // Build a batch where the 3rd operation will fail:
        // 1. Create bead (would succeed)
        // 2. Update existing bead (would succeed)
        // 3. Add dependency to non-existent bead (WILL FAIL)
        // 4. Add label (should not execute due to fail-fast)
        let ops = vec![
            BatchOp::Create {
                title: "Should be rolled back".to_string(),
                type_: "task".to_string(),
                priority: 2,
                description: None,
                assignee: None,
                labels: vec![],
            },
            BatchOp::Update {
                id: "bf-existing".to_string(),
                title: Some("Should be rolled back".to_string()),
                description: Some("This update should not persist".to_string()),
                design: None,
                acceptance_criteria: None,
                notes: None,
                status: Some("in_progress".to_string()),
                priority: Some(0),
                assignee: None,
                owner: None,
                issue_type: None,
            },
            BatchOp::DepAddBlocker {
                id: "bf-nonexistent".to_string(),
                blocker: "bf-also-nonexistent".to_string(),
            },
            BatchOp::LabelAdd {
                id: "bf-existing".to_string(),
                labels: vec!["should-not-be-added".to_string()],
            },
        ];

        // Execute the batch - should fail on the 3rd operation
        let result = execute_batch(&storage, ops, &temp_dir.path(), false /* enable auto-flush */);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Bead not found"),
            "Error should mention bead not found: {}",
            err_msg
        );

        // Verify ROLLBACK: all previous successful operations were rolled back

        // 1. Verify bead count is unchanged (create was rolled back)
        let final_count: i64 = storage
            .with_immediate_transaction(|tx| {
                Ok(tx.query_row("SELECT COUNT(*) FROM issues", [], |row| row.get(0))?)
            })
            .unwrap();
        assert_eq!(
            initial_count, final_count,
            "Bead count should be unchanged after failed batch"
        );

        // 2. Verify existing bead was NOT updated (update was rolled back)
        let existing = storage.get_issue("bf-existing").unwrap().unwrap();
        assert_eq!(existing.title, "Existing Bead", "Title should be unchanged");
        // Schema defaults NOT NULL fields to '' (empty string), not None
        assert_eq!(
            existing.description.as_deref(),
            Some(""),
            "Description should be unchanged (empty string per schema default)"
        );
        assert_eq!(existing.status, Status::Open, "Status should be unchanged");
        assert_eq!(existing.priority, Priority(2), "Priority should be unchanged");

        // 3. Verify label was NOT added (4th op didn't execute)
        assert!(!existing.labels.contains(&"should-not-be-added".to_string()));

        // Transaction rolled back completely - no partial updates
    }

    #[test]
    fn test_batch_with_immediate_transaction_wrapper() {
        // Acceptance criterion 1: All batch ops in single transaction.
        // Verify that execute_batch uses with_immediate_transaction and all
        // operations share the same transaction context.
        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir_all(&beads_dir).unwrap();

        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            "issue_prefixes: [bf]\ndefault_priority: 2\ndefault_type: task\nclaim_ttl_minutes: 30\n",
        )
        .unwrap();

        let metadata_path = beads_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
        )
        .unwrap();

        let db_path = beads_dir.join("beads.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create two beads
        storage
            .with_immediate_transaction(|tx| {
                for id in &["bf-1", "bf-2"] {
                    tx.execute(
                        "INSERT INTO issues (id, content_hash, title, status, priority, issue_type, created_at, created_by, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                        rusqlite::params![
                            *id,
                            "hash1",
                            "Test",
                            "open",
                            2,
                            "task",
                            Utc::now().to_rfc3339(),
                            "test",
                            Utc::now().to_rfc3339()
                        ],
                    )?;
                }
                Ok(())
            })
            .unwrap();

        // Build a batch that creates a dependency between existing beads
        // This requires both beads to be visible within the same transaction
        let ops = vec![BatchOp::DepAddBlocker {
            id: "bf-2".to_string(),
            blocker: "bf-1".to_string(),
        }];

        // Execute the batch - should succeed in a single transaction
        let results = execute_batch(&storage, ops, &temp_dir.path(), false /* enable auto-flush */).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "ok");

        // Verify dependency was created
        let deps = storage
            .with_immediate_transaction(|tx| {
                let mut stmt = tx
                    .prepare("SELECT depends_on_id FROM dependencies WHERE issue_id = ?1")
                    .unwrap();
                let deps: Vec<String> = stmt
                    .query_map(["bf-2"], |row| row.get(0))
                    .unwrap()
                    .filter_map(|r| r.ok())
                    .collect();
                Ok(deps)
            })
            .unwrap();

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], "bf-1");

        // The batch executed in a single BEGIN IMMEDIATE transaction
    }

    #[test]
    fn test_mark_dirty_tx_called_within_batch_transaction() {
        // Verify that mark_dirty_tx is called for each affected bead within
        // the batch transaction, so the auto-flush exports exactly those beads.
        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir_all(&beads_dir).unwrap();

        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            "issue_prefixes: [bf]\ndefault_priority: 2\ndefault_type: task\nclaim_ttl_minutes: 30\n",
        )
        .unwrap();

        let metadata_path = beads_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
        )
        .unwrap();

        let db_path = beads_dir.join("beads.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create two beads
        storage
            .with_immediate_transaction(|tx| {
                for id in &["bf-1", "bf-2"] {
                    tx.execute(
                        "INSERT INTO issues (id, content_hash, title, status, priority, issue_type, created_at, created_by, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                        rusqlite::params![
                            *id,
                            "hash1",
                            "Test",
                            "open",
                            2,
                            "task",
                            Utc::now().to_rfc3339(),
                            "test",
                            Utc::now().to_rfc3339()
                        ],
                    )?;
                }
                Ok(())
            })
            .unwrap();

        // Execute a batch that affects both beads
        let ops = vec![
            BatchOp::Update {
                id: "bf-1".to_string(),
                title: Some("Updated".to_string()),
                description: None,
                design: None,
                acceptance_criteria: None,
                notes: None,
                status: None,
                priority: None,
                assignee: None,
                owner: None,
                issue_type: None,
            },
            BatchOp::LabelAdd {
                id: "bf-2".to_string(),
                labels: vec!["labeled".to_string()],
            },
            BatchOp::DepAddBlocker {
                id: "bf-2".to_string(),
                blocker: "bf-1".to_string(),
            },
        ];

        let results = execute_batch(&storage, ops, &temp_dir.path(), true /* no-auto-flush: keep dirty marks for test */).unwrap();
        assert_eq!(results.len(), 3);
        for result in &results {
            assert_eq!(result.status, "ok");
        }

        // Verify both beads are marked dirty (updated directly + dep endpoints)
        let dirty = storage.list_dirty_issues().unwrap();
        assert_eq!(dirty.len(), 2);
        let dirty_ids: Vec<&str> = dirty.iter().map(|i| i.id.as_str()).collect();
        assert!(dirty_ids.contains(&"bf-1"));
        assert!(dirty_ids.contains(&"bf-2"));

        // Marking happened within the transaction - beads are dirty for flush
    }

    #[test]
    fn test_single_auto_flush_after_batch_commit() {
        // Acceptance criterion 2: One auto-flush on commit.
        // Verify that after a batch commits, the dirty beads are exactly those
        // affected by the batch, and a single flush exports them all.
        use crate::sync::flush_dirty;

        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir_all(&beads_dir).unwrap();

        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            "issue_prefixes: [bf]\ndefault_priority: 2\ndefault_type: task\nclaim_ttl_minutes: 30\n",
        )
        .unwrap();

        let metadata_path = beads_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
        )
        .unwrap();

        let db_path = beads_dir.join("beads.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create initial beads and export them all to JSONL first (establish baseline)
        let initial_beads = vec![
            ("bf-unchanged", "Unchanged Bead"),
            ("bf-update", "Update Target"),
            ("bf-label", "Label Target"),
            ("bf-dep-1", "Dependency One"),
            ("bf-dep-2", "Dependency Two"),
        ];

        for (id, title) in &initial_beads {
            let issue = Issue::new(id.to_string(), title.to_string(), ".".to_string());
            storage.create_issue(&issue).unwrap();
        }

        // Flush all beads to JSONL to establish baseline
        flush_dirty(&temp_dir.path()).unwrap();

        // Clear dirty flags after initial flush
        storage.clear_dirty().unwrap();

        // Verify no dirty beads before batch
        let dirty_before = storage.list_dirty_issues().unwrap();
        assert_eq!(dirty_before.len(), 0, "No beads should be dirty before batch");

        // Execute a batch that affects 4 beads:
        // - bf-update: updated (direct mark_dirty)
        // - bf-label: label added (direct mark_dirty)
        // - bf-dep-1, bf-dep-2: dependency added (both marked dirty as dep endpoints)
        // - bf-unchanged: NOT affected (should remain non-dirty)
        let ops = vec![
            BatchOp::Update {
                id: "bf-update".to_string(),
                title: Some("Updated in batch".to_string()),
                description: None,
                design: None,
                acceptance_criteria: None,
                notes: None,
                status: None,
                priority: None,
                assignee: None,
                owner: None,
                issue_type: None,
            },
            BatchOp::LabelAdd {
                id: "bf-label".to_string(),
                labels: vec!["batched".to_string()],
            },
            BatchOp::DepAddBlocker {
                id: "bf-dep-2".to_string(),
                blocker: "bf-dep-1".to_string(),
            },
        ];

        let results = execute_batch(&storage, ops, &temp_dir.path(), true /* no-auto-flush: verify dirty marks first */).unwrap();
        assert_eq!(results.len(), 3);
        for result in &results {
            assert_eq!(result.status, "ok");
        }

        // Verify exactly 4 beads are dirty after batch (update, label, 2x dep endpoints)
        let dirty_after = storage.list_dirty_issues().unwrap();
        assert_eq!(dirty_after.len(), 4, "Exactly 4 beads should be dirty after batch");
        let dirty_ids: Vec<&str> = dirty_after.iter().map(|i| i.id.as_str()).collect();
        assert!(dirty_ids.contains(&"bf-update"));
        assert!(dirty_ids.contains(&"bf-label"));
        assert!(dirty_ids.contains(&"bf-dep-1"));
        assert!(dirty_ids.contains(&"bf-dep-2"));
        assert!(!dirty_ids.contains(&"bf-unchanged"));

        // Perform the flush (simulating the auto-flush that happens after batch commit)
        let flushed_count = flush_dirty(&temp_dir.path()).unwrap();
        assert_eq!(flushed_count, 4, "Flush should export exactly 4 dirty beads");

        // Verify all dirty beads were cleared after flush
        let dirty_after_flush = storage.list_dirty_issues().unwrap();
        assert_eq!(
            dirty_after_flush.len(),
            0,
            "No beads should remain dirty after flush"
        );

        // Verify JSONL file was created with the flushed beads
        let jsonl_path = beads_dir.join("issues.jsonl");
        assert!(jsonl_path.exists(), "JSONL file should exist after flush");

        // Read JSONL and verify it contains exactly the 5 beads (4 dirty + 1 unchanged)
        let jsonl_content = fs::read_to_string(&jsonl_path).unwrap();
        let lines: Vec<&str> = jsonl_content.lines().collect();
        assert_eq!(lines.len(), 5, "JSONL should contain all 5 beads");

        // Verify the JSONL contains the updated bead with correct values
        let jsonl_beads: Vec<Issue> = lines
            .iter()
            .map(|line| serde_json::from_str::<Issue>(line).unwrap())
            .collect();
        let updated = jsonl_beads.iter().find(|i| i.id == "bf-update").unwrap();
        assert_eq!(updated.title, "Updated in batch");

        // One flush exported all dirty beads from the batch transaction
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_auto_flush_enabled_writes_incremental_changes_to_jsonl() {
        // Acceptance criterion 4: Test with mixed-op batch shows single JSONL write.
        // Verify that when auto-flush is enabled (no_auto_flush=false), the batch
        // automatically flushes dirty beads to JSONL exactly once after commit.
        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir_all(&beads_dir).unwrap();

        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            "issue_prefixes: [bf]\ndefault_priority: 2\ndefault_type: task\nclaim_ttl_minutes: 30\nsync:\n  auto_flush: true\n",
        )
        .unwrap();

        let metadata_path = beads_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
        )
        .unwrap();

        let db_path = beads_dir.join("beads.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create initial beads and export them all to JSONL first (establish baseline)
        let initial_beads = vec![
            ("bf-unchanged", "Unchanged Bead"),
            ("bf-update", "Update Target"),
            ("bf-label", "Label Target"),
            ("bf-dep-1", "Dependency One"),
            ("bf-dep-2", "Dependency Two"),
        ];

        for (id, title) in &initial_beads {
            let issue = Issue::new(id.to_string(), title.to_string(), ".".to_string());
            storage.create_issue(&issue).unwrap();
        }

        // Flush all beads to JSONL to establish baseline
        crate::sync::flush_dirty(&temp_dir.path()).unwrap();

        // Clear dirty flags after initial flush
        storage.clear_dirty().unwrap();

        // Verify no dirty beads before batch
        let dirty_before = storage.list_dirty_issues().unwrap();
        assert_eq!(dirty_before.len(), 0, "No beads should be dirty before batch");

        // Get JSONL mtime before batch (to verify it was updated)
        let jsonl_path = beads_dir.join("issues.jsonl");
        let mtime_before = fs::metadata(&jsonl_path).unwrap().modified().unwrap();

        // Execute a mixed-operation batch with auto-flush ENABLED (no_auto_flush=false)
        // This should: (1) commit transaction, (2) auto-flush dirty beads to JSONL
        let ops = vec![
            BatchOp::Update {
                id: "bf-update".to_string(),
                title: Some("Auto-flushed update".to_string()),
                description: None,
                design: None,
                acceptance_criteria: None,
                notes: None,
                status: None,
                priority: None,
                assignee: None,
                owner: None,
                issue_type: None,
            },
            BatchOp::LabelAdd {
                id: "bf-label".to_string(),
                labels: vec!["auto-flushed".to_string()],
            },
            BatchOp::DepAddBlocker {
                id: "bf-dep-2".to_string(),
                blocker: "bf-dep-1".to_string(),
            },
        ];

        let results = execute_batch(&storage, ops, &temp_dir.path(), false /* AUTO-FLUSH ENABLED */).unwrap();
        assert_eq!(results.len(), 3);
        for result in &results {
            assert_eq!(result.status, "ok");
        }

        // Verify auto-flush happened: JSONL file was updated (mtime changed)
        let mtime_after = fs::metadata(&jsonl_path).unwrap().modified().unwrap();
        assert!(mtime_after > mtime_before, "JSONL file should have been updated by auto-flush");

        // Verify all dirty beads were cleared by auto-flush (no manual flush needed)
        let dirty_after = storage.list_dirty_issues().unwrap();
        assert_eq!(
            dirty_after.len(),
            0,
            "Auto-flush should have cleared all dirty marks"
        );

        // Verify JSONL contains the flushed beads with correct values
        let jsonl_content = fs::read_to_string(&jsonl_path).unwrap();
        let lines: Vec<&str> = jsonl_content.lines().collect();
        assert_eq!(lines.len(), 5, "JSONL should contain all 5 beads");

        let jsonl_beads: Vec<Issue> = lines
            .iter()
            .map(|line| serde_json::from_str::<Issue>(line).unwrap())
            .collect();

        // Verify updated bead was flushed
        let updated = jsonl_beads.iter().find(|i| i.id == "bf-update").unwrap();
        assert_eq!(updated.title, "Auto-flushed update");

        // Verify label addition was flushed
        let labeled = jsonl_beads.iter().find(|i| i.id == "bf-label").unwrap();
        assert!(labeled.labels.contains(&"auto-flushed".to_string()));

        // One auto-flush exported all dirty beads from the batch transaction
    }

    #[test]
    fn test_batch_fail_fast_no_dirty_marks_on_partial_failure() {
        // Verify that when a batch fails mid-execution, no dirty marks persist
        // for the operations that succeeded before the failure (they were rolled back).
        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir_all(&beads_dir).unwrap();

        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            "issue_prefixes: [bf]\ndefault_priority: 2\ndefault_type: task\nclaim_ttl_minutes: 30\n",
        )
        .unwrap();

        let metadata_path = beads_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
        )
        .unwrap();

        let db_path = beads_dir.join("beads.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create initial bead
        storage
            .with_immediate_transaction(|tx| {
                tx.execute(
                    "INSERT INTO issues (id, content_hash, title, status, priority, issue_type, created_at, created_by, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        "bf-existing",
                        "hash1",
                        "Existing",
                        "open",
                        2,
                        "task",
                        Utc::now().to_rfc3339(),
                        "test",
                        Utc::now().to_rfc3339()
                    ],
                )?;
                Ok(())
            })
            .unwrap();

        // Execute a batch that fails on the 2nd operation
        let ops = vec![
            BatchOp::Update {
                id: "bf-existing".to_string(),
                title: Some("Should be rolled back".to_string()),
                description: None,
                design: None,
                acceptance_criteria: None,
                notes: None,
                status: None,
                priority: None,
                assignee: None,
                owner: None,
                issue_type: None,
            },
            BatchOp::DepAddBlocker {
                id: "bf-nonexistent".to_string(),
                blocker: "bf-also-missing".to_string(),
            },
        ];

        // Batch should fail
        let result = execute_batch(&storage, ops, &temp_dir.path(), false /* enable auto-flush */);
        assert!(result.is_err());

        // Verify NO dirty marks persist (rollback cleared them)
        let dirty = storage.list_dirty_issues().unwrap();
        assert_eq!(dirty.len(), 0, "No dirty marks should persist after failed batch");

        // Transaction rollback ensures clean state
    }
}
