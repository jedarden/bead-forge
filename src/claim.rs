use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use serde::Serialize;

/// Result of a claim operation
#[derive(Debug, Clone)]
pub struct ClaimResult {
    pub bead_id: String,
    pub reclaimed: usize,
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
/// * `now` - Current timestamp for staleness calculation
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

        // Step 3: Update the winner to in_progress
        tx.execute(
            "UPDATE issues
             SET status = 'in_progress', assignee = ?, updated_at = ?
             WHERE id = ?",
            params![worker, now.to_rfc3339(), &bead_id],
        )?;

        // Step 4: Insert event
        tx.execute(
            "INSERT INTO events (issue_id, event_type, actor, new_value, created_at)
             VALUES (?, 'assignee_changed', '', ?, ?)",
            params![&bead_id, worker, now.to_rfc3339()],
        )?;

        // Step 5: Mark as dirty
        tx.execute(
            "INSERT OR REPLACE INTO dirty_issues (issue_id, marked_at)
             VALUES (?, ?)",
            params![&bead_id, now.to_rfc3339()],
        )?;

        Ok(Some(ClaimResult {
            bead_id,
            reclaimed,
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
            claim(tx, "worker1", 30, Utc::now())
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
            claim(tx, "worker1", 30, Utc::now())
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
            claim(tx, "worker_new", 30, Utc::now())
        }).unwrap();

        assert!(result.is_some());
        let claim_result = result.unwrap();
        assert_eq!(claim_result.reclaimed, 1);

        // The open bead should be claimed (not the stale one, since it was reclaimed to open)
        // After reclaim, the stale bead is open again, so it could be claimed too
        // But the open bead has priority by created_at order
    }
}
