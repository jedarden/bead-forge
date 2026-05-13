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
    DepAddBlocker { parent: String, child: String },
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

    let mut issue = Issue::new(id.clone(), title.to_string(), ".".to_string());
    issue.issue_type = IssueType::from_str(type_).map_err(|e| anyhow!("Invalid type: {}", e))?;
    issue.priority = Priority(priority);
    issue.description = description.clone().or_else(|| Some(String::new()));
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
            issue.description.as_deref().unwrap_or(""),
            issue.design.as_deref().unwrap_or(""),
            issue.acceptance_criteria.as_deref().unwrap_or(""),
            issue.notes.as_deref().unwrap_or(""),
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
         VALUES (?1, 'closed', '', '', ?2, ?3)",
        rusqlite::params![id, reason, now.to_rfc3339()],
    )?;

    Ok(())
}

/// Mitosis: split a parent bead into multiple child beads atomically.
///
/// This function constructs a batch of operations that:
/// 1. Creates N child beads
/// 2. Adds dependencies (child blocks parent) for each child
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
            parent: format!("@{}", idx),
            child: parent_id.to_string(),
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
            parent: format!("@{}", idx),
            child: parent_id.to_string(),
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
                parent: "@0".to_string(), // References first created child
                child: parent_id.clone(),
            },
            BatchOp::DepAddBlocker {
                parent: "@1".to_string(), // References second created child
                child: parent_id.clone(),
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
        // Parent should depend on both children (child blocks parent)
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

        // Verify placeholder references
        if let BatchOp::DepAddBlocker { parent, child } = &ops[2] {
            assert_eq!(parent, "@0");
            assert_eq!(child, parent_id);
        } else {
            panic!("Expected DepAddBlocker");
        }

        if let BatchOp::DepAddBlocker { parent, child } = &ops[3] {
            assert_eq!(parent, "@1");
            assert_eq!(child, parent_id);
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
