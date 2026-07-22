//! Integration tests for epic label *ordering in output* and *epic-level listing
//! & filtering* via the CLI.
//!
//! Scope (split-child of bf-sdstf — the LAST child): ONLY
//!   1. labels are rendered in a stable alphabetical order regardless of the
//!      order they were supplied (verified through `bf labels` and `bf show`),
//!   2. `bf list --type epic` includes a labelled epic and renders its label info
//!      consistently (the structured JSON list view, since the text list line
//!      intentionally shows only id/title/status/priority),
//!   3. distinct epics keep independent label sets with no cross-contamination,
//!   4. `bf label list` (no id) lists every unique label and includes the epic's
//!      labels.
//!
//! Label creation, display-only reads, and add/remove mutation mechanics are
//! covered by the sibling children (epic_cli_label_creation / _display / _mutate)
//! and are explicitly out of scope here.
//!
//! Builds directly on the same isolated-workspace harness, freshly-built `bf`
//! binary, and `create_epic`/`extract_bead_id`/`run_labels`/`run_show`/
//! `run_label_list` helpers as the sibling `epic_cli_label_*.rs` files so the four
//! stay in lockstep.
//!
//! Why alphabetical ordering holds (and is worth pinning down): the `labels` table
//! has `PRIMARY KEY (issue_id, label)` (see src/storage/schema.rs), so SQLite walks
//! an autoindex sorted by `(issue_id, label)`. The label-fetch query
//! `SELECT label FROM labels WHERE issue_id = ?1` therefore returns rows in label
//! order even though it carries no explicit `ORDER BY`. This regression test locks
//! that behaviour in: if the schema or query ever stops walking that index, the
//! ordering tests below fail.

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

/// Create an epic via the CLI with the given labels, returning its bead ID.
fn create_epic(workspace: &std::path::Path, labels: &[&str]) -> String {
    let mut cmd = Command::new(get_bf_binary());
    cmd.args(["create", "--title", "Sorted Epic", "--type", "epic"]);
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

/// Run `bf labels <id>` and return the labels it prints, one per line, in output
/// order. Unlike the display/mutate siblings (which compare as a set), THIS file's
/// whole point is order, so the returned Vec preserves the printed sequence.
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

/// Run `bf list --type epic` (default text format) and return its full stdout.
/// Each epic renders as `[<id>] <title> - <status> (<priority>)` on its own line.
fn run_list_type_epic_text(workspace: &std::path::Path) -> String {
    let out = Command::new(get_bf_binary())
        .args(["list", "--type", "epic"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf list --type epic");
    assert!(
        out.status.success(),
        "bf list --type epic failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// Run `bf list --type epic --format json` and return each rendered issue parsed.
/// The JSON formatter emits one full Issue object per line (newline-delimited
/// JSON), so each non-empty stdout line is parsed independently.
fn run_list_type_epic_json(workspace: &std::path::Path) -> Vec<serde_json::Value> {
    let out = Command::new(get_bf_binary())
        .args(["list", "--type", "epic", "--format", "json"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf list --type epic --format json");
    assert!(
        out.status.success(),
        "bf list --type epic --format json failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            serde_json::from_str(line).unwrap_or_else(|e| {
                panic!("invalid JSON line in `bf list` output: {line:?} ({e})")
            })
        })
        .collect()
}

/// Run `bf label list` (no id) and return each unique label with its bead count.
/// The command prints `All labels:` (a header, dropped) followed by one
/// `  <label> (<count>)` line per label; only the two-space-indented lines are
/// labels.
fn run_label_list_all(workspace: &std::path::Path) -> Vec<(String, i64)> {
    let out = Command::new(get_bf_binary())
        .args(["label", "list"])
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
        .filter_map(|line| {
            let trimmed = line.trim(); // "alpha (1)"
            let open = trimmed.rfind(" (")?;
            let label = trimmed[..open].to_string();
            let tail = &trimmed[open + 2..]; // "1)"
            let count: i64 = tail.trim_end_matches(')').parse().ok()?;
            Some((label, count))
        })
        .collect()
}

/// Acceptance criterion 1 (part a): `bf labels <id>` returns labels in stable
/// alphabetical order regardless of the order they were supplied at creation.
/// Labels are deliberately supplied scrambled (mango, zebra, alpha); the output
/// must be normalized to alpha, mango, zebra — NOT echoed in insertion order.
#[test]
fn test_labels_command_returns_stable_alphabetical_order() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let bead_id = create_epic(workspace, &["mango", "zebra", "alpha"]);
    let labels = run_labels(workspace, &bead_id);

    // Exact alphabetical sequence — this is the regression the bead pins down.
    assert_eq!(
        labels,
        vec![
            "alpha".to_string(),
            "mango".to_string(),
            "zebra".to_string(),
        ],
        "bf labels should return labels in alphabetical order regardless of \
         insertion order, got {:?}",
        labels
    );
}

/// Acceptance criterion 1 (part b): the `Labels:` line in `bf show <id>` is the
/// same alphabetically ordered set, joined by ", " — proving the show path also
/// normalizes insertion order rather than echoing it.
#[test]
fn test_show_labels_line_in_stable_alphabetical_order() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let bead_id = create_epic(workspace, &["mango", "zebra", "alpha"]);
    let show_output = run_show(workspace, &bead_id);

    // The exact comma-joined alphabetical string.
    assert!(
        show_output.contains("Labels: alpha, mango, zebra"),
        "bf show should render the Labels line in alphabetical order, got:\n{}",
        show_output
    );
    // ... and it must NOT echo the scrambled insertion order.
    assert!(
        !show_output.contains("Labels: mango, zebra, alpha"),
        "bf show must not preserve insertion order on the Labels line, got:\n{}",
        show_output
    );
}

/// Acceptance criterion 2: `bf list --type epic` includes the labelled epic and
/// filters out non-epics, and the structured JSON list view renders its label info
/// consistently. (The text list line shows only id/title/status/priority by
/// design, so label info is asserted via `--format json`, which serializes the
/// full Issue including its labels array.)
#[test]
fn test_list_type_epic_includes_labeled_epic_and_renders_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let bead_id = create_epic(workspace, &["alpha", "mango"]);

    // A non-epic (task) must be excluded by --type epic — this proves the filter
    // is real rather than the epic merely being the only bead in the workspace.
    let distractor = Command::new(get_bf_binary())
        .args([
            "create",
            "--title",
            "Distractor Task",
            "--type",
            "task",
            "--label",
            "solo",
        ])
        .current_dir(workspace)
        .output()
        .expect("Failed to create distractor task");
    assert!(
        distractor.status.success(),
        "bf create (distractor task) failed: {}",
        String::from_utf8_lossy(&distractor.stderr)
    );

    // (a) Text list: the labelled epic is present ...
    let text_out = run_list_type_epic_text(workspace);
    assert!(
        text_out.contains(&bead_id),
        "bf list --type epic should include the labelled epic {}, got:\n{}",
        bead_id,
        text_out
    );
    // ... and the distractor task is NOT.
    assert!(
        !text_out.contains("Distractor Task"),
        "bf list --type epic must exclude non-epics, got:\n{}",
        text_out
    );

    // (b) JSON list: the epic renders with issue_type=epic AND its labels,
    //     in the same alphabetical order as `bf labels`.
    let json_issues = run_list_type_epic_json(workspace);
    let epic = json_issues
        .iter()
        .find(|v| v.get("id").and_then(|i| i.as_str()) == Some(bead_id.as_str()))
        .unwrap_or_else(|| {
            panic!(
                "labelled epic {} missing from JSON list output: {:?}",
                bead_id, json_issues
            )
        });
    assert_eq!(
        epic.get("issue_type").and_then(|t| t.as_str()),
        Some("epic"),
        "epic should render issue_type=epic in JSON list, got {:?}",
        epic
    );
    let json_labels: Vec<String> = epic
        .get("labels")
        .and_then(|l| l.as_array())
        .map(|a| a.iter().map(|x| x.as_str().unwrap_or("").to_string()).collect())
        .unwrap_or_default();
    assert_eq!(
        json_labels,
        vec!["alpha".to_string(), "mango".to_string()],
        "epic JSON should render labels in alphabetical order, got {:?}",
        json_labels
    );
}

/// Acceptance criterion 3: distinct epics keep independent label sets — labels on
/// one epic never bleed into another. Each epic gets a private label plus a shared
/// label; each epic's `bf labels` must be exactly its own set (alphabetized) with
/// no trace of the other epic's private label.
#[test]
fn test_distinct_epics_keep_independent_label_sets() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    let epic_a = create_epic(workspace, &["shared", "only-a"]);
    let epic_b = create_epic(workspace, &["shared", "only-b"]);

    let labels_a = run_labels(workspace, &epic_a);
    let labels_b = run_labels(workspace, &epic_b);

    // Each epic: its own private label + the shared one, alphabetized.
    assert_eq!(
        labels_a,
        vec!["only-a".to_string(), "shared".to_string()],
        "epic A should carry only its own + shared labels, got {:?}",
        labels_a
    );
    assert_eq!(
        labels_b,
        vec!["only-b".to_string(), "shared".to_string()],
        "epic B should carry only its own + shared labels, got {:?}",
        labels_b
    );
    // No cross-contamination: neither epic sees the other's private label.
    assert!(
        !labels_a.contains(&"only-b".to_string()),
        "epic A must not pick up epic B's private label, got {:?}",
        labels_a
    );
    assert!(
        !labels_b.contains(&"only-a".to_string()),
        "epic B must not pick up epic A's private label, got {:?}",
        labels_b
    );
}

/// Acceptance criterion 4: `bf label list` (no id) lists every unique label across
/// the whole workspace with per-label counts, and includes the epic's labels. The
/// shared label appears on two epics (count 2); each private label on one (count 1).
/// Ordering of the all-labels listing is by count (not alphabetical), so only
/// presence and counts are asserted — not sequence.
#[test]
fn test_label_list_all_includes_epic_labels() {
    let (_temp, beads_dir) = setup_test_workspace();
    let workspace = beads_dir.parent().unwrap();

    create_epic(workspace, &["shared", "only-a"]);
    create_epic(workspace, &["shared", "only-b"]);

    let all = run_label_list_all(workspace);
    let by_label: std::collections::HashMap<&str, i64> =
        all.iter().map(|(l, c)| (l.as_str(), *c)).collect();

    // Every epic label appears in the workspace-wide listing.
    for label in &["shared", "only-a", "only-b"] {
        assert!(
            by_label.contains_key(*label),
            "bf label list should include '{}', got {:?}",
            label,
            all
        );
    }
    // Counts reflect how many beads carry each label.
    assert_eq!(
        by_label.get("shared"),
        Some(&2),
        "'shared' should have count 2 (on two epics), got {:?}",
        all
    );
    assert_eq!(
        by_label.get("only-a"),
        Some(&1),
        "'only-a' should have count 1, got {:?}",
        all
    );
    assert_eq!(
        by_label.get("only-b"),
        Some(&1),
        "'only-b' should have count 1, got {:?}",
        all
    );
}
