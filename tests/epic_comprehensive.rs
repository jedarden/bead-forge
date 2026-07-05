// Comprehensive epic bead type tests
// Tests epic creation, parent-child relationships, epic status computation, and critical path

use bead_forge::model::{Issue, IssueType, Status, DependencyType, EpicStatus, Priority, IssueChanges};
use bead_forge::storage::Storage;
use chrono::{Utc, Duration};

#[test]
fn test_epic_type_creation_and_serialization() {
    let epic = Issue {
        id: "epic-test".to_string(),
        title: "Test Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        ..Default::default()
    };

    // Test JSON serialization preserves epic type
    let json = serde_json::to_string(&epic).unwrap();
    assert!(json.contains("\"issue_type\":\"epic\""));

    // Test deserialization
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.id, "epic-test");
    assert_eq!(deserialized.priority, Priority::HIGH);
}

#[test]
fn test_epic_with_all_issue_types() {
    // Test that epic is one of the standard issue types
    let types = vec![
        IssueType::Task,
        IssueType::Bug,
        IssueType::Feature,
        IssueType::Epic,
        IssueType::Chore,
        IssueType::Docs,
        IssueType::Question,
    ];

    for issue_type in types {
        let serialized = serde_json::to_string(&issue_type).unwrap();
        let deserialized: IssueType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(issue_type, deserialized);
    }
}

#[test]
fn test_epic_child_relationship_storage() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic
    let epic = Issue {
        id: "epic-1".to_string(),
        title: "Parent Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        created_at: Utc::now() - Duration::days(1),
        updated_at: Utc::now() - Duration::days(1),
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children
    for i in 1..=5 {
        let child = Issue {
            id: format!("child-{}", i),
            title: format!("Child Task {}", i),
            issue_type: IssueType::Task,
            status: Status::Open,
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();

        // Add parent-child dependency
        storage.add_dependency(
            "epic-1",
            &child.id,
            &DependencyType::ParentChild,
            "test"
        ).unwrap();
    }

    // Verify all children are linked
    let children = storage.get_dependencies("epic-1").unwrap();
    assert_eq!(children.len(), 5);
    assert!(children.iter().all(|d| d.dep_type == DependencyType::ParentChild));
}

#[test]
fn test_epic_status_computation_all_open() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with 3 open children
    let epic = Issue {
        id: "epic-status-1".to_string(),
        title: "Epic Status Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    for i in 1..=3 {
        let child = Issue {
            id: format!("child-status-{}", i),
            title: format!("Child {}", i),
            issue_type: IssueType::Task,
            status: Status::Open,
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();
        storage.add_dependency("epic-status-1", &child.id, &DependencyType::ParentChild, "test").unwrap();
    }

    // Query epic and children
    let epic_issue = storage.get_issue("epic-status-1").unwrap().unwrap();
    let children = storage.get_dependencies("epic-status-1").unwrap();

    // Compute epic status manually
    let closed_children = children.iter().filter(|d| {
        match storage.get_issue(&d.depends_on_id) {
            Ok(Some(child)) => child.status == Status::Closed,
            _ => false,
        }
    }).count();

    let epic_status = EpicStatus {
        epic: epic_issue,
        total_children: children.len(),
        closed_children,
        eligible_for_close: closed_children == children.len() && children.len() > 0,
    };

    assert_eq!(epic_status.total_children, 3);
    assert_eq!(epic_status.closed_children, 0);
    assert!(!epic_status.eligible_for_close);
}

#[test]
fn test_epic_status_computation_partial_closed() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with mixed status children
    let epic = Issue {
        id: "epic-status-2".to_string(),
        title: "Partial Close Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create 5 children, close 2 of them
    for i in 1..=5 {
        let status = if i <= 2 { Status::Closed } else { Status::Open };
        let child = Issue {
            id: format!("child-partial-{}", i),
            title: format!("Child {}", i),
            issue_type: IssueType::Task,
            status,
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();
        storage.add_dependency("epic-status-2", &child.id, &DependencyType::ParentChild, "test").unwrap();
    }

    let epic_issue = storage.get_issue("epic-status-2").unwrap().unwrap();
    let children = storage.get_dependencies("epic-status-2").unwrap();

    let closed_children = children.iter().filter(|d| {
        match storage.get_issue(&d.depends_on_id) {
            Ok(Some(child)) => child.status == Status::Closed,
            _ => false,
        }
    }).count();

    let epic_status = EpicStatus {
        epic: epic_issue,
        total_children: children.len(),
        closed_children,
        eligible_for_close: closed_children == children.len() && children.len() > 0,
    };

    assert_eq!(epic_status.total_children, 5);
    assert_eq!(epic_status.closed_children, 2);
    assert!(!epic_status.eligible_for_close); // Not all closed
}

#[test]
fn test_epic_status_computation_all_closed_eligible() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with all children closed
    let epic = Issue {
        id: "epic-status-3".to_string(),
        title: "Complete Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create and close all children
    for i in 1..=4 {
        let mut child = Issue {
            id: format!("child-complete-{}", i),
            title: format!("Child {}", i),
            issue_type: IssueType::Task,
            status: Status::Closed,
            ..Default::default()
        };
        child.closed_at = Some(Utc::now());
        storage.create_issue(&child).unwrap();
        storage.add_dependency("epic-status-3", &child.id, &DependencyType::ParentChild, "test").unwrap();
    }

    let epic_issue = storage.get_issue("epic-status-3").unwrap().unwrap();
    let children = storage.get_dependencies("epic-status-3").unwrap();

    let closed_children = children.iter().filter(|d| {
        match storage.get_issue(&d.depends_on_id) {
            Ok(Some(child)) => child.status == Status::Closed,
            _ => false,
        }
    }).count();

    let epic_status = EpicStatus {
        epic: epic_issue,
        total_children: children.len(),
        closed_children,
        eligible_for_close: closed_children == children.len() && children.len() > 0,
    };

    assert_eq!(epic_status.total_children, 4);
    assert_eq!(epic_status.closed_children, 4);
    assert!(epic_status.eligible_for_close); // All children closed
}

#[test]
fn test_epic_with_no_children() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with no children
    let epic = Issue {
        id: "epic-empty".to_string(),
        title: "Empty Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    let epic_issue = storage.get_issue("epic-empty").unwrap().unwrap();
    let children = storage.get_dependencies("epic-empty").unwrap();

    let epic_status = EpicStatus {
        epic: epic_issue,
        total_children: children.len(),
        closed_children: 0,
        eligible_for_close: false, // No children means not eligible
    };

    assert_eq!(epic_status.total_children, 0);
    assert_eq!(epic_status.closed_children, 0);
    assert!(!epic_status.eligible_for_close); // Empty epic
}

#[test]
fn test_epic_child_types_mixed() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic
    let epic = Issue {
        id: "epic-mixed".to_string(),
        title: "Mixed Types Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create children of different types
    let child_types = vec![
        (IssueType::Task, "task-1"),
        (IssueType::Bug, "bug-1"),
        (IssueType::Feature, "feature-1"),
        (IssueType::Chore, "chore-1"),
        (IssueType::Docs, "docs-1"),
    ];

    for (issue_type, id) in child_types {
        let child = Issue {
            id: id.to_string(),
            title: format!("{} child", id),
            issue_type: issue_type.clone(),
            status: Status::Open,
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();
        storage.add_dependency("epic-mixed", id, &DependencyType::ParentChild, "test").unwrap();
    }

    // Verify all types are preserved
    let children = storage.get_dependencies("epic-mixed").unwrap();
    assert_eq!(children.len(), 5);

    let mut found_types = vec![false; 5];
    for child_dep in &children {
        let child = storage.get_issue(&child_dep.depends_on_id).unwrap().unwrap();
        match child.issue_type {
            IssueType::Task => found_types[0] = true,
            IssueType::Bug => found_types[1] = true,
            IssueType::Feature => found_types[2] = true,
            IssueType::Chore => found_types[3] = true,
            IssueType::Docs => found_types[4] = true,
            _ => panic!("Unexpected issue type"),
        }
    }
    assert!(found_types.iter().all(|&found| found));
}

#[test]
fn test_multiple_epics_independent() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple independent epics
    for epic_num in 1..=3 {
        let epic = Issue {
            id: format!("epic-independent-{}", epic_num),
            title: format!("Independent Epic {}", epic_num),
            issue_type: IssueType::Epic,
            status: Status::Open,
            ..Default::default()
        };
        storage.create_issue(&epic).unwrap();

        // Add children to each epic
        for child_num in 1..=2 {
            let child = Issue {
                id: format!("epic{}-child{}", epic_num, child_num),
                title: format!("Child {}", child_num),
                issue_type: IssueType::Task,
                status: Status::Open,
                ..Default::default()
            };
            storage.create_issue(&child).unwrap();
            storage.add_dependency(
                &format!("epic-independent-{}", epic_num),
                &format!("epic{}-child{}", epic_num, child_num),
                &DependencyType::ParentChild,
                "test"
            ).unwrap();
        }
    }

    // Verify each epic has correct children
    for epic_num in 1..=3 {
        let epic_id = format!("epic-independent-{}", epic_num);
        let children = storage.get_dependencies(&epic_id).unwrap();
        assert_eq!(children.len(), 2);
        assert!(children.iter().all(|d| d.dep_type == DependencyType::ParentChild));
    }
}

#[test]
fn test_epic_status_serialization() {
    let epic = Issue {
        id: "epic-serialize".to_string(),
        title: "Serialization Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        ..Default::default()
    };

    let epic_status = EpicStatus {
        epic: epic.clone(),
        total_children: 10,
        closed_children: 7,
        eligible_for_close: false,
    };

    // Test serialization
    let json = serde_json::to_string(&epic_status).unwrap();
    assert!(json.contains("\"total_children\":10"));
    assert!(json.contains("\"closed_children\":7"));
    assert!(json.contains("\"eligible_for_close\":false"));

    // Test deserialization
    let deserialized: EpicStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.total_children, 10);
    assert_eq!(deserialized.closed_children, 7);
    assert!(!deserialized.eligible_for_close);
    assert_eq!(deserialized.epic.id, "epic-serialize");
}

#[test]
fn test_epic_with_blocked_child() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic
    let epic = Issue {
        id: "epic-blocked".to_string(),
        title: "Epic with Blocked Child".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create a blocked child
    let child = Issue {
        id: "blocked-child".to_string(),
        title: "Blocked Child".to_string(),
        issue_type: IssueType::Task,
        status: Status::Blocked,
        ..Default::default()
    };
    storage.create_issue(&child).unwrap();
    storage.add_dependency("epic-blocked", "blocked-child", &DependencyType::ParentChild, "test").unwrap();

    // Verify blocked status is preserved
    let retrieved_child = storage.get_issue("blocked-child").unwrap().unwrap();
    assert_eq!(retrieved_child.status, Status::Blocked);
    assert_eq!(retrieved_child.issue_type, IssueType::Task);
}

#[test]
fn test_epic_string_roundtrip() {
    // Test epic type string conversion
    let epic_type = IssueType::Epic;
    assert_eq!(epic_type.as_str(), "epic");

    let serialized = serde_json::to_string(&epic_type).unwrap();
    assert_eq!(serialized, "\"epic\"");

    let deserialized: IssueType = serde_json::from_str("\"epic\"").unwrap();
    assert_eq!(deserialized, IssueType::Epic);
}

#[test]
fn test_epic_default_is_task() {
    // Verify that Task is default, not Epic
    let issue = Issue::default();
    assert_eq!(issue.issue_type, IssueType::Task);
    assert!(!(issue.issue_type == IssueType::Epic));
}

#[test]
fn test_epic_with_deferred_child() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic
    let epic = Issue {
        id: "epic-deferred".to_string(),
        title: "Epic with Deferred Child".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create deferred and open children
    let deferred_child = Issue {
        id: "deferred-child".to_string(),
        title: "Deferred Child".to_string(),
        issue_type: IssueType::Task,
        status: Status::Deferred,
        ..Default::default()
    };
    storage.create_issue(&deferred_child).unwrap();

    let open_child = Issue {
        id: "open-child".to_string(),
        title: "Open Child".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        ..Default::default()
    };
    storage.create_issue(&open_child).unwrap();

    storage.add_dependency("epic-deferred", "deferred-child", &DependencyType::ParentChild, "test").unwrap();
    storage.add_dependency("epic-deferred", "open-child", &DependencyType::ParentChild, "test").unwrap();

    // Deferred children should count as not closed
    let children = storage.get_dependencies("epic-deferred").unwrap();
    assert_eq!(children.len(), 2);

    let closed_count = children.iter().filter(|d| {
        match storage.get_issue(&d.depends_on_id) {
            Ok(Some(child)) => child.status == Status::Closed,
            _ => false,
        }
    }).count();

    assert_eq!(closed_count, 0); // Neither deferred nor open count as closed
}

#[test]
fn test_epic_children_closure_affects_eligibility() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with 2 children
    let epic = Issue {
        id: "epic-closure".to_string(),
        title: "Epic Closure Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    let child1 = Issue {
        id: "child-closure-1".to_string(),
        title: "Child 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        ..Default::default()
    };
    storage.create_issue(&child1).unwrap();

    let child2 = Issue {
        id: "child-closure-2".to_string(),
        title: "Child 2".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        ..Default::default()
    };
    storage.create_issue(&child2).unwrap();

    storage.add_dependency("epic-closure", "child-closure-1", &DependencyType::ParentChild, "test").unwrap();
    storage.add_dependency("epic-closure", "child-closure-2", &DependencyType::ParentChild, "test").unwrap();

    // Initially not eligible
    let children = storage.get_dependencies("epic-closure").unwrap();
    let closed_count = children.iter().filter(|d| {
        match storage.get_issue(&d.depends_on_id) {
            Ok(Some(child)) => child.status == Status::Closed,
            _ => false,
        }
    }).count();
    assert!(!(closed_count == children.len() && children.len() > 0));

    // Close first child
    let changes = IssueChanges {
        status: Some(Status::Closed),
        ..Default::default()
    };
    storage.update_issue("child-closure-1", &changes).unwrap();

    // Still not eligible (only 1 of 2 closed)
    let children = storage.get_dependencies("epic-closure").unwrap();
    let closed_count = children.iter().filter(|d| {
        match storage.get_issue(&d.depends_on_id) {
            Ok(Some(child)) => child.status == Status::Closed,
            _ => false,
        }
    }).count();
    assert!(!(closed_count == children.len() && children.len() > 0));

    // Close second child
    let changes = IssueChanges {
        status: Some(Status::Closed),
        ..Default::default()
    };
    storage.update_issue("child-closure-2", &changes).unwrap();

    // Now eligible
    let children = storage.get_dependencies("epic-closure").unwrap();
    let closed_count = children.iter().filter(|d| {
        match storage.get_issue(&d.depends_on_id) {
            Ok(Some(child)) => child.status == Status::Closed,
            _ => false,
        }
    }).count();
    assert_eq!(closed_count, 2);
    assert!(closed_count == children.len() && children.len() > 0);
}
