//! Integration tests for Phase 7.1 child 3/5 (bf-2cfqm): auto-flush wiring into
//! the *non*-single-issue mutation paths — `batch`, `mitosis`, `claim`, and
//! `delete` — each with a single end-of-transaction flush.
//!
//! Properties asserted (all against the real `bf` binary in a tempdir):
//! * **batch / mitosis** flush EXACTLY ONCE at transaction end (no per-op write
//!   amplification) and preserve every untouched bead already in JSONL — proven
//!   by (a) a library-level check that `execute_batch` performs ZERO JSONL
//!   writes on its own, so the only write is the command's single follow-up
//!   flush, and (b) an end-to-end check that a batch/mitosis of N ops lands all
//!   N beads while leaving pre-existing beads intact.
//! * **claim** writes the claimed bead's new state to JSONL; a flush failure
//!   surfaces a warning without failing the claim (the bead stays dirty for
//!   recovery).
//! * **delete** removes the deleted bead's line from JSONL with no orphan line
//!   left behind, while preserving the other beads.
//! * `--no-auto-flush` leaves JSONL untouched on every one of these paths.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use bead_forge::storage::Storage;
use bead_forge::{execute_batch, BatchOp};

fn bf_path() -> PathBuf {
    std::env::var("CARGO_BIN_EXE_bf")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./target/debug/bf"))
}

/// Fresh workspace initialized via `bf init` (a real on-disk `.beads/`).
fn init_ws() -> tempfile::TempDir {
    let tmp = tempfile::TempDir::new().unwrap();
    let out = Command::new(bf_path())
        .args(["init", "--prefix", "bf"])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run bf init");
    assert!(
        out.status.success(),
        "bf init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    tmp
}

fn bf(ws: &Path, args: &[&str]) -> Output {
    Command::new(bf_path())
        .current_dir(ws)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run bf {args:?}: {e}"))
}

/// Run a bf command expecting success; returns stdout.
fn ok(ws: &Path, args: &[&str]) -> String {
    let out = bf(ws, args);
    assert!(
        out.status.success(),
        "bf {:?} failed (exit {:?}): {}",
        args,
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

fn jsonl_path(ws: &Path) -> PathBuf {
    ws.join(".beads").join("issues.jsonl")
}

/// Parse `.beads/issues.jsonl` into one JSON value per non-blank line.
fn read_jsonl(ws: &Path) -> Vec<serde_json::Value> {
    let p = jsonl_path(ws);
    if !p.exists() {
        return Vec::new();
    }
    std::fs::read_to_string(&p)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn find<'a>(beads: &'a [serde_json::Value], id: &str) -> Option<&'a serde_json::Value> {
    beads
        .iter()
        .find(|b| b.get("id").and_then(|v| v.as_str()) == Some(id))
}

fn field<'a>(bead: &'a serde_json::Value, key: &str) -> &'a str {
    bead.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

/// Create a bead via CLI (auto-flush on) and return its id.
fn create(ws: &Path, title: &str) -> String {
    ok(ws, &["create", "--title", title]).trim().to_string()
}

// ==================== batch ====================

/// The batch executor itself must never touch JSONL: it only mutates SQLite and
/// marks beads dirty. The *single* JSONL write is the command's end-of-batch
/// flush. Proving zero writes here pins down "exactly one write per batch".
#[test]
fn execute_batch_performs_no_jsonl_write() {
    let ws = init_ws();
    // Seed one bead so issues.jsonl exists with known contents.
    let seed = create(ws.path(), "seed");
    let before = std::fs::read_to_string(jsonl_path(ws.path())).unwrap();

    let beads_dir = ws.path().join(".beads");
    let metadata = bead_forge::config::load_metadata(&beads_dir).unwrap();
    let storage = Storage::open(&beads_dir.join(&metadata.database)).unwrap();
    let ops = vec![
        BatchOp::Create {
            title: "b1".into(),
            type_: "task".into(),
            priority: 2,
            description: None,
            assignee: None,
            labels: vec![],
        },
        BatchOp::Create {
            title: "b2".into(),
            type_: "task".into(),
            priority: 2,
            description: None,
            assignee: None,
            labels: vec![],
        },
    ];
    let results = execute_batch(&storage, ops, ws.path()).unwrap();
    assert_eq!(results.len(), 2);

    // execute_batch wrote to SQLite only — issues.jsonl is byte-identical.
    let after = std::fs::read_to_string(jsonl_path(ws.path())).unwrap();
    assert_eq!(
        before, after,
        "execute_batch must not write JSONL; the flush is the caller's single follow-up"
    );
    // But it DID mark the new beads dirty, so one flush exports them.
    let dirty = storage.list_dirty_issues().unwrap();
    assert_eq!(dirty.len(), 2, "batch ops must mark every touched bead dirty");
    assert!(find(&read_jsonl(ws.path()), &seed).is_some());
}

#[test]
fn batch_flushes_all_ops_and_preserves_existing() {
    let ws = init_ws();
    let s1 = create(ws.path(), "seed1");
    let s2 = create(ws.path(), "seed2");

    let ops = r#"[{"op":"create","title":"c1"},{"op":"create","title":"c2"},{"op":"create","title":"c3"}]"#;
    ok(ws.path(), &["batch", "--json", ops]);

    let beads = read_jsonl(ws.path());
    // Pre-existing beads survive (surgical merge, not full replace).
    assert!(find(&beads, &s1).is_some(), "seed1 must survive the batch flush");
    assert!(find(&beads, &s2).is_some(), "seed2 must survive the batch flush");
    // All three created beads landed in a single flush.
    let created: Vec<_> = beads
        .iter()
        .filter(|b| {
            let t = field(b, "title");
            t == "c1" || t == "c2" || t == "c3"
        })
        .collect();
    assert_eq!(created.len(), 3, "all batch-created beads must be flushed, got {beads:?}");
}

#[test]
fn batch_no_auto_flush_leaves_jsonl_untouched() {
    let ws = init_ws();
    let seed = create(ws.path(), "seed");
    let before = std::fs::read_to_string(jsonl_path(ws.path())).unwrap();

    let ops = r#"[{"op":"create","title":"ghost1"},{"op":"create","title":"ghost2"}]"#;
    ok(ws.path(), &["--no-auto-flush", "batch", "--json", ops]);

    let after = std::fs::read_to_string(jsonl_path(ws.path())).unwrap();
    assert_eq!(before, after, "--no-auto-flush batch must not rewrite issues.jsonl");
    // The seed remains; the ghosts never reached JSONL.
    let beads = read_jsonl(ws.path());
    assert!(find(&beads, &seed).is_some());
    assert!(!beads.iter().any(|b| field(b, "title").starts_with("ghost")));
}

// ==================== mitosis ====================

#[test]
fn mitosis_flushes_once_children_and_closed_parent() {
    let ws = init_ws();
    let seed = create(ws.path(), "seed");
    let parent = create(ws.path(), "parent-epic");

    let children = r#"[{"title":"ch1","type":"task","priority":2},{"title":"ch2","type":"task","priority":2}]"#;
    ok(
        ws.path(),
        &["mitosis", &parent, "--children", children, "--reason", "split"],
    );

    let beads = read_jsonl(ws.path());
    // Parent closed and flushed.
    assert_eq!(
        field(find(&beads, &parent).expect("parent must be present"), "status"),
        "closed",
        "mitosis must flush the closed parent"
    );
    // Both children flushed.
    let kids: Vec<_> = beads
        .iter()
        .filter(|b| matches!(field(b, "title"), "ch1" | "ch2"))
        .collect();
    assert_eq!(kids.len(), 2, "both mitosis children must be flushed");
    // Untouched seed preserved.
    assert!(find(&beads, &seed).is_some(), "seed must survive the mitosis flush");
}

// ==================== claim ====================

#[test]
fn claim_flushes_claimed_bead_state() {
    let ws = init_ws();
    let id = create(ws.path(), "claimable");

    let out = ok(ws.path(), &["claim", "--assignee", "worker-1", "--json"]);
    let v: serde_json::Value = serde_json::from_str(out.trim()).expect("claim --json");
    assert_eq!(v.get("bead_id").and_then(|x| x.as_str()), Some(id.as_str()));

    let beads = read_jsonl(ws.path());
    let bead = find(&beads, &id).expect("claimed bead must be in JSONL");
    assert_eq!(
        field(bead, "assignee"),
        "worker-1",
        "claim must flush the new assignee, got {bead:?}"
    );
    assert_eq!(
        field(bead, "status"),
        "in_progress",
        "claim must flush the in_progress status"
    );
}

#[test]
fn claim_flush_failure_warns_without_failing() {
    let ws = init_ws();
    let id = create(ws.path(), "claimable"); // flushes fine (creates the file)

    // Wedge the flush: replace issues.jsonl with a directory so temp+rename fails.
    let p = jsonl_path(ws.path());
    std::fs::remove_file(&p).ok();
    std::fs::create_dir(&p).unwrap();

    let out = bf(ws.path(), &["claim", "--assignee", "worker-1"]);
    assert!(
        out.status.success(),
        "a flush failure must NOT fail the claim (exit {:?})",
        out.status.code()
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("warning:") && stderr.contains("auto-flush"),
        "claim must warn on stderr when the flush fails, got: {stderr}"
    );

    // The claim committed in SQLite and the bead is still dirty for recovery.
    let beads_dir = ws.path().join(".beads");
    let metadata = bead_forge::config::load_metadata(&beads_dir).unwrap();
    let storage = Storage::open(&beads_dir.join(&metadata.database)).unwrap();
    let issue = storage.get_issue(&id).unwrap().unwrap();
    assert_eq!(issue.assignee.as_deref(), Some("worker-1"), "claim must persist");
    assert!(
        storage.list_dirty_issues().unwrap().iter().any(|i| i.id == id),
        "claimed bead must stay dirty after a failed flush so recovery is possible"
    );
}

// ==================== delete ====================

#[test]
fn delete_removes_line_and_preserves_others() {
    let ws = init_ws();
    let a = create(ws.path(), "alpha");
    let b = create(ws.path(), "beta");
    let c = create(ws.path(), "gamma");
    assert_eq!(read_jsonl(ws.path()).len(), 3);

    ok(ws.path(), &["delete", &b]);

    let beads = read_jsonl(ws.path());
    assert!(
        find(&beads, &b).is_none(),
        "deleted bead's line must be pruned from JSONL (no orphan)"
    );
    assert!(find(&beads, &a).is_some(), "alpha must remain after deleting beta");
    assert!(find(&beads, &c).is_some(), "gamma must remain after deleting beta");
    assert_eq!(beads.len(), 2, "exactly one line removed");
}

#[test]
fn delete_no_auto_flush_leaves_jsonl_untouched() {
    let ws = init_ws();
    let a = create(ws.path(), "alpha");
    let b = create(ws.path(), "beta");
    let before = std::fs::read_to_string(jsonl_path(ws.path())).unwrap();

    ok(ws.path(), &["--no-auto-flush", "delete", &b]);

    // With auto-flush off, the JSONL is not touched: the orphan line remains
    // until the next explicit `bf sync --flush-only`.
    let after = std::fs::read_to_string(jsonl_path(ws.path())).unwrap();
    assert_eq!(before, after, "--no-auto-flush delete must not rewrite issues.jsonl");
    let beads = read_jsonl(ws.path());
    assert!(find(&beads, &a).is_some());
    assert!(
        find(&beads, &b).is_some(),
        "without auto-flush the deleted bead's line lingers (recovered by explicit flush)"
    );
}
