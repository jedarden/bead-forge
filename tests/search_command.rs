//! Integration test for the `bf search` subcommand.
//!
//! Ports the scenario exercised by the orphaned root-level script
//! `test_bead_b_operations.sh` (Test 8: `bf search "Bead B"`), removed in the
//! bf-3o9 repo-hygiene cleanup. No committed test exercised the `search`
//! subcommand end-to-end until now — a `grep` for `bf search` across `tests/`
//! found only prose mentions (audit bf-5wz0l flagged this as one of two
//! coverage gaps). Covers full-text title/description matching plus the
//! `--status`, `--type`, `--label`, priority-range, and `--limit` filters.

use std::path::{Path, PathBuf};
use std::process::Command;
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

/// Create a bead with the given extra args and return its bare ID.
fn create_full(workspace: &Path, title: &str, extra: &[&str]) -> String {
    let mut args = vec!["create", "--title", title];
    args.extend_from_slice(extra);
    let (out, err, ok) = run_bf(workspace, &args);
    assert!(ok, "bf create failed: {err}");
    let id = out.trim().to_string();
    assert!(!id.is_empty(), "create produced no id: {out}");
    id
}

/// Count the non-empty result rows printed by `search` (text format prints one
/// `[id] title - status (priority)` line per match).
fn result_count(stdout: &str) -> usize {
    stdout
        .lines()
        .filter(|l| l.trim_start().starts_with('['))
        .count()
}

#[test]
fn search_matches_title_and_description() {
    let (_temp, ws) = setup();
    let alpha = create_full(&ws, "Bead Alpha", &["--description", "alpha description"]);
    let beta = create_full(&ws, "Bead Beta", &["--description", "stub needle here"]);
    let gamma = create_full(&ws, "Gamma", &["--description", "unrelated text"]);

    // Title-only match: "Alpha" appears only in alpha's title.
    let (out, err, ok) = run_bf(&ws, &["search", "Alpha"]);
    assert!(ok, "search failed: {err}");
    assert!(out.contains(&alpha), "alpha missing: {out}");
    assert!(!out.contains(&beta) && !out.contains(&gamma), "over-matched: {out}");

    // Description-only match: "needle" appears only in beta's description.
    let (out, err, ok) = run_bf(&ws, &["search", "needle"]);
    assert!(ok, "search failed: {err}");
    assert!(out.contains(&beta), "beta missing on desc match: {out}");
    assert!(
        !out.contains(&alpha) && !out.contains(&gamma),
        "description over-matched: {out}"
    );
}

#[test]
fn search_filters_by_type() {
    let (_temp, ws) = setup();
    let t1 = create_full(&ws, "Task one", &[]);
    let t2 = create_full(&ws, "Task two", &[]);
    let epic = create_full(&ws, "Epic one", &["--type", "epic"]);

    // No query + --type epic returns only the epic.
    let (out, err, ok) = run_bf(&ws, &["search", "--type", "epic"]);
    assert!(ok, "search failed: {err}");
    assert!(out.contains(&epic), "epic missing: {out}");
    assert!(!out.contains(&t1) && !out.contains(&t2), "tasks leaked in: {out}");
}

#[test]
fn search_filters_by_status() {
    let (_temp, ws) = setup();
    let open_bead = create_full(&ws, "Stays open", &[]);
    let closed_bead = create_full(&ws, "Will be closed", &[]);

    let (_o, e, ok) = run_bf(&ws, &["close", &closed_bead, "--reason", "done"]);
    assert!(ok, "bf close failed: {e}");

    let (out, err, ok) = run_bf(&ws, &["search", "--status", "closed"]);
    assert!(ok, "search failed: {err}");
    assert!(out.contains(&closed_bead), "closed bead missing: {out}");
    assert!(!out.contains(&open_bead), "open bead leaked into closed: {out}");

    let (out, err, ok) = run_bf(&ws, &["search", "--status", "open"]);
    assert!(ok, "search failed: {err}");
    assert!(out.contains(&open_bead), "open bead missing: {out}");
    assert!(!out.contains(&closed_bead), "closed bead leaked into open: {out}");
}

#[test]
fn search_filters_by_priority_range() {
    let (_temp, ws) = setup();
    let p0 = create_full(&ws, "Critical task", &["--priority", "0"]);
    let _p2 = create_full(&ws, "Normal task", &["--priority", "2"]);
    let p4 = create_full(&ws, "Backlog task", &["--priority", "4"]);

    // 0=Critical, 4=Backlog. Range [0,1] should match only the critical bead.
    let (out, err, ok) = run_bf(
        &ws,
        &["search", "--priority-min", "0", "--priority-max", "1"],
    );
    assert!(ok, "search failed: {err}");
    assert!(out.contains(&p0), "critical bead missing: {out}");
    assert!(!out.contains(&p4), "backlog bead leaked into range: {out}");
}

#[test]
fn search_filters_by_label() {
    let (_temp, ws) = setup();
    let plain = create_full(&ws, "Plain bead", &[]);
    let labeled = create_full(&ws, "Labeled bead", &["--label", "urgent"]);

    let (out, err, ok) = run_bf(&ws, &["search", "--label", "urgent"]);
    assert!(ok, "search failed: {err}");
    assert!(out.contains(&labeled), "labeled bead missing: {out}");
    assert!(!out.contains(&plain), "plain bead leaked in: {out}");
}

#[test]
fn search_limit_caps_results() {
    let (_temp, ws) = setup();
    // Four beads sharing a title substring so a single query matches all.
    for n in &["one", "two", "three", "four"] {
        create_full(&ws, &format!("Matchme {n}"), &[]);
    }

    let (out, err, ok) = run_bf(&ws, &["search", "Matchme"]);
    assert!(ok, "search failed: {err}");
    assert_eq!(result_count(&out), 4, "expected all four: {out}");

    let (out, err, ok) = run_bf(&ws, &["search", "Matchme", "--limit", "2"]);
    assert!(ok, "search failed: {err}");
    assert_eq!(result_count(&out), 2, "limit not honored: {out}");
}
