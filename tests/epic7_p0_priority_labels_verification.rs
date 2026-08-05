// Epic 7 Verification: P0 Priority with Labels
// Comprehensive test for verifying P0 epic creation with labels (bead bf-2r2kw)
// This test ensures that Epic 7 (Priority P0 with labels) works correctly

use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use serde_json::json;

#[test]
fn test_epic7_p0_priority_verification() {
    // Verify that P0 priority is correctly represented as CRITICAL (value 0)
    let p0_priority = Priority::CRITICAL;
    assert_eq!(p0_priority.0, 0, "P0 priority should have value 0");
    assert_eq!(format!("{}", p0_priority), "P0", "P0 should display as 'P0'");
}

#[test]
fn test_epic7_p0_with_critical_label() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with P0 priority and "critical" label (matching bf-2r2kw)
    let epic = Issue {
        id: "epic7-p0-critical".to_string(),
        title: "Epic 7: P0 with Critical Label".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL, // P0
        labels: vec!["critical".to_string()],
        description: Some("Test epic with P0 priority and critical label".to_string()),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Verify storage
    let retrieved = storage.get_issue("epic7-p0-critical").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(retrieved.labels.len(), 1);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert_eq!(retrieved.issue_type, IssueType::Epic);
}

#[test]
fn test_epic7_p0_with_multiple_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic with P0 priority and multiple labels (matching bf-2r2kw labels)
    let epic = Issue {
        id: "epic7-p0-multiple".to_string(),
        title: "Epic 7: P0 with Multiple Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::InProgress,
        priority: Priority::CRITICAL, // P0
        labels: vec![
            "critical".to_string(),
            "high-priority".to_string(),
        ],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Verify storage
    let retrieved = storage.get_issue("epic7-p0-multiple").unwrap().unwrap();
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"high-priority".to_string()));
}

#[test]
fn test_epic7_p0_json_serialization() {
    // Test JSON serialization for P0 epic with labels
    let epic = Issue {
        id: "epic7-p0-json".to_string(),
        title: "Epic 7: P0 JSON Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "high-priority".to_string()],
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&epic).unwrap();

    // Verify JSON structure matches expected format
    let parsed = serde_json::from_str::<serde_json::Value>(&json).unwrap();

    assert_eq!(parsed["id"], "epic7-p0-json");
    assert_eq!(parsed["issue_type"], "epic");
    assert_eq!(parsed["priority"], 0); // P0 = 0
    assert_eq!(parsed["status"], "open");

    // Verify labels array
    let labels = parsed["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 2);
    assert!(labels.iter().any(|l| l == "critical"));
    assert!(labels.iter().any(|l| l == "high-priority"));
}

#[test]
fn test_epic7_p0_display_formatting() {
    // Test that P0 displays correctly
    let p0 = Priority::CRITICAL;
    assert_eq!(format!("{}", p0), "P0");

    // Test with epic that has labels
    let epic = Issue {
        id: "epic7-display".to_string(),
        title: "Epic 7 Display Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string()],
        ..Default::default()
    };

    // Verify priority displays as P0
    assert_eq!(format!("{}", epic.priority), "P0");

    // Verify labels are accessible
    assert_eq!(epic.labels.len(), 1);
}

#[test]
fn test_epic7_p0_roundtrip() {
    // Test JSON roundtrip for P0 epic with labels
    let original = Issue {
        id: "epic7-roundtrip".to_string(),
        title: "Epic 7 Roundtrip Test".to_string(),
        issue_type: IssueType::Epic,
        Status::InProgress,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "high-priority".to_string()],
        description: Some("Testing P0 epic with labels roundtrip".to_string()),
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&original).unwrap();

    // Deserialize back
    let deserialized: Issue = serde_json::from_str(&json).unwrap();

    // Verify all fields match
    assert_eq!(deserialized.id, original.id);
    assert_eq!(deserialized.issue_type, IssueType::Epic);
    assert_eq!(deserialized.priority, Priority::CRITICAL);
    assert_eq!(deserialized.priority.0, 0);
    assert_eq!(deserialized.status, original.status);
    assert_eq!(deserialized.labels.len(), original.labels.len());
    for label in &original.labels {
        assert!(deserialized.labels.contains(label));
    }
}

#[test]
fn test_epic7_p0_priority_comparison() {
    // Test that P0 is the highest priority
    let p0 = Priority::CRITICAL;
    let p1 = Priority::HIGH;
    let p2 = Priority::MEDIUM;
    let p3 = Priority::LOW;
    let p4 = Priority::BACKLOG;

    // P0 should be less than all others (higher priority = lower value)
    assert!(p0 < p1, "P0 should be less than P1");
    assert!(p0 < p2, "P0 should be less than P2");
    assert!(p0 < p3, "P0 should be less than P3");
    assert!(p0 < p4, "P0 should be less than P4");
}

#[test]
fn test_epic7_bead_structure() {
    // Test that the structure matches bead bf-2r2kw
    let epic = Issue {
        id: "bf-2r2kw-structure-test".to_string(),
        title: "Test Epic 7: Priority P0 with labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::InProgress,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "high-priority".to_string()],
        assignee: Some("claude-code-glm47-vclbback".to_string()),
        ..Default::default()
    };

    // Verify all expected fields
    assert_eq!(epic.issue_type, IssueType::Epic);
    assert_eq!(epic.priority, Priority::CRITICAL);
    assert_eq!(epic.priority.0, 0);
    assert_eq!(epic.labels.len(), 2);
    assert!(epic.labels.contains(&"critical".to_string()));
    assert!(epic.labels.contains(&"high-priority".to_string()));
    assert_eq!(epic.status, Status::InProgress);
    assert_eq!(epic.assignee, Some("claude-code-glm47-vclbback".to_string()));
}

#[test]
fn test_epic7_p0_label_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create P0 epic with labels
    let epic = Issue {
        id: "epic7-persistence".to_string(),
        title: "Epic 7 Persistence Test".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "high-priority".to_string()],
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Add more labels
    storage.add_label("epic7-persistence", "urgent").unwrap();
    storage.add_label("epic7-persistence", "feature").unwrap();

    // Verify all labels persist
    let retrieved = storage.get_issue("epic7-persistence").unwrap().unwrap();
    assert_eq!(retrieved.labels.len(), 4);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"high-priority".to_string()));
    assert!(retrieved.labels.contains(&"urgent".to_string()));
    assert!(retrieved.labels.contains(&"feature".to_string()));

    // Verify P0 priority is unchanged
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
}

#[test]
fn test_epic7_comprehensive_verification() {
    // Comprehensive test covering all aspects of Epic 7
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create epic matching bf-2r2kw structure
    let mut epic = Issue {
        id: "epic7-comprehensive".to_string(),
        title: "Test Epic 7: Priority P0 with labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::InProgress,
        priority: Priority::CRITICAL,
        labels: vec!["critical".to_string(), "high-priority".to_string()],
        assignee: Some("claude-code-glm47-vclbback".to_string()),
        description: Some("Comprehensive test for Epic 7 verification".to_string()),
        ..Default::default()
    };

    storage.create_issue(&epic).unwrap();

    // Test 1: Verify retrieval
    let retrieved = storage.get_issue("epic7-comprehensive").unwrap().unwrap();
    assert_eq!(retrieved.id, "epic7-comprehensive");

    // Test 2: Verify P0 priority
    assert_eq!(retrieved.priority, Priority::CRITICAL);
    assert_eq!(retrieved.priority.0, 0);
    assert_eq!(format!("{}", retrieved.priority), "P0");

    // Test 3: Verify epic type
    assert_eq!(retrieved.issue_type, IssueType::Epic);

    // Test 4: Verify labels
    assert_eq!(retrieved.labels.len(), 2);
    assert!(retrieved.labels.contains(&"critical".to_string()));
    assert!(retrieved.labels.contains(&"high-priority".to_string()));

    // Test 5: Verify status
    assert_eq!(retrieved.status, Status::InProgress);

    // Test 6: Verify assignee
    assert_eq!(retrieved.assignee, Some("claude-code-glm47-vclbback".to_string()));

    // Test 7: Verify description
    assert_eq!(
        retrieved.description,
        Some("Comprehensive test for Epic 7 verification".to_string())
    );

    // Test 8: Verify JSON serialization
    let json = serde_json::to_string(&retrieved).unwrap();
    assert!(json.contains("\"priority\":0"));
    assert!(json.contains("\"issue_type\":\"epic\""));
    assert!(json.contains("critical"));
    assert!(json.contains("high-priority"));

    // Test 9: Verify label operations
    storage.add_label("epic7-comprehensive", "test-label").unwrap();
    let updated = storage.get_issue("epic7-comprehensive").unwrap().unwrap();
    assert_eq!(updated.labels.len(), 3);
    assert!(updated.labels.contains(&"test-label".to_string()));

    // Test 10: Verify priority comparison
    assert!(retrieved.priority < Priority::HIGH);
    assert!(retrieved.priority < Priority::MEDIUM);
    assert!(retrieved.priority < Priority::LOW);
}
