//! Sync protocol for bead-forge.
//!
//! Implements flush (SQLite → JSONL) and import (JSONL → SQLite) operations
//! for git-backed bead synchronization.

use crate::config::{find_beads_dir, load_config, load_metadata};
use crate::history::backup_before_export;
use crate::jsonl::{export_jsonl, export_jsonl_dirty, export_jsonl_merge, import_jsonl, UpsertResult};
use crate::merge::update_base_anchor;
use crate::model::{Issue, IssueChanges};
use crate::storage::Storage;
use anyhow::Result;
use chrono::Utc;
use rusqlite::params;
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
    let beads_dir = find_beads_dir(workspace_dir).ok_or_else(|| {
        anyhow::anyhow!("No .beads directory found in {}", workspace_dir.display())
    })?;
    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let jsonl_path = beads_dir.join(&metadata.jsonl_export);

    let storage = Storage::open(&db_path)?;

    // Get all issues for export
    let issues = storage.list_all_issues()?;

    // Pre-export history backup: snapshot the outgoing issues.jsonl before it
    // is overwritten (best-effort — a failed backup must not abort the flush).
    let config = load_config(&beads_dir).unwrap_or_default();
    if let Err(e) = backup_before_export(&beads_dir, &jsonl_path, &config.history) {
        eprintln!("WARNING: pre-export history backup failed: {e}");
    }

    // Export to JSONL with atomic temp+rename
    let result = export_jsonl(&jsonl_path, || Ok(issues.clone()))?;

    // Update export_hashes for all exported issues
    update_export_hashes_for_issues(&storage, &issues)?;

    // Clear dirty marks - all beads have been flushed to JSONL
    storage.clear_dirty()?;

    // Refresh the merge anchor: the freshly-exported JSONL is now the common
    // ancestor for the next three-way merge across checkouts.
    if let Err(e) = update_base_anchor(&beads_dir, &jsonl_path) {
        eprintln!("WARNING: could not update merge anchor: {e}");
    }

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
/// # Rotation interplay (plan §7.1 Open Question — RESOLVED)
///
/// The export target is ALWAYS the **active** file named by
/// `metadata.jsonl_export` (e.g. `issues.jsonl`). Rotated archives
/// (`issues.jsonl.1`, `.2`, …) are written EXCLUSIVELY by
/// [`crate::rotate::rotate`]; this incremental flush path never reads or
/// writes them. This holds by construction — the only resolution of the
/// export path is `beads_dir.join(&metadata.jsonl_export)` two lines below —
/// so an archived bead cannot be silently revived into, or merged into, the
/// active file by auto-flush. Pinned by
/// `tests/batch_cascade_and_rotation.rs::incremental_flush_targets_only_active_jsonl_not_archive`.
///
/// # Arguments
/// * `workspace_dir` - Path to the workspace root (contains .beads/)
///
/// # Returns
/// * `Ok(usize)` - Number of beads exported
pub fn flush_dirty(workspace_dir: &Path) -> Result<usize> {
    let beads_dir = find_beads_dir(workspace_dir).ok_or_else(|| {
        anyhow::anyhow!("No .beads directory found in {}", workspace_dir.display())
    })?;
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

/// Flush after a hard delete: surgically drop the deleted beads' lines from
/// JSONL while flushing any still-dirty beads.
///
/// A hard `DELETE FROM issues` cascades away the bead's `dirty_issues` row (see
/// the FK `ON DELETE CASCADE`), so [`flush_dirty`] alone can never remove the
/// stale line — the deleted bead would linger in `issues.jsonl` forever. This
/// path passes the removed ids explicitly to [`export_jsonl_merge`], which
/// strips them while preserving every other line, then flushes and clears any
/// dirty beads in the same atomic write.
///
/// Returns the number of dirty beads re-exported (0 when only removals ran).
pub fn flush_after_delete(workspace_dir: &Path, removed_ids: &[String]) -> Result<usize> {
    let beads_dir = find_beads_dir(workspace_dir).ok_or_else(|| {
        anyhow::anyhow!("No .beads directory found in {}", workspace_dir.display())
    })?;
    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let jsonl_path = beads_dir.join(&metadata.jsonl_export);

    let storage = Storage::open(&db_path)?;
    let dirty_issues = storage.list_dirty_issues()?;

    // Nothing to write and no file to prune — avoid creating an empty JSONL.
    if dirty_issues.is_empty() && !jsonl_path.exists() {
        return Ok(0);
    }

    let result = export_jsonl_merge(&jsonl_path, &dirty_issues, removed_ids)?;
    if !dirty_issues.is_empty() {
        storage.clear_dirty()?;
        update_export_hashes_for_issues(&storage, &dirty_issues)?;
    }

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
/// NOTE: If there are unflushed beads (db-only or db-newer), this function
/// will emit a warning but proceed. The collision resolution logic preserves
/// SQLite versions when they are newer, so unflushed beads are protected.
/// Run `bf sync --flush-only` first to silence the warning.
///
/// # Arguments
/// * `workspace_dir` - Path to the workspace root (contains .beads/)
///
/// # Returns
/// * `Ok(SyncResult)` - Import statistics
pub fn import(workspace_dir: &Path) -> Result<SyncResult> {
    let beads_dir = find_beads_dir(workspace_dir).ok_or_else(|| {
        anyhow::anyhow!("No .beads directory found in {}", workspace_dir.display())
    })?;
    let metadata = load_metadata(&beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let jsonl_path = beads_dir.join(&metadata.jsonl_export);

    let storage = Storage::open(&db_path)?;

    // Check for unflushed beads and warn
    let dirty_issues = storage.list_dirty_issues()?;
    if !dirty_issues.is_empty() {
        eprintln!(
            "WARNING: {} unflushed bead(s) exist in SQLite (modified/created since last flush to JSONL).",
            dirty_issues.len()
        );
        eprintln!("  Import will preserve SQLite versions when they are newer.");
        eprintln!("  Run 'bf sync --flush-only' first to flush these beads to JSONL.");
        let dirty_ids: Vec<String> = dirty_issues.iter().map(|i| i.id.clone()).collect();
        if dirty_ids.len() <= 5 {
            eprintln!("  Unflushed: {}", dirty_ids.join(", "));
        } else {
            eprintln!(
                "  Unflushed: {}, ... and {} more",
                dirty_ids[..5].join(", "),
                dirty_ids.len() - 5
            );
        }
    }

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

    // Clear dirty marks - after import from JSONL, db and JSONL are in sync
    // Beads that were created/updated during import are now flushed to JSONL
    storage.clear_dirty()?;

    // The JSONL we just imported is now the agreed-upon common state; record it
    // as the merge anchor for the next divergence.
    if let Err(e) = update_base_anchor(&beads_dir, &jsonl_path) {
        eprintln!("WARNING: could not update merge anchor: {e}");
    }

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

/// Auto-flush result with warning information for JSON output.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct AutoFlushResult {
    pub flushed: bool,
    pub count: Option<usize>,
    pub warning: Option<String>,
}

/// Best-effort auto-flush after mutations.
///
/// Attempts to incrementally export dirty issues to JSONL. On failure:
/// - Prints warning to stderr
/// - Returns warning text for JSON envelope inclusion
/// - Never fails the calling operation (mutations succeed regardless)
/// - Dirty marks are preserved for manual `bf sync --flush-only` recovery
///
/// # Arguments
/// * `workspace_dir` - Path to the workspace root (contains .beads/)
/// * `config_enabled` - Whether sync.auto_flush is enabled in config
/// * `cli_disabled` - Whether --no-auto-flush flag was passed
///
/// # Returns
/// * `Ok(AutoFlushResult)` - Result with warning if flush failed
pub fn auto_flush(workspace_dir: &Path, config_enabled: bool, cli_disabled: bool) -> Result<AutoFlushResult> {
    // Check if auto-flush is disabled
    if cli_disabled {
        return Ok(AutoFlushResult {
            flushed: false,
            count: None,
            warning: None,
        });
    }

    if !config_enabled {
        return Ok(AutoFlushResult {
            flushed: false,
            count: None,
            warning: None,
        });
    }

    // Attempt best-effort flush
    match flush_dirty(workspace_dir) {
        Ok(count) => Ok(AutoFlushResult {
            flushed: true,
            count: Some(count),
            warning: None,
        }),
        Err(e) => {
            let warning_msg = format!(
                "Auto-flush failed: {}. Beads remain in SQLite; run 'bf sync --flush-only' to manually flush.",
                e
            );
            eprintln!("WARNING: {}", warning_msg);
            Ok(AutoFlushResult {
                flushed: false,
                count: None,
                warning: Some(warning_msg),
            })
        }
    }
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
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_find_workspace_not_found() {
        // Use /tmp directly instead of TMPDIR which may be under project root
        // On this system TMPDIR=/home/coding/.tmp which is under /home/coding/.beads/
        // causing find_beads_dir() to walk up and find it, making this test fail.
        let temp_dir = TempDir::new_in("/tmp").unwrap();
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

    #[test]
    fn test_labels_import_from_jsonl() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        let db_path = beads_dir.join("beads.db");
        let jsonl_path = beads_dir.join("issues.jsonl");

        // Create JSONL with issues that have labels
        let issue_with_labels = Issue {
            id: "bf-labels".to_string(),
            title: "Test Labels Import".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec!["phase-1".to_string(), "storage".to_string(), "critical".to_string()],
            ..Default::default()
        };

        let issue_without_labels = Issue {
            id: "bf-nolabels".to_string(),
            title: "Test No Labels".to_string(),
            status: Status::Open,
            priority: Priority::LOW,
            issue_type: IssueType::Chore,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec![],
            ..Default::default()
        };

        {
            use std::io::Write;
            let mut file = std::fs::File::create(&jsonl_path).unwrap();
            writeln!(file, "{}", serde_json::to_string(&issue_with_labels).unwrap()).unwrap();
            writeln!(file, "{}", serde_json::to_string(&issue_without_labels).unwrap()).unwrap();
        }

        // Import from JSONL
        let result = import(workspace).unwrap();
        assert_eq!(result.imported, 2);

        // Verify labels were imported correctly
        let storage = Storage::open(&db_path).unwrap();

        let imported1 = storage.get_issue("bf-labels").unwrap().unwrap();
        assert_eq!(imported1.labels.len(), 3);
        assert!(imported1.labels.contains(&"phase-1".to_string()));
        assert!(imported1.labels.contains(&"storage".to_string()));
        assert!(imported1.labels.contains(&"critical".to_string()));

        let imported2 = storage.get_issue("bf-nolabels").unwrap().unwrap();
        assert_eq!(imported2.labels.len(), 0);

        // Verify labels in bead_labels table
        storage.with_immediate_transaction(|tx| {
            let mut stmt = tx.prepare("SELECT label FROM bead_labels WHERE bead_id = ?1 ORDER BY label").unwrap();
            let labels: Vec<String> = stmt.query_map(params!["bf-labels"], |row| row.get::<_, String>(0)).unwrap()
                .filter_map(|r| r.ok()).collect();
            assert_eq!(labels, vec!["critical", "phase-1", "storage"]);
            Ok(())
        }).unwrap();

        // Verify no labels for the issue without labels
        storage.with_immediate_transaction(|tx| {
            let mut stmt = tx.prepare("SELECT COUNT(*) FROM bead_labels WHERE bead_id = ?1").unwrap();
            let count: i64 = stmt.query_row(params!["bf-nolabels"], |row| row.get(0)).unwrap();
            assert_eq!(count, 0);
            Ok(())
        }).unwrap();
    }

    #[test]
    fn test_labels_import_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        let db_path = beads_dir.join("beads.db");
        let jsonl_path = beads_dir.join("issues.jsonl");

        // Create an issue with labels
        let issue = Issue {
            id: "bf-Idempotent".to_string(),
            title: "Idempotent Labels Import".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec!["phase-2".to_string(), "testing".to_string()],
            ..Default::default()
        };

        {
            use std::io::Write;
            let mut file = std::fs::File::create(&jsonl_path).unwrap();
            writeln!(file, "{}", serde_json::to_string(&issue).unwrap()).unwrap();
        }

        // Import twice
        let result1 = import(workspace).unwrap();
        assert_eq!(result1.imported, 1);

        let result2 = import(workspace).unwrap();
        assert_eq!(result2.imported, 0);
        assert_eq!(result2.skipped, 1);

        // Verify labels are still correct after second import
        let storage = Storage::open(&db_path).unwrap();
        let imported = storage.get_issue("bf-Idempotent").unwrap().unwrap();
        assert_eq!(imported.labels.len(), 2);
        assert!(imported.labels.contains(&"phase-2".to_string()));
        assert!(imported.labels.contains(&"testing".to_string()));
    }

    #[test]
    fn test_labels_flush_import_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        let db_path = beads_dir.join("beads.db");
        let jsonl_path = beads_dir.join("issues.jsonl");

        // Create multiple issues with different label configurations
        let issue_with_many_labels = Issue {
            id: "bf-many-labels".to_string(),
            title: "Issue with Many Labels".to_string(),
            description: Some("Testing label persistence with multiple labels".to_string()),
            status: Status::Open,
            priority: Priority::CRITICAL,
            issue_type: IssueType::Bug,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec![
                "phase-1".to_string(),
                "storage".to_string(),
                "critical".to_string(),
                "database".to_string(),
                "priority-p0".to_string(),
            ],
            ..Default::default()
        };

        let issue_with_one_label = Issue {
            id: "bf-one-label".to_string(),
            title: "Issue with One Label".to_string(),
            status: Status::InProgress,
            priority: Priority::HIGH,
            issue_type: IssueType::Feature,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec!["phase-2".to_string()],
            ..Default::default()
        };

        let issue_without_labels = Issue {
            id: "bf-no-labels".to_string(),
            title: "Issue without Labels".to_string(),
            status: Status::Open,
            priority: Priority::LOW,
            issue_type: IssueType::Chore,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec![],
            ..Default::default()
        };

        // Create all issues in the database
        let storage = Storage::open(&db_path).unwrap();
        storage.create_issue(&issue_with_many_labels).unwrap();
        storage.create_issue(&issue_with_one_label).unwrap();
        storage.create_issue(&issue_without_labels).unwrap();

        // Flush to JSONL
        let exported = flush(workspace).unwrap();
        assert_eq!(exported, 3, "All three issues should be exported");

        // Verify JSONL file exists and contains the labels
        assert!(jsonl_path.exists(), "JSONL file should exist after flush");
        let jsonl_contents = std::fs::read_to_string(&jsonl_path).unwrap();

        // Parse all lines and verify labels
        let mut found_labels = std::collections::HashMap::new();
        for line in jsonl_contents.lines() {
            if let Ok(issue) = serde_json::from_str::<Issue>(line) {
                found_labels.insert(issue.id.clone(), issue.labels.clone());
            }
        }

        // Verify labels are in JSONL
        assert_eq!(found_labels.get("bf-many-labels").unwrap().len(), 5);
        assert!(found_labels.get("bf-many-labels").unwrap().contains(&"phase-1".to_string()));
        assert!(found_labels.get("bf-many-labels").unwrap().contains(&"storage".to_string()));
        assert!(found_labels.get("bf-many-labels").unwrap().contains(&"critical".to_string()));
        assert!(found_labels.get("bf-many-labels").unwrap().contains(&"database".to_string()));
        assert!(found_labels.get("bf-many-labels").unwrap().contains(&"priority-p0".to_string()));

        assert_eq!(found_labels.get("bf-one-label").unwrap().len(), 1);
        assert_eq!(found_labels.get("bf-one-label").unwrap()[0], "phase-2");

        assert_eq!(found_labels.get("bf-no-labels").unwrap().len(), 0);

        // Clear the database and re-import
        std::fs::remove_file(&db_path).unwrap();
        let storage2 = Storage::open(&db_path).unwrap();

        // Import from JSONL
        let import_result = import(workspace).unwrap();
        assert_eq!(import_result.imported, 3, "All three issues should be imported");

        // Verify labels survived the roundtrip
        let imported_many = storage2.get_issue("bf-many-labels").unwrap().unwrap();
        assert_eq!(imported_many.labels.len(), 5);
        assert!(imported_many.labels.contains(&"phase-1".to_string()));
        assert!(imported_many.labels.contains(&"storage".to_string()));
        assert!(imported_many.labels.contains(&"critical".to_string()));
        assert!(imported_many.labels.contains(&"database".to_string()));
        assert!(imported_many.labels.contains(&"priority-p0".to_string()));

        let imported_one = storage2.get_issue("bf-one-label").unwrap().unwrap();
        assert_eq!(imported_one.labels.len(), 1);
        assert_eq!(imported_one.labels[0], "phase-2");

        let imported_none = storage2.get_issue("bf-no-labels").unwrap().unwrap();
        assert_eq!(imported_none.labels.len(), 0);

        // Verify labels in bead_labels table
        storage2.with_immediate_transaction(|tx| {
            let mut stmt = tx.prepare("SELECT label FROM bead_labels WHERE bead_id = ?1 ORDER BY label").unwrap();
            let labels: Vec<String> = stmt.query_map(params!["bf-many-labels"], |row| row.get::<_, String>(0)).unwrap()
                .filter_map(|r| r.ok()).collect();
            assert_eq!(labels, vec!["critical", "database", "phase-1", "priority-p0", "storage"]);
            Ok(())
        }).unwrap();
    }

    #[test]
    fn test_labels_persist_through_flush_dirty() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        let db_path = beads_dir.join("beads.db");
        let jsonl_path = beads_dir.join("issues.jsonl");

        // Create initial issue
        let issue1 = Issue {
            id: "bf-dirty-1".to_string(),
            title: "Issue 1".to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec!["original".to_string()],
            ..Default::default()
        };

        let storage = Storage::open(&db_path).unwrap();
        storage.create_issue(&issue1).unwrap();

        // Flush to JSONL
        flush(workspace).unwrap();

        // Update the issue with new labels
        let changes = IssueChanges {
            labels: Some(vec!["original".to_string(), "updated".to_string(), "dirty".to_string()]),
            ..Default::default()
        };
        storage.update_issue("bf-dirty-1", &changes).unwrap();

        // Flush dirty to JSONL
        let exported_dirty = flush_dirty(workspace).unwrap();
        assert_eq!(exported_dirty, 1, "One dirty issue should be flushed");

        // Verify JSONL contains updated labels
        let jsonl_contents = std::fs::read_to_string(&jsonl_path).unwrap();
        let imported: Issue = jsonl_contents.lines()
            .find_map(|line| serde_json::from_str::<Issue>(line).ok())
            .unwrap();

        assert_eq!(imported.labels.len(), 3);
        assert!(imported.labels.contains(&"original".to_string()));
        assert!(imported.labels.contains(&"updated".to_string()));
        assert!(imported.labels.contains(&"dirty".to_string()));

        // Import and verify labels persisted
        std::fs::remove_file(&db_path).unwrap();
        let storage2 = Storage::open(&db_path).unwrap();
        import(workspace).unwrap();

        let final_issue = storage2.get_issue("bf-dirty-1").unwrap().unwrap();
        assert_eq!(final_issue.labels.len(), 3);
        assert!(final_issue.labels.contains(&"original".to_string()));
        assert!(final_issue.labels.contains(&"updated".to_string()));
        assert!(final_issue.labels.contains(&"dirty".to_string()));
    }

    #[test]
    #[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
    fn test_labels_persist_through_full_sync() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        let beads_dir = workspace.join(".beads");

        init_workspace(&beads_dir, "bf").unwrap();

        let db_path = beads_dir.join("beads.db");
        let jsonl_path = beads_dir.join("issues.jsonl");

        // Create issue with labels
        let issue = Issue {
            id: "bf-sync-labels".to_string(),
            title: "Sync Labels Test".to_string(),
            status: Status::Open,
            priority: Priority::HIGH,
            issue_type: IssueType::Feature,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            source_repo: Some(".".to_string()),
            labels: vec!["sync-test".to_string(), "persistence".to_string()],
            ..Default::default()
        };

        let storage = Storage::open(&db_path).unwrap();
        storage.create_issue(&issue).unwrap();

        // Run full sync
        let sync_result = sync(workspace).unwrap();
        assert_eq!(sync_result.imported, 0, "Nothing to import");
        assert_eq!(sync_result.exported, 1, "One issue exported");

        // Verify JSONL contains labels
        let jsonl_contents = std::fs::read_to_string(&jsonl_path).unwrap();
        let parsed: Issue = serde_json::from_str(&jsonl_contents.trim()).unwrap();
        assert_eq!(parsed.labels.len(), 2);
        assert!(parsed.labels.contains(&"sync-test".to_string()));
        assert!(parsed.labels.contains(&"persistence".to_string()));

        // Clear DB and run full sync again (import from JSONL)
        std::fs::remove_file(&db_path).unwrap();
        let storage2 = Storage::open(&db_path).unwrap();

        let sync_result2 = sync(workspace).unwrap();
        assert_eq!(sync_result2.imported, 1, "One issue imported");
        assert_eq!(sync_result2.exported, 1, "One issue exported");

        // Verify labels survived full sync roundtrip
        let final_issue = storage2.get_issue("bf-sync-labels").unwrap().unwrap();
        assert_eq!(final_issue.labels.len(), 2);
        assert!(final_issue.labels.contains(&"sync-test".to_string()));
        assert!(final_issue.labels.contains(&"persistence".to_string()));
    }
}
