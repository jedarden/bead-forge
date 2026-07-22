//! Integration tests for the `bf count` CLI subcommand.
//!
//! Ports the scenarios previously covered by the ad-hoc root script
//! `test_bf_count.sh` (removed in bf-3o9). The library-level `count_issues()`
//! is exercised elsewhere; these tests cover the *CLI command path* — the
//! output of `bf count` and `bf count --status <STATUS>`, including status
//! filtering and that the output is a single integer.
//!
//! Scenarios (mirroring test_bf_count.sh):
//! - Total count reflects every created bead
//! - `--status open` / `--status closed` / `--status in_progress` filter
//! - Filtered counts are mutually consistent and sum to the total

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Resolve the freshly-built bf binary — never the system-installed one.
fn get_bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Create an isolated workspace with bf config + metadata + empty db.
fn setup_test_workspace() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("test-workspace");
    fs::create_dir_all(&workspace_dir).unwrap();
    let beads_dir = workspace_dir.join(".beads");
    fs::create_dir_all(&beads_dir).unwrap();

    fs::write(
        beads_dir.join("config.yaml"),
        r#"issue_prefixes: [bf]
default_priority: 2
default_type: task
claim_ttl_minutes: 30
"#,
    )
    .unwrap();
    fs::write(
        beads_dir.join("metadata.json"),
        r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
    )
    .unwrap();

    bead_forge::storage::Storage::open(&beads_dir.join("beads.db")).unwrap();
    (temp_dir, beads_dir)
}

fn run_count(workspace: impl AsRef<std::path::Path>, status: Option<&str>) -> u64 {
    let mut cmd = Command::new(get_bf_binary());
    cmd.arg("count").current_dir(workspace.as_ref());
    if let Some(s) = status {
        cmd.arg("--status").arg(s);
    }
    let out = cmd.output().expect("failed to run bf count");
    assert!(
        out.status.success(),
        "bf count failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    stdout
        .trim()
        .parse::<u64>()
        .unwrap_or_else(|_| panic!("bf count should print an integer, got: {stdout:?}"))
}

fn create_bead(workspace: impl AsRef<std::path::Path>, title: &str) -> String {
    let out = Command::new(get_bf_binary())
        .args([
            "create",
            "--title",
            title,
            "--type",
            "task",
            "--priority",
            "2",
        ])
        .current_dir(workspace.as_ref())
        .output()
        .expect("failed to run bf create");
    assert!(
        out.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn update_status(workspace: impl AsRef<std::path::Path>, id: &str, status: &str) {
    let out = Command::new(get_bf_binary())
        .args(["update", id, "--status", status])
        .current_dir(workspace.as_ref())
        .output()
        .expect("failed to run bf update");
    assert!(
        out.status.success(),
        "bf update --status failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn close_bead(workspace: impl AsRef<std::path::Path>, id: &str) {
    let out = Command::new(get_bf_binary())
        .args(["close", id, "--reason", "test cleanup"])
        .current_dir(workspace.as_ref())
        .output()
        .expect("failed to run bf close");
    assert!(
        out.status.success(),
        "bf close failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn test_count_total_reflects_created_beads() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // Fresh workspace is empty.
    assert_eq!(run_count(workspace, None), 0);

    create_bead(workspace, "Bead one");
    create_bead(workspace, "Bead two");
    create_bead(workspace, "Bead three");

    assert_eq!(run_count(workspace, None), 3);
}

#[test]
fn test_count_by_status_filters() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // New beads default to status `open`.
    let a = create_bead(workspace, "open A");
    let b = create_bead(workspace, "in_progress B");
    let _c = create_bead(workspace, "open C");

    update_status(workspace, &b, "in_progress");
    close_bead(workspace, &a);

    // After: open={c}, in_progress={b}, closed={a}.
    assert_eq!(run_count(workspace, Some("open")), 1);
    assert_eq!(run_count(workspace, Some("in_progress")), 1);
    assert_eq!(run_count(workspace, Some("closed")), 1);

    // Total ignores status.
    assert_eq!(run_count(workspace, None), 3);
}

#[test]
fn test_count_filtered_subset_of_total() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    for i in 0..5 {
        create_bead(workspace, &format!("Bead {i}"));
    }
    // All five remain open; closed + in_progress counts should not exceed total.
    let total = run_count(workspace, None);
    let open = run_count(workspace, Some("open"));
    let closed = run_count(workspace, Some("closed"));
    let in_progress = run_count(workspace, Some("in_progress"));

    assert_eq!(total, 5);
    assert_eq!(open, 5);
    assert_eq!(closed, 0);
    assert_eq!(in_progress, 0);
    assert!(open + closed + in_progress <= total);
}
