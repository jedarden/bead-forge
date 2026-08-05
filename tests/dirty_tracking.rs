//! Integration tests for `dirty_issues` tracking (bf-3ilem, plan §7.1).
//!
//! Every mutation command must mark the issue(s) it changes dirty **inside the
//! same transaction** as the mutation, so the dirty mark and the change commit
//! atomically. The next flush (`bf sync --flush-only` / auto-flush) then exports
//! exactly the beads that changed. These tests exercise each mutation path via
//! the library API and assert — by querying the `dirty_issues` table directly
//! with an independent SQLite connection — that the affected bead_id is present.
//!
//! They also guard the inverse invariant: read-only commands must never write
//! `dirty_issues`.

use bead_forge::claim::claim;
use bead_forge::model::{DependencyType, Issue, IssueChanges, Status};
use bead_forge::storage::Storage;
use chrono::Utc;
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::Path;
use tempfile::NamedTempFile;

/// Query the `dirty_issues` table directly (independent read connection) so the
/// assertion does not depend on any library accessor's join/filter behavior.
fn dirty_ids(db_path: &Path) -> HashSet<String> {
    let conn = Connection::open(db_path).unwrap();
    let mut stmt = conn.prepare("SELECT issue_id FROM dirty_issues").unwrap();
    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    ids
}

/// Fresh storage over a temp SQLite file. The `NamedTempFile` guard must be kept
/// alive for the duration of the test (its Drop deletes the backing file).
fn setup() -> (NamedTempFile, Storage) {
    let temp = NamedTempFile::new().unwrap();
    let storage = Storage::open(temp.path()).unwrap();
    (temp, storage)
}

fn make_issue(id: &str) -> Issue {
    Issue::new(id.to_string(), format!("Title for {id}"), ".".to_string())
}

/// Clear dirty marks so a subsequent mutation's contribution is unambiguous.
fn reset_dirty(storage: &Storage) {
    storage.clear_dirty().unwrap();
}

#[test]
fn create_marks_dirty() {
    let (temp, storage) = setup();
    storage.create_issue(&make_issue("bf-create1")).unwrap();
    assert!(dirty_ids(temp.path()).contains("bf-create1"));
}

#[test]
fn update_status_marks_dirty() {
    let (temp, storage) = setup();
    storage.create_issue(&make_issue("bf-upd-status")).unwrap();
    reset_dirty(&storage);

    let changes = IssueChanges {
        status: Some(Status::InProgress),
        ..Default::default()
    };
    storage.update_issue("bf-upd-status", &changes).unwrap();
    assert!(dirty_ids(temp.path()).contains("bf-upd-status"));
}

#[test]
fn update_priority_marks_dirty() {
    let (temp, storage) = setup();
    storage.create_issue(&make_issue("bf-upd-prio")).unwrap();
    reset_dirty(&storage);

    let changes = IssueChanges {
        priority: Some(1),
        ..Default::default()
    };
    storage.update_issue("bf-upd-prio", &changes).unwrap();
    assert!(dirty_ids(temp.path()).contains("bf-upd-prio"));
}

#[test]
fn close_marks_dirty() {
    let (temp, storage) = setup();
    storage.create_issue(&make_issue("bf-close1")).unwrap();
    reset_dirty(&storage);

    storage.close_issue("bf-close1", "done", "cli").unwrap();
    assert!(dirty_ids(temp.path()).contains("bf-close1"));
}

#[test]
fn comment_marks_dirty() {
    let (temp, storage) = setup();
    storage.create_issue(&make_issue("bf-comment1")).unwrap();
    reset_dirty(&storage);

    storage.add_comment("bf-comment1", "cli", "a note").unwrap();
    assert!(dirty_ids(temp.path()).contains("bf-comment1"));
}

#[test]
fn label_add_marks_dirty() {
    let (temp, storage) = setup();
    storage.create_issue(&make_issue("bf-label1")).unwrap();
    reset_dirty(&storage);

    storage.add_label("bf-label1", "urgent").unwrap();
    assert!(dirty_ids(temp.path()).contains("bf-label1"));
}

#[test]
fn label_remove_marks_dirty() {
    let (temp, storage) = setup();
    storage.create_issue(&make_issue("bf-label2")).unwrap();
    storage.add_label("bf-label2", "urgent").unwrap();
    reset_dirty(&storage);

    storage.remove_label("bf-label2", "urgent").unwrap();
    assert!(dirty_ids(temp.path()).contains("bf-label2"));
}

#[test]
fn dep_add_marks_dirty() {
    let (temp, storage) = setup();
    storage.create_issue(&make_issue("bf-dep-a")).unwrap();
    storage.create_issue(&make_issue("bf-dep-b")).unwrap();
    reset_dirty(&storage);

    storage
        .add_dependency("bf-dep-a", "bf-dep-b", &DependencyType::Blocks, "cli")
        .unwrap();
    // The dependency lives on bf-dep-a's record and is exported with it.
    assert!(dirty_ids(temp.path()).contains("bf-dep-a"));
}

#[test]
fn dep_remove_marks_dirty() {
    let (temp, storage) = setup();
    storage.create_issue(&make_issue("bf-dep-c")).unwrap();
    storage.create_issue(&make_issue("bf-dep-d")).unwrap();
    storage
        .add_dependency("bf-dep-c", "bf-dep-d", &DependencyType::Blocks, "cli")
        .unwrap();
    reset_dirty(&storage);

    storage.remove_dependency("bf-dep-c", "bf-dep-d").unwrap();
    assert!(dirty_ids(temp.path()).contains("bf-dep-c"));
}

#[test]
fn annotation_set_and_remove_mark_dirty() {
    let (temp, storage) = setup();
    storage.create_issue(&make_issue("bf-anno1")).unwrap();
    reset_dirty(&storage);

    storage.set_annotation("bf-anno1", "k", "v").unwrap();
    assert!(dirty_ids(temp.path()).contains("bf-anno1"));

    reset_dirty(&storage);
    storage.remove_annotation("bf-anno1", "k").unwrap();
    assert!(dirty_ids(temp.path()).contains("bf-anno1"));
}

#[test]
fn claim_marks_dirty() {
    let (temp, storage) = setup();
    storage.create_issue(&make_issue("bf-claim1")).unwrap();
    reset_dirty(&storage);

    let result = storage
        .with_immediate_transaction(|tx| claim(tx, "worker1", 30, Utc::now(), None))
        .unwrap();
    assert!(result.is_some(), "expected an open bead to claim");
    assert!(dirty_ids(temp.path()).contains("bf-claim1"));
}

/// Read-only commands must not write `dirty_issues`.
#[test]
fn read_only_commands_do_not_mark_dirty() {
    let (temp, storage) = setup();
    storage.create_issue(&make_issue("bf-ro1")).unwrap();
    storage.add_label("bf-ro1", "x").unwrap();
    storage.add_comment("bf-ro1", "cli", "hi").unwrap();
    reset_dirty(&storage);

    // A representative sweep of read paths.
    let _ = storage.get_issue("bf-ro1").unwrap();
    let _ = storage
        .list_issues(&bead_forge::model::IssueFilter::default())
        .unwrap();
    let _ = storage.get_labels("bf-ro1").unwrap();
    let _ = storage.list_comments("bf-ro1").unwrap();
    let _ = storage.get_dependencies("bf-ro1").unwrap();
    let _ = storage.get_annotations("bf-ro1").unwrap();
    let _ = storage.count_issues().unwrap();
    let _ = storage.get_stats().unwrap();

    assert!(
        dirty_ids(temp.path()).is_empty(),
        "read-only commands must not write dirty_issues, found: {:?}",
        dirty_ids(temp.path())
    );
}
