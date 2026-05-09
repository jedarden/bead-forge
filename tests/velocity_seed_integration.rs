// Integration test for velocity stats seeding from reconstructed events
use bead_forge::Storage;
use chrono::{Duration, Utc};
use std::path::Path;

#[test]
fn test_seed_velocity_from_events() {
    let db_path = Path::new("/tmp/test_velocity_seed_integration.db");
    let _ = std::fs::remove_file(db_path);

    let storage = Storage::open(db_path).unwrap();
    let now = Utc::now();

    // Create test beads with different actors and issue types
    let test_cases = vec![
        ("bf-1", "worker-claude-sonnet-4-6-01", "task", 100),
        ("bf-2", "worker-claude-sonnet-4-6-01", "task", 150),
        ("bf-3", "worker-claude-sonnet-4-6-01", "task", 200),
        ("bf-4", "worker-claude-opus-4-7-01", "task", 80),
        ("bf-5", "worker-claude-opus-4-7-01", "bug", 120),
        ("bf-6", "worker-claude-4.7", "task", 180),
        ("bf-7", "unknown-actor", "task", 300),
    ];

    storage.with_immediate_transaction(|tx| {
        for (bead_id, actor, issue_type, duration_secs) in &test_cases {
            let created_at = now - Duration::seconds(*duration_secs as i64);
            let closed_at = now;

            // Insert issue
            tx.execute(
                "INSERT INTO issues (id, title, status, issue_type, created_at, updated_at, closed_at, closed_by_session)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                [
                    bead_id,
                    format!("Test bead {}", bead_id).as_str(),
                    "closed",
                    issue_type,
                    created_at.to_rfc3339().as_str(),
                    closed_at.to_rfc3339().as_str(),
                    closed_at.to_rfc3339().as_str(),
                    actor,
                ],
            )
            .unwrap();

            // Insert closed event with actor
            tx.execute(
                "INSERT INTO events (issue_id, event_type, actor, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                [bead_id, "closed", actor, &closed_at.to_rfc3339()],
            )
            .unwrap();
        }
        Ok::<_, anyhow::Error>(())
    })
    .unwrap();

    // Seed velocity stats from events
    bead_forge::migrate::seed_velocity_from_events(&storage).unwrap();

    // Verify velocity stats were created correctly
    let stats: Vec<(String, String, String, i64, Option<i64>, Option<i64>, Option<f64>)> = storage
        .with_immediate_transaction(|tx| {
            let mut stmt = tx
                .prepare(
                    "SELECT model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds
                     FROM velocity_stats
                     ORDER BY model, harness, issue_type",
                )
                .unwrap();
            let mut rows = stmt.query([]).unwrap();
            let mut results = Vec::new();
            while let Some(row) = rows.next().unwrap() {
                results.push((
                    row.get(0).unwrap(),
                    row.get(1).unwrap(),
                    row.get(2).unwrap(),
                    row.get(3).unwrap(),
                    row.get(4).unwrap(),
                    row.get(5).unwrap(),
                    row.get(6).unwrap(),
                ));
            }
            Ok(results)
        })
        .unwrap();

    // We should have stats for:
    // - (claude-sonnet-4-6, unknown, task): 3 samples (100, 150, 200) → p50=150
    // - (claude-opus-4-7, unknown, task): 1 sample (80) → p50=80
    // - (claude-opus-4-7, unknown, bug): 1 sample (120) → p50=120
    // - (claude-4.7, unknown, task): 1 sample (180) → p50=180 (from "worker-claude-4.7")
    // - (unknown, unknown, task): 1 sample (300) → p50=300
    assert_eq!(stats.len(), 5, "Expected 5 velocity stat entries");

    // Check claude-sonnet-4-6 stats
    let sonnet_task = stats
        .iter()
        .find(|(m, h, it, ..)| m == "claude-sonnet-4-6" && h == "unknown" && it == "task")
        .expect("Should have claude-sonnet-4-6 stats");
    assert_eq!(sonnet_task.3, 3, "Should have 3 samples for sonnet task");
    assert_eq!(sonnet_task.4, Some(150), "p50 should be 150 for sonnet task");

    // Check claude-opus-4-7 task stats
    let opus_task = stats
        .iter()
        .find(|(m, h, it, ..)| m == "claude-opus-4-7" && h == "unknown" && it == "task")
        .expect("Should have claude-opus-4-7 task stats");
    assert_eq!(opus_task.3, 1, "Should have 1 sample for opus task");
    assert_eq!(opus_task.4, Some(80), "p50 should be 80 for opus task");

    // Check claude-opus-4-7 bug stats
    let opus_bug = stats
        .iter()
        .find(|(m, h, it, ..)| m == "claude-opus-4-7" && h == "unknown" && it == "bug")
        .expect("Should have claude-opus-4-7 bug stats");
    assert_eq!(opus_bug.3, 1, "Should have 1 sample for opus bug");
    assert_eq!(opus_bug.4, Some(120), "p50 should be 120 for opus bug");

    // Check claude-4.7 stats (from "worker-claude-4.7")
    let claude_task = stats
        .iter()
        .find(|(m, h, it, ..)| m == "claude-4.7" && h == "unknown" && it == "task")
        .expect("Should have claude-4.7 stats");
    assert_eq!(claude_task.3, 1, "Should have 1 sample for claude-4.7 task");
    assert_eq!(claude_task.4, Some(180), "p50 should be 180 for claude-4.7 task");

    // Check unknown actor fallback
    let unknown_task = stats
        .iter()
        .find(|(m, h, it, ..)| m == "unknown" && h == "unknown" && it == "task")
        .expect("Should have unknown actor stats");
    assert_eq!(unknown_task.3, 1, "Should have 1 sample for unknown task");
    assert_eq!(unknown_task.4, Some(300), "p50 should be 300 for unknown task");

    // Cleanup
    let _ = std::fs::remove_file(db_path);
}
