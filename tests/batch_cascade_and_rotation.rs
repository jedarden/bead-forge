//! Integration tests for Phase 7.1 child 4/5 (bead **bf-3a4hk**): the two
//! narrow guarantees the per-op wiring of child 2 (bf-2rjhk / bf-2cfqm) does
//! NOT cover.
//!
//! 1. **Batch close cascade (bf-5id blocked→open) is marked dirty inside the
//!    batch transaction and exported by the single end-of-transaction flush.**
//!    Sibling bf-2cfqm already proved a batch of `create` ops flushes exactly
//!    once (its `execute_batch_performs_no_jsonl_write` + `batch_flushes_all_ops`).
//!    This file closes the remaining audit named in bf-3a4hk: when a batch `close`
//!    cascades a dependent from blocked→open, BOTH the closed bead and the
//!    cascaded dependent must be dirty-marked together and exported in that same
//!    one flush — proving "all dirty marks from the batch are exported in one
//!    flush", cascade included.
//!
//! 2. **Rotation interplay (plan §7.1 Open Question — RESOLVED).** Incremental
//!    auto-flush writes ONLY to the active `issues.jsonl` named by
//!    `metadata.jsonl_export`, NEVER to a rotated archive (`issues.jsonl.1`).
//!    Sibling bf-bziwd pins this for the `bf update` mutation path; this file
//!    pins it for the **batch** flush path and asserts the archive is left
//!    byte-for-byte identical with an unchanged mtime.
//!
//! Both tests run in-process (calling the library directly) rather than via the
//! `bf` binary: that lets us observe the JSONL *between* `execute_batch` and the
//! flush — the precise proof that `execute_batch` writes JSONL zero times during
//! the transaction and the caller's single follow-up flush carries everything.

use std::path::Path;
use std::time::{Duration, SystemTime};

use bead_forge::config::init_workspace;
use bead_forge::model::{DependencyType, Issue, Status};
use bead_forge::rotate::{rotate, RotateOptions};
use bead_forge::storage::Storage;
use bead_forge::{autoflush, execute_batch, sync, BatchOp};
use chrono::Utc;

/// Fresh on-disk `.beads/` workspace (config + metadata + empty db).
fn init_ws() -> tempfile::TempDir {
    let tmp = tempfile::TempDir::new().unwrap();
    init_workspace(&tmp.path().join(".beads"), "bf").unwrap();
    tmp
}

fn jsonl(ws: &Path) -> std::path::PathBuf {
    ws.join(".beads").join("issues.jsonl")
}

/// Parse every non-blank line of a JSONL file into a JSON value.
fn parse_all(path: &Path) -> Vec<serde_json::Value> {
    if !path.exists() {
        return Vec::new();
    }
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn status_of<'a>(beads: &'a [serde_json::Value], id: &str) -> Option<&'a str> {
    beads
        .iter()
        .find(|b| b.get("id").and_then(|v| v.as_str()) == Some(id))
        .and_then(|b| b.get("status").and_then(|v| v.as_str()))
}

fn storage_for(ws: &Path) -> Storage {
    let beads_dir = ws.join(".beads");
    let metadata = bead_forge::config::load_metadata(&beads_dir).unwrap();
    Storage::open(&beads_dir.join(&metadata.database)).unwrap()
}

// ---------------------------------------------------------------------------
// 1. Batch close cascade (bf-5id blocked→open) is dirty-marked in-transaction
//    and exported by the single end-of-transaction flush.
// ---------------------------------------------------------------------------

/// `execute_batch`'s `close` op cascades a blocked→open transition on a
/// dependent (the bf-5id stuck-blocked class). That cascaded dependent is marked
/// dirty *inside the same transaction* as the close, so the caller's ONE
/// follow-up flush exports the closed blocker AND the dependent's new open
/// status together — with zero JSONL writes during the transaction itself.
#[test]
fn batch_close_cascade_marked_dirty_and_exported_in_single_flush() {
    let ws = init_ws();
    let storage = storage_for(ws.path());
    let jsonl_path = jsonl(ws.path());

    // Pre-seed the cascade precondition, mirroring NEEDLE safe-dispatch: a
    // dependent `bf-dep` fenced as status=blocked by an open blocker `bf-blk`.
    let mut dep = Issue::new("bf-dep".into(), "Dependent".into(), ".".into());
    dep.status = Status::Blocked;
    let mut blk = Issue::new("bf-blk".into(), "Blocker".into(), ".".into());
    blk.status = Status::Open;
    storage.create_issue(&dep).unwrap();
    storage.create_issue(&blk).unwrap();
    // bf-dep depends on bf-blk (bf-blk blocks bf-dep) — same direction
    // execute_dep_add_blocker / `bf dep add <blk> --blocks <dep>` produce.
    storage
        .add_dependency("bf-dep", "bf-blk", &DependencyType::Blocks, "test")
        .unwrap();
    // Full flush: lands both beads in JSONL and clears dirty marks.
    sync::flush(ws.path()).unwrap();
    assert_eq!(storage.list_dirty_issues().unwrap().len(), 0, "baseline must be clean");

    let baseline = std::fs::read(&jsonl_path).unwrap();
    assert_eq!(
        status_of(&parse_all(&jsonl_path), "bf-dep"),
        Some("blocked"),
        "dependent starts blocked"
    );

    // A single-op batch: close the blocker. Closing the sole non-terminal
    // blocker must cascade bf-dep blocked→open.
    let ops = vec![BatchOp::Close {
        id: "bf-blk".into(),
        reason: "done".into(),
    }];
    let results = execute_batch(&storage, ops, ws.path(), false /* enable auto-flush **/).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, "ok");

    // (1) execute_batch wrote SQLite only — JSONL is byte-identical to baseline.
    //     This is the "exactly one write" half: the transaction performs ZERO
    //     JSONL writes, so the only write is the caller's single follow-up flush.
    assert_eq!(
        std::fs::read(&jsonl_path).unwrap(),
        baseline,
        "execute_batch must not write JSONL during the transaction (no per-op flush)"
    );

    // (2) The cascade marked bf-dep dirty alongside bf-blk; ONE flush exports
    //     BOTH together. Dirty count is exactly 2 (closed blocker + cascaded
    //     dependent) — the single flush must carry every dirty mark the batch
    //     produced, cascade included.
    let n = autoflush::run(ws.path()).unwrap();
    assert_eq!(
        n, 2,
        "single flush must export the closed blocker AND the cascaded dependent"
    );

    // Final landed state: blocker closed, dependent cascaded to open — both
    // present in JSONL after that one flush.
    let after = parse_all(&jsonl_path);
    assert_eq!(status_of(&after, "bf-blk"), Some("closed"), "closed blocker exported");
    assert_eq!(
        status_of(&after, "bf-dep"),
        Some("open"),
        "cascaded dependent exported with open status (cascade dirty mark survived into the flush)"
    );

    // (3) Dirty marks cleared by the successful flush; a second flush is a
    //     no-op (no write amplification, no second pass).
    assert_eq!(
        storage.list_dirty_issues().unwrap().len(),
        0,
        "dirty marks cleared after the single flush"
    );
    let n2 = autoflush::run(ws.path()).unwrap();
    assert_eq!(n2, 0, "no second flush needed");
}

// ---------------------------------------------------------------------------
// 2. Rotation interplay: an incremental (batch) flush targets ONLY the active
//    issues.jsonl, never a rotated archive.
// ---------------------------------------------------------------------------

/// Snapshot (bytes, mtime) — the invariant every rotated archive must preserve
/// across an incremental auto-flush.
fn snapshot(path: &Path) -> (Vec<u8>, SystemTime) {
    let bytes = std::fs::read(path).unwrap();
    let mtime = std::fs::metadata(path).unwrap().modified().unwrap();
    (bytes, mtime)
}

/// After `rotate` archives an old closed bead into `issues.jsonl.1`, a batch's
/// single end-of-transaction flush must rewrite ONLY the active `issues.jsonl`
/// — the archive stays byte-for-byte identical with an unchanged mtime, and the
/// active file gains the batched bead without pulling archived content back in.
#[test]
fn incremental_flush_targets_only_active_jsonl_not_archive() {
    let ws = init_ws();
    let storage = storage_for(ws.path());
    let active = jsonl(ws.path());
    let archive = ws.path().join(".beads").join("issues.jsonl.1");

    // Seed an active bead and an old-closed bead, then flush so both land in the
    // active JSONL and dirty is cleared.
    let mut keep = Issue::new("bf-live".into(), "Stays active".into(), ".".into());
    keep.status = Status::Open;
    let mut old = Issue::new("bf-old".into(), "Rotates out".into(), ".".into());
    old.status = Status::Closed;
    old.closed_at = Some(Utc::now() - chrono::Duration::days(60));
    storage.create_issue(&keep).unwrap();
    storage.create_issue(&old).unwrap();
    sync::flush(ws.path()).unwrap();
    let initial = parse_all(&active);
    assert!(initial.iter().any(|b| b["id"] == "bf-live"));
    assert!(initial.iter().any(|b| b["id"] == "bf-old"));

    // Rotate: bf-old (closed >30d) moves to issues.jsonl.1; active keeps bf-live.
    rotate(
        &ws.path().join(".beads"),
        &RotateOptions {
            age_days: 30,
            max_size_mb: Some(100),
            max_archives: 10,
            dry_run: false,
        },
    )
    .unwrap();
    let active_after_rotate = parse_all(&active);
    assert!(
        active_after_rotate.iter().any(|b| b["id"] == "bf-live"),
        "active bead stays in the active file"
    );
    assert!(
        !active_after_rotate.iter().any(|b| b["id"] == "bf-old"),
        "rotated bead leaves the active file"
    );
    let archive_after_rotate = parse_all(&archive);
    assert_eq!(
        archive_after_rotate
            .iter()
            .filter(|b| b["id"] == "bf-old")
            .count(),
        1,
        "rotated bead landed in the archive"
    );

    // Sleep past 1s mtime granularity so any later archive write would land in a
    // strictly-later mtime bucket and be detectable.
    std::thread::sleep(Duration::from_millis(1100));

    let archive_snap = snapshot(&archive);

    // A batch mutation (create) marks one bead dirty; its single follow-up flush
    // is the incremental auto-flush path under test.
    let ops = vec![BatchOp::Create {
        title: "Newly batched".into(),
        type_: "task".into(),
        priority: 2,
        description: None,
        assignee: None,
        labels: vec![],
    }];
    let results = execute_batch(&storage, ops, ws.path(), false /* enable auto-flush **/).unwrap();
    let new_id = results[0].id.clone().expect("create op yields an id");
    let n = autoflush::run(ws.path()).unwrap();
    assert_eq!(n, 1, "the batch flush exports exactly the one new bead");

    // Active file: the new bead landed (surgical merge into the active file).
    let active_final = parse_all(&active);
    assert!(
        active_final.iter().any(|b| b["id"] == new_id),
        "new batched bead must be written to the ACTIVE issues.jsonl"
    );
    assert!(
        active_final.iter().any(|b| b["id"] == "bf-live"),
        "pre-existing active bead preserved"
    );
    assert!(
        !active_final.iter().any(|b| b["id"] == "bf-old"),
        "archived bead must not be revived into the active file by the flush"
    );

    // Archive: the crux of the open question — byte-for-byte identical AND mtime
    // unchanged. The incremental flush must never touch a rotated archive.
    let archive_final = snapshot(&archive);
    assert_eq!(
        archive_final.0, archive_snap.0,
        "auto-flush must NOT rewrite the rotated archive (content)"
    );
    assert_eq!(
        archive_final.1, archive_snap.1,
        "auto-flush must NOT bump the rotated archive's mtime"
    );
    assert!(
        !archive_final.0.windows(4).any(|w| w == new_id.as_bytes()),
        "the new active bead must not have leaked into the archive"
    );
}
