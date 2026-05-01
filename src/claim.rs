use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Worker metadata for tracking which model/harness claimed a bead
#[derive(Debug, Clone, Serialize)]
pub struct WorkerMetadata {
    pub worker_id: String,
    pub model: Option<String>,
    pub harness: Option<String>,
    pub harness_version: Option<String>,
}

/// Result of a claim operation
#[derive(Debug, Clone)]
pub struct ClaimResult {
    pub bead_id: String,
    pub reclaimed: usize,
    pub workspace_path: Option<PathBuf>,
}

/// Score for cross-workspace candidate comparison.
///
/// Higher scores are better. Ordered by:
/// 1. downstream_impact (more blocking = higher priority)
/// 2. negative critical_float (lower float = more critical)
/// 3. negative priority (lower number = higher priority)
/// 4. negative created timestamp (older = higher priority/FIFO)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score {
    pub downstream_impact: i64,
    pub critical_float: i64,
    pub priority: i32,
    pub created_at_ts: i64,
}

impl Score {
    /// Create a new score from candidate fields.
    pub fn new(downstream_impact: i64, critical_float: i64, priority: i32, created_at_ts: i64) -> Self {
        Self { downstream_impact, critical_float, priority, created_at_ts }
    }
}

/// A bead with its score for ready/claim operations
#[derive(Debug, Clone, Serialize)]
pub struct ScoredBead {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: i32,
    pub downstream_impact: i64,
    pub critical_float: f64,
    pub created_at: String,
}

/// Atomically claim a bead for a worker.
///
/// This function performs the following in a single IMMEDIATE transaction:
/// 1. Reclaim stale in_progress beads (older than claim_ttl_minutes) back to open
/// 2. Select candidates with downstream_impact + critical_float scoring
/// 3. Update the winner to in_progress with assignee=worker
/// 4. Insert an event
/// 5. Mark the bead as dirty
/// 6. Commit
///
/// # Arguments
/// * `tx` - The transaction to use (must be an IMMEDIATE transaction)
/// * `worker` - The worker ID claiming the bead
/// * `claim_ttl_minutes` - TTL in minutes after which in_progress beads are reclaimed
/// * `worker_metadata` - Optional worker metadata (model, harness, version)
///
/// # Returns
/// * `Ok(Some(claim_result))` - A bead was claimed
/// * `Ok(None)` - No beads available to claim
/// * `Err(e)` - Transaction error
pub fn claim(
    tx: &Connection,
    worker: &str,
    claim_ttl_minutes: i64,
    now: DateTime<Utc>,
    worker_metadata: Option<&WorkerMetadata>,
) -> Result<Option<ClaimResult>> {

    // Step 1: Reclaim stale in_progress beads
    let stale_cutoff = now - Duration::minutes(claim_ttl_minutes);
    let reclaimed = tx.execute(
        "UPDATE issues
         SET status = 'open', assignee = NULL, updated_at = ?
         WHERE status = 'in_progress'
           AND updated_at < ?",
        params![now.to_rfc3339(), stale_cutoff.to_rfc3339()],
    )?;

    // Step 2: Find candidate beads with impact scoring
    // Score = downstream_impact + (critical_float / 1000.0)
    // downstream_impact = count of beads blocked by this one
    // critical_float = from critical_path_cache (lower is more critical)
    let mut stmt = tx.prepare(
        "SELECT i.id,
                COALESCE(COUNT(d.issue_id), 0) as downstream_impact,
                COALESCE(c.float, 999999) as critical_float,
                i.priority
         FROM issues i
         LEFT JOIN dependencies d ON d.depends_on_id = i.id AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
         LEFT JOIN critical_path_cache c ON c.bead_id = i.id
         WHERE i.status = 'open'
           AND i.ephemeral = 0
           AND i.pinned = 0
           AND i.is_template = 0
           AND i.deleted_at IS NULL
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
        let bead_id: String = row.get(0)?;

        // Step 3: Update the winner to in_progress with a race condition check
        // The WHERE status = 'open' condition ensures we only claim if still open
        let rows_affected = tx.execute(
            "UPDATE issues
             SET status = 'in_progress', assignee = ?, updated_at = ?
             WHERE id = ? AND status = 'open'",
            params![worker, now.to_rfc3339(), &bead_id],
        )?;

        // If no rows were affected, another worker claimed this bead first
        if rows_affected == 0 {
            return Ok(None);
        }

        // Step 4: Record worker session if metadata provided
        if let Some(meta) = worker_metadata {
            tx.execute(
                "INSERT INTO worker_sessions (worker_id, model, harness, harness_version, bead_id, workspace_path)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    &meta.worker_id,
                    meta.model.as_deref(),
                    meta.harness.as_deref(),
                    meta.harness_version.as_deref(),
                    &bead_id,
                    "", // workspace_path not available in transaction context
                ],
            )?;
        }

        // Step 5: Insert event with worker metadata in comment field
        let metadata_json = worker_metadata.and_then(|m| serde_json::to_string(m).ok());
        tx.execute(
            "INSERT INTO events (issue_id, event_type, actor, new_value, comment, created_at)
             VALUES (?, 'claimed', ?, ?, ?, ?)",
            params![&bead_id, worker, worker, metadata_json, now.to_rfc3339()],
        )?;

        // Step 6: Mark as dirty
        tx.execute(
            "INSERT OR REPLACE INTO dirty_issues (issue_id, marked_at)
             VALUES (?, ?)",
            params![&bead_id, now.to_rfc3339()],
        )?;

        Ok(Some(ClaimResult {
            bead_id,
            reclaimed,
            workspace_path: None,
        }))
    } else {
        Ok(None)
    }
}

/// Get ready candidates using the same scoring logic as claim().
///
/// This returns a list of beads that would be considered for claiming,
/// ordered by the same scoring formula:
/// - downstream_impact DESC (more blocking = higher priority)
/// - critical_float ASC (lower float = more critical)
/// - priority ASC (0=Critical, 4=Backlog)
/// - created_at ASC (FIFO tiebreaker)
///
/// # Arguments
/// * `tx` - The transaction to use
/// * `limit` - Maximum number of candidates to return
///
/// # Returns
/// * `Ok(Vec<ScoredBead>)` - List of scored bead candidates
pub fn get_ready_candidates(tx: &Connection, limit: usize) -> Result<Vec<ScoredBead>> {
    let mut stmt = tx.prepare(
        "SELECT i.id, i.title, i.status, i.priority,
                COALESCE(COUNT(d.issue_id), 0) as downstream_impact,
                COALESCE(c.float, 999999) as critical_float,
                i.created_at
         FROM issues i
         LEFT JOIN dependencies d ON d.depends_on_id = i.id AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
         LEFT JOIN critical_path_cache c ON c.bead_id = i.id
         WHERE i.status = 'open'
           AND i.ephemeral = 0
           AND i.pinned = 0
           AND i.is_template = 0
           AND i.deleted_at IS NULL
         GROUP BY i.id
         ORDER BY
             downstream_impact DESC,
             critical_float ASC,
             i.priority ASC,
             i.created_at ASC
         LIMIT ?1",
    )?;

    let mut rows = stmt.query(params![limit as i64])?;
    let mut candidates = Vec::new();

    while let Some(row) = rows.next()? {
        candidates.push(ScoredBead {
            id: row.get(0)?,
            title: row.get(1)?,
            status: row.get(2)?,
            priority: row.get(3)?,
            downstream_impact: row.get(4)?,
            critical_float: row.get(5)?,
            created_at: row.get(6)?,
        });
    }

    Ok(candidates)
}

/// Claim from the highest-priority bead across multiple workspaces.
///
/// Scores each workspace's top candidate, picks the global winner,
/// and claims from that workspace.
///
/// # Arguments
/// * `workspace_paths` - Slice of workspace directory paths
/// * `worker` - The worker ID claiming the bead
/// * `claim_ttl_minutes` - TTL in minutes after which in_progress beads are reclaimed
/// * `worker_metadata` - Optional worker metadata (model, harness, version)
///
/// # Returns
/// * `Ok(Some(claim_result))` - A bead was claimed (with workspace_path set)
/// * `Ok(None)` - No beads available to claim in any workspace
/// * `Err(e)` - Transaction error
pub fn claim_any(
    workspace_paths: &[PathBuf],
    worker: &str,
    claim_ttl_minutes: i64,
    worker_metadata: Option<&WorkerMetadata>,
) -> Result<Option<ClaimResult>> {
    use crate::config::load_metadata;
    use crate::storage::Storage;

    // Score across all workspaces
    let mut best: Option<(Score, usize)> = None;
    for (idx, workspace_path) in workspace_paths.iter().enumerate() {
        let beads_dir = get_beads_dir(workspace_path)?;
        let metadata = load_metadata(&beads_dir)?;
        let db_path = beads_dir.join(&metadata.database);

        // Open each workspace's SQLite
        match Storage::open(&db_path) {
            Ok(storage) => {
                if let Some(score) = storage.top_candidate_score()? {
                    if best.as_ref().map(|(b, _)| score > *b).unwrap_or(true) {
                        best = Some((score, idx));
                    }
                }
            }
            Err(_) => {
                // Skip workspaces that can't be opened (e.g., no .beads directory)
                continue;
            }
        }
    }

    match best {
        None => Ok(None),
        Some((_, workspace_idx)) => {
            let workspace_path = &workspace_paths[workspace_idx];
            let beads_dir = get_beads_dir(workspace_path)?;
            let metadata = load_metadata(&beads_dir)?;
            let db_path = beads_dir.join(&metadata.database);
            let storage = Storage::open(&db_path)?;

            let now = Utc::now();
            match storage.with_immediate_transaction(|tx| {
                claim(tx, worker, claim_ttl_minutes, now, worker_metadata)
            })? {
                Some(mut result) => {
                    result.workspace_path = Some(workspace_path.clone());
                    Ok(Some(result))
                }
                None => Ok(None),
            }
        }
    }
}

/// Get the .beads directory from a workspace path.
///
/// If the workspace path itself contains a .beads directory, use it.
/// Otherwise, assume the path IS the .beads directory.
fn get_beads_dir(workspace_path: &Path) -> Result<std::path::PathBuf> {
    let beads_dir = workspace_path.join(".beads");
    if beads_dir.is_dir() {
        Ok(beads_dir)
    } else if workspace_path.ends_with(".beads") {
        Ok(workspace_path.to_path_buf())
    } else {
        use anyhow::bail;
        bail!("No .beads directory found in {:?}", workspace_path)
    }
}

/// Find all bead workspace directories starting from a search path.
///
/// Searches for directories containing a .beads subdirectory.
/// Searches upward from the start path through parent directories.
pub fn find_workspaces(start_path: &Path) -> Result<Vec<PathBuf>> {

    let mut workspaces = Vec::new();

    // Start from the given path and search upward
    let mut current = start_path.to_path_buf();
    loop {
        let beads_dir = current.join(".beads");
        if beads_dir.is_dir() {
            // Found a workspace - add the parent directory
            workspaces.push(current.clone());
        }

        // Move to parent directory
        if !current.pop() {
            // Reached the root, stop searching
            break;
        }
    }

    Ok(workspaces)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use crate::model::{Issue, Status};

    fn setup_test_db() -> (tempfile::NamedTempFile, Storage) {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();
        (temp_file, storage)
    }

    #[test]
    fn test_claim_basic() {
        let (_temp, mut storage) = setup_test_db();

        // Create an open bead
        let issue = Issue::new("bf-test1".to_string(), "Test bead".to_string(), ".".to_string());
        storage.create_issue(&issue).unwrap();

        // Claim it
        let result = storage.with_immediate_transaction(|tx| {
            claim(tx, "worker1", 30, Utc::now(), None)
        }).unwrap();

        assert!(result.is_some());
        let claim_result = result.unwrap();
        assert_eq!(claim_result.bead_id, "bf-test1");
        assert_eq!(claim_result.reclaimed, 0);

        // Verify the bead is now in_progress
        let updated = storage.get_issue("bf-test1").unwrap().unwrap();
        assert_eq!(updated.status, Status::InProgress);
        assert_eq!(updated.assignee.as_ref().unwrap(), "worker1");
    }

    #[test]
    fn test_claim_no_candidates() {
        let (_temp, mut storage) = setup_test_db();

        // No beads available
        let result = storage.with_immediate_transaction(|tx| {
            claim(tx, "worker1", 30, Utc::now(), None)
        }).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_claim_reclaims_stale() {
        let (_temp, mut storage) = setup_test_db();

        // Create an in_progress bead with old updated_at
        let mut issue = Issue::new("bf-stale".to_string(), "Stale bead".to_string(), ".".to_string());
        issue.status = Status::InProgress;
        issue.assignee = Some("worker_old".to_string());
        issue.updated_at = Utc::now() - Duration::minutes(60);
        storage.create_issue(&issue).unwrap();

        // Create an open bead
        let issue2 = Issue::new("bf-open".to_string(), "Open bead".to_string(), ".".to_string());
        storage.create_issue(&issue2).unwrap();

        // Claim with 30 min TTL - should reclaim the stale one
        let result = storage.with_immediate_transaction(|tx| {
            claim(tx, "worker_new", 30, Utc::now(), None)
        }).unwrap();

        assert!(result.is_some());
        let claim_result = result.unwrap();
        assert_eq!(claim_result.reclaimed, 1);

        // The open bead should be claimed (not the stale one, since it was reclaimed to open)
        // After reclaim, the stale bead is open again, so it could be claimed too
        // But the open bead has priority by created_at order
    }
}
