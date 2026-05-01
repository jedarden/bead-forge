use crate::jsonl::{export_jsonl, export_jsonl_dirty, import_jsonl, ImportResult};
use crate::model::{
    Comment, Dependency, DependencyType, Issue, IssueChanges, IssueFilter, IssueType, Status,
};
use crate::storage::schema::{apply_schema, ensure_wal_mode};
use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Transaction};
use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;
use std::sync::Mutex;
use std::time::Duration;

const MAX_RETRIES: u32 = 5;
const RETRY_BASE_MS: u64 = 50;

pub struct Storage {
    conn: Mutex<Connection>,
}

impl Storage {
    pub fn open(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        ensure_wal_mode(&conn)?;
        // Apply schema on every open - all tables use CREATE TABLE IF NOT EXISTS
        // which is a no-op for existing tables and avoids DDL lock contention
        apply_schema(&conn)?;
        Ok(Storage { conn: Mutex::new(conn) })
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
                            Ok(_) => { let _ = conn.execute_batch("COMMIT"); }
                            Err(_) => { let _ = conn.execute_batch("ROLLBACK"); }
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
        let mut stmt = conn.prepare(
            "SELECT id, content_hash, title, description, design, acceptance_criteria, notes,
                    status, priority, issue_type, assignee, owner, estimated_minutes,
                    created_at, created_by, updated_at, closed_at, close_reason,
                    closed_by_session, due_at, defer_until, external_ref, source_system,
                    source_repo, deleted_at, deleted_by, delete_reason, original_type,
                    compaction_level, compacted_at, compacted_at_commit, original_size,
                    sender, ephemeral, pinned, is_template
             FROM issues WHERE id = ?1",
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
            "SELECT id, content_hash, title, description, design, acceptance_criteria, notes,
                    status, priority, issue_type, assignee, owner, estimated_minutes,
                    created_at, created_by, updated_at, closed_at, close_reason,
                    closed_by_session, due_at, defer_until, external_ref, source_system,
                    source_repo, deleted_at, deleted_by, delete_reason, original_type,
                    compaction_level, compacted_at, compacted_at_commit, original_size,
                    sender, ephemeral, pinned, is_template
             FROM issues WHERE deleted_at IS NULL",
        );
        let mut params = Vec::new();
        let mut param_idx = 1;
        if let Some(ref status) = filter.status {
            query.push_str(&format!(" AND status = ?{}", param_idx));
            params.push(status.to_string());
            param_idx += 1;
        }
        if let Some(ref issue_type) = filter.issue_type {
            query.push_str(&format!(" AND issue_type = ?{}", param_idx));
            params.push(issue_type.to_string());
            param_idx += 1;
        }
        if let Some(ref assignee) = filter.assignee {
            query.push_str(&format!(" AND assignee = ?{}", param_idx));
            params.push(assignee.clone());
            param_idx += 1;
        }
        if let Some(priority) = filter.priority {
            query.push_str(&format!(" AND priority = ?{}", param_idx));
            params.push(priority.to_string());
            param_idx += 1;
        }
        query.push_str(" ORDER BY priority ASC, created_at ASC");
        if let Some(limit) = filter.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = filter.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&query)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
        let mut rows = stmt.query(param_refs.as_slice())?;
        let mut issues = Vec::new();
        while let Some(row) = rows.next()? {
            issues.push(Self::row_to_issue_conn(&conn, row)?);
        }
        Ok(issues)
    }

    pub fn list_all_issues(&self) -> Result<Vec<Issue>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, content_hash, title, description, design, acceptance_criteria, notes,
                    status, priority, issue_type, assignee, owner, estimated_minutes,
                    created_at, created_by, updated_at, closed_at, close_reason,
                    closed_by_session, due_at, defer_until, external_ref, source_system,
                    source_repo, deleted_at, deleted_by, delete_reason, original_type,
                    compaction_level, compacted_at, compacted_at_commit, original_size,
                    sender, ephemeral, pinned, is_template
             FROM issues WHERE deleted_at IS NULL ORDER BY id",
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
                    i.sender, i.ephemeral, i.pinned, i.is_template
             FROM issues i
             INNER JOIN dirty_issues d ON i.id = d.issue_id
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
                    &issue.id, &issue.content_hash, &issue.title,
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
            for label in &issue.labels {
                tx.execute("INSERT INTO labels (issue_id, label) VALUES (?1, ?2)", params![&issue.id, label])?;
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
        })
    }

    pub fn update_issue(&self, id: &str, changes: &IssueChanges) -> Result<()> {
        self.with_immediate_transaction(|tx| {
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
                updates.push("assignee = ?");
                params.push(Box::new(assignee.clone()));
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
            }
            // Handle label updates separately
            if let Some(ref labels) = changes.labels {
                tx.execute("DELETE FROM labels WHERE issue_id = ?1", params![id])?;
                for label in labels {
                    tx.execute("INSERT INTO labels (issue_id, label) VALUES (?1, ?2)", params![id, label])?;
                }
            }
            // Handle annotation updates separately
            if let Some(ref annotations) = changes.annotations {
                tx.execute("DELETE FROM bead_annotations WHERE bead_id = ?1", params![id])?;
                for (key, value) in annotations {
                    tx.execute("INSERT INTO bead_annotations (bead_id, key, value) VALUES (?1, ?2, ?3)", params![id, key, value])?;
                }
            }
            // Mark as dirty for export (if any changes were made)
            if !updates.is_empty() || changes.labels.is_some() || changes.annotations.is_some() {
                tx.execute(
                    "INSERT OR REPLACE INTO dirty_issues (issue_id, marked_at) VALUES (?1, ?2)",
                    params![id, now.to_rfc3339()],
                )?;
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
                tx.execute("INSERT INTO labels (issue_id, label) VALUES (?1, ?2)", params![&issue.id, label])?;
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

            Ok(())
        })
    }

    pub fn close_issue(&self, id: &str, reason: &str, actor: &str) -> Result<()> {
        self.with_immediate_transaction(|tx| {
            let now = Utc::now();
            tx.execute(
                "UPDATE issues SET status = 'closed', closed_at = ?, close_reason = ?, updated_at = ? WHERE id = ?",
                params![now.to_rfc3339(), reason, now.to_rfc3339(), id],
            )?;
            tx.execute(
                "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, created_at) VALUES (?1, 'closed', ?2, NULL, ?3, ?4)",
                params![id, actor, reason, now.to_rfc3339()],
            )?;
            tx.execute(
                "INSERT OR REPLACE INTO dirty_issues (issue_id, marked_at) VALUES (?1, ?2)",
                params![id, now.to_rfc3339()],
            )?;
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
                 AND i.status NOT IN ('closed', 'tombstone')
                 GROUP BY d.issue_id",
                params![Utc::now().to_rfc3339()],
            )?;
            Ok(())
        })
    }

    pub fn count_issues(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM issues", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn sync_from_jsonl(&self, jsonl_path: &Path) -> Result<ImportResult> {
        import_jsonl(jsonl_path, |issue| {
            let existing = self.get_issue(&issue.id)?;
            match existing {
                None => {
                    self.create_issue(issue)?;
                    Ok(true)
                }
                Some(_) => Ok(false),
            }
        })
    }

    pub fn sync_to_jsonl(&self, jsonl_path: &Path, dirty_only: bool) -> Result<usize> {
        if dirty_only {
            let result = export_jsonl_dirty(jsonl_path, || self.list_dirty_issues(), || self.clear_dirty())?;
            Ok(result.count)
        } else {
            let result = export_jsonl(jsonl_path, || self.list_all_issues())?;
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
            created_at: parse_datetime(row.get(13)?)?,
            created_by: row.get(14)?,
            updated_at: parse_datetime(row.get(15)?)?,
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
            labels: Self::load_labels_conn(conn, &id)?,
            dependencies: Self::load_dependencies_conn(conn, &id)?,
            comments: Self::load_comments_conn(conn, &id)?,
            annotations: Self::load_annotations_conn(conn, &id)?,
            id,
        })
    }

    fn load_labels_conn(conn: &Connection, issue_id: &str) -> Result<Vec<String>> {
        let mut stmt = conn.prepare("SELECT label FROM labels WHERE issue_id = ?1")?;
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
                dep_type: DependencyType::from_str(&type_str).unwrap_or(DependencyType::Custom(type_str)),
                metadata: row.get::<_, Option<String>>(3)?.and_then(|s| serde_json::from_str(&s).ok()),
                thread_id: row.get(4)?,
                created_at: parse_datetime(row.get(5)?)?,
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
                created_at: parse_datetime(row.get(4)?)?,
            });
        }
        Ok(comments)
    }

    fn load_annotations_conn(conn: &Connection, issue_id: &str) -> Result<BTreeMap<String, String>> {
        let mut stmt = conn.prepare("SELECT key, value FROM bead_annotations WHERE bead_id = ?1")?;
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

    pub fn add_dependency(&self, issue_id: &str, depends_on_id: &str, dep_type: &DependencyType, created_by: &str) -> Result<()> {
        self.with_immediate_transaction(|tx| {
            let now = Utc::now();
            tx.execute(
                "INSERT INTO dependencies (issue_id, depends_on_id, type, created_at, created_by) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![issue_id, depends_on_id, dep_type.to_string(), now.to_rfc3339(), created_by],
            )?;
            Ok(())
        })
    }

    pub fn remove_dependency(&self, issue_id: &str, depends_on_id: &str) -> Result<()> {
        self.with_immediate_transaction(|tx| {
            tx.execute("DELETE FROM dependencies WHERE issue_id = ?1 AND depends_on_id = ?2", params![issue_id, depends_on_id])?;
            Ok(())
        })
    }

    pub fn get_dependencies(&self, issue_id: &str) -> Result<Vec<Dependency>> {
        self.load_dependencies(issue_id)
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
                dep_type: DependencyType::from_str(&type_str).unwrap_or(DependencyType::Custom(type_str)),
                metadata: row.get::<_, Option<String>>(3)?.and_then(|s| serde_json::from_str(&s).ok()),
                thread_id: row.get(4)?,
                created_at: parse_datetime(row.get(5)?)?,
                created_by: row.get(6)?,
            });
        }
        Ok(deps)
    }

    pub fn add_label(&self, issue_id: &str, label: &str) -> Result<()> {
        self.with_immediate_transaction(|tx| {
            tx.execute("INSERT OR IGNORE INTO labels (issue_id, label) VALUES (?1, ?2)", params![issue_id, label])?;
            Ok(())
        })
    }

    pub fn remove_label(&self, issue_id: &str, label: &str) -> Result<()> {
        self.with_immediate_transaction(|tx| {
            tx.execute("DELETE FROM labels WHERE issue_id = ?1 AND label = ?2", params![issue_id, label])?;
            Ok(())
        })
    }

    pub fn get_labels(&self, issue_id: &str) -> Result<Vec<String>> {
        self.load_labels(issue_id)
    }

    pub fn list_all_labels(&self) -> Result<Vec<(String, i64)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT label, COUNT(*) as count FROM labels GROUP BY label ORDER BY count DESC")?;
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
            Ok(tx.last_insert_rowid())
        })
    }

    pub fn list_comments(&self, issue_id: &str) -> Result<Vec<Comment>> {
        self.load_comments(issue_id)
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
            Ok(())
        })
    }

    pub fn remove_annotation(&self, issue_id: &str, key: &str) -> Result<()> {
        self.with_immediate_transaction(|tx| {
            tx.execute(
                "DELETE FROM bead_annotations WHERE bead_id = ?1 AND key = ?2",
                params![issue_id, key],
            )?;
            Ok(())
        })
    }

    pub fn clear_annotations(&self, issue_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM bead_annotations WHERE bead_id = ?1", params![issue_id])?;
        Ok(())
    }

    pub fn search_issues(&self, query: Option<&str>, status: &[Status], issue_type: &[IssueType], assignee: Option<&str>, labels: &[String], priority_min: Option<i32>, priority_max: Option<i32>, limit: usize) -> Result<Vec<Issue>> {
        let mut sql = String::from(
            "SELECT DISTINCT i.id, i.content_hash, i.title, i.description, i.design, i.acceptance_criteria, i.notes,
                    i.status, i.priority, i.issue_type, i.assignee, i.owner, i.estimated_minutes,
                    i.created_at, i.created_by, i.updated_at, i.closed_at, i.close_reason,
                    i.closed_by_session, i.due_at, i.defer_until, i.external_ref, i.source_system,
                    i.source_repo, i.deleted_at, i.deleted_by, i.delete_reason, i.original_type,
                    i.compaction_level, i.compacted_at, i.compacted_at_commit, i.original_size,
                    i.sender, i.ephemeral, i.pinned, i.is_template
             FROM issues i
             LEFT JOIN labels l ON i.id = l.issue_id
             WHERE i.deleted_at IS NULL",
        );
        let mut params = Vec::new();
        let mut param_idx = 1;
        if let Some(q) = query {
            sql.push_str(&format!(" AND (i.title LIKE ?{} OR i.description LIKE ?{})", param_idx, param_idx + 1));
            params.push(format!("%{}%", q));
            params.push(format!("%{}%", q));
            param_idx += 2;
        }
        for s in status {
            sql.push_str(&format!(" AND i.status = ?{}", param_idx));
            params.push(s.to_string());
            param_idx += 1;
        }
        for t in issue_type {
            sql.push_str(&format!(" AND i.issue_type = ?{}", param_idx));
            params.push(t.to_string());
            param_idx += 1;
        }
        if let Some(a) = assignee {
            sql.push_str(&format!(" AND i.assignee = ?{}", param_idx));
            params.push(a.to_string());
            param_idx += 1;
        }
        if !labels.is_empty() {
            let label_conditions: Vec<String> = labels.iter().enumerate().map(|(i, _)| format!("l.label = ?{}", param_idx + i)).collect();
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
        sql.push_str(" ORDER BY i.priority ASC, i.created_at ASC");
        if limit > 0 {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
        let mut rows = stmt.query(param_refs.as_slice())?;
        let mut issues = Vec::new();
        while let Some(row) = rows.next()? {
            issues.push(Self::row_to_issue_conn(&conn, row)?);
        }
        Ok(issues)
    }

    pub fn get_stats(&self) -> Result<Stats> {
        let conn = self.conn.lock().unwrap();
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM issues WHERE deleted_at IS NULL", [], |row| row.get(0))?;
        let open: i64 = conn.query_row("SELECT COUNT(*) FROM issues WHERE status = 'open' AND deleted_at IS NULL", [], |row| row.get(0))?;
        let in_progress: i64 = conn.query_row("SELECT COUNT(*) FROM issues WHERE status = 'in_progress' AND deleted_at IS NULL", [], |row| row.get(0))?;
        let closed: i64 = conn.query_row("SELECT COUNT(*) FROM issues WHERE status = 'closed' AND deleted_at IS NULL", [], |row| row.get(0))?;
        Ok(Stats { total: total as usize, open: open as usize, in_progress: in_progress as usize, closed: closed as usize })
    }

    /// Get the score of the top candidate bead for claiming.
    ///
    /// Returns None if no candidates are available.
    /// Used by claim_any() for cross-workspace scoring comparison.
    pub fn top_candidate_score(&self) -> Result<Option<crate::claim::Score>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT COALESCE(COUNT(d.issue_id), 0) as downstream_impact,
                    COALESCE(c.float, 999999) as critical_float,
                    i.priority,
                    strftime('%s', i.created_at) as created_ts
             FROM issues i
             LEFT JOIN dependencies d ON d.depends_on_id = i.id AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
             LEFT JOIN critical_path_cache c ON c.bead_id = i.id
             WHERE i.status = 'open'
               AND i.ephemeral = 0
               AND i.pinned = 0
               AND i.is_template = 0
               AND i.deleted_at IS NULL
               AND NOT EXISTS (
                   SELECT 1 FROM dependencies blocker_dep
                   INNER JOIN issues blocker ON blocker.id = blocker_dep.depends_on_id
                   WHERE blocker_dep.issue_id = i.id
                   AND blocker_dep.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
                   AND blocker.status != 'closed'
               )
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
            Ok(Some(crate::claim::Score {
                downstream_impact: row.get(0)?,
                critical_float: row.get(1)?,
                priority: row.get(2)?,
                created_at_ts: row.get(3)?,
            }))
        } else {
            Ok(None)
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
        conn.execute(
            "INSERT INTO worker_sessions (worker_id, model, harness, harness_version, bead_id, workspace_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                worker_id,
                model,
                harness,
                harness_version,
                bead_id,
                workspace_path,
            ],
        )?;
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

fn is_busy_error(e: &rusqlite::Error) -> bool {
    matches!(e, rusqlite::Error::SqliteFailure(rusqlite::ffi::Error { code: rusqlite::ErrorCode::DatabaseBusy, .. }, _))
}

fn parse_datetime(s: String) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(&s)?.with_timezone(&Utc))
}
