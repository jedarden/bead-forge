// Test bug default priority - verify bugs get appropriate default priority
// Comprehensive test suite matching feature_default_priority.rs coverage

use bead_forge::config::Config;
use bead_forge::model::{Issue, IssueType, Priority};
use bead_forge::storage::Storage;
use chrono::Utc;

#[test]
fn test_bug_default_priority_is_p2() {
    // Create a bug using Default::default() which should apply default priority
    let bug = Issue {
        id: "bug-default-test".to_string(),
        title: "Test Bug Default Priority".to_string(),
        issue_type: IssueType::Bug,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // Verify the default priority is P2 (Medium)
    assert_eq!(
        bug.priority,
        Priority::MEDIUM,
        "Bug should have P2 (Medium) priority by default"
    );
    assert_eq!(bug.priority.0, 2, "Bug priority value should be 2");
}

#[test]
fn test_bug_default_priority_storage() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create and store a bug with default priority
    let bug = Issue {
        id: "bug-storage-default".to_string(),
        title: "Bug Storage Default Priority".to_string(),
        issue_type: IssueType::Bug,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&bug).unwrap();

    // Retrieve and verify the default priority was preserved
    let retrieved = storage.get_issue("bug-storage-default").unwrap().unwrap();

    assert_eq!(
        retrieved.issue_type,
        IssueType::Bug,
        "Issue type should be Bug"
    );
    assert_eq!(
        retrieved.priority,
        Priority::MEDIUM,
        "Retrieved bug should have P2 priority"
    );
    assert_eq!(
        retrieved.priority.0, 2,
        "Retrieved bug priority value should be 2"
    );
}

#[test]
fn test_bug_default_priority_serialization() {
    // Create a bug with default priority
    let bug = Issue {
        id: "bug-serialize-default".to_string(),
        title: "Bug Serialization Default Priority".to_string(),
        issue_type: IssueType::Bug,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&bug).unwrap();

    // Verify bug type is serialized correctly
    assert!(
        json.contains(r#""issue_type":"bug""#),
        "JSON should contain bug type"
    );

    // Verify default P2 priority is serialized as 2
    assert!(
        json.contains(r#""priority":2"#),
        "JSON should contain priority: 2"
    );

    // Deserialize and verify
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Bug);
    assert_eq!(deserialized.priority, Priority::MEDIUM);
    assert_eq!(deserialized.priority.0, 2);
}

#[test]
fn test_multiple_bugs_with_default_priority() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple bugs using default priority
    for i in 1..=5 {
        let bug = Issue {
            id: format!("bug-default-{}", i),
            title: format!("Default Priority Bug {}", i),
            issue_type: IssueType::Bug,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };
        storage.create_issue(&bug).unwrap();
    }

    // Retrieve all bugs and verify they all have P2 priority
    let all_issues = storage.list_issues(&Default::default()).unwrap();
    let bugs: Vec<_> = all_issues
        .iter()
        .filter(|i| i.issue_type == IssueType::Bug)
        .collect();

    assert_eq!(bugs.len(), 5, "Should have 5 bugs");

    // Verify each bug has P2 (Medium) priority
    for bug in bugs {
        assert_eq!(
            bug.priority,
            Priority::MEDIUM,
            "Bug {} should have P2 priority",
            bug.id
        );
        assert_eq!(
            bug.priority.0, 2,
            "Bug {} priority value should be 2",
            bug.id
        );
    }
}

#[test]
fn test_bug_default_vs_explicit_priorities() {
    // Create bugs with different priorities including default
    let priorities = vec![
        (Priority::CRITICAL, 0, "P0"),
        (Priority::HIGH, 1, "P1"),
        (Priority::MEDIUM, 2, "P2"), // This is the default
        (Priority::LOW, 3, "P3"),
        (Priority::BACKLOG, 4, "P4"),
    ];

    for (priority, value, label) in priorities {
        let bug = Issue {
            id: format!("bug-{}-{}", label, value),
            title: format!("{} Bug", label),
            issue_type: IssueType::Bug,
            priority,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };

        assert_eq!(
            bug.priority.0, value,
            "{} bug should have priority value {}",
            label, value
        );
        assert_eq!(
            format!("{}", bug.priority),
            label,
            "{} bug should display as {}",
            label,
            label
        );
    }
}

#[test]
fn test_issue_new_default_priority_for_bug() {
    // Test Issue::new() which uses Default::default() for priority
    let bug = Issue::new(
        "bug-new-test".to_string(),
        "Test Issue New".to_string(),
        ".".to_string(),
    );

    // Set the issue type to bug
    let mut bug = bug;
    bug.issue_type = IssueType::Bug;

    // Verify priority is P2 (the default)
    assert_eq!(
        bug.priority,
        Priority::MEDIUM,
        "Issue::new() should have P2 default priority for bugs"
    );
    assert_eq!(bug.priority.0, 2, "Default priority value should be 2");
}

#[test]
fn test_bug_all_priorities_exist() {
    // Test that bugs can have any priority level
    let priorities = vec![
        (Priority::CRITICAL, 0, "P0"),
        (Priority::HIGH, 1, "P1"),
        (Priority::MEDIUM, 2, "P2"),
        (Priority::LOW, 3, "P3"),
        (Priority::BACKLOG, 4, "P4"),
    ];

    for (i, (priority, value, display)) in priorities.iter().enumerate() {
        let bug = Issue {
            id: format!("bug-p{}-test", value),
            title: format!("Bug with {} priority", display),
            issue_type: IssueType::Bug,
            priority: *priority,
            ..Default::default()
        };

        // Verify priority is set correctly
        assert_eq!(bug.priority.0, *value);
        assert_eq!(format!("{}", bug.priority), *display);
        assert_eq!(bug.issue_type, IssueType::Bug);

        // Verify serialization
        let json = serde_json::to_string(&bug).unwrap();
        assert!(json.contains(&format!("\"priority\":{}", value)));
        assert!(json.contains("\"issue_type\":\"bug\""));

        // Verify roundtrip
        let deserialized: Issue = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.priority, *priority);
        assert_eq!(deserialized.issue_type, IssueType::Bug);
    }
}

#[test]
fn test_bug_vs_feature_default_priority() {
    // Verify that both bugs and features have the same default priority
    let bug = Issue {
        id: "bug-default-compare".to_string(),
        title: "Bug Default".to_string(),
        issue_type: IssueType::Bug,
        ..Default::default()
    };

    let feature = Issue {
        id: "feature-default-compare".to_string(),
        title: "Feature Default".to_string(),
        issue_type: IssueType::Feature,
        ..Default::default()
    };

    assert_eq!(
        bug.priority, feature.priority,
        "Bugs and features should have the same default priority"
    );
    assert_eq!(bug.priority, Priority::MEDIUM, "Both should default to P2");
}

#[test]
fn test_bug_explicit_p1_priority() {
    // Test that we can explicitly set P1 (HIGH) priority for bugs
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bug = Issue {
        id: "bug-p1-test".to_string(),
        title: "High Priority Bug".to_string(),
        issue_type: IssueType::Bug,
        priority: Priority::HIGH, // Explicitly P1
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&bug).unwrap();

    let retrieved = storage.get_issue("bug-p1-test").unwrap().unwrap();

    assert_eq!(
        retrieved.priority,
        Priority::HIGH,
        "Bug should have P1 priority when explicitly set"
    );
    assert_eq!(retrieved.priority.0, 1, "Bug priority value should be 1");
}

#[test]
fn test_bug_explicit_p0_critical_priority() {
    // Test that we can explicitly set P0 (CRITICAL) priority for bugs
    let bug = Issue {
        id: "bug-p0-test".to_string(),
        title: "Critical Bug".to_string(),
        issue_type: IssueType::Bug,
        priority: Priority::CRITICAL, // Explicitly P0
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    assert_eq!(
        bug.priority,
        Priority::CRITICAL,
        "Bug should have P0 priority when explicitly set"
    );
    assert_eq!(bug.priority.0, 0, "Bug priority value should be 0");
}

#[test]
fn test_bug_vs_task_default_priority() {
    // Verify that bugs and tasks have the same default priority
    let bug = Issue {
        id: "bug-default-compare-task".to_string(),
        title: "Bug Default".to_string(),
        issue_type: IssueType::Bug,
        ..Default::default()
    };

    let task = Issue {
        id: "task-default-compare-bug".to_string(),
        title: "Task Default".to_string(),
        issue_type: IssueType::Task,
        ..Default::default()
    };

    assert_eq!(
        bug.priority, task.priority,
        "Bugs and tasks should have the same default priority"
    );
    assert_eq!(bug.priority, Priority::MEDIUM, "Both should default to P2");
}

#[test]
fn test_config_default_priority() {
    // Test that config default_priority is used
    let config = Config::default();
    assert_eq!(
        config.default_priority, 2,
        "Config default priority should be 2 (P2)"
    );
}
