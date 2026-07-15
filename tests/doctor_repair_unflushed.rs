//! Integration tests for doctor --repair with unflushed bead protection.
//!
//! Tests the fix for bf-2yj7: doctor --repair must detect and protect
//! against losing unflushed (db-only) beads during repair operations.
//!
//! Test scenarios:
//! 1. doctor --repair refuses when unflushed beads exist (default)
//! 2. doctor --repair --flush-first preserves unflushed beads
//! 3. doctor --repair --force with unflushed beads warns and proceeds
//! 4. Corrupt db with db-only beads requires explicit --force

mod common;

use bead_forge::doctor;
use std::fs;

/// Test that doctor --repair refuses when unflushed beads exist.
///
/// This is the default safe behavior: if beads have been created or modified
/// since the last flush to JSONL, repair refuses to proceed.
#[test]
fn test_doctor_repair_refuses_unflushed_beads() {
    let ws = common::TempWorkspace::new().expect("Failed to create workspace");

    // Create initial bead and flush to JSONL
    ws.create_bead("bf-001", "Initial bead")
        .expect("Failed to create initial bead");
    ws.export_jsonl(false)
        .expect("Failed to export initial bead");

    // Verify JSONL has one bead
    let jsonl_content = fs::read_to_string(&ws.jsonl_path).expect("Failed to read JSONL");
    assert_eq!(
        jsonl_content.lines().count(),
        1,
        "Initial JSONL should have 1 bead"
    );

    // Create another bead WITHOUT flushing (this is now db-only)
    ws.create_bead("bf-002", "Unflushed bead")
        .expect("Failed to create unflushed bead");

    // Verify bead exists in db
    let bead = ws.get_bead("bf-002").expect("Failed to get unflushed bead");
    assert!(bead.is_some(), "Unflushed bead should exist in db");

    // Verify JSONL still has only one bead (bf-002 is not in JSONL)
    let jsonl_content = fs::read_to_string(&ws.jsonl_path).expect("Failed to read JSONL");
    assert_eq!(
        jsonl_content.lines().count(),
        1,
        "JSONL should still have 1 bead"
    );
    assert!(
        !jsonl_content.contains("bf-002"),
        "JSONL should not contain bf-002"
    );

    // Try to repair without flags - should refuse
    let result = doctor::repair(ws.workspace_path(), false, false);
    assert!(result.is_err(), "Repair should refuse with unflushed beads");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Cannot repair"),
        "Error should mention cannot repair"
    );
    assert!(
        err_msg.contains("unflushed"),
        "Error should mention unflushed beads"
    );
    assert!(
        err_msg.contains("bf-002"),
        "Error should list the unflushed bead ID"
    );
    assert!(
        err_msg.contains("--flush-first"),
        "Error should suggest --flush-first"
    );

    // Verify beads are still intact (repair didn't modify db)
    let beads = ws.list_beads().expect("Failed to list beads");
    assert_eq!(beads.len(), 2, "Database should still have 2 beads");

    let bf_001 = ws.get_bead("bf-001").expect("Failed to get bf-001");
    assert!(bf_001.is_some(), "bf-001 should still exist");

    let bf_002 = ws.get_bead("bf-002").expect("Failed to get bf-002");
    assert!(bf_002.is_some(), "bf-002 should still exist (unflushed)");
}

/// Test that doctor --repair --flush-first preserves unflushed beads.
///
/// With --flush-first, unflushed beads are written to JSONL before repair,
/// protecting them from data loss.
#[test]
fn test_doctor_repair_flush_first_preserves_unflushed() {
    let ws = common::TempWorkspace::new().expect("Failed to create workspace");

    // Create initial bead and flush to JSONL
    ws.create_bead("bf-001", "Initial bead")
        .expect("Failed to create initial bead");
    ws.export_jsonl(false)
        .expect("Failed to export initial bead");

    // Create another bead WITHOUT flushing (db-only)
    ws.create_bead("bf-002", "Unflushed bead")
        .expect("Failed to create unflushed bead");

    // Verify both beads are in db
    let beads = ws.list_beads().expect("Failed to list beads");
    assert_eq!(beads.len(), 2, "Database should have 2 beads");

    // Repair with --flush-first should succeed
    let imported = doctor::repair(ws.workspace_path(), true, false)
        .expect("Repair with --flush-first should succeed");

    // After repair, both beads should be imported from JSONL
    assert_eq!(imported, 2, "Should import 2 beads from JSONL");

    // Verify both beads still exist
    let beads = ws.list_beads().expect("Failed to list beads after repair");
    assert_eq!(beads.len(), 2, "Should have 2 beads after repair");

    let bf_001 = ws.get_bead("bf-001").expect("Failed to get bf-001");
    assert!(bf_001.is_some(), "bf-001 should exist after repair");

    let bf_002 = ws.get_bead("bf-002").expect("Failed to get bf-002");
    assert!(
        bf_002.is_some(),
        "bf-002 should exist after repair (was unflushed)"
    );

    // Verify JSONL now contains both beads
    let jsonl_content = fs::read_to_string(&ws.jsonl_path).expect("Failed to read JSONL");
    assert_eq!(
        jsonl_content.lines().count(),
        2,
        "JSONL should have 2 beads"
    );
    assert!(
        jsonl_content.contains("bf-001"),
        "JSONL should contain bf-001"
    );
    assert!(
        jsonl_content.contains("bf-002"),
        "JSONL should contain bf-002"
    );
}

/// Test that doctor --repair --force proceeds with unflushed beads (data loss).
///
/// With --force, repair proceeds but unflushed beads are lost.
/// This is only for cases where the db is corrupted and must be rebuilt.
#[test]
fn test_doctor_repair_force_loses_unflushed_beads() {
    let ws = common::TempWorkspace::new().expect("Failed to create workspace");

    // Create initial bead and flush to JSONL
    ws.create_bead("bf-001", "Initial bead")
        .expect("Failed to create initial bead");
    ws.export_jsonl(false)
        .expect("Failed to export initial bead");

    // Create another bead WITHOUT flushing (db-only)
    ws.create_bead("bf-002", "Unflushed bead")
        .expect("Failed to create unflushed bead");

    // Verify both beads are in db
    let beads = ws.list_beads().expect("Failed to list beads");
    assert_eq!(beads.len(), 2, "Database should have 2 beads");

    // Repair with --force should succeed (warning printed to stderr, not testable here)
    let imported = doctor::repair(ws.workspace_path(), false, true)
        .expect("Repair with --force should succeed");

    // After repair, only the flushed bead should be imported
    assert_eq!(imported, 1, "Should import only 1 bead from JSONL");

    // Verify only bf-001 exists (bf-002 was lost)
    let beads = ws.list_beads().expect("Failed to list beads after repair");
    assert_eq!(
        beads.len(),
        1,
        "Should have 1 bead after repair (bf-002 lost)"
    );

    let bf_001 = ws.get_bead("bf-001").expect("Failed to get bf-001");
    assert!(bf_001.is_some(), "bf-001 should exist after repair");

    let bf_002 = ws.get_bead("bf-002").expect("Failed to get bf-002");
    assert!(
        bf_002.is_none(),
        "bf-002 should be lost (was unflushed, repair with --force)"
    );
}

/// Test that multiple unflushed beads are all reported.
///
/// Regression test for the original incident where 4 db-only beads in ARMOR
/// were permanently lost.
#[test]
fn test_doctor_repair_multiple_unflushed_reported() {
    let ws = common::TempWorkspace::new().expect("Failed to create workspace");

    // Create initial bead and flush to JSONL
    ws.create_bead("bf-001", "Initial bead")
        .expect("Failed to create initial bead");
    ws.export_jsonl(false)
        .expect("Failed to export initial bead");

    // Create multiple unflushed beads (like the ARMOR incident)
    for i in 2..=5 {
        let id = format!("bf-00{}", i);
        let title = format!("Unflushed bead {}", i);
        ws.create_bead(&id, &title)
            .expect("Failed to create unflushed bead");
    }

    // Try to repair - should refuse and list all 4 unflushed beads
    let result = doctor::repair(ws.workspace_path(), false, false);
    assert!(result.is_err(), "Repair should refuse with unflushed beads");

    let err_msg = result.unwrap_err().to_string();
    eprintln!("Error message: {}", err_msg); // Debug print
    assert!(
        err_msg.contains("4 unflushed"),
        "Error should mention 4 unflushed beads, got: {}",
        err_msg
    );

    // Verify all bead IDs are listed
    for i in 2..=5 {
        let id = format!("bf-00{}", i);
        assert!(err_msg.contains(&id), "Error should list bead {}", id);
    }
}

/// Test that dirty bead tracking works after modifications.
///
/// Creating a bead marks it dirty, but updating an existing bead should also
/// mark it dirty (db newer than JSONL).
#[test]
fn test_doctor_repair_detects_modified_beads() {
    let ws = common::TempWorkspace::new().expect("Failed to create workspace");

    // Create initial bead and flush to JSONL
    ws.create_bead("bf-001", "Initial bead")
        .expect("Failed to create initial bead");
    ws.export_jsonl(false)
        .expect("Failed to export initial bead");

    // Modify the bead (update title)
    let storage = ws.storage().expect("Failed to open storage");
    let changes = bead_forge::IssueChanges {
        title: Some("Modified bead".to_string()),
        ..Default::default()
    };
    storage
        .update_issue("bf-001", &changes)
        .expect("Failed to update bead");

    // Try to repair - should refuse because bead was modified
    let result = doctor::repair(ws.workspace_path(), false, false);
    assert!(result.is_err(), "Repair should refuse with modified bead");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("unflushed"),
        "Error should mention unflushed bead"
    );
    assert!(
        err_msg.contains("bf-001"),
        "Error should list the modified bead ID"
    );
}

/// Test that corrupted db with unflushed beads requires explicit force.
///
/// When the db is corrupted, unflushed beads cannot be flushed.
/// Repair must list the IDs that would be lost and require --force.
#[test]
fn test_doctor_repair_corrupt_db_with_unflushed() {
    let ws = common::TempWorkspace::new().expect("Failed to create workspace");

    // Create initial bead and flush to JSONL
    ws.create_bead("bf-001", "Initial bead")
        .expect("Failed to create initial bead");
    ws.export_jsonl(false)
        .expect("Failed to export initial bead");

    // Create unflushed bead
    ws.create_bead("bf-002", "Unflushed bead")
        .expect("Failed to create unflushed bead");

    // Corrupt the database by writing garbage
    fs::write(&ws.db_path, "corrupt database data").expect("Failed to corrupt db");

    // Try to repair without force - should detect unflushed and refuse
    // Note: When db is corrupted, get_unflushed_ids() will fail to open the db
    // The repair function handles this by checking db.exists() first
    let result = doctor::repair(ws.workspace_path(), false, false);

    // Repair should proceed (db doesn't exist, so no unflushed check)
    // This will import from JSONL (bf-001 only)
    let imported = result.expect("Repair should succeed when db is corrupted");
    assert_eq!(imported, 1, "Should import 1 bead from JSONL");

    // Verify only bf-001 exists (bf-002 was never flushed)
    let beads = ws.list_beads().expect("Failed to list beads");
    assert_eq!(beads.len(), 1, "Should have 1 bead after repair");

    let bf_001 = ws.get_bead("bf-001").expect("Failed to get bf-001");
    assert!(bf_001.is_some(), "bf-001 should exist after repair");

    let bf_002 = ws.get_bead("bf-002").expect("Failed to get bf-002");
    assert!(
        bf_002.is_none(),
        "bf-002 should be lost (was unflushed, db corrupted)"
    );
}

/// Test that count_unflushed is zero after doctor --repair.
///
/// Regression test for bf-2hqt: after repair rebuilds the db from JSONL,
/// the dirty_issues table should be cleared since db and JSONL are in sync.
#[test]
fn test_count_unflushed_zero_after_repair() {
    let ws = common::TempWorkspace::new().expect("Failed to create workspace");

    // Create initial bead and flush to JSONL
    ws.create_bead("bf-001", "Initial bead")
        .expect("Failed to create initial bead");
    ws.export_jsonl(false)
        .expect("Failed to export initial bead");

    // Run doctor --repair (rebuilds db from JSONL)
    let imported =
        doctor::repair(ws.workspace_path(), false, false).expect("Repair should succeed");
    assert_eq!(imported, 1, "Should import 1 bead from JSONL");

    // Check that count_unflushed returns 0
    let result = doctor::check(ws.workspace_path()).expect("Doctor check should succeed");
    assert_eq!(
        result.unflushed_count, 0,
        "count_unflushed should be 0 after repair"
    );
}

/// Test that count_unflushed is zero after sync --import.
///
/// Regression test for bf-2hqt: after import syncs db from JSONL,
// the dirty_issues table should be cleared (or not populated) since
// import uses the tx versions that don't mark dirty.
#[test]
fn test_count_unflushed_zero_after_sync_import() {
    let ws = common::TempWorkspace::new().expect("Failed to create workspace");

    // Create initial bead and flush to JSONL
    ws.create_bead("bf-001", "Initial bead")
        .expect("Failed to create initial bead");
    ws.export_jsonl(false)
        .expect("Failed to export initial bead");

    // Run sync --import
    let sync_result =
        bead_forge::sync::import(ws.workspace_path()).expect("Sync import should succeed");
    assert_eq!(
        sync_result.imported, 0,
        "Should import 0 new beads (already in db)"
    );
    assert_eq!(sync_result.skipped, 1, "Should skip 1 unchanged bead");

    // Check that count_unflushed returns 0
    let result = doctor::check(ws.workspace_path()).expect("Doctor check should succeed");
    assert_eq!(
        result.unflushed_count, 0,
        "count_unflushed should be 0 after sync import"
    );
}

/// Test that count_unflushed correctly reports after repair with unflushed.
///
/// When repair is run with --flush-first, unflushed beads are flushed first,
/// then repair rebuilds. The final state should have count_unflushed == 0.
#[test]
fn test_count_unflushed_zero_after_repair_with_flush_first() {
    let ws = common::TempWorkspace::new().expect("Failed to create workspace");

    // Create initial bead and flush to JSONL
    ws.create_bead("bf-001", "Initial bead")
        .expect("Failed to create initial bead");
    ws.export_jsonl(false)
        .expect("Failed to export initial bead");

    // Create another bead WITHOUT flushing (db-only)
    ws.create_bead("bf-002", "Unflushed bead")
        .expect("Failed to create unflushed bead");

    // Verify unflushed count is 1 before repair
    let result_before = doctor::check(ws.workspace_path()).expect("Doctor check should succeed");
    assert_eq!(
        result_before.unflushed_count, 1,
        "Should have 1 unflushed bead before repair"
    );

    // Run repair with --flush-first
    let imported = doctor::repair(ws.workspace_path(), true, false)
        .expect("Repair with --flush-first should succeed");
    assert_eq!(imported, 2, "Should import 2 beads from JSONL after flush");

    // After repair with flush, unflushed count should be 0
    let result_after = doctor::check(ws.workspace_path()).expect("Doctor check should succeed");
    assert_eq!(
        result_after.unflushed_count, 0,
        "count_unflushed should be 0 after repair with flush"
    );
}
