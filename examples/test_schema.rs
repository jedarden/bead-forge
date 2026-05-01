use bead_forge::storage::schema::{apply_schema, ensure_wal_mode};
use rusqlite::Connection;

fn main() -> anyhow::Result<()> {
    let db_path = "/tmp/test-bf-db/beads.db";
    let _ = std::fs::remove_file(db_path);
    
    let conn = Connection::open(db_path)?;
    ensure_wal_mode(&conn)?;
    apply_schema(&conn)?;
    
    println!("Schema applied successfully");
    
    Ok(())
}
