//! JSON output consistency verification for bf-2nhb.
//!
//! These tests lock in the result of auditing every `--format json` code path
//! across the six commands in scope (list, ready, search, claim, stats,
//! velocity). The audit found:
//!
//! - **Issue-array commands** (`list`, `ready`, `search`, and also `recent`)
//!   all route through `get_formatter().format_issues()`, emitting JSONL —
//!   one full `Issue` object per line, NOT a single JSON array. `ready` was
//!   the last to be migrated (bead bf-64zt).
//! - **Object commands** (`claim`, `stats`) use the dedicated
//!   `format_claim_result` / `format_stats` trait methods (a single object,
//!   never an Issue, never an array) — correct by design.
//! - **`velocity`** uses `format_velocity` (a JSON array of `VelocityStats`).
//!
//! The strongest assertion here is [`issue_array_commands_share_formatter`]:
//! for the same bead, `list`, `ready`, and `search` emit *byte-identical* JSON.
//! That can only hold if all three funnel through the shared `JsonFormatter`,
//! so this test is also the runtime proof that "no custom `println!` JSON loops
//! remain" in the issue-array commands.

use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Resolve the `bf` binary under test. `CARGO_BIN_EXE_bf` is set automatically
/// by cargo for integration tests (the package defines a `[[bin]] name = "bf"`).
fn bf_path() -> PathBuf {
    std::env::var("CARGO_BIN_EXE_bf")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./target/debug/bf"))
}

/// Run `bf` with `--workspace <ws>` and the given args; assert success and
/// return stdout as a String.
fn run(ws: &std::path::Path, args: &[&str]) -> String {
    let out = Command::new(bf_path())
        .arg("--workspace")
        .arg(ws)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn bf {:?}: {e}", args));
    assert!(
        out.status.success(),
        "bf {:?} failed (exit {:?}): {}",
        args,
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("bf stdout is not valid UTF-8")
}

fn init_workspace() -> TempDir {
    let dir = TempDir::new().unwrap();
    run(dir.path(), &["init", "--prefix", "test"]);
    dir
}

/// Create a bead; returns its printed id.
fn create_bead(ws: &std::path::Path, title: &str, extra: &[&str]) -> String {
    let mut args = vec![
        "create",
        "--title",
        title,
        "--type",
        "task",
        "--priority",
        "2",
    ];
    args.extend_from_slice(extra);
    run(ws, &args).trim().to_string()
}

/// Parse JSONL (one Issue per line) into a map keyed by bead id. Lines that are
/// empty or the literal `[]` (ready's empty placeholder) are skipped.
fn jsonl_by_id(stdout: &str) -> HashMap<String, Value> {
    let mut map = HashMap::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || line == "[]" {
            continue;
        }
        let v: Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("invalid JSON line {line:?}: {e}"));
        if let Some(id) = v.get("id").and_then(|i| i.as_str()) {
            map.insert(id.to_string(), v);
        }
    }
    map
}

// ---------------------------------------------------------------------------
// Issue-array commands: list / ready / search
// ---------------------------------------------------------------------------

/// The cornerstone consistency test. For the same bead, `list`, `ready`, and
/// `search` must emit byte-identical JSON — only achievable if all three share
/// `get_formatter().format_issues()`. Any custom `println!` loop in one of them
/// would diverge (e.g. a different field set or ordering) and fail here.
#[test]
fn issue_array_commands_share_formatter() {
    let ws = init_workspace();
    let assigned = create_bead(
        ws.path(),
        "shared formatter probe",
        &["--assignee", "worker-9", "--label", "phase-1"],
    );
    let _other = create_bead(ws.path(), "another bead", &[]);

    let list_out = run(ws.path(), &["list", "--format", "json"]);
    let ready_out = run(ws.path(), &["ready", "--limit", "0", "--format", "json"]);
    let search_out = run(ws.path(), &["search", "--format", "json"]);

    let from_list = list_out
        .lines()
        .find(|l| l.contains(&format!("\"id\":\"{assigned}\"")))
        .unwrap_or_else(|| panic!("bead {assigned} missing from list output"));
    let from_ready = ready_out
        .lines()
        .find(|l| l.contains(&format!("\"id\":\"{assigned}\"")))
        .unwrap_or_else(|| panic!("bead {assigned} missing from ready output"));
    let from_search = search_out
        .lines()
        .find(|l| l.contains(&format!("\"id\":\"{assigned}\"")))
        .unwrap_or_else(|| panic!("bead {assigned} missing from search output"));

    assert_eq!(
        from_list, from_ready,
        "list and ready diverged — they do not share the formatter"
    );
    assert_eq!(
        from_list, from_search,
        "list and search diverged — they do not share the formatter"
    );
}

/// `list`/`ready`/`search` emit JSONL (one JSON object per line), not a single
/// JSON array. Every line is a parseable object that always carries `assignee`
/// (null when unset) and `labels` (always an array) — the bf-1wj display contract.
#[test]
fn issue_arrays_are_jsonl_with_stable_fields() {
    let ws = init_workspace();
    create_bead(ws.path(), "bare bead", &[]);
    create_bead(
        ws.path(),
        "labeled bead",
        &["--assignee", "worker-1", "--label", "x"],
    );

    for cmd in &["list", "ready"] {
        let out = run(ws.path(), &[cmd, "--limit", "0", "--format", "json"]);
        // Not a wrapped array: the whole stdout is not itself a JSON array.
        let trimmed = out.trim();
        assert!(
            !trimmed.starts_with('['),
            "{cmd} output must be JSONL, not a JSON array"
        );
        assert!(!trimmed.is_empty(), "{cmd} should have >0 beads to inspect");
        for line in out.lines() {
            let v: Value = serde_json::from_str(line).expect("each JSONL line is valid JSON");
            assert!(v.is_object(), "{cmd} line is not an object: {line}");
            assert!(v.get("id").is_and_some(), "{cmd} missing id");
            assert!(v.get("title").is_and_some(), "{cmd} missing title");
            assert!(v.get("status").is_and_some(), "{cmd} missing status");
            assert!(
                v.get("assignee").is_some(),
                "{cmd} must always emit assignee (bf-1wj)"
            );
            assert!(
                v.get("labels").map_or(false, |l| l.is_array()),
                "{cmd} labels must be an array (bf-1wj)"
            );
        }
    }
}

/// The `--json` shorthand must be identical to `--format json`.
#[test]
fn json_alias_matches_format_flag() {
    let ws = init_workspace();
    create_bead(ws.path(), "alias probe", &[]);
    let via_format = run(ws.path(), &["list", "--format", "json"]);
    let via_alias = run(ws.path(), &["list", "--json"]);
    assert_eq!(via_format, via_alias, "--json must equal --format json");
}

/// `search` for a known term returns only matching beads as JSONL.
#[test]
fn search_jsonl_is_consistent_with_list() {
    let ws = init_workspace();
    let needle = create_bead(ws.path(), "unique-needle-title", &[]);
    let _hay = create_bead(ws.path(), "unrelated-bead", &[]);

    let search_map = jsonl_by_id(&run(
        ws.path(),
        &["search", "unique-needle", "--format", "json"],
    ));
    let list_map = jsonl_by_id(&run(ws.path(), &["list", "--format", "json"]));

    // search returned exactly the needle.
    assert_eq!(search_map.len(), 1, "search should return only the needle");
    // and that bead's JSON is identical to what list emits for it.
    assert_eq!(
        search_map.get(&needle),
        list_map.get(&needle),
        "search and list disagree on the same bead's JSON"
    );
}

// ---------------------------------------------------------------------------
// Object command: claim
// ---------------------------------------------------------------------------

/// `claim --format json` emits a single JSON object (never an Issue, never an
/// array) via `format_claim_result`.
#[test]
fn claim_emits_single_object() {
    let ws = init_workspace();
    create_bead(ws.path(), "claimable", &[]);

    let out = run(
        ws.path(),
        &["claim", "--assignee", "claimer-1", "--format", "json"],
    );
    let v: Value = serde_json::from_str(out.trim()).expect("claim output is one JSON object");
    assert!(v.is_object(), "claim must emit a single object, got: {out}");
    assert!(v.get("bead_id").is_and_some(), "claim missing bead_id");
    assert_eq!(
        v.get("assignee").and_then(|a| a.as_str()),
        Some("claimer-1")
    );
    // Not an Issue object (no status/priority/issue_type keys) and not an array.
    assert!(
        v.get("status").is_none(),
        "claim result must not be an Issue shape"
    );
    assert!(!out.trim().starts_with('['), "claim must not be an array");
}

/// Dry-run claim adds the preview fields (title/priority/impact/workspace) plus
/// `dry_run: true`, and omits `reclaimed`.
#[test]
fn claim_dry_run_emits_preview_object() {
    let ws = init_workspace();
    create_bead(ws.path(), "claimable", &[]);

    let out = run(
        ws.path(),
        &[
            "claim",
            "--assignee",
            "previewer",
            "--dry-run",
            "--format",
            "json",
        ],
    );
    let v: Value = serde_json::from_str(out.trim()).expect("dry-run output is one JSON object");
    assert_eq!(v.get("dry_run").and_then(|d| d.as_bool()), Some(true));
    assert!(
        v.get("reclaimed").is_none(),
        "reclaimed must be omitted on dry-run"
    );
    assert!(v.get("title").is_and_some());
    assert!(v.get("workspace").is_and_some());
}

/// When no bead is available, claim emits `{}` (format_no_claim) — still valid
/// JSON, never an array.
#[test]
fn claim_empty_emits_empty_object() {
    let ws = init_workspace(); // no beads
    let out = run(
        ws.path(),
        &["claim", "--assignee", "lonely", "--format", "json"],
    );
    assert_eq!(out.trim(), "{}", "no-claim result must be {{}}");
}

// ---------------------------------------------------------------------------
// Object command: stats
// ---------------------------------------------------------------------------

/// `stats --format json` emits a single object with the four counts. With a
/// breakdown flag, the breakdown is folded INTO the object (nested map), keeping
/// the whole stdout a single valid JSON document.
#[test]
fn stats_emits_single_object_with_optional_breakdowns() {
    let ws = init_workspace();
    create_bead(ws.path(), "a", &[]);
    create_bead(ws.path(), "b", &[]);

    let base = run(ws.path(), &["stats", "--format", "json"]);
    let v: Value = serde_json::from_str(base.trim()).expect("stats base is one JSON object");
    assert!(v.is_object());
    assert!(v.get("total").is_and_some());
    assert!(v.get("open").is_and_some());
    assert!(v.get("in_progress").is_and_some());
    assert!(v.get("closed").is_and_some());
    assert!(
        v.get("by_type").is_none(),
        "by_type must be absent without --by-type"
    );

    // With breakdowns folded in, still a single parseable object. Parsing the
    // RAW output as one Value also proves nothing is appended after the object
    // (the old bug class: serde_json::from_str rejects trailing non-JSON text).
    let with_by = run(
        ws.path(),
        &["stats", "--by-type", "--by-priority", "--format", "json"],
    );
    let v: Value = serde_json::from_str(with_by.trim()).expect("stats --by-* is one JSON object");
    assert!(v.get("by_type").is_and_some(), "by_type folded in");
    assert!(v.get("by_priority").is_and_some(), "by_priority folded in");
}

// ---------------------------------------------------------------------------
// Array command: velocity
// ---------------------------------------------------------------------------

/// `velocity --format json` emits a JSON array (via format_velocity). With no
/// claim→close events it is `[]`.
#[test]
fn velocity_emits_json_array() {
    let ws = init_workspace();
    let out = run(ws.path(), &["velocity", "--format", "json"]);
    let v: Value = serde_json::from_str(out.trim()).expect("velocity output is a JSON array");
    assert!(v.is_array(), "velocity must emit a JSON array, got: {out}");
}

// ---------------------------------------------------------------------------
// Empty-result asymmetry (deliberate, documented — locks in current behavior)
// ---------------------------------------------------------------------------

/// Documents the one intentional asymmetry found by the audit. `list` and
/// `search` emit empty stdout for zero results (JSONL: zero lines = zero
/// beads); `ready` emits `[]`, a contract deliberately preserved by bf-64zt.
/// `claim` emits `{}`. This test locks in the CURRENT behavior so any
/// regression is caught; changing it is a deliberate contract decision.
#[test]
fn empty_result_behavior_is_as_documented() {
    let ws = init_workspace(); // no beads

    let list = run(ws.path(), &["list", "--format", "json"]);
    let search = run(ws.path(), &["search", "--format", "json"]);
    let ready = run(ws.path(), &["ready", "--limit", "0", "--format", "json"]);
    let claim = run(ws.path(), &["claim", "--assignee", "x", "--format", "json"]);

    assert!(list.is_empty(), "empty list emits nothing (JSONL: 0 lines)");
    assert!(
        search.is_empty(),
        "empty search emits nothing (JSONL: 0 lines)"
    );
    assert_eq!(
        ready.trim(),
        "[]",
        "empty ready emits [] (bf-64zt contract)"
    );
    assert_eq!(claim.trim(), "{}", "empty claim emits {{}}");
}

// Small helper: Option<&Value> "is present and non-null".
trait OptExt {
    fn is_and_some(&self) -> bool;
}
impl OptExt for Option<&Value> {
    fn is_and_some(&self) -> bool {
        match self {
            Some(Value::Null) => false,
            Some(_) => true,
            None => false,
        }
    }
}
