//! Comprehensive test for multi-label persistence
//!
//! This test verifies that the Issue data model correctly supports multiple labels
//! through the full persistence lifecycle:
//!
//! 1. Issue struct uses Vec<String> for labels field
//! 2. SQLite schema has bead_labels table for storage
//! 3. Read/write operations handle multiple labels correctly
//! 4. Labels persist across database close/reopen
//! 5. Serialization/deserialization preserves all labels
//!
//! Acceptance criteria for bead bf-5t1afe:
//! - Modify Issue struct in src/model.rs to use Vec<String> for labels ✅
//! - Update SQLite schema migration in src/storage/schema.rs ✅
//! - Update src/storage/sqlite.rs read/write operations for labels ✅
//! - Add test for multi-label persistence ✅ (this file)

use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;

#[test]
fn test_multi_label_creation_and_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create issue with multiple labels
    let issue = Issue {
        id: "bf-multi-label".to_string(),
        title: "Multi-Label Test Issue".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::HIGH,
        labels: vec![
            "urgent".to_string(),
            "backend".to_string(),
            "api".to_string(),
            "performance".to_string(),
        ],
        description: Some("Testing multi-label persistence".to_string()),
        ..Default::default()
    };

    storage.create_issue(&issue).unwrap();

    // Verify labels are stored correctly
    let retrieved = storage.get_issue("bf-multi-label").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 4, "Should have 4 labels");
    assert!(retrieved.labels.contains(&"urgent".to_string()));
    assert!(retrieved.labels.contains(&"backend".to_string()));
    assert!(retrieved.labels.contains(&"api".to_string()));
    assert!(retrieved.labels.contains(&"performance".to_string()));
}

#[test]
fn test_multi_label_database_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    // Create issue with multiple labels
    let issue = Issue {
        id: "bf-persist-multi".to_string(),
        title: "Multi-Label Persistence Test".to_string(),
        issue_type: IssueType::Feature,
        status: Status::Open,
        priority: Priority::MEDIUM,
        labels: vec![
            "feature".to_string(),
            "database".to_string(),
            "migration".to_string(),
        ],
        ..Default::default()
    };

    {
        let storage = Storage::open(&db_path.clone()).unwrap();
        storage.create_issue(&issue).unwrap();
    } // Storage closes here

    // Reopen database and verify labels persist
    let storage = Storage::open(&db_path).unwrap();
    let retrieved = storage.get_issue("bf-persist-multi").unwrap().unwrap();

    assert_eq!(retrieved.labels.len(), 3, "All 3 labels should persist after database close");
    assert!(retrieved.labels.contains(&"feature".to_string()));
    assert!(retrieved.labels.contains(&"database".to_string()));
    assert!(retrieved.labels.contains(&"migration".to_string()));
}

#[test]
fn test_multi_label_serialization_roundtrip() {
    // Create issue with multiple labels
    let issue = Issue {
        id: "bf-serialize-multi".to_string(),
        title: "Multi-Label Serialization Test".to_string(),
        issue_type: IssueType::Bug,
        status: Status::Open,
        priority: Priority::CRITICAL,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        labels: vec![
            "critical".to_string(),
            "security".to_string(),
            "hotfix".to_string(),
            "frontend".to_string(),
        ],
        description: Some("Testing multi-label serialization".to_string()),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&issue).unwrap();

    // Verify all labels are in JSON
    assert!(json.contains("critical"));
    assert!(json.contains("security"));
    assert!(json.contains("hotfix"));
    assert!(json.contains("frontend"));

    // Deserialize and verify
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.labels.len(), 4, "All 4 labels should survive roundtrip");
    assert!(deserialized.labels.contains(&"critical".to_string()));
    assert!(deserialized.labels.contains(&"security".to_string()));
    assert!(deserialized.labels.contains(&"hotfix".to_string()));
    assert!(deserialized.labels.contains(&"frontend".to_string()));
}

#[test]
fn test_empty_label_vector() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create issue with no labels
    let issue = Issue {
        id: "bf-no-labels".to_string(),
        title: "No Labels Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::LOW,
        labels: vec![],
        ..Default::default()
    };

    storage.create_issue(&issue).unwrap();

    let retrieved = storage.get_issue("bf-no-labels").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 0, "Empty labels vector should be preserved");
    assert!(retrieved.labels.is_empty());
}

#[test]
fn test_single_label_vector() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create issue with single label
    let issue = Issue {
        id: "bf-single-label".to_string(),
        title: "Single Label Test".to_string(),
        issue_type: IssueType::Chore,
        status: Status::Open,
        priority: Priority::BACKLOG,
        labels: vec!["maintenance".to_string()],
        ..Default::default()
    };

    storage.create_issue(&issue).unwrap();

    let retrieved = storage.get_issue("bf-single-label").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 1, "Single label should be preserved");
    assert_eq!(retrieved.labels[0], "maintenance");
}

#[test]
fn test_large_number_of_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create issue with many labels
    let many_labels: Vec<String> = (0..20)
        .map(|i| format!("label-{}", i))
        .collect();

    let issue = Issue {
        id: "bf-many-labels".to_string(),
        title: "Many Labels Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        labels: many_labels.clone(),
        ..Default::default()
    };

    storage.create_issue(&issue).unwrap();

    let retrieved = storage.get_issue("bf-many-labels").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 20, "All 20 labels should be preserved");

    for label in &many_labels {
        assert!(retrieved.labels.contains(label), "Label {} should be present", label);
    }
}

#[test]
fn test_label_update_with_multiple_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create issue with initial labels
    let issue = Issue {
        id: "bf-update-labels".to_string(),
        title: "Label Update Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::MEDIUM,
        labels: vec!["initial".to_string(), "old".to_string()],
        ..Default::default()
    };

    storage.create_issue(&issue).unwrap();

    // Update with new set of labels
    let changes = bead_forge::model::IssueChanges {
        labels: Some(vec![
            "updated".to_string(),
            "new".to_string(),
            "additional".to_string(),
        ]),
        ..Default::default()
    };

    storage.update_issue("bf-update-labels", &changes).unwrap();

    let retrieved = storage.get_issue("bf-update-labels").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 3, "Should have 3 updated labels");
    assert!(retrieved.labels.contains(&"updated".to_string()));
    assert!(retrieved.labels.contains(&"new".to_string()));
    assert!(retrieved.labels.contains(&"additional".to_string()));
    assert!(!retrieved.labels.contains(&"initial".to_string()), "Old labels should be replaced");
}

#[test]
fn test_issue_sync_equals_with_multiple_labels() {
    let issue1 = Issue {
        id: "bf-sync-test".to_string(),
        title: "Sync Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::HIGH,
        labels: vec![
            "backend".to_string(),
            "urgent".to_string(),
            "api".to_string(),
        ],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    let mut issue2 = issue1.clone();
    issue2.labels.reverse(); // Different order
    issue2.updated_at = Utc::now() + chrono::Duration::seconds(1);

    assert!(issue1.sync_equals(&issue2), "Labels should be compared regardless of order");
}