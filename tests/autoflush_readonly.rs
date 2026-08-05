//! Integration tests for Phase 7.1 child 2/5 (bead bf-2rjhk): read-only and
//! diagnostic commands MUST NEVER write `.beads/issues.jsonl`.
//!
//! Upstream `beads_rust` lesson #326: status/doctor-style commands that rewrite
//! the JSONL artifact after a read create spurious git churn and can mask real
//! drift (a rewrite that drops, reorders, or reformats beads looks like a clean
//! export). This is the regression class this test pins down: once a workspace
//! is seeded and flushed, every read-only command must leave `issues.jsonl`
//! byte-for-byte identical AND its mtime unchanged — proving no write handle
//! was ever taken. The mutation-side coverage (auto-flush ON / OFF / failure)
//! lives in `autoflush_mutation.rs` and `autoflush_wiring.rs`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

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

/// Run `bf` in `ws` (capturing stdout so it doesn't pollute the test log);
/// returns whether it exited successfully.
fn bf(ws: &Path, args: &[&str]) -> bool {
    Command::new(bf_path())
        .current_dir(ws)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run a `bf` mutation expected to succeed; returns trimmed stdout.
fn ok(ws: &Path, args: &[&str]) -> String {
    let out = Command::new(bf_path())
        .current_dir(ws)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run bf {args:?}: {e}"));
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

/// (content bytes, mtime) snapshot of `issues.jsonl` — the invariant every
/// read-only command must preserve.
fn snapshot(ws: &Path) -> (Vec<u8>, SystemTime) {
    let path = jsonl_path(ws);
    let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read issues.jsonl: {e}"));
    let mtime = fs::metadata(&path)
        .and_then(|m| m.modified())
        .expect("issues.jsonl mtime");
    (bytes, mtime)
}

/// Assert `issues.jsonl` is byte-identical AND its mtime is unchanged vs
/// `snap`. Content equality catches the drift-masking rewrite (#326); mtime
/// equality catches a rewrite that happens to reproduce identical bytes (wasted
/// I/O / churn visible to anything watching mtimes).
fn assert_unchanged(ws: &Path, snap: &(Vec<u8>, SystemTime), label: &str) {
    let (bytes, mtime) = snapshot(ws);
    assert_eq!(
        bytes, snap.0,
        "{label}: read-only command rewrote issues.jsonl (content drift — the #326 regression)"
    );
    assert_eq!(
        mtime, snap.1,
        "{label}: read-only command bumped issues.jsonl mtime (write churn)"
    );
}

/// Create a bead via CLI (auto-flush on) and return its id.
fn create(ws: &Path, title: &str) -> String {
    ok(ws, &["create", "--title", title]).trim().to_string()
}

/// Parse `.beads/issues.jsonl` into one JSON value per non-blank line.
fn read_jsonl(ws: &Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(jsonl_path(ws))
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

#[test]
fn readonly_commands_never_write_jsonl() {
    let ws = init_ws();

    // Seed a workspace with enough variety that every read-only command has
    // real data to traverse: two beads, a dependency, a label, a comment, and
    // an annotation. Each mutation auto-flushes, so issues.jsonl is canonical.
    let blocker = create(ws.path(), "Blocker bead");
    let blocked = create(ws.path(), "Blocked bead");
    ok(ws.path(), &["label", "add", &blocked, "--label", "urgent"]);
    // `dep add <blocker> --blocks <blocked>`: blocked depends on blocker.
    ok(ws.path(), &["dep", "add", &blocker, "--blocks", &blocked]);
    ok(
        ws.path(),
        &["comments", "add", &blocked, "a", "seeded", "note"],
    );
    ok(ws.path(), &["annotate", "set", &blocked, "env", "prod"]);

    // Canonical snapshot. Sleep past coarse (1s) filesystem mtime granularity so
    // that any later write would land in a strictly-later mtime bucket and be
    // detectable, rather than hidden inside the snapshot's own second.
    let snap = snapshot(ws.path());
    std::thread::sleep(Duration::from_millis(1100));

    // Every read-only / diagnostic surface. Args interpolate the seeded ids.
    let cmds: Vec<(&str, Vec<String>)> = vec![
        ("list", vec!["list".into()]),
        (
            "list --json",
            vec!["list".into(), "--format".into(), "json".into()],
        ),
        ("show", vec!["show".into(), blocked.clone()]),
        (
            "show --json",
            vec![
                "show".into(),
                blocked.clone(),
                "--format".into(),
                "json".into(),
            ],
        ),
        ("ready", vec!["ready".into()]),
        (
            "critical-path",
            vec!["critical-path".into(), blocked.clone()],
        ),
        ("velocity", vec!["velocity".into()]),
        ("doctor", vec!["doctor".into()]),
        ("labels", vec!["labels".into(), blocked.clone()]),
        (
            "label list",
            vec!["label".into(), "list".into(), blocked.clone()],
        ),
        (
            "dep list",
            vec!["dep".into(), "list".into(), blocked.clone()],
        ),
        (
            "dep tree",
            vec!["dep".into(), "tree".into(), blocked.clone()],
        ),
        ("search", vec!["search".into(), "blocker".into()]),
        ("stats", vec!["stats".into()]),
        ("log", vec!["log".into(), blocked.clone()]),
        ("recent", vec!["recent".into()]),
        ("count", vec!["count".into()]),
        ("config list", vec!["config".into(), "list".into()]),
        (
            "config get",
            vec!["config".into(), "get".into(), "claim_ttl_minutes".into()],
        ),
        ("config path", vec!["config".into(), "path".into()]),
        ("schema all", vec!["schema".into(), "all".into()]),
        (
            "comments list",
            vec!["comments".into(), "list".into(), blocked.clone()],
        ),
        (
            "annotate get",
            vec![
                "annotate".into(),
                "get".into(),
                blocked.clone(),
                "env".into(),
            ],
        ),
        (
            "annotate list",
            vec!["annotate".into(), "list".into(), blocked.clone()],
        ),
    ];

    for (label, args) in &cmds {
        let argv: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let succeeded = bf(ws.path(), &argv);
        assert!(
            succeeded,
            "{label} unexpectedly failed (stderr above); cannot confirm it left JSONL untouched"
        );
        assert_unchanged(ws.path(), &snap, label);
    }

    // Sanity: the seeded data is still readable exactly as flushed.
    assert!(find(&read_jsonl(ws.path()), &blocker).is_some());
    assert!(find(&read_jsonl(ws.path()), &blocked).is_some());
}

#[test]
fn doctor_does_not_flush_even_with_unflushed_beads() {
    // The sharp #326 case: `doctor` is a DIAGNOSTIC. Even when it observes
    // unflushed (db-only) beads, it must only REPORT them — never rewrite the
    // JSONL artifact itself (that was the upstream regression: a status/doctor
    // read that "helpfully" flushed, creating churn and masking drift). The
    // unflushed bead stays db-only until an explicit `bf sync --flush-only`.
    let ws = init_ws();
    let _flushed = create(ws.path(), "flushed"); // lands in issues.jsonl
    let ghost = ok(
        ws.path(),
        &["--no-auto-flush", "create", "--title", "ghost"],
    )
    .trim()
    .to_string();
    // ghost is db-only: it must be absent from issues.jsonl before AND after.
    assert!(find(&read_jsonl(ws.path()), &ghost).is_none());

    let snap = snapshot(ws.path());
    std::thread::sleep(Duration::from_millis(1100));

    assert!(
        bf(ws.path(), &["doctor"]),
        "bf doctor failed; cannot confirm it left JSONL untouched"
    );
    assert_unchanged(ws.path(), &snap, "doctor (with unflushed beads)");

    // The diagnostic must not have silently flushed the ghost bead.
    assert!(
        find(&read_jsonl(ws.path()), &ghost).is_none(),
        "doctor must not flush db-only beads into issues.jsonl"
    );

    // Recovery path still works: an explicit flush-only lands the ghost bead.
    ok(ws.path(), &["sync", "--flush-only"]);
    assert!(
        find(&read_jsonl(ws.path()), &ghost).is_some(),
        "sync --flush-only should recover the previously-unflushed bead"
    );
}
