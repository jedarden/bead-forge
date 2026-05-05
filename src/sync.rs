//! Sync protocol for bead-forge.
//!
//! Implements flush (SQLite → JSONL) and import (JSONL → SQLite) operations
//! for git-backed bead synchronization.

use crate::config::{find_beads_dir, load_metadata};
use crate::jsonl::{export_jsonl, export_jsonl_dirty, import_jsonl, UpsertResult};
use crate::model::Issue;
use crate::storage::Storage;
use anyhow::Result;
use chrono::Utc;
use std::path::{Path, PathBuf};

/// Sync operation results.
#[derive(Debug, Clone, Default)]
pub struct SyncResult {
    pub imported: usize,
    pub exported: usize,
    pub updated: usize,
    pub skipped: usize,
}

/// Flush all beads from SQLite to JSONL.
///
/// This is the primary export operation for git commit. Exports all beads
/// sorted by ID for stable diffs.
///
/// # Arguments
/// * `workspace_dir` - Path to the workspace root (contains .beads/)
///
/// # Returns
/// * `Ok(usize)` - Number of beads exported
pub fn flush(workspace_dir: &Path) -> Result<usize> {
    let beads_dir = find_beads_dir(workspace_dir)
        .ok_or_else(|| anyhow::anyhow!("No .beads directory found in {}", workspace_dir.display()))?;
    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let jsonl_path = beads_dir.join(&metadata.jsonl_export);

    let storage = Storage::open(&db_path)?;

    // Get all issues for export
    let issues = storage.list_all_issues()?;

    // Export to JSONL with atomic temp+rename
    let result = export_jsonl(&jsonl_path, || Ok(issues.clone()))?;

    // Update export_hashes for all exported issues
    update_export_hashes_for_issues(&storage, &issues)?;

    Ok(result.count)
}

/// Flush only dirty beads from SQLite to JSONL.
///
/// Incremental export for faster sync on large workspaces. Only exports
/// beads that have been modified since the last flush.
///
/// NOTE: This function exports ONLY dirty beads to the JSONL file, replacing
/// its contents. For a full export of all beads, use `flush()` instead.
///
/// # Arguments
/// * `workspace_dir` - Path to the workspace root (contains .beads/)
///
/// # Returns
/// * `Ok(usize)` - Number of beads exported
pub fn flush_dirty(workspace_dir: &Path) -> Result<usize> {
    let beads_dir = find_beads_dir(workspace_dir)
        .ok_or_else(|| anyhow::anyhow!("No .beads directory found in {}", workspace_dir.display()))?;
    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let jsonl_path = beads_dir.join(&metadata.jsonl_export);

    let storage = Storage::open(&db_path)?;

    // Get dirty issues for export
    let dirty_issues = storage.list_dirty_issues()?;
    if dirty_issues.is_empty() {
        return Ok(0);
    }

    // Export to JSONL with atomic temp+rename (only dirty issues)
    let result = export_jsonl_dirty(
        &jsonl_path,
        || Ok(dirty_issues.clone()),
        || storage.clear_dirty(),
    )?;

    // Update export_hashes for dirty issues only
    update_export_hashes_for_issues(&storage, &dirty_issues)?;

    Ok(result.count)
}

/// Import beads from JSONL into SQLite.
///
/// Compares each bead in JSONL with SQLite state using content_hash.
/// INSERTs new beads, UPDATEs changed beads, SKIPs unchanged beads.
///
/// Collision resolution: when both JSONL and SQLite have changes for the
/// same bead, the one with the later `updated_at` timestamp wins.
///
/// # Arguments
/// * `workspace_dir` - Path to the workspace root (contains .beads/)
///
/// # Returns
/// * `Ok(SyncResult)` - Import statistics
pub fn import(workspace_dir: &Path) -> Result<SyncResult> {
    let beads_dir = find_beads_dir(workspace_dir)
        .ok_or_else(|| anyhow::anyhow!("No .beads directory found in {}", workspace_dir.display()))?;
    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let jsonl_path = beads_dir.join(&metadata.jsonl_export);

    let storage = Storage::open(&db_path)?;

    // Stream import with content_hash comparison
    let result = storage.with_immediate_transaction(|tx| {
        import_jsonl(&jsonl_path, |issue| {
            let incoming_hash = issue.content_hash();
            let existing = Storage::get_issue_tx(tx, &issue.id)?;

            match existing {
                None => {
                    // New bead - insert
                    Storage::create_issue_tx(tx, &issue)?;
                    Ok(UpsertResult::New)
                }
                Some(existing_issue) => {
                    let existing_hash = existing_issue.content_hash();

                    if incoming_hash == existing_hash {
                        // Content unchanged - skip
                        Ok(UpsertResult::Unchanged)
                    } else {
                        // Content changed - use deterministic collision resolution
                        // The bead with the later updated_at wins
                        if issue.updated_at > existing_issue.updated_at {
                            Storage::update_issue_from_json_tx(tx, &issue)?;
                            Ok(UpsertResult::Updated)
                        } else {
                            // SQLite version is newer - skip JSONL version
                            Ok(UpsertResult::Unchanged)
                        }
                    }
                }
            }
        })
    })?;

    // Rebuild blocked cache after import
    storage.rebuild_blocked_cache()?;

    Ok(SyncResult {
        imported: result.imported,
        exported: 0,
        updated: result.updated,
        skipped: result.skipped,
    })
}

/// Full sync: import then flush.
///
/// Performs both import and flush operations. Use this when you want to
/// ensure bidirectional synchronization.
///
/// # Arguments
/// * `workspace_dir` - Path to the workspace root (contains .beads/)
///
/// # Returns
/// * `Ok(SyncResult)` - Sync statistics
pub fn sync(workspace_dir: &Path) -> Result<SyncResult> {
    let import_result = import(workspace_dir)?;
    let exported = flush(workspace_dir)?;

    Ok(SyncResult {
        imported: import_result.imported,
        exported,
        updated: import_result.updated,
        skipped: import_result.skipped,
    })
}

/// Update export_hashes table for a set of issues.
///
/// This tracks which beads have been exported and their content hashes,
/// enabling incremental export operations.
fn update_export_hashes_for_issues(storage: &Storage, issues: &[Issue]) -> Result<()> {
    storage.with_immediate_transaction(|tx| {
        let now = Utc::now().to_rfc3339();

        for issue in issues {
            let hash = issue.content_hash();
            tx.execute(
                "INSERT OR REPLACE INTO export_hashes (issue_id, content_hash, exported_at)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![&issue.id, &hash, &now],
            )?;
        }

        Ok(())
    })
}

/// Find the .beads directory for a workspace.
pub fn find_workspace(start_dir: &Path) -> Result<PathBuf> {
    find_beads_dir(start_dir)
        .ok_or_else(|| anyhow::anyhow!("No .beads directory found in {}", start_dir.display()))
}

/// Get the JSONL path for a workspace.
pub fn get_jsonl_path(workspace_dir: &Path) -> Result<PathBuf> {
    let beads_dir = find_workspace(workspace_dir)?;
    let metadata = load_metadata(&beads_dir)?;
    Ok(beads_dir.join(metadata.jsonl_export))
}

/// Get the database path for a workspace.
pub fn get_db_path(workspace_dir: &Path) -> Result<PathBuf> {
    let beads_dir = find_workspace(workspace_dir)?;
    let metadata = load_metadata(&beads_dir)?;
    Ok(beads_dir.join(metadata.database))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::init_workspace;
    use crate::model::{Issue, IssueType, Priority, Status};
    use crate::storage::Storage;
    use tempfile::TempDir;

    #[test]
    fn test_find_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        let found = find_workspace(workspace).unwrap();
        assert_eq!(found, beads_dir);
    }

    #[test]
    fn test_find_workspace_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        let result = find_workspace(workspace);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_jsonl_path() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        let jsonl_path = get_jsonl_path(workspace).unwrap();
        assert_eq!(jsonl_path, beads_dir.join("issues.jsonl"));
    }

    #[test]
    fn test_get_db_path() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        let db_path = get_db_path(workspace).unwrap();
        assert_eq!(db_path, beads_dir.join("beads.db"));
    }

    #[test]
    fn test_flush_and_import_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        let db_path = beads_dir.join("beads.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create a test issue
        let issue = Issue {
            id: "bf-test".to_string(),
            title: "Test Issue".to_string(),
            description: Some("Test Description".to_string()),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();

        // Flush to JSONL
        let exported = flush(workspace).unwrap();
        assert_eq!(exported, 1);

        // Verify JSONL file exists
        let jsonl_path = beads_dir.join("issues.jsonl");
        assert!(jsonl_path.exists());

        // Clear the database
        std::fs::remove_file(&db_path).unwrap();
        let storage2 = Storage::open(&db_path).unwrap();

        // Import from JSONL
        let result = import(workspace).unwrap();
        assert_eq!(result.imported, 1);

        // Verify the issue was imported correctly
        let imported = storage2.get_issue("bf-test").unwrap().unwrap();
        assert_eq!(imported.id, "bf-test");
        assert_eq!(imported.title, "Test Issue");
        assert_eq!(imported.description, Some("Test Description".to_string()));
    }

    #[test]
    fn test_import_skips_unchanged() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        let db_path = beads_dir.join("beads.db");
        let storage = Storage::open(&db_path).unwrap();

        // Create a test issue
        let issue = Issue {
            id: "bf-test".to_string(),
            title: "Test Issue".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        storage.create_issue(&issue).unwrap();

        // Flush to JSONL
        flush(workspace).unwrap();

        // Import again - should skip unchanged
        let result = import(workspace).unwrap();
        assert_eq!(result.imported, 0);
        assert_eq!(result.skipped, 1);
    }

    #[test]
    fn test_collision_resolution_newer_wins() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        let db_path = beads_dir.join("beads.db");
        let jsonl_path = beads_dir.join("issues.jsonl");

        // Create an initial issue in the database
        let storage = Storage::open(&db_path).unwrap();
        let base_time = Utc::now();
        let old_issue = Issue {
            id: "bf-test".to_string(),
            title: "Old Title".to_string(),
            description: Some("Old Description".to_string()),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: base_time,
            updated_at: base_time,
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        storage.create_issue(&old_issue).unwrap();

        // Create JSONL with a newer version
        let newer_issue = Issue {
            id: "bf-test".to_string(),
            title: "New Title".to_string(),
            description: Some("New Description".to_string()),
            status: Status::Open,
            priority: Priority::HIGH,
            issue_type: IssueType::Bug,
            created_at: base_time,
            updated_at: base_time + chrono::Duration::seconds(10),
            source_repo: Some(".".to_string()),
            ..Default::default()
        };

        // Write the newer issue to JSONL
        {
            use std::io::Write;
            let mut file = std::fs::File::create(&jsonl_path).unwrap();
            writeln!(file, "{}", serde_json::to_string(&newer_issue).unwrap()).unwrap();
        }

        // Import - should update to newer version
        let result = import(workspace).unwrap();
        assert_eq!(result.updated, 1);

        // Verify the newer version won
        let storage2 = Storage::open(&db_path).unwrap();
        let current = storage2.get_issue("bf-test").unwrap().unwrap();
        assert_eq!(current.title, "New Title");
        assert_eq!(current.priority, Priority::HIGH);
    }

    #[test]
    fn test_collision_resolution_older_skipped() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        let db_path = beads_dir.join("beads.db");
        let jsonl_path = beads_dir.join("issues.jsonl");

        // Create a newer issue in the database
        let storage = Storage::open(&db_path).unwrap();
        let base_time = Utc::now();
        let newer_issue = Issue {
            id: "bf-test".to_string(),
            title: "New Title".to_string(),
            status: Status::Open,
            priority: Priority::HIGH,
            issue_type: IssueType::Bug,
            created_at: base_time,
            updated_at: base_time + chrono::Duration::seconds(10),
            source_repo: Some(".".to_string()),
            ..Default::default()
        };
        storage.create_issue(&newer_issue).unwrap();

        // Create JSONL with an older version
        let old_issue = Issue {
            id: "bf-test".to_string(),
            title: "Old Title".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: base_time,
            updated_at: base_time,
            source_repo: Some(".".to_string()),
            ..Default::default()
        };

        // Write the older issue to JSONL
        {
            use std::io::Write;
            let mut file = std::fs::File::create(&jsonl_path).unwrap();
            writeln!(file, "{}", serde_json::to_string(&old_issue).unwrap()).unwrap();
        }

        // Import - should skip older version
        let result = import(workspace).unwrap();
        assert_eq!(result.skipped, 1);

        // Verify the newer version is still in place
        let storage2 = Storage::open(&db_path).unwrap();
        let current = storage2.get_issue("bf-test").unwrap().unwrap();
        assert_eq!(current.title, "New Title");
        assert_eq!(current.priority, Priority::HIGH);
    }

    #[test]
    fn test_flush_dirty_with_no_changes() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        // Flush dirty with no dirty issues should return 0
        let exported = flush_dirty(workspace).unwrap();
        assert_eq!(exported, 0);
    }
}
