/// Test for limit=0 behavior in ready command
use bead_forge::claim::get_ready_candidates;
use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;
use tempfile::TempDir;

#[test]
fn test_ready_limit_zero_returns_all() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let storage = Storage::open(&db_path).unwrap();

    // Create 15 test beads
    for i in 1..=15 {
        let bead = Issue {
            id: format!("bf-test-{:03}", i),
            title: format!("Test bead {}", i),
            description: None,
            design: None,
            acceptance_criteria: None,
            notes: None,
            status: Status::Open,
            priority: Priority(i as i32 % 5),
            issue_type: IssueType::Task,
            assignee: None,
            owner: None,
            estimated_minutes: None,
            created_at: Utc::now(),
            created_by: None,
            updated_at: Utc::now(),
            closed_at: None,
            close_reason: None,
            closed_by_session: None,
            due_at: None,
            defer_until: None,
            external_ref: None,
            source_system: None,
            source_repo: None,
            deleted_at: None,
            deleted_by: None,
            delete_reason: None,
            original_type: None,
            compaction_level: None,
            compacted_at: None,
            compacted_at_commit: None,
            original_size: None,
            sender: None,
            ephemeral: false,
            pinned: false,
            is_template: false,
            labels: vec![],
            dependencies: vec![],
            comments: vec![],
            events: vec![],
            content_hash: None,
            annotations: Default::default(),
        };
        storage.create_issue(&bead).unwrap();
    }

    // Test with limit=0 (should return all 15 beads)
    storage
        .with_immediate_transaction(|tx| {
            let candidates = get_ready_candidates(tx, 0, None, None).unwrap();
            assert_eq!(candidates.len(), 15, "limit=0 should return all 15 beads");
            Ok(())
        })
        .unwrap();

    // Test with limit=5 (should return only 5 beads)
    storage
        .with_immediate_transaction(|tx| {
            let candidates = get_ready_candidates(tx, 5, None, None).unwrap();
            assert_eq!(candidates.len(), 5, "limit=5 should return only 5 beads");
            Ok(())
        })
        .unwrap();

    // Test with a very large limit (simulating unlimited)
    let large_limit = usize::MAX;
    storage
        .with_immediate_transaction(|tx| {
            let candidates = get_ready_candidates(tx, large_limit, None, None).unwrap();
            assert_eq!(
                candidates.len(),
                15,
                "large limit should return all 15 beads"
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn test_ready_limit_zero_direct_sql_check() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let storage = Storage::open(&db_path).unwrap();

    // Create 5 test beads
    for i in 1..=5 {
        let bead = Issue {
            id: format!("bf-test-{:03}", i),
            title: format!("Test bead {}", i),
            description: None,
            design: None,
            acceptance_criteria: None,
            notes: None,
            status: Status::Open,
            priority: Priority(2),
            issue_type: IssueType::Task,
            assignee: None,
            owner: None,
            estimated_minutes: None,
            created_at: Utc::now(),
            created_by: None,
            updated_at: Utc::now(),
            closed_at: None,
            close_reason: None,
            closed_by_session: None,
            due_at: None,
            defer_until: None,
            external_ref: None,
            source_system: None,
            source_repo: None,
            deleted_at: None,
            deleted_by: None,
            delete_reason: None,
            original_type: None,
            compaction_level: None,
            compacted_at: None,
            compacted_at_commit: None,
            original_size: None,
            sender: None,
            ephemeral: false,
            pinned: false,
            is_template: false,
            labels: vec![],
            dependencies: vec![],
            comments: vec![],
            events: vec![],
            content_hash: None,
            annotations: Default::default(),
        };
        storage.create_issue(&bead).unwrap();
    }

    // Verify that SQL LIMIT 0 would return 0 rows by testing via sqlite
    storage.with_immediate_transaction(|tx| {
        let count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM issues i
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
               )",
            [],
            |row| row.get(0),
        ).unwrap();

        println!("Total ready candidates: {}", count);
        assert_eq!(count, 5, "Should have 5 ready candidates");

        // Verify LIMIT 0 returns 0 rows in raw SQL
        let limited_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM (
                SELECT i.id FROM issues i
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
                LIMIT 0
            )",
            [],
            |row| row.get(0),
        ).unwrap();

        println!("LIMIT 0 returns: {} rows", limited_count);
        assert_eq!(limited_count, 0, "SQL LIMIT 0 should return 0 rows");

        Ok(())
    }).unwrap();
}
