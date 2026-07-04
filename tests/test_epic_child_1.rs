// Test epic child 1 - bf-2k7fn
// Tests epic-child relationships and dependency tracking

use bead_forge::model::{Issue, IssueType, Status, DependencyType};
use bead_forge::storage::Storage;

#[test]
fn test_epic_child_relationship() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create an epic
    let epic = Issue {
        id: "epic-1".to_string(),
        title: "Test Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create a child task
    let child = Issue {
        id: "child-1".to_string(),
        title: "Test Child Task".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        ..Default::default()
    };
    storage.create_issue(&child).unwrap();

    // Create parent-child dependency (epic depends on child)
    storage.add_dependency(
        "epic-1",
        "child-1",
        &DependencyType::ParentChild,
        "test"
    ).unwrap();

    // Test 1: Verify dependency exists
    let deps = storage.get_dependencies("epic-1").unwrap();
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].depends_on_id, "child-1");
    assert_eq!(deps[0].dep_type, DependencyType::ParentChild);

    // Test 2: Query dependents (what depends on child)
    let dependents = storage.get_dependents("child-1").unwrap();
    assert_eq!(dependents.len(), 1);
    assert_eq!(dependents[0].issue_id, "epic-1");
    assert_eq!(dependents[0].dep_type, DependencyType::ParentChild);

    // Test 3: Retrieve epic and child to verify they were stored correctly
    let retrieved_epic = storage.get_issue("epic-1").unwrap().unwrap();
    assert_eq!(retrieved_epic.issue_type, IssueType::Epic);
    assert_eq!(retrieved_epic.status, Status::Open);

    let retrieved_child = storage.get_issue("child-1").unwrap().unwrap();
    assert_eq!(retrieved_child.issue_type, IssueType::Task);
    assert_eq!(retrieved_child.status, Status::Open);
}

#[test]
fn test_multiple_epic_children() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic
    let epic = Issue {
        id: "epic-2".to_string(),
        title: "Multi-child Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create multiple children
    for i in 1..=3 {
        let child = Issue {
            id: format!("child-{}", i),
            title: format!("Child Task {}", i),
            issue_type: if i == 3 { IssueType::Feature } else { IssueType::Task },
            status: Status::Open,
            ..Default::default()
        };
        storage.create_issue(&child).unwrap();

        storage.add_dependency(
            "epic-2",
            &child.id,
            &DependencyType::ParentChild,
            "test"
        ).unwrap();
    }

    // Test: Get all dependencies of epic (should be 3 children)
    let children = storage.get_dependencies("epic-2").unwrap();
    assert_eq!(children.len(), 3);

    // Test: Verify different child types are preserved
    let tasks: Vec<_> = children.iter().filter(|d| {
        storage.get_issue(&d.depends_on_id).unwrap().unwrap().issue_type == IssueType::Task
    }).collect();
    let features: Vec<_> = children.iter().filter(|d| {
        storage.get_issue(&d.depends_on_id).unwrap().unwrap().issue_type == IssueType::Feature
    }).collect();
    assert_eq!(tasks.len(), 2);
    assert_eq!(features.len(), 1);
}

#[test]
fn test_epic_type_serialization() {
    use bead_forge::model::Issue;

    let epic = Issue {
        id: "test-epic".to_string(),
        title: "Test Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        ..Default::default()
    };

    // Test JSON serialization
    let json = serde_json::to_string(&epic).unwrap();
    assert!(json.contains("\"issue_type\":\"epic\""));

    // Test JSON deserialization
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Epic);

    // Test that all issue types serialize correctly
    let task = Issue { issue_type: IssueType::Task, ..Default::default() };
    let feature = Issue { issue_type: IssueType::Feature, ..Default::default() };
    let bug = Issue { issue_type: IssueType::Bug, ..Default::default() };
    let chore = Issue { issue_type: IssueType::Chore, ..Default::default() };
    let docs = Issue { issue_type: IssueType::Docs, ..Default::default() };
    let question = Issue { issue_type: IssueType::Question, ..Default::default() };

    assert!(serde_json::to_string(&task).unwrap().contains("\"issue_type\":\"task\""));
    assert!(serde_json::to_string(&feature).unwrap().contains("\"issue_type\":\"feature\""));
    assert!(serde_json::to_string(&bug).unwrap().contains("\"issue_type\":\"bug\""));
    assert!(serde_json::to_string(&chore).unwrap().contains("\"issue_type\":\"chore\""));
    assert!(serde_json::to_string(&docs).unwrap().contains("\"issue_type\":\"docs\""));
    assert!(serde_json::to_string(&question).unwrap().contains("\"issue_type\":\"question\""));
}

#[test]
fn test_dependency_tree_epic_to_children() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic
    let epic = Issue {
        id: "epic-3".to_string(),
        title: "Tree Test Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        ..Default::default()
    };
    storage.create_issue(&epic).unwrap();

    // Create child with its own dependency
    let child1 = Issue {
        id: "child-1".to_string(),
        title: "Child 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        ..Default::default()
    };
    storage.create_issue(&child1).unwrap();

    let subtask = Issue {
        id: "subtask-1".to_string(),
        title: "Subtask 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        ..Default::default()
    };
    storage.create_issue(&subtask).unwrap();

    // Epic -> child1 (ParentChild)
    storage.add_dependency(
        "epic-3",
        "child-1",
        &DependencyType::ParentChild,
        "test"
    ).unwrap();

    // child1 -> subtask (Blocks)
    storage.add_dependency(
        "child-1",
        "subtask-1",
        &DependencyType::Blocks,
        "test"
    ).unwrap();

    // Test dependency tree "down" from epic
    let tree = storage.get_dep_tree("epic-3", "down", 0).unwrap();
    // Should have at least child-1 at depth 0 (direct dependency)
    assert!(tree.iter().any(|n| n.id == "child-1" && n.depth == 0));

    // Test dependency tree "up" from subtask
    let tree_up = storage.get_dep_tree("subtask-1", "up", 0).unwrap();
    // Should have child-1 at depth 0 (direct dependent), epic-3 at depth 1 (indirect)
    assert!(tree_up.iter().any(|n| n.id == "child-1" && n.depth == 0));
    assert!(tree_up.iter().any(|n| n.id == "epic-3" && n.depth == 1));
}
