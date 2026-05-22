use rusqlite::Connection;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open(".beads/beads.db")?;

    // Check worker_sessions for bf-1leo
    let mut stmt = conn.prepare("SELECT bead_id, claimed_at, closed_at, duration_seconds FROM worker_sessions WHERE bead_id = 'bf-1leo'")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<i64>>(3)?,
        ))
    })?;

    println!("Worker sessions for bf-1leo:");
    for row in rows {
        let (bead_id, claimed_at, closed_at, duration) = row?;
        println!("  bead_id={}, claimed_at={:?}, closed_at={:?}, duration={:?}", bead_id, claimed_at, closed_at, duration);
    }

    // Delete malformed entries (empty claimed_at)
    let affected = conn.execute("DELETE FROM worker_sessions WHERE bead_id = 'bf-1leo' AND (claimed_at IS NULL OR claimed_at = '')", [])?;
    println!("\nDeleted {} malformed worker_sessions entries", affected);

    Ok(())
}
