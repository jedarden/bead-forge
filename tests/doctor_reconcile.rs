//! Regression tests for `bf doctor --reconcile` (bead bf-29wxxl).
//!
//! Two fixes shipped forward-only and never backfilled the rows that predated them:
//!
//!   1. The blocked->open cascade (bf-5id, commit 519449b) only fires on future `close`
//!      events. Every bead whose last blocker closed *before* that fix landed stayed at
//!      `status='blocked'` forever — 39 of them in the live fleet, transitively pinning
//!      everything downstream and emptying `bf ready`.
//!   2. Empty-assignee normalization (bf-4mj7l/bf-2uhsk) only applies on write, so rows
//!      still holding `assignee = ""` read back as assigned (docs/plan/plan.md §3.4).
//!
//! These tests seed exactly those pre-existing states — a blocker closed *without* going
//! through the cascading `close_issue` path, and a literal empty-string assignee — and
//! prove `doctor::reconcile` repairs them, leaves legitimately blocked beads alone, and
//! is idempotent.

mod common;

use bead_forge::doctor;
use bead_forge::model::{DependencyType, Issue, Status};
use chrono::Utc;

/// Seed a bead at an arbitrary status directly, bypassing `close_issue`/`update_issue`.
///
/// This is the whole point of the fixture: a bead closed through `Storage::close_issue`
/// today *would* cascade its dependents to open. Writing the closed row directly is what
/// reproduces the pre-fix history the cascade never saw.
fn seed(ws: &common::TempWorkspace, id: &str, status: Status) {
    let mut issue = Issue::new(id.to_string(), format!("Bead {id}"), ".".to_string());
    issue.status = status;
    if issue.status == Status::Closed {
        // Schema CHECK: a closed row must carry closed_at (bf-4eido).
        issue.closed_at = Some(Utc::now() - chrono::Duration::days(26));
    }
    ws.create_issue(&issue).expect("seed bead");
}

fn block(ws: &common::TempWorkspace, issue_id: &str, depends_on: &str) {
    ws.storage()
        .unwrap()
        .add_dependency(issue_id, depends_on, &DependencyType::Blocks, "test")
        .expect("add dependency");
}

fn status_of(ws: &common::TempWorkspace, id: &str) -> Status {
    ws.get_bead(id).unwrap().expect("bead exists").status
}

/// The live bf-36pil case: a bead whose *only* blocker closed long before the cascade fix
/// existed, so it was never flipped back to open.
#[test]
fn reconcile_reopens_bead_whose_sole_blocker_closed_before_the_cascade_fix() {
    let ws = common::TempWorkspace::new().unwrap();
    seed(&ws, "bf-blocker", Status::Closed);
    seed(&ws, "bf-stuck", Status::Blocked);
    block(&ws, "bf-stuck", "bf-blocker");

    // Precondition: the row really is stuck, and `bf doctor` already sees it.
    assert_eq!(status_of(&ws, "bf-stuck"), Status::Blocked);
    let before = doctor::check(ws.workspace_path()).unwrap();
    assert_eq!(before.stale_blocked_ids, vec!["bf-stuck".to_string()]);

    let report = doctor::reconcile(ws.workspace_path()).unwrap();
    assert_eq!(report.unblocked, vec!["bf-stuck".to_string()]);
    assert_eq!(status_of(&ws, "bf-stuck"), Status::Open);

    // And the doctor check now comes back clean for this class.
    let after = doctor::check(ws.workspace_path()).unwrap();
    assert!(after.stale_blocked_ids.is_empty());
}

/// A bead with a still-open blocker is genuinely blocked and must not be touched.
#[test]
fn reconcile_leaves_beads_with_a_live_blocker_alone() {
    let ws = common::TempWorkspace::new().unwrap();
    seed(&ws, "bf-closed-blocker", Status::Closed);
    seed(&ws, "bf-open-blocker", Status::Open);
    seed(&ws, "bf-really-blocked", Status::Blocked);
    block(&ws, "bf-really-blocked", "bf-closed-blocker");
    block(&ws, "bf-really-blocked", "bf-open-blocker");

    let report = doctor::reconcile(ws.workspace_path()).unwrap();
    assert!(report.unblocked.is_empty());
    assert_eq!(status_of(&ws, "bf-really-blocked"), Status::Blocked);
}

/// A blocked chain unwinds one link per reconcile — the tail's blocker becomes open, which
/// is non-terminal, so the bead above it stays blocked until that one actually closes.
/// (bf-lo3da -> ... -> bf-36pil -> bf-1ioxa in the live data.)
#[test]
fn reconcile_unwinds_only_the_link_whose_blockers_are_terminal() {
    let ws = common::TempWorkspace::new().unwrap();
    seed(&ws, "bf-root-closed", Status::Closed);
    seed(&ws, "bf-mid", Status::Blocked);
    seed(&ws, "bf-top", Status::Blocked);
    block(&ws, "bf-mid", "bf-root-closed");
    block(&ws, "bf-top", "bf-mid");

    let report = doctor::reconcile(ws.workspace_path()).unwrap();
    assert_eq!(report.unblocked, vec!["bf-mid".to_string()]);
    assert_eq!(status_of(&ws, "bf-mid"), Status::Open);
    // bf-top is still blocked by bf-mid, which is now open (non-terminal) — correct.
    assert_eq!(status_of(&ws, "bf-top"), Status::Blocked);
}

/// A bead set to `blocked` by hand with no dependency rows has no blocker state to derive
/// `open` from, so reconcile reports it rather than silently reopening it.
#[test]
fn reconcile_reports_but_does_not_touch_blocked_beads_with_no_dependencies() {
    let ws = common::TempWorkspace::new().unwrap();
    seed(&ws, "bf-manual", Status::Blocked);

    let report = doctor::reconcile(ws.workspace_path()).unwrap();
    assert!(report.unblocked.is_empty());
    assert_eq!(
        report.blocked_without_dependencies,
        vec!["bf-manual".to_string()]
    );
    assert_eq!(status_of(&ws, "bf-manual"), Status::Blocked);
}

/// Legacy `assignee = ""` rows are rewritten to NULL; real assignees are left alone.
#[test]
fn reconcile_normalizes_empty_string_assignees_to_null() {
    let ws = common::TempWorkspace::new().unwrap();
    let mut legacy = Issue::new(
        "bf-legacy".to_string(),
        "Legacy".to_string(),
        ".".to_string(),
    );
    legacy.assignee = Some(String::new());
    ws.create_issue(&legacy).unwrap();

    let mut claimed = Issue::new(
        "bf-claimed".to_string(),
        "Claimed".to_string(),
        ".".to_string(),
    );
    claimed.assignee = Some("worker-1".to_string());
    ws.create_issue(&claimed).unwrap();

    // Precondition: the empty string round-trips as Some("") — indistinguishable from an
    // assignment to any consumer testing `assignee.is_some()`.
    assert_eq!(
        ws.get_bead("bf-legacy").unwrap().unwrap().assignee,
        Some(String::new())
    );
    let before = doctor::check(ws.workspace_path()).unwrap();
    assert_eq!(before.empty_assignee_ids, vec!["bf-legacy".to_string()]);

    let report = doctor::reconcile(ws.workspace_path()).unwrap();
    assert_eq!(report.normalized_assignees, vec!["bf-legacy".to_string()]);
    assert_eq!(ws.get_bead("bf-legacy").unwrap().unwrap().assignee, None);
    assert_eq!(
        ws.get_bead("bf-claimed").unwrap().unwrap().assignee,
        Some("worker-1".to_string())
    );

    let after = doctor::check(ws.workspace_path()).unwrap();
    assert!(after.empty_assignee_ids.is_empty());
}

/// Reconciled rows must reach JSONL, or the next rebuild-from-JSONL resurrects the stale
/// state that was just repaired.
#[test]
fn reconciled_rows_are_marked_dirty_for_the_next_flush() {
    let ws = common::TempWorkspace::new().unwrap();
    seed(&ws, "bf-blocker", Status::Closed);
    seed(&ws, "bf-stuck", Status::Blocked);
    block(&ws, "bf-stuck", "bf-blocker");
    let mut legacy = Issue::new(
        "bf-legacy".to_string(),
        "Legacy".to_string(),
        ".".to_string(),
    );
    legacy.assignee = Some(String::new());
    ws.create_issue(&legacy).unwrap();

    // Flush everything first so the only dirty rows afterwards are reconcile's own.
    ws.export_jsonl(false).unwrap();
    assert!(ws
        .storage()
        .unwrap()
        .list_dirty_issues()
        .unwrap()
        .is_empty());

    doctor::reconcile(ws.workspace_path()).unwrap();

    let dirty: Vec<String> = ws
        .storage()
        .unwrap()
        .list_dirty_issues()
        .unwrap()
        .into_iter()
        .map(|i| i.id)
        .collect();
    assert!(dirty.contains(&"bf-stuck".to_string()), "dirty: {dirty:?}");
    assert!(dirty.contains(&"bf-legacy".to_string()), "dirty: {dirty:?}");
}

/// Running reconcile on an already-reconciled (or healthy) workspace changes nothing.
#[test]
fn reconcile_is_idempotent() {
    let ws = common::TempWorkspace::new().unwrap();
    seed(&ws, "bf-blocker", Status::Closed);
    seed(&ws, "bf-stuck", Status::Blocked);
    block(&ws, "bf-stuck", "bf-blocker");
    let mut legacy = Issue::new(
        "bf-legacy".to_string(),
        "Legacy".to_string(),
        ".".to_string(),
    );
    legacy.assignee = Some(String::new());
    ws.create_issue(&legacy).unwrap();

    let first = doctor::reconcile(ws.workspace_path()).unwrap();
    assert!(!first.is_clean());

    let second = doctor::reconcile(ws.workspace_path()).unwrap();
    assert!(second.is_clean(), "second run changed rows: {second:?}");
    assert!(second.unblocked.is_empty());
    assert!(second.normalized_assignees.is_empty());
    assert_eq!(status_of(&ws, "bf-stuck"), Status::Open);
}

/// A workspace with nothing to fix reports clean.
#[test]
fn reconcile_on_a_healthy_workspace_is_a_no_op() {
    let ws = common::TempWorkspace::new().unwrap();
    ws.create_bead("bf-fine", "Nothing wrong here").unwrap();

    let report = doctor::reconcile(ws.workspace_path()).unwrap();
    assert!(report.is_clean());
    assert!(report.blocked_without_dependencies.is_empty());
    assert_eq!(status_of(&ws, "bf-fine"), Status::Open);
}
