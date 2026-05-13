// Integration test for velocity stats update on bead close
use bead_forge::Storage;
use chrono::Utc;
use std::path::Path;

#[test]
fn test_velocity_update_on_close() {
    let db_path = Path::new("/tmp/test_velocity_close_integration.db");
    let _ = std::fs::remove_file(db_path);

    let storage = Storage::open(db_path).unwrap();

    // Create a test bead
    let bead_id = "bf-test-velocity-close";
    let now = Utc::now();

    storage.with_immediate_transaction(|tx| {
        tx.execute(
            "INSERT INTO issues (id, title, status, issue_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            [bead_id, "Test bead", "in_progress", "task", &now.to_rfc3339(), &now.to_rfc3339()],
        ).unwrap();

        // Create a worker session for this bead
        let claimed_at = now - chrono::Duration::minutes(10);
        tx.execute(
            "INSERT INTO worker_sessions (worker_id, model, harness, bead_id, claimed_at, workspace_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            ["worker1", "claude-4.7", "cli", bead_id, &claimed_at.to_rfc3339(), "."],
        ).unwrap();

        Ok::<_, anyhow::Error>(())
    }).unwrap();

    // Close the bead - this should trigger velocity stats update
    storage
        .close_issue(bead_id, "Test completed", "test")
        .unwrap();

    // Verify the session was updated
    let (closed_at, duration): (Option<String>, Option<i64>) =
        storage
            .with_immediate_transaction(|tx| {
                Ok(tx.query_row(
            "SELECT closed_at, duration_seconds FROM worker_sessions WHERE bead_id = ?1",
            [bead_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap())
            })
            .unwrap();

    assert!(closed_at.is_some(), "closed_at should be set");
    assert!(duration.is_some(), "duration_seconds should be set");

    let duration = duration.unwrap();
    assert!(
        duration >= 590 && duration <= 610,
        "Duration should be ~600 seconds (10 minutes), got {}",
        duration
    );

    // Verify velocity_stats was updated
    let (model, harness, issue_type, sample_count, p50): (
        String,
        String,
        String,
        i64,
        Option<i64>,
    ) = storage
        .with_immediate_transaction(|tx| {
            Ok(tx.query_row(
            "SELECT model, harness, issue_type, sample_count, p50_seconds FROM velocity_stats
             WHERE model = ?1 AND harness = ?2 AND issue_type = ?3",
            ["claude-4.7", "cli", "task"],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).unwrap())
        })
        .unwrap();

    assert_eq!(model, "claude-4.7");
    assert_eq!(harness, "cli");
    assert_eq!(issue_type, "task");
    assert_eq!(sample_count, 1);
    assert!(p50.is_some(), "p50 should be set");

    // Cleanup
    let _ = std::fs::remove_file(db_path);
}
