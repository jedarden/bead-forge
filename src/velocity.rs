//! Worker performance tracking and velocity-aware claim scoring.
//!
//! This module tracks how long workers take to complete beads based on their
//! (model, harness, issue_type) configuration and uses this data to inform
//! claim scoring.
//!
//! ## Data Flow
//!
//! 1. When a bead is claimed, `worker_sessions` row is created (in claim.rs)
//! 2. When a bead is closed, the session is updated with `closed_at` and `duration_seconds`
//! 3. Velocity stats are recomputed from the last 50 sessions per (model, harness, issue_type)
//! 4. Claim scoring uses expected_seconds from velocity_stats: score = impact / expected_seconds

use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Velocity statistics for a (model, harness, issue_type) tuple.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VelocityStats {
    pub model: String,
    pub harness: String,
    pub issue_type: String,
    pub sample_count: i64,
    pub p50_seconds: Option<i64>,
    pub p90_seconds: Option<i64>,
    pub avg_seconds: Option<f64>,
    pub last_updated: Option<String>,
}

/// Worker session record from worker_sessions table.
#[derive(Debug, Clone)]
pub struct WorkerSession {
    pub worker_id: String,
    pub model: Option<String>,
    pub harness: Option<String>,
    pub harness_version: Option<String>,
    pub claimed_at: String,
    pub bead_id: Option<String>,
    pub workspace_path: String,
    pub closed_at: Option<String>,
    pub duration_seconds: Option<i64>,
}

/// Update a worker session with close time and duration.
///
/// Called when a bead is closed. Finds the most recent claim session for the bead
/// and updates it with closed_at and duration_seconds.
///
/// # Arguments
/// * `tx` - The transaction to use
/// * `bead_id` - The bead that was closed
/// * `closed_at` - When the bead was closed
///
/// # Returns
/// * `Ok(true)` if a session was found and updated
/// * `Ok(false)` if no matching session was found
pub fn update_session_on_close(
    tx: &Connection,
    bead_id: &str,
    closed_at: DateTime<Utc>,
) -> Result<bool> {
    // Find the most recent session for this bead
    let session = tx
        .query_row(
            "SELECT ws.claimed_at, ws.model, ws.harness, i.issue_type
             FROM worker_sessions ws
             INNER JOIN issues i ON i.id = ws.bead_id
             WHERE ws.bead_id = ?1
             ORDER BY ws.claimed_at DESC
             LIMIT 1",
            params![bead_id],
            |row| {
                let claimed_at: String = row.get(0)?;
                let model: Option<String> = row.get(1)?;
                let harness: Option<String> = row.get(2)?;
                let issue_type: Option<String> = row.get(3)?;
                Ok((claimed_at, model, harness, issue_type))
            },
        )
        .optional()?;

    let (claimed_at_str, model, harness, issue_type): (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    ) = match session {
        None => return Ok(false), // No worker session exists for this bead
        Some(s) => s,
    };

    // Parse claimed_at and calculate duration
    let claimed_at = DateTime::parse_from_rfc3339(&claimed_at_str)
        .map_err(|e| anyhow::anyhow!("Invalid claimed_at format: {}", e))?
        .with_timezone(&Utc);

    let duration_seconds = closed_at.signed_duration_since(claimed_at).num_seconds();

    // Update the session with closed_at and duration_seconds
    let rows_updated = tx.execute(
        "UPDATE worker_sessions
         SET closed_at = ?1, duration_seconds = ?2
         WHERE bead_id = ?3 AND claimed_at = ?4",
        params![
            closed_at.to_rfc3339(),
            duration_seconds,
            bead_id,
            claimed_at_str
        ],
    )?;

    if rows_updated > 0 {
        // Recompute velocity stats for this (model, harness, issue_type) tuple
        if let (Some(m), Some(h), Some(it)) = (model, harness, issue_type) {
            let _ = recompute_velocity_stats(tx, &m, &h, &it);
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Recompute velocity statistics for a given (model, harness, issue_type) tuple.
///
/// Queries the last 50 completed sessions for the tuple and computes
/// p50, p90, and average duration in seconds.
///
/// # Arguments
/// * `tx` - The transaction to use
/// * `model` - The model name
/// * `harness` - The harness name
/// * `issue_type` - The issue type
fn recompute_velocity_stats(
    tx: &Connection,
    model: &str,
    harness: &str,
    issue_type: &str,
) -> Result<()> {
    // Get last 50 completed sessions for this tuple
    let mut stmt = tx.prepare(
        "SELECT duration_seconds
         FROM worker_sessions ws
         INNER JOIN issues i ON i.id = ws.bead_id
         WHERE ws.model = ?1
           AND ws.harness = ?2
           AND i.issue_type = ?3
           AND ws.duration_seconds IS NOT NULL
         ORDER BY ws.closed_at DESC
         LIMIT 50",
    )?;

    let mut durations: Vec<i64> = stmt
        .query_map(params![model, harness, issue_type], |row| row.get(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let sample_count = durations.len() as i64;
    let (p50_seconds, p90_seconds, avg_seconds) = if durations.is_empty() {
        (None, None, None)
    } else {
        durations.sort_unstable();
        let len = durations.len();

        // Calculate percentiles
        let p50_idx = (len as f64 * 0.5).floor() as usize;
        let p90_idx = (len as f64 * 0.9).floor() as usize;
        let p50 = Some(durations[p50_idx]);
        let p90 = if len > 1 {
            Some(durations[p90_idx])
        } else {
            p50
        };

        // Calculate average
        let sum: i64 = durations.iter().sum();
        let avg = Some(sum as f64 / len as f64);

        (p50, p90, avg)
    };

    // Upsert into velocity_stats
    tx.execute(
        "INSERT OR REPLACE INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            model,
            harness,
            issue_type,
            sample_count,
            p50_seconds,
            p90_seconds,
            avg_seconds,
            Utc::now().to_rfc3339(),
        ],
    )?;

    Ok(())
}

/// Get expected duration in seconds for a (model, harness, issue_type) tuple.
///
/// Returns the p50 (median) duration if available, otherwise None.
///
/// # Arguments
/// * `tx` - The transaction to use
/// * `model` - The model name (or empty string for unknown)
/// * `harness` - The harness name (or empty string for unknown)
/// * `issue_type` - The issue type
///
/// # Returns
/// * `Ok(Some(seconds))` - Expected duration in seconds
/// * `Ok(None)` - No velocity data available for this tuple
pub fn get_expected_seconds(
    tx: &Connection,
    model: &str,
    harness: &str,
    issue_type: &str,
) -> Result<Option<i64>> {
    // Try exact match first
    let result: Option<i64> = tx.query_row(
        "SELECT p50_seconds
         FROM velocity_stats
         WHERE model = ?1 AND harness = ?2 AND issue_type = ?3
           AND sample_count >= 3",
        params![model, harness, issue_type],
        |row| row.get(0),
    )?;

    if let Some(seconds) = result {
        return Ok(Some(seconds));
    }

    // Fallback: try with just model and issue_type (any harness)
    let result: Option<i64> = tx.query_row(
        "SELECT p50_seconds
         FROM velocity_stats
         WHERE model = ?1 AND harness = '' AND issue_type = ?3
           AND sample_count >= 3",
        params![model, "", issue_type],
        |row| row.get(0),
    )?;

    if let Some(seconds) = result {
        return Ok(Some(seconds));
    }

    // Fallback: try with just issue_type (any model/harness)
    let result: Option<i64> = tx.query_row(
        "SELECT p50_seconds
         FROM velocity_stats
         WHERE model = '' AND harness = '' AND issue_type = ?1
           AND sample_count >= 10",
        params![issue_type],
        |row| row.get(0),
    )?;

    Ok(result)
}

/// Get all velocity statistics.
///
/// Returns a list of all (model, harness, issue_type) tuples with their stats.
///
/// # Arguments
/// * `tx` - The transaction to use
///
/// # Returns
/// * `Ok(Vec<VelocityStats>)` - List of velocity stats
pub fn get_all_velocity_stats(tx: &Connection) -> Result<Vec<VelocityStats>> {
    let mut stmt = tx.prepare(
        "SELECT model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated
         FROM velocity_stats
         ORDER BY sample_count DESC",
    )?;

    let mut stats = Vec::new();
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        stats.push(VelocityStats {
            model: row.get(0)?,
            harness: row.get(1)?,
            issue_type: row.get(2)?,
            sample_count: row.get(3)?,
            p50_seconds: row.get(4)?,
            p90_seconds: row.get(5)?,
            avg_seconds: row.get(6)?,
            last_updated: row.get(7)?,
        });
    }

    Ok(stats)
}

/// Get velocity statistics filtered by model and/or harness.
///
/// # Arguments
/// * `tx` - The transaction to use
/// * `model_filter` - Optional model filter
/// * `harness_filter` - Optional harness filter
///
/// # Returns
/// * `Ok(Vec<VelocityStats>)` - List of velocity stats
pub fn get_velocity_stats(
    tx: &Connection,
    model_filter: Option<&str>,
    harness_filter: Option<&str>,
) -> Result<Vec<VelocityStats>> {
    let mut query = String::from(
        "SELECT model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated
         FROM velocity_stats WHERE 1=1",
    );

    let mut params = Vec::new();
    let mut param_idx = 1;

    if let Some(model) = model_filter {
        query.push_str(&format!(" AND model = ?{}", param_idx));
        params.push(model.to_string());
        param_idx += 1;
    }

    if let Some(harness) = harness_filter {
        query.push_str(&format!(" AND harness = ?{}", param_idx));
        params.push(harness.to_string());
        param_idx += 1;
    }

    query.push_str(" ORDER BY sample_count DESC");

    let mut stmt = tx.prepare(&query)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();

    let mut stats = Vec::new();
    let mut rows = stmt.query(param_refs.as_slice())?;

    while let Some(row) = rows.next()? {
        stats.push(VelocityStats {
            model: row.get(0)?,
            harness: row.get(1)?,
            issue_type: row.get(2)?,
            sample_count: row.get(3)?,
            p50_seconds: row.get(4)?,
            p90_seconds: row.get(5)?,
            avg_seconds: row.get(6)?,
            last_updated: row.get(7)?,
        });
    }

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Issue, IssueType, Status};
    use crate::storage::schema::apply_schema;

    fn setup_test_db() -> tempfile::NamedTempFile {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();
        apply_schema(&conn).unwrap();
        temp_file
    }

    #[test]
    fn test_update_session_on_close() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create a test issue
        let mut issue = Issue::new("bf-test1".to_string(), "Test".to_string(), ".".to_string());
        issue.issue_type = IssueType::Task;
        conn.execute(
            "INSERT INTO issues (id, title, status, issue_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &issue.id,
                &issue.title,
                "in_progress",
                "task",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();

        // Create a worker session
        let claimed_at = Utc::now() - chrono::Duration::minutes(10);
        conn.execute(
            "INSERT INTO worker_sessions (worker_id, model, harness, bead_id, claimed_at, workspace_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params!["worker1", "claude-4.7", "cli", "bf-test1", claimed_at.to_rfc3339(), "."],
        ).unwrap();

        // Close the bead
        let closed_at = Utc::now();
        let updated = update_session_on_close(&conn, "bf-test1", closed_at).unwrap();

        assert!(updated);

        // Verify the session was updated
        let (session_closed_at, duration): (Option<String>, Option<i64>) = conn
            .query_row(
                "SELECT closed_at, duration_seconds FROM worker_sessions WHERE bead_id = ?1",
                params!["bf-test1"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert!(session_closed_at.is_some());
        assert!(duration.is_some());
        // 10 minutes = 600 seconds (allow some tolerance)
        assert!(duration.unwrap() >= 590 && duration.unwrap() <= 610);
    }

    #[test]
    fn test_recompute_velocity_stats() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create test issues and sessions
        for i in 0..10 {
            let bead_id = format!("bf-test{}", i);
            let created_at = Utc::now();
            let closed_at = created_at + chrono::Duration::seconds(100 + (i as i64));
            conn.execute(
                "INSERT INTO issues (id, title, status, issue_type, created_at, updated_at, closed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    &bead_id,
                    &format!("Test {}", i),
                    "closed",
                    "task",
                    created_at.to_rfc3339(),
                    created_at.to_rfc3339(),
                    closed_at.to_rfc3339(),
                ],
            ).unwrap();

            let claimed_at = created_at - chrono::Duration::seconds((i * 100) as i64);

            conn.execute(
                "INSERT INTO worker_sessions (worker_id, model, harness, bead_id, claimed_at, closed_at, duration_seconds, workspace_path)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "worker1",
                    "claude-4.7",
                    "cli",
                    &bead_id,
                    claimed_at.to_rfc3339(),
                    closed_at.to_rfc3339(),
                    100 + i as i64,
                    ".",
                ],
            ).unwrap();
        }

        // Recompute stats
        recompute_velocity_stats(&conn, "claude-4.7", "cli", "task").unwrap();

        // Verify stats were computed
        let (count, p50, avg): (i64, Option<i64>, Option<f64>) = conn
            .query_row(
                "SELECT sample_count, p50_seconds, avg_seconds FROM velocity_stats
             WHERE model = ?1 AND harness = ?2 AND issue_type = ?3",
                params!["claude-4.7", "cli", "task"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert_eq!(count, 10);
        assert!(p50.is_some());
        assert!(avg.is_some());
    }
}
