//! Regression tests for bf-1wj: `bf ready --format json` must always include
//! `assignee` (null when unset) and `labels` ([] when empty), matching what
//! `bf show` reports for the same bead.
//!
//! Root cause: the shared `JsonFormatter` serializes the `Issue` struct, whose
//! `assignee`/`labels` fields carry `skip_serializing_if` (compact on-disk JSONL
//! for br/bd compatibility). That omitted the keys entirely from CLI display
//! output, silently breaking downstream consumers (e.g. a NEEDLE explore strand)
//! that filter ready candidates on assignee/labels — an omitted key is
//! indistinguishable from a genuinely unset value once deserialized.
//!
//! `ready` does NOT filter on assignee/labels (it selects on status='open'), so
//! an open bead carrying a stale assignee and a label appears in the output WITH
//! those fields populated.

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn bf_path() -> PathBuf {
    std::env::var("CARGO_BIN_EXE_bf")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./target/debug/bf"))
}

fn init_workspace() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let out = Command::new(bf_path())
        .args(["init", "--prefix", "test"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to init workspace");
    assert!(
        out.status.success(),
        "bf init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    temp_dir
}

fn create_bead(workspace: &std::path::Path, title: &str, extra: &[&str]) -> String {
    let mut cmd = Command::new(bf_path());
    cmd.args([
        "create",
        "--title",
        title,
        "--type",
        "task",
        "--priority",
        "2",
    ])
    .args(extra)
    .current_dir(workspace);
    let out = cmd.output().expect("Failed to create bead");
    assert!(
        out.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Parse `bf ready --format json` (JSONL, one object per line) into a map keyed
/// by bead id.
fn ready_json(workspace: &std::path::Path) -> std::collections::HashMap<String, serde_json::Value> {
    let out = Command::new(bf_path())
        .args(["ready", "--limit", "0", "--format", "json"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf ready");
    assert!(
        out.status.success(),
        "bf ready failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let mut map = std::collections::HashMap::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || line == "[]" {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("invalid JSON line {line:?}: {e}"));
        if let Some(id) = v.get("id").and_then(|i| i.as_str()) {
            map.insert(id.to_string(), v);
        }
    }
    map
}

/// A bead with a real assignee and a real label appears in `bf ready --json`
/// WITH those fields populated (not omitted) — the exact scenario from bf-1wj.
#[test]
fn ready_json_includes_populated_assignee_and_labels() {
    let ws = init_workspace();
    let id = create_bead(
        ws.path(),
        "assigned bead",
        &[
            "--assignee",
            "claude-code-glm-4.7-alpha",
            "--label",
            "split-child",
        ],
    );

    let beads = ready_json(ws.path());
    let bead = beads
        .get(&id)
        .unwrap_or_else(|| panic!("bead {id} missing from ready output"));

    assert_eq!(
        bead.get("assignee").and_then(|a| a.as_str()),
        Some("claude-code-glm-4.7-alpha"),
        "assignee must be populated, got {:?}",
        bead.get("assignee")
    );
    let labels = bead
        .get("labels")
        .and_then(|l| l.as_array())
        .expect("labels must be an array");
    assert!(
        labels.iter().any(|l| l.as_str() == Some("split-child")),
        "labels must contain split-child, got {labels:?}"
    );
}

/// A bead with no assignee and no labels still reports `assignee: null` and
/// `labels: []` — the keys are always present so downstream consumers can
/// deserialize them reliably.
#[test]
fn ready_json_emits_null_assignee_and_empty_labels_when_unset() {
    let ws = init_workspace();
    let id = create_bead(ws.path(), "bare bead", &[]);

    let beads = ready_json(ws.path());
    let bead = beads
        .get(&id)
        .unwrap_or_else(|| panic!("bead {id} missing from ready output"));

    assert_eq!(
        bead.get("assignee"),
        Some(&serde_json::Value::Null),
        "assignee key must be present and null when unset"
    );
    assert_eq!(
        bead.get("labels"),
        Some(&serde_json::Value::Array(vec![])),
        "labels key must be present and an empty array when none"
    );
}
