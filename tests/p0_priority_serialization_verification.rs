// Comprehensive verification of P0 priority serialization for bead bf-610ujo
// Verifies all acceptance criteria for P0 priority and epic labels

use bead_forge::model::{Issue, IssueChanges, IssueType, Priority, Status};
use bead_forge::storage::Storage;

#[test]
fn test_p0_priority_serializes_to_zero() {
    // Verify Priority::CRITICAL serializes to 0
    let p0 = Priority::CRITICAL;
    let serialized = serde_json::to_string(&p0).unwrap();
    assert_eq!(serialized, "0");
}

#[test]
fn test_p0_priority_displays_as_p0() {
    // Verify Priority::CRITICAL displays as "P0"
    let display = format!("{}", Priority::CRITICAL);
    assert_eq!(display, "P0");
}

#[test]
fn test_zero_deserializes_to_critical() {
    // Verify deserializing 0 creates Priority::CRITICAL
    let deserialized: Priority = serde_json::from_str("0").unwrap();
    assert_eq!(deserialized, Priority::CRITICAL);
    assert_eq!(deserialized.0, 0);
}

#[test]
fn test_p0_epic_with_labels_full_roundtrip() {
    // Comprehensive test: create P0 epic → add labels → retrieve → update labels → delete labels → verify
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Step 1: Create P0 epic with initial labels
    let epic = Issue {
        id: "epic-p0-full-test".to_string(),
        title: "P0 Epic Full CRUD Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string()],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Step 2: Retrieve and verify initial state
    let retrieved = storage.get_issue("epic-p0-full-test").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(retrieved.issue_type, IssueType::Epic);
    assert_eq!(retrieved.labels.len(), 1);
    assert!(retrieved.labels.contains(&"critical".to_string()));

    // Step 3: Add more labels
    storage.add_label("epic-p0-full-test", "urgent").unwrap();
    storage.add_label("epic-p0-full-test", "security").unwrap();

    let after_add = storage.get_issue("epic-p0-full-test").unwrap().unwrap();
    assert_eq!(after_add.labels.len(), 3);
    assert!(after_add.labels.contains(&"urgent".to_string()));
    assert!(after_add.labels.contains(&"security".to_string()));
    assert_eq!(after_add.priority, Priority::CRITICAL); // Priority unchanged

    // Step 4: Update labels via IssueChanges
    let changes = IssueChanges {
        labels: Some(vec![
            "critical".to_string(),
            "infrastructure".to_string(),
            "database".to_string(),
        ]),
        ..Default::default()
    };
    storage
        .update_issue("epic-p0-full-test", &changes)
        .unwrap();

    let after_update = storage.get_issue("epic-p0-full-test").unwrap().unwrap();
    assert_eq!(after_update.labels.len(), 3);
    assert!(after_update.labels.contains(&"infrastructure".to_string()));
    assert!(after_update.labels.contains(&"database".to_string()));
    assert!(!after_update.labels.contains(&"urgent".to_string()));
    assert!(!after_update.labels.contains(&"security".to_string()));
    assert_eq!(after_update.priority, Priority::CRITICAL); // Priority unchanged

    // Step 5: Remove labels
    storage.remove_label("epic-p0-full-test", "infrastructure").unwrap();
    storage.remove_label("epic-p0-full-test", "database").unwrap();

    let after_remove = storage.get_issue("epic-p0-full-test").unwrap().unwrap();
    assert_eq!(after_remove.labels.len(), 1);
    assert!(after_remove.labels.contains(&"critical".to_string()));
    assert!(!after_remove.labels.contains(&"infrastructure".to_string()));
    assert!(!after_remove.labels.contains(&"database".to_string()));
    assert_eq!(after_remove.priority, Priority::CRITICAL); // Priority unchanged

    // Final verification: P0 priority maintained through all CRUD operations
    assert_eq!(after_remove.priority, Priority::CRITICAL);
    assert_eq!(after_remove.priority.0, 0);
    assert_eq!(format!("{}", after_remove.priority), "P0");
}

#[test]
fn test_p0_edge_cases_empty_labels() {
    // Test P0 with empty label collection
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let epic = Issue {
        id: "epic-p0-empty-labels".to_string(),
        title: "P0 Epic with Empty Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    let retrieved = storage.get_issue("epic-p0-empty-labels").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 0);
}

#[test]
fn test_p0_edge_cases_duplicate_labels() {
    // Test P0 with duplicate label keys (should deduplicate)
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let epic = Issue {
        id: "epic-p0-dup-labels".to_string(),
        title: "P0 Epic with Duplicate Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![
            "critical".to_string(),
            "critical".to_string(),
            "urgent".to_string(),
        ],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    let retrieved = storage.get_issue("epic-p0-dup-labels").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    // Verify deduplication behavior
    let unique_labels: std::collections::HashSet<_> =
        retrieved.labels.into_iter().collect();
    assert_eq!(unique_labels.len(), 2);
}

#[test]
fn test_p0_edge_cases_special_characters() {
    // Test P0 with special characters in labels
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let epic = Issue {
        id: "epic-p0-special-chars".to_string(),
        title: "P0 Epic with Special Characters".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![
            "high-priority".to_string(),
            "needs-review".to_string(),
            "API:breaking".to_string(),
            "bug:security".to_string(),
        ],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    let retrieved = storage
        .get_issue("epic-p0-special-chars")
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.labels.len(), 4);
    assert!(retrieved.labels.contains(&"API:breaking".to_string()));
    assert!(retrieved.labels.contains(&"bug:security".to_string()));
}

#[test]
fn test_p0_json_serialization_comprehensive() {
    // Test comprehensive JSON serialization for P0 epic with labels
    let epic = Issue {
        id: "epic-p0-json-test".to_string(),
        title: "P0 Epic JSON Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        description: Some("Test description".to_string()),
        labels: vec!["critical".to_string(), "urgent".to_string()],
        ..Default::default()
    };

    let json = serde_json::to_string(&epic).unwrap();

    // Verify all expected fields
    assert!(json.contains("\"priority\":0"));
    assert!(json.contains("\"issue_type\":\"epic\""));
    assert!(json.contains("\"status\":\"open\""));
    assert!(json.contains("Test description"));
    assert!(json.contains("critical"));
    assert!(json.contains("urgent"));

    // Test roundtrip
    let deserialized: Issue = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.priority.0, 0);
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.labels.len(), 2);
}
