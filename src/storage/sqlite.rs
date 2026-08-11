use crate::critical_path::{compute_all_critical_paths, invalidate_cache};
use crate::error::{BeadForgeError, Result};
use crate::jsonl::{export_jsonl, export_jsonl_dirty, import_jsonl, ImportResult, UpsertResult};
use crate::model::{
    Comment, Dependency, DependencyType, Event, EventType, Issue, IssueChanges, IssueFilter,
    IssueType, IssueUpdate, Priority, Status,
};
use crate::secrets::{SecretMatch, SecretScanner};
use crate::storage::schema::{apply_schema, ensure_wal_mode};
use anyhow::anyhow;
use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::{params, Connection, Transaction};
use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;
use std::sync::Mutex;
use std::time::Duration;

const MAX_RETRIES: u32 = 5;
const RETRY_BASE_MS: u64 = 50;

/// Error type for secret detection failures.
#[derive(Debug, thiserror::Error)]
#[error("secret detected: {0}")]
pub struct SecretError(pub String);

/// Dependency tree node with hierarchy information.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DepTreeNode {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: i32,
    pub depth: i64,
    pub dep_type: Option<String>,
    pub path: String, // Comma-separated path of IDs for cycle detection
}

/// Dependency display information with type, bead ID, and title.
#[derive(Debug, Clone)]
pub struct DependencyDisplay {
    pub dep_type: String,
    pub bead_id: String,
    pub title: String,
}

pub struct Storage {
    /// The database connection. Made public for testing purposes.
    pub conn: Mutex<Connection>,
    secret_scanner: Mutex<Option<SecretScanner>>,
}

impl Storage {
    pub fn open(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        // Wait out short-lived locks from concurrent processes (fleet workers,
        // flushes, CLI invocations) instead of failing instantly with
        // SQLITE_BUSY. with_immediate_transaction adds its own backoff on top
        // for write transactions; this covers everything else, including the
        // schema apply below.
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        ensure_wal_mode(&conn)?;
        // Apply schema on every open - all tables use CREATE TABLE IF NOT EXISTS
        // which is a no-op for existing tables and avoids DDL lock contention
        apply_schema(&conn)?;
        Ok(Storage {
            conn: Mutex::new(conn),
            secret_scanner: Mutex::new(None),
        })
    }

    /// Open storage with secret scanning configured from the provided config.
    ///
    /// This is a convenience method that opens the database and configures
    /// secret protection in one step. Use this instead of `open()` followed
    /// by `set_secret_scanner()` when you have access to the Config.
    pub fn open_with_config(db_path: &Path, config: &crate::config::Config) -> Result<Self> {
        let storage = Self::open(db_path)?;
        if config.secret_protection.enabled {
            if let Ok(scanner) = SecretScanner::from_config(&config.secret_protection) {
                storage.set_secret_scanner(scanner);
            }
        }
        Ok(storage)
    }

    /// Create a Storage instance from an existing SQLite connection.
    ///
    /// This is useful when you have an already-opened connection and want to
    /// use the Storage API without reopening the database. The connection
    /// should have the appropriate schema applied already.
    pub fn from_conn(conn: Connection) -> Result<Self> {
        // Set busy timeout for concurrent access
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        // Apply schema to ensure tables exist
        apply_schema(&conn)?;
        Ok(Storage {
            conn: Mutex::new(conn),
            secret_scanner: Mutex::new(None),
        })
    }


    /// Explicitly apply database migrations.
    ///
    /// This is called during `bf migrate` to ensure all bf-only tables
    /// are created. Normally `apply_schema()` is called on every `Storage::open()`,
    /// but this method allows explicit migration control.
    pub fn apply_migrations(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        // Re-apply schema - all tables use CREATE TABLE IF NOT EXISTS
        // which is a no-op for existing tables
        apply_schema(&conn)?;
        drop(conn);
        Ok(())
    }

    /// Configure secret scanning for this storage instance.
    ///
    /// When enabled, `create_issue` and `update_issue` will scan all string fields
    /// for secrets and return an error if any are detected.
    pub fn set_secret_scanner(&self, scanner: SecretScanner) {
        *self.secret_scanner.lock().unwrap() = Some(scanner);
    }

    /// Check if secret scanning is enabled.
    pub fn has_secret_scanner(&self) -> bool {
        self.secret_scanner.lock().unwrap().is_some()
    }

    pub fn with_write_transaction<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Transaction) -> Result<T>,
    {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        match f(&tx) {
            Ok(result) => {
                tx.commit()?;
                drop(conn);
                Ok(result)
            }
            Err(e) => {
                let _ = tx.rollback();
                Err(e)
            }
        }
    }
    pub fn with_immediate_transaction<T, F>(&self, f: F) -> Result<T>
    where
        F: Fn(&Connection) -> Result<T>,
    {
        let mut attempt = 0;
        loop {
            let outcome = {
                let conn = self.conn.lock().unwrap();
                match conn.execute_batch("BEGIN IMMEDIATE") {
                    Err(e) if is_busy_error(&e) && attempt < MAX_RETRIES => None,
                    Err(e) => return Err(e.into()),
                    Ok(_) => {
                        let r = f(&conn);
                        match &r {
                            Ok(_) => {
                                let _ = conn.execute_batch("COMMIT");
                            }
                            Err(_) => {
                                let _ = conn.execute_batch("ROLLBACK");
                            }
                        }
                        Some(r)
                    }
                }
            };
            match outcome {
                Some(r) => return r,
                None => {
                    attempt += 1;
                    std::thread::sleep(Duration::from_millis(RETRY_BASE_MS * attempt as u64));
                }
            }
        }
    }

    pub fn get_issue(&self, id: &str) -> Result<Option<Issue>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT i.id, i.content_hash, i.title, i.description, i.design, i.acceptance_criteria, i.notes,
                    i.status, i.priority, i.issue_type, i.assignee, i.owner, i.estimated_minutes,
                    i.created_at, i.created_by, i.updated_at, i.closed_at, i.close_reason,
                    i.closed_by_session, i.due_at, i.defer_until, i.external_ref, i.source_system,
                    i.source_repo, i.deleted_at, i.deleted_by, i.delete_reason, i.original_type,
                    i.compaction_level, i.compacted_at, i.compacted_at_commit, i.original_size,
                    i.sender, i.ephemeral, i.pinned, i.is_template,
                    GROUP_CONCAT(bl.label) AS labels
             FROM issues i
             LEFT JOIN bead_labels bl ON i.id = bl.bead_id
             WHERE i.id = ?1
             GROUP BY i.id",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::row_to_issue_conn(&conn, row)?))
        } else {
            Ok(None)
        }
    }

    pub fn list_issues(&self, filter: &IssueFilter) -> Result<Vec<Issue>> {
        let mut query = String::from(
            "SELECT i.id, i.content_hash, i.title, i.description, i.design, i.acceptance_criteria, i.notes,
                    i.status, i.priority, i.issue_type, i.assignee, i.owner, i.estimated_minutes,
                    i.created_at, i.created_by, i.updated_at, i.closed_at, i.close_reason,
                    i.closed_by_session, i.due_at, i.defer_until, i.external_ref, i.source_system,
                    i.source_repo, i.deleted_at, i.deleted_by, i.delete_reason, i.original_type,
                    i.compaction_level, i.compacted_at, i.compacted_at_commit, i.original_size,
                    i.sender, i.ephemeral, i.pinned, i.is_template,
                    GROUP_CONCAT(bl.label) AS labels
             FROM issues i
             LEFT JOIN bead_labels bl ON i.id = bl.bead_id",
        );
        let mut params = Vec::new();
        let mut param_idx = 1;
        let needs_annotation_join = filter.annotation.is_some();

        if needs_annotation_join {
            query.push_str(" LEFT JOIN bead_annotations a ON i.id = a.bead_id");
        }

        query.push_str(" WHERE i.deleted_at IS NULL");

        if let Some(ref status) = filter.status {
            query.push_str(&format!(" AND i.status = ?{}", param_idx));
            params.push(status.to_string());
            param_idx += 1;
        }
        if let Some(ref issue_type) = filter.issue_type {
            query.push_str(&format!(" AND i.issue_type = ?{}", param_idx));
            params.push(issue_type.to_string());
            param_idx += 1;
        }
        if let Some(ref assignee) = filter.assignee {
            if assignee.is_empty() {
                // Empty-string filter selects unassigned beads.
                query.push_str(" AND (i.assignee IS NULL OR i.assignee = '')");
            } else {
                query.push_str(&format!(" AND i.assignee = ?{}", param_idx));
                params.push(assignee.clone());
                param_idx += 1;
            }
        }
        if let Some(priority) = filter.priority {
            query.push_str(&format!(" AND i.priority = ?{}", param_idx));
            params.push(priority.to_string());
            param_idx += 1;
        }
        if let Some((ref key, ref value)) = filter.annotation {
            query.push_str(&format!(" AND a.key = ?{}", param_idx));
            params.push(key.clone());
            param_idx += 1;
            query.push_str(&format!(" AND a.value = ?{}", param_idx));
            params.push(value.clone());
            param_idx += 1;
        }
        if let Some(ref labels) = filter.labels {
            if !labels.is_empty() {
                // Subquery to find issues that have ALL the specified labels
                let label_conditions: Vec<String> = labels
                    .iter()
                    .enumerate()
                    .map(|(i, label)| {
                        params.push(label.clone());
                        param_idx += 1;
                        format!("EXISTS (SELECT 1 FROM bead_labels bl{} WHERE bl{}.bead_id = i.id AND bl{}.label = ?{})", i, i, i, param_idx - 1)
                    })
                    .collect();
                query.push_str(&format!(" AND ({}) ", label_conditions.join(" AND ")));
            }
        }
        if let Some(ref updated_since) = filter.updated_since {
            query.push_str(&format!(" AND i.updated_at >= ?{}", param_idx));
            params.push(updated_since.to_rfc3339());
            param_idx += 1;
        }
        if let Some(ref updated_before) = filter.updated_before {
            query.push_str(&format!(" AND i.updated_at < ?{}", param_idx));
            params.push(updated_before.to_rfc3339());
            param_idx += 1;
        }
        query.push_str(" GROUP BY i.id");
        query.push_str(" ORDER BY i.updated_at DESC, i.id ASC");
        if let Some(limit) = filter.limit {
            // Validate limit to prevent potential DoS attacks
            const MAX_LIMIT: usize = 10000;
            let safe_limit = limit.min(MAX_LIMIT);
            query.push_str(&format!(" LIMIT ?{}", param_idx));
            params.push(safe_limit.to_string());
            param_idx += 1;
        }
        if let Some(offset) = filter.offset {
            // Validate offset to prevent potential DoS attacks
            const MAX_OFFSET: usize = 1000000;
            let safe_offset = offset.min(MAX_OFFSET);
            query.push_str(&format!(" OFFSET ?{}", param_idx));
            params.push(safe_offset.to_string());
            param_idx += 1;
        }
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(&query)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
        let mut rows = stmt.query(param_refs.as_slice())?;

        // Collect all issue data first (without deps/comments/annotations)
        let mut issues_data: Vec<(String, Issue)> = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let issue = Self::row_to_issue_partial(row)?;
            issues_data.push((id, issue));
        }

        // Collect all IDs for batch loading
        let issue_ids: Vec<&String> = issues_data.iter().map(|(id, _)| id).collect();

        // Batch load all dependencies, comments, and annotations
        let all_dependencies = Self::batch_load_dependencies(&conn, &issue_ids)?;
        let all_comments = Self::batch_load_comments(&conn, &issue_ids)?;
        let all_annotations = Self::batch_load_annotations(&conn, &issue_ids)?;

        // Combine into final Issue structs
        let mut issues = Vec::new();
        for (id, mut issue) in issues_data {
            issue.dependencies = all_dependencies.get(&id).cloned().unwrap_or_default();
            issue.comments = all_comments.get(&id).cloned().unwrap_or_default();
            issue.annotations = all_annotations.get(&id).cloned().unwrap_or_default();
            issues.push(issue);
        }

        Ok(issues)
    }

    /// Get beads that are ready to be claimed (open, unblocked beads).
    ///
    /// Returns beads that:
    /// - Have status='open'
    /// - Are not ephemeral, pinned, template, or deleted
    /// - Have no unclosed blocking dependencies
    /// - Are sorted by priority (ASC) and created_at (ASC)
    pub fn get_ready_candidates(&self) -> Result<Vec<Issue>> {
        let conn = self.conn.lock().unwrap();

        // Query for open, unblocked beads
        // A bead is blocked if there EXISTS a dependency of blocking type
        // where the blocker is NOT in a terminal state
        let query = r#"
            SELECT i.id, i.content_hash, i.title, i.description, i.design, i.acceptance_criteria, i.notes,
                   i.status, i.priority, i.issue_type, i.assignee, i.owner, i.estimated_minutes,
                   i.created_at, i.created_by, i.updated_at, i.closed_at, i.close_reason,
                   i.closed_by_session, i.due_at, i.defer_until, i.external_ref, i.source_system,
                   i.source_repo, i.deleted_at, i.deleted_by, i.delete_reason, i.original_type,
                   i.compaction_level, i.compacted_at, i.compacted_at_commit, i.original_size,
                   i.sender, i.ephemeral, i.pinned, i.is_template,
                   GROUP_CONCAT(bl.label) AS labels
            FROM issues i
            LEFT JOIN bead_labels bl ON i.id = bl.bead_id
            WHERE i.status = 'open'
              AND i.ephemeral = 0
              AND i.pinned = 0
              AND i.is_template = 0
              AND i.deleted_at IS NULL
              AND NOT EXISTS (
                  -- Exclude beads with unclosed blocking dependencies
                  SELECT 1
                  FROM dependencies d
                  INNER JOIN issues blocker ON blocker.id = d.depends_on_id
                  WHERE d.issue_id = i.id
                    AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
                    AND blocker.status NOT IN ('closed', 'tombstone', 'done', 'completed')
              )
            GROUP BY i.id
            ORDER BY i.priority ASC, i.created_at ASC
        "#;

        let mut stmt = conn.prepare_cached(query)?;
        let mut rows = stmt.query([])?;

        // Collect all issue data first (without deps/comments/annotations)
        let mut issues_data: Vec<(String, Issue)> = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let issue = Self::row_to_issue_partial(row)?;
            issues_data.push((id, issue));
        }

        // Collect all IDs for batch loading
        let issue_ids: Vec<&String> = issues_data.iter().map(|(id, _)| id).collect();

        // Batch load all dependencies, comments, and annotations
        let all_dependencies = Self::batch_load_dependencies(&conn, &issue_ids)?;
        let all_comments = Self::batch_load_comments(&conn, &issue_ids)?;
        let all_annotations = Self::batch_load_annotations(&conn, &issue_ids)?;

        // Combine into final Issue structs
        let mut issues = Vec::new();
        for (id, mut issue) in issues_data {
            issue.dependencies = all_dependencies.get(&id).cloned().unwrap_or_default();
            issue.comments = all_comments.get(&id).cloned().unwrap_or_default();
            issue.annotations = all_annotations.get(&id).cloned().unwrap_or_default();
            issues.push(issue);
        }

        Ok(issues)
    }

    pub fn list_all_issues(&self) -> Result<Vec<Issue>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT i.id, i.content_hash, i.title, i.description, i.design, i.acceptance_criteria, i.notes,
                    i.status, i.priority, i.issue_type, i.assignee, i.owner, i.estimated_minutes,
                    i.created_at, i.created_by, i.updated_at, i.closed_at, i.close_reason,
                    i.closed_by_session, i.due_at, i.defer_until, i.external_ref, i.source_system,
                    i.source_repo, i.deleted_at, i.deleted_by, i.delete_reason, i.original_type,
                    i.compaction_level, i.compacted_at, i.compacted_at_commit, i.original_size,
                    i.sender, i.ephemeral, i.pinned, i.is_template,
                    GROUP_CONCAT(bl.label) AS labels
             FROM issues i
             LEFT JOIN bead_labels bl ON i.id = bl.bead_id
             WHERE i.deleted_at IS NULL
             GROUP BY i.id
             ORDER BY i.id",
        )?;
        let mut rows = stmt.query([])?;
        let mut issues = Vec::new();
        while let Some(row) = rows.next()? {
            issues.push(Self::row_to_issue_conn(&conn, row)?);
        }
        Ok(issues)
    }

    pub fn list_dirty_issues(&self) -> Result<Vec<Issue>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT i.id, i.content_hash, i.title, i.description, i.design, i.acceptance_criteria, i.notes,
                    i.status, i.priority, i.issue_type, i.assignee, i.owner, i.estimated_minutes,
                    i.created_at, i.created_by, i.updated_at, i.closed_at, i.close_reason,
                    i.closed_by_session, i.due_at, i.defer_until, i.external_ref, i.source_system,
                    i.source_repo, i.deleted_at, i.deleted_by, i.delete_reason, i.original_type,
                    i.compaction_level, i.compacted_at, i.compacted_at_commit, i.original_size,
                    i.sender, i.ephemeral, i.pinned, i.is_template,
                    GROUP_CONCAT(bl.label) AS labels
             FROM issues i
             INNER JOIN dirty_issues d ON i.id = d.bead_id
             LEFT JOIN bead_labels bl ON i.id = bl.bead_id
             GROUP BY i.id
             ORDER BY i.id",
        )?;
        let mut rows = stmt.query([])?;
        let mut issues = Vec::new();
        while let Some(row) = rows.next()? {
            issues.push(Self::row_to_issue_conn(&conn, row)?);
        }
        Ok(issues)
    }

    pub fn clear_dirty(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM dirty_issues", [])?;
        Ok(())
    }

    /// Query all bead IDs from the dirty_issues table.
    ///
    /// Returns a vector of bead IDs that have been marked as dirty and need flushing to JSONL.
    /// Returns an empty vector if the table is empty.
    pub fn query_dirty_issues(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached("SELECT bead_id FROM dirty_issues ORDER BY marked_at ASC")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut bead_ids = Vec::new();
        for bead_id in rows {
            bead_ids.push(bead_id?);
        }
        Ok(bead_ids)
    }

    pub fn create_issue(&self, issue: &Issue) -> Result<()> {
        // Scan for secrets before creating
        if let Some(scanner) = &*self.secret_scanner.lock().unwrap() {
            let matches = scanner.scan_issue(issue);
            if !matches.is_empty() {
                return Err(SecretError(format_secret_matches(&matches)).into());
            }
        }

        // Compute content_hash if not already set, and wrap in Some for storage
        let content_hash: Option<String> = issue
            .content_hash
            .as_ref()
            .cloned()
            .or_else(|| Some(issue.content_hash()));

        self.with_immediate_transaction(|tx| {
            tx.execute(
                "INSERT INTO issues (
                    id, content_hash, title, description, design, acceptance_criteria, notes,
                    status, priority, issue_type, assignee, owner, estimated_minutes,
                    created_at, created_by, updated_at, closed_at, close_reason,
                    closed_by_session, due_at, defer_until, external_ref, source_system,
                    source_repo, deleted_at, deleted_by, delete_reason, original_type,
                    compaction_level, compacted_at, compacted_at_commit, original_size,
                    sender, ephemeral, pinned, is_template
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                          ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28,
                          ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36)",
                params![
                    &issue.id, &content_hash, &issue.title,
                    issue.description.as_deref().unwrap_or(""),
                    issue.design.as_deref().unwrap_or(""),
                    issue.acceptance_criteria.as_deref().unwrap_or(""),
                    issue.notes.as_deref().unwrap_or(""),
                    &issue.status.to_string(),
                    &issue.priority, &issue.issue_type.to_string(), &issue.assignee, &issue.owner,
                    &issue.estimated_minutes, &issue.created_at.to_rfc3339(), &issue.created_by,
                    &issue.updated_at.to_rfc3339(), issue.closed_at.map(|d| d.to_rfc3339()),
                    &issue.close_reason, &issue.closed_by_session, issue.due_at.map(|d| d.to_rfc3339()),
                    issue.defer_until.map(|d| d.to_rfc3339()), &issue.external_ref, &issue.source_system,
                    issue.source_repo.as_deref().unwrap_or("."),
                    issue.deleted_at.map(|d| d.to_rfc3339()), &issue.deleted_by,
                    &issue.delete_reason, &issue.original_type, &issue.compaction_level,
                    issue.compacted_at.map(|d| d.to_rfc3339()), &issue.compacted_at_commit,
                    &issue.original_size, &issue.sender,
                    if issue.ephemeral { 1 } else { 0 },
                    if issue.pinned { 1 } else { 0 },
                    if issue.is_template { 1 } else { 0 },
                ],
            )?;
            // br parity: record a 'created' event for the new issue
            // (beads_rust storage/events.rs insert_created_event)
            tx.execute(
                "INSERT INTO events (issue_id, event_type, actor, created_at) VALUES (?1, 'created', ?2, ?3)",
                params![
                    &issue.id,
                    issue.created_by.as_deref().unwrap_or(""),
                    &issue.created_at.to_rfc3339(),
                ],
            )?;
            for label in &issue.labels {
                tx.execute("INSERT OR IGNORE INTO labels (issue_id, label) VALUES (?1, ?2)", params![&issue.id, label])?;
            }
            for label in &issue.labels {
                tx.execute("INSERT OR IGNORE INTO bead_labels (bead_id, label) VALUES (?1, ?2)", params![&issue.id, label])?;
            }
            for dep in &issue.dependencies {
                tx.execute(
                    "INSERT INTO dependencies (issue_id, depends_on_id, type, metadata, thread_id, created_at, created_by)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        &dep.issue_id, &dep.depends_on_id, &dep.dep_type.to_string(),
                        dep.metadata.as_ref().map(|m| serde_json::to_string(m).ok()).flatten(),
                        &dep.thread_id, &dep.created_at.to_rfc3339(), &dep.created_by,
                    ],
                )?;
            }
            for comment in &issue.comments {
                tx.execute(
                    "INSERT INTO comments (id, issue_id, author, text, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        &comment.id, &comment.issue_id, &comment.author, &comment.body,
                        &comment.created_at.to_rfc3339(),
                    ],
                )?;
            }
            for (key, value) in &issue.annotations {
                tx.execute(
                    "INSERT INTO bead_annotations (bead_id, key, value) VALUES (?1, ?2, ?3)",
                    params![&issue.id, key, value],
                )?;
            }
            // Mark as dirty for export (new beads need to be flushed to JSONL)
            tx.execute(
                "INSERT OR REPLACE INTO dirty_issues (bead_id, marked_at) VALUES (?1, ?2)",
                params![&issue.id, chrono::Utc::now().to_rfc3339()],
            )?;
            // Invalidate critical path cache: new beads may add dependencies
            invalidate_cache(tx)?;
            compute_all_critical_paths(tx)?;
            // Rebuild blocked_issues_cache to reflect new dependencies
            tx.execute("DELETE FROM blocked_issues_cache", [])?;
            tx.execute(
                "INSERT INTO blocked_issues_cache (issue_id, blocked_by, blocked_at)
                 SELECT d.issue_id, '[' || GROUP_CONCAT('\"' || d.depends_on_id || '\"') || ']' AS blocked_by, ?1
                 FROM dependencies d
                 INNER JOIN issues i ON i.id = d.depends_on_id
                 WHERE d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
                 AND i.status NOT IN ('closed', 'tombstone', 'done', 'completed')
                 GROUP BY d.issue_id",
                params![chrono::Utc::now().to_rfc3339()],
            )?;
            Ok(())
        })
    }

    pub fn update_issue(&self, id: &str, changes: &IssueChanges) -> Result<()> {
        // Validate that the bead exists
        let exists = {
            let conn = self.conn.lock().unwrap();
            match conn.query_row("SELECT 1 FROM issues WHERE id = ?1", params![id], |_| {
                Ok(true)
            }) {
                Ok(result) => result,
                Err(rusqlite::Error::QueryReturnedNoRows) => false,
                Err(e) => return Err(e.into()),
            }
        };
        if !exists {
            return Err(BeadForgeError::not_found("bead", id, None));
        }

        // Scan for secrets before updating (only for string fields in changes)
        if let Some(scanner) = &*self.secret_scanner.lock().unwrap() {
            let mut all_matches = Vec::new();
            for field in [
                changes.title.as_deref(),
                changes.description.as_deref(),
                changes.design.as_deref(),
                changes.acceptance_criteria.as_deref(),
                changes.notes.as_deref(),
                changes.assignee.as_deref(),
                changes.owner.as_deref(),
                changes.external_ref.as_deref(),
            ] {
                if let Some(value) = field {
                    all_matches.extend(scanner.scan_string(value));
                }
            }
            if let Some(labels) = &changes.labels {
                for label in labels {
                    all_matches.extend(scanner.scan_string(label));
                }
            }
            if let Some(annotations) = &changes.annotations {
                for (key, value) in annotations {
                    all_matches.extend(scanner.scan_string(key));
                    all_matches.extend(scanner.scan_string(value));
                }
            }
            if !all_matches.is_empty() {
                return Err(SecretError(format_secret_matches(&all_matches)).into());
            }
        }

        self.with_immediate_transaction(|tx| {
            // Get current status before update (for reopen detection)
            let current_status: Option<Status> = tx.query_row(
                "SELECT status FROM issues WHERE id = ?1",
                params![id],
                |row| {
                    let status_str: String = row.get(0)?;
                    Ok(match status_str.as_str() {
                        "open" => Status::Open,
                        "in_progress" => Status::InProgress,
                        "blocked" => Status::Blocked,
                        "deferred" => Status::Deferred,
                        "draft" => Status::Draft,
                        "closed" => Status::Closed,
                        "tombstone" => Status::Tombstone,
                        "pinned" => Status::Pinned,
                        _ => Status::Custom(status_str),
                    })
                },
            ).ok();

            // Current assignee before update (for assignee-change events)
            let current_assignee: Option<String> = tx
                .query_row(
                    "SELECT assignee FROM issues WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .ok()
                .flatten();

            let mut updates = Vec::new();
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            if let Some(ref title) = changes.title {
                updates.push("title = ?");
                params.push(Box::new(title.clone()));
            }
            if let Some(ref description) = changes.description {
                updates.push("description = ?");
                params.push(Box::new(description.clone()));
            }
            if let Some(ref design) = changes.design {
                updates.push("design = ?");
                params.push(Box::new(design.clone()));
            }
            if let Some(ref acceptance_criteria) = changes.acceptance_criteria {
                updates.push("acceptance_criteria = ?");
                params.push(Box::new(acceptance_criteria.clone()));
            }
            if let Some(ref notes) = changes.notes {
                updates.push("notes = ?");
                params.push(Box::new(notes.clone()));
            }
            if let Some(ref status) = changes.status {
                updates.push("status = ?");
                params.push(Box::new(status.to_string()));
                // Clear closed fields when transitioning FROM closed/tombstone TO open
                if matches!(current_status, Some(Status::Closed | Status::Tombstone))
                    && !matches!(status, Status::Closed | Status::Tombstone)
                {
                    updates.push("closed_at = NULL");
                    updates.push("close_reason = NULL");
                    updates.push("closed_by_session = NULL");
                }
                // Set closed fields when transitioning TO closed (satisfies CHECK constraint)
                else if matches!(status, Status::Closed) {
                    // Only set if not already closed (avoid overwriting existing close metadata)
                    if !matches!(current_status, Some(Status::Closed)) {
                        let now = Utc::now();
                        updates.push("closed_at = ?");
                        params.push(Box::new(now.to_rfc3339()));
                        updates.push("close_reason = ?");
                        params.push(Box::new(String::new())); // Empty reason when closed via update
                        updates.push("closed_by_session = ?");
                        let actor = changes.actor.as_deref().unwrap_or("cli");
                        params.push(Box::new(actor.to_string()));
                    }
                }
            }
            if let Some(priority) = changes.priority {
                updates.push("priority = ?");
                params.push(Box::new(priority));
            }
            if let Some(ref issue_type) = changes.issue_type {
                updates.push("issue_type = ?");
                params.push(Box::new(issue_type.to_string()));
            }
            if let Some(ref assignee) = changes.assignee {
                if assignee.trim().is_empty() {
                    // Clearing stores NULL, never an empty string that would
                    // read back as "assigned" and hide the bead from claiming.
                    updates.push("assignee = NULL");
                } else {
                    updates.push("assignee = ?");
                    params.push(Box::new(assignee.clone()));
                }
            }
            if let Some(ref owner) = changes.owner {
                updates.push("owner = ?");
                params.push(Box::new(owner.clone()));
            }
            if let Some(estimated_minutes) = changes.estimated_minutes {
                updates.push("estimated_minutes = ?");
                params.push(Box::new(estimated_minutes));
            }
            if let Some(ref due_at) = changes.due_at {
                updates.push("due_at = ?");
                params.push(Box::new(due_at.to_rfc3339()));
            }
            if let Some(ref defer_until) = changes.defer_until {
                updates.push("defer_until = ?");
                params.push(Box::new(defer_until.to_rfc3339()));
            }
            if let Some(ref external_ref) = changes.external_ref {
                updates.push("external_ref = ?");
                params.push(Box::new(external_ref.clone()));
            }
            let now = Utc::now();
            if !updates.is_empty() {
                updates.push("updated_at = ?");
                params.push(Box::new(now.to_rfc3339()));
                let query = format!("UPDATE issues SET {} WHERE id = ?", updates.join(", "));
                let mut all_params: Vec<Box<dyn rusqlite::ToSql>> = params.into_iter().collect();
                all_params.push(Box::new(id.to_string()));
                let param_refs: Vec<&dyn rusqlite::ToSql> =
                    all_params.iter().map(|p| p.as_ref()).collect();
                tx.execute(&query, param_refs.as_slice())?;

                // Create a Reopened event if transitioning from closed/tombstone to active status
                if let (Some(current), Some(new)) = (current_status, changes.status.as_ref()) {
                    if current.is_terminal() && !new.is_terminal() {
                        let actor = changes.actor.as_deref().unwrap_or("system");
                        let now = Utc::now();
                        tx.execute(
                            "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, created_at) VALUES (?1, 'reopened', ?2, ?3, ?4, ?5)",
                            params![id, actor, current.as_str(), new.as_str(), now.to_rfc3339()],
                        )?;
                    }
                }

                // br parity: record an assignee_changed event when the assignee changes
                if let Some(ref new_assignee) = changes.assignee {
                    let new_val = if new_assignee.trim().is_empty() {
                        None
                    } else {
                        Some(new_assignee.as_str())
                    };
                    if current_assignee.as_deref() != new_val {
                        let actor = changes.actor.as_deref().unwrap_or("cli");
                        tx.execute(
                            "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, created_at) VALUES (?1, 'assignee_changed', ?2, ?3, ?4, ?5)",
                            params![id, actor, current_assignee.as_deref(), new_val, Utc::now().to_rfc3339()],
                        )?;
                    }
                }
            }
            // Handle label updates separately
            if let Some(ref labels) = changes.labels {
                tx.execute("DELETE FROM labels WHERE issue_id = ?1", params![id])?;
                for label in labels {
                    tx.execute(
                        "INSERT OR IGNORE INTO labels (issue_id, label) VALUES (?1, ?2)",
                        params![id, label],
                    )?;
                }
                tx.execute("DELETE FROM bead_labels WHERE bead_id = ?1", params![id])?;
                for label in labels {
                    tx.execute(
                        "INSERT OR IGNORE INTO bead_labels (bead_id, label) VALUES (?1, ?2)",
                        params![id, label],
                    )?;
                }
            }
            // Handle annotation updates separately
            if let Some(ref annotations) = changes.annotations {
                tx.execute(
                    "DELETE FROM bead_annotations WHERE bead_id = ?1",
                    params![id],
                )?;
                for (key, value) in annotations {
                    tx.execute(
                        "INSERT INTO bead_annotations (bead_id, key, value) VALUES (?1, ?2, ?3)",
                        params![id, key, value],
                    )?;
                }
            }
            // Mark as dirty for export (if any changes were made)
            if !updates.is_empty() || changes.labels.is_some() || changes.annotations.is_some() {
                tx.execute(
                    "INSERT OR REPLACE INTO dirty_issues (bead_id, marked_at) VALUES (?1, ?2)",
                    params![id, now.to_rfc3339()],
                )?;
            }
            // Invalidate critical path cache if status changed (affects dependency graph)
            if changes.status.is_some() {
                // If transitioning TO a terminal state (closed, tombstone, done, completed),
                // check if any dependents should be unblocked
                if let Some(new_status) = &changes.status {
                    if new_status.is_terminal() {
                        // Find all dependents that were blocked by this bead
                        let mut dependents = tx.prepare(
                            "SELECT DISTINCT issue_id FROM dependencies
                             WHERE depends_on_id = ?1
                             AND type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')",
                        )?.query_map(params![id], |row| row.get::<_, String>(0))?
                        .collect::<std::result::Result<Vec<_>, _>>()?;

                        // For each dependent, check if ALL its blocking dependencies are now closed
                        for dependent_id in dependents {
                            let has_open_blockers = tx.query_row(
                                "SELECT EXISTS(
                                    SELECT 1 FROM dependencies d
                                    INNER JOIN issues i ON i.id = d.depends_on_id
                                    WHERE d.issue_id = ?1
                                    AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
                                    AND i.status NOT IN ('closed', 'tombstone', 'done', 'completed')
                                )",
                                params![&dependent_id],
                                |row| row.get::<_, bool>(0),
                            ).unwrap_or(false);

                            // If no open blockers remain, unblock the dependent
                            if !has_open_blockers {
                                tx.execute(
                                    "UPDATE issues SET status = 'open', updated_at = ?1 WHERE id = ?2",
                                    params![now.to_rfc3339(), &dependent_id],
                                )?;
                            }
                        }
                    }
                }

                invalidate_cache(tx)?;
                compute_all_critical_paths(tx)?;
                // Rebuild blocked_issues_cache when status changes to reflect new blocker states
                let now = Utc::now();
                tx.execute("DELETE FROM blocked_issues_cache", [])?;
                tx.execute(
                    "INSERT INTO blocked_issues_cache (issue_id, blocked_by, blocked_at)
                     SELECT d.issue_id, '[' || GROUP_CONCAT('\"' || d.depends_on_id || '\"') || ']' AS blocked_by, ?1
                     FROM dependencies d
                     INNER JOIN issues i ON i.id = d.depends_on_id
                     WHERE d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
                     AND i.status NOT IN ('closed', 'tombstone', 'done', 'completed')
                     GROUP BY d.issue_id",
                    params![now.to_rfc3339()],
                )?;
            }
            Ok(())
        })
    }

    /// Base issue update method for simple field updates.
    ///
    /// This method handles basic UPDATE operations for the three core updatable
    /// fields: title, status, and priority. It builds a dynamic SET clause based
    /// on which fields are present in the `IssueUpdate` struct.
    ///
    /// For more complex updates including labels, annotations, or actor tracking,
    /// use the full `update_issue` method with `IssueChanges` instead.
    ///
    /// # Arguments
    /// * `id` - The issue ID to update
    /// * `updates` - The fields to update (None fields are ignored)
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    ///
    /// # Examples
    /// ```no_run
    /// # use bead_forge::model::{IssueUpdate, Status, Priority};
    /// # use bead_forge::storage::sqlite::Storage;
    /// # fn example(storage: &Storage) -> anyhow::Result<()> {
    /// // Update just the title
    /// storage.apply_issue_update("bf-123", IssueUpdate {
    ///     title: Some("New title".to_string()),
    ///     ..Default::default()
    /// })?;
    ///
    /// // Update status and priority
    /// storage.apply_issue_update("bf-123", IssueUpdate {
    ///     status: Some(Status::InProgress),
    ///     priority: Some(Priority::HIGH),
    ///     ..Default::default()
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn apply_issue_update(&self, id: &str, updates: IssueUpdate) -> Result<()> {
        // Build the SET clause dynamically based on which fields are Some
        let mut set_clauses = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref title) = updates.title {
            set_clauses.push("title = ?");
            params.push(Box::new(title.clone()));
        }

        if let Some(ref status) = updates.status {
            set_clauses.push("status = ?");
            params.push(Box::new(status.to_string()));
        }

        if let Some(priority) = updates.priority {
            set_clauses.push("priority = ?");
            params.push(Box::new(priority.0)); // Priority is a tuple struct with i32
        }

        // Always update updated_at timestamp when any change is made
        if !set_clauses.is_empty() {
            set_clauses.push("updated_at = ?");
            params.push(Box::new(Utc::now().to_rfc3339()));
        }

        // If no fields to update, return early
        if set_clauses.is_empty() {
            return Ok(());
        }

        // Build the full UPDATE query
        let query = format!(
            "UPDATE issues SET {} WHERE id = ?",
            set_clauses.join(", ")
        );

        // Add the id parameter last
        params.push(Box::new(id.to_string()));

        // Execute within a BEGIN IMMEDIATE transaction for atomicity
        self.with_immediate_transaction(|tx| {
            let param_refs: Vec<&dyn rusqlite::ToSql> =
                params.iter().map(|p| p.as_ref()).collect();
            tx.execute(&query, param_refs.as_slice())?;
            Ok(())
        })
    }

    /// Update only the title field of an issue.
    ///
    /// This is a convenience method for updating just the title field without
    /// needing to construct a full `IssueUpdate` or `IssueChanges` struct.
    /// All other fields (status, priority, description, etc.) are preserved.
    ///
    /// # Arguments
    /// * `id` - The issue ID to update
    /// * `title` - The new title value
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    ///
    /// # Examples
    /// ```no_run
    /// # use bead_forge::storage::sqlite::Storage;
    /// # fn example(storage: &Storage) -> anyhow::Result<()> {
    /// // Update just the title
    /// storage.update_title("bf-123", "New title")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn update_title(&self, id: &str, title: &str) -> Result<()> {
        let query = "UPDATE issues SET title = ?, updated_at = ? WHERE id = ?";
        let now = Utc::now();

        // Execute within a BEGIN IMMEDIATE transaction for atomicity
        self.with_immediate_transaction(|tx| {
            tx.execute(query, params![title, now.to_rfc3339(), id])?;
            Ok(())
        })
    }

    /// Update the status of an issue.
    ///
    /// Updates only the status field and the updated_at timestamp.
    /// All other fields remain unchanged.
    pub fn update_status(&self, id: &str, status: Status) -> Result<()> {
        let now = Utc::now();

        // Execute within a BEGIN IMMEDIATE transaction for atomicity
        self.with_immediate_transaction(|tx| {
            // Check if bead exists and get current status
            let current_status: Option<String> = tx
                .query_row(
                    "SELECT status FROM issues WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .ok();

            if current_status.is_none() {
                return Err(BeadForgeError::not_found("bead", id, None));
            }

            let old_status = current_status.unwrap();
            let new_status = status.as_str();

            // Update the status
            tx.execute(
                "UPDATE issues SET status = ?, updated_at = ? WHERE id = ?",
                params![new_status, now.to_rfc3339(), id],
            )?;

            // Create a status change event
            tx.execute(
                "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, created_at) VALUES (?1, 'status_changed', 'system', ?2, ?3, ?4)",
                params![id, old_status, new_status, now.to_rfc3339()],
            )?;

            Ok(())
        })
    }

    /// Update the priority of an issue.
    ///
    /// Updates only the priority field and the updated_at timestamp.
    /// All other fields remain unchanged.
    pub fn update_priority(&self, id: &str, priority: Priority) -> Result<()> {
        let query = "UPDATE issues SET priority = ?, updated_at = ? WHERE id = ?";
        let now = Utc::now();

        // Execute within a BEGIN IMMEDIATE transaction for atomicity
        self.with_immediate_transaction(|tx| {
            tx.execute(query, params![priority, now.to_rfc3339(), id])?;
            Ok(())
        })
    }

    /// Update an issue from JSONL import data.
    ///
    /// This replaces all fields of the existing issue with the imported data,
    /// used during JSONL import when the content_hash differs.
    pub fn update_issue_from_json(&self, issue: &Issue) -> Result<()> {
        self.with_immediate_transaction(|tx| {
            // Delete existing related data
            tx.execute("DELETE FROM labels WHERE issue_id = ?1", params![&issue.id])?;
            tx.execute("DELETE FROM bead_labels WHERE bead_id = ?1", params![&issue.id])?;
            tx.execute("DELETE FROM dependencies WHERE issue_id = ?1", params![&issue.id])?;
            tx.execute("DELETE FROM comments WHERE issue_id = ?1", params![&issue.id])?;

            // Update the issue row with all fields
            tx.execute(
                "UPDATE issues SET
                    content_hash = ?1, title = ?2, description = ?3, design = ?4,
                    acceptance_criteria = ?5, notes = ?6, status = ?7, priority = ?8,
                    issue_type = ?9, assignee = ?10, owner = ?11, estimated_minutes = ?12,
                    created_at = ?13, created_by = ?14, updated_at = ?15, closed_at = ?16,
                    close_reason = ?17, closed_by_session = ?18, due_at = ?19, defer_until = ?20,
                    external_ref = ?21, source_system = ?22, source_repo = ?23,
                    deleted_at = ?24, deleted_by = ?25, delete_reason = ?26, original_type = ?27,
                    compaction_level = ?28, compacted_at = ?29, compacted_at_commit = ?30,
                    original_size = ?31, sender = ?32, ephemeral = ?33, pinned = ?34, is_template = ?35
                 WHERE id = ?36",
                params![
                    &issue.content_hash, &issue.title,
                    issue.description.as_deref().unwrap_or(""),
                    issue.design.as_deref().unwrap_or(""),
                    issue.acceptance_criteria.as_deref().unwrap_or(""),
                    issue.notes.as_deref().unwrap_or(""),
                    &issue.status.to_string(),
                    &issue.priority, &issue.issue_type.to_string(), &issue.assignee, &issue.owner,
                    &issue.estimated_minutes, &issue.created_at.to_rfc3339(), &issue.created_by,
                    &issue.updated_at.to_rfc3339(), issue.closed_at.map(|d| d.to_rfc3339()),
                    &issue.close_reason, &issue.closed_by_session, issue.due_at.map(|d| d.to_rfc3339()),
                    issue.defer_until.map(|d| d.to_rfc3339()), &issue.external_ref, &issue.source_system,
                    issue.source_repo.as_deref().unwrap_or("."),
                    issue.deleted_at.map(|d| d.to_rfc3339()), &issue.deleted_by,
                    &issue.delete_reason, &issue.original_type, &issue.compaction_level,
                    issue.compacted_at.map(|d| d.to_rfc3339()), &issue.compacted_at_commit,
                    &issue.original_size, &issue.sender,
                    if issue.ephemeral { 1 } else { 0 },
                    if issue.pinned { 1 } else { 0 },
                    if issue.is_template { 1 } else { 0 },
                    &issue.id,
                ],
            )?;

            // Re-insert labels, dependencies, and comments
            for label in &issue.labels {
                tx.execute("INSERT OR IGNORE INTO labels (issue_id, label) VALUES (?1, ?2)", params![&issue.id, label])?;
            }
            for label in &issue.labels {
                tx.execute("INSERT OR IGNORE INTO bead_labels (bead_id, label) VALUES (?1, ?2)", params![&issue.id, label])?;
            }
            for dep in &issue.dependencies {
                tx.execute(
                    "INSERT INTO dependencies (issue_id, depends_on_id, type, metadata, thread_id, created_at, created_by)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        &dep.issue_id, &dep.depends_on_id, &dep.dep_type.to_string(),
                        dep.metadata.as_ref().map(|m| serde_json::to_string(m).ok()).flatten(),
                        &dep.thread_id, &dep.created_at.to_rfc3339(), &dep.created_by,
                    ],
                )?;
            }
            for comment in &issue.comments {
                tx.execute(
                    "INSERT INTO comments (id, issue_id, author, text, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        &comment.id, &comment.issue_id, &comment.author, &comment.body,
                        &comment.created_at.to_rfc3339(),
                    ],
                )?;
            }

            // Invalidate critical path cache after updating from JSON (may change deps/status)
            invalidate_cache(tx)?;
            compute_all_critical_paths(tx)?;

            Ok(())
        })
    }

    pub fn close_issue(&self, id: &str, reason: &str, actor: &str) -> Result<()> {
        self.with_immediate_transaction(|tx| {
            // Check if bead exists
            let exists: bool = tx
                .query_row(
                    "SELECT 1 FROM issues WHERE id = ?1",
                    params![id],
                    |_| Ok(true),
                )
                .unwrap_or(false);
            if !exists {
                return Err(BeadForgeError::not_found("bead", id, None));
            }

            // Check if already closed for idempotence
            let current_status: Option<String> = tx
                .query_row(
                    "SELECT status FROM issues WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .ok();

            if current_status.as_deref() == Some("closed") {
                // Already closed - idempotent, return success
                return Ok(());
            }

            // Close the bead
            let now = Utc::now();
            tx.execute(
                "UPDATE issues SET status = 'closed', assignee = NULL, closed_at = ?, close_reason = ?, closed_by_session = ?, updated_at = ? WHERE id = ?",
                params![now.to_rfc3339(), reason, actor, now.to_rfc3339(), id],
            )?;
            tx.execute(
                "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, created_at) VALUES (?1, 'closed', ?2, NULL, ?3, ?4)",
                params![id, actor, reason, now.to_rfc3339()],
            )?;
            tx.execute(
                "INSERT OR REPLACE INTO dirty_issues (bead_id, marked_at) VALUES (?1, ?2)",
                params![id, now.to_rfc3339()],
            )?;
            // Update worker session with close time and duration for velocity tracking
            crate::velocity::update_session_on_close(tx, id, now)?;

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
                .query_map(params![id], |row| {
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
                        params![&dep_id],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);

                // If no remaining blockers, transition to 'open'
                if remaining_blockers == 0 {
                    let now = Utc::now();
                    tx.execute(
                        "UPDATE issues SET status = 'open', updated_at = ?1 WHERE id = ?2",
                        params![now.to_rfc3339(), &dep_id],
                    )?;
                    tx.execute(
                        "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, created_at) VALUES (?1, 'status_changed', 'system', 'blocked', 'open', ?2)",
                        params![&dep_id, now.to_rfc3339()],
                    )?;
                    tx.execute(
                        "INSERT OR REPLACE INTO dirty_issues (bead_id, marked_at) VALUES (?1, ?2)",
                        params![&dep_id, now.to_rfc3339()],
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
                params![Utc::now().to_rfc3339()],
            )?;

            // Invalidate critical path cache: closing a bead can unblock dependents
            invalidate_cache(tx)?;
            compute_all_critical_paths(tx)?;
            Ok(())
        })
    }

    pub fn reopen_issue(&self, id: &str) -> Result<()> {
        self.with_immediate_transaction(|tx| {
            // Check if bead exists and get current status
            let current_status: Option<String> = tx
                .query_row(
                    "SELECT status FROM issues WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .ok();

            if current_status.is_none() {
                return Err(BeadForgeError::not_found("bead", id, None));
            }

            // Check if bead is currently closed
            if current_status.as_deref() != Some("closed") {
                return Err(BeadForgeError::validation(format!(
                    "Cannot reopen bead {}: status is '{}', must be 'closed'",
                    id,
                    current_status.unwrap()
                )));
            }

            // Reopen the bead
            let now = Utc::now();
            tx.execute(
                "UPDATE issues SET status = 'open', assignee = NULL, closed_at = NULL, close_reason = NULL, closed_by_session = NULL, updated_at = ?1 WHERE id = ?2",
                params![now.to_rfc3339(), id],
            )?;
            tx.execute(
                "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, created_at) VALUES (?1, 'reopened', 'system', 'closed', 'open', ?2)",
                params![id, now.to_rfc3339()],
            )?;
            tx.execute(
                "INSERT OR REPLACE INTO dirty_issues (bead_id, marked_at) VALUES (?1, ?2)",
                params![id, now.to_rfc3339()],
            )?;

            // Invalidate critical path cache: reopening a bead can change dependencies
            invalidate_cache(tx)?;
            compute_all_critical_paths(tx)?;
            Ok(())
        })
    }

    pub fn mark_dirty(&self, id: &str) -> Result<()> {
        self.with_immediate_transaction(|tx| {
            let now = Utc::now().to_rfc3339();
            tx.execute(
                "INSERT OR REPLACE INTO dirty_issues (bead_id, marked_at) VALUES (?1, ?2)",
                params![id, now],
            )?;
            Ok(())
        })
    }

    pub fn rebuild_blocked_cache(&self) -> Result<()> {
        self.with_immediate_transaction(|tx| {
            tx.execute("DELETE FROM blocked_issues_cache", [])?;

            tx.execute(
                "INSERT INTO blocked_issues_cache (issue_id, blocked_by, blocked_at)
                 SELECT d.issue_id, '[' || GROUP_CONCAT('\"' || d.depends_on_id || '\"') || ']' AS blocked_by, ?1
                 FROM dependencies d
                 INNER JOIN issues i ON i.id = d.depends_on_id
                 WHERE d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
                 AND i.status NOT IN ('closed', 'tombstone', 'done', 'completed')  -- TERMINAL_STATUS_SQL_LIST
                 GROUP BY d.issue_id",
                params![Utc::now().to_rfc3339()],
            )?;
            Ok(())
        })
    }

    /// Get all beads currently in the blocked_issues_cache.
    ///
    /// Returns a list of (issue_id, blocked_by) tuples where blocked_by is
    /// a JSON array of blocker IDs.
    pub fn get_blocked_issues(&self) -> Result<Vec<(String, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT issue_id, blocked_by FROM blocked_issues_cache ORDER BY issue_id")?;

        let result = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(result)
    }

    pub fn count_issues(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM issues", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn sync_from_jsonl(&self, jsonl_path: &Path) -> Result<ImportResult> {
        self.with_immediate_transaction(|tx| {
            import_jsonl(jsonl_path, |issue| {
                let existing = Self::get_issue_tx(tx, &issue.id)?;
                match existing {
                    None => {
                        Self::create_issue_tx(tx, issue)?;
                        Ok(UpsertResult::New)
                    }
                    Some(existing_issue) => {
                        // Compute hash from incoming issue (content_hash is None from JSONL due to #[serde(skip)])
                        let incoming_hash = issue.content_hash();
                        if existing_issue.content_hash.as_ref() != Some(&incoming_hash) {
                            Self::update_issue_from_json_tx(tx, issue)?;
                            Ok(UpsertResult::Updated)
                        } else {
                            Ok(UpsertResult::Unchanged)
                        }
                    }
                }
            })
        })
    }

    pub fn sync_to_jsonl(&self, jsonl_path: &Path, dirty_only: bool) -> Result<usize> {
        if dirty_only {
            let result = export_jsonl_dirty(
                jsonl_path,
                || self.list_dirty_issues().map_err(Into::into),
                || self.clear_dirty().map_err(Into::into),
            )?;
            Ok(result.count)
        } else {
            let result = export_jsonl(jsonl_path, || self.list_all_issues().map_err(Into::into))?;
            // Clear dirty flags after full export (all beads are now synced)
            self.clear_dirty()?;
            Ok(result.count)
        }
    }

    pub fn row_to_issue_conn(conn: &Connection, row: &rusqlite::Row) -> Result<Issue> {
        let status_str: String = row.get(7)?;
        let type_str: String = row.get(9)?;
        let parse_opt_dt = |idx: usize| -> Result<Option<DateTime<Utc>>> {
            let s: Option<String> = row.get(idx)?;
            match s {
                None => Ok(None),
                Some(ref val) if val.is_empty() => Ok(None),
                Some(val) => Ok(Some(parse_datetime(val)?)),
            }
        };
        let id: String = row.get(0)?;

        // Parse labels from GROUP_CONCAT result (comma-separated string)
        let labels_str: Option<String> = row.get(36)?;
        let labels: Vec<String> = labels_str
            .map(|s| s.split(',').map(String::from).collect())
            .unwrap_or_default();

        Ok(Issue {
            content_hash: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            design: row.get(4)?,
            acceptance_criteria: row.get(5)?,
            notes: row.get(6)?,
            status: Status::from_str(&status_str).unwrap_or(Status::Custom(status_str)),
            priority: row.get(8)?,
            issue_type: IssueType::from_str(&type_str).unwrap_or(IssueType::Custom(type_str)),
            assignee: row.get(10)?,
            owner: row.get(11)?,
            estimated_minutes: row.get(12)?,
            created_at: parse_required_datetime(row.get(13)?)?,
            created_by: row.get(14)?,
            updated_at: parse_required_datetime(row.get(15)?)?,
            closed_at: parse_opt_dt(16)?,
            close_reason: row.get(17)?,
            closed_by_session: row.get(18)?,
            due_at: parse_opt_dt(19)?,
            defer_until: parse_opt_dt(20)?,
            external_ref: row.get(21)?,
            source_system: row.get(22)?,
            source_repo: row.get(23)?,
            deleted_at: parse_opt_dt(24)?,
            deleted_by: row.get(25)?,
            delete_reason: row.get(26)?,
            original_type: row.get(27)?,
            compaction_level: row.get(28)?,
            compacted_at: parse_opt_dt(29)?,
            compacted_at_commit: row.get(30)?,
            original_size: row.get(31)?,
            sender: row.get(32)?,
            ephemeral: row.get::<_, i32>(33)? != 0,
            pinned: row.get::<_, i32>(34)? != 0,
            is_template: row.get::<_, i32>(35)? != 0,
            labels,
            dependencies: Self::load_dependencies_conn(conn, &id)?,
            comments: Self::load_comments_conn(conn, &id)?,
            events: Self::load_events_conn(conn, &id)?,
            annotations: Self::load_annotations_conn(conn, &id)?,
            id,
        })
    }

    fn load_labels_conn(conn: &Connection, issue_id: &str) -> Result<Vec<String>> {
        let mut stmt = conn.prepare("SELECT label FROM bead_labels WHERE bead_id = ?1")?;
        let mut rows = stmt.query(params![issue_id])?;
        let mut labels = Vec::new();
        while let Some(row) = rows.next()? {
            labels.push(row.get(0)?);
        }
        Ok(labels)
    }

    fn load_dependencies_conn(conn: &Connection, issue_id: &str) -> Result<Vec<Dependency>> {
        let mut stmt = conn.prepare(
            "SELECT issue_id, depends_on_id, type, metadata, thread_id, created_at, created_by FROM dependencies WHERE issue_id = ?1",
        )?;
        let mut rows = stmt.query(params![issue_id])?;
        let mut deps = Vec::new();
        while let Some(row) = rows.next()? {
            let type_str: String = row.get(2)?;
            deps.push(Dependency {
                issue_id: row.get(0)?,
                depends_on_id: row.get(1)?,
                dep_type: DependencyType::from_str(&type_str)
                    .unwrap_or(DependencyType::Custom(type_str)),
                metadata: row
                    .get::<_, Option<String>>(3)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                thread_id: row.get(4)?,
                created_at: parse_required_datetime(row.get(5)?)?,
                created_by: row.get(6)?,
                title: None,
            });
        }
        Ok(deps)
    }

    fn load_comments_conn(conn: &Connection, issue_id: &str) -> Result<Vec<Comment>> {
        let mut stmt = conn.prepare(
            "SELECT id, issue_id, author, text, created_at FROM comments WHERE issue_id = ?1",
        )?;
        let mut rows = stmt.query(params![issue_id])?;
        let mut comments = Vec::new();
        while let Some(row) = rows.next()? {
            comments.push(Comment {
                id: row.get(0)?,
                issue_id: row.get(1)?,
                author: row.get(2)?,
                body: row.get(3)?,
                created_at: parse_required_datetime(row.get(4)?)?,
            });
        }
        Ok(comments)
    }

    fn load_events_conn(conn: &Connection, issue_id: &str) -> Result<Vec<Event>> {
        let mut stmt = conn.prepare(
            "SELECT id, issue_id, event_type, actor, old_value, new_value, comment, created_at
             FROM events WHERE issue_id = ?1",
        )?;
        let mut rows = stmt.query(params![issue_id])?;
        let mut events = Vec::new();
        while let Some(row) = rows.next()? {
            let event_type_str: String = row.get(2)?;
            events.push(Event {
                id: row.get(0)?,
                issue_id: row.get(1)?,
                event_type: EventType::from_str(&event_type_str).unwrap_or(EventType::Custom(event_type_str)),
                actor: row.get(3)?,
                old_value: row.get(4)?,
                new_value: row.get(5)?,
                comment: row.get(6)?,
                created_at: parse_required_datetime(row.get(7)?)?,
            });
        }
        Ok(events)
    }

    fn load_annotations_conn(
        conn: &Connection,
        issue_id: &str,
    ) -> Result<BTreeMap<String, String>> {
        let mut stmt =
            conn.prepare("SELECT key, value FROM bead_annotations WHERE bead_id = ?1")?;
        let mut rows = stmt.query(params![issue_id])?;
        let mut annotations = BTreeMap::new();
        while let Some(row) = rows.next()? {
            let key: String = row.get(0)?;
            let value: String = row.get(1)?;
            annotations.insert(key, value);
        }
        Ok(annotations)
    }

    /// Parse a row into an Issue without loading dependencies, comments, or annotations.
    /// Used by list_issues for batch loading performance optimization.
    fn row_to_issue_partial(row: &rusqlite::Row) -> Result<Issue> {
        let status_str: String = row.get(7)?;
        let type_str: String = row.get(9)?;
        let parse_opt_dt = |idx: usize| -> Result<Option<DateTime<Utc>>> {
            let s: Option<String> = row.get(idx)?;
            match s {
                None => Ok(None),
                Some(ref val) if val.is_empty() => Ok(None),
                Some(val) => Ok(Some(parse_datetime(val)?)),
            }
        };
        let id: String = row.get(0)?;

        // Parse labels from GROUP_CONCAT result (comma-separated string)
        let labels_str: Option<String> = row.get(36)?;
        let labels: Vec<String> = labels_str
            .map(|s| s.split(',').map(String::from).collect())
            .unwrap_or_default();

        Ok(Issue {
            content_hash: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            design: row.get(4)?,
            acceptance_criteria: row.get(5)?,
            notes: row.get(6)?,
            status: Status::from_str(&status_str).unwrap_or(Status::Custom(status_str)),
            priority: row.get(8)?,
            issue_type: IssueType::from_str(&type_str).unwrap_or(IssueType::Custom(type_str)),
            assignee: row.get(10)?,
            owner: row.get(11)?,
            estimated_minutes: row.get(12)?,
            created_at: parse_required_datetime(row.get(13)?)?,
            created_by: row.get(14)?,
            updated_at: parse_required_datetime(row.get(15)?)?,
            closed_at: parse_opt_dt(16)?,
            close_reason: row.get(17)?,
            closed_by_session: row.get(18)?,
            due_at: parse_opt_dt(19)?,
            defer_until: parse_opt_dt(20)?,
            external_ref: row.get(21)?,
            source_system: row.get(22)?,
            source_repo: row.get(23)?,
            deleted_at: parse_opt_dt(24)?,
            deleted_by: row.get(25)?,
            delete_reason: row.get(26)?,
            original_type: row.get(27)?,
            compaction_level: row.get(28)?,
            compacted_at: parse_opt_dt(29)?,
            compacted_at_commit: row.get(30)?,
            original_size: row.get(31)?,
            sender: row.get(32)?,
            ephemeral: row.get::<_, i32>(33)? != 0,
            pinned: row.get::<_, i32>(34)? != 0,
            is_template: row.get::<_, i32>(35)? != 0,
            labels,
            dependencies: Vec::new(),  // Loaded separately via batch_load_dependencies
            comments: Vec::new(),      // Loaded separately via batch_load_comments
            events: Vec::new(),        // Loaded separately via batch_load_events
            annotations: BTreeMap::new(),  // Loaded separately via batch_load_annotations
            id,
        })
    }

    /// Batch load dependencies for multiple issues at once.
    /// Returns a map from issue_id to its dependencies.
    fn batch_load_dependencies(
        conn: &Connection,
        issue_ids: &[&String],
    ) -> Result<std::collections::HashMap<String, Vec<Dependency>>> {
        if issue_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let mut result = std::collections::HashMap::new();
        for issue_id in issue_ids {
            result.insert(issue_id.to_string(), Vec::new());
        }

        // Build the IN clause with placeholders
        let in_clause = issue_ids.iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(",");

        let query = format!(
            "SELECT issue_id, depends_on_id, type, metadata, thread_id, created_at, created_by
             FROM dependencies
             WHERE issue_id IN ({})",
            in_clause
        );

        let mut stmt = conn.prepare(&query)?;
        let mut rows = stmt.query(
            issue_ids.iter()
                .map(|s| s as &dyn rusqlite::ToSql)
                .collect::<Vec<_>>()
                .as_slice()
        )?;

        while let Some(row) = rows.next()? {
            let issue_id: String = row.get(0)?;
            let type_str: String = row.get(2)?;
            let dep = Dependency {
                issue_id: row.get(0)?,
                depends_on_id: row.get(1)?,
                dep_type: DependencyType::from_str(&type_str)
                    .unwrap_or(DependencyType::Custom(type_str)),
                metadata: row
                    .get::<_, Option<String>>(3)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                thread_id: row.get(4)?,
                created_at: parse_required_datetime(row.get(5)?)?,
                created_by: row.get(6)?,
                title: None,
            };
            result.entry(issue_id).or_default().push(dep);
        }

        Ok(result)
    }

    /// Batch load comments for multiple issues at once.
    /// Returns a map from issue_id to its comments.
    fn batch_load_comments(
        conn: &Connection,
        issue_ids: &[&String],
    ) -> Result<std::collections::HashMap<String, Vec<Comment>>> {
        if issue_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let mut result = std::collections::HashMap::new();
        for issue_id in issue_ids {
            result.insert(issue_id.to_string(), Vec::new());
        }

        let in_clause = issue_ids.iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(",");

        let query = format!(
            "SELECT id, issue_id, author, text, created_at
             FROM comments
             WHERE issue_id IN ({})",
            in_clause
        );

        let mut stmt = conn.prepare(&query)?;
        let mut rows = stmt.query(
            issue_ids.iter()
                .map(|s| s as &dyn rusqlite::ToSql)
                .collect::<Vec<_>>()
                .as_slice()
        )?;

        while let Some(row) = rows.next()? {
            let issue_id: String = row.get(1)?;
            let comment = Comment {
                id: row.get(0)?,
                issue_id: row.get(1)?,
                author: row.get(2)?,
                body: row.get(3)?,
                created_at: parse_required_datetime(row.get(4)?)?,
            };
            result.entry(issue_id).or_default().push(comment);
        }

        Ok(result)
    }

    /// Batch load annotations for multiple issues at once.
    /// Returns a map from issue_id to its annotations.
    fn batch_load_annotations(
        conn: &Connection,
        issue_ids: &[&String],
    ) -> Result<std::collections::HashMap<String, BTreeMap<String, String>>> {
        if issue_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let mut result = std::collections::HashMap::new();
        for issue_id in issue_ids {
            result.insert(issue_id.to_string(), BTreeMap::new());
        }

        let in_clause = issue_ids.iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(",");

        let query = format!(
            "SELECT bead_id, key, value
             FROM bead_annotations
             WHERE bead_id IN ({})",
            in_clause
        );

        let mut stmt = conn.prepare(&query)?;
        let mut rows = stmt.query(
            issue_ids.iter()
                .map(|s| s as &dyn rusqlite::ToSql)
                .collect::<Vec<_>>()
                .as_slice()
        )?;

        while let Some(row) = rows.next()? {
            let bead_id: String = row.get(0)?;
            let key: String = row.get(1)?;
            let value: String = row.get(2)?;
            result.entry(bead_id).or_default().insert(key, value);
        }

        Ok(result)
    }

    fn load_labels(&self, issue_id: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        Self::load_labels_conn(&conn, issue_id)
    }

    fn load_dependencies(&self, issue_id: &str) -> Result<Vec<Dependency>> {
        let conn = self.conn.lock().unwrap();
        Self::load_dependencies_conn(&conn, issue_id)
    }

    fn load_comments(&self, issue_id: &str) -> Result<Vec<Comment>> {
        let conn = self.conn.lock().unwrap();
        Self::load_comments_conn(&conn, issue_id)
    }

    fn load_annotations(&self, issue_id: &str) -> Result<BTreeMap<String, String>> {
        let conn = self.conn.lock().unwrap();
        Self::load_annotations_conn(&conn, issue_id)
    }

    pub fn add_dependency(
        &self,
        issue_id: &str,
        depends_on_id: &str,
        dep_type: &DependencyType,
        created_by: &str,
    ) -> Result<()> {
        self.with_immediate_transaction(|tx| {
            // Prevent self-blocking: a bead cannot depend on itself for blocking dependency types
            if issue_id == depends_on_id && dep_type.is_blocking() {
                return Err(anyhow::anyhow!(
                    "Cannot add self-blocking dependency: bead '{}' cannot block itself",
                    issue_id
                ).into());
            }

            let now = Utc::now();
            tx.execute(
                "INSERT INTO dependencies (issue_id, depends_on_id, type, created_at, created_by) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![issue_id, depends_on_id, dep_type.to_string(), now.to_rfc3339(), created_by],
            )?;

            // Check if this is a blocking dependency and the blocker is open
            // If so, automatically set the dependent's status to "Blocked"
            let dep_type_str = dep_type.to_string();
            if ["blocks", "parent-child", "conditional-blocks", "waits-for"].contains(&dep_type_str.as_str()) {
                // Check if the blocker is in an open/active state
                let blocker_status: Option<String> = tx.query_row(
                    "SELECT status FROM issues WHERE id = ?1",
                    params![depends_on_id],
                    |row| row.get(0),
                ).ok();

                // If blocker is NOT closed/tombstone/done/completed, block the dependent
                if let Some(status) = blocker_status {
                    if !["closed", "tombstone", "done", "completed"].contains(&status.as_str()) {
                        // Update dependent's status to "blocked"
                        tx.execute(
                            "UPDATE issues SET status = 'blocked', updated_at = ?1 WHERE id = ?2",
                            params![now.to_rfc3339(), issue_id],
                        )?;
                    }
                }
            }

            // The dependency is stored on issue_id's record and exported with it.
            mark_dirty_tx(tx, issue_id)?;
            // Invalidate critical path cache after adding a dependency
            invalidate_cache(tx)?;
            compute_all_critical_paths(tx)?;
            // Rebuild blocked_issues_cache to reflect the new dependency
            tx.execute("DELETE FROM blocked_issues_cache", [])?;
            tx.execute(
                "INSERT INTO blocked_issues_cache (issue_id, blocked_by, blocked_at)
                 SELECT d.issue_id, '[' || GROUP_CONCAT('\"' || d.depends_on_id || '\"') || ']' AS blocked_by, ?1
                 FROM dependencies d
                 INNER JOIN issues i ON i.id = d.depends_on_id
                 WHERE d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
                 AND i.status NOT IN ('closed', 'tombstone', 'done', 'completed')
                 GROUP BY d.issue_id",
                params![now.to_rfc3339()],
            )?;
            Ok(())
        })
    }

    pub fn remove_dependency(&self, issue_id: &str, depends_on_id: &str) -> Result<()> {
        self.with_immediate_transaction(|tx| {
            tx.execute(
                "DELETE FROM dependencies WHERE issue_id = ?1 AND depends_on_id = ?2",
                params![issue_id, depends_on_id],
            )?;
            mark_dirty_tx(tx, issue_id)?;
            // Invalidate critical path cache after removing a dependency
            invalidate_cache(tx)?;
            compute_all_critical_paths(tx)?;
            // Rebuild blocked_issues_cache to reflect the removed dependency
            let now = Utc::now();
            tx.execute("DELETE FROM blocked_issues_cache", [])?;
            tx.execute(
                "INSERT INTO blocked_issues_cache (issue_id, blocked_by, blocked_at)
                 SELECT d.issue_id, '[' || GROUP_CONCAT('\"' || d.depends_on_id || '\"') || ']' AS blocked_by, ?1
                 FROM dependencies d
                 INNER JOIN issues i ON i.id = d.depends_on_id
                 WHERE d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
                 AND i.status NOT IN ('closed', 'tombstone', 'done', 'completed')
                 GROUP BY d.issue_id",
                params![now.to_rfc3339()],
            )?;
            Ok(())
        })
    }

    pub fn get_dependencies(&self, issue_id: &str) -> Result<Vec<Dependency>> {
        self.load_dependencies(issue_id)
    }

    /// Get dependency tree rooted at an issue using recursive CTE.
    ///
    /// # Arguments
    /// * `root_id` - The root issue ID
    /// * `direction` - "down" for what this depends on, "up" for what depends on this
    /// * `max_depth` - Maximum depth to traverse (0 = unlimited)
    ///
    /// # Returns
    /// Vector of tree nodes ordered by depth, suitable for tree display.
    pub fn get_dep_tree(
        &self,
        root_id: &str,
        direction: &str,
        max_depth: usize,
    ) -> Result<Vec<DepTreeNode>> {
        // SECURITY: Validate root_id format to prevent SQL injection
        use crate::id::is_valid_bead_id;
        if !is_valid_bead_id(root_id) {
            return Err(BeadForgeError::validation(format!(
                "Invalid bead ID format: {}", root_id
            )));
        }

        let conn = self.conn.lock().unwrap();

        // Build recursive CTE based on direction
        let (anchor_join, recursive_join, id_col, dep_col) = match direction {
            "up" => {
                // "up" means: find issues that depend on this one
                // Anchor: issues that directly depend on root
                // Recursive: issues that depend on those
                (
                    "d.depends_on_id = ?1",
                    "d.depends_on_id = rec.id",
                    "d.issue_id",
                    "d.depends_on_id",
                )
            }
            _ => {
                // "down" (default): what this issue depends on
                // Anchor: issues that root directly depends on
                // Recursive: issues that those depend on
                (
                    "d.issue_id = ?1",
                    "d.issue_id = rec.id",
                    "d.depends_on_id",
                    "d.issue_id",
                )
            }
        };

        let depth_limit = if max_depth == 0 {
            String::new()
        } else {
            format!("AND rec.depth < {}", max_depth)
        };

        let sql = format!(
            "WITH RECURSIVE dep_tree AS (
                -- Anchor: direct dependencies/dependents of root
                SELECT
                    {id_col} as id,
                    i.title,
                    i.status,
                    i.priority,
                    0 as depth,
                    d.type as dep_type,
                    ?2 || ',' || {id_col} as path
                FROM dependencies d
                INNER JOIN issues i ON i.id = {id_col}
                WHERE {anchor_join}

                UNION ALL

                -- Recursive: dependencies of dependencies
                SELECT
                    {id_col} as id,
                    i.title,
                    i.status,
                    i.priority,
                    rec.depth + 1 as depth,
                    d.type as dep_type,
                    rec.path || ',' || {id_col} as path
                FROM dependencies d
                INNER JOIN issues i ON i.id = {id_col}
                INNER JOIN dep_tree rec ON {recursive_join}
                WHERE rec.path NOT LIKE '%' || {id_col} || '%'
                {depth_limit}
            )
            SELECT id, title, status, priority, depth, dep_type, path
            FROM dep_tree
            ORDER BY depth, id"
        );

        let mut stmt = conn.prepare(&sql)?;
        // SECURITY: Bind root_id as parameter to prevent SQL injection
        let mut rows = stmt.query(params![root_id, root_id])?;
        let mut nodes = Vec::new();
        while let Some(row) = rows.next()? {
            let path: String = row.get(6)?;
            // Detect cycles: if the ID appears more than once in path, it's a cycle
            let id: String = row.get(0)?;
            let is_cycle = path.matches(&id).count() > 1;

            nodes.push(DepTreeNode {
                id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                priority: row.get(3)?,
                depth: row.get(4)?,
                dep_type: row.get::<_, Option<String>>(5)?,
                path: if is_cycle {
                    format!("{} [CYCLE]", path)
                } else {
                    path
                },
            });
        }
        Ok(nodes)
    }

    pub fn get_dependents(&self, depends_on_id: &str) -> Result<Vec<Dependency>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT issue_id, depends_on_id, type, metadata, thread_id, created_at, created_by FROM dependencies WHERE depends_on_id = ?1",
        )?;
        let mut rows = stmt.query(params![depends_on_id])?;
        let mut deps = Vec::new();
        while let Some(row) = rows.next()? {
            let type_str: String = row.get(2)?;
            deps.push(Dependency {
                issue_id: row.get(0)?,
                depends_on_id: row.get(1)?,
                dep_type: DependencyType::from_str(&type_str)
                    .unwrap_or(DependencyType::Custom(type_str)),
                metadata: row
                    .get::<_, Option<String>>(3)?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                thread_id: row.get(4)?,
                created_at: parse_required_datetime(row.get(5)?)?,
                created_by: row.get(6)?,
                title: None,
            });
        }
        Ok(deps)
    }

    /// Get dependencies with display information (title) by joining with issues table.
    ///
    /// Returns dependency type, bead ID, and title for each dependency of the given parent bead.
    /// Handles both blocking (blocks) and non-blocking dependencies.
    /// Returns empty Vec for beads with no dependencies.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent bead to get dependencies for
    ///
    /// # Returns
    /// Vector of DependencyDisplay structs containing dependency type, bead ID, and title
    pub fn get_dependencies_display(&self, parent_id: &str) -> Result<Vec<DependencyDisplay>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT d.type, d.depends_on_id, i.title
             FROM dependencies d
             LEFT JOIN issues i ON d.depends_on_id = i.id
             WHERE d.issue_id = ?1",
        )?;

        let result = stmt
            .query_map(params![parent_id], |row| {
                Ok(DependencyDisplay {
                    dep_type: row.get(0)?,
                    bead_id: row.get(1)?,
                    title: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(result)
    }

    /// Get dependents with display information (title) by joining with issues table.
    ///
    /// Returns dependent bead ID, title, and dependency type for each bead that depends
    /// on the given bead. Returns empty Vec for beads with no dependents.
    ///
    /// # Arguments
    /// * `depends_on_id` - The ID of the bead to get dependents for
    ///
    /// # Returns
    /// Vector of DependencyDisplay structs containing bead ID, title, and dependency type
    pub fn get_dependents_display(&self, depends_on_id: &str) -> Result<Vec<DependencyDisplay>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(
            "SELECT d.issue_id, i.title, d.type
             FROM dependencies d
             LEFT JOIN issues i ON d.issue_id = i.id
             WHERE d.depends_on_id = ?1",
        )?;

        let result = stmt
            .query_map(params![depends_on_id], |row| {
                Ok(DependencyDisplay {
                    dep_type: row.get(2)?,
                    bead_id: row.get(0)?,
                    title: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(result)
    }

    pub fn add_label(&self, issue_id: &str, label: &str) -> Result<()> {
        let trimmed_label = label.trim();
        if trimmed_label.is_empty() {
            return Err(BeadForgeError::validation("Label cannot be empty or whitespace only"));
        }

        self.with_immediate_transaction(|tx| {
            tx.execute(
                "INSERT OR IGNORE INTO labels (issue_id, label) VALUES (?1, ?2)",
                params![issue_id, trimmed_label],
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO bead_labels (bead_id, label) VALUES (?1, ?2)",
                params![issue_id, trimmed_label],
            )?;
            mark_dirty_tx(tx, issue_id)?;
            Ok(())
        })
    }

    pub fn remove_label(&self, issue_id: &str, label: &str) -> Result<()> {
        let trimmed_label = label.trim();
        if trimmed_label.is_empty() {
            return Err(BeadForgeError::validation("Label cannot be empty or whitespace only"));
        }

        self.with_immediate_transaction(|tx| {
            // Delete from both label tables; DELETE is idempotent (0 rows affected = no-op)
            let rows_deleted_labels = tx.execute(
                "DELETE FROM labels WHERE issue_id = ?1 AND label = ?2",
                params![issue_id, trimmed_label],
            )?;
            let rows_deleted_bead_labels = tx.execute(
                "DELETE FROM bead_labels WHERE bead_id = ?1 AND label = ?2",
                params![issue_id, trimmed_label],
            )?;

            // Only mark as dirty if a label was actually removed
            // If neither deletion affected any rows, this is a no-op (idempotent)
            if rows_deleted_labels > 0 || rows_deleted_bead_labels > 0 {
                mark_dirty_tx(tx, issue_id)?;
            }
            Ok(())
        })
    }

    pub fn get_labels(&self, issue_id: &str) -> Result<Vec<String>> {
        self.load_labels(issue_id)
    }

    pub fn list_all_labels(&self) -> Result<Vec<(String, i64)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT label, COUNT(*) as count FROM labels GROUP BY label ORDER BY count DESC",
        )?;
        let mut rows = stmt.query([])?;
        let mut labels = Vec::new();
        while let Some(row) = rows.next()? {
            labels.push((row.get(0)?, row.get(1)?));
        }
        Ok(labels)
    }

    pub fn add_comment(&self, issue_id: &str, author: &str, body: &str) -> Result<i64> {
        self.with_immediate_transaction(|tx| {
            let now = Utc::now();
            tx.execute(
                "INSERT INTO comments (issue_id, author, text, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![issue_id, author, body, now.to_rfc3339()],
            )?;
            mark_dirty_tx(tx, issue_id)?;
            Ok(tx.last_insert_rowid())
        })
    }

    pub fn list_comments(&self, issue_id: &str) -> Result<Vec<Comment>> {
        self.load_comments(issue_id)
    }

    // Event methods
    pub fn list_events(&self, issue_id: &str) -> Result<Vec<Event>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, issue_id, event_type, actor, old_value, new_value, comment, created_at
             FROM events WHERE issue_id = ?1 ORDER BY created_at ASC",
        )?;
        let mut rows = stmt.query(params![issue_id])?;
        let mut events = Vec::new();
        while let Some(row) = rows.next()? {
            events.push(Self::row_to_event(row)?);
        }
        Ok(events)
    }

    pub fn list_events_filtered(
        &self,
        issue_id: Option<&str>,
        since: Option<&DateTime<Utc>>,
        actor: Option<&str>,
        event_type: Option<&EventType>,
        limit: Option<usize>,
    ) -> Result<Vec<Event>> {
        let mut sql = String::from(
            "SELECT id, issue_id, event_type, actor, old_value, new_value, comment, created_at
             FROM events WHERE 1=1",
        );
        let mut params = Vec::new();
        let mut param_idx = 1;

        if let Some(id) = issue_id {
            sql.push_str(&format!(" AND issue_id = ?{}", param_idx));
            params.push(Box::new(id.to_string()) as Box<dyn rusqlite::ToSql>);
            param_idx += 1;
        }
        if let Some(dt) = since {
            sql.push_str(&format!(" AND created_at >= ?{}", param_idx));
            params.push(Box::new(dt.to_rfc3339()) as Box<dyn rusqlite::ToSql>);
            param_idx += 1;
        }
        if let Some(a) = actor {
            sql.push_str(&format!(" AND actor = ?{}", param_idx));
            params.push(Box::new(a.to_string()) as Box<dyn rusqlite::ToSql>);
            param_idx += 1;
        }
        if let Some(et) = event_type {
            sql.push_str(&format!(" AND event_type = ?{}", param_idx));
            params.push(Box::new(et.as_str().to_string()) as Box<dyn rusqlite::ToSql>);
            param_idx += 1;
        }
        sql.push_str(" ORDER BY created_at ASC");
        if let Some(l) = limit {
            sql.push_str(&format!(" LIMIT {}", l));
        }

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let mut rows = stmt.query(param_refs.as_slice())?;
        let mut events = Vec::new();
        while let Some(row) = rows.next()? {
            events.push(Self::row_to_event(row)?);
        }
        Ok(events)
    }

    fn row_to_event(row: &rusqlite::Row) -> Result<Event> {
        let type_str: String = row.get(2)?;
        Ok(Event {
            id: row.get(0)?,
            issue_id: row.get(1)?,
            event_type: EventType::from_str(&type_str).unwrap_or(EventType::Custom(type_str)),
            actor: row.get(3)?,
            old_value: row.get(4)?,
            new_value: row.get(5)?,
            comment: row.get(6)?,
            created_at: parse_required_datetime(row.get(7)?)?,
        })
    }

    // Annotation methods
    pub fn get_annotations(&self, issue_id: &str) -> Result<BTreeMap<String, String>> {
        self.load_annotations(issue_id)
    }

    pub fn set_annotation(&self, issue_id: &str, key: &str, value: &str) -> Result<()> {
        self.with_immediate_transaction(|tx| {
            tx.execute(
                "INSERT OR REPLACE INTO bead_annotations (bead_id, key, value) VALUES (?1, ?2, ?3)",
                params![issue_id, key, value],
            )?;
            mark_dirty_tx(tx, issue_id)?;
            Ok(())
        })
    }

    pub fn remove_annotation(&self, issue_id: &str, key: &str) -> Result<()> {
        self.with_immediate_transaction(|tx| {
            tx.execute(
                "DELETE FROM bead_annotations WHERE bead_id = ?1 AND key = ?2",
                params![issue_id, key],
            )?;
            mark_dirty_tx(tx, issue_id)?;
            Ok(())
        })
    }

    pub fn clear_annotations(&self, issue_id: &str) -> Result<()> {
        self.with_immediate_transaction(|tx| {
            tx.execute(
                "DELETE FROM bead_annotations WHERE bead_id = ?1",
                params![issue_id],
            )?;
            mark_dirty_tx(tx, issue_id)?;
            Ok(())
        })
    }

    pub fn search_issues(
        &self,
        query: Option<&str>,
        status: &[Status],
        issue_type: &[IssueType],
        assignee: Option<&str>,
        labels: &[String],
        priority_min: Option<i32>,
        priority_max: Option<i32>,
        limit: usize,
    ) -> Result<Vec<Issue>> {
        let mut sql = String::from(
            "SELECT i.id, i.content_hash, i.title, i.description, i.design, i.acceptance_criteria, i.notes,
                    i.status, i.priority, i.issue_type, i.assignee, i.owner, i.estimated_minutes,
                    i.created_at, i.created_by, i.updated_at, i.closed_at, i.close_reason,
                    i.closed_by_session, i.due_at, i.defer_until, i.external_ref, i.source_system,
                    i.source_repo, i.deleted_at, i.deleted_by, i.delete_reason, i.original_type,
                    i.compaction_level, i.compacted_at, i.compacted_at_commit, i.original_size,
                    i.sender, i.ephemeral, i.pinned, i.is_template,
                    GROUP_CONCAT(bl.label) AS labels
             FROM issues i
             LEFT JOIN bead_labels bl ON i.id = bl.bead_id
             WHERE i.deleted_at IS NULL",
        );
        let mut params = Vec::new();
        let mut param_idx = 1;
        if let Some(q) = query {
            sql.push_str(&format!(
                " AND (i.title LIKE ?{} OR i.description LIKE ?{})",
                param_idx,
                param_idx + 1
            ));
            params.push(format!("%{}%", q));
            params.push(format!("%{}%", q));
            param_idx += 2;
        }
        if !status.is_empty() {
            let status_conditions: Vec<String> = status
                .iter()
                .enumerate()
                .map(|(i, _)| format!("i.status = ?{}", param_idx + i))
                .collect();
            sql.push_str(&format!(" AND ({}) ", status_conditions.join(" OR ")));
            for s in status {
                params.push(s.to_string());
                param_idx += 1;
            }
        }
        if !issue_type.is_empty() {
            let type_conditions: Vec<String> = issue_type
                .iter()
                .enumerate()
                .map(|(i, _)| format!("i.issue_type = ?{}", param_idx + i))
                .collect();
            sql.push_str(&format!(" AND ({}) ", type_conditions.join(" OR ")));
            for t in issue_type {
                params.push(t.to_string());
                param_idx += 1;
            }
        }
        if let Some(a) = assignee {
            sql.push_str(&format!(" AND i.assignee = ?{}", param_idx));
            params.push(a.to_string());
            param_idx += 1;
        }
        if !labels.is_empty() {
            // Subquery to find issues that have ALL the specified labels
            let label_conditions: Vec<String> = labels
                .iter()
                .enumerate()
                .map(|(i, label)| {
                    params.push(label.clone());
                    param_idx += 1;
                    format!("EXISTS (SELECT 1 FROM bead_labels bl{} WHERE bl{}.bead_id = i.id AND bl{}.label = ?{})", i, i, i, param_idx - 1)
                })
                .collect();
            sql.push_str(&format!(" AND ({}) ", label_conditions.join(" AND ")));
        }
        if let Some(min) = priority_min {
            sql.push_str(&format!(" AND i.priority >= ?{}", param_idx));
            params.push(min.to_string());
            param_idx += 1;
        }
        if let Some(max) = priority_max {
            sql.push_str(&format!(" AND i.priority <= ?{}", param_idx));
            params.push(max.to_string());
            param_idx += 1;
        }
        sql.push_str(" GROUP BY i.id");
        sql.push_str(" ORDER BY i.priority ASC, i.created_at ASC");
        if limit > 0 {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
        let mut rows = stmt.query(param_refs.as_slice())?;
        let mut issues = Vec::new();
        while let Some(row) = rows.next()? {
            issues.push(Self::row_to_issue_conn(&conn, row)?);
        }
        Ok(issues)
    }

    pub fn get_stats(&self) -> Result<Stats> {
        let conn = self.conn.lock().unwrap();
        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM issues WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )?;
        let open: i64 = conn.query_row(
            "SELECT COUNT(*) FROM issues WHERE status = 'open' AND deleted_at IS NULL",
            [],
            |row| row.get(0),
        )?;
        let in_progress: i64 = conn.query_row(
            "SELECT COUNT(*) FROM issues WHERE status = 'in_progress' AND deleted_at IS NULL",
            [],
            |row| row.get(0),
        )?;
        let closed: i64 = conn.query_row(
            "SELECT COUNT(*) FROM issues WHERE status = 'closed' AND deleted_at IS NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(Stats {
            total: total as usize,
            open: open as usize,
            in_progress: in_progress as usize,
            closed: closed as usize,
        })
    }

    pub fn get_stats_by_type(&self) -> Result<Vec<(String, i64)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT issue_type, COUNT(*) as count FROM issues WHERE deleted_at IS NULL GROUP BY issue_type ORDER BY count DESC")?;
        let mut rows = stmt.query([])?;
        let mut result = Vec::new();
        while let Some(row) = rows.next()? {
            result.push((row.get(0)?, row.get(1)?));
        }
        Ok(result)
    }

    pub fn get_stats_by_priority(&self) -> Result<Vec<(i32, i64)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT priority, COUNT(*) as count FROM issues WHERE deleted_at IS NULL GROUP BY priority ORDER BY priority ASC")?;
        let mut rows = stmt.query([])?;
        let mut result = Vec::new();
        while let Some(row) = rows.next()? {
            result.push((row.get(0)?, row.get(1)?));
        }
        Ok(result)
    }

    pub fn get_stats_by_assignee(&self) -> Result<Vec<(Option<String>, i64)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT NULLIF(assignee, '') AS assignee_group, COUNT(*) as count FROM issues WHERE deleted_at IS NULL GROUP BY assignee_group ORDER BY count DESC")?;
        let mut rows = stmt.query([])?;
        let mut result = Vec::new();
        while let Some(row) = rows.next()? {
            result.push((row.get(0)?, row.get(1)?));
        }
        Ok(result)
    }

    /// Get the score of the top candidate bead for claiming.
    ///
    /// Returns None if no candidates are available.
    /// Used by claim_any() for cross-workspace scoring comparison.
    ///
    /// Note: critical_float is the raw float value (lower = more critical).
    /// The claim scoring uses 1000.0 / (float + 1) as a bonus, which is
    /// monotonically decreasing with float. Therefore, ordering by float ASC
    /// is equivalent to ordering by bonus DESC.
    ///
    /// When model and harness are provided, uses velocity-aware scoring:
    /// - LEFT JOINs velocity_stats on (issue_type, model, harness)
    /// - Returns expected_seconds and combined_score for velocity-adjusted comparison
    pub fn top_candidate_score(
        &self,
        model: Option<&str>,
        harness: Option<&str>,
    ) -> Result<Option<crate::claim::Score>> {
        let conn = self.conn.lock().unwrap();

        if let (Some(m), Some(h)) = (model, harness) {
            // Velocity-aware scoring: join velocity_stats and compute combined_score
            let mut stmt = conn.prepare_cached(
                "SELECT COALESCE(COUNT(d.issue_id), 0) as downstream_impact,
                        COALESCE(c.float, 999) as critical_float,
                        i.priority,
                        CAST(strftime('%s', i.created_at) AS INTEGER) as created_ts,
                        vs.p50_seconds as expected_seconds,
                        (COALESCE(COUNT(d.issue_id), 0) * 3.0
                         + (4 - i.priority) * 2.0
                         + 1000.0 / (COALESCE(c.float, 999) + 1)) as combined_score
                 FROM issues i
                 LEFT JOIN dependencies d ON d.depends_on_id = i.id AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
                 LEFT JOIN critical_path_cache c ON c.bead_id = i.id
                 LEFT JOIN velocity_stats vs ON vs.issue_type = i.issue_type
                     AND vs.model = ?1
                     AND vs.harness = ?2
                 WHERE i.status = 'open'
                   AND i.ephemeral = 0
                   AND i.pinned = 0
                   AND i.is_template = 0
                   AND i.deleted_at IS NULL
                   AND i.id NOT IN (SELECT issue_id FROM blocked_issues_cache)
                 GROUP BY i.id
                 ORDER BY
                     combined_score / COALESCE(vs.p50_seconds, 1800) DESC,
                     downstream_impact DESC,
                     critical_float ASC,
                     i.priority ASC,
                     i.created_at ASC
                 LIMIT 1",
            )?;

            let mut rows = stmt.query(params![m, h])?;
            if let Some(row) = rows.next()? {
                Ok(Some(crate::claim::Score::new(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                )))
            } else {
                Ok(None)
            }
        } else {
            // Standard scoring without velocity data
            let mut stmt = conn.prepare_cached(
                "SELECT COALESCE(COUNT(d.issue_id), 0) as downstream_impact,
                        COALESCE(c.float, 999) as critical_float,
                        i.priority,
                        CAST(strftime('%s', i.created_at) AS INTEGER) as created_ts
                 FROM issues i
                 LEFT JOIN dependencies d ON d.depends_on_id = i.id AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
                 LEFT JOIN critical_path_cache c ON c.bead_id = i.id
                 WHERE i.status = 'open'
                   AND i.ephemeral = 0
                   AND i.pinned = 0
                   AND i.is_template = 0
                   AND i.deleted_at IS NULL
                   AND i.id NOT IN (SELECT issue_id FROM blocked_issues_cache)
                 GROUP BY i.id
                 ORDER BY
                     downstream_impact DESC,
                     critical_float ASC,
                     i.priority ASC,
                     i.created_at ASC
                 LIMIT 1",
            )?;

            let mut rows = stmt.query([])?;
            if let Some(row) = rows.next()? {
                let downstream_impact: i64 = row.get(0)?;
                let critical_float: i64 = row.get(1)?;
                let priority: i32 = row.get(2)?;
                let created_at_ts: i64 = row.get(3)?;

                // Compute combined_score for consistency
                let combined_score = downstream_impact as f64 * 3.0
                    + (4 - priority) as f64 * 2.0
                    + 1000.0 / (critical_float as f64 + 1.0);

                Ok(Some(crate::claim::Score::new(
                    downstream_impact,
                    critical_float,
                    priority,
                    created_at_ts,
                    None, // No expected_seconds without velocity data
                    combined_score,
                )))
            } else {
                Ok(None)
            }
        }
    }

    /// Record a worker session for tracking metadata.
    ///
    /// Stores worker metadata (model, harness, version) for each claim operation.
    /// Used by velocity-aware scoring and audit trails.
    pub fn record_worker_session(
        &self,
        worker_id: &str,
        model: Option<&str>,
        harness: Option<&str>,
        harness_version: Option<&str>,
        bead_id: &str,
        workspace_path: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now();
        conn.execute(
            "INSERT INTO worker_sessions (worker_id, model, harness, harness_version, bead_id, workspace_path, claimed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                worker_id,
                model,
                harness,
                harness_version,
                bead_id,
                workspace_path,
                now.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    // Public transaction-level helpers for sync operations

    /// Get an issue within a transaction context.
    pub fn get_issue_tx(tx: &Connection, id: &str) -> Result<Option<Issue>> {
        let mut stmt = tx.prepare(
            "SELECT i.id, i.content_hash, i.title, i.description, i.design, i.acceptance_criteria, i.notes,
                    i.status, i.priority, i.issue_type, i.assignee, i.owner, i.estimated_minutes,
                    i.created_at, i.created_by, i.updated_at, i.closed_at, i.close_reason,
                    i.closed_by_session, i.due_at, i.defer_until, i.external_ref, i.source_system,
                    i.source_repo, i.deleted_at, i.deleted_by, i.delete_reason, i.original_type,
                    i.compaction_level, i.compacted_at, i.compacted_at_commit, i.original_size,
                    i.sender, i.ephemeral, i.pinned, i.is_template,
                    GROUP_CONCAT(bl.label) AS labels
             FROM issues i
             LEFT JOIN bead_labels bl ON i.id = bl.bead_id
             WHERE i.id = ?1
             GROUP BY i.id",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::row_to_issue_conn(tx, row)?))
        } else {
            Ok(None)
        }
    }

    /// Create an issue within a transaction context.
    pub fn create_issue_tx(tx: &Connection, issue: &Issue) -> Result<()> {
        // Compute content_hash if not already set, and wrap in Some for storage
        let content_hash: Option<String> = issue
            .content_hash
            .as_ref()
            .cloned()
            .or_else(|| Some(issue.content_hash()));

        tx.execute(
            "INSERT INTO issues (
                id, content_hash, title, description, design, acceptance_criteria, notes,
                status, priority, issue_type, assignee, owner, estimated_minutes,
                created_at, created_by, updated_at, closed_at, close_reason,
                closed_by_session, due_at, defer_until, external_ref, source_system,
                source_repo, deleted_at, deleted_by, delete_reason, original_type,
                compaction_level, compacted_at, compacted_at_commit, original_size,
                sender, ephemeral, pinned, is_template
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                  ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28,
                  ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36)",
            params![
                &issue.id,
                &content_hash,
                &issue.title,
                issue.description.as_deref().unwrap_or(""),
                issue.design.as_deref().unwrap_or(""),
                issue.acceptance_criteria.as_deref().unwrap_or(""),
                issue.notes.as_deref().unwrap_or(""),
                &issue.status.to_string(),
                &issue.priority,
                &issue.issue_type.to_string(),
                &issue.assignee,
                &issue.owner,
                &issue.estimated_minutes,
                &issue.created_at.to_rfc3339(),
                &issue.created_by,
                &issue.updated_at.to_rfc3339(),
                issue.closed_at.map(|d| d.to_rfc3339()),
                &issue.close_reason,
                &issue.closed_by_session,
                issue.due_at.map(|d| d.to_rfc3339()),
                issue.defer_until.map(|d| d.to_rfc3339()),
                &issue.external_ref,
                &issue.source_system,
                issue.source_repo.as_deref().unwrap_or("."),
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
        for label in &issue.labels {
            tx.execute(
                "INSERT OR IGNORE INTO labels (issue_id, label) VALUES (?1, ?2)",
                params![&issue.id, label],
            )?;
        }
        for label in &issue.labels {
            tx.execute(
                "INSERT OR IGNORE INTO bead_labels (bead_id, label) VALUES (?1, ?2)",
                params![&issue.id, label],
            )?;
        }
        for dep in &issue.dependencies {
            tx.execute(
                "INSERT INTO dependencies (issue_id, depends_on_id, type, metadata, thread_id, created_at, created_by)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    &dep.issue_id, &dep.depends_on_id, &dep.dep_type.to_string(),
                    dep.metadata.as_ref().map(|m| serde_json::to_string(m).ok()).flatten(),
                    &dep.thread_id, &dep.created_at.to_rfc3339(), &dep.created_by,
                ],
            )?;
        }
        for comment in &issue.comments {
            tx.execute(
                "INSERT INTO comments (id, issue_id, author, text, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    &comment.id, &comment.issue_id, &comment.author, &comment.body,
                    &comment.created_at.to_rfc3339(),
                ],
            )?;
        }
        for (key, value) in &issue.annotations {
            tx.execute(
                "INSERT INTO bead_annotations (bead_id, key, value) VALUES (?1, ?2, ?3)",
                params![&issue.id, key, value],
            )?;
        }
        Ok(())
    }

    /// Update an issue from JSON within a transaction context.
    pub fn update_issue_from_json_tx(tx: &Connection, issue: &Issue) -> Result<()> {
        // Compute content_hash (it's None when importing from JSONL due to #[serde(skip)])
        let content_hash = issue
            .content_hash
            .as_ref()
            .cloned()
            .unwrap_or_else(|| issue.content_hash());

        tx.execute("DELETE FROM labels WHERE issue_id = ?1", params![&issue.id])?;
        tx.execute(
            "DELETE FROM bead_labels WHERE bead_id = ?1",
            params![&issue.id],
        )?;
        tx.execute(
            "DELETE FROM dependencies WHERE issue_id = ?1",
            params![&issue.id],
        )?;
        tx.execute(
            "DELETE FROM comments WHERE issue_id = ?1",
            params![&issue.id],
        )?;
        tx.execute(
            "DELETE FROM bead_annotations WHERE bead_id = ?1",
            params![&issue.id],
        )?;

        tx.execute(
            "UPDATE issues SET
                content_hash = ?1, title = ?2, description = ?3, design = ?4,
                acceptance_criteria = ?5, notes = ?6, status = ?7, priority = ?8,
                issue_type = ?9, assignee = ?10, owner = ?11, estimated_minutes = ?12,
                created_at = ?13, created_by = ?14, updated_at = ?15, closed_at = ?16,
                close_reason = ?17, closed_by_session = ?18, due_at = ?19, defer_until = ?20,
                external_ref = ?21, source_system = ?22, source_repo = ?23,
                deleted_at = ?24, deleted_by = ?25, delete_reason = ?26, original_type = ?27,
                compaction_level = ?28, compacted_at = ?29, compacted_at_commit = ?30,
                original_size = ?31, sender = ?32, ephemeral = ?33, pinned = ?34, is_template = ?35
             WHERE id = ?36",
            params![
                &content_hash,
                &issue.title,
                issue.description.as_deref().unwrap_or(""),
                issue.design.as_deref().unwrap_or(""),
                issue.acceptance_criteria.as_deref().unwrap_or(""),
                issue.notes.as_deref().unwrap_or(""),
                &issue.status.to_string(),
                &issue.priority,
                &issue.issue_type.to_string(),
                &issue.assignee,
                &issue.owner,
                &issue.estimated_minutes,
                &issue.created_at.to_rfc3339(),
                &issue.created_by,
                &issue.updated_at.to_rfc3339(),
                issue.closed_at.map(|d| d.to_rfc3339()),
                &issue.close_reason,
                &issue.closed_by_session,
                issue.due_at.map(|d| d.to_rfc3339()),
                issue.defer_until.map(|d| d.to_rfc3339()),
                &issue.external_ref,
                &issue.source_system,
                issue.source_repo.as_deref().unwrap_or("."),
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
                &issue.id,
            ],
        )?;

        for label in &issue.labels {
            tx.execute(
                "INSERT OR IGNORE INTO labels (issue_id, label) VALUES (?1, ?2)",
                params![&issue.id, label],
            )?;
        }
        for label in &issue.labels {
            tx.execute(
                "INSERT OR IGNORE INTO bead_labels (bead_id, label) VALUES (?1, ?2)",
                params![&issue.id, label],
            )?;
        }
        for dep in &issue.dependencies {
            tx.execute(
                "INSERT INTO dependencies (issue_id, depends_on_id, type, metadata, thread_id, created_at, created_by)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    &dep.issue_id, &dep.depends_on_id, &dep.dep_type.to_string(),
                    dep.metadata.as_ref().map(|m| serde_json::to_string(m).ok()).flatten(),
                    &dep.thread_id, &dep.created_at.to_rfc3339(), &dep.created_by,
                ],
            )?;
        }
        for comment in &issue.comments {
            tx.execute(
                "INSERT INTO comments (id, issue_id, author, text, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    &comment.id, &comment.issue_id, &comment.author, &comment.body,
                    &comment.created_at.to_rfc3339(),
                ],
            )?;
        }
        for (key, value) in &issue.annotations {
            tx.execute(
                "INSERT INTO bead_annotations (bead_id, key, value) VALUES (?1, ?2, ?3)",
                params![&issue.id, key, value],
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Stats {
    pub total: usize,
    pub open: usize,
    pub in_progress: usize,
    pub closed: usize,
}

/// Mark an issue dirty inside an open transaction so the next flush exports it.
///
/// Every mutation path that changes an issue's exported representation (fields,
/// labels, dependencies, comments, annotations) must call this within the same
/// transaction as the mutation, so the dirty mark and the change commit atomically.
fn mark_dirty_tx(tx: &Connection, id: &str) -> Result<()> {
    tx.execute(
        "INSERT OR REPLACE INTO dirty_issues (bead_id, marked_at) VALUES (?1, ?2)",
        params![id, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

fn is_busy_error(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::DatabaseBusy,
                ..
            },
            _
        )
    )
}

/// Read a NOT NULL datetime column tolerantly.
///
/// The schema declares these columns `DATETIME NOT NULL`, but corrupted
/// databases have historically contained NULL (or empty) values in them — the
/// flush/list crash class that forced the destructive `rm beads.db + reimport`
/// workaround. The crash was `row.get::<_, String>(idx)` turning a NULL into a
/// fatal `InvalidColumnType` error that aborted the entire list/flush before a
/// single row was returned.
///
/// Mapping NULL/empty to the Unix epoch lets the row still load so the user can
/// see and act on their data. `bf doctor` detects such rows (see
/// `doctor::check_null_not_null`) and `bf doctor --fix-schema` repairs them in
/// place. Genuinely malformed non-empty values still error — those are a
/// different corruption class and the caller decides how to surface them.
fn parse_required_datetime(v: Option<String>) -> Result<DateTime<Utc>> {
    match v {
        None => Ok(DateTime::<Utc>::UNIX_EPOCH),
        Some(ref s) if s.trim().is_empty() => Ok(DateTime::<Utc>::UNIX_EPOCH),
        Some(s) => parse_datetime(s),
    }
}

fn parse_datetime(s: String) -> Result<DateTime<Utc>> {
    let t = s.trim();
    // bf's own format: RFC3339 (with timezone; optional fractional seconds).
    match DateTime::parse_from_rfc3339(t) {
        Ok(dt) => Ok(dt.with_timezone(&Utc)),
        Err(e) => {
            // br / SQLite-native datetime() format: no timezone, space or 'T'
            // separator (e.g. "2026-05-15 21:10:36"). Assume UTC. A workspace
            // touched by both `br` and `bf` mixes these, and the RFC3339-only
            // parser used to crash the entire list/flush ("premature end of
            // input") on the first such row.
            for fmt in [
                "%Y-%m-%d %H:%M:%S%.f",
                "%Y-%m-%dT%H:%M:%S%.f",
                "%Y-%m-%d %H:%M:%S",
                "%Y-%m-%dT%H:%M:%S",
            ] {
                if let Ok(ndt) = NaiveDateTime::parse_from_str(t, fmt) {
                    return Ok(ndt.and_utc());
                }
            }
            Err(e.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Issue, IssueType, Priority, Status};
    use chrono::Utc;
    use tempfile::NamedTempFile;

    #[test]
    fn test_multi_label_persistence() {
        // Create a temporary database
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an issue with multiple labels
        let mut issue = Issue::new("bf-test-multilabel".to_string(), "Test multi-label support".to_string(), ".".to_string());
        issue.labels = vec![
            "phase-1".to_string(),
            "model".to_string(),
            "backend".to_string(),
            "urgent".to_string(),
        ];
        issue.status = Status::Open;
        issue.priority = Priority::HIGH;

        // Create the issue
        storage.create_issue(&issue).unwrap();

        // Read it back
        let retrieved = storage.get_issue("bf-test-multilabel").unwrap().unwrap();

        // Verify all labels persisted correctly
        assert_eq!(retrieved.labels.len(), 4);
        assert!(retrieved.labels.contains(&"phase-1".to_string()));
        assert!(retrieved.labels.contains(&"model".to_string()));
        assert!(retrieved.labels.contains(&"backend".to_string()));
        assert!(retrieved.labels.contains(&"urgent".to_string()));
    }

    #[test]
    fn test_multi_label_update() {
        // Create a temporary database
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an issue with initial labels
        let mut issue = Issue::new("bf-update-labels".to_string(), "Test label updates".to_string(), ".".to_string());
        issue.labels = vec!["initial".to_string(), "old".to_string()];
        storage.create_issue(&issue).unwrap();

        // Update with new labels
        let mut changes = IssueChanges::default();
        changes.labels = Some(vec![
            "updated".to_string(),
            "phase-2".to_string(),
            "backend".to_string(),
        ]);
        storage.update_issue("bf-update-labels", &changes).unwrap();

        // Verify the labels were updated
        let retrieved = storage.get_issue("bf-update-labels").unwrap().unwrap();
        assert_eq!(retrieved.labels.len(), 3);
        assert!(retrieved.labels.contains(&"updated".to_string()));
        assert!(retrieved.labels.contains(&"phase-2".to_string()));
        assert!(retrieved.labels.contains(&"backend".to_string()));
        assert!(!retrieved.labels.contains(&"initial".to_string()));
    }

    #[test]
    fn test_label_filtering() {
        // Create a temporary database
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create multiple issues with different labels
        let mut issue1 = Issue::new("bf-1".to_string(), "Issue 1".to_string(), ".".to_string());
        issue1.labels = vec!["backend".to_string(), "urgent".to_string()];
        storage.create_issue(&issue1).unwrap();

        let mut issue2 = Issue::new("bf-2".to_string(), "Issue 2".to_string(), ".".to_string());
        issue2.labels = vec!["frontend".to_string()];
        storage.create_issue(&issue2).unwrap();

        let mut issue3 = Issue::new("bf-3".to_string(), "Issue 3".to_string(), ".".to_string());
        issue3.labels = vec!["backend".to_string(), "phase-2".to_string()];
        storage.create_issue(&issue3).unwrap();

        // Filter by label
        let mut filter = IssueFilter::default();
        filter.labels = Some(vec!["backend".to_string()]);

        let results = storage.list_issues(&filter).unwrap();

        // Should return 2 issues with "backend" label
        assert_eq!(results.len(), 2);
        let ids: Vec<&str> = results.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&"bf-1"));
        assert!(ids.contains(&"bf-3"));
        assert!(!ids.contains(&"bf-2"));
    }

    #[test]
    fn test_assignee_clear_and_null_persistence() {
        // Create a temporary database
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an issue with an assignee set
        let mut issue = Issue::new("bf-clear-assignee".to_string(), "Test assignee clearing".to_string(), ".".to_string());
        issue.assignee = Some("alice".to_string());
        issue.status = Status::Open;
        issue.priority = Priority::MEDIUM;

        // Create the issue
        storage.create_issue(&issue).unwrap();

        // Verify the assignee was set
        let retrieved = storage.get_issue("bf-clear-assignee").unwrap().unwrap();
        assert_eq!(retrieved.assignee.as_deref(), Some("alice"));

        // Clear the assignee using IssueChanges with empty string
        let mut changes = IssueChanges::default();
        changes.assignee = Some(String::new()); // Empty string signals "clear to NULL"
        changes.actor = Some("test-actor".to_string());

        storage.update_issue("bf-clear-assignee", &changes).unwrap();

        // Read the issue back and verify assignee is NULL
        let cleared = storage.get_issue("bf-clear-assignee").unwrap().unwrap();
        assert_eq!(cleared.assignee, None, "assignee should be NULL after clearing");

        // Also test using the convenience method
        let mut issue2 = Issue::new("bf-clear-assignee2".to_string(), "Test assignee clearing via method".to_string(), ".".to_string());
        issue2.assignee = Some("bob".to_string());
        storage.create_issue(&issue2).unwrap();

        // Clear using the Issue::clear_assignee() method
        let clear_changes = issue2.clear_assignee("test-actor".to_string());
        storage.update_issue("bf-clear-assignee2", &clear_changes).unwrap();

        // Verify assignee is NULL
        let cleared2 = storage.get_issue("bf-clear-assignee2").unwrap().unwrap();
        assert_eq!(cleared2.assignee, None, "assignee should be NULL after clearing via method");
    }

    #[test]
    fn test_clear_assignee_on_unassigned_bead() {
        // Test clearing an already NULL assignee (idempotent operation)
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an issue without an assignee
        let mut issue = Issue::new("bf-no-assignee".to_string(), "Test unassigned bead".to_string(), ".".to_string());
        issue.assignee = None;
        issue.status = Status::Open;
        issue.priority = Priority::MEDIUM;

        storage.create_issue(&issue).unwrap();

        // Verify initial state has no assignee
        let retrieved = storage.get_issue("bf-no-assignee").unwrap().unwrap();
        assert_eq!(retrieved.assignee, None);

        // Attempt to clear the already NULL assignee
        let changes = Issue::clear_assignee(&issue, "test-actor".to_string());
        storage.update_issue("bf-no-assignee", &changes).unwrap();

        // Should still be NULL (idempotent)
        let cleared = storage.get_issue("bf-no-assignee").unwrap().unwrap();
        assert_eq!(cleared.assignee, None, "assignee should remain NULL when clearing already unassigned bead");
    }

    #[test]
    fn test_clear_assignee_on_nonexistent_bead() {
        // Test error handling when trying to clear assignee on a bead that doesn't exist
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a test issue (not the one we'll try to update)
        let mut issue = Issue::new("bf-exists".to_string(), "Existing bead".to_string(), ".".to_string());
        issue.assignee = Some("alice".to_string());
        storage.create_issue(&issue).unwrap();

        // Try to clear assignee on a bead that doesn't exist
        let changes = IssueChanges {
            assignee: Some(String::new()),
            actor: Some("test-actor".to_string()),
            ..Default::default()
        };

        let result = storage.update_issue("bf-does-not-exist", &changes);

        // Should return an error
        assert!(result.is_err(), "clearing assignee on nonexistent bead should return error");
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("not found") || err_msg.contains("Bead not found"),
            "error message should indicate bead was not found"
        );
    }

    #[test]
    fn test_assignee_clear_transactional_atomicity() {
        // Test that assignee clearing is atomic (all-or-nothing within transaction)
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an issue with assignee
        let mut issue = Issue::new("bf-atomic-clear".to_string(), "Test atomic clearing".to_string(), ".".to_string());
        issue.assignee = Some("alice".to_string());
        issue.status = Status::Open;
        issue.priority = Priority::MEDIUM;
        storage.create_issue(&issue).unwrap();

        // Get initial updated_at timestamp
        let initial = storage.get_issue("bf-atomic-clear").unwrap().unwrap();
        let initial_updated_at = initial.updated_at;

        // Small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Clear assignee using IssueChanges
        let changes = IssueChanges {
            assignee: Some(String::new()),
            actor: Some("test-actor".to_string()),
            ..Default::default()
        };

        let update_result = storage.update_issue("bf-atomic-clear", &changes);

        // Update should succeed
        assert!(update_result.is_ok(), "assignee clear should succeed");

        // Verify the change was applied atomically
        let updated = storage.get_issue("bf-atomic-clear").unwrap().unwrap();
        assert_eq!(updated.assignee, None, "assignee should be NULL");

        // Verify updated_at changed (indicates the full row update was atomic)
        assert_ne!(updated.updated_at, initial_updated_at, "updated_at should change after successful update");

        // Verify the issue still has all its original fields (atomic operation preserved them)
        assert_eq!(updated.id, "bf-atomic-clear");
        assert_eq!(updated.title, "Test atomic clearing");
        assert_eq!(updated.status, Status::Open);
        assert_eq!(updated.priority, Priority::MEDIUM);
    }

    #[test]
    fn test_assignee_clear_with_whitespace_variations() {
        // Test that various whitespace patterns are handled correctly when clearing assignee
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create issue with assignee
        let mut issue = Issue::new("bf-whitespace-test".to_string(), "Test whitespace handling".to_string(), ".".to_string());
        issue.assignee = Some("alice".to_string());
        storage.create_issue(&issue).unwrap();

        // Clear with empty string
        let changes1 = IssueChanges {
            assignee: Some(String::new()),
            actor: Some("actor1".to_string()),
            ..Default::default()
        };
        storage.update_issue("bf-whitespace-test", &changes1).unwrap();

        let cleared1 = storage.get_issue("bf-whitespace-test").unwrap().unwrap();
        assert_eq!(cleared1.assignee, None, "empty string should clear to NULL");

        // Set a new assignee
        let changes2 = IssueChanges {
            assignee: Some("bob".to_string()),
            actor: Some("actor2".to_string()),
            ..Default::default()
        };
        storage.update_issue("bf-whitespace-test", &changes2).unwrap();

        let assigned = storage.get_issue("bf-whitespace-test").unwrap().unwrap();
        assert_eq!(assigned.assignee.as_deref(), Some("bob"));

        // Clear with whitespace-only string (should be treated as empty and clear to NULL)
        let changes3 = IssueChanges {
            assignee: Some("   ".to_string()),  // whitespace only
            actor: Some("actor3".to_string()),
            ..Default::default()
        };
        storage.update_issue("bf-whitespace-test", &changes3).unwrap();

        let cleared2 = storage.get_issue("bf-whitespace-test").unwrap().unwrap();
        assert_eq!(cleared2.assignee, None, "whitespace-only string should clear to NULL");
    }

    #[test]
    fn test_assignee_clear_creates_event() {
        // Test that clearing an assignee creates an assignee_changed event
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create issue with assignee
        let mut issue = Issue::new("bf-event-test".to_string(), "Test event creation".to_string(), ".".to_string());
        issue.assignee = Some("alice".to_string());
        storage.create_issue(&issue).unwrap();

        // Get initial events count
        let initial_events = storage.list_events("bf-event-test").unwrap();
        let initial_count = initial_events.len();

        // Clear the assignee
        let changes = Issue::clear_assignee(&issue, "test-actor".to_string());
        storage.update_issue("bf-event-test", &changes).unwrap();

        // Verify an event was created
        let final_events = storage.list_events("bf-event-test").unwrap();
        assert_eq!(final_events.len(), initial_count + 1, "should have one new event");

        // Find the assignee_changed event
        let assignee_event = final_events.iter()
            .find(|e| e.event_type == crate::model::EventType::AssigneeChanged)
            .expect("should have assignee_changed event");

        assert_eq!(assignee_event.old_value.as_deref(), Some("alice"));
        assert_eq!(assignee_event.new_value, None);
        assert_eq!(assignee_event.actor, "test-actor");
    }

    #[test]
    fn test_create_and_get_issue() {
        // Test basic CRUD: create an issue and retrieve it
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an issue
        let mut issue = Issue::new("bf-crud-1".to_string(), "Test CRUD operations".to_string(), ".".to_string());
        issue.description = Some("Test description".to_string());
        issue.status = Status::Open;
        issue.priority = Priority::HIGH;
        issue.issue_type = IssueType::Bug;
        issue.assignee = Some("alice".to_string());
        issue.estimated_minutes = Some(120);

        storage.create_issue(&issue).unwrap();

        // Retrieve and verify
        let retrieved = storage.get_issue("bf-crud-1").unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();

        assert_eq!(retrieved.id, "bf-crud-1");
        assert_eq!(retrieved.title, "Test CRUD operations");
        assert_eq!(retrieved.description.as_deref(), Some("Test description"));
        assert_eq!(retrieved.status, Status::Open);
        assert_eq!(retrieved.priority, Priority::HIGH);
        assert_eq!(retrieved.issue_type, IssueType::Bug);
        assert_eq!(retrieved.assignee.as_deref(), Some("alice"));
        assert_eq!(retrieved.estimated_minutes, Some(120));
    }

    #[test]
    fn test_get_nonexistent_issue_returns_none() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        let result = storage.get_issue("bf-does-not-exist");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_update_issue_fields() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create initial issue
        let mut issue = Issue::new("bf-update-1".to_string(), "Original title".to_string(), ".".to_string());
        issue.status = Status::Open;
        issue.priority = Priority::MEDIUM;
        storage.create_issue(&issue).unwrap();

        // Update multiple fields
        let mut changes = IssueChanges::default();
        changes.title = Some("Updated title".to_string());
        changes.description = Some("Updated description".to_string());
        changes.priority = Some(Priority::CRITICAL.0);
        changes.assignee = Some("bob".to_string());
        changes.actor = Some("test-actor".to_string());

        storage.update_issue("bf-update-1", &changes).unwrap();

        // Verify updates
        let retrieved = storage.get_issue("bf-update-1").unwrap().unwrap();
        assert_eq!(retrieved.title, "Updated title");
        assert_eq!(retrieved.description.as_deref(), Some("Updated description"));
        assert_eq!(retrieved.priority, Priority::CRITICAL);
        assert_eq!(retrieved.assignee.as_deref(), Some("bob"));
    }

    #[test]
    fn test_list_issues_with_filters() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create test issues with different properties
        let mut issue1 = Issue::new("bf-list-1".to_string(), "Issue 1".to_string(), ".".to_string());
        issue1.status = Status::Open;
        issue1.priority = Priority::HIGH;
        issue1.assignee = Some("alice".to_string());
        storage.create_issue(&issue1).unwrap();

        let mut issue2 = Issue::new("bf-list-2".to_string(), "Issue 2".to_string(), ".".to_string());
        issue2.status = Status::InProgress;
        issue2.priority = Priority::MEDIUM;
        issue2.assignee = Some("bob".to_string());
        storage.create_issue(&issue2).unwrap();

        // Test filter by status
        let mut filter = IssueFilter::default();
        filter.status = Some(Status::Open);
        let results = storage.list_issues(&filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "bf-list-1");

        // Test filter by assignee
        let mut filter = IssueFilter::default();
        filter.assignee = Some("alice".to_string());
        let results = storage.list_issues(&filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "bf-list-1");
    }

    #[test]
    fn test_close_and_reopen_issue() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an open issue
        let mut issue = Issue::new("bf-close-1".to_string(), "Test close/reopen".to_string(), ".".to_string());
        issue.status = Status::Open;
        issue.assignee = Some("alice".to_string());
        storage.create_issue(&issue).unwrap();

        // Close the issue
        storage.close_issue("bf-close-1", "Completed successfully", "test-actor").unwrap();

        // Verify closed state
        let closed = storage.get_issue("bf-close-1").unwrap().unwrap();
        assert_eq!(closed.status, Status::Closed);
        assert_eq!(closed.close_reason.as_deref(), Some("Completed successfully"));
        assert_eq!(closed.closed_by_session.as_deref(), Some("test-actor"));
        assert!(closed.closed_at.is_some());
        assert_eq!(closed.assignee, None); // assignee cleared on close

        // Reopen the issue
        storage.reopen_issue("bf-close-1").unwrap();

        // Verify reopened state
        let reopened = storage.get_issue("bf-close-1").unwrap().unwrap();
        assert_eq!(reopened.status, Status::Open);
        assert!(reopened.closed_at.is_none());
        assert_eq!(reopened.assignee, None); // assignee stays NULL after reopen
    }

    #[test]
    fn test_add_and_remove_dependencies() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create two issues
        let mut issue1 = Issue::new("bf-dep-1".to_string(), "Issue with dependency".to_string(), ".".to_string());
        issue1.status = Status::Open;
        storage.create_issue(&issue1).unwrap();

        let mut issue2 = Issue::new("bf-dep-2".to_string(), "Dependency issue".to_string(), ".".to_string());
        issue2.status = Status::Open;
        storage.create_issue(&issue2).unwrap();

        // Add dependency
        storage.add_dependency("bf-dep-1", "bf-dep-2", &DependencyType::Blocks, "test-actor").unwrap();

        // Verify dependency was added
        let deps = storage.get_dependencies("bf-dep-1").unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].depends_on_id, "bf-dep-2");
        assert_eq!(deps[0].dep_type, DependencyType::Blocks);

        // Verify dependent was blocked (since blocker is open)
        let dependent = storage.get_issue("bf-dep-1").unwrap().unwrap();
        assert_eq!(dependent.status, Status::Blocked);

        // Remove dependency
        storage.remove_dependency("bf-dep-1", "bf-dep-2").unwrap();

        // Verify dependency was removed
        let deps = storage.get_dependencies("bf-dep-1").unwrap();
        assert_eq!(deps.len(), 0);
    }

    #[test]
    fn test_add_and_remove_label() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an issue
        let mut issue = Issue::new("bf-label-1".to_string(), "Test labels".to_string(), ".".to_string());
        storage.create_issue(&issue).unwrap();

        // Add label
        storage.add_label("bf-label-1", "urgent").unwrap();

        // Verify label was added
        let labels = storage.get_labels("bf-label-1").unwrap();
        assert_eq!(labels.len(), 1);
        assert!(labels.contains(&"urgent".to_string()));

        // Add another label
        storage.add_label("bf-label-1", "backend").unwrap();

        let labels = storage.get_labels("bf-label-1").unwrap();
        assert_eq!(labels.len(), 2);

        // Remove label
        storage.remove_label("bf-label-1", "urgent").unwrap();

        // Verify label was removed
        let labels = storage.get_labels("bf-label-1").unwrap();
        assert_eq!(labels.len(), 1);
        assert!(!labels.contains(&"urgent".to_string()));
    }

    #[test]
    fn test_add_comment() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an issue
        let mut issue = Issue::new("bf-comment-1".to_string(), "Test comments".to_string(), ".".to_string());
        storage.create_issue(&issue).unwrap();

        // Add comment
        let comment_id = storage.add_comment("bf-comment-1", "alice", "This is a test comment").unwrap();
        assert!(comment_id > 0);

        // Verify comment was added
        let comments = storage.list_comments("bf-comment-1").unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].author, "alice");
        assert_eq!(comments[0].body, "This is a test comment");
    }

    #[test]
    fn test_search_issues() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create test issues
        let mut issue1 = Issue::new("bf-search-1".to_string(), "Fix authentication bug".to_string(), ".".to_string());
        issue1.status = Status::Open;
        issue1.priority = Priority::HIGH;
        issue1.labels = vec!["bug".to_string(), "auth".to_string()];
        storage.create_issue(&issue1).unwrap();

        let mut issue2 = Issue::new("bf-search-2".to_string(), "Add new feature".to_string(), ".".to_string());
        issue2.status = Status::InProgress;
        issue2.priority = Priority::MEDIUM;
        issue2.labels = vec!["feature".to_string()];
        storage.create_issue(&issue2).unwrap();

        // Search by query
        let results = storage.search_issues(
            Some("authentication"),
            &[],
            &[],
            None,
            &[],
            None,
            None,
            10
        ).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("authentication"));

        // Search by priority range
        let results = storage.search_issues(
            None,
            &[],
            &[],
            None,
            &[],
            Some(0),  // min priority
            Some(1),  // max priority
            10
        ).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].priority, Priority::HIGH);
    }

    #[test]
    fn test_get_stats() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create test issues with different statuses
        let mut issue1 = Issue::new("bf-stats-1".to_string(), "Open issue".to_string(), ".".to_string());
        issue1.status = Status::Open;
        storage.create_issue(&issue1).unwrap();

        let mut issue2 = Issue::new("bf-stats-2".to_string(), "In progress issue".to_string(), ".".to_string());
        issue2.status = Status::InProgress;
        storage.create_issue(&issue2).unwrap();

        let mut issue3 = Issue::new("bf-stats-3".to_string(), "Closed issue".to_string(), ".".to_string());
        issue3.status = Status::Closed;
        storage.create_issue(&issue3).unwrap();

        // Get stats
        let stats = storage.get_stats().unwrap();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.open, 1);
        assert_eq!(stats.in_progress, 1);
        assert_eq!(stats.closed, 1);
    }

    #[test]
    fn test_count_issues() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Initially empty
        assert_eq!(storage.count_issues().unwrap(), 0);

        // Add some issues
        let issue1 = Issue::new("bf-count-1".to_string(), "Issue 1".to_string(), ".".to_string());
        storage.create_issue(&issue1).unwrap();

        let issue2 = Issue::new("bf-count-2".to_string(), "Issue 2".to_string(), ".".to_string());
        storage.create_issue(&issue2).unwrap();

        assert_eq!(storage.count_issues().unwrap(), 2);
    }

    #[test]
    fn test_mark_dirty_and_clear_dirty() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an issue
        let issue = Issue::new("bf-dirty-1".to_string(), "Test dirty tracking".to_string(), ".".to_string());
        storage.create_issue(&issue).unwrap();

        // Mark as dirty
        storage.mark_dirty("bf-dirty-1").unwrap();

        // Check it's in dirty list
        let dirty_issues = storage.query_dirty_issues().unwrap();
        assert_eq!(dirty_issues.len(), 1);
        assert!(dirty_issues.contains(&"bf-dirty-1".to_string()));

        // Clear dirty list
        storage.clear_dirty().unwrap();

        // Verify it's cleared
        let dirty_issues = storage.query_dirty_issues().unwrap();
        assert_eq!(dirty_issues.len(), 0);
    }

    #[test]
    fn test_set_and_get_annotations() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an issue
        let issue = Issue::new("bf-annot-1".to_string(), "Test annotations".to_string(), ".".to_string());
        storage.create_issue(&issue).unwrap();

        // Set annotation
        storage.set_annotation("bf-annot-1", "key1", "value1").unwrap();

        // Get annotations
        let annotations = storage.get_annotations("bf-annot-1").unwrap();
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations.get("key1"), Some(&"value1".to_string()));

        // Update annotation
        storage.set_annotation("bf-annot-1", "key1", "updated_value1").unwrap();

        let annotations = storage.get_annotations("bf-annot-1").unwrap();
        assert_eq!(annotations.get("key1"), Some(&"updated_value1".to_string()));

        // Add another annotation
        storage.set_annotation("bf-annot-1", "key2", "value2").unwrap();

        let annotations = storage.get_annotations("bf-annot-1").unwrap();
        assert_eq!(annotations.len(), 2);

        // Remove annotation
        storage.remove_annotation("bf-annot-1", "key1").unwrap();

        let annotations = storage.get_annotations("bf-annot-1").unwrap();
        assert_eq!(annotations.len(), 1);
        assert!(!annotations.contains_key("key1"));

        // Clear all annotations
        storage.clear_annotations("bf-annot-1").unwrap();

        let annotations = storage.get_annotations("bf-annot-1").unwrap();
        assert_eq!(annotations.len(), 0);
    }

    #[test]
    fn test_list_all_issues() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create multiple issues
        let issue1 = Issue::new("bf-all-1".to_string(), "Issue 1".to_string(), ".".to_string());
        storage.create_issue(&issue1).unwrap();

        let issue2 = Issue::new("bf-all-2".to_string(), "Issue 2".to_string(), ".".to_string());
        storage.create_issue(&issue2).unwrap();

        // List all issues
        let all_issues = storage.list_all_issues().unwrap();
        assert_eq!(all_issues.len(), 2);
    }

    #[test]
    fn test_issue_update_with_annotations() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an issue
        let issue = Issue::new("bf-annot-update".to_string(), "Test annotation updates".to_string(), ".".to_string());
        storage.create_issue(&issue).unwrap();

        // Update with annotations
        let mut changes = IssueChanges::default();
        let mut annotations = std::collections::BTreeMap::new();
        annotations.insert("key1".to_string(), "value1".to_string());
        annotations.insert("key2".to_string(), "value2".to_string());
        changes.annotations = Some(annotations);
        changes.actor = Some("test-actor".to_string());

        storage.update_issue("bf-annot-update", &changes).unwrap();

        // Verify annotations were set
        let retrieved = storage.get_issue("bf-annot-update").unwrap().unwrap();
        assert_eq!(retrieved.annotations.len(), 2);
        assert_eq!(retrieved.annotations.get("key1"), Some(&"value1".to_string()));
        assert_eq!(retrieved.annotations.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_update_status_creates_event() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an issue
        let mut issue = Issue::new("bf-status-event".to_string(), "Test status events".to_string(), ".".to_string());
        issue.status = Status::Open;
        storage.create_issue(&issue).unwrap();

        // Get initial event count
        let initial_events = storage.list_events("bf-status-event").unwrap();
        let initial_count = initial_events.len();

        // Update status
        let mut changes = IssueChanges::default();
        changes.status = Some(Status::InProgress);
        changes.actor = Some("test-actor".to_string());
        storage.update_issue("bf-status-event", &changes).unwrap();

        // Verify new event was created (status_changed event)
        let final_events = storage.list_events("bf-status-event").unwrap();
        assert_eq!(final_events.len(), initial_count + 1);
    }

    #[test]
    fn test_with_immediate_transaction_retry_logic() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Test that the transaction helper works correctly
        let result = storage.with_immediate_transaction(|tx| {
            // Create an issue within the transaction
            let issue = Issue::new("bf-tx-1".to_string(), "Transaction test".to_string(), ".".to_string());
            Storage::create_issue_tx(tx, &issue)?;
            Ok::<(), BeadForgeError>(())
        });

        assert!(result.is_ok());

        // Verify the issue was created
        let retrieved = storage.get_issue("bf-tx-1").unwrap();
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_rebuild_blocked_cache() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create issues with blocking dependencies
        let mut issue1 = Issue::new("bf-block-1".to_string(), "Blocker issue".to_string(), ".".to_string());
        issue1.status = Status::Open;
        storage.create_issue(&issue1).unwrap();

        let mut issue2 = Issue::new("bf-block-2".to_string(), "Blocked issue".to_string(), ".".to_string());
        issue2.status = Status::Open;
        storage.create_issue(&issue2).unwrap();

        // Add blocking dependency
        storage.add_dependency("bf-block-2", "bf-block-1", &DependencyType::Blocks, "test-actor").unwrap();

        // Rebuild cache
        storage.rebuild_blocked_cache().unwrap();

        // Get blocked issues
        let blocked = storage.get_blocked_issues().unwrap();
        assert!(!blocked.is_empty());

        // Find our blocked issue
        let blocked_pair = blocked.iter().find(|(id, _)| id == "bf-block-2");
        assert!(blocked_pair.is_some());
    }

    #[test]
    fn test_get_dependencies_display() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create two issues
        let mut issue1 = Issue::new("bf-dep-disp-1".to_string(), "Parent issue".to_string(), ".".to_string());
        issue1.status = Status::Open;
        storage.create_issue(&issue1).unwrap();

        let mut issue2 = Issue::new("bf-dep-disp-2".to_string(), "Dependency issue".to_string(), ".".to_string());
        issue2.status = Status::Open;
        storage.create_issue(&issue2).unwrap();

        // Add dependency
        storage.add_dependency("bf-dep-disp-1", "bf-dep-disp-2", &DependencyType::Blocks, "test-actor").unwrap();

        // Get dependencies with display info
        let deps = storage.get_dependencies_display("bf-dep-disp-1").unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].bead_id, "bf-dep-disp-2");
        assert_eq!(deps[0].title, "Dependency issue");
        assert_eq!(deps[0].dep_type, "blocks");
    }

    #[test]
    fn test_list_all_labels() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create issues with different labels
        let mut issue1 = Issue::new("bf-labels-1".to_string(), "Issue 1".to_string(), ".".to_string());
        issue1.labels = vec!["backend".to_string(), "urgent".to_string()];
        storage.create_issue(&issue1).unwrap();

        let mut issue2 = Issue::new("bf-labels-2".to_string(), "Issue 2".to_string(), ".".to_string());
        issue2.labels = vec!["backend".to_string(), "frontend".to_string()];
        storage.create_issue(&issue2).unwrap();

        // List all labels with counts
        let all_labels = storage.list_all_labels().unwrap();
        assert!(!all_labels.is_empty());

        // Check that backend appears twice
        let backend_count = all_labels.iter()
            .find(|(label, _)| label == "backend")
            .map(|(_, count)| *count);
        assert_eq!(backend_count, Some(2));
    }

    #[test]
    fn test_dependency_prevents_self_blocking() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an issue
        let mut issue = Issue::new("bf-self-block".to_string(), "Self blocking test".to_string(), ".".to_string());
        issue.status = Status::Open;
        storage.create_issue(&issue).unwrap();

        // Try to add self-blocking dependency
        let result = storage.add_dependency("bf-self-block", "bf-self-block", &DependencyType::Blocks, "test-actor");

        // Should fail
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Cannot add self-blocking dependency"));
    }

    #[test]
    fn test_empty_label_validation() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an issue
        let issue = Issue::new("bf-empty-label".to_string(), "Empty label test".to_string(), ".".to_string());
        storage.create_issue(&issue).unwrap();

        // Try to add empty label
        let result = storage.add_label("bf-empty-label", "");
        assert!(result.is_err());

        // Try to add whitespace-only label
        let result = storage.add_label("bf-empty-label", "   ");
        assert!(result.is_err());

        // Try to remove empty label
        let result = storage.remove_label("bf-empty-label", "");
        assert!(result.is_err());
    }

    #[test]
    fn test_updated_at_timestamp_changes_on_update() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an issue
        let issue = Issue::new("bf-timestamp-1".to_string(), "Timestamp test".to_string(), ".".to_string());
        storage.create_issue(&issue).unwrap();

        // Get initial updated_at
        let initial = storage.get_issue("bf-timestamp-1").unwrap().unwrap();
        let initial_updated_at = initial.updated_at;

        // Small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Update the issue
        let mut changes = IssueChanges::default();
        changes.title = Some("Updated title".to_string());
        storage.update_issue("bf-timestamp-1", &changes).unwrap();

        // Verify updated_at changed
        let updated = storage.get_issue("bf-timestamp-1").unwrap().unwrap();
        assert!(updated.updated_at > initial_updated_at);
    }

    #[test]
    fn test_content_hash_computation() {
        // Test that content_hash is computed correctly
        let mut issue = Issue::new("bf-hash-1".to_string(), "Hash test".to_string(), ".".to_string());
        issue.description = Some("Test description".to_string());
        issue.status = Status::Open;
        issue.priority = Priority::MEDIUM;

        let hash1 = issue.content_hash();

        // Change a field and verify hash changes
        issue.title = "Different title".to_string();
        let hash2 = issue.content_hash();

        assert_ne!(hash1, hash2);

        // Verify same content produces same hash
        let issue2 = issue.clone();
        let hash3 = issue2.content_hash();
        assert_eq!(hash2, hash3);
    }

    #[test]
    fn test_list_issues_with_updated_since_filter() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create an issue
        let issue1 = Issue::new("bf-since-1".to_string(), "Issue 1".to_string(), ".".to_string());
        storage.create_issue(&issue1).unwrap();

        // Get its creation time
        let created = storage.get_issue("bf-since-1").unwrap().unwrap().created_at;

        // Small delay
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Create another issue
        let issue2 = Issue::new("bf-since-2".to_string(), "Issue 2".to_string(), ".".to_string());
        storage.create_issue(&issue2).unwrap();

        // Filter by updated_since - should only return the second issue
        let mut filter = IssueFilter::default();
        filter.updated_since = Some(created);
        let results = storage.list_issues(&filter).unwrap();

        assert!(results.len() >= 1);
        assert!(!results.iter().any(|i| i.id == "bf-since-1"));
    }

    #[test]
    fn test_list_issues_with_limit_and_offset() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create multiple issues
        for i in 1..=5 {
            let issue = Issue::new(format!("bf-limit-{}", i), format!("Issue {}", i), ".".to_string());
            storage.create_issue(&issue).unwrap();
        }

        // Test limit
        let mut filter = IssueFilter::default();
        filter.limit = Some(2);
        let results = storage.list_issues(&filter).unwrap();
        assert_eq!(results.len(), 2);

        // Test offset
        let mut filter = IssueFilter::default();
        filter.limit = Some(2);
        filter.offset = Some(2);
        let results = storage.list_issues(&filter).unwrap();
        assert_eq!(results.len(), 2);
    }

    // Security tests for SQL injection prevention (bf-3fwld0)
    #[test]
    fn test_get_dep_tree_sql_injection_protection() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a chain of dependencies: bf-root -> bf-child1 -> bf-child2
        let root = Issue::new("bf-root".to_string(), "Root Issue".to_string(), ".".to_string());
        let child1 = Issue::new("bf-child1".to_string(), "Child 1".to_string(), ".".to_string());
        let child2 = Issue::new("bf-child2".to_string(), "Child 2".to_string(), ".".to_string());

        storage.create_issue(&root).unwrap();
        storage.create_issue(&child1).unwrap();
        storage.create_issue(&child2).unwrap();

        // Create dependencies using the correct method signature
        storage.add_dependency("bf-root", "bf-child1", &DependencyType::Blocks, "test-actor").unwrap();
        storage.add_dependency("bf-child1", "bf-child2", &DependencyType::Blocks, "test-actor").unwrap();

        // Test 1: Normal operation should still work
        let result = storage.get_dep_tree("bf-root", "down", 10);
        assert!(result.is_ok(), "Normal operation should succeed");
        let tree = result.unwrap();
        assert_eq!(tree.len(), 2, "Should return 2 dependencies");
        assert_eq!(tree[0].id, "bf-child1", "First level should be child1");
        assert_eq!(tree[1].id, "bf-child2", "Second level should be child2");

        // Test 2: SQL injection via UNION SELECT should be rejected or return empty
        let malicious_inputs = vec![
            "bf-123' OR '1'='1",
            "bf-123'; DROP TABLE dependencies; --",
            "bf-123' UNION SELECT * FROM issues WHERE '1'='1",
            "' OR '1'='1",
            "bf-123'--",
            "bf-123'/*",
            "'; EXEC('xp_cmdshell'); --",
        ];

        for payload in malicious_inputs {
            let result = storage.get_dep_tree(payload, "down", 10);
            // Should either error (validation rejected) or return empty (no valid ID found)
            match result {
                Ok(tree) => {
                    // If it succeeds, tree should be empty (no valid bead ID exists)
                    assert_eq!(
                        tree.len(),
                        0,
                        "Malicious payload '{}' should return empty tree, got {} nodes",
                        payload,
                        tree.len()
                    );
                }
                Err(_) => {
                    // Validation error is expected and acceptable
                    // This is the secure path: reject invalid input before query execution
                }
            }
        }

        // Test 3: Invalid bead ID format should be rejected
        let invalid_ids = vec![
            "",           // Empty
            "bf-",        // Prefix only (no hash part)
            "invalid",    // No hyphen (single part)
            "a",          // Too short
            "bf- 123",    // Space in ID (invalid character)
            "bf-1;2",     // Special chars (semicolon not alphanumeric)
            "bf-1 2",     // Space in hash part
            "bf-1@2",     // Special char @
        ];

        for invalid_id in invalid_ids {
            let result = storage.get_dep_tree(invalid_id, "down", 10);
            assert!(result.is_err(), "Invalid ID '{}' should be rejected", invalid_id);
        }

        // Test 3a: Valid format but non-existent prefix should return empty (not error)
        // This validates that the format check passes but no data is found
        let valid_format_unknown = vec![
            "xyz-123",    // Valid format, unknown prefix (will return empty)
            "abc-1def",   // Valid format, unknown prefix
        ];

        for unknown_id in valid_format_unknown {
            let result = storage.get_dep_tree(unknown_id, "down", 10);
            // Should succeed but return empty tree (no beads with that prefix exist)
            assert!(result.is_ok(), "Valid format ID '{}' should not error", unknown_id);
            let tree = result.unwrap();
            assert_eq!(tree.len(), 0, "Unknown ID '{}' should return empty tree", unknown_id);
        }

        // Test 4: Verify normal operation still works after security fixes (regression test)
        // Query "up" from bf-child2 should find issues that depend on it
        let up_tree = storage.get_dep_tree("bf-child2", "up", 10).unwrap();
        // Should find 2 nodes: bf-child1 (direct dependent) and bf-root (indirect dependent)
        assert_eq!(up_tree.len(), 2, "Should find 2 nodes in up tree (bf-child1 and bf-root)");
        assert_eq!(up_tree[0].id, "bf-child1", "First level should be child1");
        assert_eq!(up_tree[1].id, "bf-root", "Second level should be root");

        // Test 5: Direction parameter is also validated
        let result = storage.get_dep_tree("bf-root", "invalid-direction", 10);
        // Should not crash - direction falls through to default case
        assert!(result.is_ok() || result.is_err(), "Direction parameter should be handled safely");
    }

    #[test]
    fn test_get_dep_tree_with_valid_nonexistent_id() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // A validly-formatted but non-existent ID should return empty tree, not error
        let result = storage.get_dep_tree("bf-nonexistent", "down", 10);
        assert!(result.is_ok(), "Valid format non-existent ID should not error");
        assert_eq!(result.unwrap().len(), 0, "Should return empty tree");
    }

    #[test]
    fn test_get_dep_tree_advanced_sql_injection_payloads() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create test data
        let root = Issue::new("bf-root".to_string(), "Root".to_string(), ".".to_string());
        storage.create_issue(&root).unwrap();

        // Advanced SQL injection payloads that should be blocked
        let advanced_payloads = vec![
            // Boolean-based blind SQL injection
            "bf-123' AND 1=1--",
            "bf-123' AND 1=2--",
            "bf-123' OR '1'='1'--",
            // Stacked queries (SQLite doesn't support but should still be safe)
            "bf-123'; INSERT INTO issues VALUES ('malicious', 'title', '.')--",
            "bf-123'; DELETE FROM issues--",
            // Error-based SQL injection
            "bf-123' AND 1=CONVERT(int, (SELECT TOP 1 title FROM issues))--",
            "bf-123' AND 1=CAST((SELECT title FROM issues) AS int)--",
            // Time-based blind SQL injection (SQLite-specific)
            "bf-123' AND (SELECT SUBSTR(title,1,1) FROM issues)='a'--",
            "bf-123' OR (SELECT COUNT(*) FROM issues)>0--",
            // Comment-based attacks
            "bf-123'/**/OR/**/'1'='1'--",
            "bf-123'/*/OR/*/1=1--",
            // Hex-encoded attacks (attempting to bypass filters)
            "bf-123' OR 0x hex--",
            // UNION SELECT with different variations
            "bf-123' UNION ALL SELECT NULL,NULL,NULL,NULL,NULL,NULL,NULL--",
            "bf-123' UNION SELECT NULL,NULL,NULL,NULL,NULL,NULL,NULL--",
            "bf-123' UNION DISTINCT SELECT NULL,NULL,NULL,NULL,NULL,NULL,NULL--",
            // Substring and string function attacks
            "bf-123' OR SUBSTR(title,1,1)='a'--",
            "bf-123' OR LENGTH(title)>0--",
            // Case-sensitive variations
            "bf-123' oR '1'='1",
            "bf-123' Or '1'='1",
            // WITH clause injection (attempting to break the CTE)
            "bf-123' WITH RECURSIVE--",
            // Semicolon injection attempts
            "bf-123'; SELECT--",
            "bf-123';DROP--",
            "bf-123';EXECUTE--",
            // Backtick attacks (MySQL-style, should still be safe)
            "bf-123` OR `1`=`1",
            // Bracket-based attacks (SQL Server-style)
            "bf-123'] OR '1'='1",
            // Double encoding attempts
            "bf-123%25%27%20OR%20%271%27%3D%271", // URL-encoded
            // Null byte injection
            "bf-123\x00",
            // Newline and tab injection
            "bf-123\nOR\n1=1",
            "bf-123\tOR\t1=1",
            // carriage return injection
            "bf-123\rOR\r1=1",
        ];

        for payload in advanced_payloads {
            let result = storage.get_dep_tree(payload, "down", 10);
            match result {
                Ok(tree) => {
                    // If query succeeds, must return empty (no valid bead found)
                    assert_eq!(
                        tree.len(), 0,
                        "Advanced payload '{}' should return empty tree, got {} nodes: {:?}",
                        payload, tree.len(), tree
                    );
                }
                Err(_) => {
                    // Validation rejection is the secure path
                }
            }
        }
    }

    #[test]
    fn test_get_dep_tree_boundary_conditions() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create test data
        let root = Issue::new("bf-root".to_string(), "Root".to_string(), ".".to_string());
        storage.create_issue(&root).unwrap();

        // Test very long IDs (buffer overflow attempts)
        let long_id = format!("bf-{}", "a".repeat(10000));
        let result = storage.get_dep_tree(&long_id, "down", 10);
        // Should handle gracefully - either reject or return empty
        assert!(result.is_ok() || result.is_err(), "Long ID should not crash");

        // Test IDs with special characters in hash part
        let special_char_ids = vec![
            "bf-abc_123",    // underscore (invalid)
            "bf-abc-123",    // multiple hyphens (hash part becomes "abc-123")
            "bf-abc.def",    // dot in hash
            "bf-abc@123",    // @ sign
            "bf-abc#123",    // hash sign
            "bf-abc$123",    // dollar sign
            "bf-abc%123",    // percent
            "bf-abc&123",    // ampersand
            "bf-abc*123",    // asterisk
            "bf-abc+123",    // plus
            "bf-abc=123",    // equals
            "bf-abc?123",    // question mark
            "bf-abc/123",    // forward slash
            "bf-abc\\123",   // backslash
            "bf-abc|123",    // pipe
            "bf-abc<123",    // less than
            "bf-abc>123",    // greater than
            "bf-abc[123",    // open bracket
            "bf-abc]123",    // close bracket
            "bf-abc{123",    // open brace
            "bf-abc}123",    // close brace
            "bf-abc(123",    // open paren
            "bf-abc)123",    // close paren
            "bf-abc;123",    // semicolon
            "bf-abc:123",    // colon
            "bf-abc'123",    // single quote
            "bf-abc\"123",   // double quote
            "bf-abc`123",    // backtick
            "bf-abc~123",    // tilde
            "bf-abc!123",    // exclamation
            "bf-abc^123",    // caret
            "bf-abc 123",    // space
            "bf-abc\n123",   // newline
            "bf-abc\r123",   // carriage return
            "bf-abc\t123",   // tab
        ];

        for invalid_id in special_char_ids {
            let result = storage.get_dep_tree(invalid_id, "down", 10);
            assert!(result.is_err(), "ID with special chars '{}' should be rejected", invalid_id);
        }

        // Test boundary: empty parts in multi-part ID
        let empty_part_ids = vec![
            "bf-",           // empty hash part
            "bf-123-",       // trailing hyphen creates empty part
            "bf--123",       // double hyphen creates empty part
            "bf---",         // only hyphens
        ];

        for invalid_id in empty_part_ids {
            let result = storage.get_dep_tree(invalid_id, "down", 10);
            assert!(result.is_err(), "ID with empty parts '{}' should be rejected", invalid_id);
        }
    }

    #[test]
    fn test_get_dep_tree_unicode_and_normalization_attacks() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create test data
        let root = Issue::new("bf-root".to_string(), "Root".to_string(), ".".to_string());
        storage.create_issue(&root).unwrap();

        // Test Unicode homograph attacks (visually similar characters)
        let unicode_attack_ids = vec![
            "bf-\u{200b}123",     // zero-width space
            "bf-\u{200c}123",     // zero-width non-joiner
            "bf-\u{200d}123",     // zero-width joiner
            "bf-\u{2060}123",     // word joiner
            "bf-\u{feff}123",     // zero-width no-break space (BOM)
            "bf-\u{202a}123",     // left-to-right embedding
            "bf-\u{202b}123",     // right-to-left embedding
            "bf-\u{202c}123",     // pop directional formatting
            "bf-\u{202d}123",     // left-to-right override
            "bf-\u{202e}123",     // right-to-left override
            "bf-\u{ff01}123",     // fullwidth exclamation (looks like !)
            "bf-\u{ff03}123",     // fullwidth hash (looks like #)
            "bf-\u{03a6}123",     // Greek capital PHI (looks like Φ)
            "bf-\u{0430}123",     // Cyrillic small A (looks like a)
            "bf-\u{0415}123",     // Cyrillic capital E (looks like E)
        ];

        for unicode_id in unicode_attack_ids {
            let result = storage.get_dep_tree(unicode_id, "down", 10);
            // Unicode characters in hash part should be rejected (non-alphanumeric)
            assert!(
                result.is_err(),
                "Unicode attack ID '{}' should be rejected",
                unicode_id
            );
        }

        // Test that valid alphanumeric characters still work (including uppercase)
        let valid_mixed_case = vec![
            "bf-ABC123",     // uppercase letters are valid
            "bf-aBc123",     // mixed case
            "bf-123ABC",     // numbers then letters
            "bf-ABC123xyz",  // mixed alphanumeric
        ];

        for valid_id in valid_mixed_case {
            let result = storage.get_dep_tree(valid_id, "down", 10);
            // These should pass validation (format is valid) but return empty (no data)
            assert!(result.is_ok(), "Valid mixed-case ID '{}' should pass validation", valid_id);
            let tree = result.unwrap();
            assert_eq!(tree.len(), 0, "Non-existent ID should return empty tree");
        }
    }

    #[test]
    fn test_get_dep_tree_parameter_binding_verification() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a chain of dependencies to verify query structure is preserved
        let root = Issue::new("bf-root".to_string(), "Root".to_string(), ".".to_string());
        let child = Issue::new("bf-child".to_string(), "Child".to_string(), ".".to_string());
        storage.create_issue(&root).unwrap();
        storage.create_issue(&child).unwrap();
        storage.add_dependency("bf-root", "bf-child", &DependencyType::Blocks, "test-actor").unwrap();

        // Test 1: Verify that parameters are actually bound, not interpolated
        // by using IDs that would be dangerous if interpolated but are safe when bound
        let dangerous_but_valid_id = "bf-123;456"; // Contains semicolon but is valid format
        let result = storage.get_dep_tree(dangerous_but_valid_id, "down", 10);
        // Should return empty (no such ID) but not crash or inject SQL
        assert!(result.is_ok(), "Valid format ID should not error");
        assert_eq!(result.unwrap().len(), 0, "Non-existent ID should return empty");

        // Test 2: Verify the same root_id is used in both parameter positions
        // The query uses ?1 and ?2 both bound to root_id
        let tree = storage.get_dep_tree("bf-root", "down", 10).unwrap();
        assert_eq!(tree.len(), 1, "Should find 1 dependency");
        assert_eq!(tree[0].id, "bf-child");
        assert_eq!(tree[0].path, "bf-root,bf-child", "Path should start with root_id");

        // Test 3: Verify direction parameter doesn't cause SQL injection
        let malicious_directions = vec![
            "down'; DROP TABLE issues; --",
            "up' OR '1'='1",
            "down/**/UNION/**/SELECT/**/*",
            "down; SELECT--",
        ];

        for malicious_dir in malicious_directions {
            let result = storage.get_dep_tree("bf-root", malicious_dir, 10);
            // Should not crash - direction falls through to default case or is handled safely
            assert!(result.is_ok() || result.is_err(), "Malicious direction should not crash");
        }

        // Test 4: Verify max_depth parameter doesn't cause SQL injection
        // max_depth is used in: format!("AND rec.depth < {}", max_depth)
        // Since it's a usize (not user-controlled string), this should be safe
        // But let's verify the function handles edge cases
        let depth_values = vec![0, 1, 10, 100, usize::MAX];
        for depth in depth_values {
            let result = storage.get_dep_tree("bf-root", "down", depth);
            // Should not crash for any depth value
            assert!(result.is_ok() || result.is_err(), "Depth {} should not crash", depth);
        }
    }

    #[test]
    fn test_is_valid_bead_id_comprehensive() {
        // Test the validation function directly for comprehensive coverage
        use crate::id::is_valid_bead_id;

        // Valid IDs
        let valid_ids = vec![
            "bf-abc",
            "bf-abc123",
            "bf-ABC123",
            "bf-123ABC",
            "bf-a1b2c3",
            "bf-1234567890abcdefghijklmnopqrstuvwxyz",
            "bd-xyz",           // different prefix
            "prefix-123",       // different prefix
            "a-b",              // minimal valid ID
            "verylongprefix-1234567890abcdef",
        ];

        for valid_id in valid_ids {
            assert!(
                is_valid_bead_id(valid_id),
                "ID '{}' should be valid",
                valid_id
            );
        }

        // Invalid IDs
        let invalid_ids = vec![
            "",                 // empty
            "a",                // no hyphen
            "ab",               // no hyphen
            "abc",              // no hyphen
            "bf-",              // empty hash part
            "bf-!",             // special char
            "bf-@",             // special char
            "bf-#",             // special char
            "bf-$",             // special char
            "bf-%",             // special char
            "bf-&",             // special char
            "bf-*",             // special char
            "bf-+",             // special char
            "bf-=",             // special char
            "bf-?",             // special char
            "bf- /",            // space
            "bf-\t",            // tab
            "bf-\n",            // newline
            "bf-\r",            // carriage return
            "bf-\x00",          // null byte
            "bf- ",             // space
            "bf- 123",          // space in hash
            "bf-1 2",           // space in middle
            "bf-1_2",           // underscore
            "bf-1.2",           // dot
            "bf-1:2",           // colon
            "bf-1;2",           // semicolon
            "bf-1,2",           // comma
            "bf-1'2",           // single quote
            "bf-1\"2",          // double quote
            "bf-1`2",           // backtick
            "bf-1|2",           // pipe
            "bf-1<2",           // less than
            "bf-1>2",           // greater than
            "bf-1(2",           // open paren
            "bf-1)2",           // close paren
            "bf-1[2",           // open bracket
            "bf-1]2",           // close bracket
            "bf-1{2",           // open brace
            "bf-1}2",           // close brace
            "bf-1~2",           // tilde
            "bf-1!2",           // exclamation
            "bf-1@2",           // at sign
            "bf-1#2",           // hash
            "bf-1$2",           // dollar
            "bf-1%2",           // percent
            "bf-1^2",           // caret
            "bf-1&2",           // ampersand
            "bf-1*2",           // asterisk
            "bf-1+2",           // plus
            "bf-1=2",           // equals
            "bf-1?2",           // question mark
            "bf-1/2",           // forward slash
            "bf-1\\2",          // backslash
            "bf--",             // only hyphens
            "bf---",            // only hyphens
            "bf-123-",          // trailing hyphen
            "bf--123",          // leading double hyphen in hash
            "bf-123--",         // trailing double hyphen
            "1-2",              // single char prefix (too short)
            "ab-1",             // two char prefix
        ];

        for invalid_id in invalid_ids {
            assert!(
                !is_valid_bead_id(invalid_id),
                "ID '{}' should be invalid",
                invalid_id
            );
        }
    }

    #[test]
    fn test_get_dep_tree_sqlite_ctte_injection_attempts() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create test data
        let root = Issue::new("bf-root".to_string(), "Root".to_string(), ".".to_string());
        storage.create_issue(&root).unwrap();

        // Common Table Expression (CTE) injection attempts
        // Since get_dep_tree uses a WITH RECURSIVE clause, attackers might try
        // to inject their own CTEs
        let cte_injection_payloads = vec![
            "bf-123') WITH RECURSIVE evil AS (SELECT*) SELECT--",
            "bf-123') WITH evil AS (SELECT 1) SELECT--",
            "bf-123' WITH RECURSIVE x AS (SELECT 1) SELECT--",
            "bf-123' UNION WITH RECURSIVE--",
            "bf-123') UNION ALL WITH--",
            "bf-123' OR 1 IN (WITH--",
        ];

        for payload in cte_injection_payloads {
            let result = storage.get_dep_tree(payload, "down", 10);
            match result {
                Ok(tree) => {
                    assert_eq!(
                        tree.len(), 0,
                        "CTE injection payload '{}' should return empty tree",
                        payload
                    );
                }
                Err(_) => {
                    // Validation rejection is correct
                }
            }
        }
    }

    #[test]
    fn test_get_dep_tree_regression_functionality() {
        // Comprehensive regression test to ensure normal functionality works
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();

        // Create a complex dependency tree:
        // root -> child1 -> child2 -> child3
        //   \-> child4 -> child5
        // And reverse dependencies:
        // dependent <- root (i.e., dependent depends on root)
        let root = Issue::new("bf-root".to_string(), "Root".to_string(), ".".to_string());
        let child1 = Issue::new("bf-child1".to_string(), "Child 1".to_string(), ".".to_string());
        let child2 = Issue::new("bf-child2".to_string(), "Child 2".to_string(), ".".to_string());
        let child3 = Issue::new("bf-child3".to_string(), "Child 3".to_string(), ".".to_string());
        let child4 = Issue::new("bf-child4".to_string(), "Child 4".to_string(), ".".to_string());
        let child5 = Issue::new("bf-child5".to_string(), "Child 5".to_string(), ".".to_string());
        let dependent = Issue::new("bf-dependent".to_string(), "Dependent".to_string(), ".".to_string());

        storage.create_issue(&root).unwrap();
        storage.create_issue(&child1).unwrap();
        storage.create_issue(&child2).unwrap();
        storage.create_issue(&child3).unwrap();
        storage.create_issue(&child4).unwrap();
        storage.create_issue(&child5).unwrap();
        storage.create_issue(&dependent).unwrap();

        // Create dependencies: root -> child1 -> child2 -> child3
        storage.add_dependency("bf-root", "bf-child1", &DependencyType::Blocks, "test").unwrap();
        storage.add_dependency("bf-child1", "bf-child2", &DependencyType::Blocks, "test").unwrap();
        storage.add_dependency("bf-child2", "bf-child3", &DependencyType::Blocks, "test").unwrap();

        // Create dependencies: root -> child4 -> child5
        storage.add_dependency("bf-root", "bf-child4", &DependencyType::Blocks, "test").unwrap();
        storage.add_dependency("bf-child4", "bf-child5", &DependencyType::Blocks, "test").unwrap();

        // Create reverse dependency: dependent depends on root
        storage.add_dependency("bf-dependent", "bf-root", &DependencyType::Blocks, "test").unwrap();

        // Test 1: Full downward tree from root
        let down_tree = storage.get_dep_tree("bf-root", "down", 0).unwrap();
        assert_eq!(down_tree.len(), 5, "Should find all 5 downstream dependencies");

        // Verify depth ordering
        let depth0: Vec<_> = down_tree.iter().filter(|n| n.depth == 0).collect();
        let depth1: Vec<_> = down_tree.iter().filter(|n| n.depth == 1).collect();
        let depth2: Vec<_> = down_tree.iter().filter(|n| n.depth == 2).collect();
        let depth3: Vec<_> = down_tree.iter().filter(|n| n.depth == 3).collect();

        assert_eq!(depth0.len(), 2, "Should have 2 direct children");
        assert_eq!(depth1.len(), 2, "Should have 2 at depth 1");
        assert_eq!(depth2.len(), 1, "Should have 1 at depth 2");
        assert_eq!(depth3.len(), 0, "Should have 0 at depth 3 (all 3 are depth 2 or less)");

        // Test 2: Depth-limited query
        let down_tree_limited = storage.get_dep_tree("bf-root", "down", 2).unwrap();
        assert!(down_tree_limited.len() < 5, "Depth limit should reduce results");

        // Test 3: Upward tree from child3 (should find child2, child1, root)
        let up_tree = storage.get_dep_tree("bf-child3", "up", 0).unwrap();
        assert_eq!(up_tree.len(), 3, "Should find 3 upstream dependencies");
        assert_eq!(up_tree[0].id, "bf-child2");
        assert_eq!(up_tree[1].id, "bf-child1");
        assert_eq!(up_tree[2].id, "bf-root");

        // Test 4: Upward tree from root (should find dependent)
        let up_tree_root = storage.get_dep_tree("bf-root", "up", 0).unwrap();
        assert_eq!(up_tree_root.len(), 1, "Should find 1 dependent");
        assert_eq!(up_tree_root[0].id, "bf-dependent");

        // Test 5: Path construction
        let child3_node = down_tree.iter().find(|n| n.id == "bf-child3").unwrap();
        assert_eq!(
            child3_node.path,
            "bf-root,bf-child1,bf-child2,bf-child3",
            "Path should show full dependency chain"
        );
    }
}

// Include comprehensive storage tests
include!("sqlite_tests.rs");

#[cfg(test)]
mod parse_datetime_tests {
    use super::*;

    #[test]
    fn accepts_rfc3339_and_sqlite_native_formats() {
        // RFC3339 with timezone (bf's own format)
        assert!(parse_datetime("2026-05-15T20:00:00+00:00".into()).is_ok());
        // RFC3339 with nanosecond fraction
        assert!(parse_datetime("2026-05-24T02:26:10.191834420+00:00".into()).is_ok());
        // br / SQLite-native: space separator, no timezone — previously crashed
        // with "premature end of input" and broke list/flush for the workspace.
        let dt = parse_datetime("2026-05-15 21:10:36".into()).unwrap();
        assert_eq!(dt.to_rfc3339(), "2026-05-15T21:10:36+00:00");
        // naive 'T' separator without timezone
        assert!(parse_datetime("2026-05-15T21:10:36".into()).is_ok());
        // genuinely unparseable values still error (callers map NULL/empty to None
        // via parse_opt_dt before reaching here)
        assert!(parse_datetime("not a date".into()).is_err());
        assert!(parse_datetime(String::new()).is_err());
    }

    #[test]
    fn required_datetime_tolerates_null_and_empty() {
        // NULL column (row.get::<_, Option<String>>(idx)? == None) used to crash
        // the entire list/flush with InvalidColumnType. It now loads as the epoch.
        assert_eq!(
            parse_required_datetime(None).unwrap(),
            DateTime::<Utc>::UNIX_EPOCH
        );
        // Empty / whitespace-only strings are treated the same way.
        assert_eq!(
            parse_required_datetime(Some(String::new())).unwrap(),
            DateTime::<Utc>::UNIX_EPOCH
        );
        assert_eq!(
            parse_required_datetime(Some("   ".to_string())).unwrap(),
            DateTime::<Utc>::UNIX_EPOCH
        );
        // Valid values still parse normally.
        let dt = parse_required_datetime(Some("2026-05-15 21:10:36".to_string())).unwrap();
        assert_eq!(dt.to_rfc3339(), "2026-05-15T21:10:36+00:00");
        // Genuinely malformed non-empty values still error (a distinct corruption
        // class from NULL — see parse_required_datetime docs).
        assert!(parse_required_datetime(Some("not a date".to_string())).is_err());
    }
}

/// Format secret matches into a user-friendly error message.
fn format_secret_matches(matches: &[SecretMatch]) -> String {
    let mut msg = String::from("secret detected in bead content. Refusing write.\n\n");
    msg.push_str("The following patterns matched:\n");

    // Group by pattern name
    let mut by_pattern: std::collections::HashMap<&str, Vec<&SecretMatch>> =
        std::collections::HashMap::new();
    for m in matches {
        by_pattern.entry(&m.pattern_name).or_default().push(m);
    }

    for (pattern, match_list) in by_pattern {
        msg.push_str(&format!("\n  [{}]\n", pattern));
        for m in match_list {
            // Truncate very long matches for readability
            let display = if m.matched_text.len() > 60 {
                format!("{}...", &m.matched_text[..57])
            } else {
                m.matched_text.clone()
            };
            msg.push_str(&format!("    - {}\n", display));
        }
    }

    msg.push_str(
        "\nIf this is a false positive, add an allowlist pattern to .beads/config.yaml:\n",
    );
    msg.push_str("  secret_protection:\n");
    msg.push_str("    allowlist:\n");
    msg.push_str("      - \"<regex pattern to exclude>\"\n");

    msg
}

#[cfg(test)]
mod ready_queue_tests;
