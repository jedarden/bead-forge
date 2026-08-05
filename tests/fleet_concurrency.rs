//! Fleet-scale concurrent-writer hardening (Phase 7.9).
//!
//! These tests spawn N genuinely concurrent `bf` *processes* (not threads
//! sharing one `Storage`) doing create / claim / close against a single
//! workspace, then assert the invariants the upstream `beads_rust` bug classes
//! violate:
//!
//! * **parallel-write silent loss** — every create must survive; concurrent
//!   ID generation must not collide-and-overwrite; `count` must equal the
//!   number of successful creates.
//! * **spurious sync conflicts during import** — after a fleet of writers, a
//!   flush + fresh-database import must round-trip every bead without loss.
//! * **double claim** — no bead may be claimed by two workers.
//!
//! Each test uses its own tempdir workspace (Test Strategy Rule 1: never touch
//! a live workspace).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use tempfile::TempDir;

fn bf() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bf"))
}

/// Run `bf` with args in `workspace`, returning (stdout, stderr, success).
fn run_bf(workspace: &Path, args: &[&str]) -> (String, String, bool) {
    let output = bf()
        .current_dir(workspace)
        .args(args)
        .output()
        .expect("failed to execute bf");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.success(),
    )
}

fn setup() -> (TempDir, PathBuf) {
    let temp = TempDir::new().unwrap();
    let workspace = temp.path().to_path_buf();
    let (_o, e, ok) = run_bf(&workspace, &["init", "--prefix", "bf"]);
    assert!(ok, "bf init failed: {e}");
    (temp, workspace)
}

fn count(workspace: &Path) -> usize {
    let (out, err, ok) = run_bf(workspace, &["count"]);
    assert!(ok, "bf count failed: {err}");
    out.trim()
        .lines()
        .last()
        .and_then(|l| l.trim().parse::<usize>().ok())
        .unwrap_or_else(|| panic!("could not parse count from: {out:?}"))
}

/// Parse `bead_id` out of `bf claim --format json` output.
fn parse_claimed_id(stdout: &str) -> Option<String> {
    let marker = "\"bead_id\":\"";
    let start = stdout.find(marker)? + marker.len();
    let rest = &stdout[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

#[test]
fn fleet_concurrent_creates_no_silent_loss() {
    let (_temp, workspace) = setup();

    let num_workers = 12;
    let creates_per_worker = 4;
    let expected = num_workers * creates_per_worker;

    let successes = Arc::new(Mutex::new(0usize));
    let mut handles = vec![];

    for w in 0..num_workers {
        let ws = workspace.clone();
        let succ = Arc::clone(&successes);
        handles.push(thread::spawn(move || {
            let mut local = 0;
            for i in 0..creates_per_worker {
                let title = format!("worker-{w}-bead-{i}");
                let (_o, _e, ok) = run_bf(&ws, &["create", "--title", &title]);
                if ok {
                    local += 1;
                }
            }
            *succ.lock().unwrap() += local;
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    let successful = *successes.lock().unwrap();
    assert_eq!(
        successful, expected,
        "some concurrent creates failed outright ({successful}/{expected})"
    );

    // The decisive check: every successful create is durably present. Under the
    // parallel-write-silent-loss bug, count would come back short.
    let n = count(&workspace);
    assert_eq!(
        n, expected,
        "silent write loss: expected {expected} beads, database holds {n}"
    );
}

#[test]
fn fleet_creates_survive_flush_and_reimport() {
    // Hunts spurious sync conflicts / loss on the import path: a fleet writes
    // beads, we flush to JSONL, wipe the DB, and re-import. Nothing may vanish.
    let (_temp, workspace) = setup();
    let beads_dir = workspace.join(".beads");

    let num_workers = 8;
    let creates_per_worker = 3;
    let expected = num_workers * creates_per_worker;

    let mut handles = vec![];
    for w in 0..num_workers {
        let ws = workspace.clone();
        handles.push(thread::spawn(move || {
            for i in 0..creates_per_worker {
                let title = format!("w{w}-b{i}");
                run_bf(&ws, &["create", "--title", &title]);
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    // Flush everything to JSONL.
    let (_o, e, ok) = run_bf(&workspace, &["sync", "--flush-only"]);
    assert!(ok, "flush failed: {e}");

    let jsonl = beads_dir.join("issues.jsonl");
    let jsonl_lines = std::fs::read_to_string(&jsonl)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();
    assert_eq!(
        jsonl_lines, expected,
        "flush lost beads: {jsonl_lines} lines for {expected} beads"
    );

    // Wipe the live DB and re-import from the artifact.
    std::fs::remove_file(beads_dir.join("beads.db")).unwrap();
    let (_o, e, ok) = run_bf(&workspace, &["sync", "--import-only"]);
    assert!(ok, "import failed: {e}");

    let n = count(&workspace);
    assert_eq!(n, expected, "import round-trip lost beads: {n}/{expected}");

    // The merge anchor should have been laid down by the flush/import.
    assert!(
        beads_dir.join("beads.base.jsonl").exists(),
        "merge anchor beads.base.jsonl was not created"
    );
}

#[test]
fn fleet_concurrent_claims_no_double_claim() {
    let (_temp, workspace) = setup();

    let num_beads = 15;
    for i in 0..num_beads {
        let title = format!("claimable-{i}");
        let (_o, e, ok) = run_bf(&workspace, &["create", "--title", &title]);
        assert!(ok, "create failed: {e}");
    }

    // More workers than beads: the surplus must get nothing, never a stolen bead.
    let num_workers = 20;
    let claimed = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut handles = vec![];

    for w in 0..num_workers {
        let ws = workspace.clone();
        let claimed = Arc::clone(&claimed);
        handles.push(thread::spawn(move || {
            let assignee = format!("worker-{w:02}");
            let (out, _e, ok) =
                run_bf(&ws, &["claim", "--assignee", &assignee, "--format", "json"]);
            if ok {
                if let Some(id) = parse_claimed_id(&out) {
                    claimed.lock().unwrap().push(id);
                }
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    let claimed = claimed.lock().unwrap();
    let unique: HashSet<&String> = claimed.iter().collect();
    assert_eq!(
        unique.len(),
        claimed.len(),
        "double claim detected: {:?}",
        *claimed
    );
    assert_eq!(
        claimed.len(),
        num_beads,
        "expected exactly {num_beads} beads claimed, got {}",
        claimed.len()
    );
}
