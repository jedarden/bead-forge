//! Sync protocol for bead-forge.
//!
//! Implements flush (SQLite → JSONL) and import (JSONL → SQLite) operations
//! for git-backed bead synchronization.

use crate::config::{find_beads_dir, load_metadata};
use crate::jsonl::{export_jsonl, export_jsonl_dirty, import_jsonl};
use crate::model::Issue;
use crate::storage::Storage;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Sync operation results.
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub imported: usize,
    pub exported: usize,
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
    let result = export_jsonl(&jsonl_path, || storage.list_all_issues())?;

    // Update export_hashes for incremental export tracking
    update_export_hashes(&storage)?;

    Ok(result.count)
}

/// Flush only dirty beads from SQLite to JSONL.
///
/// Incremental export for faster sync on large workspaces. Only exports
/// beads that have been modified since the last flush.
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
    let result = export_jsonl_dirty(&jsonl_path, || storage.list_dirty_issues(), || storage.clear_dirty())?;

    // Update export_hashes for incremental export tracking
    update_export_hashes(&storage)?;

    Ok(result.count)
}

/// Import beads from JSONL into SQLite.
///
/// Compares each bead in JSONL with SQLite state using content_hash.
/// INSERTs new beads, UPDATEs changed beads, SKIPs unchanged beads.
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

    let result = import_jsonl(&jsonl_path, |issue| {
        let existing = storage.get_issue(&issue.id)?;
        match existing {
            None => {
                // New bead - insert it
                storage.create_issue(issue)?;
                Ok(true)
            }
            Some(existing_issue) => {
                // Existing bead - check if changed
                if existing_issue.content_hash != issue.content_hash {
                    storage.update_issue_from_json(issue)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    })?;

    // Rebuild blocked cache after import
    storage.rebuild_blocked_cache()?;

    Ok(SyncResult {
        imported: result.imported,
        exported: 0,
        skipped: result.updated, // updated = skipped (unchanged)
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
        skipped: import_result.skipped,
    })
}

/// Update export_hashes table after a flush.
///
/// This tracks which beads have been exported and their content hashes,
/// enabling incremental export operations.
fn update_export_hashes(storage: &Storage) -> Result<()> {
    storage.with_immediate_transaction(|tx| {
        // Clear old export hashes
        tx.execute("DELETE FROM export_hashes", [])?;

        // Insert current hashes for all issues
        tx.execute(
            "INSERT INTO export_hashes (issue_id, content_hash, exported_at)
             SELECT id, content_hash, ?1 FROM issues WHERE deleted_at IS NULL",
            rusqlite::params![chrono::Utc::now().to_rfc3339()],
        )?;

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
}
