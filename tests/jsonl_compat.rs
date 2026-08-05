//! JSONL round-trip compatibility tests with br.
//!
//! Tests that bf can:
//! - Import br-generated JSONL files
//! - Export JSONL that br can import
//! - Handle all br field types correctly
//! - Preserve data integrity across round-trips

mod common;

use std::fs;

#[test]
fn test_jsonl_round_trip_simple_bead() {
    let ws = common::TempWorkspace::new().unwrap();
    ws.create_bead("bf-001", "Simple bead").unwrap();

    // Export to JSONL
    let exported = ws.export_jsonl(false).unwrap();
    assert_eq!(exported, 1);

    // Read exported JSONL
    let jsonl_content = fs::read_to_string(&ws.jsonl_path).unwrap();
    let lines: Vec<&str> = jsonl_content.lines().collect();
    assert_eq!(lines.len(), 1);

    // Verify it can be re-imported
    let storage = ws.storage().unwrap();
    storage.sync_from_jsonl(&ws.jsonl_path).unwrap();

    let bead = ws.get_bead("bf-001").unwrap().unwrap();
    assert_eq!(bead.id, "bf-001");
    assert_eq!(bead.title, "Simple bead");
}

#[test]
fn test_jsonl_round_trip_with_all_fields() {
    let ws = common::TempWorkspace::new().unwrap();

    // Create a bead with all fields populated
    let mut bead = bead_forge::Issue::new(
        "bf-full".to_string(),
        "Full bead".to_string(),
        "/path/to/repo".to_string(),
    );
    bead.description = Some("Description".to_string());
    bead.design = Some("Design".to_string());
    bead.acceptance_criteria = Some("Criteria".to_string());
    bead.notes = Some("Notes".to_string());
    bead.priority = bead_forge::model::Priority(0);
    bead.assignee = Some("alice".to_string());
    bead.owner = Some("bob".to_string());
    bead.estimated_minutes = Some(60);
    bead.due_at = Some(chrono::Utc::now());
    bead.external_ref = Some("EXT-123".to_string());
    bead.source_system = Some("jira".to_string());
    bead.ephemeral = true;
    bead.pinned = true;
    bead.is_template = true;
    bead.labels = vec!["urgent".to_string(), "bug".to_string()];

    let storage = ws.storage().unwrap();
    storage.create_issue(&bead).unwrap();

    // Export to JSONL
    ws.export_jsonl(false).unwrap();

    // Import into fresh workspace
    let ws2 = common::TempWorkspace::new().unwrap();
    fs::copy(&ws.jsonl_path, &ws2.jsonl_path).unwrap();
    ws2.import_jsonl().unwrap();

    let imported = ws2.get_bead("bf-full").unwrap().unwrap();
    assert_eq!(imported.id, "bf-full");
    assert_eq!(imported.title, "Full bead");
    assert_eq!(imported.description, bead.description);
    assert_eq!(imported.design, bead.design);
    assert_eq!(imported.acceptance_criteria, bead.acceptance_criteria);
    assert_eq!(imported.notes, bead.notes);
    assert_eq!(imported.priority, bead.priority);
    assert_eq!(imported.assignee, bead.assignee);
    assert_eq!(imported.owner, bead.owner);
    assert_eq!(imported.estimated_minutes, bead.estimated_minutes);
    assert_eq!(imported.external_ref, bead.external_ref);
    assert_eq!(imported.source_system, bead.source_system);
    assert_eq!(imported.ephemeral, bead.ephemeral);
    assert_eq!(imported.pinned, bead.pinned);
    assert_eq!(imported.is_template, bead.is_template);

    // Labels need special handling (order may vary)
    assert_eq!(imported.labels.len(), bead.labels.len());
    for label in &bead.labels {
        assert!(imported.labels.contains(label));
    }
}

#[test]
fn test_jsonl_round_trip_with_dependencies() {
    let ws = common::TempWorkspace::new().unwrap();

    // Create parent bead
    ws.create_bead("bf-parent", "Parent").unwrap();

    // Create child bead with dependency
    let mut child =
        bead_forge::Issue::new("bf-child".to_string(), "Child".to_string(), ".".to_string());
    child.dependencies.push(bead_forge::model::Dependency {
        issue_id: "bf-child".to_string(),
        depends_on_id: "bf-parent".to_string(),
        dep_type: bead_forge::model::DependencyType::Blocks,
        created_at: chrono::Utc::now(),
        created_by: Some("test".to_string()),
        metadata: None,
        thread_id: None,
        title: None,
    });

    let storage = ws.storage().unwrap();
    storage.create_issue(&child).unwrap();

    // Export and re-import
    ws.export_jsonl(false).unwrap();

    let ws2 = common::TempWorkspace::new().unwrap();
    fs::copy(&ws.jsonl_path, &ws2.jsonl_path).unwrap();
    ws2.import_jsonl().unwrap();

    let imported = ws2.get_bead("bf-child").unwrap().unwrap();
    assert_eq!(imported.dependencies.len(), 1);
    assert_eq!(imported.dependencies[0].issue_id, "bf-child");
    assert_eq!(imported.dependencies[0].depends_on_id, "bf-parent");
}

#[test]
fn test_jsonl_round_trip_closed_bead() {
    let ws = common::TempWorkspace::new().unwrap();

    let mut bead = bead_forge::Issue::new(
        "bf-closed".to_string(),
        "Closed bead".to_string(),
        ".".to_string(),
    );
    bead.status = bead_forge::Status::Closed;
    bead.closed_at = Some(chrono::Utc::now());
    bead.close_reason = Some("Done".to_string());
    bead.closed_by_session = Some("session-123".to_string());

    let storage = ws.storage().unwrap();
    storage.create_issue(&bead).unwrap();

    // Export and re-import
    ws.export_jsonl(false).unwrap();

    let ws2 = common::TempWorkspace::new().unwrap();
    fs::copy(&ws.jsonl_path, &ws2.jsonl_path).unwrap();
    ws2.import_jsonl().unwrap();

    let imported = ws2.get_bead("bf-closed").unwrap().unwrap();
    assert_eq!(imported.status.to_string(), "closed");
    assert!(imported.closed_at.is_some());
    assert_eq!(imported.close_reason, Some("Done".to_string()));
    assert_eq!(imported.closed_by_session, Some("session-123".to_string()));
}

#[test]
fn test_jsonl_import_empty_file() {
    let ws = common::TempWorkspace::new().unwrap();

    // Write empty JSONL
    fs::write(&ws.jsonl_path, "").unwrap();

    let result = ws.import_jsonl().unwrap();
    assert_eq!(result.imported, 0);
    assert_eq!(result.updated, 0);
    assert_eq!(result.skipped, 0);
}

#[test]
fn test_jsonl_import_mixed_statuses() {
    let ws = common::TempWorkspace::new().unwrap();

    let jsonl = format!(
        "{}\n{}\n{}\n{}",
        common::sample_bead_jsonl("bf-001", "Open bead"),
        common::sample_closed_bead_jsonl("bf-002", "Closed bead", "Complete"),
        r#"{"id":"bf-003","title":"In progress","description":"","design":"","acceptance_criteria":"","notes":"","status":"in_progress","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":".","labels":[],"dependencies":[],"comments":[]}"#,
        r#"{"id":"bf-004","title":"Blocked","description":"","design":"","acceptance_criteria":"","notes":"","status":"blocked","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":".","labels":[],"dependencies":[],"comments":[]}"#
    );

    fs::write(&ws.jsonl_path, jsonl).unwrap();
    ws.import_jsonl().unwrap();

    assert_eq!(ws.count_beads().unwrap(), 4);

    let open = ws.get_bead("bf-001").unwrap().unwrap();
    assert_eq!(open.status.to_string(), "open");

    let closed = ws.get_bead("bf-002").unwrap().unwrap();
    assert_eq!(closed.status.to_string(), "closed");

    let in_progress = ws.get_bead("bf-003").unwrap().unwrap();
    assert_eq!(in_progress.status.to_string(), "in_progress");

    let blocked = ws.get_bead("bf-004").unwrap().unwrap();
    assert_eq!(blocked.status.to_string(), "blocked");
}

#[test]
fn test_jsonl_export_dirty_only() {
    let ws = common::TempWorkspace::new().unwrap();

    // Create two beads
    ws.create_bead("bf-001", "Bead 1").unwrap();
    ws.create_bead("bf-002", "Bead 2").unwrap();

    // Clear dirty flags (both beads are dirty after creation)
    let storage = ws.storage().unwrap();
    storage.clear_dirty().unwrap();

    // Mark one as dirty
    storage.mark_dirty("bf-001").unwrap();

    // Export dirty only
    let exported = ws.export_jsonl(true).unwrap();
    assert_eq!(exported, 1);

    // Verify only dirty bead is in JSONL
    let jsonl_content = fs::read_to_string(&ws.jsonl_path).unwrap();
    assert!(jsonl_content.contains("bf-001"));
    assert!(!jsonl_content.contains("bf-002"));

    // After export, dirty should be cleared
    let dirty_count: i64 = storage
        .with_immediate_transaction(|tx| {
            tx.query_row("SELECT COUNT(*) FROM dirty_issues", [], |row| row.get(0))
                .map_err(|e| anyhow::anyhow!(e))
        })
        .unwrap();
    assert_eq!(dirty_count, 0);
}

#[test]
fn test_jsonl_round_trip_recomputes_content_hash() {
    let ws = common::TempWorkspace::new().unwrap();

    // Create bead with specific content
    let mut bead = bead_forge::Issue::new(
        "bf-hash".to_string(),
        "Hash test".to_string(),
        ".".to_string(),
    );
    bead.description = Some("Specific content for hash".to_string());

    let storage = ws.storage().unwrap();
    storage.create_issue(&bead).unwrap();

    // Get original bead
    let original = ws.get_bead("bf-hash").unwrap().unwrap();

    // Export and re-import
    ws.export_jsonl(false).unwrap();

    let ws2 = common::TempWorkspace::new().unwrap();
    fs::copy(&ws.jsonl_path, &ws2.jsonl_path).unwrap();

    // First import
    let result1 = ws2.import_jsonl().unwrap();
    assert_eq!(result1.imported, 1);

    // Second import should skip unchanged (hash is recomputed and matches)
    let result2 = ws2.import_jsonl().unwrap();
    assert_eq!(result2.imported, 0);
    assert_eq!(result2.skipped, 1);

    // Verify content is semantically identical using sync_equals
    let reimported = ws2.get_bead("bf-hash").unwrap().unwrap();
    assert!(
        original.sync_equals(&reimported),
        "Content should be semantically identical after JSONL round-trip"
    );

    // Verify content_hash was computed (not None)
    assert!(
        reimported.content_hash.is_some(),
        "content_hash should be computed on import"
    );

    // Verify the JSONL doesn't contain content_hash (it's #[serde(skip)])
    let jsonl_content = fs::read_to_string(&ws.jsonl_path).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&jsonl_content.lines().next().unwrap()).unwrap();
    assert!(
        json.get("content_hash").is_none(),
        "content_hash should NOT be serialized in JSONL (it's #[serde(skip)])"
    );
}

#[test]
fn test_jsonl_import_updates_changed_content() {
    let ws = common::TempWorkspace::new().unwrap();

    // Create initial bead
    let mut bead = bead_forge::Issue::new(
        "bf-update".to_string(),
        "Original".to_string(),
        ".".to_string(),
    );
    bead.description = Some("Original description".to_string());

    let storage = ws.storage().unwrap();
    storage.create_issue(&bead).unwrap();

    // Export to JSONL first (this will compute and store content_hash)
    ws.export_jsonl(false).unwrap();

    // Read the exported JSONL and modify it
    let original_jsonl = fs::read_to_string(&ws.jsonl_path).unwrap();
    let mut original_value: serde_json::Value = serde_json::from_str(&original_jsonl).unwrap();

    // Modify the content
    original_value["title"] = serde_json::json!("Updated");
    original_value["description"] = serde_json::json!("New description");

    // Write the modified JSONL
    let modified_jsonl = serde_json::to_string(&original_value).unwrap();
    fs::write(&ws.jsonl_path, modified_jsonl).unwrap();

    let result = ws.import_jsonl().unwrap();
    // The updated count should be 1 since the content_hash will differ
    assert_eq!(result.updated, 1);

    let updated = ws.get_bead("bf-update").unwrap().unwrap();
    assert_eq!(updated.title, "Updated");
    assert_eq!(updated.description, Some("New description".to_string()));
}

#[test]
fn test_jsonl_import_with_comments() {
    let ws = common::TempWorkspace::new().unwrap();

    let jsonl = r#"{"id":"bf-comment","title":"With comment","description":"","design":"","acceptance_criteria":"","notes":"","status":"open","priority":2,"issue_type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":".","labels":[],"dependencies":[],"comments":[{"id":1,"issue_id":"bf-comment","author":"alice","text":"Great idea!","created_at":"2024-01-01T01:00:00Z"}]}"#;

    fs::write(&ws.jsonl_path, jsonl).unwrap();
    ws.import_jsonl().unwrap();

    let bead = ws.get_bead("bf-comment").unwrap().unwrap();
    assert_eq!(bead.comments.len(), 1);
    assert_eq!(bead.comments[0].author, "alice");
    assert_eq!(bead.comments[0].body, "Great idea!");
}

#[test]
fn test_jsonl_round_trip_preserves_timestamps() {
    let ws = common::TempWorkspace::new().unwrap();

    // Create bead with specific timestamps
    let created = chrono::DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let updated = chrono::DateTime::parse_from_rfc3339("2024-01-16T14:45:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let mut bead = bead_forge::Issue::new(
        "bf-ts".to_string(),
        "Timestamp test".to_string(),
        ".".to_string(),
    );
    bead.created_at = created;
    bead.updated_at = updated;

    let storage = ws.storage().unwrap();
    storage.create_issue(&bead).unwrap();

    // Export and re-import
    ws.export_jsonl(false).unwrap();

    let ws2 = common::TempWorkspace::new().unwrap();
    fs::copy(&ws.jsonl_path, &ws2.jsonl_path).unwrap();
    ws2.import_jsonl().unwrap();

    let reimported = ws2.get_bead("bf-ts").unwrap().unwrap();
    assert_eq!(reimported.created_at, created);
    assert_eq!(reimported.updated_at, updated);
}

#[test]
fn test_jsonl_handles_unicode() {
    let ws = common::TempWorkspace::new().unwrap();

    // Create bead with Unicode content
    let mut bead = bead_forge::Issue::new(
        "bf-unicode".to_string(),
        "Unicode: 你好 🎉 Ñoño".to_string(),
        ".".to_string(),
    );
    bead.description = Some("Description with émojis: 🚀 🔥".to_string());
    bead.notes = Some("Notes: Café, naïve, Zürich".to_string());

    let storage = ws.storage().unwrap();
    storage.create_issue(&bead).unwrap();

    // Export and re-import
    ws.export_jsonl(false).unwrap();

    let ws2 = common::TempWorkspace::new().unwrap();
    fs::copy(&ws.jsonl_path, &ws2.jsonl_path).unwrap();
    ws2.import_jsonl().unwrap();

    let reimported = ws2.get_bead("bf-unicode").unwrap().unwrap();
    assert_eq!(reimported.title, "Unicode: 你好 🎉 Ñoño");
    assert_eq!(
        reimported.description,
        Some("Description with émojis: 🚀 🔥".to_string())
    );
    assert_eq!(
        reimported.notes,
        Some("Notes: Café, naïve, Zürich".to_string())
    );
}

#[test]
fn test_jsonl_handles_special_characters() {
    let ws = common::TempWorkspace::new().unwrap();

    // Test JSON escaping in content - construct the Issue and serialize it properly
    let special_content = "Text with \"quotes\" and\nnewlines and\ttabs";

    let mut bead = bead_forge::Issue::new(
        "bf-special".to_string(),
        "Special".to_string(),
        ".".to_string(),
    );
    bead.description = Some(special_content.to_string());
    bead.created_at = chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    bead.updated_at = chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let jsonl = serde_json::to_string(&bead).unwrap();
    fs::write(&ws.jsonl_path, jsonl).unwrap();
    ws.import_jsonl().unwrap();

    let imported = ws.get_bead("bf-special").unwrap().unwrap();
    assert_eq!(imported.description, Some(special_content.to_string()));
}

#[test]
fn test_jsonl_import_preserves_issue_types() {
    let ws = common::TempWorkspace::new().unwrap();

    let jsonl = format!(
        "{}\n{}\n{}\n{}",
        r#"{"id":"bf-task","title":"Task","description":"","design":"","acceptance_criteria":"","notes":"","status":"open","priority":2,"issue_type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":".","labels":[],"dependencies":[],"comments":[]}"#,
        r#"{"id":"bf-bug","title":"Bug","description":"","design":"","acceptance_criteria":"","notes":"","status":"open","priority":2,"issue_type":"bug","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":".","labels":[],"dependencies":[],"comments":[]}"#,
        r#"{"id":"bf-feature","title":"Feature","description":"","design":"","acceptance_criteria":"","notes":"","status":"open","priority":2,"issue_type":"feature","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":".","labels":[],"dependencies":[],"comments":[]}"#,
        r#"{"id":"bf-epic","title":"Epic","description":"","design":"","acceptance_criteria":"","notes":"","status":"open","priority":2,"issue_type":"epic","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":".","labels":[],"dependencies":[],"comments":[]}"#
    );

    fs::write(&ws.jsonl_path, jsonl).unwrap();
    ws.import_jsonl().unwrap();

    let task = ws.get_bead("bf-task").unwrap().unwrap();
    assert!(matches!(task.issue_type, bead_forge::IssueType::Task));

    let bug = ws.get_bead("bf-bug").unwrap().unwrap();
    assert!(matches!(bug.issue_type, bead_forge::IssueType::Bug));

    let feature = ws.get_bead("bf-feature").unwrap().unwrap();
    assert!(matches!(feature.issue_type, bead_forge::IssueType::Feature));

    let epic = ws.get_bead("bf-epic").unwrap().unwrap();
    assert!(matches!(epic.issue_type, bead_forge::IssueType::Epic));
}

#[test]
fn test_jsonl_handles_empty_optional_fields() {
    let ws = common::TempWorkspace::new().unwrap();

    // JSONL with null and empty string optional fields
    let jsonl = r#"{"id":"bf-empty","title":"Empty fields","description":null,"design":"","acceptance_criteria":"","notes":"","status":"open","priority":2,"type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","source_repo":".","assignee":null,"owner":null,"due_at":null,"defer_until":null,"external_ref":null,"source_system":null,"labels":[],"dependencies":[],"comments":[]}"#;

    fs::write(&ws.jsonl_path, jsonl).unwrap();
    ws.import_jsonl().unwrap();

    let bead = ws.get_bead("bf-empty").unwrap().unwrap();
    assert_eq!(bead.description, Some("".to_string()));
    assert_eq!(bead.design, Some("".to_string()));
    assert_eq!(bead.assignee, None);
    assert_eq!(bead.owner, None);
    assert_eq!(bead.due_at, None);
    assert_eq!(bead.defer_until, None);
    assert_eq!(bead.external_ref, None);
    assert_eq!(bead.source_system, None);
}
