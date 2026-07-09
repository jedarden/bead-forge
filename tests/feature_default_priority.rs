// Test Feature Default Priority
// Tests that features created without specifying a priority get the default P2 (Medium) priority

use bead_forge::model::{Issue, IssueType, Priority};
use bead_forge::storage::Storage;
use chrono::Utc;

#[test]
fn test_feature_default_priority_is_p2() {
    // Create a feature using Default::default() which should apply default priority
    let feature = Issue {
        id: "feature-default-test".to_string(),
        title: "Test Feature Default Priority".to_string(),
        issue_type: IssueType::Feature,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // Verify the default priority is P2 (Medium)
    assert_eq!(feature.priority, Priority::MEDIUM, "Feature should have P2 (Medium) priority by default");
    assert_eq!(feature.priority.0, 2, "Feature priority value should be 2");
}

#[test]
fn test_feature_default_priority_storage() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create and store a feature with default priority
    let feature = Issue {
        id: "feature-storage-default".to_string(),
        title: "Feature Storage Default Priority".to_string(),
        issue_type: IssueType::Feature,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&feature).unwrap();

    // Retrieve and verify the default priority was preserved
    let retrieved = storage.get_issue("feature-storage-default").unwrap().unwrap();

    assert_eq!(retrieved.issue_type, IssueType::Feature, "Issue type should be Feature");
    assert_eq!(retrieved.priority, Priority::MEDIUM, "Retrieved feature should have P2 priority");
    assert_eq!(retrieved.priority.0, 2, "Retrieved feature priority value should be 2");
}

#[test]
fn test_feature_default_priority_serialization() {
    // Create a feature with default priority
    let feature = Issue {
        id: "feature-serialize-default".to_string(),
        title: "Feature Serialization Default Priority".to_string(),
        issue_type: IssueType::Feature,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&feature).unwrap();

    // Verify feature type is serialized correctly
    assert!(json.contains(r#""issue_type":"feature""#), "JSON should contain feature type");

    // Verify default P2 priority is serialized as 2
    assert!(json.contains(r#""priority":2"#), "JSON should contain priority: 2");

    // Deserialize and verify
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Feature);
    assert_eq!(deserialized.priority, Priority::MEDIUM);
    assert_eq!(deserialized.priority.0, 2);
}

#[test]
fn test_multiple_features_with_default_priority() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create multiple features using default priority
    for i in 1..=5 {
        let feature = Issue {
            id: format!("feature-default-{}", i),
            title: format!("Default Priority Feature {}", i),
            issue_type: IssueType::Feature,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };
        storage.create_issue(&feature).unwrap();
    }

    // Retrieve all features and verify they all have P2 priority
    let all_issues = storage.list_issues(&Default::default()).unwrap();
    let features: Vec<_> = all_issues
        .iter()
        .filter(|i| i.issue_type == IssueType::Feature)
        .collect();

    assert_eq!(features.len(), 5, "Should have 5 features");

    // Verify each feature has P2 (Medium) priority
    for feature in features {
        assert_eq!(
            feature.priority,
            Priority::MEDIUM,
            "Feature {} should have P2 priority",
            feature.id
        );
        assert_eq!(feature.priority.0, 2, "Feature {} priority value should be 2", feature.id);
    }
}

#[test]
fn test_feature_default_vs_explicit_priorities() {
    // Create features with different priorities including default
    let priorities = vec![
        (Priority::CRITICAL, 0, "P0"),
        (Priority::HIGH, 1, "P1"),
        (Priority::MEDIUM, 2, "P2"), // This is the default
        (Priority::LOW, 3, "P3"),
        (Priority::BACKLOG, 4, "P4"),
    ];

    for (priority, value, label) in priorities {
        let feature = Issue {
            id: format!("feature-{}-{}", label, value),
            title: format!("{} Feature", label),
            issue_type: IssueType::Feature,
            priority,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };

        assert_eq!(
            feature.priority.0, value,
            "{} feature should have priority value {}",
            label, value
        );
        assert_eq!(
            format!("{}", feature.priority),
            label,
            "{} feature should display as {}",
            label, label
        );
    }
}

#[test]
fn test_issue_new_default_priority_for_feature() {
    // Test Issue::new() which uses Default::default() for priority
    let feature = Issue::new(
        "feature-new-test".to_string(),
        "Test Issue New".to_string(),
        ".".to_string(),
    );

    // Set the issue type to feature
    let mut feature = feature;
    feature.issue_type = IssueType::Feature;

    // Verify priority is P2 (the default)
    assert_eq!(
        feature.priority,
        Priority::MEDIUM,
        "Issue::new() should have P2 default priority for features"
    );
    assert_eq!(feature.priority.0, 2, "Default priority value should be 2");
}

#[test]
fn test_feature_all_priorities_exist() {
    // Test that features can have any priority level
    let priorities = vec![
        (Priority::CRITICAL, 0, "P0"),
        (Priority::HIGH, 1, "P1"),
        (Priority::MEDIUM, 2, "P2"),
        (Priority::LOW, 3, "P3"),
        (Priority::BACKLOG, 4, "P4"),
    ];

    for (i, (priority, value, display)) in priorities.iter().enumerate() {
        let feature = Issue {
            id: format!("feature-p{}-test", value),
            title: format!("Feature with {} priority", display),
            issue_type: IssueType::Feature,
            priority: *priority,
            ..Default::default()
        };

        // Verify priority is set correctly
        assert_eq!(feature.priority.0, *value);
        assert_eq!(format!("{}", feature.priority), *display);
        assert_eq!(feature.issue_type, IssueType::Feature);

        // Verify serialization
        let json = serde_json::to_string(&feature).unwrap();
        assert!(json.contains(&format!("\"priority\":{}", value)));
        assert!(json.contains("\"issue_type\":\"feature\""));

        // Verify roundtrip
        let deserialized: Issue = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.priority, *priority);
        assert_eq!(deserialized.issue_type, IssueType::Feature);

        println!("Test {} for {}: Feature has priority {} (display: {})", i + 1, display, value, format!("{}", feature.priority));
    }
}

#[test]
fn test_feature_vs_bug_default_priority() {
    // Verify that both features and bugs have the same default priority
    let feature = Issue {
        id: "feature-default-compare".to_string(),
        title: "Feature Default".to_string(),
        issue_type: IssueType::Feature,
        ..Default::default()
    };

    let bug = Issue {
        id: "bug-default-compare".to_string(),
        title: "Bug Default".to_string(),
        issue_type: IssueType::Bug,
        ..Default::default()
    };

    assert_eq!(feature.priority, bug.priority, "Features and bugs should have the same default priority");
    assert_eq!(feature.priority, Priority::MEDIUM, "Both should default to P2");
}

#[test]
fn test_feature_explicit_p1_priority() {
    // Test that we can explicitly set P1 (HIGH) priority for features
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let feature = Issue {
        id: "feature-p1-test".to_string(),
        title: "High Priority Feature".to_string(),
        issue_type: IssueType::Feature,
        priority: Priority::HIGH, // Explicitly P1
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&feature).unwrap();

    let retrieved = storage.get_issue("feature-p1-test").unwrap().unwrap();

    assert_eq!(retrieved.priority, Priority::HIGH, "Feature should have P1 priority when explicitly set");
    assert_eq!(retrieved.priority.0, 1, "Feature priority value should be 1");
}

#[test]
fn test_feature_explicit_p0_critical_priority() {
    // Test that we can explicitly set P0 (CRITICAL) priority for features
    let feature = Issue {
        id: "feature-p0-test".to_string(),
        title: "Critical Feature".to_string(),
        issue_type: IssueType::Feature,
        priority: Priority::CRITICAL, // Explicitly P0
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    assert_eq!(feature.priority, Priority::CRITICAL, "Feature should have P0 priority when explicitly set");
    assert_eq!(feature.priority.0, 0, "Feature priority value should be 0");
}
