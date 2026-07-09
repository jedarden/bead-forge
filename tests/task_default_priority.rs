// Test Task Default Priority
// Tests that tasks created without specifying a priority get the default P2 (Medium) priority

use bead_forge::model::{Issue, IssueType, Priority};
use bead_forge::storage::Storage;
use chrono::Utc;

#[test]
fn test_task_default_priority_is_p2() {
    // Create a task using Default::default() which should apply default priority
    let task = Issue {
        id: "task-default-test".to_string(),
        title: "Test Task Default Priority".to_string(),
        issue_type: IssueType::Task,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // Verify the default priority is P2 (Medium)
    assert_eq!(task.priority, Priority::MEDIUM, "Task should have P2 (Medium) priority by default");
    assert_eq!(task.priority.0, 2, "Task priority value should be 2");
}

#[test]
fn test_task_default_priority_storage() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create and store a task with default priority
    let task = Issue {
        id: "task-storage-default".to_string(),
        title: "Task Storage Default Priority".to_string(),
        issue_type: IssueType::Task,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&task).unwrap();

    // Retrieve and verify the default priority was preserved
    let retrieved = storage.get_issue("task-storage-default").unwrap().unwrap();

    assert_eq!(retrieved.issue_type, IssueType::Task, "Issue type should be Task");
    assert_eq!(retrieved.priority, Priority::MEDIUM, "Retrieved task should have P2 priority");
    assert_eq!(retrieved.priority.0, 2, "Retrieved task priority value should be 2");
}

#[test]
fn test_task_default_priority_serialization() {
    // Create a task with default priority
    let task = Issue {
        id: "task-serialize-default".to_string(),
        title: "Task Serialization Default Priority".to_string(),
        issue_type: IssueType::Task,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&task).unwrap();

    // Verify task type is serialized correctly
    assert!(json.contains(r#""issue_type":"task""#), "JSON should contain task type");

    // Verify default P2 priority is serialized as 2
    assert!(json.contains(r#""priority":2"#), "JSON should contain priority: 2");

    // Deserialize and verify
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Task);
    assert_eq!(deserialized.priority, Priority::MEDIUM);
    assert_eq!(deserialized.priority.0, 2);
}

#[test]
fn test_multiple_tasks_with_default_priority() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple tasks using default priority
    for i in 1..=5 {
        let task = Issue {
            id: format!("task-default-{}", i),
            title: format!("Default Priority Task {}", i),
            issue_type: IssueType::Task,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };
        storage.create_issue(&task).unwrap();
    }

    // Retrieve all tasks and verify they all have P2 priority
    let all_issues = storage.list_issues(&Default::default()).unwrap();
    let tasks: Vec<_> = all_issues
        .iter()
        .filter(|i| i.issue_type == IssueType::Task)
        .collect();

    assert_eq!(tasks.len(), 5, "Should have 5 tasks");

    // Verify each task has P2 (Medium) priority
    for task in tasks {
        assert_eq!(
            task.priority,
            Priority::MEDIUM,
            "Task {} should have P2 priority",
            task.id
        );
        assert_eq!(task.priority.0, 2, "Task {} priority value should be 2", task.id);
    }
}

#[test]
fn test_task_default_vs_explicit_priorities() {
    // Create tasks with different priorities including default
    let priorities = vec![
        (Priority::CRITICAL, 0, "P0"),
        (Priority::HIGH, 1, "P1"),
        (Priority::MEDIUM, 2, "P2"), // This is the default
        (Priority::LOW, 3, "P3"),
        (Priority::BACKLOG, 4, "P4"),
    ];

    for (priority, value, label) in priorities {
        let task = Issue {
            id: format!("task-{}-{}", label, value),
            title: format!("{} Task", label),
            issue_type: IssueType::Task,
            priority,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };

        assert_eq!(
            task.priority.0, value,
            "{} task should have priority value {}",
            label, value
        );
        assert_eq!(
            format!("{}", task.priority),
            label,
            "{} task should display as {}",
            label, label
        );
    }
}

#[test]
fn test_issue_new_default_priority_for_task() {
    // Test Issue::new() which uses Default::default() for priority
    let task = Issue::new(
        "task-new-test".to_string(),
        "Test Issue New".to_string(),
        ".".to_string(),
    );

    // The task type should already be Task by default
    assert_eq!(
        task.issue_type,
        IssueType::Task,
        "Issue::new() should create Task by default"
    );

    // Verify priority is P2 (the default)
    assert_eq!(
        task.priority,
        Priority::MEDIUM,
        "Issue::new() should have P2 default priority"
    );
    assert_eq!(task.priority.0, 2, "Default priority value should be 2");
}

#[test]
fn test_task_and_epic_both_have_p2_default() {
    // Verify that both tasks and epics use the same default priority
    let task = Issue {
        id: "task-p2".to_string(),
        title: "Task".to_string(),
        issue_type: IssueType::Task,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    let epic = Issue {
        id: "epic-p2".to_string(),
        title: "Epic".to_string(),
        issue_type: IssueType::Epic,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // Both should have the same default priority
    assert_eq!(
        task.priority,
        epic.priority,
        "Task and Epic should both have P2 default priority"
    );
    assert_eq!(task.priority, Priority::MEDIUM);
    assert_eq!(epic.priority, Priority::MEDIUM);
    assert_eq!(task.priority.0, 2);
    assert_eq!(epic.priority.0, 2);
}

#[test]
fn test_task_priority_is_not_p0_by_default() {
    // Explicitly test that tasks do NOT default to P0 (CRITICAL)
    let task = Issue {
        id: "task-not-p0".to_string(),
        title: "Task Should Not Be P0".to_string(),
        issue_type: IssueType::Task,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // The default priority should NOT be P0
    assert_ne!(
        task.priority,
        Priority::CRITICAL,
        "Task should NOT default to P0 (CRITICAL)"
    );
    assert_ne!(task.priority.0, 0, "Task priority value should NOT be 0");

    // The default should be P2
    assert_eq!(task.priority, Priority::MEDIUM);
    assert_eq!(task.priority.0, 2);
}

#[test]
fn test_all_issue_types_have_p2_default() {
    // Verify all standard issue types use P2 as the default priority
    let issue_types = vec![
        IssueType::Task,
        IssueType::Bug,
        IssueType::Feature,
        IssueType::Epic,
        IssueType::Chore,
        IssueType::Docs,
        IssueType::Question,
    ];

    for issue_type in issue_types {
        let issue = Issue {
            id: format!("{}-p2-test", issue_type.as_str()),
            title: format!("{} P2 Default Test", issue_type.as_str()),
            issue_type: issue_type.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };

        assert_eq!(
            issue.priority,
            Priority::MEDIUM,
            "{} issue type should have P2 default priority",
            issue_type.as_str()
        );
        assert_eq!(issue.priority.0, 2);
    }
}
