//! Canonical flush-FAILURE contract (Phase 7.1 child 3/5, bf-3jc66).
//!
//! This file owns the failure contract end to end. A flush failure must:
//!  1. never fail the mutation (the storage write already committed);
//!  2. surface a warning on BOTH stderr and the `--json` envelope in one stable
//!     shape (a top-level non-null string `warning` key; a `warning:` line on
//!     stderr that never touches stdout);
//!  3. retain the `dirty_issues` marks so the next flush retries automatically;
//!  4. clear on the next explicit `bf sync --flush-only`.
//!
//! Sister suites (`autoflush_mutation.rs`, `autoflush_wiring.rs`) cover the
//! happy and disabled paths; this one asserts the failure contract in a single
//! place so an agent consumer can rely on it.

use bead_forge::storage::Storage;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn bf() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bf"))
}

/// Run `bf` in `ws`; returns (stdout, stderr, success).
fn run(ws: &Path, args: &[&str]) -> (String, String, bool) {
    let out = bf()
        .current_dir(ws)
        .args(args)
        .output()
        .expect("failed to execute bf");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

fn init_ws() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let (_o, e, ok) = run(tmp.path(), &["init", "--prefix", "bf"]);
    assert!(ok, "bf init failed: {e}");
    tmp
}

fn jsonl_path(ws: &Path) -> PathBuf {
    ws.join(".beads").join("issues.jsonl")
}

/// Wedge the auto-flush deterministically: make `issues.jsonl` a directory so
/// the atomic temp+rename export cannot replace it. No permission games, so it
/// works regardless of the test-runner's uid. (Same wedge the happy-path suite
/// uses.)
fn wedge(ws: &Path) {
    let p = jsonl_path(ws);
    if p.exists() {
        fs::remove_file(&p).ok();
    }
    fs::create_dir(&p).unwrap();
}

fn unwedge(ws: &Path) {
    let p = jsonl_path(ws);
    if p.is_dir() {
        fs::remove_dir(&p).unwrap();
    }
}

fn storage_for(ws: &Path) -> Storage {
    let metadata = bead_forge::config::load_metadata(&ws.join(".beads")).unwrap();
    let db_path = ws.join(".beads").join(&metadata.database);
    Storage::open(&db_path).unwrap()
}

fn dirty_ids(storage: &Storage) -> Vec<String> {
    storage
        .list_dirty_issues()
        .unwrap()
        .into_iter()
        .map(|i| i.id)
        .collect()
}

fn jsonl_ids(ws: &Path) -> Vec<String> {
    let p = jsonl_path(ws);
    if !p.exists() || p.is_dir() {
        return Vec::new();
    }
    fs::read_to_string(&p)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            serde_json::from_str::<Value>(l).unwrap()["id"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}

#[test]
fn create_json_succeeds_warns_retains_dirty_and_recovers() {
    let tmp = init_ws();
    let ws = tmp.path();
    wedge(ws);

    // (1) The mutation committed, so exit status is success even though the
    // follow-up flush failed.
    let (stdout, stderr, ok) = run(ws, &["create", "--json", "--title", "wedged"]);
    assert!(ok, "a flush failure must NOT fail create (exit not 0)");

    // stdout must be pure JSON — a single parseable object. This is the
    // `bf create --json | jq .` contract: the human warning line never leaks
    // onto stdout and corrupts no pipe.
    let parsed: Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON (jq-parseable)");
    let id = parsed
        .get("id")
        .and_then(|v| v.as_str())
        .expect("--json must carry the created id")
        .to_string();

    // (2) The --json envelope carries the stable `warning` field: a non-null
    // string naming the failure AND the recovery command.
    let warning = parsed
        .get("warning")
        .and_then(|v| v.as_str())
        .expect("--json must carry a non-null `warning` string on flush failure");
    assert!(
        warning.contains("auto-flush"),
        "warning must describe the auto-flush failure, got: {warning}"
    );
    assert!(
        warning.contains("bf sync --flush-only"),
        "warning must be actionable (name the recovery command), got: {warning}"
    );

    // The prefixed `warning:` line goes to stderr only — never stdout.
    assert!(
        stderr.contains("warning:") && stderr.contains("auto-flush"),
        "stderr must carry the prefixed warning line, got: {stderr}"
    );
    assert!(
        !stdout.contains("warning:"),
        "the prefixed warning line must not appear on stdout, got: {stdout}"
    );

    // The bead exists in the live store despite the flush failure.
    let storage = storage_for(ws);
    let created = storage
        .get_issue(&id)
        .expect("get_issue query failed")
        .expect("bead must exist in db after a successful create");
    assert_eq!(created.title, "wedged");

    // (3) The dirty mark survives the failed flush so the next flush retries.
    assert!(
        dirty_ids(&storage).iter().any(|d| d == &id),
        "dirty mark must survive the failed flush"
    );

    // (4) Once the wedge is cleared, an explicit flush recovers the bead into
    // JSONL and clears the dirty set.
    unwedge(ws);
    let (_o, e, ok) = run(ws, &["sync", "--flush-only"]);
    assert!(ok, "sync --flush-only must succeed once writable: {e}");
    assert!(
        jsonl_ids(ws).iter().any(|x| x == &id),
        "retained-dirty bead must land in issues.jsonl after the recovery flush"
    );
    assert!(
        !dirty_ids(&storage).iter().any(|d| d == &id),
        "dirty mark must be cleared by the recovery flush"
    );
}

#[test]
fn human_path_succeeds_and_warns_stderr_only() {
    let tmp = init_ws();
    let ws = tmp.path();
    wedge(ws);

    // The non-JSON path also never fails and also warns; stdout is just the
    // bare id, so piping it never sees the warning.
    let (stdout, stderr, ok) = run(ws, &["create", "--title", "human wedged"]);
    assert!(ok, "a flush failure must NOT fail create");
    let id = stdout.trim().to_string();
    assert!(
        !id.is_empty() && !id.contains('\n'),
        "stdout must be just the bare id, got: {stdout}"
    );

    assert!(
        stderr.contains("warning:") && stderr.contains("auto-flush"),
        "stderr must carry the prefixed warning, got: {stderr}"
    );
    assert!(
        !stdout.contains("warning:"),
        "the warning line must not leak onto stdout, got: {stdout}"
    );

    // Dirty retention holds on the human path too.
    let storage = storage_for(ws);
    assert!(
        dirty_ids(&storage).iter().any(|d| d == &id),
        "dirty mark must survive the failed flush on the human path"
    );
}

#[test]
fn update_json_path_also_surfaces_the_warning() {
    // Every mutation command that emits `--json` must fold the warning with the
    // SAME shape. `create` is exercised above; this guards that a second
    // mutation path (update of an existing bead) keeps the contract identical
    // rather than silently dropping the warning on the floor.
    let tmp = init_ws();
    let ws = tmp.path();
    let (out, _e, ok) = run(ws, &["create", "--title", "seed"]);
    assert!(ok, "seed create failed");
    let id = out.trim().to_string();

    wedge(ws);

    // `update` has no --json today, so it follows the human contract: succeed,
    // warn on stderr only, retain dirty marks. (Guards against the warning
    // being dropped while update still calls autoflush_after_mutation.)
    let (stdout, stderr, ok) = run(ws, &["update", &id, "--title", "renamed wedged"]);
    assert!(ok, "a flush failure must NOT fail update");
    assert!(
        stderr.contains("warning:") && stderr.contains("auto-flush"),
        "update must surface the flush warning on stderr, got: {stderr}"
    );
    assert!(
        !stdout.contains("warning:"),
        "the warning line must not leak onto update stdout, got: {stdout}"
    );

    let storage = storage_for(ws);
    assert!(
        dirty_ids(&storage).iter().any(|d| d == &id),
        "dirty mark must survive the failed flush on update"
    );
}
