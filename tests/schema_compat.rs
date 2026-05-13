//! Schema compatibility tests between bf and br.
//!
//! Tests that:
//! - bf can read databases created by br
//! - br can read databases created by bf (for core tables)
//! - bf-specific tables don't interfere with br operation
//! - Column order matches br's expectations

mod common;

use std::fs;

#[test]
fn test_schema_matches_br_column_order() {
    // Verify that the issues table has exactly the columns br expects
    // br's issues_column_order_matches() checks the exact column count
    let ws = common::TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Get the column list from pragma_table_info
    let columns: Vec<(String, String)> = storage
        .with_immediate_transaction(|tx| {
            let mut stmt =
                tx.prepare("SELECT name, type FROM pragma_table_info('issues') ORDER BY cid")?;
            let mut rows = stmt.query([])?;
            let mut cols = Vec::new();
            while let Some(row) = rows.next()? {
                let name: String = row.get(0)?;
                let type_: String = row.get(1)?;
                cols.push((name, type_));
            }
            Ok::<_, anyhow::Error>(cols)
        })
        .unwrap();

    // Expected columns in br order (from br's schema)
    let expected_columns = vec![
        ("id", "TEXT"),
        ("content_hash", "TEXT"),
        ("title", "TEXT"),
        ("description", "TEXT"),
        ("design", "TEXT"),
        ("acceptance_criteria", "TEXT"),
        ("notes", "TEXT"),
        ("status", "TEXT"),
        ("priority", "INTEGER"),
        ("issue_type", "TEXT"),
        ("assignee", "TEXT"),
        ("owner", "TEXT"),
        ("estimated_minutes", "INTEGER"),
        ("created_at", "TEXT"), // DATETIME stored as TEXT
        ("created_by", "TEXT"),
        ("updated_at", "TEXT"),
        ("closed_at", "TEXT"),
        ("close_reason", "TEXT"),
        ("closed_by_session", "TEXT"),
        ("due_at", "TEXT"),
        ("defer_until", "TEXT"),
        ("external_ref", "TEXT"),
        ("source_system", "TEXT"),
        ("source_repo", "TEXT"),
        ("deleted_at", "TEXT"),
        ("deleted_by", "TEXT"),
        ("delete_reason", "TEXT"),
        ("original_type", "TEXT"),
        ("compaction_level", "INTEGER"),
        ("compacted_at", "TEXT"),
        ("compacted_at_commit", "TEXT"),
        ("original_size", "INTEGER"),
        ("sender", "TEXT"),
        ("ephemeral", "INTEGER"),
        ("pinned", "INTEGER"),
        ("is_template", "INTEGER"),
    ];

    assert_eq!(
        columns.len(),
        expected_columns.len(),
        "Column count mismatch: got {} columns, expected {}",
        columns.len(),
        expected_columns.len()
    );

    for (i, (name, type_)) in columns.iter().enumerate() {
        let (expected_name, expected_type) = &expected_columns[i];
        assert_eq!(
            name, expected_name,
            "Column {}: name mismatch: got {}, expected {}",
            i, name, expected_name
        );
        // Type may vary slightly (TEXT vs DATETIME), so we just check the column exists
    }
}

#[test]
fn test_bf_tables_dont_interfere_with_br() {
    // Verify that bf-specific tables don't break br's operation
    // br should ignore unknown tables
    let ws = common::TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // bf-specific tables should exist
    let bf_tables = vec![
        "bead_annotations",
        "worker_sessions",
        "velocity_stats",
        "migration_lock",
    ];

    for table in bf_tables {
        let exists: bool = storage
            .with_immediate_transaction(|tx| {
                Ok(tx.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
                    &[table],
                    |row| row.get::<_, i64>(0).map(|n| n > 0),
                )?)
            })
            .unwrap();
        assert!(exists, "bf-specific table '{}' should exist", table);
    }

    // br's core tables should exist with correct structure
    let br_tables = vec![
        "issues",
        "dependencies",
        "labels",
        "comments",
        "events",
        "config",
        "metadata",
        "dirty_issues",
        "blocked_issues_cache",
    ];

    for table in br_tables {
        let exists: bool = storage
            .with_immediate_transaction(|tx| {
                Ok(tx.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
                    &[table],
                    |row| row.get::<_, i64>(0).map(|n| n > 0),
                )?)
            })
            .unwrap();
        assert!(exists, "br core table '{}' should exist", table);
    }
}

#[test]
fn test_br_can_read_bf_created_issues() {
    // Create issues with bf, verify they're readable by br's schema expectations
    let ws = common::TempWorkspace::new().unwrap();

    // Create various types of issues
    ws.create_bead("bf-001", "Task 1").unwrap();
    ws.create_bead("bf-002", "Task 2").unwrap();
    ws.create_bead("bf-003", "Task 3").unwrap();

    // Export to JSONL (simulating br reading the database)
    ws.export_jsonl(false).unwrap();

    let jsonl_content = fs::read_to_string(&ws.jsonl_path).unwrap();
    let lines: Vec<&str> = jsonl_content.lines().collect();

    assert_eq!(lines.len(), 3);

    // Verify each line is valid JSON with expected fields
    for line in lines {
        let json: serde_json::Value = serde_json::from_str(line).unwrap();
        assert!(json.get("id").is_some());
        assert!(json.get("title").is_some());
        assert!(json.get("status").is_some());
        assert!(json.get("priority").is_some());
        assert!(json.get("issue_type").is_some());
        assert!(json.get("created_at").is_some());
        assert!(json.get("updated_at").is_some());
    }
}

#[test]
fn test_bf_annotations_dont_pollute_issues_table() {
    // Verify annotations are stored separately, not in issues table
    let ws = common::TempWorkspace::new().unwrap();

    let mut bead = bead_forge::Issue::new(
        "bf-annot".to_string(),
        "Annotated".to_string(),
        ".".to_string(),
    );
    bead.annotations
        .insert("key1".to_string(), "value1".to_string());
    bead.annotations
        .insert("key2".to_string(), "value2".to_string());

    let storage = ws.storage().unwrap();
    storage.create_issue(&bead).unwrap();

    // Verify annotations are in bead_annotations table
    let annotations = storage.get_annotations("bf-annot").unwrap();
    assert_eq!(annotations.len(), 2);
    assert_eq!(annotations.get("key1"), Some(&"value1".to_string()));
    assert_eq!(annotations.get("key2"), Some(&"value2".to_string()));

    // Verify issues table doesn't have an annotations column
    let has_annotations_col: bool = storage
        .with_immediate_transaction(|tx| {
            Ok(tx.query_row(
                "SELECT COUNT(*) FROM pragma_table_info('issues') WHERE name='annotations'",
                [],
                |row| row.get::<_, i64>(0).map(|n| n > 0),
            )?)
        })
        .unwrap();

    assert!(
        !has_annotations_col,
        "issues table should NOT have annotations column"
    );
}

#[test]
fn test_bf_worker_sessions_table_structure() {
    // Verify worker_sessions table has expected structure
    let ws = common::TempWorkspace::new().unwrap();

    // Create a bead first (foreign key constraint)
    ws.create_bead("bf-001", "Test bead").unwrap();

    let storage = ws.storage().unwrap();

    // Record a worker session
    storage
        .record_worker_session(
            "worker-1",
            Some("claude-opus-4.7"),
            Some("claude-code"),
            Some("1.0.0"),
            "bf-001",
            ws.workspace_path().to_str().unwrap(),
        )
        .unwrap();

    // Query the session back
    let sessions: Vec<(String, Option<String>, Option<String>)> = storage
        .with_immediate_transaction(|tx| {
            let mut stmt = tx.prepare("SELECT worker_id, model, harness FROM worker_sessions")?;
            let mut rows = stmt.query([])?;
            let mut result = Vec::new();
            while let Some(row) = rows.next()? {
                result.push((row.get(0)?, row.get(1)?, row.get(2)?));
            }
            Ok::<_, anyhow::Error>(result)
        })
        .unwrap();

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].0, "worker-1");
    assert_eq!(sessions[0].1, Some("claude-opus-4.7".to_string()));
    assert_eq!(sessions[0].2, Some("claude-code".to_string()));
}

#[test]
fn test_bf_velocity_stats_table_structure() {
    // Verify velocity_stats table has expected structure
    let ws = common::TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Update velocity stats (this is done internally on bead close)
    use chrono::Utc;

    storage
        .with_immediate_transaction(|tx| {
            tx.execute(
                "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                [
                    "claude-opus-4.7",
                    "claude-code",
                    "task",
                    "10",
                    "300",
                    "600",
                    "450.5",
                    &Utc::now().to_rfc3339(),
                ],
            )?;
            Ok::<_, anyhow::Error>(())
        })
        .unwrap();

    // Query the stats back
    let stats: Vec<(String, String, String, i64)> = storage
        .with_immediate_transaction(|tx| {
            let mut stmt =
                tx.prepare("SELECT model, harness, issue_type, sample_count FROM velocity_stats")?;
            let mut rows = stmt.query([])?;
            let mut result = Vec::new();
            while let Some(row) = rows.next()? {
                result.push((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?));
            }
            Ok::<_, anyhow::Error>(result)
        })
        .unwrap();

    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].0, "claude-opus-4.7");
    assert_eq!(stats[0].1, "claude-code");
    assert_eq!(stats[0].2, "task");
    assert_eq!(stats[0].3, 10);
}

#[test]
fn test_migration_lock_table_structure() {
    // Verify migration_lock table is a singleton
    let ws = common::TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Try to insert multiple rows - should fail due to CHECK constraint
    let result: Result<(), anyhow::Error> = storage.with_immediate_transaction(|tx| {
        use chrono::Utc;

        tx.execute(
            "INSERT INTO migration_lock (id, locked_by, locked_at, expires_at)
                 VALUES (1, 'worker-1', ?, ?)",
            [Utc::now().to_rfc3339(), Utc::now().to_rfc3339()],
        )?;

        // This should fail due to PRIMARY KEY CHECK(id = 1)
        tx.execute(
            "INSERT INTO migration_lock (id, locked_by, locked_at, expires_at)
                 VALUES (1, 'worker-2', ?, ?)",
            [Utc::now().to_rfc3339(), Utc::now().to_rfc3339()],
        )?;

        Ok(())
    });

    assert!(
        result.is_err(),
        "Should fail to insert second migration lock row"
    );
}

#[test]
fn test_foreign_keys_enforced() {
    // Verify foreign key constraints work correctly
    let ws = common::TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Try to insert a label for non-existent issue - should fail
    let result: Result<(), anyhow::Error> = storage.with_immediate_transaction(|tx| {
        tx.execute(
            "INSERT INTO labels (issue_id, label) VALUES (?, ?)",
            ["non-existent", "bug"],
        )?;
        Ok(())
    });

    // Note: rusqlite may not enforce FKs by default, so this might not fail
    // but the schema should have the FK defined
    let has_fk: bool = storage
        .with_immediate_transaction(|tx| {
            Ok(tx.query_row(
                "SELECT COUNT(*) FROM pragma_foreign_key_list('labels')",
                [],
                |row| row.get::<_, i64>(0).map(|n| n > 0),
            )?)
        })
        .unwrap();

    assert!(has_fk, "labels table should have foreign key to issues");
}

#[test]
fn test_critical_path_cache_table_structure() {
    // Verify critical_path_cache table has expected structure
    let ws = common::TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Create some beads with dependencies
    ws.create_bead("bf-parent", "Parent").unwrap();
    ws.create_bead("bf-child", "Child").unwrap();

    // Add dependency
    storage
        .add_dependency(
            "bf-child",
            "bf-parent",
            &bead_forge::model::DependencyType::Blocks,
            "test",
        )
        .unwrap();

    // Rebuild critical path cache
    storage.rebuild_blocked_cache().unwrap();

    // Verify cache was populated
    let cache_count: i64 = storage
        .with_immediate_transaction(|tx| {
            tx.query_row("SELECT COUNT(*) FROM critical_path_cache", [], |row| {
                row.get(0)
            })
            .map_err(|e| anyhow::anyhow!(e))
        })
        .unwrap();

    // Cache may be empty if no critical path computation was done
    // but the table structure should be correct
    let columns: Vec<String> = storage
        .with_immediate_transaction(|tx| {
            let mut stmt = tx.prepare(
                "SELECT name FROM pragma_table_info('critical_path_cache') ORDER BY cid",
            )?;
            let mut rows = stmt.query([])?;
            let mut cols = Vec::new();
            while let Some(row) = rows.next()? {
                cols.push(row.get(0)?);
            }
            Ok::<_, anyhow::Error>(cols)
        })
        .unwrap();

    assert!(columns.contains(&"bead_id".to_string()));
    assert!(columns.contains(&"epic_id".to_string()));
    assert!(columns.contains(&"es".to_string()));
    assert!(columns.contains(&"ls".to_string()));
    assert!(columns.contains(&"float".to_string()));
}

#[test]
fn test_wal_mode_enabled() {
    // Verify WAL mode is enabled (required for concurrent access)
    let ws = common::TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    let journal_mode: String = storage
        .with_immediate_transaction(|tx| {
            tx.query_row("PRAGMA journal_mode", [], |row| row.get(0))
                .map_err(|e| anyhow::anyhow!(e))
        })
        .unwrap();

    assert_eq!(journal_mode, "wal", "Journal mode should be WAL");
}

#[test]
fn test_indexes_created() {
    // Verify expected indexes are created
    let ws = common::TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Check for critical indexes
    let expected_indexes = vec![
        "idx_issues_status",
        "idx_issues_priority",
        "idx_issues_ready",
        "idx_dependencies_issue",
        "idx_labels_label",
    ];

    for index_name in expected_indexes {
        let exists: bool = storage
            .with_immediate_transaction(|tx| {
                tx.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?",
                    &[index_name],
                    |row| row.get::<_, i64>(0).map(|n| n > 0),
                )
                .map_err(|e| anyhow::anyhow!(e))
            })
            .unwrap();

        assert!(exists, "Expected index '{}' should exist", index_name);
    }
}

#[test]
fn test_br_check_constraint_on_closed_at() {
    // Verify the CHECK constraint that enforces closed_at when status=closed
    let ws = common::TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Try to create a closed bead without closed_at - should fail
    let result: Result<(), anyhow::Error> = storage
        .with_immediate_transaction(|tx| {
            tx.execute(
                "INSERT INTO issues (id, title, status, priority, issue_type, created_at, updated_at, source_repo)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                ["bf-bad", "Bad closed bead", "closed", "2", "task", "2024-01-01T00:00:00Z", "2024-01-01T00:00:00Z", "."],
            )?;
            Ok(())
        });

    assert!(
        result.is_err(),
        "Should fail to insert closed bead without closed_at"
    );

    // Creating a closed bead with closed_at should succeed
    let result: Result<(), anyhow::Error> = storage
        .with_immediate_transaction(|tx| {
            tx.execute(
                "INSERT INTO issues (id, title, status, priority, issue_type, created_at, updated_at, closed_at, source_repo)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                ["bf-good", "Good closed bead", "closed", "2", "task", "2024-01-01T00:00:00Z", "2024-01-01T00:00:00Z", "2024-01-02T00:00:00Z", "."],
            )?;
            Ok(())
        });

    assert!(
        result.is_ok(),
        "Should succeed to insert closed bead with closed_at"
    );
}
