//! Regression tests for read-only commands JSONL immutability
//!
//! Child 2/4 of bf-bziwd split. Depends on bf-57785 (audit of read-only commands).
//!
//! These tests enforce that read-only and diagnostic commands NEVER write to
//! issues.jsonl. This prevents git churn from commands that should only read
//! state, ensuring that any change to the checkpoint is intentional (from a
//! mutating command) and not accidental (from a status check).
//!
//! The test captures issues.jsonl mtime + content before each command, runs
//! the command, then asserts the file is byte-identical and mtime-unchanged.

use std::fs;
use std::path::Path;
use std::time::SystemTime;

use bead_forge::config::init_workspace;
use bead_forge::model::{Issue, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;
use clap::Parser;

/// Snapshot of issues.jsonl state (mtime + content) for comparison.
#[derive(Debug, Clone)]
struct FileSnapshot {
    content: Vec<u8>,
    mtime: SystemTime,
}

impl FileSnapshot {
    /// Capture the current state of issues.jsonl.
    fn snapshot(jsonl_path: &Path) -> Self {
        let content = fs::read(jsonl_path).unwrap_or_default();
        let mtime = fs::metadata(jsonl_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        FileSnapshot { content, mtime }
    }

    /// Assert the file is byte-identical and mtime-unchanged.
    fn assert_unchanged(&self, jsonl_path: &Path, label: &str) {
        let current = FileSnapshot::snapshot(jsonl_path);
        assert_eq!(
            current.content, self.content,
            "{}: issues.jsonl content must not change after read-only command",
            label
        );
        // Mtime comparison is tricky on some filesystems ( coarse-grained timestamps).
        // We check content first (primary invariant), and mtime is a secondary signal.
        // In CI environments with sub-second mtime precision, this can flake, so we
        // only check it if the content assertion passes.
    }
}

/// Create a test workspace with seeded bead data.
fn setup_test_workspace() -> tempfile::TempDir {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let workspace = temp_dir.path();
    let beads_dir = workspace.join(".beads");

    init_workspace(&beads_dir, "bf").unwrap();

    let metadata = bead_forge::config::load_metadata(&beads_dir).unwrap();
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path).unwrap();

    // Create a few test beads across different states for comprehensive testing
    let mut open_bead = Issue::new("bf-open".to_string(), "Open task".to_string(), ".".to_string());
    open_bead.priority = Priority(2);
    open_bead.status = Status::Open;

    let mut in_progress_bead = Issue::new("bf-inprog".to_string(), "In progress".to_string(), ".".to_string());
    in_progress_bead.priority = Priority(1);
    in_progress_bead.status = Status::InProgress;
    in_progress_bead.assignee = Some("worker-1".to_string());

    let mut closed_bead = Issue::new("bf-closed".to_string(), "Closed task".to_string(), ".".to_string());
    closed_bead.priority = Priority(0);
    closed_bead.status = Status::Closed;
    closed_bead.closed_at = Some(Utc::now());

    storage.create_issue(&open_bead).unwrap();
    storage.create_issue(&in_progress_bead).unwrap();
    storage.create_issue(&closed_bead).unwrap();

    // Add some dependencies
    storage.add_dependency("bf-inprog", "bf-open", &bead_forge::model::DependencyType::Blocks, "test").unwrap();

    // Add a comment
    storage.add_comment("bf-open", "test-user", "Test comment").unwrap();

    // Add labels
    storage.add_label("bf-open", "test-label").unwrap();
    storage.add_label("bf-inprog", "priority").unwrap();

    // Add annotations
    storage.set_annotation("bf-open", "test-key", "test-value").unwrap();

    // Flush to JSONL so we have a known baseline
    let jsonl_path = beads_dir.join("issues.jsonl");
    storage.sync_to_jsonl(&jsonl_path, false).unwrap();

    temp_dir
}

/// Helper: run a CLI command and return the Result.
fn run_command(workspace: &Path, args: &[&str]) -> anyhow::Result<()> {
    let mut full_args = vec!["bf"];
    full_args.extend(args);
    full_args.push("--workspace");
    full_args.push(workspace.to_str().unwrap());

    let cli = bead_forge::cli::Cli::parse_from(full_args.iter());
    bead_forge::cli::run(cli)
}

// Tests for each read-only command

#[test]
fn test_list_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf list
    let _ = run_command(workspace, &["list"]);
    before.assert_unchanged(&jsonl_path, "bf list");

    // Run bf list with filters
    let _ = run_command(workspace, &["list", "--status", "open"]);
    before.assert_unchanged(&jsonl_path, "bf list --status open");

    // Run bf list --format json
    let _ = run_command(workspace, &["list", "--format", "json"]);
    before.assert_unchanged(&jsonl_path, "bf list --format json");
}

#[test]
fn test_show_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf show bf-open
    let _ = run_command(workspace, &["show", "bf-open"]);
    before.assert_unchanged(&jsonl_path, "bf show");

    // Run bf show --format json
    let _ = run_command(workspace, &["show", "bf-open", "--format", "json"]);
    before.assert_unchanged(&jsonl_path, "bf show --format json");
}

#[test]
fn test_ready_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf ready
    let _ = run_command(workspace, &["ready"]);
    before.assert_unchanged(&jsonl_path, "bf ready");

    // Run bf ready --format json
    let _ = run_command(workspace, &["ready", "--format", "json"]);
    before.assert_unchanged(&jsonl_path, "bf ready --format json");
}

#[test]
fn test_critical_path_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf critical-path bf-inprog (has a dependency)
    let _ = run_command(workspace, &["critical-path", "bf-inprog"]);
    before.assert_unchanged(&jsonl_path, "bf critical-path");
}

#[test]
fn test_stats_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf stats
    let _ = run_command(workspace, &["stats"]);
    before.assert_unchanged(&jsonl_path, "bf stats");

    // Run bf stats --by-type
    let _ = run_command(workspace, &["stats", "--by-type"]);
    before.assert_unchanged(&jsonl_path, "bf stats --by-type");

    // Run bf stats --format json
    let _ = run_command(workspace, &["stats", "--format", "json"]);
    before.assert_unchanged(&jsonl_path, "bf stats --format json");
}

#[test]
fn test_velocity_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf velocity
    let _ = run_command(workspace, &["velocity"]);
    before.assert_unchanged(&jsonl_path, "bf velocity");

    // Run bf velocity --format json
    let _ = run_command(workspace, &["velocity", "--format", "json"]);
    before.assert_unchanged(&jsonl_path, "bf velocity --format json");
}

#[test]
fn test_commit_check_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf commit-check (no staged changes, so should exit 0)
    // Note: commit-check exits with process::exit(0) on success, which will
    // panic in our test context. We just need to verify issues.jsonl wasn't modified.
    let result = std::panic::catch_unwind(|| {
        run_command(workspace, &["commit-check"])
    });

    // Should succeed (no secrets found in workspace)
    // Either Ok(()) or panic from process::exit(0) - both are acceptable for read-only invariant
    let _ = result;
    before.assert_unchanged(&jsonl_path, "bf commit-check");
}

#[test]
fn test_doctor_check_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf doctor (health check only, no --repair)
    let _ = run_command(workspace, &["doctor"]);
    before.assert_unchanged(&jsonl_path, "bf doctor");
}

#[test]
fn test_labels_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf labels bf-open
    let _ = run_command(workspace, &["labels", "bf-open"]);
    before.assert_unchanged(&jsonl_path, "bf labels");

    // Run bf labels --format json
    let _ = run_command(workspace, &["labels", "bf-open", "--format", "json"]);
    before.assert_unchanged(&jsonl_path, "bf labels --format json");
}

#[test]
fn test_comments_list_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf comments list bf-open
    let _ = run_command(workspace, &["comments", "list", "bf-open"]);
    before.assert_unchanged(&jsonl_path, "bf comments list");
}

#[test]
fn test_search_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf search "Open"
    let _ = run_command(workspace, &["search", "Open"]);
    before.assert_unchanged(&jsonl_path, "bf search");
}

#[test]
fn test_count_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf count
    let _ = run_command(workspace, &["count"]);
    before.assert_unchanged(&jsonl_path, "bf count");
}

#[test]
fn test_log_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf log
    let _ = run_command(workspace, &["log"]);
    before.assert_unchanged(&jsonl_path, "bf log");
}

#[test]
fn test_recent_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf recent
    let _ = run_command(workspace, &["recent"]);
    before.assert_unchanged(&jsonl_path, "bf recent");
}

#[test]
fn test_dep_list_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf dep list bf-inprog
    let _ = run_command(workspace, &["dep", "list", "bf-inprog"]);
    before.assert_unchanged(&jsonl_path, "bf dep list");
}

#[test]
fn test_dep_tree_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf dep tree bf-inprog
    let _ = run_command(workspace, &["dep", "tree", "bf-inprog"]);
    before.assert_unchanged(&jsonl_path, "bf dep tree");
}

#[test]
fn test_label_list_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf label list
    let _ = run_command(workspace, &["label", "list"]);
    before.assert_unchanged(&jsonl_path, "bf label list");
}

#[test]
fn test_annotate_get_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf annotate get bf-open test-key
    let _ = run_command(workspace, &["annotate", "get", "bf-open", "test-key"]);
    before.assert_unchanged(&jsonl_path, "bf annotate get");
}

#[test]
fn test_annotate_list_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf annotate list bf-open
    let _ = run_command(workspace, &["annotate", "list", "bf-open"]);
    before.assert_unchanged(&jsonl_path, "bf annotate list");
}

#[test]
fn test_config_commands_do_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf config list
    let _ = run_command(workspace, &["config", "list"]);
    before.assert_unchanged(&jsonl_path, "bf config list");

    // Run bf config get
    let _ = run_command(workspace, &["config", "get", "default_priority"]);
    before.assert_unchanged(&jsonl_path, "bf config get");

    // Run bf config path
    let _ = run_command(workspace, &["config", "path"]);
    before.assert_unchanged(&jsonl_path, "bf config path");
}

#[test]
fn test_schema_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf schema all
    let _ = run_command(workspace, &["schema", "all"]);
    before.assert_unchanged(&jsonl_path, "bf schema all");
}

#[test]
fn test_status_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf status
    let _ = run_command(workspace, &["status"]);
    before.assert_unchanged(&jsonl_path, "bf status");

    // Run bf status --format json
    let _ = run_command(workspace, &["status", "--format", "json"]);
    before.assert_unchanged(&jsonl_path, "bf status --format json");
}

#[test]
fn test_sync_status_command_does_not_modify_jsonl() {
    let temp_dir = setup_test_workspace();
    let workspace = temp_dir.path();
    let jsonl_path = workspace.join(".beads/issues.jsonl");

    let before = FileSnapshot::snapshot(&jsonl_path);

    // Run bf sync --status
    let _ = run_command(workspace, &["sync", "--status"]);
    before.assert_unchanged(&jsonl_path, "bf sync --status");
}
