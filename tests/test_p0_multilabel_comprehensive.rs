// Comprehensive test for multi-label P0 priority beads
// This test verifies that beads with P0 (critical) priority can have multiple labels
// and that all operations (create, update, serialize, filter) work correctly.

use bead_forge::model::{Issue, IssueChanges, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use chrono::Utc;

#[test]
fn test_p0_multilabel_beacon_bf_55ma7u_properties() {
    // Test bead bf-55ma7u has correct P0 multi-label properties
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Recreate bf-55ma7u test bead structure
    let bead = Issue {
        id: "bf-55ma7u".to_string(),
        title: "Test multi-label P0 priority bead".to_string(),
        issue_type: IssueType::Task,
        status: Status::InProgress,
        priority: Priority::CRITICAL, // P0
        labels: vec![
            "deferred".to_string(),
            "failure-count:1".to_string(),
            "multi-label".to_string(),
            "p0-test".to_string(),
            "test".to_string(),
        ],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Verify all properties
    let retrieved = storage.get_issue("bf-55ma7u").unwrap().unwrap();

    // Test P0 priority
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);

    // Test multiple labels (5 labels)
    assert_eq!(retrieved.labels.len(), 5);

    // Test specific labels exist
    assert!(retrieved.labels.contains(&"multi-label".to_string()));
    assert!(retrieved.labels.contains(&"p0-test".to_string()));
    assert!(retrieved.labels.contains(&"deferred".to_string()));
    assert!(retrieved.labels.contains(&"failure-count:1".to_string()));
    assert!(retrieved.labels.contains(&"test".to_string()));
}

#[test]
fn test_p0_multilabel_serialization_roundtrip() {
    // Test that P0 beads with multiple labels serialize/deserialize correctly
    let bead = Issue {
        id: "bf-p0-ser".to_string(),
        title: "P0 Multi-label Serialization".to_string(),
        issue_type: IssueType::Task,
        status: Status::InProgress,
        priority: Priority::CRITICAL,
        labels: vec![
            "critical".to_string(),
            "multi-label".to_string(),
            "p0-test".to_string(),
            "test".to_string(),
        ],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&bead).unwrap();

    // Verify P0 priority is serialized as 0
    assert!(json.contains(r#""priority":0"#));

    // Verify labels array exists
    assert!(json.contains(r#""labels":["#));

    // Deserialize
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    // Verify all properties preserved
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.labels.len(), 4);
    assert!(deserialized.labels.contains(&"multi-label".to_string()));
    assert!(deserialized.labels.contains(&"p0-test".to_string()));
}

#[test]
fn test_p0_multilabel_label_operations() {
    // Test adding and removing labels from P0 beads
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead with initial labels
    let bead = Issue {
        id: "bf-p0-ops".to_string(),
        title: "P0 Label Operations".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["p0".to_string(), "initial".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Add more labels
    storage.add_label("bf-p0-ops", "multi-label").unwrap();
    storage.add_label("bf-p0-ops", "another-label").unwrap();

    let retrieved = storage.get_issue("bf-p0-ops").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 4);
    assert_eq!(retrieved.priority, Priority::CRITICAL);

    // Remove a label
    storage.remove_label("bf-p0-ops", "initial").unwrap();

    let retrieved = storage.get_issue("bf-p0-ops").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 3);
    assert!(!retrieved.labels.contains(&"initial".to_string()));
}

#[test]
fn test_p0_multilabel_filter_by_priority() {
    // Test filtering beads by P0 priority
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 bead with multiple labels
    let p0_bead = Issue {
        id: "bf-p0-filter".to_string(),
        title: "P0 Filter Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "multi-label".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    // Create P1 bead with multiple labels
    let p1_bead = Issue {
        id: "bf-p1-filter".to_string(),
        title: "P1 Filter Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::HIGH,
        labels: vec!["urgent".to_string(), "multi-label".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&p0_bead).unwrap();
    storage.create_issue(&p1_bead).unwrap();

    // Filter by P0 priority
    let filter = bead_forge::model::IssueFilter {
        priority: Some(0),
        ..Default::default()
    };

    let results = storage.list_issues(&filter).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].priority, Priority::CRITICAL);
    assert_eq!(results[0].id, "bf-p0-filter");
}

#[test]
fn test_p0_multilabel_filter_by_labels() {
    // Test filtering beads by labels when they have P0 priority
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create beads with different labels
    let bead1 = Issue {
        id: "bf-p0-label1".to_string(),
        title: "P0 Label Test 1".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["p0".to_string(), "critical".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    let bead2 = Issue {
        id: "bf-p0-label2".to_string(),
        title: "P0 Label Test 2".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["p0".to_string(), "multi-label".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&bead1).unwrap();
    storage.create_issue(&bead2).unwrap();

    // Filter by "p0" label (should return both)
    let filter = bead_forge::model::IssueFilter {
        labels: Some(vec!["p0".to_string()]),
        ..Default::default()
    };

    let results = storage.list_issues(&filter).unwrap();
    assert_eq!(results.len(), 2);

    // Filter by "multi-label" (should return only bead2)
    let filter = bead_forge::model::IssueFilter {
        labels: Some(vec!["multi-label".to_string()]),
        ..Default::default()
    };

    let results = storage.list_issues(&filter).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "bf-p0-label2");
}

#[test]
fn test_p0_multilabel_update_preserves_priority_and_labels() {
    // Test that updating a P0 bead preserves its priority and labels
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "bf-p0-update".to_string(),
        title: "P0 Update Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["p0".to_string(), "test-label".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    // Update the bead (change status but not priority or labels)
    let changes = IssueChanges {
        status: Some(Status::InProgress),
        ..Default::default()
    };

    storage.update_issue("bf-p0-update", &changes).unwrap();

    let retrieved = storage.get_issue("bf-p0-update").unwrap().unwrap();

    // Verify priority and labels unchanged
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"p0".to_string()));
    assert!(retrieved.labels.contains(&"test-label".to_string()));

    // Verify status changed
    assert_eq!(retrieved.status, Status::InProgress);
}

#[test]
fn test_p0_multilabel_with_all_statuses() {
    // Test P0 beads with multiple labels work with all statuses
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let statuses = vec![
        ("bf-p0-open", Status::Open),
        ("bf-p0-inprogress", Status::InProgress),
        ("bf-p0-blocked", Status::Blocked),
        ("bf-p0-deferred", Status::Deferred),
    ];

    for (id, status) in statuses {
        let bead = Issue {
            id: id.to_string(),
            title: format!("P0 {:?}", status),
            issue_type: IssueType::Task,
            status: status.clone(),
            priority: Priority::CRITICAL,
            labels: vec!["p0".to_string(), "multi-label".to_string(), "test".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };

        storage.create_issue(&bead).unwrap();

        let retrieved = storage.get_issue(id).unwrap().unwrap();
        assert_eq!(retrieved.priority, Priority::CRITICAL);
        assert_eq!(retrieved.status, status);
        assert_eq!(retrieved.labels.len(), 3);
    }
}

#[test]
fn test_p0_multilabel_with_special_characters() {
    // Test P0 beads with labels containing special characters
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "bf-p0-special".to_string(),
        title: "P0 Special Characters".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![
            "p0:test".to_string(),
            "critical-fix".to_string(),
            "team:backend".to_string(),
            "multi-label:test".to_string(),
        ],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    storage.create_issue(&bead).unwrap();

    let retrieved = storage.get_issue("bf-p0-special").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 4);
    assert!(retrieved.labels.contains(&"p0:test".to_string()));
    assert!(retrieved.labels.contains(&"multi-label:test".to_string()));
}

#[test]
fn test_p0_multilabel_order_independence() {
    // Test that label order doesn't affect equality for sync_equals
    let bead1 = Issue {
        id: "bf-p0-order".to_string(),
        title: "P0 Order Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![
            "p0".to_string(),
            "critical".to_string(),
            "multi-label".to_string(),
        ],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    let mut bead2 = bead1.clone();
    bead2.labels.reverse();

    // sync_equals should return true (order-independent for labels)
    assert!(bead1.sync_equals(&bead2));
    assert!(bead2.sync_equals(&bead1));

    // Change a label and verify sync_equals detects difference
    bead2.labels = vec!["p0".to_string(), "different".to_string()];
    assert!(!bead1.sync_equals(&bead2));
}
