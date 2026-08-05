// Test P0 Epic with Multiple Labels
// Tests creating P0 epics with multiple labels and verifying label operations
// This test file covers the requirements for bead bf-685vij

use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;

#[test]
fn test_epic7_p0_with_multiple_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with P0 priority and multiple labels (critical, high-priority)
    let epic = Issue {
        id: "epic-p0-multi-labels".to_string(),
        title: "P0 Epic with Multiple Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL, // P0 = 0
        labels: vec!["critical".to_string(), "high-priority".to_string()],
        description: Some("Testing P0 epic with multiple labels".to_string()),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Verify the epic was stored correctly
    let retrieved = storage.get_issue("epic-p0-multi-labels").unwrap().unwrap();

    // Test 1: Verify ID matches
    assert_eq!(retrieved.id, "epic-p0-multi-labels");

    // Test 2: Verify issue type is epic
    assert_eq!(retrieved.issue_type, IssueType::Epic);

    // Test 3: Verify priority is P0 (critical = 0)
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);

    // Test 4: Verify labels are stored and retrieved correctly
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"high-priority".to_string()));

    // Test 5: Verify status
    assert_eq!(retrieved.status, Status::Open);

    // Test 6: Verify description is preserved
    assert_eq!(
        retrieved.description,
        Some("Testing P0 epic with multiple labels".to_string())
    );
}

#[test]
fn test_epic7_p0_label_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with P0 priority and initial labels
    let epic = Issue {
        id: "epic-p0-label-persistence".to_string(),
        title: "P0 Epic Label Persistence Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "high-priority".to_string()],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Verify initial state
    let initial = storage.get_issue("epic-p0-label-persistence").unwrap().unwrap();
    assert_eq!(initial.priority, Priority::CRITICAL);
    assert_eq!(initial.priority.0, 0);
    assert_eq!(initial.labels.len(), 2);
    assert!(initial.labels.contains(&"critical".to_string()));
    assert!(initial.labels.contains(&"high-priority".to_string()));

    // Add more labels to existing P0 epic
    storage
        .add_label("epic-p0-label-persistence", "urgent")
        .unwrap();
    storage
        .add_label("epic-p0-label-persistence", "security")
        .unwrap();

    // Verify labels after addition
    let after_add = storage
        .get_issue("epic-p0-label-persistence")
        .unwrap()
        .unwrap();
    assert_eq!(after_add.labels.len(), 4);
    assert!(after_add.labels.contains(&"critical".to_string()));
    assert!(after_add.labels.contains(&"high-priority".to_string()));
    assert!(after_add.labels.contains(&"urgent".to_string()));
    assert!(after_add.labels.contains(&"security".to_string()));

    // Test: P0 priority remains unchanged after label operations
    assert_eq!(after_add.priority, Priority::CRITICAL);
    assert_eq!(after_add.priority.0, 0);

    // Remove a label
    storage
        .remove_label("epic-p0-label-persistence", "high-priority")
        .unwrap();

    // Verify labels after removal
    let after_remove = storage
        .get_issue("epic-p0-label-persistence")
        .unwrap()
        .unwrap();
    assert_eq!(after_remove.labels.len(), 3);
    assert!(after_remove.labels.contains(&"critical".to_string()));
    assert!(after_remove.labels.contains(&"urgent".to_string()));
    assert!(after_remove.labels.contains(&"security".to_string()));
    assert!(!after_remove.labels.contains(&"high-priority".to_string()));

    // Test: P0 priority remains unchanged after label removal
    assert_eq!(after_remove.priority, Priority::CRITICAL);
    assert_eq!(after_remove.priority.0, 0);

    // Test: All labels persist across operations (verify final state)
    let final_state = storage
        .get_issue("epic-p0-label-persistence")
        .unwrap()
        .unwrap();
    assert_eq!(final_state.priority, Priority::CRITICAL);
    assert_eq!(final_state.priority.0, 0);
    assert_eq!(final_state.labels.len(), 3);
    assert!(final_state.labels.contains(&"critical".to_string()));
    assert!(final_state.labels.contains(&"urgent".to_string()));
    assert!(final_state.labels.contains(&"security".to_string()));
}

#[test]
fn test_epic7_p0_multiple_labels_serialization() {
    // Test JSON serialization for P0 epic with multiple labels
    let epic = Issue {
        id: "epic-p0-multi-json".to_string(),
        title: "P0 Epic Multiple Labels JSON Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![
            "critical".to_string(),
            "high-priority".to_string(),
            "urgent".to_string(),
        ],
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&epic).unwrap();

    // Verify JSON structure
    assert!(json.contains("\"priority\":0"));
    assert!(json.contains("\"issue_type\":\"epic\""));
    assert!(json.contains("critical"));
    assert!(json.contains("high-priority"));
    assert!(json.contains("urgent"));

    // Test roundtrip
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.priority.0, 0);
    assert_eq!(deserialized.labels.len(), 3);
    assert!(deserialized.labels.contains(&"critical".to_string()));
    assert!(deserialized.labels.contains(&"high-priority".to_string()));
    assert!(deserialized.labels.contains(&"urgent".to_string()));
}

#[test]
fn test_epic7_p0_label_operations_comprehensive() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 epic with empty labels
    let epic = Issue {
        id: "epic-p0-comprehensive".to_string(),
        title: "P0 Epic Comprehensive Label Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Add labels one by one
    storage
        .add_label("epic-p0-comprehensive", "label1")
        .unwrap();
    let after_first = storage.get_issue("epic-p0-comprehensive").unwrap().unwrap();
    assert_eq!(after_first.labels.len(), 1);
    assert_eq!(after_first.priority, Priority::CRITICAL);

    storage
        .add_label("epic-p0-comprehensive", "label2")
        .unwrap();
    let after_second = storage
        .get_issue("epic-p0-comprehensive")
        .unwrap()
        .unwrap();
    assert_eq!(after_second.labels.len(), 2);
    assert_eq!(after_second.priority, Priority::CRITICAL);

    storage
        .add_label("epic-p0-comprehensive", "label3")
        .unwrap();
    let after_third = storage
        .get_issue("epic-p0-comprehensive")
        .unwrap()
        .unwrap();
    assert_eq!(after_third.labels.len(), 3);
    assert_eq!(after_third.priority, Priority::CRITICAL);

    // Verify all labels are present
    assert!(after_third.labels.contains(&"label1".to_string()));
    assert!(after_third.labels.contains(&"label2".to_string()));
    assert!(after_third.labels.contains(&"label3".to_string()));

    // Remove labels
    storage
        .remove_label("epic-p0-comprehensive", "label2")
        .unwrap();
    let after_remove = storage
        .get_issue("epic-p0-comprehensive")
        .unwrap()
        .unwrap();
    assert_eq!(after_remove.labels.len(), 2);
    assert_eq!(after_remove.priority, Priority::CRITICAL);

    // Verify priority never changed
    assert_eq!(after_remove.priority, Priority::CRITICAL);
    assert_eq!(after_remove.priority.0, 0);
}
