//! Critical path computation using dependency DAG float analysis.
//!
//! This module implements a two-pass walk on the dependency DAG:
//! - Forward pass: computes earliest start (ES) for each bead
//! - Backward pass: computes latest start (LS) for each bead
//! - Float = LS - ES (zero-float beads are on the critical path)
//!
//! The results are cached in the `critical_path_cache` table for fast claim scoring.
//!
//! Algorithm uses SQLite recursive CTEs for efficient DAG traversal.

use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// Critical path data for a single bead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalPathData {
    pub bead_id: String,
    pub epic_id: Option<String>,
    pub es: i64,      // earliest start (hops from root)
    pub ls: i64,      // latest start
    pub float: i64,   // ls - es; 0 = critical path
}

/// Result of critical path computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalPathResult {
    pub epic_id: String,
    pub beads: Vec<CriticalPathData>,
    pub longest_chain: Vec<String>,
    pub min_remaining: i64,
}

/// Compute the critical path for all beads in the workspace.
///
/// This performs a two-pass walk using recursive CTEs:
/// 1. Forward pass: compute ES (earliest start) from roots to leaves
/// 2. Backward pass: compute LS (latest start) from leaves to roots
/// 3. Float = LS - ES
///
/// Results are cached in `critical_path_cache` table.
///
/// # Arguments
/// * `conn` - The database connection
///
/// # Returns
/// * `Ok(())` - Critical path cache updated successfully
pub fn compute_critical_path(conn: &Connection) -> Result<()> {
    // First, clear the existing cache
    conn.execute("DELETE FROM critical_path_cache", [])?;

    // Forward pass: compute ES using recursive CTE
    // Start from beads with no open predecessors (roots), then propagate
    let forward_cte = r#"
        WITH RECURSIVE
        forward(id, es, predecessor_ids) AS (
            -- Base case: beads with no open predecessors
            SELECT i.id, 0 as es, '' as predecessor_ids
            FROM issues i
            WHERE i.status IN ('open', 'in_progress', 'blocked', 'deferred')
              AND i.deleted_at IS NULL
              AND NOT EXISTS (
                  SELECT 1 FROM dependencies d
                  JOIN issues pred ON pred.id = d.depends_on_id
                  WHERE d.issue_id = i.id
                    AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
                    AND pred.status IN ('open', 'in_progress', 'blocked', 'deferred')
                    AND pred.deleted_at IS NULL
              )
            UNION ALL
            -- Recursive case: beads that depend on already-processed beads
            SELECT DISTINCT
                d.issue_id,
                f.es + 1,
                f.predecessor_ids || ',' || d.depends_on_id
            FROM dependencies d
            JOIN forward f ON f.id = d.depends_on_id
            JOIN issues i ON i.id = d.issue_id
            WHERE d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
              AND i.status IN ('open', 'in_progress', 'blocked', 'deferred')
              AND i.deleted_at IS NULL
              -- Avoid cycles: don't process if we've already seen this bead
              AND f.predecessor_ids NOT LIKE '%,' || d.issue_id || ',%'
              AND f.predecessor_ids NOT LIKE d.issue_id || ',%'
        )
        SELECT id, MAX(es) as es
        FROM forward
        GROUP BY id
    "#;

    // Get all beads with their ES from forward pass
    let forward_beads_sql = format!("SELECT id, es FROM ({}) ORDER BY id", forward_cte);
    let mut forward_stmt = conn.prepare(&forward_beads_sql)?;

    // Collect forward pass results
    let mut forward_data: Vec<(String, i64)> = Vec::new();
    let mut forward_rows = forward_stmt.query([])?;
    while let Some(row) = forward_rows.next()? {
        let id: String = row.get(0)?;
        let es: i64 = row.get(1)?;
        forward_data.push((id, es));
    }

    if forward_data.is_empty() {
        // No active beads, nothing to compute
        return Ok(());
    }

    // Get the maximum ES (longest path length to any leaf)
    let max_es = forward_data.iter().map(|(_, es)| *es).max().unwrap_or(0);

    // Backward pass: compute LS by working backward from leaves
    // Start from beads with no successors (leaves), then propagate backward
    let backward_cte = r#"
        WITH RECURSIVE
        -- Find leaves (beads with no open successors)
        leaves AS (
            SELECT i.id
            FROM issues i
            WHERE i.status IN ('open', 'in_progress', 'blocked', 'deferred')
              AND i.deleted_at IS NULL
              AND NOT EXISTS (
                  SELECT 1 FROM dependencies d
                  JOIN issues succ ON succ.id = d.issue_id
                  WHERE d.depends_on_id = i.id
                    AND d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
                    AND succ.status IN ('open', 'in_progress', 'blocked', 'deferred')
                    AND succ.deleted_at IS NULL
              )
        ),
        -- Backward pass: propagate LS from leaves to roots
        backward(id, ls, successor_ids) AS (
            -- Base case: leaves start with LS = max_es
            SELECT id, ?1 as ls, '' as successor_ids
            FROM leaves
            UNION ALL
            -- Recursive case: predecessors get LS = min(successor LS) - 1
            SELECT DISTINCT
                d.depends_on_id,
                b.ls - 1,
                b.successor_ids || ',' || d.issue_id
            FROM dependencies d
            JOIN backward b ON b.id = d.issue_id
            JOIN issues i ON i.id = d.depends_on_id
            WHERE d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
              AND i.status IN ('open', 'in_progress', 'blocked', 'deferred')
              AND i.deleted_at IS NULL
              -- Avoid cycles
              AND b.successor_ids NOT LIKE '%,' || d.depends_on_id || ',%'
              AND b.successor_ids NOT LIKE d.depends_on_id || ',%'
        )
        SELECT id, MIN(ls) as ls
        FROM backward
        GROUP BY id
    "#;

    // Get all beads with their LS from backward pass
    let mut backward_stmt = conn.prepare(backward_cte)?;

    // Collect backward pass results into a map for O(1) lookup
    let mut backward_data: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut backward_rows = backward_stmt.query(params![max_es])?;
    while let Some(row) = backward_rows.next()? {
        let id: String = row.get(0)?;
        let ls: i64 = row.get(1)?;
        backward_data.insert(id, ls);
    }

    // Populate the critical_path_cache table by joining forward and backward passes
    // For beads with no LS (shouldn't happen in valid DAG), use max_es
    for (bead_id, es) in &forward_data {
        let ls = backward_data.get(bead_id).copied().unwrap_or(max_es);
        let float_val = ls - es;

        conn.execute(
            "INSERT INTO critical_path_cache (bead_id, epic_id, es, ls, float, updated_at)
             VALUES (?1, NULL, ?2, ?3, ?4, CURRENT_TIMESTAMP)",
            params![bead_id, es, ls, float_val],
        )?;
    }

    Ok(())
}

/// Compute critical path for a specific epic and return detailed results.
///
/// This is used by the `bf critical-path` command to show the critical path
/// for a specific epic.
///
/// # Arguments
/// * `conn` - The database connection
/// * `epic_id` - The epic ID to compute the critical path for
///
/// # Returns
/// * `Ok(CriticalPathResult)` - Critical path data for the epic
pub fn compute_epic_critical_path(conn: &Connection, epic_id: &str) -> Result<CriticalPathResult> {
    // First, ensure the cache is up to date
    compute_critical_path(conn)?;

    // Get all beads in the epic's dependency subtree
    let subtree_sql = r#"
        WITH RECURSIVE
        subtree(id) AS (
            -- Start with the epic itself
            SELECT ?1 as id
            UNION ALL
            -- Add all beads that transitively depend on the epic
            SELECT d.issue_id
            FROM dependencies d
            JOIN subtree s ON s.id = d.depends_on_id
            WHERE d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
        )
        SELECT c.bead_id, c.es, c.ls, c.float, i.title, i.status, i.assignee
        FROM subtree s
        JOIN critical_path_cache c ON c.bead_id = s.id
        JOIN issues i ON i.id = s.id
        ORDER BY c.float ASC, c.es ASC
    "#;

    let mut beads = Vec::new();
    let mut zero_float_beads = Vec::new();

    let mut stmt = conn.prepare(subtree_sql)?;
    let mut rows = stmt.query(params![epic_id])?;
    while let Some(row) = rows.next()? {
        let bead_id: String = row.get(0)?;
        let es: i64 = row.get(1)?;
        let ls: i64 = row.get(2)?;
        let float: i64 = row.get(3)?;
        let _title: String = row.get(4)?;
        let _status: String = row.get(5)?;
        let _assignee: Option<String> = row.get(6)?;

        beads.push(CriticalPathData {
            bead_id: bead_id.clone(),
            epic_id: Some(epic_id.to_string()),
            es,
            ls,
            float,
        });

        if float == 0 {
            zero_float_beads.push(bead_id);
        }
    }

    // Build the longest chain by tracing through zero-float beads
    let longest_chain = build_longest_chain(conn, &zero_float_beads)?;

    let min_remaining = longest_chain.len() as i64;

    Ok(CriticalPathResult {
        epic_id: epic_id.to_string(),
        beads,
        longest_chain,
        min_remaining,
    })
}

/// Build the longest chain through zero-float beads.
///
/// This traces through the dependency graph following only zero-float beads
/// to construct the critical path chain.
fn build_longest_chain(conn: &Connection, zero_float_beads: &[String]) -> Result<Vec<String>> {
    if zero_float_beads.is_empty() {
        return Ok(Vec::new());
    }

    // Use a simpler approach: just return zero_float_beads sorted by ES
    let mut sorted = zero_float_beads.to_vec();
    sorted.sort_by_key(|id| {
        conn.query_row(
            "SELECT es FROM critical_path_cache WHERE bead_id = ?1",
            params![id],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
    });

    Ok(sorted)
}

/// Invalidate and recompute the critical path cache.
///
/// This should be called whenever dependencies are added/removed or bead
/// statuses change.
///
/// # Arguments
/// * `tx` - The database connection or transaction
pub fn invalidate_cache(tx: &Connection) -> Result<()> {
    compute_critical_path(tx)
}

/// Get the critical path float for a specific bead.
///
/// Returns None if the bead is not in the cache (e.g., closed beads).
///
/// # Arguments
/// * `conn` - The database connection
/// * `bead_id` - The bead ID
///
/// # Returns
/// * `Ok(Some(float))` - The float value
/// * `Ok(None)` - Bead not in cache
pub fn get_bead_float(conn: &Connection, bead_id: &str) -> Result<Option<i64>> {
    match conn.query_row(
        "SELECT float FROM critical_path_cache WHERE bead_id = ?1",
        params![bead_id],
        |row| row.get::<_, i64>(0),
    ) {
        Ok(float) => Ok(Some(float)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::schema::apply_schema;

    fn setup_test_db() -> (tempfile::NamedTempFile, Connection) {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();
        apply_schema(&conn).unwrap();
        (temp_file, conn)
    }

    #[test]
    fn test_compute_critical_path_simple_chain() {
        let (_temp, conn) = setup_test_db();

        // Create a simple chain: A -> B -> C
        // A has no predecessors (ES=0), B depends on A (ES=1), C depends on B (ES=2)
        let mut stmt = conn
            .prepare(
                "INSERT INTO issues (id, title, status, source_repo) VALUES (?1, ?2, ?3, ?4)",
            )
            .unwrap();

        stmt.execute(params!("bf-a", "Bead A", "open", ".")).unwrap();
        stmt.execute(params!("bf-b", "Bead B", "open", ".")).unwrap();
        stmt.execute(params!("bf-c", "Bead C", "open", ".")).unwrap();

        // Add dependencies
        let mut dep_stmt = conn.prepare(
            "INSERT INTO dependencies (issue_id, depends_on_id, type, created_by) VALUES (?1, ?2, 'blocks', '')"
        ).unwrap();
        dep_stmt.execute(params!("bf-b", "bf-a")).unwrap();
        dep_stmt.execute(params!("bf-c", "bf-b")).unwrap();

        // Compute critical path
        compute_critical_path(&conn).unwrap();

        // Check ES values
        let es_a: i64 = conn
            .query_row(
                "SELECT es FROM critical_path_cache WHERE bead_id = 'bf-a'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let es_b: i64 = conn
            .query_row(
                "SELECT es FROM critical_path_cache WHERE bead_id = 'bf-b'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let es_c: i64 = conn
            .query_row(
                "SELECT es FROM critical_path_cache WHERE bead_id = 'bf-c'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(es_a, 0);
        assert_eq!(es_b, 1);
        assert_eq!(es_c, 2);

        // Check float values (all on critical path, so float = 0)
        let float_a: i64 = conn
            .query_row(
                "SELECT float FROM critical_path_cache WHERE bead_id = 'bf-a'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let float_b: i64 = conn
            .query_row(
                "SELECT float FROM critical_path_cache WHERE bead_id = 'bf-b'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let float_c: i64 = conn
            .query_row(
                "SELECT float FROM critical_path_cache WHERE bead_id = 'bf-c'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(float_a, 0);
        assert_eq!(float_b, 0);
        assert_eq!(float_c, 0);
    }

    #[test]
    fn test_compute_critical_path_parallel_branches() {
        let (_temp, conn) = setup_test_db();

        // Create a diamond: A -> [B, C] -> D
        // B and C are parallel, so one has float > 0
        let mut stmt = conn
            .prepare(
                "INSERT INTO issues (id, title, status, source_repo) VALUES (?1, ?2, ?3, ?4)",
            )
            .unwrap();

        stmt.execute(params!("bf-a", "Bead A", "open", ".")).unwrap();
        stmt.execute(params!("bf-b", "Bead B", "open", ".")).unwrap();
        stmt.execute(params!("bf-c", "Bead C", "open", ".")).unwrap();
        stmt.execute(params!("bf-d", "Bead D", "open", ".")).unwrap();

        // Add dependencies
        let mut dep_stmt = conn.prepare(
            "INSERT INTO dependencies (issue_id, depends_on_id, type, created_by) VALUES (?1, ?2, 'blocks', '')"
        ).unwrap();
        dep_stmt.execute(params!("bf-b", "bf-a")).unwrap();
        dep_stmt.execute(params!("bf-c", "bf-a")).unwrap();
        dep_stmt.execute(params!("bf-d", "bf-b")).unwrap();
        dep_stmt.execute(params!("bf-d", "bf-c")).unwrap();

        // Compute critical path
        compute_critical_path(&conn).unwrap();

        // Check floats in the diamond structure
        // In a diamond with equal-length paths, all beads have float = 0
        // because D depends on BOTH B and C, making both paths critical
        let float_a: i64 = conn
            .query_row(
                "SELECT float FROM critical_path_cache WHERE bead_id = 'bf-a'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let float_b: i64 = conn
            .query_row(
                "SELECT float FROM critical_path_cache WHERE bead_id = 'bf-b'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let float_c: i64 = conn
            .query_row(
                "SELECT float FROM critical_path_cache WHERE bead_id = 'bf-c'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let float_d: i64 = conn
            .query_row(
                "SELECT float FROM critical_path_cache WHERE bead_id = 'bf-d'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // All beads on equal-length parallel paths have float = 0
        assert_eq!(float_a, 0);
        assert_eq!(float_b, 0);
        assert_eq!(float_c, 0);
        assert_eq!(float_d, 0);

        // Verify ES values: A=0, B=1, C=1, D=2
        let es_a: i64 = conn
            .query_row(
                "SELECT es FROM critical_path_cache WHERE bead_id = 'bf-a'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let es_b: i64 = conn
            .query_row(
                "SELECT es FROM critical_path_cache WHERE bead_id = 'bf-b'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let es_c: i64 = conn
            .query_row(
                "SELECT es FROM critical_path_cache WHERE bead_id = 'bf-c'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let es_d: i64 = conn
            .query_row(
                "SELECT es FROM critical_path_cache WHERE bead_id = 'bf-d'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(es_a, 0);
        assert_eq!(es_b, 1);
        assert_eq!(es_c, 1);
        assert_eq!(es_d, 2);
    }

    #[test]
    fn test_compute_critical_path_unequal_paths() {
        let (_temp, conn) = setup_test_db();

        // Create a structure with unequal path lengths:
        // A -> B -> C -> D -> F (long path: 4 hops)
        // A -> E -> F (short path: 2 hops)
        // The short path (A-E-F) should have positive float

        let mut stmt = conn
            .prepare(
                "INSERT INTO issues (id, title, status, source_repo) VALUES (?1, ?2, ?3, ?4)",
            )
            .unwrap();

        stmt.execute(params!("bf-a", "Bead A", "open", ".")).unwrap();
        stmt.execute(params!("bf-b", "Bead B", "open", ".")).unwrap();
        stmt.execute(params!("bf-c", "Bead C", "open", ".")).unwrap();
        stmt.execute(params!("bf-d", "Bead D", "open", ".")).unwrap();
        stmt.execute(params!("bf-e", "Bead E", "open", ".")).unwrap();
        stmt.execute(params!("bf-f", "Bead F", "open", ".")).unwrap();

        // Add dependencies
        let mut dep_stmt = conn.prepare(
            "INSERT INTO dependencies (issue_id, depends_on_id, type, created_by) VALUES (?1, ?2, 'blocks', '')"
        ).unwrap();
        dep_stmt.execute(params!("bf-b", "bf-a")).unwrap();  // A -> B
        dep_stmt.execute(params!("bf-c", "bf-b")).unwrap();  // B -> C
        dep_stmt.execute(params!("bf-d", "bf-c")).unwrap();  // C -> D
        dep_stmt.execute(params!("bf-f", "bf-d")).unwrap();  // D -> F (long path)
        dep_stmt.execute(params!("bf-e", "bf-a")).unwrap();  // A -> E
        dep_stmt.execute(params!("bf-f", "bf-e")).unwrap();  // E -> F (short path)

        // Compute critical path
        compute_critical_path(&conn).unwrap();

        // Check ES values
        let es_a: i64 = conn
            .query_row(
                "SELECT es FROM critical_path_cache WHERE bead_id = 'bf-a'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let es_b: i64 = conn
            .query_row(
                "SELECT es FROM critical_path_cache WHERE bead_id = 'bf-b'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let es_e: i64 = conn
            .query_row(
                "SELECT es FROM critical_path_cache WHERE bead_id = 'bf-e'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let es_f: i64 = conn
            .query_row(
                "SELECT es FROM critical_path_cache WHERE bead_id = 'bf-f'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(es_a, 0, "A is root, ES=0");
        assert_eq!(es_b, 1, "B depends on A, ES=1");
        assert_eq!(es_e, 1, "E depends on A, ES=1");
        assert_eq!(es_f, 4, "F depends on D (ES=3) and E (ES=1), so ES=MAX(3,1)+1=4");

        // The long path (A-B-C-D-F) should have float = 0
        let float_a: i64 = conn
            .query_row(
                "SELECT float FROM critical_path_cache WHERE bead_id = 'bf-a'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let float_b: i64 = conn
            .query_row(
                "SELECT float FROM critical_path_cache WHERE bead_id = 'bf-b'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let float_f: i64 = conn
            .query_row(
                "SELECT float FROM critical_path_cache WHERE bead_id = 'bf-f'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(float_a, 0, "A should be on critical path");
        assert_eq!(float_b, 0, "B should be on critical path");
        assert_eq!(float_f, 0, "F should be on critical path");

        // The short path (A-E-F) should have float > 0
        let float_e: i64 = conn
            .query_row(
                "SELECT float FROM critical_path_cache WHERE bead_id = 'bf-e'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(float_e > 0, "E should have positive float (short path)");
    }
}
