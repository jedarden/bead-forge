use bead_forge::model::{Issue, IssueType};
use chrono::Utc;

#[test]
fn test_epic_type_creation() {
    // Create Issue with IssueType::Epic, verify type is Epic
    let issue = Issue {
        id: "bf-test-epic".to_string(),
        title: "Test Epic".to_string(),
        issue_type: IssueType::Epic,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    assert_eq!(issue.issue_type, IssueType::Epic);
}

#[test]
fn test_epic_type_serialization() {
    // Serialize epic to JSON, verify "issue_type":"epic"
    let issue = Issue {
        id: "bf-test-epic".to_string(),
        title: "Test Epic".to_string(),
        issue_type: IssueType::Epic,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    let json = serde_json::to_string(&issue).unwrap();
    assert!(json.contains(r#""issue_type":"epic""#));
}

#[test]
fn test_epic_type_roundtrip() {
    // Deserialize from JSON, verify IssueType::Epic preserved
    let json = r#"{
        "id": "bf-test-epic",
        "title": "Test Epic",
        "issue_type": "epic",
        "created_at": "2026-07-06T00:00:00Z",
        "updated_at": "2026-07-06T00:00:00Z"
    }"#;

    let issue: Issue = serde_json::from_str(json).unwrap();
    assert_eq!(issue.issue_type, IssueType::Epic);

    // Also verify serialization produces the same format
    let serialized = serde_json::to_string(&issue).unwrap();
    let deserialized: Issue = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.issue_type, IssueType::Epic);
}

#[test]
fn test_epic_string_representation() {
    // Verify epic.as_str() returns "epic"
    let epic = IssueType::Epic;
    assert_eq!(epic.as_str(), "epic");
}

#[test]
fn test_epic_default_is_task() {
    // Verify Issue::default() has Task type, not Epic
    let default_issue = Issue::default();
    assert_eq!(default_issue.issue_type, IssueType::Task);
    assert_ne!(default_issue.issue_type, IssueType::Epic);
}
