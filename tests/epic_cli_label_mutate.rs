//! Integration tests for *mutating* an epic's labels via the CLI after creation.
//!
//! Scope (split-child of bf-sdstf): ONLY `bf label add` and `bf label remove`
//! against an existing epic — single and multiple labels, idempotent removal of
//! a label that is not present, and set semantics for duplicate adds. Label
//! creation, display-only reads, and list `--type` filtering/sorting are covered
//! by sibling children and are explicitly out of scope here.
//!
//! Builds directly on the fixtures/pattern from `tests/epic_cli_label_creation.rs`
//! and `tests/epic_cli_label_display.rs` (the sibling children): the same
//! isolated-workspace harness, freshly-built `bf` binary, and
//! `create_epic`/`extract_bead_id`/`run_labels` helpers are reused verbatim so the
//! four siblings stay in lockstep.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Create a temporary workspace for testing (mirrors tests/epic_cli.rs and the
/// sibling `epic_cli_label_*.rs` files).
fn setup_test_workspace() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path().join("test-workspace");
    fs::create_dir_all(&workspace_dir).unwrap();
    let beads_dir = workspace_dir.join(".beads");
    fs::create_dir_all(&beads_dir).unwrap();

    let config_path = beads_dir.join("config.yaml");
    fs::write(
        &config_path,
        r#"issue_prefixes: [bf]
default_priority: 2
default_type: task
claim_ttl_minutes: 30
"#,
    )
    .unwrap();

    let metadata_path = beads_dir.join("metadata.json");
    fs::write(
        &metadata_path,
        r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
    )
    .unwrap();

    let db_path = beads_dir.join("beads.db");
    bead_forge::storage::Storage::open(&db_path).unwrap();

    (temp_dir, beads_dir)
}

/// Get the path to the bf binary — never the system-installed one.
fn get_bf_binary() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Extract bead ID from command output.
fn extract_bead_id(output: &str) -> String {
    output
        .lines()
        .find(|line| line.contains("bf-"))
        .and_then(|line| line.split("bf-").nth(1))
        .map(|id| format!("bf-{}", id.trim().split_whitespace().next().unwrap_or(id)))
        .expect("Could not extract bead ID from output")
}

/// Run `bf labels <id>` in the text format and return the labels it prints.
///
/// `bf labels <id>` prints exactly one label per line (no header), so the set
/// of non-empty trimmed lines IS the label set. We deliberately do not assume
/// any ordering — display sorting is a sibling scope — so callers compare as a
/// set, not a sequence.
fn run_labels(workspace: &std::path::Path, bead_id: &str) -> Vec<String> {
    let out = Command::new(get_bf_binary())
        .args(["labels", bead_id])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf labels");
    assert!(
        out.status.success(),
        "bf labels failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

/// Create an epic via the CLI with the given labels, returning its bead ID.
fn create_epic(workspace: &std::path::Path, labels: &[&str]) -> String {
    let mut cmd = Command::new(get_bf_binary());
    cmd.args(["create", "--title", "Mutable Epic", "--type", "epic"]);
    for label in labels {
        cmd.args(["--label", label]);
    }
    let out = cmd
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(
        out.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    extract_bead_id(&stdout)
}

/// Run `bf label add <id> --label X ...` (one or more `-l`/`--label` flags) and
/// assert it succeeds.
fn run_label_add(workspace: &std::path::Path, bead_id: &str, labels: &[&str]) {
    let mut cmd = Command::new(get_bf_binary());
    cmd.args(["label", "add", bead_id]);
    for label in labels {
        cmd.args(["--label", label]);
    }
    let out = cmd
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf label add");
    assert!(
        out.status.success(),
        "bf label add failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Run `bf label remove <id> --label X ...` (one or more `-l`/`--label` flags)
/// and assert it succeeds. Asserting success here is what pins criterion 4 — a
/// no-op removal must still exit 0.
fn run_label_remove(workspace: &std::path::Path, bead_id: &str, labels: &[&str]) {
    let mut cmd = Command::new(get_bf_binary());
    cmd.args(["label", "remove", bead_id]);
    for label in labels {
        cmd.args(["--label", label]);
    }
    let out = cmd
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf label remove");
    assert!(
        out.status.success(),
        "bf label remove failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Acceptance criterion 1: `bf label add <id> --label X` adds X. Verified via
/// `bf labels` that the count increased (one more than before the add) and that
/// X is present (alongside the label the epic already had).
#[test]
fn test_label_add_single_label() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let bead_id = create_epic(workspace, &["phase-1"]);

    let before = run_labels(workspace, &bead_id);
    assert_eq!(
        before.len(),
        1,
        "epic should start with exactly one label, got {:?}",
        before
    );

    run_label_add(workspace, &bead_id, &["phase-2"]);

    let after = run_labels(workspace, &bead_id);
    // Count increased by exactly one.
    assert_eq!(
        after.len(),
        before.len() + 1,
        "bf label add should increase the label count by one, got {:?}",
        after
    );
    // The newly-added label is present ...
    assert!(
        after.contains(&"phase-2".to_string()),
        "bf label add should add 'phase-2', got {:?}",
        after
    );
    // ... and the original label survives the add.
    assert!(
        after.contains(&"phase-1".to_string()),
        "bf label add must not drop the pre-existing 'phase-1', got {:?}",
        after
    );
}

/// Acceptance criterion 2: `bf label add` with multiple `--label` flags adds all
/// of them in one invocation. Verified via `bf labels` that the count reflects
/// every flag and each label is present.
#[test]
fn test_label_add_multiple_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let bead_id = create_epic(workspace, &["phase-1"]);

    let before = run_labels(workspace, &bead_id);
    assert_eq!(before.len(), 1);

    // One `label add` invocation, three --label flags.
    let added = ["phase-2", "phase-3", "test"];
    run_label_add(workspace, &bead_id, &added);

    let after = run_labels(workspace, &bead_id);
    // Every flag became a label: original one plus three new.
    assert_eq!(
        after.len(),
        before.len() + added.len(),
        "bf label add with {} --label flags should yield {} more labels, got {:?}",
        added.len(),
        added.len(),
        after
    );
    for label in &added {
        assert!(
            after.contains(&label.to_string()),
            "bf label add should have added '{}', got {:?}",
            label,
            after
        );
    }
}

/// Acceptance criterion 3: `bf label remove <id> --label X` removes X. Verified
/// via `bf labels` that X is gone while the others remain (count dropped by one).
#[test]
fn test_label_remove_single_label() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let bead_id = create_epic(workspace, &["phase-1", "phase-2", "test"]);

    let before = run_labels(workspace, &bead_id);
    assert_eq!(
        before.len(),
        3,
        "epic should start with exactly three labels, got {:?}",
        before
    );

    run_label_remove(workspace, &bead_id, &["phase-2"]);

    let after = run_labels(workspace, &bead_id);
    // Count dropped by exactly one.
    assert_eq!(
        after.len(),
        before.len() - 1,
        "bf label remove should decrease the label count by one, got {:?}",
        after
    );
    // The removed label is gone ...
    assert!(
        !after.contains(&"phase-2".to_string()),
        "bf label remove should drop 'phase-2', got {:?}",
        after
    );
    // ... while the untouched labels survive.
    assert!(
        after.contains(&"phase-1".to_string()),
        "bf label remove must not drop 'phase-1', got {:?}",
        after
    );
    assert!(
        after.contains(&"test".to_string()),
        "bf label remove must not drop 'test', got {:?}",
        after
    );
}

/// Acceptance criterion 4: removing a label that is not present is a no-op
/// (idempotent), not an error. Verified that the command exits successfully AND
/// that the existing label set is unchanged.
#[test]
fn test_label_remove_not_present_is_noop() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let bead_id = create_epic(workspace, &["phase-1"]);

    let before = run_labels(workspace, &bead_id);
    assert_eq!(before.len(), 1);

    // Removing a label that was never attached must succeed (exit 0) —
    // run_label_remove asserts the success status, which is the key assertion.
    run_label_remove(workspace, &bead_id, &["does-not-exist"]);

    let after = run_labels(workspace, &bead_id);
    // And the existing labels are untouched — count and contents identical.
    assert_eq!(
        after.len(),
        before.len(),
        "removing an absent label must not change the count, got {:?}",
        after
    );
    assert!(
        after.contains(&"phase-1".to_string()),
        "removing an absent label must leave 'phase-1' intact, got {:?}",
        after
    );
}

/// Acceptance criterion 5: adding a label that is already present does not
/// create a second copy (set semantics). Verified via `bf labels` that the count
/// is unchanged after the duplicate add.
#[test]
fn test_label_add_duplicate_is_set_semantics() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let bead_id = create_epic(workspace, &["phase-1"]);

    let before = run_labels(workspace, &bead_id);
    assert_eq!(before.len(), 1);

    // Adding the very label the epic already has must be a no-op, not an error.
    run_label_add(workspace, &bead_id, &["phase-1"]);

    let after = run_labels(workspace, &bead_id);
    // Still exactly one label — no duplicate row was inserted.
    assert_eq!(
        after.len(),
        before.len(),
        "adding a duplicate label must not increase the count (set semantics), got {:?}",
        after
    );
    assert!(
        after.contains(&"phase-1".to_string()),
        "the original 'phase-1' label must still be present, got {:?}",
        after
    );
}
