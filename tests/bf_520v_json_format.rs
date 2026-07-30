//! Integration tests for bf-520v: JSON output of the multi-bead listing commands.
//!
//! The `JsonFormatter::format_issues` path powers `bf list`/`bf ready`/`bf
//! search`/`bf recent` when invoked with `--format json`. It emits **JSONL** —
//! one self-contained JSON object per line, joined by `\n`, with no array
//! wrapper — for a non-empty result, and the empty string for `list`. (`ready`
//! special-cases the empty case to `[]`; `list` does not.) These tests pin that
//! shape end-to-end through the CLI for the empty / single / multiple cases,
//! complementing the unit tests in `src/format/json.rs`.

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Resolve the freshly-built bf binary — never the system-installed one.
fn bf_path() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Create an isolated workspace via `bf init`.
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

/// Create a task bead via the CLI, returning its printed id.
fn create_bead(workspace: &std::path::Path, title: &str) -> String {
    let out = Command::new(bf_path())
        .args(["create", "--title", title, "--type", "task", "--priority", "2"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(
        out.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Run `bf list --format json` and return raw stdout.
fn list_json(workspace: &std::path::Path) -> String {
    let out = Command::new(bf_path())
        .args(["list", "--format", "json"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf list");
    assert!(
        out.status.success(),
        "bf list failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// Run `bf ready --format json` (unlimited) and return raw stdout.
fn ready_json(workspace: &std::path::Path) -> String {
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
    String::from_utf8(out.stdout).unwrap()
}

/// Parse JSONL (one object per line), skipping blank/`[]` lines, into a list of
/// values. Panics if any non-empty line is not valid JSON.
fn parse_jsonl(stdout: &str) -> Vec<serde_json::Value> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && *line != "[]")
        .map(|line| {
            serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("invalid JSON line {line:?}: {e}"))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// bf list --format json
// ---------------------------------------------------------------------------

/// `bf list` on an empty workspace emits nothing — `format_issues(&[])` is the
/// empty string, and `list` does not special-case emptiness the way `ready`
/// does (no `[]`).
#[test]
fn list_json_empty_yields_no_output() {
    let ws = init_workspace();
    let out = list_json(ws.path());
    assert!(out.is_empty(), "empty list must print nothing, got {out:?}");
}

/// A single bead produces exactly one valid JSON object on stdout.
#[test]
fn list_json_single_bead_is_one_valid_line() {
    let ws = init_workspace();
    let id = create_bead(ws.path(), "only one");

    let out = list_json(ws.path());
    let parsed = parse_jsonl(&out);
    assert_eq!(parsed.len(), 1, "expected one JSON object, got {out:?}");

    let bead = &parsed[0];
    assert_eq!(bead.get("id").and_then(|v| v.as_str()), Some(id.as_str()));
    assert_eq!(bead.get("title").and_then(|v| v.as_str()), Some("only one"));
    // Display-normalized keys are present even on a bare bead.
    assert!(bead.get("assignee").is_some());
    assert!(bead.get("labels").is_some());
}

/// Multiple beads produce JSONL — one object per line, each independently
/// valid, and every created bead is represented.
#[test]
fn list_json_multiple_beads_is_jsonl_one_per_line() {
    let ws = init_workspace();
    let id_a = create_bead(ws.path(), "alpha");
    let id_b = create_bead(ws.path(), "beta");
    let id_c = create_bead(ws.path(), "gamma");

    let out = list_json(ws.path());
    assert_eq!(
        out.lines().count(),
        3,
        "three beads must produce exactly three JSONL lines; got {out:?}"
    );

    let parsed = parse_jsonl(&out);
    assert_eq!(parsed.len(), 3);

    let ids: Vec<String> = parsed
        .iter()
        .map(|v| v.get("id").and_then(|i| i.as_str()).unwrap().to_string())
        .collect();
    for expected in [&id_a, &id_b, &id_c] {
        assert!(
            ids.iter().any(|id| id == expected),
            "bead {expected} missing from list output: {ids:?}"
        );
    }

    // Concatenated JSONL must not parse as a single JSON value (no array wrapper).
    assert!(
        serde_json::from_str::<serde_json::Value>(&out).is_err(),
        "multi-bead JSONL output must not be a single JSON value"
    );
}

// ---------------------------------------------------------------------------
// bf ready --format json
// ---------------------------------------------------------------------------

/// `bf ready` on an empty workspace prints `[]` (its empty special case), not
/// the empty string `list` emits.
#[test]
fn ready_json_empty_emits_empty_array() {
    let ws = init_workspace();
    let out = ready_json(ws.path());
    assert_eq!(out.trim(), "[]", "empty ready must print [], got {out:?}");
}

/// A single open, unblocked bead is the sole ready candidate and appears as one
/// valid JSON object (not `[]`, not array-wrapped).
#[test]
fn ready_json_single_bead_is_one_valid_line() {
    let ws = init_workspace();
    let id = create_bead(ws.path(), "ready one");

    let out = ready_json(ws.path());
    assert_ne!(out.trim(), "[]", "a ready bead must produce output, not []");

    let parsed = parse_jsonl(&out);
    assert_eq!(parsed.len(), 1, "expected one ready object, got {out:?}");

    let bead = &parsed[0];
    assert_eq!(bead.get("id").and_then(|v| v.as_str()), Some(id.as_str()));
    assert_eq!(bead.get("title").and_then(|v| v.as_str()), Some("ready one"));
    // Ready candidates are open.
    assert_eq!(bead.get("status").and_then(|v| v.as_str()), Some("open"));
}

/// Multiple open beads each appear as a separate JSONL line in the ready output.
#[test]
fn ready_json_multiple_beads_is_jsonl_one_per_line() {
    let ws = init_workspace();
    let id_a = create_bead(ws.path(), "ready alpha");
    let id_b = create_bead(ws.path(), "ready beta");
    let id_c = create_bead(ws.path(), "ready gamma");

    let out = ready_json(ws.path());
    assert_ne!(out.trim(), "[]");

    let parsed = parse_jsonl(&out);
    assert_eq!(parsed.len(), 3, "expected three ready objects, got {out:?}");

    let ids: Vec<String> = parsed
        .iter()
        .map(|v| v.get("id").and_then(|i| i.as_str()).unwrap().to_string())
        .collect();
    for expected in [&id_a, &id_b, &id_c] {
        assert!(
            ids.iter().any(|id| id == expected),
            "ready bead {expected} missing from output: {ids:?}"
        );
    }

    // All candidates must be open (ready filters on status='open').
    for bead in &parsed {
        assert_eq!(
            bead.get("status").and_then(|v| v.as_str()),
            Some("open"),
            "ready bead must be open: {bead}"
        );
    }
}
