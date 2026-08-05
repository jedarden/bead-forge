use crate::critical_path::{compute_all_critical_paths, invalidate_cache};
use crate::jsonl::{export_jsonl, export_jsonl_dirty, import_jsonl, ImportResult, UpsertResult};
use crate::model::{
    Comment, Dependency, DependencyType, Event, EventType, Issue, IssueChanges, IssueFilter,
    IssueType, Status,
};
use crate::secrets::{SecretMatch, SecretScanner};
use crate::storage::schema::{apply_schema, ensure_wal_mode};
use anyhow::{anyhow, Result};
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
pub struct SecretError(String);

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
            query.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = filter.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(&query)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
        let mut rows = stmt.query(param_refs.as_slice())?;
        let mut issues = Vec::new();
        while let Some(row) = rows.next()? {
            issues.push(Self::row_to_issue_conn(&conn, row)?);
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
             INNER JOIN dirty_issues d ON i.id = d.issue_id
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
                "INSERT OR REPLACE INTO dirty_issues (issue_id, marked_at) VALUES (?1, ?2)",
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
            return Err(anyhow!("Bead not found: {}", id));
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
                // Clear closed fields when transitioning from closed/tombstone to open
                if !matches!(status, Status::Closed | Status::Tombstone) {
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
                    "INSERT OR REPLACE INTO dirty_issues (issue_id, marked_at) VALUES (?1, ?2)",
                    params![id, now.to_rfc3339()],
                )?;
            }
            // Invalidate critical path cache if status changed (affects dependency graph)
            if changes.status.is_some() {
                invalidate_cache(tx)?;
                compute_all_critical_paths(tx)?;
            }
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
                return Err(anyhow!("Bead not found: {}", id));
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
                "UPDATE issues SET status = 'closed', closed_at = ?, close_reason = ?, closed_by_session = ?, updated_at = ? WHERE id = ?",
                params![now.to_rfc3339(), reason, actor, now.to_rfc3339(), id],
            )?;
            tx.execute(
                "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, created_at) VALUES (?1, 'closed', ?2, NULL, ?3, ?4)",
                params![id, actor, reason, now.to_rfc3339()],
            )?;
            tx.execute(
                "INSERT OR REPLACE INTO dirty_issues (issue_id, marked_at) VALUES (?1, ?2)",
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
                        "INSERT OR REPLACE INTO dirty_issues (issue_id, marked_at) VALUES (?1, ?2)",
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
                return Err(anyhow!("Bead not found: {}", id));
            }

            // Check if bead is currently closed
            if current_status.as_deref() != Some("closed") {
                return Err(anyhow!(
                    "Cannot reopen bead {}: status is '{}', must be 'closed'",
                    id,
                    current_status.unwrap()
                ));
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
                "INSERT OR REPLACE INTO dirty_issues (issue_id, marked_at) VALUES (?1, ?2)",
                params![id, now.to_rfc3339()],
            )?;

            // Invalidate critical path cache: reopening a bead can change dependencies
            invalidate_cache(tx)?;
            compute_all_critical_paths(tx)?;
            Ok(())
        })
    }

    pub fn mark_dirty(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO dirty_issues (issue_id, marked_at) VALUES (?1, ?2)",
            params![id, now],
        )?;
        Ok(())
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
                || self.list_dirty_issues(),
                || self.clear_dirty(),
            )?;
            Ok(result.count)
        } else {
            let result = export_jsonl(jsonl_path, || self.list_all_issues())?;
            // Clear dirty flags after full export (all beads are now synced)
            self.clear_dirty()?;
            Ok(result.count)
        }
    }

    fn row_to_issue_conn(conn: &Connection, row: &rusqlite::Row) -> Result<Issue> {
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
            let now = Utc::now();
            tx.execute(
                "INSERT INTO dependencies (issue_id, depends_on_id, type, created_at, created_by) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![issue_id, depends_on_id, dep_type.to_string(), now.to_rfc3339(), created_by],
            )?;
            // The dependency is stored on issue_id's record and exported with it.
            mark_dirty_tx(tx, issue_id)?;
            // Invalidate critical path cache after adding a dependency
            invalidate_cache(tx)?;
            compute_all_critical_paths(tx)?;
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
                    '{root_id}' || ',' || {id_col} as path
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
        let mut rows = stmt.query(params![root_id])?;
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
            });
        }
        Ok(deps)
    }

    pub fn add_label(&self, issue_id: &str, label: &str) -> Result<()> {
        let trimmed_label = label.trim();
        if trimmed_label.is_empty() {
            return Err(anyhow::anyhow!("Label cannot be empty or whitespace only"));
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
            return Err(anyhow::anyhow!("Label cannot be empty or whitespace only"));
        }

        self.with_immediate_transaction(|tx| {
            tx.execute(
                "DELETE FROM labels WHERE issue_id = ?1 AND label = ?2",
                params![issue_id, trimmed_label],
            )?;
            tx.execute(
                "DELETE FROM bead_labels WHERE bead_id = ?1 AND label = ?2",
                params![issue_id, trimmed_label],
            )?;
            mark_dirty_tx(tx, issue_id)?;
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
             LEFT JOIN labels l ON i.id = l.issue_id
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
            let label_conditions: Vec<String> = labels
                .iter()
                .enumerate()
                .map(|(i, _)| format!("l.label = ?{}", param_idx + i))
                .collect();
            sql.push_str(&format!(" AND ({}) ", label_conditions.join(" OR ")));
            for label in labels {
                params.push(label.clone());
                param_idx += 1;
            }
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
        "INSERT OR REPLACE INTO dirty_issues (issue_id, marked_at) VALUES (?1, ?2)",
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
