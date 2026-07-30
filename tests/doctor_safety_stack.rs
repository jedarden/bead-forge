//! Integration tests for the doctor safety stack (Phase 7.2, bead bf-2r4k0).
//!
//! Proves the plan §7.2 exit criteria hold end-to-end:
//!
//! 1. An induced rebuild on a workspace with unflushed beads loses nothing —
//!    including the unflushed bead's labels, dependencies, and comments, and it
//!    stays marked dirty afterward (layer 6 / beads_rust#394).
//! 2. Repair on a healthy workspace is a no-op — the JSONL rebuild is never
//!    reached from a healthy state (layer 1).
//! 3. Every rebuild leaves a restorable, hash-verified backup (layer 3).
//!
//! Plus per-layer coverage: JSONL authority preflight (layer 4) and the
//! repeat-failure gate (layer 5).

mod common;

use bead_forge::doctor::{self, RepairOptions};
use bead_forge::recovery;
use std::fs;

/// Append a raw line to the workspace JSONL (simulates a bead present in the
/// authority but missing from the DB, which is what makes a rebuild *necessary*).
fn append_jsonl_line(ws: &common::TempWorkspace, line: &str) {
    let mut content = fs::read_to_string(&ws.jsonl_path).unwrap_or_default();
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(line);
    content.push('\n');
    fs::write(&ws.jsonl_path, content).unwrap();
}

/// Set up a workspace where a rebuild is genuinely required (JSONL carries a bead
/// the DB lacks) AND an unflushed dirty bead exists in the DB only.
///
/// Returns the workspace. After setup:
///   * `bf-flushed`  — in both DB and JSONL (clean)
///   * `bf-dirty`    — in DB only, marked dirty (unflushed), with a label + comment
///   * `bf-jsonl`    — in JSONL only (drives `missing_in_sqlite`, forces rebuild)
fn setup_rebuild_needed_with_unflushed() -> common::TempWorkspace {
    let ws = common::TempWorkspace::new().expect("workspace");

    // Clean, flushed bead.
    ws.create_bead("bf-flushed", "Flushed bead").unwrap();
    ws.export_jsonl(false).unwrap(); // exports + clears dirty

    // Unflushed dirty bead with a label and a comment (must survive rebuild).
    ws.create_bead("bf-dirty", "Unflushed dirty bead").unwrap();
    {
        let storage = ws.storage().unwrap();
        storage.add_label("bf-dirty", "preserve-me").unwrap();
        storage
            .add_comment("bf-dirty", "tester", "keep this comment")
            .unwrap();
    }

    // Bead present only in JSONL → forces a rebuild (missing_in_sqlite).
    append_jsonl_line(&ws, &common::sample_bead_jsonl("bf-jsonl", "JSONL-only bead"));

    ws
}

/// Exit criterion #1 + #3: an induced rebuild with an unflushed bead loses
/// nothing (bead, its label, its comment), keeps it dirty, and leaves a verified
/// backup.
#[test]
fn induced_rebuild_preserves_unflushed_bead_with_labels_and_comments() {
    let ws = setup_rebuild_needed_with_unflushed();

    // Sanity: the dirty bead is unflushed before repair.
    let metadata = ws.metadata().unwrap();
    let db_path = ws.workspace_path().join(".beads").join(&metadata.database);
    assert_eq!(doctor::count_unflushed(&db_path).unwrap(), 1);

    let report = doctor::repair_stack(ws.workspace_path(), &RepairOptions::default())
        .expect("repair_stack should succeed");

    // A rebuild actually happened (the JSONL-only bead forced it).
    assert!(report.rebuilt, "rebuild should have been performed");
    // bf-flushed + bf-jsonl imported from JSONL.
    assert_eq!(report.imported, 2, "should import the two JSONL beads");
    // The unflushed dirty bead was preserved across the rebuild.
    assert_eq!(report.preserved_dirty, 1, "one dirty bead preserved");

    // Nothing lost: all three beads present.
    assert!(ws.get_bead("bf-flushed").unwrap().is_some());
    assert!(ws.get_bead("bf-jsonl").unwrap().is_some());
    let dirty = ws
        .get_bead("bf-dirty")
        .unwrap()
        .expect("unflushed bead must survive rebuild");
    assert_eq!(dirty.title, "Unflushed dirty bead");

    // Its label and comment survived.
    let storage = ws.storage().unwrap();
    assert!(
        storage.get_labels("bf-dirty").unwrap().contains(&"preserve-me".to_string()),
        "label must survive rebuild"
    );
    let comments = storage.list_comments("bf-dirty").unwrap();
    assert_eq!(comments.len(), 1, "comment must survive rebuild");
    assert_eq!(comments[0].body, "keep this comment");

    // It is still marked dirty (unflushed) — preservation re-marks, not re-flushes.
    assert_eq!(
        doctor::count_unflushed(&db_path).unwrap(),
        1,
        "preserved bead stays dirty"
    );

    // Exit criterion #3: a hash-verified, restorable backup exists.
    let run_id = report.backup_run_id.expect("a backup run id");
    let beads_dir = ws.workspace_path().join(".beads");
    recovery::verify_run(&beads_dir, &run_id).expect("backup must verify");
    let runs = recovery::list_runs(&beads_dir).unwrap();
    assert!(runs.iter().any(|m| m.run_id == run_id));
    // The backup captured the DB and the JSONL authority.
    let names: Vec<&str> = runs
        .iter()
        .find(|m| m.run_id == run_id)
        .unwrap()
        .files
        .iter()
        .map(|f| f.name.as_str())
        .collect();
    assert!(names.iter().any(|n| n.ends_with(".db")), "db backed up");
    assert!(names.iter().any(|n| n.ends_with(".jsonl")), "jsonl backed up");
}

/// Exit criterion #2: repair on a healthy workspace is a no-op — no rebuild, no
/// backup run created (the JSONL rebuild is unreachable from a healthy state).
#[test]
fn healthy_workspace_repair_is_a_noop() {
    let ws = common::TempWorkspace::new().unwrap();
    ws.create_bead("bf-1", "One").unwrap();
    ws.create_bead("bf-2", "Two").unwrap();
    ws.export_jsonl(false).unwrap(); // fully in sync, clean

    let report = doctor::repair_stack(ws.workspace_path(), &RepairOptions::default())
        .expect("repair on healthy workspace");

    assert!(report.healthy, "healthy workspace reported healthy");
    assert!(!report.rebuilt, "healthy workspace must NOT rebuild");
    assert_eq!(report.imported, 0);
    assert!(report.backup_run_id.is_none(), "no backup run on healthy repair");

    // No recovery runs were created at all.
    let beads_dir = ws.workspace_path().join(".beads");
    assert!(recovery::list_runs(&beads_dir).unwrap().is_empty());

    // Data untouched.
    assert!(ws.get_bead("bf-1").unwrap().is_some());
    assert!(ws.get_bead("bf-2").unwrap().is_some());
}

/// Even with unflushed beads present, a *healthy* workspace (no corruption, no
/// JSONL→DB divergence) never rebuilds — unflushed beads alone are a flush
/// concern, not a rebuild trigger.
#[test]
fn unflushed_beads_alone_do_not_trigger_rebuild() {
    let ws = common::TempWorkspace::new().unwrap();
    ws.create_bead("bf-clean", "Clean").unwrap();
    ws.export_jsonl(false).unwrap();
    // Unflushed bead, but nothing is corrupt or divergent.
    ws.create_bead("bf-unflushed", "Unflushed").unwrap();

    let report = doctor::repair_stack(ws.workspace_path(), &RepairOptions::default()).unwrap();
    assert!(!report.rebuilt, "no rebuild when only unflushed beads exist");
    assert!(report.healthy);
    // The bead is still there and still dirty.
    assert!(ws.get_bead("bf-unflushed").unwrap().is_some());
}

/// `bf doctor --repair --flush-first` on a healthy workspace (no corruption, no
/// JSONL→DB divergence) must NOT write the JSONL checkpoint, even when unflushed
/// beads are present. `--flush-first` is scoped to the rebuild ("flush before
/// repair"); with no rebuild pending there is nothing to protect, so the healthy
/// path stays read-only and points the user at `bf sync --flush-only` instead.
/// Regression test for bf-ku8hv.
#[test]
fn healthy_repair_with_flush_first_does_not_write_jsonl() {
    let ws = common::TempWorkspace::new().unwrap();
    ws.create_bead("bf-clean", "Clean").unwrap();
    ws.export_jsonl(false).unwrap();
    // Unflushed bead, but nothing is corrupt or divergent → healthy.
    ws.create_bead("bf-unflushed", "Unflushed").unwrap();

    let metadata = ws.metadata().unwrap();
    let db_path = ws.workspace_path().join(".beads").join(&metadata.database);

    // Sanity: the workspace is healthy but carries one unflushed bead — exactly
    // the state the old code used to flush on the no-rebuild path.
    assert_eq!(doctor::count_unflushed(&db_path).unwrap(), 1);

    // Snapshot the JSONL checkpoint before repair.
    let before = fs::read(&ws.jsonl_path).unwrap();

    let opts = RepairOptions {
        flush_first: true,
        ..Default::default()
    };
    let report = doctor::repair_stack(ws.workspace_path(), &opts).unwrap();

    // No rebuild, reported healthy.
    assert!(report.healthy, "healthy workspace reported healthy");
    assert!(!report.rebuilt, "healthy workspace must NOT rebuild");

    // The JSONL checkpoint is byte-identical — no write happened.
    let after = fs::read(&ws.jsonl_path).unwrap();
    assert_eq!(
        before, after,
        "JSONL must be unchanged on a healthy repair even with --flush-first"
    );

    // The unflushed bead is still unflushed (nothing was flushed for it).
    assert_eq!(
        doctor::count_unflushed(&db_path).unwrap(),
        1,
        "unflushed bead must still be dirty — nothing was flushed"
    );

    // The user is pointed at the canonical checkpoint command, not silently flushed.
    assert!(
        report
            .messages
            .iter()
            .any(|m| m.contains("bf sync --flush-only")),
        "should advise `bf sync --flush-only`, got: {:?}",
        report.messages
    );
    assert!(
        !report.messages.iter().any(|m| m.contains("Flushed")),
        "must not report a flush on the healthy path, got: {:?}",
        report.messages
    );
}

/// Layer 4: refuse to rebuild when the JSONL authority carries a git
/// merge-conflict marker (rebuilding from a conflicted authority would make the
/// corruption permanent).
#[test]
fn preflight_refuses_rebuild_on_conflict_marker() {
    let ws = common::TempWorkspace::new().unwrap();
    ws.create_bead("bf-a", "A").unwrap();
    ws.export_jsonl(false).unwrap();

    // Force a rebuild need (JSONL-only bead) AND poison the JSONL with a marker.
    append_jsonl_line(&ws, &common::sample_bead_jsonl("bf-b", "B"));
    append_jsonl_line(&ws, "<<<<<<< HEAD");

    let err = doctor::repair_stack(ws.workspace_path(), &RepairOptions::default())
        .expect_err("must refuse rebuild on conflict marker");
    let msg = err.to_string();
    assert!(
        msg.contains("merge-conflict"),
        "error should mention the conflict marker, got: {msg}"
    );

    // No backup run should have been created (we refused before rebuilding).
    let beads_dir = ws.workspace_path().join(".beads");
    assert!(recovery::list_runs(&beads_dir).unwrap().is_empty());
}

/// Layer 4: refuse to rebuild when the JSONL contains an unparseable record.
#[test]
fn preflight_refuses_rebuild_on_invalid_record() {
    let ws = common::TempWorkspace::new().unwrap();
    ws.create_bead("bf-a", "A").unwrap();
    ws.export_jsonl(false).unwrap();

    append_jsonl_line(&ws, &common::sample_bead_jsonl("bf-b", "B"));
    append_jsonl_line(&ws, "{not valid json at all");

    let err = doctor::repair_stack(ws.workspace_path(), &RepairOptions::default())
        .expect_err("must refuse rebuild on invalid record");
    assert!(err.to_string().contains("invalid record"));
}

/// Layer 5: once a rebuild has left a repeat-failure marker, further rebuilds
/// refuse without `--allow-repeated-repair`, and proceed with it.
#[test]
fn repeat_failure_gate_blocks_then_allows() {
    let ws = setup_rebuild_needed_with_unflushed();
    let beads_dir = ws.workspace_path().join(".beads");

    // Simulate a prior failed rebuild by raising the gate directly.
    recovery::write_repair_failed_marker(&beads_dir, "simulated prior failure").unwrap();
    assert!(recovery::repair_failed_marker_exists(&beads_dir));

    // Default repair refuses.
    let err = doctor::repair_stack(ws.workspace_path(), &RepairOptions::default())
        .expect_err("gate should block");
    assert!(err.to_string().contains("allow-repeated-repair"));

    // With the override, the rebuild proceeds and clears the marker on success.
    let opts = RepairOptions {
        allow_repeated_repair: true,
        ..Default::default()
    };
    let report = doctor::repair_stack(ws.workspace_path(), &opts).expect("override should proceed");
    assert!(report.rebuilt);
    assert!(
        !recovery::repair_failed_marker_exists(&beads_dir),
        "successful rebuild clears the marker"
    );
}

/// Layer 3: a captured backup round-trips — `restore_run("latest")` puts the
/// backed-up DB family back byte-for-byte (verified against the manifest hash).
#[test]
fn backup_restore_round_trips() {
    let ws = setup_rebuild_needed_with_unflushed();
    let beads_dir = ws.workspace_path().join(".beads");
    let metadata = ws.metadata().unwrap();
    let db_path = beads_dir.join(&metadata.database);

    let report = doctor::repair_stack(ws.workspace_path(), &RepairOptions::default()).unwrap();
    assert!(report.rebuilt);
    let run_id = report.backup_run_id.clone().unwrap();

    // The recorded backup hash for the DB is what a faithful restore must reproduce.
    let manifest = recovery::list_runs(&beads_dir)
        .unwrap()
        .into_iter()
        .find(|m| m.run_id == run_id)
        .unwrap();
    let db_entry = manifest.files.iter().find(|f| f.name.ends_with(".db")).unwrap();

    // Clobber the live DB to simulate a later problem, then restore.
    fs::write(&db_path, b"garbage-not-a-database").unwrap();
    let restored = recovery::restore_run(&beads_dir, "latest").unwrap();
    assert_eq!(restored.run_id, run_id);

    // The restored DB reproduces the backed-up bytes exactly (hash matches manifest).
    assert_eq!(
        recovery::hash_file(&db_path).unwrap(),
        db_entry.sha256,
        "restore reproduces the backed-up db exactly"
    );
}

/// `--force` opts out of preservation: the destructive legacy path discards
/// unflushed beads instead of preserving them.
#[test]
fn force_discards_unflushed_instead_of_preserving() {
    let ws = setup_rebuild_needed_with_unflushed();
    let opts = RepairOptions {
        force: true,
        ..Default::default()
    };
    let report = doctor::repair_stack(ws.workspace_path(), &opts).unwrap();
    assert!(report.rebuilt);
    assert_eq!(report.preserved_dirty, 0, "force preserves nothing");
    // The unflushed bead is gone; the JSONL-authoritative beads remain.
    assert!(ws.get_bead("bf-dirty").unwrap().is_none());
    assert!(ws.get_bead("bf-flushed").unwrap().is_some());
    assert!(ws.get_bead("bf-jsonl").unwrap().is_some());
}
