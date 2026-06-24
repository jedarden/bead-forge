//! Test for bf-2hqt: count_unflushed over-reports after doctor --repair / sync --import-only cycle

mod common;

use bead_forge::doctor;
use std::fs;

/// Test the exact scenario from bf-2hqt description:
/// 1. Have a workspace with a recently flushed issues.jsonl
/// 2. Run `bf doctor --repair` (or `bf sync --import-only`)
/// 3. Run `bf status` or `bf sync --flush-only`
/// 4. Observe count_unflushed > 0 even though no mutations occurred since last flush
#[test]
fn test_count_unflushed_after_repair_cycle() {
    let ws = common::TempWorkspace::new().expect("Failed to create workspace");

    // Step 1: Create a bead and flush to JSONL
    ws.create_bead("bf-001", "Test bead").expect("Failed to create bead");
    ws.export_jsonl(false).expect("Failed to export to JSONL");

    // Verify everything is clean
    let doctor_result = doctor::check(ws.workspace_path())
        .expect("Doctor check should succeed");
    assert_eq!(
        doctor_result.unflushed_count, 0,
        "Should start with 0 unflushed beads"
    );

    // Step 2: Run doctor --repair (rebuilds db FROM JSONL)
    let imported = doctor::repair(ws.workspace_path(), false, false)
        .expect("Repair should succeed");
    assert_eq!(imported, 1, "Should import 1 bead from JSONL");

    // Step 3 & 4: Check that count_unflushed is still 0
    // (no mutations occurred since last flush)
    let doctor_result_after = doctor::check(ws.workspace_path())
        .expect("Doctor check should succeed after repair");
    assert_eq!(
        doctor_result_after.unflushed_count, 0,
        "After repair with no intervening mutations, count_unflushed should be 0"
    );

    // Additional check: run sync --flush-only (should do nothing, no beads to flush)
    let flushed = bead_forge::sync::flush(ws.workspace_path())
        .expect("Flush should succeed");
    assert_eq!(flushed, 1, "Should flush 1 bead");

    // And still count_unflushed should be 0
    let doctor_result_final = doctor::check(ws.workspace_path())
        .expect("Doctor check should succeed after flush");
    assert_eq!(
        doctor_result_final.unflushed_count, 0,
        "After flush cycle, count_unflushed should still be 0"
    );
}

/// Test the same scenario with sync --import-only instead of repair
#[test]
fn test_count_unflushed_after_import_cycle() {
    let ws = common::TempWorkspace::new().expect("Failed to create workspace");

    // Step 1: Create a bead and flush to JSONL
    ws.create_bead("bf-001", "Test bead").expect("Failed to create bead");
    ws.export_jsonl(false).expect("Failed to export to JSONL");

    // Verify everything is clean
    let doctor_result = doctor::check(ws.workspace_path())
        .expect("Doctor check should succeed");
    assert_eq!(
        doctor_result.unflushed_count, 0,
        "Should start with 0 unflushed beads"
    );

    // Step 2: Run sync --import-only (imports FROM JSONL)
    let sync_result = bead_forge::sync::import(ws.workspace_path())
        .expect("Import should succeed");
    assert_eq!(sync_result.imported, 0, "Should import 0 new beads (already exists)");
    assert_eq!(sync_result.skipped, 1, "Should skip 1 unchanged bead");

    // Step 3 & 4: Check that count_unflushed is still 0
    // (no mutations occurred since last flush)
    let doctor_result_after = doctor::check(ws.workspace_path())
        .expect("Doctor check should succeed after import");
    assert_eq!(
        doctor_result_after.unflushed_count, 0,
        "After import with no intervening mutations, count_unflushed should be 0"
    );

    // Additional check: run sync --flush-only (should do nothing, no beads to flush)
    let flushed = bead_forge::sync::flush(ws.workspace_path())
        .expect("Flush should succeed");
    assert_eq!(flushed, 1, "Should flush 1 bead");

    // And still count_unflushed should be 0
    let doctor_result_final = doctor::check(ws.workspace_path())
        .expect("Doctor check should succeed after flush");
    assert_eq!(
        doctor_result_final.unflushed_count, 0,
        "After flush cycle, count_unflushed should still be 0"
    );
}

/// Test the edge case where we have modifications before import
#[test]
fn test_count_unflushed_with_modifications_before_import() {
    let ws = common::TempWorkspace::new().expect("Failed to create workspace");

    // Create initial bead and flush to JSONL
    ws.create_bead("bf-001", "Original title").expect("Failed to create bead");
    ws.export_jsonl(false).expect("Failed to export to JSONL");

    // Now make a modification (marking it as dirty)
    let storage = ws.storage().expect("Failed to open storage");
    let changes = bead_forge::IssueChanges {
        title: Some("Modified title".to_string()),
        ..Default::default()
    };
    storage
        .update_issue("bf-001", &changes)
        .expect("Failed to update bead");

    // Verify we have 1 unflushed bead
    let doctor_result_before = doctor::check(ws.workspace_path())
        .expect("Doctor check should succeed");
    assert_eq!(
        doctor_result_before.unflushed_count, 1,
        "Should have 1 unflushed bead after modification"
    );

    // Now run import (simulating pulling updated JSONL from git)
    // Since the JSONL has the old version and we have a newer version in SQLite,
    // the SQLite version should win and the bead should remain marked as dirty
    let sync_result = bead_forge::sync::import(ws.workspace_path())
        .expect("Import should succeed");

    // After import, the dirty marks should be cleared because we just synced from JSONL
    let doctor_result_after = doctor::check(ws.workspace_path())
        .expect("Doctor check should succeed after import");
    assert_eq!(
        doctor_result_after.unflushed_count, 0,
        "After import, all dirty marks should be cleared (synced from JSONL)"
    );
}
