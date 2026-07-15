// Test bug default priority - verify bugs get appropriate default priority

use bead_forge::config::Config;
use bead_forge::model::{Issue, IssueType, Priority};
use bead_forge::storage::Storage;
use std::path::Path;

#[test]
fn test_bug_default_priority() {
    // Create a test database
    let db_path = Path::new("/tmp/test_bug_priority.db");
    let _ = std::fs::remove_file(db_path);
    let storage = Storage::open(db_path).expect("Failed to open database");

    // Test 1: Create a bug and check its default priority
    let bug = Issue::new(
        "bf-test1".to_string(),
        "Test bug".to_string(),
        ".".to_string(),
    );
    let bug = Issue {
        id: "bf-test1".to_string(),
        title: "Test bug".to_string(),
        issue_type: IssueType::Bug,
        priority: Priority::default(), // Uses Default impl
        source_repo: Some(".".to_string()),
        ..bug
    };

    storage.create_issue(&bug).expect("Failed to create bug");

    let retrieved = storage
        .get_issue("bf-test1")
        .expect("Failed to retrieve bug")
        .expect("Bug not found");

    // Check: What is the actual default priority for a bug?
    println!("Bug priority: {}", retrieved.priority);
    println!("Bug priority value: {}", retrieved.priority.0);

    // Test 2: Create a task and compare priorities
    let task = Issue::new(
        "bf-test2".to_string(),
        "Test task".to_string(),
        ".".to_string(),
    );
    let task = Issue {
        id: "bf-test2".to_string(),
        title: "Test task".to_string(),
        issue_type: IssueType::Task,
        priority: Priority::default(),
        source_repo: Some(".".to_string()),
        ..task
    };

    storage.create_issue(&task).expect("Failed to create task");

    let retrieved_task = storage
        .get_issue("bf-test2")
        .expect("Failed to retrieve task")
        .expect("Task not found");

    println!("Task priority: {}", retrieved_task.priority);
    println!("Task priority value: {}", retrieved_task.priority.0);

    // Test 3: Verify the Priority Default impl
    let default_priority = Priority::default();
    println!("Default Priority value: {}", default_priority.0);
    assert_eq!(
        default_priority,
        Priority::MEDIUM,
        "Default priority should be MEDIUM (P2)"
    );

    // Cleanup
    std::fs::remove_file(db_path).ok();
}

#[test]
fn test_explicit_bug_priority_p1() {
    // Test that we can explicitly set P1 (HIGH) priority for bugs
    let db_path = Path::new("/tmp/test_bug_priority_p1.db");
    let _ = std::fs::remove_file(db_path);
    let storage = Storage::open(db_path).expect("Failed to open database");

    let bug = Issue::new(
        "bf-test3".to_string(),
        "Critical bug".to_string(),
        ".".to_string(),
    );
    let bug = Issue {
        id: "bf-test3".to_string(),
        title: "Critical bug".to_string(),
        issue_type: IssueType::Bug,
        priority: Priority::HIGH, // Explicitly P1
        source_repo: Some(".".to_string()),
        ..bug
    };

    storage.create_issue(&bug).expect("Failed to create bug");

    let retrieved = storage
        .get_issue("bf-test3")
        .expect("Failed to retrieve bug")
        .expect("Bug not found");

    assert_eq!(
        retrieved.priority,
        Priority::HIGH,
        "Bug should have P1 priority when explicitly set"
    );
    assert_eq!(retrieved.priority.0, 1, "Bug priority value should be 1");

    // Cleanup
    std::fs::remove_file(db_path).ok();
    std::fs::remove_file(Path::new("/tmp/test_bug_priority_p1.db")).ok();
}

#[test]
fn test_config_default_priority() {
    // Test that config default_priority is used
    let config = Config::default();
    println!("Config default_priority: {}", config.default_priority);
    assert_eq!(
        config.default_priority, 2,
        "Config default priority should be 2 (P2)"
    );
}
