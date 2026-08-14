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

/// Parse datetime string that may be RFC3339 or SQLite native format.
///
/// Handles both RFC3339 with timezone (e.g., "2026-05-15T21:10:36+00:00") and
/// SQLite's native datetime format without timezone (e.g., "2026-05-15 21:10:36").
/// Assumes UTC for SQLite-native format.
///
/// Returns an error for empty or malformed datetime strings.
fn parse_datetime(s: &str) -> Result<DateTime<Utc>> {
    use chrono::NaiveDateTime;
    let t = s.trim();
    // Early reject of empty strings (these should be NULL in the database, not empty strings)
    if t.is_empty() {
        return Err(anyhow::anyhow!("Invalid claimed_at format: empty string"));
    }
    // Try RFC3339 first (bf's own format with timezone; optional fractional seconds)
    match DateTime::parse_from_rfc3339(t) {
        Ok(dt) => Ok(dt.with_timezone(&Utc)),
        Err(_) => {
            // SQLite-native datetime() format: no timezone, space or 'T' separator
            for fmt in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"] {
                if let Ok(ndt) = NaiveDateTime::parse_from_str(t, fmt) {
                    return Ok(ndt.and_utc());
                }
            }
            Err(anyhow::anyhow!("Invalid claimed_at format: {}", t))
        }
    }
}

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
    // Find the most recent session for this bead with a valid claimed_at timestamp
    // Filter out empty strings and NULL values to avoid parse errors
    let session = tx
        .query_row(
            "SELECT ws.claimed_at, ws.model, ws.harness, i.issue_type
             FROM worker_sessions ws
             INNER JOIN issues i ON i.id = ws.bead_id
             WHERE ws.bead_id = ?1
             AND ws.claimed_at IS NOT NULL
             AND ws.claimed_at != ''
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

    // Parse claimed_at and calculate duration. Sessions recorded via claim.rs's
    // INSERT column list previously omitted claimed_at, silently falling back to
    // SQLite's non-RFC3339 CURRENT_TIMESTAMP default ("YYYY-MM-DD HH:MM:SS"); accept
    // that legacy format too (parse_datetime already does), and skip velocity
    // tracking for this row -- instead of erroring the whole close -- if it's
    // still unparseable (e.g. empty/corrupt).
    let claimed_at = match parse_datetime(&claimed_at_str) {
        Ok(dt) => dt,
        Err(_) => return Ok(false),
    };

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
    let result: Option<i64> = tx
        .query_row(
            "SELECT p50_seconds
             FROM velocity_stats
             WHERE model = ?1 AND harness = ?2 AND issue_type = ?3
               AND sample_count >= 3",
            params![model, harness, issue_type],
            |row| row.get(0),
        )
        .optional()?
        .flatten();

    if let Some(seconds) = result {
        return Ok(Some(seconds));
    }

    // Fallback: try with just model and issue_type (any harness)
    let result: Option<i64> = tx
        .query_row(
            "SELECT p50_seconds
             FROM velocity_stats
             WHERE model = ?1 AND harness = '' AND issue_type = ?3
               AND sample_count >= 3",
            params![model, "", issue_type],
            |row| row.get(0),
        )
        .optional()?
        .flatten();

    if let Some(seconds) = result {
        return Ok(Some(seconds));
    }

    // Fallback: try with just issue_type (any model/harness)
    let result: Option<i64> = tx
        .query_row(
            "SELECT p50_seconds
             FROM velocity_stats
             WHERE model = '' AND harness = '' AND issue_type = ?1
               AND sample_count >= 10",
            params![issue_type],
            |row| row.get(0),
        )
        .optional()?
        .flatten();

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
    use chrono::Datelike;
    use chrono::Timelike;

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

    #[test]
    fn test_parse_datetime_rfc3339_with_timezone() {
        // Test RFC3339 format with timezone
        let result = parse_datetime("2026-05-15T21:10:36+00:00").unwrap();
        // Verify it's a valid datetime by checking year and month
        assert_eq!(result.year(), 2026);
        assert_eq!(result.month(), 5);
        assert_eq!(result.day(), 15);
        assert_eq!(result.hour(), 21);
        assert_eq!(result.minute(), 10);
        assert_eq!(result.second(), 36);

        // Test RFC3339 format with fractional seconds
        let result = parse_datetime("2026-05-15T21:10:36.123+00:00").unwrap();
        assert_eq!(result.year(), 2026);
        assert_eq!(result.month(), 5);
        assert_eq!(result.day(), 15);

        // Test RFC3339 format with non-UTC timezone
        let result = parse_datetime("2026-05-15T17:10:36-04:00").unwrap();
        // Should be converted to UTC (17:10-04:00 = 21:10+00:00)
        assert_eq!(result.hour(), 21);
        assert_eq!(result.minute(), 10);
    }

    #[test]
    fn test_parse_datetime_sqlite_native_without_timezone() {
        // Test SQLite native format with space separator
        let result = parse_datetime("2026-05-15 21:10:36").unwrap();
        assert_eq!(result.year(), 2026);
        assert_eq!(result.month(), 5);
        assert_eq!(result.day(), 15);
        assert_eq!(result.hour(), 21);
        assert_eq!(result.minute(), 10);
        assert_eq!(result.second(), 36);

        // Test SQLite native format with T separator
        let result = parse_datetime("2026-05-15T21:10:36").unwrap();
        assert_eq!(result.year(), 2026);
        assert_eq!(result.month(), 5);
        assert_eq!(result.day(), 15);

        // Test edge case: different valid datetime
        let result = parse_datetime("2024-01-01 12:00:00").unwrap();
        assert_eq!(result.year(), 2024);
        assert_eq!(result.month(), 1);
        assert_eq!(result.day(), 1);
        assert_eq!(result.hour(), 12);
    }

    #[test]
    fn test_parse_datetime_empty_string_rejection() {
        // Test empty string
        let result = parse_datetime("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty string"));

        // Test whitespace-only string (trimmed to empty)
        let result = parse_datetime("   ");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty string"));
    }

    #[test]
    fn test_parse_datetime_malformed_datetime_rejection() {
        // Test completely invalid format
        let result = parse_datetime("not-a-date");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid claimed_at format"));

        // Test partial date (missing time)
        let result = parse_datetime("2026-05-15");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid claimed_at format"));

        // Test invalid date values
        let result = parse_datetime("2026-13-01 12:00:00"); // Invalid month
        assert!(result.is_err());

        // Test invalid time values
        let result = parse_datetime("2026-05-15 25:00:00"); // Invalid hour
        assert!(result.is_err());

        // Test format with invalid separator
        let result = parse_datetime("2026/05/15 21:10:36"); // Wrong separator
        assert!(result.is_err());

        // Test RFC3339-like but missing timezone
        let result = parse_datetime("2026-05-15T21:10:36"); // Missing timezone
                                                            // This should fall through to SQLite format parsing
                                                            // and succeed since SQLite format accepts this
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_session_on_close_with_empty_claimed_at() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create a test issue
        conn.execute(
            "INSERT INTO issues (id, title, status, issue_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "bf-test-empty",
                "Test Empty",
                "in_progress",
                "task",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();

        // Create a worker session with empty claimed_at string
        conn.execute(
            "INSERT INTO worker_sessions (worker_id, model, harness, bead_id, claimed_at, workspace_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params!["worker1", "claude-4.7", "cli", "bf-test-empty", "", "."],
        )
        .unwrap();

        // Close the bead
        let closed_at = Utc::now();
        let updated = update_session_on_close(&conn, "bf-test-empty", closed_at).unwrap();

        // Should return false since empty claimed_at is filtered out
        assert!(!updated);

        // Verify the session was NOT updated
        let (session_closed_at, duration): (Option<String>, Option<i64>) = conn
            .query_row(
                "SELECT closed_at, duration_seconds FROM worker_sessions WHERE bead_id = ?1",
                params!["bf-test-empty"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert!(session_closed_at.is_none());
        assert!(duration.is_none());
    }

    #[test]
    fn test_update_session_on_close_sqlite_default_format() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create a test issue
        conn.execute(
            "INSERT INTO issues (id, title, status, issue_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "bf-test-sqlite-default",
                "Test SQLite Default",
                "in_progress",
                "task",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();

        // Create a worker session using SQLite's CURRENT_TIMESTAMP default
        // This produces SQLite native format: "YYYY-MM-DD HH:MM:SS" without timezone
        conn.execute(
            "INSERT INTO worker_sessions (worker_id, model, harness, bead_id, workspace_path)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                "worker1",
                "claude-4.7",
                "cli",
                "bf-test-sqlite-default",
                "."
            ],
        )
        .unwrap();

        // Verify the claimed_at was set to CURRENT_TIMESTAMP (not NULL)
        let claimed_at_from_db: String = conn
            .query_row(
                "SELECT claimed_at FROM worker_sessions WHERE bead_id = ?1",
                params!["bf-test-sqlite-default"],
                |row| row.get(0),
            )
            .unwrap();

        // SQLite's CURRENT_TIMESTAMP format is "YYYY-MM-DD HH:MM:SS" (space separator)
        assert!(!claimed_at_from_db.is_empty());
        assert!(claimed_at_from_db.contains(' ') || claimed_at_from_db.contains('T'));

        // Close the bead - should successfully parse SQLite native format
        let closed_at = Utc::now();
        let updated = update_session_on_close(&conn, "bf-test-sqlite-default", closed_at).unwrap();

        // Should successfully update since SQLite default format is parseable
        assert!(updated);

        // Verify the session was updated
        let (session_closed_at, duration): (Option<String>, Option<i64>) = conn
            .query_row(
                "SELECT closed_at, duration_seconds FROM worker_sessions WHERE bead_id = ?1",
                params!["bf-test-sqlite-default"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert!(session_closed_at.is_some());
        assert!(duration.is_some());
        // Duration should be reasonable (within a few seconds of now to now minus insertion time)
        let duration_val = duration.unwrap();
        assert!(duration_val >= 0, "Duration should be non-negative");
        // Allow up to 60 seconds (test execution time)
        assert!(
            duration_val <= 60,
            "Duration should be reasonable for test execution"
        );
    }

    #[test]
    fn test_update_session_on_close_triggers_velocity_recompute() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create a test issue
        conn.execute(
            "INSERT INTO issues (id, title, status, issue_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "bf-test-recompute",
                "Test Recompute",
                "in_progress",
                "task",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();

        // Create a worker session with model, harness, and issue_type
        let claimed_at = Utc::now() - chrono::Duration::minutes(5);
        conn.execute(
            "INSERT INTO worker_sessions (worker_id, model, harness, bead_id, claimed_at, workspace_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "worker1",
                "claude-4.7",
                "cli",
                "bf-test-recompute",
                claimed_at.to_rfc3339(),
                ".",
            ],
        )
        .unwrap();

        // Verify no velocity stats exist yet
        let count_before: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM velocity_stats WHERE model = ?1 AND harness = ?2 AND issue_type = ?3",
                params!["claude-4.7", "cli", "task"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_before, 0);

        // Close the bead
        let closed_at = Utc::now();
        let updated = update_session_on_close(&conn, "bf-test-recompute", closed_at).unwrap();

        // Should successfully update
        assert!(updated);

        // Verify velocity stats were recomputed and created
        let (sample_count, p50, p90, avg): (i64, Option<i64>, Option<i64>, Option<f64>) = conn
            .query_row(
                "SELECT sample_count, p50_seconds, p90_seconds, avg_seconds
                 FROM velocity_stats
                 WHERE model = ?1 AND harness = ?2 AND issue_type = ?3",
                params!["claude-4.7", "cli", "task"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();

        assert_eq!(sample_count, 1);
        assert!(p50.is_some());
        assert!(p90.is_some());
        assert!(avg.is_some());

        // Verify the stats are reasonable (around 5 minutes = 300 seconds)
        let p50_val = p50.unwrap();
        assert!(
            p50_val >= 290 && p50_val <= 310,
            "p50 should be around 300 seconds, got {}",
            p50_val
        );

        let avg_val = avg.unwrap();
        assert!(
            avg_val >= 290.0 && avg_val <= 310.0,
            "avg should be around 300 seconds, got {}",
            avg_val
        );
    }

    #[test]
    fn test_update_session_on_close_multiple_sessions_same_bead() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create a test issue
        conn.execute(
            "INSERT INTO issues (id, title, status, issue_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "bf-test-multi",
                "Test Multiple",
                "in_progress",
                "bug",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();

        // Create multiple worker sessions for the same bead (e.g., after retry)
        let claimed_at_1 = Utc::now() - chrono::Duration::minutes(20);
        let claimed_at_2 = Utc::now() - chrono::Duration::minutes(10);

        conn.execute(
            "INSERT INTO worker_sessions (worker_id, model, harness, bead_id, claimed_at, workspace_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "worker1",
                "claude-4.7",
                "cli",
                "bf-test-multi",
                claimed_at_1.to_rfc3339(),
                ".",
            ],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO worker_sessions (worker_id, model, harness, bead_id, claimed_at, workspace_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "worker1",
                "claude-4.7",
                "cli",
                "bf-test-multi",
                claimed_at_2.to_rfc3339(),
                ".",
            ],
        )
        .unwrap();

        // Close the bead
        let closed_at = Utc::now();
        let updated = update_session_on_close(&conn, "bf-test-multi", closed_at).unwrap();

        // Should successfully update
        assert!(updated);

        // Verify only the most recent session was updated (not both)
        let count_updated: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM worker_sessions WHERE bead_id = ?1 AND closed_at IS NOT NULL",
                params!["bf-test-multi"],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count_updated, 1, "Only one session should be updated");

        // Verify the most recent session was updated (claimed_at_2)
        let (session_closed_at, duration): (Option<String>, Option<i64>) = conn
            .query_row(
                "SELECT closed_at, duration_seconds FROM worker_sessions
                 WHERE bead_id = ?1 AND claimed_at = ?2",
                params!["bf-test-multi", claimed_at_2.to_rfc3339()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert!(session_closed_at.is_some());
        assert!(duration.is_some());
        // 10 minutes = 600 seconds (allow some tolerance)
        assert!(duration.unwrap() >= 590 && duration.unwrap() <= 610);
    }

    #[test]
    fn test_update_session_on_close_no_matching_session() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create a test issue
        conn.execute(
            "INSERT INTO issues (id, title, status, issue_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "bf-test-nosession",
                "Test No Session",
                "in_progress",
                "task",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();

        // Close the bead without creating a worker session
        let closed_at = Utc::now();
        let updated = update_session_on_close(&conn, "bf-test-nosession", closed_at).unwrap();

        // Should return false since no session exists
        assert!(!updated);
    }

    #[test]
    fn test_recompute_velocity_stats_empty_sessions() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create test issue but no completed sessions
        conn.execute(
            "INSERT INTO issues (id, title, status, issue_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "bf-test-empty",
                "Test Empty",
                "in_progress",
                "task",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();

        // Recompute stats with no completed sessions
        recompute_velocity_stats(&conn, "claude-4.7", "cli", "task").unwrap();

        // Verify stats were created with empty values
        let (count, p50, p90, avg): (i64, Option<i64>, Option<i64>, Option<f64>) = conn
            .query_row(
                "SELECT sample_count, p50_seconds, p90_seconds, avg_seconds FROM velocity_stats
                 WHERE model = ?1 AND harness = ?2 AND issue_type = ?3",
                params!["claude-4.7", "cli", "task"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();

        assert_eq!(count, 0);
        assert!(p50.is_none(), "p50 should be None for empty session list");
        assert!(p90.is_none(), "p90 should be None for empty session list");
        assert!(avg.is_none(), "avg should be None for empty session list");
    }

    #[test]
    fn test_recompute_velocity_stats_single_session() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create a single test issue and session
        let bead_id = "bf-test-single";
        let created_at = Utc::now();
        let closed_at = created_at + chrono::Duration::seconds(300);
        conn.execute(
            "INSERT INTO issues (id, title, status, issue_type, created_at, updated_at, closed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                bead_id,
                "Test Single",
                "closed",
                "bug",
                created_at.to_rfc3339(),
                created_at.to_rfc3339(),
                closed_at.to_rfc3339(),
            ],
        )
        .unwrap();

        let claimed_at = created_at;
        conn.execute(
            "INSERT INTO worker_sessions (worker_id, model, harness, bead_id, claimed_at, closed_at, duration_seconds, workspace_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "worker1",
                "claude-4.7",
                "cli",
                bead_id,
                claimed_at.to_rfc3339(),
                closed_at.to_rfc3339(),
                300,
                ".",
            ],
        )
        .unwrap();

        // Recompute stats with single session
        recompute_velocity_stats(&conn, "claude-4.7", "cli", "bug").unwrap();

        // Verify stats with single session
        let (count, p50, p90, avg): (i64, Option<i64>, Option<i64>, Option<f64>) = conn
            .query_row(
                "SELECT sample_count, p50_seconds, p90_seconds, avg_seconds FROM velocity_stats
                 WHERE model = ?1 AND harness = ?2 AND issue_type = ?3",
                params!["claude-4.7", "cli", "bug"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();

        assert_eq!(count, 1);
        assert!(p50.is_some(), "p50 should be Some for single session");
        assert_eq!(p50.unwrap(), 300, "p50 should equal the single duration");

        // For single session, p90 should equal p50 (index 0 for both)
        assert!(p90.is_some(), "p90 should be Some for single session");
        assert_eq!(p90.unwrap(), 300, "p90 should equal p50 for single session");

        assert!(avg.is_some(), "avg should be Some for single session");
        assert!(
            (avg.unwrap() - 300.0).abs() < 0.01,
            "avg should equal the single duration"
        );
    }

    #[test]
    fn test_get_expected_seconds_fallback_to_model_and_issue_type() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create stats with empty harness (model + issue_type only)
        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-4.7",
                "",  // Empty harness for model+issue_type fallback
                "bug",
                5,   // sample_count >= 3 required
                180, // p50
                300, // p90
                200.0, // avg
                Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();

        // Query with unknown harness should fallback to model+issue_type
        let result = get_expected_seconds(&conn, "claude-4.7", "unknown-harness", "bug").unwrap();

        assert!(result.is_some(), "Should find fallback to model+issue_type");
        assert_eq!(
            result.unwrap(),
            180,
            "Should return p50 from model+issue_type stats"
        );
    }

    #[test]
    fn test_get_expected_seconds_fallback_to_issue_type_only() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create stats with empty model and harness (issue_type only)
        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "",  // Empty model for issue_type-only fallback
                "",  // Empty harness for issue_type-only fallback
                "bug",
                15,  // sample_count >= 10 required for issue_type-only fallback
                150, // p50
                250, // p90
                175.0, // avg
                Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();

        // Query with unknown model and harness should fallback to issue_type only
        let result =
            get_expected_seconds(&conn, "unknown-model", "unknown-harness", "bug").unwrap();

        assert!(result.is_some(), "Should find fallback to issue_type only");
        assert_eq!(
            result.unwrap(),
            150,
            "Should return p50 from issue_type-only stats"
        );
    }

    #[test]
    fn test_get_expected_seconds_insufficient_sample_count_fallback() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create stats with insufficient sample_count for exact match
        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-4.7",
                "cli",
                "bug",
                2,   // sample_count < 3, should not match exact query
                100, // p50
                150, // p90
                110.0, // avg
                Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();

        // Create stats with sufficient sample_count for model+issue_type fallback
        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-4.7",
                "",  // Empty harness
                "bug",
                5,   // sample_count >= 3
                180, // p50
                300, // p90
                200.0, // avg
                Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();

        // Should skip exact match (insufficient sample_count) and use model+issue_type fallback
        let result = get_expected_seconds(&conn, "claude-4.7", "cli", "bug").unwrap();

        assert!(result.is_some(), "Should find fallback to model+issue_type");
        assert_eq!(result.unwrap(), 180, "Should return p50 from model+issue_type stats (not exact match with insufficient samples)");
    }

    #[test]
    fn test_get_expected_seconds_no_data_available() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // No stats in database

        // Query should return None when no data available
        let result =
            get_expected_seconds(&conn, "unknown-model", "unknown-harness", "unknown-type")
                .unwrap();

        assert!(
            result.is_none(),
            "Should return None when no velocity data available"
        );
    }

    #[test]
    fn test_get_velocity_stats_no_filter() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Insert multiple velocity stats with different sample counts
        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-4.7", "cli", "bug", 10, 180, 300, 200.0, Utc::now().to_rfc3339()
            ],
        ).unwrap();

        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-4.7", "gui", "task", 5, 120, 200, 150.0, Utc::now().to_rfc3339()
            ],
        ).unwrap();

        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-5.0", "cli", "bug", 15, 150, 250, 175.0, Utc::now().to_rfc3339()
            ],
        ).unwrap();

        // Query with no filter should return all stats
        let stats = get_velocity_stats(&conn, None, None).unwrap();

        assert_eq!(stats.len(), 3, "Should return all 3 stats");

        // Verify ordering by sample_count DESC
        assert_eq!(
            stats[0].sample_count, 15,
            "First should have highest sample_count"
        );
        assert_eq!(
            stats[1].sample_count, 10,
            "Second should have middle sample_count"
        );
        assert_eq!(
            stats[2].sample_count, 5,
            "Third should have lowest sample_count"
        );
    }

    #[test]
    fn test_get_velocity_stats_model_filter_only() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Insert stats for different models
        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-4.7", "cli", "bug", 10, 180, 300, 200.0, Utc::now().to_rfc3339()
            ],
        ).unwrap();

        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-4.7", "gui", "task", 5, 120, 200, 150.0, Utc::now().to_rfc3339()
            ],
        ).unwrap();

        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-5.0", "cli", "bug", 15, 150, 250, 175.0, Utc::now().to_rfc3339()
            ],
        ).unwrap();

        // Query with model filter should return only matching stats
        let stats = get_velocity_stats(&conn, Some("claude-4.7"), None).unwrap();

        assert_eq!(stats.len(), 2, "Should return only claude-4.7 stats");
        assert!(
            stats.iter().all(|s| s.model == "claude-4.7"),
            "All results should have model=claude-4.7"
        );

        // Verify ordering by sample_count DESC within filtered results
        assert_eq!(
            stats[0].sample_count, 10,
            "First should have higher sample_count"
        );
        assert_eq!(
            stats[1].sample_count, 5,
            "Second should have lower sample_count"
        );
    }

    #[test]
    fn test_get_velocity_stats_harness_filter_only() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Insert stats for different harnesses
        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-4.7", "cli", "bug", 10, 180, 300, 200.0, Utc::now().to_rfc3339()
            ],
        ).unwrap();

        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-5.0", "cli", "task", 15, 150, 250, 175.0, Utc::now().to_rfc3339()
            ],
        ).unwrap();

        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-4.7", "gui", "bug", 5, 120, 200, 150.0, Utc::now().to_rfc3339()
            ],
        ).unwrap();

        // Query with harness filter should return only matching stats
        let stats = get_velocity_stats(&conn, None, Some("cli")).unwrap();

        assert_eq!(stats.len(), 2, "Should return only cli harness stats");
        assert!(
            stats.iter().all(|s| s.harness == "cli"),
            "All results should have harness=cli"
        );

        // Verify ordering by sample_count DESC within filtered results
        assert_eq!(
            stats[0].sample_count, 15,
            "First should have higher sample_count"
        );
        assert_eq!(
            stats[1].sample_count, 10,
            "Second should have lower sample_count"
        );
    }

    #[test]
    fn test_get_velocity_stats_combined_model_and_harness_filter() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Insert stats for different (model, harness) combinations
        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-4.7", "cli", "bug", 10, 180, 300, 200.0, Utc::now().to_rfc3339()
            ],
        ).unwrap();

        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-4.7", "gui", "bug", 5, 120, 200, 150.0, Utc::now().to_rfc3339()
            ],
        ).unwrap();

        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-5.0", "cli", "task", 15, 150, 250, 175.0, Utc::now().to_rfc3339()
            ],
        ).unwrap();

        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-5.0", "gui", "task", 8, 140, 220, 160.0, Utc::now().to_rfc3339()
            ],
        ).unwrap();

        // Query with both model and harness filters should return only exact matches
        let stats = get_velocity_stats(&conn, Some("claude-4.7"), Some("cli")).unwrap();

        assert_eq!(stats.len(), 1, "Should return only claude-4.7 + cli stats");
        assert_eq!(stats[0].model, "claude-4.7", "Should match model");
        assert_eq!(stats[0].harness, "cli", "Should match harness");
        assert_eq!(
            stats[0].sample_count, 10,
            "Should have correct sample_count"
        );
    }

    #[test]
    fn test_get_velocity_stats_no_matching_results() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Insert some stats
        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-4.7", "cli", "bug", 10, 180, 300, 200.0, Utc::now().to_rfc3339()
            ],
        ).unwrap();

        // Query with non-matching filters should return empty results
        let stats = get_velocity_stats(&conn, Some("non-existent-model"), Some("cli")).unwrap();
        assert_eq!(
            stats.len(),
            0,
            "Should return empty vec for non-matching model"
        );

        let stats =
            get_velocity_stats(&conn, Some("claude-4.7"), Some("non-existent-harness")).unwrap();
        assert_eq!(
            stats.len(),
            0,
            "Should return empty vec for non-matching harness"
        );

        let stats = get_velocity_stats(&conn, None, Some("non-existent-harness")).unwrap();
        assert_eq!(
            stats.len(),
            0,
            "Should return empty vec for non-matching harness (no model filter)"
        );
    }

    #[test]
    fn test_get_velocity_stats_ordering_by_sample_count_desc() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Insert multiple stats with varying sample counts
        let sample_counts = vec![3, 15, 7, 20, 1, 10];

        for (i, sample_count) in sample_counts.iter().enumerate() {
            conn.execute(
                "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    format!("model-{}", i),
                    format!("harness-{}", i),
                    "bug",
                    sample_count,
                    100 + i as i64,
                    200 + i as i64,
                    150.0 + i as f64,
                    Utc::now().to_rfc3339()
                ],
            ).unwrap();
        }

        // Query without filter should return results ordered by sample_count DESC
        let stats = get_velocity_stats(&conn, None, None).unwrap();

        assert_eq!(stats.len(), 6, "Should return all 6 stats");

        // Verify descending order
        let mut last_count = i64::MAX;
        for stat in &stats {
            assert!(
                stat.sample_count <= last_count,
                "Should be sorted by sample_count DESC"
            );
            last_count = stat.sample_count;
        }

        // Verify exact ordering
        assert_eq!(stats[0].sample_count, 20, "First should be 20");
        assert_eq!(stats[1].sample_count, 15, "Second should be 15");
        assert_eq!(stats[2].sample_count, 10, "Third should be 10");
        assert_eq!(stats[3].sample_count, 7, "Fourth should be 7");
        assert_eq!(stats[4].sample_count, 3, "Fifth should be 3");
        assert_eq!(stats[5].sample_count, 1, "Sixth should be 1");
    }

    #[test]
    fn test_get_velocity_stats_filtered_results_maintain_ordering() {
        let temp_file = setup_test_db();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Insert stats with same model but different sample counts
        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-4.7", "cli", "bug", 10, 180, 300, 200.0, Utc::now().to_rfc3339()
            ],
        ).unwrap();

        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-4.7", "gui", "bug", 25, 200, 350, 220.0, Utc::now().to_rfc3339()
            ],
        ).unwrap();

        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-4.7", "tui", "task", 5, 100, 150, 120.0, Utc::now().to_rfc3339()
            ],
        ).unwrap();

        conn.execute(
            "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "claude-5.0", "cli", "bug", 30, 150, 250, 175.0, Utc::now().to_rfc3339()
            ],
        ).unwrap();

        // Query with model filter should maintain ordering by sample_count DESC
        let stats = get_velocity_stats(&conn, Some("claude-4.7"), None).unwrap();

        assert_eq!(stats.len(), 3, "Should return 3 claude-4.7 stats");

        // Verify ordering within filtered results
        assert_eq!(
            stats[0].sample_count, 25,
            "First should have highest sample_count (gui)"
        );
        assert_eq!(
            stats[1].sample_count, 10,
            "Second should have middle sample_count (cli)"
        );
        assert_eq!(
            stats[2].sample_count, 5,
            "Third should have lowest sample_count (tui)"
        );

        // Verify harness matches ordering
        assert_eq!(stats[0].harness, "gui");
        assert_eq!(stats[1].harness, "cli");
        assert_eq!(stats[2].harness, "tui");
    }
}
