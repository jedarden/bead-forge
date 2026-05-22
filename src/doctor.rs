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
    use std::collections::HashMap;

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

    // Stream JSONL and compare
    if jsonl_path.exists() {
        let iter = stream_issues(jsonl_path)?;
        for result in iter {
            let jsonl_issue = result?;
            let jsonl_hash = jsonl_issue.content_hash();
            let bead_id = jsonl_issue.id.clone();

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

    // Find beads in SQLite but not in JSONL
    for bead_id in sqlite_issues.keys() {
        // We need to check if this bead was seen in JSONL
        // Since we already iterated JSONL, anything not found there is missing
        let was_in_jsonl = if jsonl_path.exists() {
            // Re-stream to check - this is O(n*m) but acceptable for doctor check
            let iter = stream_issues(jsonl_path)?;
            let mut found = false;
            for result in iter {
                if let Ok(issue) = result {
                    if &issue.id == bead_id {
                        found = true;
                        break;
                    }
                }
            }
            found
        } else {
            false
        };

        if !was_in_jsonl {
            drift.missing_in_jsonl.push(bead_id.clone());
        }
    }

    Ok(drift)
}

/// Check consistency between database and JSONL.
///
/// Returns Some(count) if counts differ, None if they match.
fn check_consistency(db_path: &Path, jsonl_path: &Path) -> Result<Option<usize>> {
    let conn = Connection::open(db_path)?;
    let db_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM issues WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    )?;

    let jsonl_count = if jsonl_path.exists() {
        let iter = stream_issues(jsonl_path)?;
        iter.count() as i64
    } else {
        0
    };

    if db_count != jsonl_count {
        Ok(Some(db_count as usize))
    } else {
        Ok(None)
    }
}

/// Repair the database by rebuilding from JSONL.
///
/// This is the recovery operation when the database is corrupted or missing.
/// The JSONL file is the authoritative source of truth.
///
/// # Arguments
/// * `workspace_dir` - Path to the workspace root (contains .beads/)
///
/// # Returns
/// * `Ok(usize)` - Number of beads imported
pub fn repair(workspace_dir: &Path) -> Result<usize> {
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

        // Repair from JSONL
        let imported = repair(workspace).unwrap();
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
}
