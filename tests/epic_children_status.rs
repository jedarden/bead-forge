//! Epic children and status-computation tests (bf-hfsim).
//!
//! Children are attached to an epic with a `ParentChild` dependency whose
//! `issue_id` is the epic and whose `depends_on_id` is the child -- the same
//! direction used by `tests/epic_comprehensive.rs`. That means the child set of
//! an epic is `get_dependencies(epic_id)` filtered to `ParentChild`.
//!
//! `EpicStatus` is a plain data struct (no computation lives in `src/`), so the
//! roll-up rules under test are the ones the epic suite already encodes:
//!   * `total_children`   -- number of `ParentChild` edges out of the epic,
//!   * `closed_children`  -- children whose status is exactly `Status::Closed`
//!     (`Blocked` and `Deferred` are *not* closed),
//!   * `eligible_for_close` -- every child closed *and* at least one child, so
//!     a childless epic is never auto-eligible.

use bead_forge::model::{DependencyType, EpicStatus, Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;

/// Open a `Storage` backed by a fresh temp dir; the dir is returned so the
/// caller keeps it alive for the duration of the test.
fn temp_storage() -> (tempfile::TempDir, Storage) {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();
    (dir, storage)
}

/// Create an epic and store it.
fn create_epic(storage: &Storage, id: &str, title: &str) -> Issue {
    let epic = Issue {
        id: id.to_string(),
        title: title.to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();
    epic
}

/// Create a child issue and link it to `epic_id` with a `ParentChild` edge.
fn create_child(
    storage: &Storage,
    epic_id: &str,
    id: &str,
    issue_type: IssueType,
    status: Status,
) -> Issue {
    let mut child = Issue {
        id: id.to_string(),
        title: format!("Child {id}"),
        issue_type,
        status: status.clone(),
        priority: Priority::MEDIUM,
        ..Default::default()
    };
    if status == Status::Closed {
        child.closed_at = Some(Utc::now());
    }
    storage.create_issue(&child).unwrap();
    storage
        .add_dependency(epic_id, id, &DependencyType::ParentChild, "test")
        .unwrap();
    child
}

/// Read the epic's children back out of storage, in edge-insertion order.
fn children_of(storage: &Storage, epic_id: &str) -> Vec<Issue> {
    storage
        .get_dependencies(epic_id)
        .unwrap()
        .into_iter()
        .filter(|d| d.dep_type == DependencyType::ParentChild)
        .filter_map(|d| storage.get_issue(&d.depends_on_id).unwrap())
        .collect()
}

/// Roll the epic's children up into an `EpicStatus`.
fn epic_status(storage: &Storage, epic_id: &str) -> EpicStatus {
    let epic = storage.get_issue(epic_id).unwrap().unwrap();
    let children = children_of(storage, epic_id);
    let total_children = children.len();
    let closed_children = children
        .iter()
        .filter(|c| c.status == Status::Closed)
        .count();

    EpicStatus {
        epic,
        total_children,
        closed_children,
        eligible_for_close: total_children > 0 && closed_children == total_children,
    }
}

#[test]
fn test_epic_with_children() {
    let (_dir, storage) = temp_storage();
    create_epic(&storage, "epic-children", "Epic With Children");

    for i in 1..=5 {
        create_child(
            &storage,
            "epic-children",
            &format!("epic-children-c{i}"),
            IssueType::Task,
            Status::Open,
        );
    }

    let status = epic_status(&storage, "epic-children");
    assert_eq!(status.epic.issue_type, IssueType::Epic);
    assert_eq!(status.total_children, 5);
    assert_eq!(status.closed_children, 0);
    assert!(!status.eligible_for_close);

    // Every edge is a parent-child edge pointing at a distinct child.
    let deps = storage.get_dependencies("epic-children").unwrap();
    assert_eq!(deps.len(), 5);
    assert!(deps
        .iter()
        .all(|d| d.dep_type == DependencyType::ParentChild));
    let mut child_ids: Vec<_> = deps.iter().map(|d| d.depends_on_id.clone()).collect();
    child_ids.sort();
    assert_eq!(
        child_ids,
        vec![
            "epic-children-c1",
            "epic-children-c2",
            "epic-children-c3",
            "epic-children-c4",
            "epic-children-c5",
        ]
    );
}

#[test]
fn test_epic_no_children() {
    let (_dir, storage) = temp_storage();
    create_epic(&storage, "epic-empty", "Childless Epic");

    let status = epic_status(&storage, "epic-empty");
    assert_eq!(status.total_children, 0);
    assert_eq!(status.closed_children, 0);
    // A childless epic has nothing to roll up, so it is never auto-eligible.
    assert!(!status.eligible_for_close);
}

#[test]
fn test_epic_mixed_child_types() {
    let (_dir, storage) = temp_storage();
    create_epic(&storage, "epic-mixed", "Mixed Child Types");

    let types = [
        IssueType::Task,
        IssueType::Bug,
        IssueType::Feature,
        IssueType::Chore,
        IssueType::Docs,
    ];
    for (i, issue_type) in types.iter().enumerate() {
        create_child(
            &storage,
            "epic-mixed",
            &format!("epic-mixed-c{i}"),
            issue_type.clone(),
            Status::Open,
        );
    }

    let status = epic_status(&storage, "epic-mixed");
    assert_eq!(status.total_children, 5);
    assert_eq!(status.closed_children, 0);

    // The roll-up is type-blind, but each child keeps its own type.
    let mut stored_types: Vec<_> = children_of(&storage, "epic-mixed")
        .into_iter()
        .map(|c| c.issue_type)
        .collect();
    stored_types.sort_by_key(|t| t.to_string());
    let mut expected = types.to_vec();
    expected.sort_by_key(|t| t.to_string());
    assert_eq!(stored_types, expected);
}

#[test]
fn test_epic_all_open_children() {
    let (_dir, storage) = temp_storage();
    create_epic(&storage, "epic-open", "All Open Children");

    for i in 1..=4 {
        create_child(
            &storage,
            "epic-open",
            &format!("epic-open-c{i}"),
            IssueType::Task,
            Status::Open,
        );
    }

    let status = epic_status(&storage, "epic-open");
    assert_eq!(status.total_children, 4);
    assert_eq!(status.closed_children, 0);
    assert!(!status.eligible_for_close);
    assert!(children_of(&storage, "epic-open")
        .iter()
        .all(|c| c.status == Status::Open && c.closed_at.is_none()));
}

#[test]
fn test_epic_partial_closed() {
    let (_dir, storage) = temp_storage();
    create_epic(&storage, "epic-partial", "Partially Closed Epic");

    for i in 1..=5 {
        let status = if i <= 2 { Status::Closed } else { Status::Open };
        create_child(
            &storage,
            "epic-partial",
            &format!("epic-partial-c{i}"),
            IssueType::Task,
            status,
        );
    }

    let status = epic_status(&storage, "epic-partial");
    assert_eq!(status.total_children, 5);
    assert_eq!(status.closed_children, 2);
    assert!(!status.eligible_for_close);

    let open_children = children_of(&storage, "epic-partial")
        .iter()
        .filter(|c| c.status == Status::Open)
        .count();
    assert_eq!(open_children, 3);
}

#[test]
fn test_epic_all_closed_eligible() {
    let (_dir, storage) = temp_storage();
    create_epic(&storage, "epic-closed", "Fully Closed Epic");

    for i in 1..=3 {
        create_child(
            &storage,
            "epic-closed",
            &format!("epic-closed-c{i}"),
            IssueType::Task,
            Status::Closed,
        );
    }

    let status = epic_status(&storage, "epic-closed");
    assert_eq!(status.total_children, 3);
    assert_eq!(status.closed_children, 3);
    assert!(status.eligible_for_close);
    // The epic itself is still open -- eligibility is a recommendation, not a
    // side effect of closing the last child.
    assert_eq!(status.epic.status, Status::Open);
}

#[test]
fn test_epic_blocked_child() {
    let (_dir, storage) = temp_storage();
    create_epic(&storage, "epic-blocked", "Epic With Blocked Child");

    create_child(
        &storage,
        "epic-blocked",
        "epic-blocked-c1",
        IssueType::Task,
        Status::Blocked,
    );
    create_child(
        &storage,
        "epic-blocked",
        "epic-blocked-c2",
        IssueType::Task,
        Status::Closed,
    );

    let blocked = storage.get_issue("epic-blocked-c1").unwrap().unwrap();
    assert_eq!(blocked.status, Status::Blocked);

    let status = epic_status(&storage, "epic-blocked");
    assert_eq!(status.total_children, 2);
    // Blocked is not closed, so the epic is still incomplete.
    assert_eq!(status.closed_children, 1);
    assert!(!status.eligible_for_close);
}

#[test]
fn test_epic_deferred_child() {
    let (_dir, storage) = temp_storage();
    create_epic(&storage, "epic-deferred", "Epic With Deferred Child");

    create_child(
        &storage,
        "epic-deferred",
        "epic-deferred-c1",
        IssueType::Task,
        Status::Deferred,
    );
    create_child(
        &storage,
        "epic-deferred",
        "epic-deferred-c2",
        IssueType::Task,
        Status::Closed,
    );
    create_child(
        &storage,
        "epic-deferred",
        "epic-deferred-c3",
        IssueType::Task,
        Status::Closed,
    );

    let deferred = storage.get_issue("epic-deferred-c1").unwrap().unwrap();
    assert_eq!(deferred.status, Status::Deferred);
    assert!(deferred.closed_at.is_none());
    assert!(!deferred.status.is_terminal());

    let status = epic_status(&storage, "epic-deferred");
    assert_eq!(status.total_children, 3);
    // Deferred work is postponed, not finished -- it does not count as closed.
    assert_eq!(status.closed_children, 2);
    assert!(!status.eligible_for_close);
}
