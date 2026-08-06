// Integration test for multi-label bead creation
// Tests end-to-end flow: CLI parse → storage → verify labels persisted
// Covers 0, 1, and 3 labels as specified in the acceptance criteria

use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;

#[test]
fn test_integration_multi_label_bead_creation() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Test 1: Create bead with 0 labels
    let bead_no_labels = Issue {
        id: "test-no-labels".to_string(),
        title: "Bead with no labels".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::MEDIUM,
        labels: vec![],
        ..Default::default()
    };

    storage.create_issue(&bead_no_labels).unwrap();

    // Retrieve and verify 0 labels
    let retrieved = storage.get_issue("test-no-labels").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 0, "Bead with 0 labels should have empty labels vec");
    assert_eq!(retrieved.id, "test-no-labels");
    assert_eq!(retrieved.title, "Bead with no labels");

    // Test 2: Create bead with 1 label
    let bead_one_label = Issue {
        id: "test-one-label".to_string(),
        title: "Bead with one label".to_string(),
        issue_type: IssueType::Bug,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["urgent".to_string()],
        ..Default::default()
    };

    storage.create_issue(&bead_one_label).unwrap();

    // Retrieve and verify 1 label
    let retrieved = storage.get_issue("test-one-label").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 1, "Bead should have exactly 1 label");
    assert_eq!(retrieved.labels[0], "urgent", "Label should be 'urgent'");
    assert_eq!(retrieved.id, "test-one-label");
    assert_eq!(retrieved.title, "Bead with one label");

    // Test 3: Create bead with 3 labels
    let bead_three_labels = Issue {
        id: "test-three-labels".to_string(),
        title: "Bead with three labels".to_string(),
        issue_type: IssueType::Feature,
        status: Status::Open,
        priority: Priority::HIGH,
        labels: vec![
            "frontend".to_string(),
            "performance".to_string(),
            "P0".to_string(),
        ],
        ..Default::default()
    };

    storage.create_issue(&bead_three_labels).unwrap();

    // Retrieve and verify 3 labels
    let retrieved = storage.get_issue("test-three-labels").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 3, "Bead should have exactly 3 labels");
    assert!(retrieved.labels.contains(&"frontend".to_string()), "Should contain 'frontend' label");
    assert!(retrieved.labels.contains(&"performance".to_string()), "Should contain 'performance' label");
    assert!(retrieved.labels.contains(&"P0".to_string()), "Should contain 'P0' label");
    assert_eq!(retrieved.id, "test-three-labels");
    assert_eq!(retrieved.title, "Bead with three labels");

    // Verify order is preserved
    assert_eq!(retrieved.labels[0], "frontend", "First label should be 'frontend'");
    assert_eq!(retrieved.labels[1], "performance", "Second label should be 'performance'");
    assert_eq!(retrieved.labels[2], "P0", "Third label should be 'P0'");
}

#[test]
fn test_integration_label_persistence_across_retrieval() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create bead with multiple labels
    let bead = Issue {
        id: "test-persistence".to_string(),
        title: "Label persistence test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::MEDIUM,
        labels: vec![
            "backend".to_string(),
            "api".to_string(),
            "security".to_string(),
        ],
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Retrieve multiple times to verify persistence
    for i in 0..3 {
        let retrieved = storage.get_issue("test-persistence").unwrap().unwrap();
        assert_eq!(
            retrieved.labels.len(),
            3,
            "Retrieval {} should have 3 labels",
            i + 1
        );
        assert_eq!(retrieved.labels[0], "backend");
        assert_eq!(retrieved.labels[1], "api");
        assert_eq!(retrieved.labels[2], "security");
    }
}

#[test]
fn test_integration_labels_with_other_fields() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create bead with labels AND other fields
    let bead = Issue {
        id: "test-complex".to_string(),
        title: "Complex bead with labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::InProgress,
        priority: Priority::CRITICAL,
        description: Some("This is a complex bead".to_string()),
        assignee: Some("test-worker".to_string()),
        labels: vec!["epic".to_string(), "backend".to_string()],
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Retrieve and verify all fields
    let retrieved = storage.get_issue("test-complex").unwrap().unwrap();
    assert_eq!(retrieved.id, "test-complex");
    assert_eq!(retrieved.title, "Complex bead with labels");
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.status, Status::InProgress);
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.description, Some("This is a complex bead".to_string()));
    assert_eq!(retrieved.assignee, Some("test-worker".to_string()));
    assert_eq!(retrieved.labels.len(), 2);
    assert_eq!(retrieved.labels[0], "epic");
    assert_eq!(retrieved.labels[1], "backend");
}

#[test]
fn test_integration_get_labels_method() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create bead with 3 labels
    let bead = Issue {
        id: "test-get-labels".to_string(),
        title: "Test get_labels method".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::MEDIUM,
        labels: vec!["label1".to_string(), "label2".to_string(), "label3".to_string()],
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Use get_labels method specifically
    let labels = storage.get_labels("test-get-labels").unwrap();
    assert_eq!(labels.len(), 3);
    assert_eq!(labels[0], "label1");
    assert_eq!(labels[1], "label2");
    assert_eq!(labels[2], "label3");
}

#[test]
fn test_integration_label_serialization_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let storage = Storage::open(&db_path).unwrap();

    // Create bead with labels
    let bead = Issue {
        id: "test-serialization".to_string(),
        title: "Serialization roundtrip test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::HIGH,
        labels: vec!["json".to_string(), "roundtrip".to_string(), "test".to_string()],
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Retrieve and serialize to JSON
    let retrieved = storage.get_issue("test-serialization").unwrap().unwrap();
    let json = serde_json::to_string(&retrieved).unwrap();

    // Verify labels are in JSON
    assert!(json.contains("json"), "JSON should contain 'json' label");
    assert!(json.contains("roundtrip"), "JSON should contain 'roundtrip' label");
    assert!(json.contains("test"), "JSON should contain 'test' label");

    // Deserialize and verify
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.labels.len(), 3);
    assert_eq!(deserialized.labels[0], "json");
    assert_eq!(deserialized.labels[1], "roundtrip");
    assert_eq!(deserialized.labels[2], "test");
}
