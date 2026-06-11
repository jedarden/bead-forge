//! Doctor and repair operations for bead-forge.
//!
//! Provides health checking and recovery operations for bead databases,
//! including corruption detection and JSONL-based repair.

use crate::config::{find_beads_dir, load_metadata};
use crate::jsonl::{import_jsonl, stream_issues, UpsertResult};
use crate::storage::Storage;
use anyhow::{anyhow, Result};
use rusqlite::Connection;
use std::path::Path;

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

    result.db_ok = db_ok;
    result.jsonl_ok = jsonl_ok;
    result.issues = issues;

    Ok(result)
}

/// Check database integrity.
fn check_database(db_path: &Path) -> Result<(usize, bool)> {
    let conn = Connection::open(db_path)?;

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
fn count_unflushed(db_path: &Path) -> Result<usize> {
    use rusqlite::Connection;

    let conn = Connection::open(db_path)?;

    // Check if dirty_issues table exists
    let table_exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='dirty_issues'",
        [],
        |row| row.get(0),
    )?;

    if table_exists == 0 {
        return Ok(0);
    }

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM dirty_issues",
        [],
        |row| row.get(0),
    )?;

    Ok(count as usize)
}

/// Get IDs of unflushed beads.
///
/// Returns a list of bead IDs that exist in SQLite but have been marked as dirty
/// (modified or created since the last flush to JSONL).
fn get_unflushed_ids(db_path: &Path) -> Result<Vec<String>> {
    use rusqlite::Connection;

    let conn = Connection::open(db_path)?;

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
            eprintln!("Flushing all beads to JSONL before repair (including {} unflushed)...", unflushed_ids.len());
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

    let result = import_jsonl(&jsonl_path, |issue| {
        storage.create_issue(issue)?;
        Ok(UpsertResult::New)
    })?;

    // Rebuild blocked cache
    storage.rebuild_blocked_cache()?;

    Ok(result.imported)
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

    let result = import_jsonl(jsonl_path, |issue| {
        storage.create_issue(issue)?;
        Ok(UpsertResult::New)
    })?;

    // Rebuild blocked cache
    storage.rebuild_blocked_cache()?;

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
}
