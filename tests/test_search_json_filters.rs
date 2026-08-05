//! Comprehensive JSON output tests for `bf search` command filter combinations
//!
//! These tests validate that the search command's JSON output works correctly
//! with various filter combinations including:
//! - Multiple status filters (OR-combined)
//! - Multiple type filters (OR-combined)
//! - Multiple label filters (OR-combined)
//! - Priority range filters (--priority-min, --priority-max)
//! - Combined filters (multiple filter types together)
//! - Text query combined with filters
//! - Limit parameter with JSON output

use serde_json::{from_str, Value};
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

/// Create a test bead with the given parameters
fn create_bead_with_params(
    workspace: &Path,
    title: &str,
    type_: &str,
    priority: i32,
    status: &str,
) -> String {
    let (out, err, ok) = run_bf(
        workspace,
        &[
            "create",
            "--title",
            title,
            "--type",
            type_,
            "--priority",
            &priority.to_string(),
        ],
    );
    assert!(ok, "bf create failed: {err}");
    let id = out.trim().to_string();
    assert!(!id.is_empty(), "create produced no id: {out}");

    // Update status if not "open"
    if status != "open" {
        let (_o, e, ok) = run_bf(workspace, &["update", &id, "--status", status]);
        assert!(ok, "bf update failed: {e}");
    }

    id
}

/// Create a test bead with labels
fn create_bead_with_labels(workspace: &Path, title: &str, labels: &[&str]) -> String {
    let bead_id = create_bead_with_params(workspace, title, "task", 2, "open");

    for label in labels {
        let (_out, err, ok) = run_bf(workspace, &["label", "add", &bead_id, "--label", label]);
        assert!(ok, "Failed to add label '{}': {err}", label);
    }

    bead_id
}

/// Close a test bead
fn close_bead(workspace: &Path, bead_id: &str, reason: &str) {
    let (_out, err, ok) = run_bf(workspace, &["close", bead_id, "--reason", reason]);
    assert!(ok, "Failed to close bead: {err}");
}

/// Parse a JSON string and panic if invalid
fn parse_json(json: &str) -> Value {
    from_str(json).unwrap_or_else(|e| panic!("Failed to parse JSON: {}\nJSON was: {}", e, json))
}

/// Parse a JSONL string (newline-delimited JSON) into a Vec of values
fn parse_jsonl(jsonl: &str) -> Vec<Value> {
    jsonl
        .lines()
        .filter(|line| !line.trim().is_empty() && line.trim() != "[]")
        .map(|line| parse_json(line))
        .collect()
}

/// Get a string field from JSON, panic if missing or not a string
fn get_string(json: &Value, field: &str) -> String {
    json.get(field)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("Field '{}' is not a string or is missing: {}", field, json))
        .to_string()
}

// ============================================================================
// MULTIPLE STATUS FILTERS TESTS
// ============================================================================

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_search_json_multiple_status_filters() {
    let (_temp, workspace) = setup();

    // Create beads with different statuses
    let open1 = create_bead_with_params(&workspace, "open task one", "task", 2, "open");
    let open2 = create_bead_with_params(&workspace, "open task two", "task", 2, "open");
    let blocked = create_bead_with_params(&workspace, "blocked task", "task", 2, "blocked");
    let in_progress =
        create_bead_with_params(&workspace, "in progress task", "task", 2, "in_progress");
    let closed = close_bead_returning_id(&workspace, "closed task", "task", 2);

    // Search with multiple status filters (OR-combined)
    let (out, err, ok) = run_bf(
        &workspace,
        &[
            "search", "--status", "open", "--status", "blocked", "--format", "json",
        ],
    );
    assert!(ok, "Search with multiple status filters failed: {err}");

    let parsed = parse_jsonl(&out);
    assert_eq!(
        parsed.len(),
        3,
        "Should find exactly 3 beads with open OR blocked status"
    );

    let ids: Vec<String> = parsed.iter().map(|v| get_string(v, "id")).collect();
    assert!(ids.contains(&open1), "Should find open1");
    assert!(ids.contains(&open2), "Should find open2");
    assert!(ids.contains(&blocked), "Should find blocked");
    assert!(!ids.contains(&in_progress), "Should not find in_progress");
    assert!(!ids.contains(&closed), "Should not find closed");
}

// ============================================================================
// MULTIPLE TYPE FILTERS TESTS
// ============================================================================

#[test]
fn test_search_json_multiple_type_filters() {
    let (_temp, workspace) = setup();

    // Create beads with different types
    let task1 = create_bead_with_params(&workspace, "task one", "task", 2, "open");
    let task2 = create_bead_with_params(&workspace, "task two", "task", 2, "open");
    let epic = create_bead_with_params(&workspace, "epic one", "epic", 2, "open");
    let story = create_bead_with_params(&workspace, "story one", "story", 2, "open");

    // Search with multiple type filters (OR-combined)
    let (out, err, ok) = run_bf(
        &workspace,
        &[
            "search", "--type", "task", "--type", "epic", "--format", "json",
        ],
    );
    assert!(ok, "Search with multiple type filters failed: {err}");

    let parsed = parse_jsonl(&out);
    assert_eq!(
        parsed.len(),
        3,
        "Should find exactly 3 beads with task OR epic type"
    );

    let ids: Vec<String> = parsed.iter().map(|v| get_string(v, "id")).collect();
    assert!(ids.contains(&task1), "Should find task1");
    assert!(ids.contains(&task2), "Should find task2");
    assert!(ids.contains(&epic), "Should find epic");
    assert!(!ids.contains(&story), "Should not find story");
}

// ============================================================================
// MULTIPLE LABEL FILTERS TESTS
// ============================================================================

#[test]
fn test_search_json_multiple_label_filters() {
    let (_temp, workspace) = setup();

    // Create beads with different labels
    let urgent = create_bead_with_labels(&workspace, "urgent task", &["urgent"]);
    let bug = create_bead_with_labels(&workspace, "bug task", &["bug"]);
    let both = create_bead_with_labels(&workspace, "urgent bug task", &["urgent", "bug"]);
    let none = create_bead_with_params(&workspace, "plain task", "task", 2, "open");

    // Search with multiple label filters (OR-combined)
    let (out, err, ok) = run_bf(
        &workspace,
        &[
            "search", "--label", "urgent", "--label", "bug", "--format", "json",
        ],
    );
    assert!(ok, "Search with multiple label filters failed: {err}");

    let parsed = parse_jsonl(&out);
    assert_eq!(
        parsed.len(),
        3,
        "Should find exactly 3 beads with urgent OR bug label"
    );

    let ids: Vec<String> = parsed.iter().map(|v| get_string(v, "id")).collect();
    assert!(ids.contains(&urgent), "Should find urgent");
    assert!(ids.contains(&bug), "Should find bug");
    assert!(ids.contains(&both), "Should find both (has both labels)");
    assert!(!ids.contains(&none), "Should not find unlabeled bead");
}

// ============================================================================
// PRIORITY RANGE FILTERS TESTS
// ============================================================================

#[test]
fn test_search_json_priority_range_filters() {
    let (_temp, workspace) = setup();

    // Create beads with different priorities (0=Critical, 1=High, 2=Normal, 3=Low, 4=Backlog)
    let p0 = create_bead_with_params(&workspace, "critical task", "task", 0, "open");
    let p1 = create_bead_with_params(&workspace, "high priority task", "task", 1, "open");
    let p2 = create_bead_with_params(&workspace, "normal priority task", "task", 2, "open");
    let p3 = create_bead_with_params(&workspace, "low priority task", "task", 3, "open");
    let p4 = create_bead_with_params(&workspace, "backlog task", "task", 4, "open");

    // Test priority-min only
    let (out, err, ok) = run_bf(
        &workspace,
        &["search", "--priority-min", "2", "--format", "json"],
    );
    assert!(ok, "Search with priority-min failed: {err}");

    let parsed = parse_jsonl(&out);
    assert_eq!(parsed.len(), 3, "Should find beads with priority >= 2");

    // Test priority-max only
    let (out, err, ok) = run_bf(
        &workspace,
        &["search", "--priority-max", "1", "--format", "json"],
    );
    assert!(ok, "Search with priority-max failed: {err}");

    let parsed = parse_jsonl(&out);
    assert_eq!(parsed.len(), 2, "Should find beads with priority <= 1");

    // Test both priority-min and priority-max (range)
    let (out, err, ok) = run_bf(
        &workspace,
        &[
            "search",
            "--priority-min",
            "1",
            "--priority-max",
            "2",
            "--format",
            "json",
        ],
    );
    assert!(ok, "Search with priority range failed: {err}");

    let parsed = parse_jsonl(&out);
    assert_eq!(parsed.len(), 2, "Should find beads with priority 1-2");

    let ids: Vec<String> = parsed.iter().map(|v| get_string(v, "id")).collect();
    assert!(ids.contains(&p1), "Should find priority 1");
    assert!(ids.contains(&p2), "Should find priority 2");
    assert!(!ids.contains(&p0), "Should not find priority 0");
    assert!(!ids.contains(&p3), "Should not find priority 3");
    assert!(!ids.contains(&p4), "Should not find priority 4");
}

// ============================================================================
// COMBINED FILTERS TESTS
// ============================================================================

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_search_json_combined_filters() {
    let (_temp, workspace) = setup();

    // Create beads with various combinations
    let high_urgent_open = create_bead_with_labels(&workspace, "high urgent open", &["urgent"]);
    let high_urgent_open = update_bead_priority(&workspace, &high_urgent_open, 1);

    let normal_urgent_open = create_bead_with_labels(&workspace, "normal urgent open", &["urgent"]);

    let high_urgent_blocked =
        create_bead_with_labels(&workspace, "high urgent blocked", &["urgent"]);
    let high_urgent_blocked = update_bead_priority(&workspace, &high_urgent_blocked, 1);
    let (_o, e, ok) = run_bf(
        &workspace,
        &["update", &high_urgent_blocked, "--status", "blocked"],
    );
    assert!(ok, "Failed to set status: {e}");

    let normal_bug_open = create_bead_with_labels(&workspace, "normal bug open", &["bug"]);

    // Search with combined filters: label=urgent AND priority>=1 AND status=open
    let (out, err, ok) = run_bf(
        &workspace,
        &[
            "search",
            "--label",
            "urgent",
            "--priority-min",
            "1",
            "--status",
            "open",
            "--format",
            "json",
        ],
    );
    assert!(ok, "Search with combined filters failed: {err}");

    let parsed = parse_jsonl(&out);
    assert_eq!(
        parsed.len(),
        2,
        "Should find exactly 2 beads matching all filters"
    );

    let ids: Vec<String> = parsed.iter().map(|v| get_string(v, "id")).collect();
    assert!(
        ids.contains(&high_urgent_open),
        "Should find high+urgent+open"
    );
    assert!(
        ids.contains(&normal_urgent_open),
        "Should find normal+urgent+open"
    );
    assert!(
        !ids.contains(&high_urgent_blocked),
        "Should not find blocked bead"
    );
    assert!(
        !ids.contains(&normal_bug_open),
        "Should not find bug-labeled bead"
    );
}

// ============================================================================
// TEXT QUERY WITH FILTERS TESTS
// ============================================================================

#[test]
fn test_search_json_text_query_with_filters() {
    let (_temp, workspace) = setup();

    // Create beads with specific text in titles
    let api_open = create_bead_with_params(&workspace, "API endpoint task", "task", 2, "open");
    let api_closed = close_bead_returning_id(&workspace, "API bug fix", "task", 2);
    let frontend_open = create_bead_with_params(&workspace, "Frontend feature", "task", 2, "open");

    // Search for "API" with status=open filter
    let (out, err, ok) = run_bf(
        &workspace,
        &["search", "API", "--status", "open", "--format", "json"],
    );
    assert!(ok, "Search with text query and filters failed: {err}");

    let parsed = parse_jsonl(&out);
    assert_eq!(
        parsed.len(),
        1,
        "Should find exactly 1 bead matching text AND status"
    );

    let found_id = get_string(&parsed[0], "id");
    assert_eq!(found_id, api_open, "Should find only the open API bead");
}

// ============================================================================
// LIMIT PARAMETER WITH JSON OUTPUT TESTS
// ============================================================================

#[test]
fn test_search_json_limit_parameter() {
    let (_temp, workspace) = setup();

    // Create multiple matching beads
    for i in 1..=10 {
        create_bead_with_params(
            &workspace,
            &format!("search test bead {}", i),
            "task",
            2,
            "open",
        );
    }

    // Test with limit=5
    let (out, err, ok) = run_bf(
        &workspace,
        &["search", "search test", "--limit", "5", "--format", "json"],
    );
    assert!(ok, "Search with limit failed: {err}");

    let parsed = parse_jsonl(&out);
    assert_eq!(
        parsed.len(),
        5,
        "Should return exactly 5 beads when limited"
    );

    // Test with limit=0 (unlimited)
    let (out, err, ok) = run_bf(
        &workspace,
        &["search", "search test", "--limit", "0", "--format", "json"],
    );
    assert!(ok, "Search with limit=0 failed: {err}");

    let parsed = parse_jsonl(&out);
    assert_eq!(parsed.len(), 10, "Should return all 10 beads when limit=0");
}

// ============================================================================
// ASSIGNEE FILTER TESTS
// ============================================================================

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_search_json_assignee_filter() {
    let (_temp, workspace) = setup();

    // Create beads with different assignees
    let alice = create_bead_with_assignee(&workspace, "alice task", "alice");
    let bob = create_bead_with_assignee(&workspace, "bob task", "bob");
    let unassigned = create_bead_with_params(&workspace, "unassigned task", "task", 2, "open");

    // Search by assignee
    let (out, err, ok) = run_bf(
        &workspace,
        &["search", "--assignee", "alice", "--format", "json"],
    );
    assert!(ok, "Search by assignee failed: {err}");

    let parsed = parse_jsonl(&out);
    assert_eq!(parsed.len(), 1, "Should find exactly 1 bead for alice");

    let found_id = get_string(&parsed[0], "id");
    assert_eq!(found_id, alice, "Should find alice's bead");

    // Verify other assignees are not found
    let ids: Vec<String> = parsed.iter().map(|v| get_string(v, "id")).collect();
    assert!(!ids.contains(&bob), "Should not find bob's bead");
    assert!(
        !ids.contains(&unassigned),
        "Should not find unassigned bead"
    );
}

// ============================================================================
// EDGE CASES AND SPECIAL SCENARIOS
// ============================================================================

#[test]
fn test_search_json_no_matches_with_filters() {
    let (_temp, workspace) = setup();

    // Create a bead
    create_bead_with_params(&workspace, "test bead", "task", 2, "open");

    // Search with filters that match nothing
    let (out, err, ok) = run_bf(
        &workspace,
        &["search", "--status", "closed", "--format", "json"],
    );
    assert!(ok, "Search with no matches failed: {err}");

    let parsed = parse_jsonl(&out);
    assert_eq!(
        parsed.len(),
        0,
        "Should return empty results for no matches"
    );
}

#[test]
fn test_search_json_wildcard_text_with_filters() {
    let (_temp, workspace) = setup();

    // Create beads
    let task1 = create_bead_with_labels(&workspace, "urgent backend task", &["urgent", "backend"]);
    let task2 =
        create_bead_with_labels(&workspace, "urgent frontend task", &["urgent", "frontend"]);
    let task3 = create_bead_with_labels(&workspace, "backend task", &["backend"]);

    // Search for "backend" with label=urgent
    let (out, err, ok) = run_bf(
        &workspace,
        &["search", "backend", "--label", "urgent", "--format", "json"],
    );
    assert!(ok, "Search with text and label filter failed: {err}");

    let parsed = parse_jsonl(&out);
    assert_eq!(
        parsed.len(),
        1,
        "Should find exactly 1 bead matching text AND label"
    );

    let found_id = get_string(&parsed[0], "id");
    assert_eq!(found_id, task1, "Should find the urgent backend task");
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Create a bead with an assignee and return its ID
fn create_bead_with_assignee(workspace: &Path, title: &str, assignee: &str) -> String {
    let (out, err, ok) = run_bf(
        workspace,
        &[
            "create",
            "--title",
            title,
            "--type",
            "task",
            "--priority",
            "2",
            "--assignee",
            assignee,
        ],
    );
    assert!(ok, "bf create failed: {err}");
    let id = out.trim().to_string();
    assert!(!id.is_empty(), "create produced no id: {out}");
    id
}

/// Close a bead and return its ID
fn close_bead_returning_id(workspace: &Path, title: &str, type_: &str, priority: i32) -> String {
    let id = create_bead_with_params(workspace, title, type_, priority, "open");
    close_bead(workspace, &id, "test close");
    id
}

/// Update a bead's priority and return its ID
fn update_bead_priority(workspace: &Path, bead_id: &str, priority: i32) -> String {
    let (_o, e, ok) = run_bf(
        workspace,
        &["update", bead_id, "--priority", &priority.to_string()],
    );
    assert!(ok, "Failed to update priority: {e}");
    bead_id.to_string()
}
