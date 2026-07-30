//! Integration tests for JSONL round-trip workflow.
//!
//! Tests the complete cycle:
//! 1. Create beads in SQLite
//! 2. Export to JSONL (full and dirty-only)
//! 3. Import back into SQLite
//! 4. Verify all data is preserved correctly
//!
//! Tests cover various bead configurations including labels, dependencies,
//! comments, and edge cases.

mod common;

use bead_forge::model::{DependencyType, Issue, IssueChanges, IssueType, Priority, Status};
use chrono::{Duration, Utc};
use std::fs;

/// Test basic round-trip: export all beads, import them back, verify identical.
#[test]
fn test_full_export_import_roundtrip_preserves_all_beads() {
    let ws1 = common::TempWorkspace::new().expect("Failed to create workspace 1");

    // Create test beads
    let bead1 = Issue::new("bf-001".to_string(), "First bead".to_string(), ".".to_string());
    let bead2 = Issue::new("bf-002".to_string(), "Second bead".to_string(), ".".to_string());
    let bead3 = Issue::new("bf-003".to_string(), "Third bead".to_string(), ".".to_string());

    ws1.create_issue(&bead1).expect("Failed to create bead1");
    ws1.create_issue(&bead2).expect("Failed to create bead2");
    ws1.create_issue(&bead3).expect("Failed to create bead3");

    // Export all beads to JSONL
    let export_count = ws1.export_jsonl(false).expect("Failed to export JSONL");
    assert_eq!(export_count, 3, "Should export 3 beads");

    // Read exported JSONL
    let exported_jsonl = fs::read_to_string(&ws1.jsonl_path).expect("Failed to read exported JSONL");

    // Create second workspace and import
    let ws2 = common::TempWorkspace::new().expect("Failed to create workspace 2");
    fs::write(&ws2.jsonl_path, &exported_jsonl).expect("Failed to write JSONL to ws2");

    let import_result = ws2.import_jsonl().expect("Failed to import JSONL");
    assert_eq!(import_result.imported, 3, "Should import 3 beads as new");

    // Verify all beads present in ws2
    let imported_beads = ws2.list_beads().expect("Failed to list beads");
    assert_eq!(imported_beads.len(), 3, "Should have 3 beads after import");

    // Verify each bead's core fields match
    for bead in &imported_beads {
        let original = match bead.id.as_str() {
            "bf-001" => &bead1,
            "bf-002" => &bead2,
            "bf-003" => &bead3,
            _ => panic!("Unexpected bead ID: {}", bead.id),
        };

        assert_eq!(bead.id, original.id);
        assert_eq!(bead.title, original.title);
        assert_eq!(bead.status, original.status);
        assert_eq!(bead.priority, original.priority);
        assert_eq!(bead.issue_type, original.issue_type);
        assert_eq!(bead.source_repo, original.source_repo);
    }
}

/// Test round-trip with beads that have labels.
#[test]
fn test_roundtrip_preserves_labels() {
    let ws1 = common::TempWorkspace::new().expect("Failed to create workspace");

    // Create beads with various label configurations
    let mut bead_with_labels = Issue::new("bf-labels".to_string(), "Bead with labels".to_string(), ".".to_string());
    bead_with_labels.labels = vec![
        "phase-1".to_string(),
        "storage".to_string(),
        "critical".to_string(),
    ];

    let bead_no_labels = Issue::new("bf-nolabels".to_string(), "Bead without labels".to_string(), ".".to_string());

    let mut bead_single_label = Issue::new("bf-single".to_string(), "Single label".to_string(), ".".to_string());
    bead_single_label.labels = vec!["bug".to_string()];

    ws1.create_issue(&bead_with_labels).expect("Failed to create bead_with_labels");
    ws1.create_issue(&bead_no_labels).expect("Failed to create bead_no_labels");
    ws1.create_issue(&bead_single_label).expect("Failed to create bead_single_label");

    // Export and import
    let export_count = ws1.export_jsonl(false).expect("Failed to export");
    assert_eq!(export_count, 3);

    let exported_jsonl = fs::read_to_string(&ws1.jsonl_path).expect("Failed to read exported JSONL");
    let ws2 = common::TempWorkspace::new().expect("Failed to create workspace 2");
    fs::write(&ws2.jsonl_path, &exported_jsonl).expect("Failed to write JSONL");

    ws2.import_jsonl().expect("Failed to import");

    // Verify labels preserved
    let imported_beads = ws2.list_beads().expect("Failed to list beads");

    let imported_with_labels = imported_beads.iter().find(|b| b.id == "bf-labels").unwrap();
    assert_eq!(imported_with_labels.labels.len(), 3, "Should preserve all labels");
    assert!(imported_with_labels.labels.contains(&"phase-1".to_string()));
    assert!(imported_with_labels.labels.contains(&"storage".to_string()));
    assert!(imported_with_labels.labels.contains(&"critical".to_string()));

    let imported_no_labels = imported_beads.iter().find(|b| b.id == "bf-nolabels").unwrap();
    assert_eq!(imported_no_labels.labels.len(), 0, "Should have no labels");

    let imported_single = imported_beads.iter().find(|b| b.id == "bf-single").unwrap();
    assert_eq!(imported_single.labels.len(), 1, "Should have single label");
    assert!(imported_single.labels.contains(&"bug".to_string()));
}

/// Test round-trip with beads that have dependencies.
#[test]
fn test_roundtrip_preserves_dependencies() {
    let ws1 = common::TempWorkspace::new().expect("Failed to create workspace");

    // Create dependent beads
    let bead_a = Issue::new("bf-a".to_string(), "Bead A".to_string(), ".".to_string());
    let bead_b = Issue::new("bf-b".to_string(), "Bead B".to_string(), ".".to_string());
    let bead_c = Issue::new("bf-c".to_string(), "Bead C".to_string(), ".".to_string());

    ws1.create_issue(&bead_a).expect("Failed to create bead_a");
    ws1.create_issue(&bead_b).expect("Failed to create bead_b");
    ws1.create_issue(&bead_c).expect("Failed to create bead_c");

    // Add dependencies using storage directly
    let storage = ws1.storage().expect("Failed to open storage");

    // B depends on A
    storage
        .add_dependency("bf-b", "bf-a", &DependencyType::Blocks, "test")
        .expect("Failed to add dependency B->A");

    // C depends on B (chain: C -> B -> A)
    storage
        .add_dependency("bf-c", "bf-b", &DependencyType::Blocks, "test")
        .expect("Failed to add dependency C->B");

    // Export and import
    let export_count = ws1.export_jsonl(false).expect("Failed to export");
    assert_eq!(export_count, 3);

    let exported_jsonl = fs::read_to_string(&ws1.jsonl_path).expect("Failed to read exported JSONL");
    let ws2 = common::TempWorkspace::new().expect("Failed to create workspace 2");
    fs::write(&ws2.jsonl_path, &exported_jsonl).expect("Failed to write JSONL");

    ws2.import_jsonl().expect("Failed to import");

    // Verify dependencies preserved
    let imported_beads = ws2.list_beads().expect("Failed to list beads");

    let imported_b = imported_beads.iter().find(|b| b.id == "bf-b").unwrap();
    assert_eq!(imported_b.dependencies.len(), 1, "B should have 1 dependency");
    assert_eq!(imported_b.dependencies[0].depends_on_id, "bf-a");
    assert_eq!(imported_b.dependencies[0].dep_type, DependencyType::Blocks);

    let imported_c = imported_beads.iter().find(|b| b.id == "bf-c").unwrap();
    assert_eq!(imported_c.dependencies.len(), 1, "C should have 1 dependency");
    assert_eq!(imported_c.dependencies[0].depends_on_id, "bf-b");
}

/// Test round-trip with beads that have comments.
#[test]
fn test_roundtrip_preserves_comments() {
    let ws1 = common::TempWorkspace::new().expect("Failed to create workspace");

    let bead = Issue::new("bf-comments".to_string(), "Bead with comments".to_string(), ".".to_string());
    ws1.create_issue(&bead).expect("Failed to create bead");

    // Add comments
    let storage = ws1.storage().expect("Failed to open storage");

    storage
        .add_comment("bf-comments", "alice", "First comment")
        .expect("Failed to add comment1");

    storage
        .add_comment("bf-comments", "bob", "Second comment")
        .expect("Failed to add comment2");

    // Export and import
    let export_count = ws1.export_jsonl(false).expect("Failed to export");
    assert_eq!(export_count, 1);

    let exported_jsonl = fs::read_to_string(&ws1.jsonl_path).expect("Failed to read exported JSONL");
    let ws2 = common::TempWorkspace::new().expect("Failed to create workspace 2");
    fs::write(&ws2.jsonl_path, &exported_jsonl).expect("Failed to write JSONL");

    ws2.import_jsonl().expect("Failed to import");

    // Verify comments preserved
    let imported_beads = ws2.list_beads().expect("Failed to list beads");
    assert_eq!(imported_beads.len(), 1);

    let imported = &imported_beads[0];
    assert_eq!(imported.comments.len(), 2, "Should preserve all comments");

    // Comments should be in creation order
    assert_eq!(imported.comments[0].author, "alice");
    assert_eq!(imported.comments[0].body, "First comment");
    assert_eq!(imported.comments[1].author, "bob");
    assert_eq!(imported.comments[1].body, "Second comment");
}

/// Test round-trip with complex beads (labels + dependencies + comments).
#[test]
fn test_roundtrip_with_complex_bead_configuration() {
    let ws1 = common::TempWorkspace::new().expect("Failed to create workspace");

    // Create a complex bead with everything
    let mut parent = Issue::new("bf-parent".to_string(), "Parent epic".to_string(), ".".to_string());
    parent.labels = vec!["epic".to_string(), "phase-2".to_string()];
    parent.issue_type = IssueType::Epic;
    parent.priority = Priority(0); // P0

    let mut child = Issue::new("bf-child".to_string(), "Child task".to_string(), ".".to_string());
    child.labels = vec!["task".to_string(), "frontend".to_string()];
    child.description = Some("Implement the feature".to_string());
    child.assignee = Some("developer".to_string());

    ws1.create_issue(&parent).expect("Failed to create parent");
    ws1.create_issue(&child).expect("Failed to create child");

    let storage = ws1.storage().expect("Failed to open storage");

    // Add dependency (child depends on parent)
    storage
        .add_dependency("bf-child", "bf-parent", &DependencyType::ParentChild, "test")
        .expect("Failed to add dependency");

    // Add comment to child
    storage
        .add_comment("bf-child", "pm", "Please prioritize this")
        .expect("Failed to add comment");

    // Export and import
    let export_count = ws1.export_jsonl(false).expect("Failed to export");
    assert_eq!(export_count, 2);

    let exported_jsonl = fs::read_to_string(&ws1.jsonl_path).expect("Failed to read exported JSONL");
    let ws2 = common::TempWorkspace::new().expect("Failed to create workspace 2");
    fs::write(&ws2.jsonl_path, &exported_jsonl).expect("Failed to write JSONL");

    ws2.import_jsonl().expect("Failed to import");

    // Verify everything preserved
    let imported_beads = ws2.list_beads().expect("Failed to list beads");
    assert_eq!(imported_beads.len(), 2);

    let imported_parent = imported_beads.iter().find(|b| b.id == "bf-parent").unwrap();
    assert_eq!(imported_parent.title, "Parent epic");
    assert_eq!(imported_parent.issue_type, IssueType::Epic);
    assert_eq!(imported_parent.priority, Priority(0));
    assert_eq!(imported_parent.labels.len(), 2);
    assert!(imported_parent.labels.contains(&"epic".to_string()));

    let imported_child = imported_beads.iter().find(|b| b.id == "bf-child").unwrap();
    assert_eq!(imported_child.title, "Child task");
    assert_eq!(imported_child.description, Some("Implement the feature".to_string()));
    assert_eq!(imported_child.assignee, Some("developer".to_string()));
    assert_eq!(imported_child.labels.len(), 2);
    assert_eq!(imported_child.dependencies.len(), 1);
    assert_eq!(imported_child.dependencies[0].dep_type, DependencyType::ParentChild);
    assert_eq!(imported_child.comments.len(), 1);
    assert_eq!(imported_child.comments[0].author, "pm");
}

/// Test dirty export/import cycle: only modified beads are exported/imported.
#[test]
fn test_dirty_export_import_cycle_only_modifies_changed_beads() {
    let ws1 = common::TempWorkspace::new().expect("Failed to create workspace 1");

    // Create initial beads
    let bead1 = Issue::new("bf-unchanged".to_string(), "Unchanged bead".to_string(), ".".to_string());
    let bead2 = Issue::new("bf-modified".to_string(), "Original title".to_string(), ".".to_string());
    let bead3 = Issue::new("bf-new".to_string(), "New bead".to_string(), ".".to_string());

    ws1.create_issue(&bead1).expect("Failed to create bead1");
    ws1.create_issue(&bead2).expect("Failed to create bead2");

    // Initial full export
    let initial_export = ws1.export_jsonl(false).expect("Failed to export");
    assert_eq!(initial_export, 2);

    let initial_jsonl = fs::read_to_string(&ws1.jsonl_path).expect("Failed to read initial JSONL");

    // Now modify bead2 (it becomes dirty)
    let storage = ws1.storage().expect("Failed to open storage");

    let changes = IssueChanges {
        title: Some("Modified title".to_string()),
        actor: Some("test".to_string()),
        ..Default::default()
    };
    storage
        .update_issue("bf-modified", &changes)
        .expect("Failed to update bead2");

    // Add bead3 (also dirty)
    ws1.create_issue(&bead3).expect("Failed to create bead3");

    // Dirty export: only modified and new beads
    let dirty_export_count = ws1.export_jsonl(true).expect("Failed to dirty export");
    assert_eq!(dirty_export_count, 2, "Should export 2 dirty beads (modified + new)");

    // Verify JSONL still contains all 3 beads
    let final_jsonl = fs::read_to_string(&ws1.jsonl_path).expect("Failed to read final JSONL");
    let lines: Vec<&str> = final_jsonl.lines().collect();
    assert_eq!(lines.len(), 3, "Final JSONL should have all 3 beads");

    // Import into new workspace
    let ws2 = common::TempWorkspace::new().expect("Failed to create workspace 2");
    fs::write(&ws2.jsonl_path, &final_jsonl).expect("Failed to write JSONL");

    let import_result = ws2.import_jsonl().expect("Failed to import");
    assert_eq!(import_result.imported, 3, "Should import all 3 beads");

    // Verify modifications preserved
    let imported_beads = ws2.list_beads().expect("Failed to list beads");
    let imported_modified = imported_beads.iter().find(|b| b.id == "bf-modified").unwrap();
    assert_eq!(imported_modified.title, "Modified title", "Modified title should be preserved");
}

/// Test SQLite state verification after round-trip.
#[test]
fn test_sqlite_state_matches_after_roundtrip() {
    let ws1 = common::TempWorkspace::new().expect("Failed to create workspace 1");

    // Create a bead with all fields populated
    let mut bead = Issue::new("bf-full".to_string(), "Fully populated bead".to_string(), "test-repo".to_string());
    bead.description = Some("Detailed description".to_string());
    bead.design = Some("Technical design".to_string());
    bead.acceptance_criteria = Some("Criteria satisfied".to_string());
    bead.notes = Some("Additional notes".to_string());
    bead.status = Status::InProgress;
    bead.priority = Priority(1);
    bead.issue_type = IssueType::Feature;
    bead.assignee = Some("developer".to_string());
    bead.owner = Some("team-lead".to_string());
    bead.estimated_minutes = Some(120);
    bead.created_by = Some("creator".to_string());
    bead.due_at = Some(Utc::now() + Duration::days(7));
    bead.external_ref = Some("JIRA-123".to_string());
    bead.source_system = Some("jira".to_string());
    bead.labels = vec!["feature".to_string(), "backend".to_string()];

    ws1.create_issue(&bead).expect("Failed to create bead");

    // Get the bead from storage to capture all fields as stored
    let stored_bead = ws1.get_bead("bf-full").expect("Failed to get stored bead").unwrap();

    // Export and import
    let export_count = ws1.export_jsonl(false).expect("Failed to export");
    assert_eq!(export_count, 1);

    let exported_jsonl = fs::read_to_string(&ws1.jsonl_path).expect("Failed to read exported JSONL");
    let ws2 = common::TempWorkspace::new().expect("Failed to create workspace 2");
    fs::write(&ws2.jsonl_path, &exported_jsonl).expect("Failed to write JSONL");

    ws2.import_jsonl().expect("Failed to import");

    // Verify SQLite state matches using sync_equals
    let imported_beads = ws2.list_beads().expect("Failed to list beads");
    assert_eq!(imported_beads.len(), 1);

    let imported = &imported_beads[0];

    // Use sync_equals which handles all sync-relevant fields
    assert!(stored_bead.sync_equals(imported), "Imported bead should match stored bead (sync_equals)");

    // Also verify individual critical fields
    assert_eq!(imported.id, stored_bead.id);
    assert_eq!(imported.title, stored_bead.title);
    assert_eq!(imported.description, stored_bead.description);
    assert_eq!(imported.design, stored_bead.design);
    assert_eq!(imported.acceptance_criteria, stored_bead.acceptance_criteria);
    assert_eq!(imported.notes, stored_bead.notes);
    assert_eq!(imported.status, stored_bead.status);
    assert_eq!(imported.priority, stored_bead.priority);
    assert_eq!(imported.issue_type, stored_bead.issue_type);
    assert_eq!(imported.assignee, stored_bead.assignee);
    assert_eq!(imported.owner, stored_bead.owner);
    assert_eq!(imported.estimated_minutes, stored_bead.estimated_minutes);
    assert_eq!(imported.created_by, stored_bead.created_by);
    assert_eq!(imported.due_at, stored_bead.due_at);
    assert_eq!(imported.external_ref, stored_bead.external_ref);
    assert_eq!(imported.source_system, stored_bead.source_system);
    assert_eq!(imported.source_repo, stored_bead.source_repo);
    assert_eq!(imported.labels, stored_bead.labels);
}

/// Test round-trip with empty workspace.
#[test]
fn test_roundtrip_with_empty_workspace() {
    let ws1 = common::TempWorkspace::new().expect("Failed to create workspace");

    // Export empty workspace
    let export_count = ws1.export_jsonl(false).expect("Failed to export empty");
    assert_eq!(export_count, 0);

    let exported_jsonl = fs::read_to_string(&ws1.jsonl_path).expect("Failed to read exported JSONL");

    // Should create empty file
    let ws2 = common::TempWorkspace::new().expect("Failed to create workspace 2");
    fs::write(&ws2.jsonl_path, &exported_jsonl).expect("Failed to write JSONL");

    let import_result = ws2.import_jsonl().expect("Failed to import empty");
    assert_eq!(import_result.imported, 0);
    assert_eq!(import_result.updated, 0);
    assert_eq!(import_result.skipped, 0);

    let imported_beads = ws2.list_beads().expect("Failed to list beads");
    assert_eq!(imported_beads.len(), 0);
}

/// Test round-trip preserves all timestamps correctly.
#[test]
fn test_roundtrip_preserves_timestamps() {
    let ws1 = common::TempWorkspace::new().expect("Failed to create workspace 1");

    // Create bead with specific timestamps
    let created_at = Utc::now() - Duration::hours(24);
    let updated_at = Utc::now() - Duration::hours(12);
    let closed_at = Utc::now() - Duration::hours(6);

    let mut bead = Issue::new("bf-times".to_string(), "Timestamp test".to_string(), ".".to_string());
    bead.created_at = created_at;
    bead.updated_at = updated_at;
    bead.closed_at = Some(closed_at);
    bead.status = Status::Closed;
    bead.close_reason = Some("Completed".to_string());

    ws1.create_issue(&bead).expect("Failed to create bead");

    // Export and import
    let export_count = ws1.export_jsonl(false).expect("Failed to export");
    assert_eq!(export_count, 1);

    let exported_jsonl = fs::read_to_string(&ws1.jsonl_path).expect("Failed to read exported JSONL");
    let ws2 = common::TempWorkspace::new().expect("Failed to create workspace 2");
    fs::write(&ws2.jsonl_path, &exported_jsonl).expect("Failed to write JSONL");

    ws2.import_jsonl().expect("Failed to import");

    // Verify timestamps preserved (with microsecond precision tolerance)
    let imported_beads = ws2.list_beads().expect("Failed to list beads");
    assert_eq!(imported_beads.len(), 1);

    let imported = &imported_beads[0];
    let time_diff = imported.created_at - created_at;
    assert!(time_diff.num_seconds().abs() < 1, "created_at should be preserved");

    let time_diff = imported.updated_at - updated_at;
    assert!(time_diff.num_seconds().abs() < 1, "updated_at should be preserved");

    assert!(imported.closed_at.is_some(), "closed_at should be preserved");
    let time_diff = imported.closed_at.unwrap() - closed_at;
    assert!(time_diff.num_seconds().abs() < 1, "closed_at should be preserved");

    assert_eq!(imported.close_reason, Some("Completed".to_string()));
}

/// Test round-trip with multiple export/import cycles.
#[test]
fn test_multiple_export_import_cycles_stable() {
    let ws1 = common::TempWorkspace::new().expect("Failed to create workspace 1");

    // Create beads
    let bead1 = Issue::new("bf-1".to_string(), "Bead 1".to_string(), ".".to_string());
    let bead2 = Issue::new("bf-2".to_string(), "Bead 2".to_string(), ".".to_string());

    ws1.create_issue(&bead1).expect("Failed to create bead1");
    ws1.create_issue(&bead2).expect("Failed to create bead2");

    // First export/import cycle
    let export_count = ws1.export_jsonl(false).expect("Failed to export cycle 1");
    assert_eq!(export_count, 2);

    let jsonl1 = fs::read_to_string(&ws1.jsonl_path).expect("Failed to read JSONL cycle 1");

    let ws2 = common::TempWorkspace::new().expect("Failed to create workspace 2");
    fs::write(&ws2.jsonl_path, &jsonl1).expect("Failed to write JSONL cycle 1");
    ws2.import_jsonl().expect("Failed to import cycle 1");

    // Second export/import cycle (from ws2)
    let export_count = ws2.export_jsonl(false).expect("Failed to export cycle 2");
    assert_eq!(export_count, 2);

    let jsonl2 = fs::read_to_string(&ws2.jsonl_path).expect("Failed to read JSONL cycle 2");

    let ws3 = common::TempWorkspace::new().expect("Failed to create workspace 3");
    fs::write(&ws3.jsonl_path, &jsonl2).expect("Failed to write JSONL cycle 2");
    ws3.import_jsonl().expect("Failed to import cycle 2");

    // Verify JSONL content is stable (same after multiple cycles)
    assert_eq!(jsonl1, jsonl2, "JSONL should be stable across export/import cycles");

    // Verify all workspaces have the same beads
    let beads1 = ws1.list_beads().expect("Failed to list ws1");
    let beads2 = ws2.list_beads().expect("Failed to list ws2");
    let beads3 = ws3.list_beads().expect("Failed to list ws3");

    assert_eq!(beads1.len(), beads2.len());
    assert_eq!(beads2.len(), beads3.len());
    assert_eq!(beads1.len(), 2);

    // Verify beads are equivalent across workspaces
    for bead in &beads1 {
        let ws2_bead = beads2.iter().find(|b| b.id == bead.id).unwrap();
        let ws3_bead = beads3.iter().find(|b| b.id == bead.id).unwrap();

        assert!(bead.sync_equals(ws2_bead), "ws1 and ws2 beads should match");
        assert!(bead.sync_equals(ws3_bead), "ws1 and ws3 beads should match");
    }
}
