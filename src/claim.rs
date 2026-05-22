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
/// 1. combined_score / expected_seconds (impact per unit time, when velocity data available)
/// 2. downstream_impact (more blocking = higher priority)
/// 3. negative critical_float (lower float = more critical)
/// 4. negative priority (lower number = higher priority)
/// 5. negative created timestamp (older = higher priority/FIFO)
#[derive(Debug, Clone, Copy)]
pub struct Score {
    pub downstream_impact: i64,
    pub critical_float: i64,
    pub priority: i32,
    pub created_at_ts: i64,
    /// Expected duration in seconds from velocity_stats (None = no velocity data)
    pub expected_seconds: Option<i64>,
    /// Combined impact score (downstream * 3 + (4-priority) * 2 + critical_bonus)
    pub combined_score: f64,
}

impl PartialEq for Score {
    fn eq(&self, other: &Self) -> bool {
        self.downstream_impact == other.downstream_impact
            && self.critical_float == other.critical_float
            && self.priority == other.priority
            && self.created_at_ts == other.created_at_ts
            && self.expected_seconds == other.expected_seconds
            && self
                .combined_score
                .partial_cmp(&other.combined_score)
                .map(|o| o == std::cmp::Ordering::Equal)
                .unwrap_or(false)
    }
}

impl Eq for Score {}

impl Score {
    /// Create a new score from candidate fields.
    pub fn new(
        downstream_impact: i64,
        critical_float: i64,
        priority: i32,
        created_at_ts: i64,
        expected_seconds: Option<i64>,
        combined_score: f64,
    ) -> Self {
        Self {
            downstream_impact,
            critical_float,
            priority,
            created_at_ts,
            expected_seconds,
            combined_score,
        }
    }

    /// Get the velocity-adjusted score (impact per unit time).
    ///
    /// When expected_seconds is available, returns combined_score / expected_seconds.
    /// Otherwise returns combined_score (no adjustment).
    fn velocity_adjusted_score(&self) -> f64 {
        let expected_sec = self.expected_seconds.unwrap_or(1800);
        if expected_sec > 0 {
            self.combined_score / expected_sec as f64
        } else {
            self.combined_score
        }
    }
}

impl PartialOrd for Score {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Score {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Primary: velocity-adjusted score DESC (higher impact per unit time is better)
        match self
            .velocity_adjusted_score()
            .partial_cmp(&other.velocity_adjusted_score())
        {
            Some(std::cmp::Ordering::Equal) => {}
            Some(ord) => return ord.reverse(),
            None => {
                // NaN comparison - treat as equal and fall through to other fields
            }
        }
        // downstream_impact: DESC (higher is better)
        match other.downstream_impact.cmp(&self.downstream_impact) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        // critical_float: ASC (lower is better)
        match self.critical_float.cmp(&other.critical_float) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        // priority: ASC (lower number is better)
        match self.priority.cmp(&other.priority) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        // created_at_ts: ASC (older is better/FIFO)
        self.created_at_ts.cmp(&other.created_at_ts)
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
    // Step 0: Check migration_lock - return NONE if migration is in progress
    let lock_count: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM migration_lock WHERE expires_at > ?1",
            params![now.to_rfc3339()],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if lock_count > 0 {
        // Migration in progress - return None gracefully
        return Ok(None);
    }

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
    // Score = downstream_impact + critical_path_bonus
    // downstream_impact = count of beads blocked by this one
    // critical_path_bonus = 1000.0 / (float + 1) where float is from critical_path_cache
    // Zero-float beads get bonus = 1000, float-5 beads get ~167, non-critical beads get ~1
    // Velocity-aware scoring: prefer beads with lower expected duration
    // score = impact / expected_seconds (with fallback to standard scoring)
    let (model, harness) = if let Some(meta) = worker_metadata {
        (meta.model.clone(), meta.harness.clone())
    } else {
        (None, None)
    };

    // Velocity-aware claim: JOIN velocity_stats for requesting worker.
    // score = (impact * 3.0 + (4 - priority) * 2.0 + critical_path_bonus) / p50_seconds
    // Fallback p50_seconds = 1800 when no velocity data exists for this (model, harness, issue_type).
    if model.is_some() && harness.is_some() {
        let m = model.as_deref().unwrap_or("");
        let h = harness.as_deref().unwrap_or("");

        let mut stmt = tx.prepare(
            "SELECT i.id
             FROM issues i
             LEFT JOIN dependencies d ON d.depends_on_id = i.id
                 AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
             LEFT JOIN critical_path_cache c ON c.bead_id = i.id
             LEFT JOIN velocity_stats vs ON vs.issue_type = i.issue_type
                 AND vs.model = ?1
                 AND vs.harness = ?2
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
             ORDER BY (
                 COALESCE(COUNT(d.issue_id), 0) * 3.0
                 + (4 - i.priority) * 2.0
                 + 1000.0 / (COALESCE(c.float, 999) + 1)
             ) / COALESCE(vs.p50_seconds, 1800) DESC
             LIMIT 1",
        )?;

        let mut rows = stmt.query(params![m, h])?;

        if let Some(row) = rows.next()? {
            let bead_id: String = row.get(0)?;

            // Step 3: Update the winner to in_progress with a race condition check
            let rows_affected = tx.execute(
                "UPDATE issues
                 SET status = 'in_progress', assignee = ?, updated_at = ?
                 WHERE id = ? AND status = 'open'",
                params![worker, now.to_rfc3339(), &bead_id],
            )?;

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
                        "",
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

            return Ok(Some(ClaimResult {
                bead_id,
                reclaimed,
                workspace_path: None,
            }));
        }

        Ok(None)
    } else {
        // Standard scoring without velocity data (original SQL-based scoring)
        let mut stmt = tx.prepare(
            "SELECT i.id, i.issue_type,
                    COALESCE(COUNT(d.issue_id), 0) as downstream_impact,
                    1000.0 / (COALESCE(c.float, 999) + 1) as critical_path_bonus,
                    i.priority
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
                 critical_path_bonus DESC,
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
}

/// Get ready candidates using the same scoring logic as claim().
///
/// This returns a list of beads that would be considered for claiming,
/// ordered by the same scoring formula:
/// - velocity_adjusted_score DESC (impact per unit time, when velocity data available)
/// - downstream_impact DESC (more blocking = higher priority)
/// - critical_path_bonus DESC (1000.0/(float+1), higher bonus = more critical)
/// - priority ASC (0=Critical, 4=Backlog)
/// - created_at ASC (FIFO tiebreaker)
///
/// # Arguments
/// * `tx` - The transaction to use
/// * `limit` - Maximum number of candidates to return
/// * `model` - Optional model name for velocity-aware scoring
/// * `harness` - Optional harness name for velocity-aware scoring
///
/// # Returns
/// * `Ok(Vec<ScoredBead>)` - List of scored bead candidates
pub fn get_ready_candidates(
    tx: &Connection,
    limit: usize,
    model: Option<&str>,
    harness: Option<&str>,
) -> Result<Vec<ScoredBead>> {
    let mut stmt = if let (Some(_m), Some(_h)) = (model, harness) {
        // Velocity-aware scoring: divide combined score by expected seconds
        tx.prepare(
            "SELECT i.id, i.title, i.status, i.priority,
                    COALESCE(COUNT(d.issue_id), 0) as downstream_impact,
                    1000.0 / (COALESCE(c.float, 999) + 1) as critical_path_bonus,
                    i.created_at,
                    vs.p50_seconds as expected_seconds
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
               AND NOT EXISTS (
                   SELECT 1 FROM dependencies blocker_dep
                   INNER JOIN issues blocker ON blocker.id = blocker_dep.depends_on_id
                   WHERE blocker_dep.issue_id = i.id
                   AND blocker_dep.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
                   AND blocker.status != 'closed'
               )
             GROUP BY i.id
             ORDER BY (
                 COALESCE(COUNT(d.issue_id), 0) * 3.0
                 + (4 - i.priority) * 2.0
                 + 1000.0 / (COALESCE(c.float, 999) + 1)
             ) / COALESCE(vs.p50_seconds, 1800) DESC,
                 downstream_impact DESC,
                 critical_path_bonus DESC,
                 i.priority ASC,
                 i.created_at ASC
             LIMIT ?3",
        )?
    } else {
        // Standard scoring without velocity data
        tx.prepare(
            "SELECT i.id, i.title, i.status, i.priority,
                    COALESCE(COUNT(d.issue_id), 0) as downstream_impact,
                    1000.0 / (COALESCE(c.float, 999) + 1) as critical_path_bonus,
                    i.created_at
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
                 critical_path_bonus DESC,
                 i.priority ASC,
                 i.created_at ASC
             LIMIT ?1",
        )?
    };

    let mut rows = if model.is_some() && harness.is_some() {
        stmt.query(params![model.unwrap(), harness.unwrap(), limit as i64])?
    } else {
        stmt.query(params![limit as i64])?
    };

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

    // Extract model and harness for velocity-aware scoring
    let (model, harness) = if let Some(meta) = worker_metadata {
        (meta.model.as_deref(), meta.harness.as_deref())
    } else {
        (None, None)
    };

    // Score across all workspaces
    let mut best: Option<(Score, usize)> = None;
    for (idx, workspace_path) in workspace_paths.iter().enumerate() {
        let beads_dir = get_beads_dir(workspace_path)?;
        let metadata = load_metadata(&beads_dir)?;
        let db_path = beads_dir.join(&metadata.database);

        // Open each workspace's SQLite
        match Storage::open(&db_path) {
            Ok(storage) => {
                if let Some(score) = storage.top_candidate_score(model, harness)? {
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
    use crate::model::{Issue, Status};
    use crate::storage::Storage;

    fn setup_test_db() -> (tempfile::NamedTempFile, Storage) {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let storage = Storage::open(temp_file.path()).unwrap();
        (temp_file, storage)
    }

    #[test]
    fn test_claim_basic() {
        let (_temp, mut storage) = setup_test_db();

        // Create an open bead
        let issue = Issue::new(
            "bf-test1".to_string(),
            "Test bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&issue).unwrap();

        // Claim it
        let result = storage
            .with_immediate_transaction(|tx| claim(tx, "worker1", 30, Utc::now(), None))
            .unwrap();

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
        let result = storage
            .with_immediate_transaction(|tx| claim(tx, "worker1", 30, Utc::now(), None))
            .unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_claim_reclaims_stale() {
        let (_temp, mut storage) = setup_test_db();

        // Create an in_progress bead with old updated_at
        let mut issue = Issue::new(
            "bf-stale".to_string(),
            "Stale bead".to_string(),
            ".".to_string(),
        );
        issue.status = Status::InProgress;
        issue.assignee = Some("worker_old".to_string());
        issue.updated_at = Utc::now() - Duration::minutes(60);
        storage.create_issue(&issue).unwrap();

        // Create an open bead
        let issue2 = Issue::new(
            "bf-open".to_string(),
            "Open bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&issue2).unwrap();

        // Claim with 30 min TTL - should reclaim the stale one
        let result = storage
            .with_immediate_transaction(|tx| claim(tx, "worker_new", 30, Utc::now(), None))
            .unwrap();

        assert!(result.is_some());
        let claim_result = result.unwrap();
        assert_eq!(claim_result.reclaimed, 1);

        // The open bead should be claimed (not the stale one, since it was reclaimed to open)
        // After reclaim, the stale bead is open again, so it could be claimed too
        // But the open bead has priority by created_at order
    }

    #[test]
    fn test_concurrent_claim_no_double_claim() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let (_temp, storage) = setup_test_db();
        let storage = Arc::new(storage);

        // Create 20 open beads
        for i in 0..20 {
            let issue = Issue::new(
                format!("bf-{:0>4}", i),
                format!("Test bead {}", i),
                ".".to_string(),
            );
            storage.create_issue(&issue).unwrap();
        }

        // Spawn 20 workers trying to claim concurrently
        let claimed_beads: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let mut handles = vec![];

        for worker_id in 0..20 {
            let storage_clone = Arc::clone(&storage);
            let claimed_clone = Arc::clone(&claimed_beads);

            let handle = thread::spawn(move || {
                let result = storage_clone
                    .with_immediate_transaction(|tx| {
                        claim(tx, &format!("worker-{}", worker_id), 30, Utc::now(), None)
                    })
                    .unwrap();

                if let Some(claim_result) = result {
                    let mut claimed = claimed_clone.lock().unwrap();
                    claimed.push(claim_result.bead_id);
                }
            });

            handles.push(handle);
        }

        // Wait for all workers to complete
        for handle in handles {
            handle.join().unwrap();
        }

        let claimed = claimed_beads.lock().unwrap();

        // All 20 beads should be claimed exactly once
        assert_eq!(
            claimed.len(),
            20,
            "Expected 20 unique claims, got {}",
            claimed.len()
        );

        // No duplicates allowed
        let mut unique_beads = claimed.clone();
        unique_beads.sort();
        unique_beads.dedup();
        assert_eq!(
            unique_beads.len(),
            20,
            "Found duplicate claims: {:?}",
            claimed
        );
    }

    #[test]
    fn test_critical_path_bonus_in_claim() {
        use crate::storage::schema;

        let (_temp, mut storage) = setup_test_db();

        // Create a dependency chain: bf-critical (float=0) -> bf-blocked
        // And an independent bead: bf-independent (no critical path data)

        let critical = Issue::new(
            "bf-critical".to_string(),
            "Critical path bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&critical).unwrap();

        let blocked = Issue::new(
            "bf-blocked".to_string(),
            "Blocked bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&blocked).unwrap();

        let independent = Issue::new(
            "bf-independent".to_string(),
            "Independent bead".to_string(),
            ".".to_string(),
        );
        storage.create_issue(&independent).unwrap();

        // Add dependency: bf-blocked depends on bf-critical
        storage
            .add_dependency(
                "bf-blocked",
                "bf-critical",
                &crate::model::DependencyType::Blocks,
                "test",
            )
            .unwrap();

        // Manually populate critical_path_cache
        // (create_issue already calls compute_all_critical_paths, so use REPLACE)
        storage
            .with_immediate_transaction(|tx| {
                // First, clear any existing entries for our test beads
                tx.execute("DELETE FROM critical_path_cache WHERE bead_id IN ('bf-critical', 'bf-blocked', 'bf-independent')", [])?;

                // bf-critical has float=0 (on critical path)
                tx.execute(
                    "INSERT INTO critical_path_cache (bead_id, epic_id, es, ls, float, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        "bf-critical",
                        "bf-critical",
                        0i64,  // es
                        0i64,  // ls
                        0i64,  // float = ls - es = 0
                        Utc::now().to_rfc3339()
                    ],
                )?;

                // bf-blocked has float=1 (not on critical path)
                tx.execute(
                    "INSERT INTO critical_path_cache (bead_id, epic_id, es, ls, float, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        "bf-blocked",
                        "bf-critical",
                        1i64,  // es
                        2i64,  // ls
                        1i64,  // float = ls - es = 1
                        Utc::now().to_rfc3339()
                    ],
                )?;

                // bf-independent has no entry in critical_path_cache
                Ok(())
            })
            .unwrap();

        // Get ready candidates - should be ordered by critical path bonus
        let candidates = storage
            .with_immediate_transaction(|tx| get_ready_candidates(tx, 10, None, None))
            .unwrap();

        // bf-critical should be first (bonus=1000.0/1=1000)
        // bf-independent should be second (bonus=1000.0/1000=1)
        // bf-blocked is NOT in the list because it's blocked by bf-critical
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].id, "bf-critical");
        assert_eq!(candidates[1].id, "bf-independent");

        // Verify bonus values
        // critical_float stores the BONUS value, not the raw float
        assert!((candidates[0].critical_float - 1000.0).abs() < 0.01); // float=0 → bonus=1000
        assert!((candidates[1].critical_float - 1.0).abs() < 0.01); // no cache → bonus≈1
    }

    #[test]
    fn test_critical_path_zero_float_outranks_high_priority() {
        let (_temp, mut storage) = setup_test_db();

        // Create a zero-float bead with low priority (4=Backlog)
        let mut zero_float = Issue::new("bf-zero".to_string(), "Zero float".to_string(), ".".to_string());
        zero_float.priority = crate::model::Priority(4);
        storage.create_issue(&zero_float).unwrap();

        // Create a non-critical bead with high priority (0=Critical)
        let mut high_priority = Issue::new(
            "bf-high".to_string(),
            "High priority non-critical".to_string(),
            ".".to_string(),
        );
        high_priority.priority = crate::model::Priority(0);
        storage.create_issue(&high_priority).unwrap();

        // Populate critical_path_cache
        storage
            .with_immediate_transaction(|tx| {
                // Clear any existing entries first
                tx.execute("DELETE FROM critical_path_cache WHERE bead_id IN ('bf-zero', 'bf-high')", [])?;

                // bf-zero has float=0, priority=4
                tx.execute(
                    "INSERT INTO critical_path_cache (bead_id, epic_id, es, ls, float, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params!["bf-zero", "bf-zero", 0i64, 0i64, 0i64, Utc::now().to_rfc3339()],
                )?;

                // bf-high has no critical path entry (float defaults to 999)
                Ok(())
            })
            .unwrap();

        // Get ready candidates
        let candidates = storage
            .with_immediate_transaction(|tx| get_ready_candidates(tx, 10, None, None))
            .unwrap();

        assert_eq!(candidates.len(), 2);
        // Zero-float with low priority should outrank non-critical with high priority
        // because the bonus (1000) is much larger than priority difference
        assert_eq!(candidates[0].id, "bf-zero");
        assert_eq!(candidates[0].priority, 4);
        assert_eq!(candidates[1].id, "bf-high");
        assert_eq!(candidates[1].priority, 0);
    }
}
