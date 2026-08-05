//! Doctor and repair operations for bead-forge.
//!
//! Provides health checking and recovery operations for bead databases,
//! including corruption detection and JSONL-based repair.

use crate::config::{find_beads_dir, load_metadata};
use crate::jsonl::{import_jsonl, stream_issues, UpsertResult};
use crate::model::Issue;
use crate::recovery;
use crate::storage::Storage;
use anyhow::{anyhow, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// Doctor check results.
#[derive(Debug, Clone, Default)]
pub struct DoctorResult {
    pub db_ok: bool,
    pub jsonl_ok: bool,
    pub jsonl_line_count: usize,
    pub db_issue_count: usize,
    pub issues: Vec<String>,
    // Drift tracking
    pub missing_in_jsonl: Vec<String>,
    pub missing_in_sqlite: Vec<String>,
    pub hash_mismatch: Vec<String>,
    // Unflushed bead tracking (db-only or db-newer beads that haven't been flushed to JSONL)
    pub unflushed_count: usize,
    // Beads with a custom status (e.g. "completed") that reads as done but isn't the
    // canonical "closed"/"tombstone" -- see bf-wre. Status::is_terminal() now treats
    // these as terminal for blocking/scheduling purposes, but they still display as
    // non-terminal everywhere else, so this check surfaces them for manual `bf close`.
    pub pseudo_terminal_status_ids: Vec<String>,
    // Beads stuck in 'blocked' status despite having no active blockers -- see bf-5id.
    // These beads should be 'open' but were never transitioned when their last blocker
    // closed, so this check surfaces them for `bf doctor --reconcile` (bf-29wxxl).
    pub stale_blocked_ids: Vec<String>,
    // Beads whose assignee is the literal empty string instead of NULL -- see bf-29wxxl.
    // `bf create`/`bf update` normalize this to NULL now (bf-4mj7l/bf-2uhsk), but rows
    // written before that fix still carry "", and any consumer testing `assignee is not
    // None` reads them as already-claimed. Repaired by `bf doctor --reconcile`.
    pub empty_assignee_ids: Vec<String>,
    // Rows that violate a NOT NULL column constraint (NULL stored in a column the
    // schema declares NOT NULL) -- see bf-3hm5h. The historical case is a NULL
    // created_at/updated_at that used to crash the entire list/flush with
    // InvalidColumnType, forcing the destructive `rm beads.db + reimport` workaround.
    // Reads are now NULL-tolerant, and this check surfaces the rows for `bf doctor
    // --fix-schema` to repair in place.
    pub null_not_null: Vec<NullNotNullViolation>,
}

/// A NOT NULL column that contains NULL values in one or more rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NullNotNullViolation {
    /// Table containing the offending column.
    pub table: String,
    /// Column declared NOT NULL that holds NULL values.
    pub column: String,
    /// Declared column type (e.g. "DATETIME", "TEXT", "INTEGER").
    pub decl_type: String,
    /// Number of rows with NULL in this column.
    pub count: usize,
}

/// Perform a health check on the bead database and JSONL file.
///
/// # Arguments
/// * `workspace_dir` - Path to the workspace root (contains .beads/)
///
/// # Returns
/// * `Ok(DoctorResult)` - Health check results
pub fn check(workspace_dir: &Path) -> Result<DoctorResult> {
    let beads_dir = find_beads_dir(workspace_dir)
        .ok_or_else(|| anyhow!("No .beads directory found in {}", workspace_dir.display()))?;
    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let jsonl_path = beads_dir.join(&metadata.jsonl_export);

    let mut issues = Vec::new();
    let mut db_ok = true;
    let mut jsonl_ok = true;
    let mut result = DoctorResult::default();

    // Check database
    let (db_issue_count, db_integrity_ok) = check_database(&db_path)?;
    result.db_issue_count = db_issue_count;
    if !db_integrity_ok {
        db_ok = false;
        issues.push("Database integrity check failed".to_string());
    }

    // Check JSONL file
    let (jsonl_line_count, jsonl_valid) = check_jsonl(&jsonl_path)?;
    result.jsonl_line_count = jsonl_line_count;
    if !jsonl_valid {
        jsonl_ok = false;
        issues.push("JSONL file contains invalid lines".to_string());
    }

    // Check consistency with content_hash comparison
    if db_ok && jsonl_ok {
        let drift = check_consistency_with_hash(&db_path, &jsonl_path)?;

        let total_drift = drift.missing_in_jsonl.len()
            + drift.missing_in_sqlite.len()
            + drift.hash_mismatch.len();

        result.missing_in_jsonl = drift.missing_in_jsonl.clone();
        result.missing_in_sqlite = drift.missing_in_sqlite.clone();
        result.hash_mismatch = drift.hash_mismatch.clone();

        if total_drift > 0 {
            issues.push(format!(
                "Drift detected: {} missing in JSONL, {} missing in SQLite, {} hash mismatch",
                result.missing_in_jsonl.len(),
                result.missing_in_sqlite.len(),
                result.hash_mismatch.len()
            ));
        }

        // Count unflushed beads
        let unflushed_count = count_unflushed(&db_path)?;
        result.unflushed_count = unflushed_count;

        if unflushed_count > 0 {
            issues.push(format!(
                "{} unflushed bead(s) exist (modified or created since last flush to JSONL)",
                unflushed_count
            ));
        }
    }

    // Check for beads stuck in a done-sounding custom status instead of closed/tombstone
    // (bf-wre). Independent of db_ok/jsonl_ok since it only needs the sqlite db to be
    // openable, not a full consistency check.
    let pseudo_terminal_ids = check_pseudo_terminal_statuses(&db_path)?;
    if !pseudo_terminal_ids.is_empty() {
        issues.push(format!(
            "{} bead(s) have a done-sounding status (e.g. \"completed\") instead of \"closed\" \
             -- run `bf close <id>` on each to fully close them: {}",
            pseudo_terminal_ids.len(),
            pseudo_terminal_ids.join(", ")
        ));
    }
    result.pseudo_terminal_status_ids = pseudo_terminal_ids;

    // Check for beads stuck in 'blocked' status despite having no active blockers
    // (bf-5id). These are stale 'blocked' beads that should be 'open' but were never
    // transitioned when their last blocker closed. Independent of db_ok/jsonl_ok since
    // it only needs the sqlite db to be openable.
    let stale_blocked_ids = check_stale_blocked_statuses(&db_path)?;
    if !stale_blocked_ids.is_empty() {
        issues.push(format!(
            "{} bead(s) stuck in 'blocked' status despite having no active blockers \
             -- run `bf doctor --reconcile` to flip them back to open: {}",
            stale_blocked_ids.len(),
            stale_blocked_ids.join(", ")
        ));
    }
    result.stale_blocked_ids = stale_blocked_ids;

    // Check for legacy empty-string assignees (bf-29wxxl). The CLI normalizes these to
    // NULL on write now, but rows that predate that fix were never backfilled and read
    // back as "assigned" to consumers that test for presence rather than emptiness.
    let empty_assignee_ids = check_empty_assignees(&db_path)?;
    if !empty_assignee_ids.is_empty() {
        issues.push(format!(
            "{} bead(s) have an empty-string assignee instead of NULL (reads back as \
             assigned and can hide the bead from claiming) -- run `bf doctor --reconcile` \
             to normalize them: {}",
            empty_assignee_ids.len(),
            empty_assignee_ids.join(", ")
        ));
    }
    result.empty_assignee_ids = empty_assignee_ids;

    // Check for NULL values stored in NOT NULL columns (bf-3hm5h). Independent of
    // db_ok/jsonl_ok since it only needs the sqlite db to be openable, and it is
    // precisely the kind of low-level corruption that other checks (which read
    // through the storage layer) would otherwise have crashed on.
    let null_not_null = check_null_not_null(&db_path)?;
    if !null_not_null.is_empty() {
        let total: usize = null_not_null.iter().map(|v| v.count).sum();
        let detail = null_not_null
            .iter()
            .map(|v| format!("{}.{} ({})", v.table, v.column, v.count))
            .collect::<Vec<_>>()
            .join(", ");
        issues.push(format!(
            "{} NULL value(s) in NOT NULL column(s) [{}] -- run `bf doctor --fix-schema` \
             to repair in place",
            total, detail
        ));
    }
    result.null_not_null = null_not_null;

    result.db_ok = db_ok;
    result.jsonl_ok = jsonl_ok;
    result.issues = issues;

    Ok(result)
}

/// Find beads whose status is 'blocked' but have no active (non-terminal) blockers.
/// These are stale 'blocked' beads that should be 'open' but were never transitioned
/// when their last blocker closed (bf-5id).
fn check_stale_blocked_statuses(db_path: &Path) -> Result<Vec<String>> {
    let conn = Connection::open(db_path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;

    // Find beads where:
    // 1. status = 'blocked'
    // 2. NOT deleted
    // 3. COUNT of active blockers (non-terminal status) = 0
    let mut stmt = conn.prepare(
        r#"
        SELECT i.id
        FROM issues i
        WHERE i.status = 'blocked'
        AND i.deleted_at IS NULL
        AND (
            SELECT COUNT(DISTINCT d.depends_on_id)
            FROM dependencies d
            INNER JOIN issues blocker ON blocker.id = d.depends_on_id
            WHERE d.issue_id = i.id
            AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
            AND blocker.status NOT IN ('closed', 'tombstone', 'done', 'completed')
        ) = 0
        ORDER BY i.id
        "#,
    )?;

    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(ids)
}

/// Find beads whose `assignee` is the literal empty string rather than NULL (bf-29wxxl).
///
/// `bf create`/`bf update` normalize an empty assignee to NULL (bf-4mj7l/bf-2uhsk), but
/// that fix was forward-only: rows written before it still store `""`, which any consumer
/// checking `assignee is not None` reads as "already claimed" (docs/plan/plan.md §3.4).
fn check_empty_assignees(db_path: &Path) -> Result<Vec<String>> {
    let conn = Connection::open(db_path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    let mut stmt = conn
        .prepare("SELECT id FROM issues WHERE assignee = '' AND deleted_at IS NULL ORDER BY id")?;
    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(ids)
}

/// Find beads whose status is a `TERMINAL_STATUS_ALIASES` value (e.g. "completed",
/// "done") rather than the canonical "closed"/"tombstone". These beads are functionally
/// terminal (see `Status::is_terminal()`) but weren't actually closed, so `bf show`/`bf
/// list` still displays them as active and `closed_at`/`close_reason` are never set.
fn check_pseudo_terminal_statuses(db_path: &Path) -> Result<Vec<String>> {
    let conn = Connection::open(db_path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    let placeholders = crate::model::TERMINAL_STATUS_ALIASES
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(", ");
    let query = format!(
        "SELECT id FROM issues WHERE LOWER(status) IN ({placeholders}) AND deleted_at IS NULL ORDER BY id"
    );
    let mut stmt = conn.prepare(&query)?;
    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(ids)
}

/// List the user tables in the database (excluding SQLite internal tables).
fn user_tables(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )?;
    let tables = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tables)
}

/// A NOT NULL column, discovered by introspecting the live schema.
struct NotNullColumn {
    table: String,
    column: String,
    decl_type: String,
}

/// Introspect every user table via `PRAGMA table_info` and return the columns the
/// schema declares NOT NULL. Driving off the live schema (rather than a hardcoded
/// list) keeps the detector and fixer correct as the schema evolves -- this is the
/// "generally" in "NULL-in-NOT-NULL rows generally" (bf-3hm5h).
fn not_null_columns(conn: &Connection) -> Result<Vec<NotNullColumn>> {
    let mut cols = Vec::new();
    for table in user_tables(conn)? {
        // PRAGMA table_info columns: cid, name, type, notnull, dflt_value, pk
        let mut stmt = conn.prepare(&format!(
            "PRAGMA table_info(\"{}\")",
            table.replace('"', "\"\"")
        ))?;
        let rows = stmt
            .query_map([], |row| {
                let name: String = row.get(1)?;
                let decl_type: String = row.get(2)?;
                let notnull: i64 = row.get(3)?;
                Ok((name, decl_type, notnull))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (name, decl_type, notnull) in rows {
            if notnull == 1 {
                cols.push(NotNullColumn {
                    table: table.clone(),
                    column: name,
                    decl_type,
                });
            }
        }
    }
    Ok(cols)
}

/// Quote a SQLite identifier (table or column name) for safe interpolation.
fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Detect rows storing NULL in a NOT NULL column (bf-3hm5h).
///
/// The schema forbids this, but corrupted databases have contained it -- most
/// notably a NULL `created_at`/`updated_at` that turned every `bf list`/`bf flush`
/// into a fatal `InvalidColumnType` crash. Reads are now NULL-tolerant; this
/// detector surfaces the underlying rows so `fix_null_not_null` can repair them.
fn check_null_not_null(db_path: &Path) -> Result<Vec<NullNotNullViolation>> {
    let conn = Connection::open(db_path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;

    let mut violations = Vec::new();
    for col in not_null_columns(&conn)? {
        // NOTE: test `typeof(col) = 'null'`, NOT `col IS NULL`. SQLite's query planner
        // assumes a NOT NULL column never holds NULL and folds `col IS NULL` to a
        // constant FALSE -- so `IS NULL` can *never* surface this corruption. `typeof()`
        // forces per-row evaluation and reports 'null' for a stored NULL regardless of
        // the column's declared constraint.
        let count: i64 = conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM {} WHERE typeof({}) = 'null'",
                quote_ident(&col.table),
                quote_ident(&col.column)
            ),
            [],
            |row| row.get(0),
        )?;
        if count > 0 {
            violations.push(NullNotNullViolation {
                table: col.table,
                column: col.column,
                decl_type: col.decl_type,
                count: count as usize,
            });
        }
    }
    Ok(violations)
}

/// Repair rows that store NULL in a NOT NULL column, in place (bf-3hm5h).
///
/// This is a non-destructive fixer -- unlike `repair`, it does not rebuild from
/// JSONL, so it works even when JSONL is absent or itself suspect, and it never
/// touches valid rows. Each NULL is replaced with a type-appropriate sentinel:
///
/// * `DATETIME` columns -> the Unix epoch (`1970-01-01T00:00:00+00:00`), matching
///   the value NULL-tolerant reads already substitute, so a fixed row round-trips
///   identically to how it was already being displayed.
/// * `INTEGER`/`REAL`/numeric columns -> `0`.
/// * everything else (TEXT/BLOB) -> the empty string, matching the schema's
///   `NOT NULL DEFAULT ''` convention for text columns.
///
/// Returns the total number of column values updated.
pub fn fix_null_not_null(workspace_dir: &Path) -> Result<usize> {
    let beads_dir = find_beads_dir(workspace_dir)
        .ok_or_else(|| anyhow!("No .beads directory found in {}", workspace_dir.display()))?;
    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);

    let violations = check_null_not_null(&db_path)?;
    if violations.is_empty() {
        return Ok(0);
    }

    let conn = Connection::open(&db_path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;

    let mut fixed = 0usize;
    for v in &violations {
        let upper = v.decl_type.to_uppercase();
        // SQLite type affinity: DATE/TIME columns first (they must not be caught by
        // the INT affinity rule below), then numeric, then everything else.
        let replacement = if upper.contains("DATE") || upper.contains("TIME") {
            "'1970-01-01T00:00:00+00:00'".to_string()
        } else if upper.contains("INT")
            || upper.contains("REAL")
            || upper.contains("FLOA")
            || upper.contains("DOUB")
            || upper.contains("NUM")
            || upper.contains("DEC")
        {
            "0".to_string()
        } else {
            "''".to_string()
        };

        // `typeof(col) = 'null'`, NOT `col IS NULL`: the planner folds `IS NULL` on a
        // NOT NULL column to FALSE, so an `IS NULL` UPDATE would silently match zero
        // rows and "repair" nothing. See check_null_not_null for the full rationale.
        let updated = conn.execute(
            &format!(
                "UPDATE {} SET {} = {} WHERE typeof({}) = 'null'",
                quote_ident(&v.table),
                quote_ident(&v.column),
                replacement,
                quote_ident(&v.column)
            ),
            [],
        )?;
        fixed += updated;
    }

    Ok(fixed)
}

/// Structured outcome of a `reconcile` run (bf-29wxxl).
#[derive(Debug, Clone, Default)]
pub struct ReconcileReport {
    /// Beads flipped from `blocked` back to `open` because every blocking dependency
    /// they carry is already terminal.
    pub unblocked: Vec<String>,
    /// Beads whose empty-string assignee was rewritten to NULL.
    pub normalized_assignees: Vec<String>,
    /// Beads at status `blocked` that carry no blocking dependency edge at all. These
    /// were set blocked by hand, so there is nothing to reconcile them against and
    /// `reconcile` deliberately leaves them alone -- they are reported for review.
    pub blocked_without_dependencies: Vec<String>,
}

impl ReconcileReport {
    /// True when nothing needed fixing (beads left for manual review don't count).
    pub fn is_clean(&self) -> bool {
        self.unblocked.is_empty() && self.normalized_assignees.is_empty()
    }
}

/// Backfill data that predates a forward-only fix (bf-29wxxl).
///
/// Two code fixes shipped without ever reconciling the rows that predated them, and both
/// starve `bf ready`:
///
/// 1. The blocked->open cascade (bf-5id) only fires on future `close` events. A bead whose
///    last blocker closed *before* that fix landed stayed at `status='blocked'` forever,
///    transitively pinning everything downstream of it.
/// 2. Empty-assignee normalization (bf-4mj7l/bf-2uhsk) only applies on write. Rows already
///    holding `assignee = ''` read back as assigned (docs/plan/plan.md §3.4).
///
/// This is a non-destructive, in-place fixer -- like `fix_null_not_null` it never rebuilds
/// from JSONL and never touches rows that are already correct. Repaired rows are marked
/// dirty so the next flush carries them into JSONL, and the blocked cache is rebuilt.
///
/// Only beads that actually carry a blocking dependency edge are unblocked: a bead set to
/// `blocked` by hand with no dependencies has no blocker state to derive `open` from, so it
/// is reported in `blocked_without_dependencies` instead of being silently reopened.
///
/// Returns the ids touched in each pass.
pub fn reconcile(workspace_dir: &Path) -> Result<ReconcileReport> {
    let beads_dir = find_beads_dir(workspace_dir)
        .ok_or_else(|| anyhow!("No .beads directory found in {}", workspace_dir.display()))?;
    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);

    let storage = Storage::open(&db_path)?;

    let report = storage.with_immediate_transaction(|tx| {
        let now = chrono::Utc::now().to_rfc3339();
        let mut report = ReconcileReport::default();

        // Pass 1 -- stale blocked rows whose every blocking dependency is terminal.
        // Mirrors the cascade condition in Storage::close_issue so a reconciled row is
        // indistinguishable from one the cascade would have produced at close time.
        let stale: Vec<String> = tx
            .prepare(
                r#"
                SELECT i.id
                FROM issues i
                WHERE i.status = 'blocked'
                AND i.deleted_at IS NULL
                AND EXISTS (
                    SELECT 1 FROM dependencies d
                    WHERE d.issue_id = i.id
                    AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
                )
                AND NOT EXISTS (
                    SELECT 1 FROM dependencies d
                    INNER JOIN issues blocker ON blocker.id = d.depends_on_id
                    WHERE d.issue_id = i.id
                    AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
                    AND blocker.status NOT IN ('closed', 'tombstone', 'done', 'completed')
                )
                ORDER BY i.id
                "#,
            )?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        for id in &stale {
            tx.execute(
                "UPDATE issues SET status = 'open', updated_at = ?1 WHERE id = ?2",
                rusqlite::params![&now, id],
            )?;
            tx.execute(
                "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, created_at) \
                 VALUES (?1, 'status_changed', 'doctor-reconcile', 'blocked', 'open', ?2)",
                rusqlite::params![id, &now],
            )?;
            tx.execute(
                "INSERT OR REPLACE INTO dirty_issues (issue_id, marked_at) VALUES (?1, ?2)",
                rusqlite::params![id, &now],
            )?;
        }
        report.unblocked = stale;

        // Blocked beads with no blocking edge at all -- reported, never rewritten.
        report.blocked_without_dependencies = tx
            .prepare(
                r#"
                SELECT i.id
                FROM issues i
                WHERE i.status = 'blocked'
                AND i.deleted_at IS NULL
                AND NOT EXISTS (
                    SELECT 1 FROM dependencies d
                    WHERE d.issue_id = i.id
                    AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
                )
                ORDER BY i.id
                "#,
            )?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        // Pass 2 -- legacy empty-string assignees.
        let empty: Vec<String> = tx
            .prepare("SELECT id FROM issues WHERE assignee = '' AND deleted_at IS NULL ORDER BY id")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        for id in &empty {
            tx.execute(
                "UPDATE issues SET assignee = NULL, updated_at = ?1 WHERE id = ?2",
                rusqlite::params![&now, id],
            )?;
            tx.execute(
                "INSERT INTO events (issue_id, event_type, actor, old_value, new_value, created_at) \
                 VALUES (?1, 'assignee_changed', 'doctor-reconcile', '', NULL, ?2)",
                rusqlite::params![id, &now],
            )?;
            tx.execute(
                "INSERT OR REPLACE INTO dirty_issues (issue_id, marked_at) VALUES (?1, ?2)",
                rusqlite::params![id, &now],
            )?;
        }
        report.normalized_assignees = empty;

        Ok::<_, anyhow::Error>(report)
    })?;

    // Reopened beads no longer belong in the display-facing blocked cache. Cheap and
    // idempotent, so run it whenever anything moved.
    if !report.unblocked.is_empty() {
        storage.rebuild_blocked_cache()?;
    }

    Ok(report)
}

/// Check database integrity.
fn check_database(db_path: &Path) -> Result<(usize, bool)> {
    let conn = Connection::open(db_path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;

    // Apply schema if database is new (no tables yet)
    let needs_schema: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='issues'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|n| n == 0)
        .unwrap_or(true);
    if needs_schema {
        crate::storage::schema::apply_schema(&conn)?;
    }

    // Run PRAGMA integrity_check
    let integrity_result: String =
        conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;

    let integrity_ok = integrity_result == "ok";

    // Count issues
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM issues WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    )?;

    Ok((count as usize, integrity_ok))
}

/// Check JSONL file validity.
fn check_jsonl(jsonl_path: &Path) -> Result<(usize, bool)> {
    if !jsonl_path.exists() {
        return Ok((0, true)); // Empty workspace is valid
    }

    let mut count = 0;
    let mut valid = true;

    match stream_issues(jsonl_path) {
        Ok(iter) => {
            for result in iter {
                match result {
                    Ok(_) => count += 1,
                    Err(e) => {
                        valid = false;
                        eprintln!("Invalid JSONL line: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            return Err(anyhow!("Failed to read JSONL file: {}", e));
        }
    }

    Ok((count, valid))
}

/// Consistency drift details.
#[derive(Debug, Clone, Default)]
struct ConsistencyDrift {
    /// Bead IDs that exist in SQLite but not in JSONL
    missing_in_jsonl: Vec<String>,
    /// Bead IDs that exist in JSONL but not in SQLite
    missing_in_sqlite: Vec<String>,
    /// Bead IDs where content_hash differs between JSONL and SQLite
    hash_mismatch: Vec<String>,
}

/// Check consistency between database and JSONL using content_hash.
///
/// Compares each bead's content_hash between JSONL and SQLite to detect drift.
fn check_consistency_with_hash(db_path: &Path, jsonl_path: &Path) -> Result<ConsistencyDrift> {
    use std::collections::{HashMap, HashSet};

    let storage = Storage::open(db_path)?;

    // Build a map of all SQLite issues with their content_hash
    let sqlite_issues: HashMap<String, String> = storage
        .list_all_issues()?
        .into_iter()
        .map(|issue| {
            let hash = issue.content_hash();
            (issue.id, hash)
        })
        .collect();

    let mut drift = ConsistencyDrift::default();
    let mut jsonl_seen: HashSet<String> = HashSet::new();

    // Stream JSONL and compare, tracking seen IDs
    if jsonl_path.exists() {
        let iter = stream_issues(jsonl_path)?;
        for result in iter {
            let jsonl_issue = result?;
            let jsonl_hash = jsonl_issue.content_hash();
            let bead_id = jsonl_issue.id.clone();
            jsonl_seen.insert(bead_id.clone());

            match sqlite_issues.get(&bead_id) {
                Some(sqlite_hash) => {
                    if sqlite_hash != &jsonl_hash {
                        drift.hash_mismatch.push(bead_id);
                    }
                }
                None => {
                    drift.missing_in_sqlite.push(bead_id);
                }
            }
        }
    }

    // Find beads in SQLite but not in JSONL (using tracked seen IDs)
    for bead_id in sqlite_issues.keys() {
        if !jsonl_seen.contains(bead_id) {
            drift.missing_in_jsonl.push(bead_id.clone());
        }
    }

    Ok(drift)
}

/// Count unflushed beads (db-only or db-newer beads not yet flushed to JSONL).
///
/// Returns the count of beads that exist in SQLite but have been marked as dirty
/// (modified or created since the last flush to JSONL).
pub fn count_unflushed(db_path: &Path) -> Result<usize> {
    use rusqlite::Connection;

    let conn = Connection::open(db_path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;

    // Check if dirty_issues table exists
    let table_exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='dirty_issues'",
        [],
        |row| row.get(0),
    )?;

    if table_exists == 0 {
        return Ok(0);
    }

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM dirty_issues", [], |row| row.get(0))?;

    Ok(count as usize)
}

/// Get IDs of unflushed beads.
///
/// Returns a list of bead IDs that exist in SQLite but have been marked as dirty
/// (modified or created since the last flush to JSONL).
fn get_unflushed_ids(db_path: &Path) -> Result<Vec<String>> {
    use rusqlite::Connection;

    let conn = Connection::open(db_path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;

    // Check if dirty_issues table exists
    let table_exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='dirty_issues'",
        [],
        |row| row.get(0),
    )?;

    if table_exists == 0 {
        return Ok(Vec::new());
    }

    let mut stmt = conn.prepare("SELECT issue_id FROM dirty_issues ORDER BY issue_id")?;
    let mut rows = stmt.query([])?;

    let mut ids = Vec::new();
    while let Some(row) = rows.next()? {
        ids.push(row.get(0)?);
    }

    Ok(ids)
}

/// Repair the database by rebuilding from JSONL.
///
/// This is the recovery operation when the database is corrupted or missing.
/// The JSONL file is the authoritative source of truth for repair.
///
/// # Arguments
/// * `workspace_dir` - Path to the workspace root (contains .beads/)
/// * `flush_first` - If true, flush unflushed beads to JSONL before repairing
/// * `force` - If true, proceed even with unflushed beads (they will be lost)
///
/// # Returns
/// * `Ok(usize)` - Number of beads imported
pub fn repair(workspace_dir: &Path, flush_first: bool, force: bool) -> Result<usize> {
    let beads_dir = find_beads_dir(workspace_dir)
        .ok_or_else(|| anyhow!("No .beads directory found in {}", workspace_dir.display()))?;
    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let jsonl_path = beads_dir.join(&metadata.jsonl_export);

    // Check if JSONL exists
    if !jsonl_path.exists() {
        return Err(anyhow!(
            "Cannot repair: JSONL file not found at {}",
            jsonl_path.display()
        ));
    }

    // Check for unflushed beads if database exists and is valid
    // If db is corrupted, we can't detect unflushed beads - proceed with warning
    let (unflushed_ids, db_corrupted) = if db_path.exists() {
        match get_unflushed_ids(&db_path) {
            Ok(ids) => (ids, false),
            Err(_) => {
                // Database is corrupted or unreadable
                // We can't detect unflushed beads, so proceed with a warning
                if flush_first {
                    // Cannot flush from a corrupt database
                    return Err(anyhow!(
                        "Cannot flush: database is corrupted and unreadable.\n\
                         Flushing from a corrupt DB would poison the JSONL checkpoint.\n\
                         Unflushed beads cannot be recovered.\n\
                         Remove --flush-first to proceed with repair only."
                    ));
                }
                // Proceed without unflushed check (db is unreadable)
                // No --force needed since we can't detect unflushed beads anyway
                eprintln!("WARNING: Database is corrupted and unreadable.");
                eprintln!("  Cannot detect unflushed beads - any db-only beads will be lost.");
                eprintln!("  Proceeding with repair from JSONL...");
                (Vec::new(), true)
            }
        }
    } else {
        // DB doesn't exist - no unflushed beads possible
        (Vec::new(), false)
    };

    if !unflushed_ids.is_empty() {
        if flush_first {
            // Flush ALL beads to JSONL first (not just dirty ones)
            // We must export everything because repair will rebuild db from JSONL
            eprintln!(
                "Flushing all beads to JSONL before repair (including {} unflushed)...",
                unflushed_ids.len()
            );
            let storage = Storage::open(&db_path)?;
            let flushed = storage.sync_to_jsonl(&jsonl_path, false)?;
            eprintln!("Flushed {} bead(s) to JSONL", flushed);
        } else if !force {
            // Refuse to proceed with unflushed beads
            return Err(anyhow!(
                "Cannot repair: {} unflushed bead(s) exist ({}).\n\
                 Run 'bf doctor --repair --flush-first' to flush before repair,\n\
                 or 'bf doctor --repair --force' to proceed (these beads will be LOST).\n\
                 Unflushed beads: {}",
                unflushed_ids.len(),
                if unflushed_ids.len() <= 5 {
                    unflushed_ids.iter().cloned().collect::<Vec<_>>().join(", ")
                } else {
                    format!(
                        "{}, ... and {} more",
                        unflushed_ids[..5].join(", "),
                        unflushed_ids.len() - 5
                    )
                },
                unflushed_ids.join(", ")
            ));
        } else {
            // Force mode: warn but proceed
            eprintln!(
                "WARNING: {} unflushed bead(s) will be LOST: {}",
                unflushed_ids.len(),
                if unflushed_ids.len() <= 5 {
                    unflushed_ids.iter().cloned().collect::<Vec<_>>().join(", ")
                } else {
                    format!(
                        "{}, ... and {} more",
                        unflushed_ids[..5].join(", "),
                        unflushed_ids.len() - 5
                    )
                }
            );
            eprintln!("Proceeding with repair due to --force flag");
        }
    }

    // Backup existing database if it exists
    if db_path.exists() {
        let backup_path = db_path.with_extension(&format!(
            "db.backup.{}",
            chrono::Utc::now().format("%Y%m%d%H%M%S")
        ));
        std::fs::copy(&db_path, &backup_path)?;
        eprintln!("Backed up existing database to {}", backup_path.display());
    }

    // Delete old database
    if db_path.exists() {
        std::fs::remove_file(&db_path)?;
    }

    // Create new database and import JSONL
    let storage = Storage::open(&db_path)?;

    let result = storage.sync_from_jsonl(&jsonl_path)?;

    // Rebuild blocked cache
    storage.rebuild_blocked_cache()?;

    // Clear dirty marks - after rebuild from JSONL, db and JSONL are in sync
    // (create_issue_tx doesn't mark dirty, but we clear for safety)
    storage.clear_dirty()?;

    Ok(result.imported)
}

// ===========================================================================
// Doctor safety stack (Phase 7.2 — layered repair + beads_rust#394 preservation)
//
// `repair_stack` is the user-facing `bf doctor --repair` path. Unlike the
// low-level `repair` primitive above (which unconditionally rebuilds from JSONL),
// it walks a layered safety architecture and only reaches the destructive JSONL
// rebuild as a last resort, always behind a verified restorable backup:
//
//   1. Healthy early-return  — a workspace with no corruption/divergence never
//      rebuilds; only trivial, non-destructive fixers run.
//   2. Local repair first    — integrity triage + VACUUM/REINDEX/cache rebuild;
//      if that resolves the problem, the JSONL rebuild is never reached.
//   3. Verified backups      — SHA-256-hashed copies of the DB family + JSONL into
//      `.beads/recovery/<run-id>/` before any rebuild; restore-on-failure.
//   4. JSONL authority preflight — refuse rebuild if the JSONL carries merge
//      conflict markers or unparseable records (never rebuild from a poisoned
//      authority).
//   5. Repeat-failure gate   — a rebuild that fails post-verification writes a
//      marker; further rebuilds refuse without `--allow-repeated-repair`.
//   6. Preservation across rebuild (beads_rust#394) — snapshot unflushed dirty
//      issues (with labels/deps/comments) *and* tombstones before the rebuild,
//      restore + re-mark them dirty after, and report the preserved count.
// ===========================================================================

/// Options controlling a layered `repair_stack` run.
#[derive(Debug, Clone, Default)]
pub struct RepairOptions {
    /// Flush unflushed (dirty) beads to JSONL before rebuild.
    pub flush_first: bool,
    /// Discard unflushed dirty beads instead of preserving them across a rebuild.
    /// Preservation is the default; `force` opts out (the legacy destructive path).
    pub force: bool,
    /// Proceed even if a prior rebuild left a repeat-failure marker (layer 5).
    pub allow_repeated_repair: bool,
}

/// Structured outcome of a layered `repair_stack` run.
#[derive(Debug, Clone, Default)]
pub struct RepairReport {
    /// True when the workspace was healthy (or locally repairable) and no JSONL
    /// rebuild was performed.
    pub healthy: bool,
    /// True when a JSONL rebuild was performed.
    pub rebuilt: bool,
    /// Beads imported from JSONL during a rebuild (0 if no rebuild).
    pub imported: usize,
    /// Unflushed dirty beads snapshotted and restored across the rebuild (layer 6).
    pub preserved_dirty: usize,
    /// Non-destructive local fixers that were applied (layer 2).
    pub local_fixes: Vec<String>,
    /// Run id of the verified pre-rebuild backup (layer 3), if a rebuild ran.
    pub backup_run_id: Option<String>,
    /// Human-readable notes for the CLI summary.
    pub messages: Vec<String>,
}

/// Resolve the `.beads` dir + db/jsonl paths for a workspace.
fn resolve_paths(workspace_dir: &Path) -> Result<(PathBuf, PathBuf, PathBuf)> {
    let beads_dir = find_beads_dir(workspace_dir)
        .ok_or_else(|| anyhow!("No .beads directory found in {}", workspace_dir.display()))?;
    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let jsonl_path = beads_dir.join(&metadata.jsonl_export);
    Ok((beads_dir, db_path, jsonl_path))
}

/// The DB family + JSONL authority paths that a pre-rebuild backup captures.
fn backup_targets(db_path: &Path, jsonl_path: &Path) -> Vec<PathBuf> {
    let db_str = db_path.to_string_lossy().into_owned();
    vec![
        db_path.to_path_buf(),
        PathBuf::from(format!("{}-wal", db_str)),
        PathBuf::from(format!("{}-shm", db_str)),
        jsonl_path.to_path_buf(),
    ]
}

/// Layer 4 — JSONL authority preflight.
///
/// Refuse to treat the JSONL as authoritative for a rebuild if it contains git
/// merge-conflict markers or any unparseable record. Rebuilding from a poisoned
/// authority is exactly how a bad flush or a botched merge becomes permanent.
pub fn preflight_jsonl(jsonl_path: &Path) -> Result<()> {
    if !jsonl_path.exists() {
        return Ok(()); // A fresh/empty workspace has nothing to poison.
    }

    // Scan raw lines for conflict markers first — these produce the clearest error.
    let content = std::fs::read_to_string(jsonl_path)
        .map_err(|e| anyhow!("Cannot read JSONL for preflight: {}", e))?;
    for (idx, line) in content.lines().enumerate() {
        if line.starts_with("<<<<<<<")
            || line.starts_with(">>>>>>>")
            || line == "======="
            || line.starts_with("======= ")
            || line.starts_with("|||||||")
        {
            return Err(anyhow!(
                "Refusing to rebuild: JSONL contains a git merge-conflict marker at line {} \
                 (\"{}\"). Resolve the conflict in {} before repairing — rebuilding from a \
                 conflicted authority would make the corruption permanent.",
                idx + 1,
                line.chars().take(20).collect::<String>(),
                jsonl_path.display()
            ));
        }
    }

    // Then confirm every record parses. A single invalid record means the JSONL is
    // not a trustworthy authority for a full rebuild.
    let mut invalid = Vec::new();
    for (idx, result) in stream_issues(jsonl_path)?.enumerate() {
        if let Err(e) = result {
            invalid.push(format!("line {}: {}", idx + 1, e));
            if invalid.len() >= 5 {
                break;
            }
        }
    }
    if !invalid.is_empty() {
        return Err(anyhow!(
            "Refusing to rebuild: JSONL contains {} invalid record(s) [{}]. \
             Fix or remove the bad lines in {} before repairing.",
            invalid.len(),
            invalid.join("; "),
            jsonl_path.display()
        ));
    }

    Ok(())
}

/// Layer 2 — non-destructive local repair.
///
/// Runs integrity triage and cheap local recovery (VACUUM, REINDEX, blocked-cache
/// rebuild). Returns the list of fixers applied. Never touches JSONL and never
/// drops data; safe to run on any openable database.
fn local_repair(db_path: &Path) -> Result<Vec<String>> {
    let mut applied = Vec::new();
    if !db_path.exists() {
        return Ok(applied);
    }

    {
        let conn = Connection::open(db_path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        // VACUUM compacts and can clear certain free-list inconsistencies.
        conn.execute_batch("VACUUM")?;
        applied.push("VACUUM".to_string());
        // REINDEX rebuilds every index from the (authoritative) table rows.
        conn.execute_batch("REINDEX")?;
        applied.push("REINDEX".to_string());
    }

    // Rebuild the derived blocked-issues cache from the live graph.
    let storage = Storage::open(db_path)?;
    storage.rebuild_blocked_cache()?;
    applied.push("rebuild-blocked-cache".to_string());

    Ok(applied)
}

/// Core rebuild used by the safety stack: replace the DB by importing from JSONL.
///
/// Unlike the `repair` primitive, this performs no dirty-bead guard and writes no
/// ad-hoc `.db.backup.*` file — the layered caller has already taken a verified
/// recovery backup and captured any dirty state to restore afterwards.
fn rebuild_db_from_jsonl(db_path: &Path, jsonl_path: &Path) -> Result<usize> {
    if db_path.exists() {
        std::fs::remove_file(db_path)?;
    }
    // Remove stale SQLite sidecars so the fresh DB starts clean.
    for sidecar in ["-wal", "-shm"] {
        let p = PathBuf::from(format!("{}{}", db_path.to_string_lossy(), sidecar));
        if p.exists() {
            let _ = std::fs::remove_file(&p);
        }
    }

    let storage = Storage::open(db_path)?;
    let result = storage.sync_from_jsonl(jsonl_path)?;
    storage.rebuild_blocked_cache()?;
    storage.clear_dirty()?;
    Ok(result.imported)
}

/// Layer 6 (restore half) — re-insert snapshotted dirty beads after a rebuild and
/// re-mark them dirty. Beads already reconstituted from JSONL are replaced with the
/// (newer, unflushed) snapshot version so their in-flight edits survive.
///
/// Returns the number of beads restored.
fn restore_dirty_snapshot(db_path: &Path, snapshot: &[Issue]) -> Result<usize> {
    if snapshot.is_empty() {
        return Ok(0);
    }
    let storage = Storage::open(db_path)?;
    let now = chrono::Utc::now().to_rfc3339();
    storage.with_immediate_transaction(|tx| {
        for issue in snapshot {
            if Storage::get_issue_tx(tx, &issue.id)?.is_some() {
                // JSONL carried an older copy — overwrite with the unflushed snapshot.
                Storage::update_issue_from_json_tx(tx, issue)?;
            } else {
                Storage::create_issue_tx(tx, issue)?;
            }
            // Re-mark dirty: these beads are still unflushed relative to JSONL.
            tx.execute(
                "INSERT OR REPLACE INTO dirty_issues (issue_id, marked_at) VALUES (?1, ?2)",
                rusqlite::params![issue.id, now],
            )?;
        }
        Ok::<_, anyhow::Error>(())
    })?;
    Ok(snapshot.len())
}

/// Layered, safe `bf doctor --repair` (Phase 7.2).
///
/// Walks the six-layer safety stack described above. See [`RepairOptions`] /
/// [`RepairReport`].
pub fn repair_stack(workspace_dir: &Path, opts: &RepairOptions) -> Result<RepairReport> {
    let (beads_dir, db_path, jsonl_path) = resolve_paths(workspace_dir)?;
    let mut report = RepairReport::default();

    if !jsonl_path.exists() {
        return Err(anyhow!(
            "Cannot repair: JSONL file not found at {}",
            jsonl_path.display()
        ));
    }

    // ---- Layer 5: repeat-failure gate (checked before doing any work) ----
    if recovery::repair_failed_marker_exists(&beads_dir) && !opts.allow_repeated_repair {
        return Err(anyhow!(
            "Refusing to repair: a previous rebuild failed post-verification and its backup was \
             restored (see {}/{}). Investigate before retrying, then pass \
             --allow-repeated-repair to override.",
            recovery::RECOVERY_DIR,
            "repair-failed.marker"
        ));
    }

    // ---- Layer 1: health assessment ----
    let health = check(workspace_dir)?;

    // A rebuild is only justified by genuine corruption or JSONL↔DB divergence.
    // Unflushed (db-only) beads are NOT a rebuild trigger — that is a flush concern,
    // and rebuilding for them would risk exactly the data we must preserve. Invalid
    // JSONL counts as unhealthy so the flow reaches the layer-4 preflight, which then
    // refuses (never rebuild from a poisoned authority) rather than silently passing.
    let needs_rebuild = !health.db_ok
        || !health.jsonl_ok
        || !health.missing_in_sqlite.is_empty()
        || !health.hash_mismatch.is_empty();

    // ---- Layer 2: local repair first (always safe, never drops data) ----
    if db_path.exists() {
        match local_repair(&db_path) {
            Ok(fixes) => report.local_fixes = fixes,
            Err(e) => report
                .messages
                .push(format!("Local repair partially failed: {}", e)),
        }
    }

    // Re-assess after local repair: it may have resolved integrity issues, making a
    // rebuild unnecessary.
    let post_local = check(workspace_dir)?;
    let still_needs_rebuild = !post_local.db_ok
        || !post_local.jsonl_ok
        || !post_local.missing_in_sqlite.is_empty()
        || !post_local.hash_mismatch.is_empty();

    if !needs_rebuild || !still_needs_rebuild {
        // Healthy (or locally repaired) — the JSONL rebuild is unreachable from here.
        // A repair that repairs nothing must not write: honor the read-only contract.
        // `--flush-first` is scoped to the rebuild ("flush unflushed beads *before*
        // repair"); with no rebuild pending there is nothing to protect, so this branch
        // never flushes regardless of the flag. If unflushed beads are present, point
        // the user at the canonical checkpoint command (`bf sync --flush-only`) rather
        // than silently writing the JSONL checkpoint (bf-ku8hv).
        report.healthy = true;
        // A clean state clears any prior repeat-failure marker.
        let _ = recovery::clear_repair_failed_marker(&beads_dir);
        if post_local.unflushed_count > 0 {
            report.messages.push(format!(
                "{} unflushed bead(s) present; run `bf sync --flush-only` to checkpoint them",
                post_local.unflushed_count
            ));
        }
        return Ok(report);
    }

    // ---- Layer 4: JSONL authority preflight ----
    preflight_jsonl(&jsonl_path)?;

    // ---- Optional pre-flush, then Layer 6 snapshot ----
    if opts.flush_first && db_path.exists() {
        // Only possible if the DB is readable; skip silently on a corrupt DB.
        if let Ok(storage) = Storage::open(&db_path) {
            if let Ok(flushed) = storage.sync_to_jsonl(&jsonl_path, false) {
                report.messages.push(format!(
                    "Flushed {} bead(s) to JSONL before rebuild",
                    flushed
                ));
            }
        }
    }

    // Snapshot unflushed dirty beads (with labels/deps/comments and any dirty
    // tombstones) unless the caller explicitly opted into the destructive path.
    let dirty_snapshot: Vec<Issue> = if opts.force {
        Vec::new()
    } else if db_path.exists() {
        Storage::open(&db_path)
            .and_then(|s| s.list_dirty_issues())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    // ---- Layer 3: verified pre-rebuild backup ----
    let manifest = recovery::create_backup(
        &beads_dir,
        &backup_targets(&db_path, &jsonl_path),
        "pre-rebuild",
    )?;
    report.backup_run_id = Some(manifest.run_id.clone());

    // ---- Rebuild from JSONL ----
    let imported = rebuild_db_from_jsonl(&db_path, &jsonl_path)?;
    report.rebuilt = true;
    report.imported = imported;

    // ---- Layer 6: restore preserved dirty beads ----
    report.preserved_dirty = restore_dirty_snapshot(&db_path, &dirty_snapshot)?;

    // ---- Post-verification: did the rebuild produce a sound workspace? ----
    let verify = check(workspace_dir)?;
    // Preserved dirty beads legitimately show up as db-only (missing_in_jsonl); that
    // is the expected unflushed state and must not count as a verification failure.
    let verify_ok = verify.db_ok
        && verify.jsonl_ok
        && verify.missing_in_sqlite.is_empty()
        && verify.hash_mismatch.is_empty();

    if !verify_ok {
        // Roll back to the verified backup and raise the repeat-failure gate.
        let detail = format!(
            "post-verify failed (db_ok={}, jsonl_ok={}, missing_in_sqlite={}, hash_mismatch={}); \
             restored backup run {}",
            verify.db_ok,
            verify.jsonl_ok,
            verify.missing_in_sqlite.len(),
            verify.hash_mismatch.len(),
            manifest.run_id
        );
        recovery::restore_run(&beads_dir, &manifest.run_id)?;
        recovery::write_repair_failed_marker(&beads_dir, &detail)?;
        return Err(anyhow!(
            "Rebuild failed post-verification; restored the pre-rebuild backup (run {}). {}. \
             Further rebuilds will refuse without --allow-repeated-repair.",
            manifest.run_id,
            detail
        ));
    }

    // Success — clear any prior failure marker.
    recovery::clear_repair_failed_marker(&beads_dir)?;
    report.messages.push(format!(
        "Rebuilt from JSONL: {} imported, {} unflushed bead(s) preserved; verified backup at {}/{}",
        report.imported,
        report.preserved_dirty,
        recovery::RECOVERY_DIR,
        manifest.run_id
    ));

    Ok(report)
}

/// Rebuild the blocked issues cache.
///
/// This materialized view should be rebuilt after dependency or status changes.
pub fn rebuild_cache(workspace_dir: &Path) -> Result<()> {
    let beads_dir = find_beads_dir(workspace_dir)
        .ok_or_else(|| anyhow!("No .beads directory found in {}", workspace_dir.display()))?;
    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);

    let storage = Storage::open(&db_path)?;
    storage.rebuild_blocked_cache()?;

    Ok(())
}

/// Reclaim stale in_progress beads.
///
/// Resets beads that have been in_progress for longer than the TTL
/// back to open status.
///
/// # Arguments
/// * `workspace_dir` - Path to the workspace root (contains .beads/)
/// * `ttl_minutes` - TTL in minutes (default from config is 30)
///
/// # Returns
/// * `Ok(usize)` - Number of beads reclaimed
pub fn reclaim_stale(workspace_dir: &Path, ttl_minutes: i64) -> Result<usize> {
    let beads_dir = find_beads_dir(workspace_dir)
        .ok_or_else(|| anyhow!("No .beads directory found in {}", workspace_dir.display()))?;
    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);

    let storage = Storage::open(&db_path)?;

    let reclaimed = storage.with_immediate_transaction(|tx| {
        let stale_cutoff = chrono::Utc::now() - chrono::Duration::minutes(ttl_minutes);

        let reclaimed = tx.execute(
            "UPDATE issues
             SET status = 'open', assignee = NULL, updated_at = ?
             WHERE status = 'in_progress' AND updated_at < ?",
            rusqlite::params![chrono::Utc::now().to_rfc3339(), stale_cutoff.to_rfc3339()],
        )?;

        Ok::<_, anyhow::Error>(reclaimed)
    })?;

    Ok(reclaimed)
}

/// Initialize a new database from an existing JSONL file.
///
/// Use this to create a fresh database from a JSONL export without
/// affecting the existing database.
///
/// # Arguments
/// * `workspace_dir` - Path to the workspace root (contains .beads/)
/// * `jsonl_path` - Path to the JSONL file to import from
///
/// # Returns
/// * `Ok(usize)` - Number of beads imported
pub fn init_from_jsonl(workspace_dir: &Path, jsonl_path: &Path) -> Result<usize> {
    let beads_dir = find_beads_dir(workspace_dir)
        .ok_or_else(|| anyhow!("No .beads directory found in {}", workspace_dir.display()))?;
    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);

    // Check if JSONL exists
    if !jsonl_path.exists() {
        return Err(anyhow!("JSONL file not found at {}", jsonl_path.display()));
    }

    // Create new database and import JSONL
    let storage = Storage::open(&db_path)?;

    let result = storage.sync_from_jsonl(jsonl_path)?;

    // Rebuild blocked cache
    storage.rebuild_blocked_cache()?;

    // Clear dirty marks - after import from JSONL, db and JSONL are in sync
    // (create_issue_tx doesn't mark dirty, but we clear for safety)
    storage.clear_dirty()?;

    Ok(result.imported)
}

/// Verify database schema version.
///
/// Checks that all required tables and indexes exist.
pub fn verify_schema(workspace_dir: &Path) -> Result<bool> {
    let beads_dir = find_beads_dir(workspace_dir)
        .ok_or_else(|| anyhow!("No .beads directory found in {}", workspace_dir.display()))?;
    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);

    let conn = Connection::open(&db_path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;

    // Check for critical tables
    let tables = [
        "issues",
        "dependencies",
        "labels",
        "comments",
        "events",
        "config",
        "metadata",
        "dirty_issues",
        "export_hashes",
        "blocked_issues_cache",
        "child_counters",
    ];

    for table in &tables {
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            &[table],
            |row| row.get(0),
        )?;

        if exists == 0 {
            eprintln!("Missing table: {}", table);
            return Ok(false);
        }
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::init_workspace;
    use crate::jsonl::export_jsonl;
    use crate::model::{Issue, IssueType, Priority, Status};
    use tempfile::TempDir;

    #[test]
    fn test_check_empty_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        let result = check(workspace).unwrap();
        assert!(result.db_ok);
        assert!(result.jsonl_ok);
        assert_eq!(result.db_issue_count, 0);
        assert_eq!(result.jsonl_line_count, 0);
    }

    #[test]
    fn test_repair_from_jsonl() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);
        let jsonl_path = beads_dir.join(&metadata.jsonl_export);

        // Create initial database and export to JSONL
        let storage = Storage::open(&db_path).unwrap();
        let issue = Issue {
            id: "bf-test".to_string(),
            title: "Test".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();

        export_jsonl(&jsonl_path, || storage.list_all_issues()).unwrap();

        // Delete database
        std::fs::remove_file(&db_path).unwrap();

        // Repair from JSONL (no unflushed beads, so no need for flush_first or force)
        let imported = repair(workspace, false, false).unwrap();
        assert_eq!(imported, 1);

        // Verify repaired database
        let storage = Storage::open(&db_path).unwrap();
        let retrieved = storage.get_issue("bf-test").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test");
    }

    #[test]
    fn test_verify_schema() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        // Open storage to create database and apply schema
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);
        let _storage = Storage::open(&db_path).unwrap();

        let result = verify_schema(workspace).unwrap();
        assert!(result);
    }

    #[test]
    fn test_reclaim_stale() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);

        let storage = Storage::open(&db_path).unwrap();

        // Create a stale in_progress bead
        let mut issue = Issue {
            id: "bf-stale".to_string(),
            title: "Stale".to_string(),
            status: Status::InProgress,
            assignee: Some("worker".to_string()),
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        issue.updated_at = chrono::Utc::now() - chrono::Duration::minutes(60);
        storage.create_issue(&issue).unwrap();

        // Reclaim with 30 min TTL
        let reclaimed = reclaim_stale(workspace, 30).unwrap();
        assert_eq!(reclaimed, 1);

        // Verify bead is now open
        let retrieved = storage.get_issue("bf-stale").unwrap().unwrap();
        assert_eq!(retrieved.status, Status::Open);
        assert!(retrieved.assignee.is_none());
    }

    #[test]
    fn test_repair_refuses_with_unflushed_beads() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);
        let jsonl_path = beads_dir.join(&metadata.jsonl_export);

        // Export initial empty JSONL
        let storage = Storage::open(&db_path).unwrap();
        export_jsonl(&jsonl_path, || storage.list_all_issues()).unwrap();

        // Create a new bead (unflushed - only in db)
        let issue = Issue {
            id: "bf-unflushed".to_string(),
            title: "Unflushed Bead".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();

        // Verify bead exists in db
        let retrieved = storage.get_issue("bf-unflushed").unwrap();
        assert!(retrieved.is_some());

        // Attempt repair without flush_first or force - should refuse
        let result = repair(workspace, false, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("unflushed bead"));
        assert!(err_msg.contains("bf-unflushed"));

        // Verify db is unchanged (bead still exists)
        let storage = Storage::open(&db_path).unwrap();
        let retrieved = storage.get_issue("bf-unflushed").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Unflushed Bead");
    }

    #[test]
    fn test_repair_with_flush_first_protects_beads() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);
        let jsonl_path = beads_dir.join(&metadata.jsonl_export);

        // Export initial empty JSONL
        let storage = Storage::open(&db_path).unwrap();
        export_jsonl(&jsonl_path, || storage.list_all_issues()).unwrap();

        // Create a new bead (unflushed - only in db)
        let issue = Issue {
            id: "bf-protected".to_string(),
            title: "Protected Bead".to_string(),
            status: Status::Open,
            priority: Priority::HIGH,
            issue_type: IssueType::Task,
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();

        // Repair with --flush-first
        let imported = repair(workspace, true, false).unwrap();
        assert_eq!(imported, 1);

        // Verify bead exists in repaired db
        let storage = Storage::open(&db_path).unwrap();
        let retrieved = storage.get_issue("bf-protected").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Protected Bead");

        // Verify bead exists in JSONL
        let jsonl_content = std::fs::read_to_string(&jsonl_path).unwrap();
        assert!(jsonl_content.contains("bf-protected"));
        assert!(jsonl_content.contains("Protected Bead"));
    }

    #[test]
    fn test_repair_with_force_warns_but_proceeds() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);
        let jsonl_path = beads_dir.join(&metadata.jsonl_export);

        // Export initial JSONL with one bead
        let storage = Storage::open(&db_path).unwrap();
        let original_issue = Issue {
            id: "bf-original".to_string(),
            title: "Original Bead".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        storage.create_issue(&original_issue).unwrap();
        export_jsonl(&jsonl_path, || storage.list_all_issues()).unwrap();

        // Create another bead (unflushed - only in db)
        let unflushed_issue = Issue {
            id: "bf-unflushed".to_string(),
            title: "Unflushed Bead".to_string(),
            status: Status::Open,
            priority: Priority::HIGH,
            issue_type: IssueType::Task,
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        storage.create_issue(&unflushed_issue).unwrap();

        // Verify both beads exist in db
        let storage = Storage::open(&db_path).unwrap();
        assert!(storage.get_issue("bf-original").unwrap().is_some());
        assert!(storage.get_issue("bf-unflushed").unwrap().is_some());

        // Repair with --force (should warn but proceed)
        let imported = repair(workspace, false, true).unwrap();
        assert_eq!(imported, 1); // Only imported bf-original from JSONL

        // Verify: original bead exists, unflushed bead is gone
        let storage = Storage::open(&db_path).unwrap();
        assert!(storage.get_issue("bf-original").unwrap().is_some());
        assert!(storage.get_issue("bf-unflushed").unwrap().is_none());
    }

    #[test]
    fn test_doctor_reports_unflushed_count() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);
        let jsonl_path = beads_dir.join(&metadata.jsonl_export);

        // Export initial empty JSONL
        let storage = Storage::open(&db_path).unwrap();
        export_jsonl(&jsonl_path, || storage.list_all_issues()).unwrap();

        // Create two beads (unflushed)
        for i in 1..=2 {
            let issue = Issue {
                id: format!("bf-unflushed-{}", i),
                title: format!("Unflushed Bead {}", i),
                status: Status::Open,
                priority: Priority::MEDIUM,
                issue_type: IssueType::Task,
                source_repo: Some(".".to_string()),
                ..Default::default()
            };
            storage.create_issue(&issue).unwrap();
        }

        // Run doctor check
        let result = check(workspace).unwrap();

        // Verify unflushed count is reported
        assert_eq!(result.unflushed_count, 2);
        assert!(result.issues.iter().any(|i| i.contains("unflushed")));
    }

    #[test]
    fn test_repair_clears_unflushed_count() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);
        let jsonl_path = beads_dir.join(&metadata.jsonl_export);

        // Create a test bead and flush to JSONL
        let storage = Storage::open(&db_path).unwrap();
        let issue = Issue {
            id: "bf-test".to_string(),
            title: "Test".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();

        // Use the proper flush() function which clears dirty marks
        crate::sync::flush(workspace).unwrap();

        // Verify unflushed count is 0 after flush
        let unflushed = count_unflushed(&db_path).unwrap();
        assert_eq!(unflushed, 0);

        // Run repair (no unflushed beads, so no need for force or flush_first)
        let imported = repair(workspace, false, false).unwrap();
        assert_eq!(imported, 1);

        // After repair, unflushed count should still be 0
        let unflushed_after = count_unflushed(&db_path).unwrap();
        assert_eq!(
            unflushed_after, 0,
            "Repair should not leave unflushed beads"
        );

        // Run doctor check to verify no unflushed issues
        let result = check(workspace).unwrap();
        assert_eq!(
            result.unflushed_count, 0,
            "Doctor should report 0 unflushed after repair"
        );
    }

    #[test]
    fn test_import_leaves_zero_unflushed() {
        // Regression test for bf-2hqt: import from JSONL should leave unflushed_count == 0
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);
        let jsonl_path = beads_dir.join(&metadata.jsonl_export);

        // Create initial database with one bead
        let storage = Storage::open(&db_path).unwrap();
        let issue = Issue {
            id: "bf-test".to_string(),
            title: "Test".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();

        // Flush to JSONL (clears dirty marks)
        export_jsonl(&jsonl_path, || storage.list_all_issues()).unwrap();
        storage.clear_dirty().unwrap();

        // Verify unflushed count is 0 after flush
        let unflushed = count_unflushed(&db_path).unwrap();
        assert_eq!(unflushed, 0, "Should be 0 after flush");

        // Delete the database to simulate a fresh import scenario
        std::fs::remove_file(&db_path).unwrap();

        // Import from JSONL (this recreates the database)
        let result = crate::sync::import(workspace).unwrap();
        assert_eq!(result.imported, 1);

        // After import, unflushed count should still be 0 (beads came from JSONL)
        let unflushed_after = count_unflushed(&db_path).unwrap();
        assert_eq!(
            unflushed_after, 0,
            "Import should not leave unflushed beads"
        );

        // Run doctor check to verify no unflushed issues
        let doctor_result = check(workspace).unwrap();
        assert_eq!(
            doctor_result.unflushed_count, 0,
            "Doctor should report 0 unflushed after import"
        );
    }

    #[test]
    fn test_repair_cycle_clears_unflushed_correctly() {
        // Test that after repair -> import -> repair cycle, unflushed count stays at 0
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);
        let jsonl_path = beads_dir.join(&metadata.jsonl_export);

        // Create initial database with a bead
        let storage = Storage::open(&db_path).unwrap();
        let issue = Issue {
            id: "bf-test".to_string(),
            title: "Test".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();

        // Flush to JSONL using sync::flush (the proper flush function)
        crate::sync::flush(workspace).unwrap();

        // Verify unflushed count is 0
        let unflushed = count_unflushed(&db_path).unwrap();
        assert_eq!(unflushed, 0, "Should be 0 after flush");

        // Simulate a repair scenario (delete db, then import)
        std::fs::remove_file(&db_path).unwrap();
        let imported = crate::sync::import(workspace).unwrap();
        assert_eq!(imported.imported, 1);

        // After import cycle, unflushed should be 0
        let unflushed_after = count_unflushed(&db_path).unwrap();
        assert_eq!(unflushed_after, 0, "Should be 0 after import cycle");

        // Run doctor to verify
        let result = check(workspace).unwrap();
        assert_eq!(
            result.unflushed_count, 0,
            "Doctor should report 0 unflushed"
        );
    }

    #[test]
    fn test_import_clears_pre_existing_dirty_marks() {
        // Regression test for bf-2hqt: import should clear dirty marks from before the import
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);
        let jsonl_path = beads_dir.join(&metadata.jsonl_export);

        // Create initial database with two beads
        let storage = Storage::open(&db_path).unwrap();
        let issue1 = Issue {
            id: "bf-test1".to_string(),
            title: "Test 1".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        let issue2 = Issue {
            id: "bf-test2".to_string(),
            title: "Test 2".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        storage.create_issue(&issue1).unwrap();
        storage.create_issue(&issue2).unwrap();

        // Flush to JSONL
        crate::sync::flush(workspace).unwrap();

        // Now make a modification (mark one bead as dirty)
        let storage = Storage::open(&db_path).unwrap();
        let mut changes = crate::model::IssueChanges::default();
        changes.title = Some("Modified Title".to_string());
        storage.update_issue("bf-test1", &changes).unwrap();

        // Verify one bead is dirty
        let unflushed_before = count_unflushed(&db_path).unwrap();
        assert_eq!(unflushed_before, 1, "Should have 1 dirty bead");

        // Now run import (simulates pulling updated JSONL from git)
        let import_result = crate::sync::import(workspace).unwrap();

        // After import, all dirty marks should be cleared (db is now in sync with JSONL)
        let unflushed_after = count_unflushed(&db_path).unwrap();
        assert_eq!(
            unflushed_after, 0,
            "Import should clear all dirty marks, leaving 0"
        );

        // Run doctor to verify
        let result = check(workspace).unwrap();
        assert_eq!(
            result.unflushed_count, 0,
            "Doctor should report 0 unflushed after import"
        );
    }

    /// Regression test for bf-wre: `bf doctor` must surface beads stuck with a
    /// done-sounding custom status (e.g. "completed") instead of "closed", since those
    /// never get closed_at/close_reason set and were previously invisible to any check.
    #[test]
    fn test_check_reports_pseudo_terminal_statuses() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);

        let storage = Storage::open(&db_path).unwrap();

        let normal = Issue {
            id: "bf-normal".to_string(),
            title: "Normal open bead".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        storage.create_issue(&normal).unwrap();

        let pseudo_done = Issue {
            id: "bf-pseudo-done".to_string(),
            title: "Bead marked completed instead of closed".to_string(),
            status: Status::Custom("completed".to_string()),
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        storage.create_issue(&pseudo_done).unwrap();

        let properly_closed = Issue {
            id: "bf-properly-closed".to_string(),
            title: "Bead closed the right way".to_string(),
            status: Status::Closed,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            source_repo: Some(".".to_string()),
            closed_at: Some(chrono::Utc::now()),
            ..Default::default()
        };
        storage.create_issue(&properly_closed).unwrap();

        let result = check(workspace).unwrap();
        assert_eq!(
            result.pseudo_terminal_status_ids,
            vec!["bf-pseudo-done".to_string()],
            "only the status=completed bead should be flagged, not the open or properly-closed ones"
        );
        assert!(result.issues.iter().any(|i| i.contains("bf-pseudo-done")));
    }

    /// Force a NULL into a NOT NULL datetime column, bypassing the schema constraint
    /// the same way a corrupted/partially-migrated DB does. `PRAGMA writable_schema`
    /// lets us drop the NOT NULL from the table definition long enough to write the
    /// bad row, then we restore it -- leaving a DB that is byte-for-byte what the
    /// historical crash class produced.
    fn inject_null_datetime(db_path: &Path, issue_id: &str) {
        use rusqlite::Connection;
        let conn = Connection::open(db_path).unwrap();
        // Grab the current CREATE TABLE for issues and rewrite it without NOT NULL
        // on created_at, temporarily, so the UPDATE below is accepted.
        let create_sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='issues'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let relaxed = create_sql.replace(
            "created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP",
            "created_at DATETIME",
        );
        assert_ne!(relaxed, create_sql, "expected to relax created_at NOT NULL");
        conn.execute_batch("PRAGMA writable_schema=ON;").unwrap();
        conn.execute(
            "UPDATE sqlite_master SET sql=?1 WHERE type='table' AND name='issues'",
            [relaxed],
        )
        .unwrap();
        conn.execute_batch("PRAGMA writable_schema=OFF;").unwrap();
        // Reopen so the relaxed schema takes effect, then null out created_at.
        drop(conn);
        let conn = Connection::open(db_path).unwrap();
        let updated = conn
            .execute(
                "UPDATE issues SET created_at = NULL WHERE id = ?1",
                [issue_id],
            )
            .unwrap();
        assert_eq!(updated, 1, "expected to null exactly one row's created_at");
        // Restore the NOT NULL declaration so the detector (which keys off
        // PRAGMA table_info's notnull flag) sees a genuine NOT-NULL column holding a
        // NULL value -- exactly the corrupted state seen in the wild. The DEFAULT is
        // deliberately dropped on restore: SQLite substitutes a column's DEFAULT when
        // reading a stored NULL, which would mask the very NULL we injected. schema_
        // version must be bumped or the direct sqlite_master edit is discarded on the
        // next open.
        let restored = create_sql.replace(
            "created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP",
            "created_at DATETIME NOT NULL",
        );
        let ver: i64 = conn
            .query_row("PRAGMA schema_version", [], |r| r.get(0))
            .unwrap();
        conn.execute_batch("PRAGMA writable_schema=ON;").unwrap();
        conn.execute(
            "UPDATE sqlite_master SET sql=?1 WHERE type='table' AND name='issues'",
            [restored],
        )
        .unwrap();
        conn.execute_batch(&format!("PRAGMA schema_version={};", ver + 1))
            .unwrap();
        conn.execute_batch("PRAGMA writable_schema=OFF;").unwrap();
    }

    /// Regression for bf-3hm5h: a NULL created_at must NOT crash list/flush, and the
    /// row must still load (as the Unix epoch) rather than aborting the whole read.
    #[test]
    fn test_null_datetime_does_not_crash_list() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");
        init_workspace(&beads_dir, "bf").unwrap();
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);

        let storage = Storage::open(&db_path).unwrap();
        let issue = Issue {
            id: "bf-nulldt".to_string(),
            title: "Bead with NULL created_at".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();
        drop(storage);

        inject_null_datetime(&db_path, "bf-nulldt");

        // The whole point: this used to panic/error with InvalidColumnType.
        let storage = Storage::open(&db_path).unwrap();
        let all = storage
            .list_all_issues()
            .expect("list must not crash on NULL created_at");
        let loaded = all
            .iter()
            .find(|i| i.id == "bf-nulldt")
            .expect("row with NULL created_at should still load");
        assert_eq!(
            loaded.created_at,
            chrono::DateTime::<chrono::Utc>::UNIX_EPOCH,
            "NULL datetime should read back as the Unix epoch"
        );
    }

    /// Regression for bf-3hm5h: doctor must detect NULL-in-NOT-NULL rows.
    #[test]
    fn test_check_reports_null_not_null() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");
        init_workspace(&beads_dir, "bf").unwrap();
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);

        let storage = Storage::open(&db_path).unwrap();
        storage
            .create_issue(&Issue {
                id: "bf-nulldt".to_string(),
                title: "NULL created_at".to_string(),
                status: Status::Open,
                priority: Priority::MEDIUM,
                issue_type: IssueType::Task,
                source_repo: Some(".".to_string()),
                ..Default::default()
            })
            .unwrap();
        drop(storage);

        inject_null_datetime(&db_path, "bf-nulldt");

        let result = check(workspace).unwrap();
        assert_eq!(
            result.null_not_null.len(),
            1,
            "one violated column expected"
        );
        let v = &result.null_not_null[0];
        assert_eq!(v.table, "issues");
        assert_eq!(v.column, "created_at");
        assert_eq!(v.count, 1);
        assert!(result
            .issues
            .iter()
            .any(|i| i.contains("NOT NULL") && i.contains("issues.created_at")));
    }

    /// Regression for bf-3hm5h: `--fix-schema` must repair NULL-in-NOT-NULL rows in
    /// place (replacing a NULL datetime with the Unix epoch) without rebuilding from
    /// JSONL, and the row must survive.
    #[test]
    fn test_fix_null_not_null_repairs_in_place() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");
        init_workspace(&beads_dir, "bf").unwrap();
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);

        let storage = Storage::open(&db_path).unwrap();
        storage
            .create_issue(&Issue {
                id: "bf-nulldt".to_string(),
                title: "NULL created_at".to_string(),
                status: Status::Open,
                priority: Priority::MEDIUM,
                issue_type: IssueType::Task,
                source_repo: Some(".".to_string()),
                ..Default::default()
            })
            .unwrap();
        drop(storage);

        inject_null_datetime(&db_path, "bf-nulldt");

        // Repair, then confirm the detector is satisfied and the concrete value stuck.
        let fixed = fix_null_not_null(workspace).unwrap();
        assert_eq!(fixed, 1, "exactly one NULL value should be fixed");

        let after = check_null_not_null(&db_path).unwrap();
        assert!(after.is_empty(), "no violations should remain after fix");

        let conn = Connection::open(&db_path).unwrap();
        let created_at: String = conn
            .query_row(
                "SELECT created_at FROM issues WHERE id = 'bf-nulldt'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(created_at, "1970-01-01T00:00:00+00:00");

        // Second run is a no-op (idempotent).
        assert_eq!(fix_null_not_null(workspace).unwrap(), 0);
    }

    /// A clean database must report zero NULL-in-NOT-NULL violations.
    #[test]
    fn test_check_no_null_not_null_on_clean_db() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");
        init_workspace(&beads_dir, "bf").unwrap();
        let metadata = load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);

        let storage = Storage::open(&db_path).unwrap();
        storage
            .create_issue(&Issue {
                id: "bf-clean".to_string(),
                title: "Clean bead".to_string(),
                status: Status::Open,
                priority: Priority::MEDIUM,
                issue_type: IssueType::Task,
                source_repo: Some(".".to_string()),
                ..Default::default()
            })
            .unwrap();
        drop(storage);

        let result = check(workspace).unwrap();
        assert!(
            result.null_not_null.is_empty(),
            "clean db should have no NULL-in-NOT-NULL violations"
        );
    }
}
