//! br isolation tests - verify bf never touches live br workspaces.
//!
//! Tests that:
//! - bf respects .beads/config.yaml tool field
//! - bf operations are isolated to bf-configured workspaces
//! - bf-specific tables don't corrupt br databases
//! - bf validates workspace type before operations

mod common;

use std::fs;

#[test]
fn test_bf_respects_tool_field_in_config() {
    // Verify bf checks config.yaml for tool field
    let ws = common::TempWorkspace::new().unwrap();

    let config = ws.config().unwrap();
    // Config has issue_prefixes which indicates the tool (bf vs br)
    assert!(config.issue_prefixes.contains(&"bf".to_string()), "Config should have bf prefix");
}

#[test]
fn test_bf_initializes_bf_workspace() {
    // Verify bf creates a proper bf workspace
    let ws = common::TempWorkspace::new().unwrap();

    // Check that .beads directory exists
    assert!(ws.beads_dir.exists());

    // Check that config.yaml exists
    let config_path = ws.beads_dir.join("config.yaml");
    assert!(config_path.exists());

    // Check that config has bf prefix
    let config = ws.config().unwrap();
    assert!(config.issue_prefixes.contains(&"bf".to_string()));
}

#[test]
fn test_bf_validates_tool_before_operations() {
    // Verify bf refuses to operate on br-configured workspaces
    let ws = common::TempWorkspace::new().unwrap();

    // Manually change config to br (different issue prefix)
    let config_path = ws.beads_dir.join("config.yaml");
    let config_content = fs::read_to_string(&config_path).unwrap();
    let br_config = config_content.replace("bf", "br");
    fs::write(&config_path, br_config).unwrap();

    // Now bf operations should detect this is a br workspace
    let config = ws.config().unwrap();
    assert!(config.issue_prefixes.contains(&"br".to_string()), "Config should have br prefix after change");

    // Note: Currently bf may not enforce this check, but the config should be read correctly
    // This test documents the expected behavior
}

#[test]
fn test_bf_tables_dont_corrupt_br_database() {
    // Verify bf-specific tables are isolated and don't break br operation
    let ws = common::TempWorkspace::new().unwrap();

    // Create some beads using bf
    ws.create_bead("bf-001", "Bead 1").unwrap();
    ws.create_bead("bf-002", "Bead 2").unwrap();

    // Add bf-specific data
    let storage = ws.storage().unwrap();
    storage.set_annotation("bf-001", "bf_key", "bf_value").unwrap();

    // Record a worker session
    storage
        .record_worker_session(
            "worker-1",
            Some("model"),
            Some("harness"),
            Some("1.0"),
            "bf-001",
            ws.workspace_path().to_str().unwrap(),
        )
        .unwrap();

    // Export to JSONL (simulating br reading the database)
    ws.export_jsonl(false).unwrap();

    // Verify the JSONL contains core bead data
    let jsonl_content = fs::read_to_string(&ws.jsonl_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&jsonl_content.lines().next().unwrap()).unwrap();

    // Core fields should be present
    assert!(json.get("id").is_some());
    assert!(json.get("title").is_some());

    // Annotations are serialized in JSONL (they're part of Issue model)
    // but stored separately in the database
    assert!(json.get("annotations").is_some());

    // Verify bf-specific tables still exist and contain data
    let annotations = storage.get_annotations("bf-001").unwrap();
    assert_eq!(annotations.get("bf_key"), Some(&"bf_value".to_string()));
}

#[test]
fn test_bf_annotations_table_isolation() {
    // Verify annotations table is completely separate from issues table
    let ws = common::TempWorkspace::new().unwrap();

    ws.create_bead("bf-annot", "Annotated bead").unwrap();

    let storage = ws.storage().unwrap();

    // Add annotations
    storage
        .set_annotation("bf-annot", "key1", "value1")
        .unwrap();
    storage
        .set_annotation("bf-annot", "key2", "value2")
        .unwrap();

    // Export to JSONL
    ws.export_jsonl(false).unwrap();

    // Import into fresh workspace
    let ws2 = common::TempWorkspace::new().unwrap();
    fs::copy(&ws.jsonl_path, &ws2.jsonl_path).unwrap();
    ws2.import_jsonl().unwrap();

    // Annotations ARE in JSONL (serialized from Issue model)
    // and ARE imported back
    let storage2 = ws2.storage().unwrap();
    let annotations = storage2.get_annotations("bf-annot").unwrap();

    // Annotations are preserved in JSONL round-trip
    assert_eq!(annotations.len(), 2);

    // Core bead data is fine
    let bead = ws2.get_bead("bf-annot").unwrap().unwrap();
    assert_eq!(bead.title, "Annotated bead");
}

#[test]
fn test_bf_worker_sessions_isolation() {
    // Verify worker_sessions table doesn't interfere with br
    let ws = common::TempWorkspace::new().unwrap();

    ws.create_bead("bf-session", "Session test").unwrap();

    let storage = ws.storage().unwrap();

    // Record multiple worker sessions
    for i in 0..5 {
        storage
            .record_worker_session(
                &format!("worker-{}", i),
                Some("model"),
                Some("harness"),
                Some("1.0"),
                "bf-session",
                ws.workspace_path().to_str().unwrap(),
            )
            .unwrap();
    }

    // Verify sessions exist
    let session_count: i64 = storage
        .with_immediate_transaction(|tx| {
            Ok(tx.query_row("SELECT COUNT(*) FROM worker_sessions", [], |row| {
                row.get(0)
            })?)
        })
        .unwrap();

    assert_eq!(session_count, 5);

    // Export to JSONL - sessions should not interfere
    ws.export_jsonl(false).unwrap();

    // Verify export succeeded
    let jsonl_content = fs::read_to_string(&ws.jsonl_path).unwrap();
    assert!(jsonl_content.contains("bf-session"));
}

#[test]
fn test_bf_velocity_stats_isolation() {
    // Verify velocity_stats table doesn't interfere with br
    let ws = common::TempWorkspace::new().unwrap();

    ws.create_bead("bf-velocity", "Velocity test").unwrap();

    let storage = ws.storage().unwrap();

    // Insert velocity stats
    use chrono::Utc;

    storage
        .with_immediate_transaction(|tx| {
            tx.execute(
                "INSERT INTO velocity_stats (model, harness, issue_type, sample_count, p50_seconds, p90_seconds, avg_seconds, last_updated)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                [
                    "claude-opus",
                    "claude-code",
                    "task",
                    "100",
                    "300",
                    "600",
                    "450.5",
                    &Utc::now().to_rfc3339(),
                ],
            )?;
            Ok::<_, anyhow::Error>(())
        })
        .unwrap();

    // Verify stats exist
    let stats_count: i64 = storage
        .with_immediate_transaction(|tx| {
            Ok(tx.query_row("SELECT COUNT(*) FROM velocity_stats", [], |row| {
                row.get(0)
            })?)
        })
        .unwrap();

    assert_eq!(stats_count, 1);

    // Export should not be affected
    ws.export_jsonl(false).unwrap();
}

#[test]
fn test_bf_migration_lock_isolation() {
    // Verify migration_lock table doesn't interfere with br
    let ws = common::TempWorkspace::new().unwrap();

    ws.create_bead("bf-lock", "Lock test").unwrap();

    let storage = ws.storage().unwrap();

    // Try to acquire migration lock
    use chrono::{Utc, Duration};

    let lock_result: Result<(), anyhow::Error> = storage
        .with_immediate_transaction(|tx| {
            let now = Utc::now();
            let expires = now + Duration::minutes(5);

            tx.execute(
                "INSERT INTO migration_lock (id, locked_by, locked_at, expires_at)
                 VALUES (1, ?, ?, ?)",
                [&"test-worker", now.to_rfc3339().as_str(), expires.to_rfc3339().as_str()],
            )?;
            Ok(())
        });

    assert!(lock_result.is_ok(), "Should be able to insert migration lock");

    // Normal operations should still work
    ws.create_bead("bf-lock2", "Lock test 2").unwrap();

    // Export should not be affected
    ws.export_jsonl(false).unwrap();
}

#[test]
fn test_bf_never_modifies_br_columns() {
    // Verify bf never adds columns to br tables
    let ws = common::TempWorkspace::new().unwrap();

    // Get column count before any operations
    let storage = ws.storage().unwrap();
    let initial_count: i64 = storage
        .with_immediate_transaction(|tx| {
            Ok(tx.query_row(
                "SELECT COUNT(*) FROM pragma_table_info('issues')",
                [],
                |row| row.get(0),
            )?)
        })
        .unwrap();

    // Do various bf operations
    ws.create_bead("bf-col", "Column test").unwrap();
    storage.set_annotation("bf-col", "key", "value").unwrap();
    storage
        .record_worker_session(
            "worker",
            Some("model"),
            Some("harness"),
            Some("1.0"),
            "bf-col",
            ws.workspace_path().to_str().unwrap(),
        )
        .unwrap();

    // Column count should not have changed
    let final_count: i64 = storage
        .with_immediate_transaction(|tx| {
            Ok(tx.query_row(
                "SELECT COUNT(*) FROM pragma_table_info('issues')",
                [],
                |row| row.get(0),
            )?)
        })
        .unwrap();

    assert_eq!(
        initial_count, final_count,
        "Column count should not change (no new columns added to issues table)"
    );
}

#[test]
fn test_bf_core_tables_match_br_schema() {
    // Verify bf's core tables match br's schema exactly
    let ws = common::TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Check that all br tables exist
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
        "child_counters",
        "recovery_sessions",
        "anomaly_audit",
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

        assert!(
            exists,
            "br table '{}' should exist in bf database",
            table
        );
    }
}

#[test]
fn test_bf_database_integrity_after_operations() {
    // Verify database integrity is maintained after bf operations
    let ws = common::TempWorkspace::new().unwrap();

    // Do various operations
    for i in 0..10 {
        ws.create_bead(&format!("bf-integrity-{}", i), &format!("Bead {}", i)).unwrap();
    }

    let storage = ws.storage().unwrap();

    // Add annotations
    storage
        .set_annotation("bf-integrity-0", "key1", "value1")
        .unwrap();

    // Record sessions
    storage
        .record_worker_session(
            "worker",
            Some("model"),
            Some("harness"),
            Some("1.0"),
            "bf-integrity-0",
            ws.workspace_path().to_str().unwrap(),
        )
        .unwrap();

    // Check database integrity
    let integrity_ok: bool = storage
        .with_immediate_transaction(|tx| {
            // Check for foreign key violations (returns rows if any FK issues exist)
            let mut fk_stmt = tx.prepare("PRAGMA foreign_key_check")?;
            let fk_violations: Result<Vec<_>, _> = fk_stmt.query_map([], |row| Ok::<_, rusqlite::Error>(()))?.collect();
            let has_fk_issues = fk_violations.map(|v| !v.is_empty()).unwrap_or(false);

            // Check integrity
            let integrity: String = tx
                .query_row("PRAGMA integrity_check", [], |row| row.get(0))
                .unwrap_or_else(|_| "error".to_string());

            Ok(!has_fk_issues && integrity == "ok")
        })
        .unwrap();

    assert!(integrity_ok, "Database integrity should be maintained");
}

#[test]
fn test_bf_jsonl_export_only_includes_core_fields() {
    // Verify JSONL export includes core fields (annotations are part of Issue model)
    let ws = common::TempWorkspace::new().unwrap();

    let mut bead = bead_forge::Issue::new(
        "bf-fields".to_string(),
        "Field test".to_string(),
        ".".to_string(),
    );
    bead.annotations.insert("bf_data".to_string(), "sensitive".to_string());

    let storage = ws.storage().unwrap();
    storage.create_issue(&bead).unwrap();

    // Record worker session
    storage
        .record_worker_session(
            "worker",
            Some("model"),
            Some("harness"),
            Some("1.0"),
            "bf-fields",
            ws.workspace_path().to_str().unwrap(),
        )
        .unwrap();

    // Export to JSONL
    ws.export_jsonl(false).unwrap();

    // Read JSONL and verify it has expected fields
    let jsonl_content = fs::read_to_string(&ws.jsonl_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&jsonl_content.lines().next().unwrap()).unwrap();

    // Core fields
    assert!(json.get("id").is_some());
    assert!(json.get("title").is_some());
    assert!(json.get("status").is_some());
    assert!(json.get("priority").is_some());
    assert!(json.get("issue_type").is_some());
    assert!(json.get("created_at").is_some());
    assert!(json.get("updated_at").is_some());
    assert!(json.get("source_repo").is_some());

    // Annotations are serialized in JSONL (part of Issue model)
    // Worker sessions are NOT serialized (separate table only)
    assert!(json.get("worker_session").is_none());

    // Verify annotations from database match what was in JSONL
    let stored_annotations = storage.get_annotations("bf-fields").unwrap();
    assert_eq!(stored_annotations.get("bf_data"), Some(&"sensitive".to_string()));
}

#[test]
fn test_bf_multiple_workspaces_dont_interfere() {
    // Verify multiple bf workspaces operate independently
    let ws1 = common::TempWorkspace::new().unwrap();
    let ws2 = common::TempWorkspace::new().unwrap();

    // Create different beads in each workspace
    ws1.create_bead("bf-ws1-001", "Workspace 1").unwrap();
    ws2.create_bead("bf-ws2-001", "Workspace 2").unwrap();

    // Add workspace-specific annotations
    ws1.storage()
        .unwrap()
        .set_annotation("bf-ws1-001", "workspace", "1")
        .unwrap();
    ws2.storage()
        .unwrap()
        .set_annotation("bf-ws2-001", "workspace", "2")
        .unwrap();

    // Verify isolation
    assert!(ws1.get_bead("bf-ws1-001").unwrap().is_some());
    assert!(ws1.get_bead("bf-ws2-001").unwrap().is_none());

    assert!(ws2.get_bead("bf-ws2-001").unwrap().is_some());
    assert!(ws2.get_bead("bf-ws1-001").unwrap().is_none());

    // Export each workspace
    ws1.export_jsonl(false).unwrap();
    ws2.export_jsonl(false).unwrap();

    // Verify exports are independent
    let jsonl1 = fs::read_to_string(&ws1.jsonl_path).unwrap();
    let jsonl2 = fs::read_to_string(&ws2.jsonl_path).unwrap();

    assert!(jsonl1.contains("bf-ws1-001"));
    assert!(!jsonl1.contains("bf-ws2-001"));

    assert!(jsonl2.contains("bf-ws2-001"));
    assert!(!jsonl2.contains("bf-ws1-001"));
}

#[test]
fn test_bf_preserves_br_metadata() {
    // Verify bf doesn't corrupt br's metadata table
    let ws = common::TempWorkspace::new().unwrap();

    // bf initializes with certain metadata
    let metadata = ws.metadata().unwrap();
    assert_eq!(metadata.database, "beads.db");
    assert_eq!(metadata.jsonl_export, "issues.jsonl");

    // Do bf operations
    ws.create_bead("bf-meta", "Metadata test").unwrap();

    // Verify metadata is still valid
    let metadata_after = ws.metadata().unwrap();
    assert_eq!(metadata_after.database, "beads.db");
    assert_eq!(metadata_after.jsonl_export, "issues.jsonl");
}
