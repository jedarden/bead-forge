//! Integration tests for *reading/displaying* epic labels via the CLI.
//!
//! Scope (split-child of bf-sdstf): ONLY reading labels back — via `bf show`,
//! `bf labels`, and `bf label list` — for an epic created with labels. Label
//! add/remove mutations, list `--type` filtering, and display sorting are
//! covered by sibling children and are explicitly out of scope here.
//!
//! Builds directly on the fixtures/pattern from `tests/epic_cli_label_creation.rs`
//! (the label-creation sibling): the same isolated-workspace harness, freshly-built
//! `bf` binary, and `create_epic`/`extract_bead_id` helpers are reused verbatim so
//! the two files stay in lockstep.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Create a temporary workspace for testing (mirrors tests/epic_cli.rs and the
/// creation sibling `epic_cli_label_creation.rs`).
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

/// Create an epic via the CLI with the given labels, returning its bead ID.
fn create_epic(workspace: &std::path::Path, labels: &[&str]) -> String {
    let mut cmd = Command::new(get_bf_binary());
    cmd.args(["create", "--title", "Labelled Epic", "--type", "epic"]);
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

/// Run `bf show <id>` (default text format) and return its full stdout.
fn run_show(workspace: &std::path::Path, bead_id: &str) -> String {
    let out = Command::new(get_bf_binary())
        .args(["show", bead_id])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf show");
    assert!(
        out.status.success(),
        "bf show failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// Run `bf labels <id>` in the text format and return the labels it prints.
///
/// `bf labels <id>` prints exactly one label per line with no header, so the
/// set of non-empty trimmed lines IS the label set. We deliberately do not
/// assume any ordering — display sorting is a sibling scope — so callers compare
/// as a set, not a sequence.
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

/// Run `bf label list <id>` and return the labels it prints.
///
/// `bf label list <id>` prints a header `Labels for <id>:` (un-indented) followed
/// by one label per line indented by two spaces. Only the indented lines are
/// labels — filtering on the two-space indent both drops the header and confirms
/// each label was emitted on its own line.
fn run_label_list(workspace: &std::path::Path, bead_id: &str) -> Vec<String> {
    let out = Command::new(get_bf_binary())
        .args(["label", "list", bead_id])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf label list");
    assert!(
        out.status.success(),
        "bf label list failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    stdout
        .lines()
        .filter(|line| line.starts_with("  "))
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

/// Acceptance criterion 1: `bf show <id>` on an epic prints its labels AND
/// records the type as epic.
#[test]
fn test_show_epic_prints_labels_and_type() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let bead_id = create_epic(workspace, &["phase-1", "phase-2"]);
    let show_output = run_show(workspace, &bead_id);

    // The type must be recorded as epic — printed verbatim as "Type: epic".
    assert!(
        show_output.contains("Type: epic"),
        "bf show should print 'Type: epic', got:\n{}",
        show_output
    );
    // A labelled epic surfaces a Labels line, with every label present on it.
    assert!(
        show_output.contains("Labels:"),
        "bf show should print a Labels line for a labelled epic, got:\n{}",
        show_output
    );
    for label in &["phase-1", "phase-2"] {
        assert!(
            show_output.contains(label),
            "bf show should surface label '{}', got:\n{}",
            label,
            show_output
        );
    }
}

/// Acceptance criterion 2: `bf labels <id>` (direct SELECT) returns exactly the
/// labels — no header, no extra/garbage lines, one label per line.
#[test]
fn test_labels_returns_exactly_the_labels_one_per_line() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let expected = ["phase-1", "phase-2", "test"];
    let bead_id = create_epic(workspace, &expected);

    let labels = run_labels(workspace, &bead_id);

    // Exactly the expected count — a stray header or garbage line would inflate it.
    assert_eq!(
        labels.len(),
        expected.len(),
        "bf labels should print exactly {} labels, got {:?}",
        expected.len(),
        labels
    );
    // And exactly the expected set — ordering is a sibling scope, compare as a set.
    let label_set: std::collections::HashSet<&str> =
        labels.iter().map(|s| s.as_str()).collect();
    for label in &expected {
        assert!(
            label_set.contains(label),
            "bf labels should include '{}', got {:?}",
            label,
            labels
        );
    }
}

/// Acceptance criterion 3: `bf label list <id>` returns the same set of labels
/// as `bf labels <id>`.
#[test]
fn test_label_list_returns_same_set_as_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let bead_id = create_epic(workspace, &["phase-1", "phase-2", "test"]);

    let via_labels = run_labels(workspace, &bead_id);
    let via_label_list = run_label_list(workspace, &bead_id);

    // `bf label list` is just a differently-formatted view (header + indentation)
    // of the same underlying labels. Ordering is a sibling scope, so compare as sets.
    let labels_set: std::collections::HashSet<&str> =
        via_labels.iter().map(|s| s.as_str()).collect();
    let list_set: std::collections::HashSet<&str> =
        via_label_list.iter().map(|s| s.as_str()).collect();
    assert_eq!(
        labels_set, list_set,
        "`bf labels` and `bf label list` should return the same label set; \
         got labels={:?} list={:?}",
        via_labels, via_label_list
    );
    assert_eq!(
        via_label_list.len(),
        3,
        "bf label list should return 3 labels, got {:?}",
        via_label_list
    );
}

/// Acceptance criterion 4: an epic with zero labels displays gracefully — `bf show`
/// emits no Labels line at all (neither an empty `Labels:` nor a trailing
/// `Labels: `).
#[test]
fn test_show_epic_with_zero_labels_displays_gracefully() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    // create_epic asserts the create exit status; with no labels it must still
    // succeed and produce a valid epic.
    let bead_id = create_epic(workspace, &[]);
    let show_output = run_show(workspace, &bead_id);

    // The type is still epic, proving the bead itself rendered fine.
    assert!(
        show_output.contains("Type: epic"),
        "bf show should still print 'Type: epic' for a label-less epic, got:\n{}",
        show_output
    );
    // And crucially, no line begins with "Labels:" — no empty/garbage label line.
    let has_labels_line = show_output
        .lines()
        .any(|line| line.trim_start().starts_with("Labels:"));
    assert!(
        !has_labels_line,
        "bf show must not emit a Labels line for a label-less epic, got:\n{}",
        show_output
    );
}
