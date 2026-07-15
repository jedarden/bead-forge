// Test epic JSON format
// Tests comprehensive JSON serialization/deserialization for epic issues
// Bead: bf-4adhu

use bead_forge::format::{get_formatter, Formatter, JsonFormatter, OutputFormat};
use bead_forge::model::{Issue, IssueType, Priority, Status};
use chrono::{DateTime, Utc};
use serde_json::Value;

#[test]
fn test_epic_json_format_basic() {
    // Test basic epic JSON format with minimal fields
    let epic = Issue {
        id: "epic-json-basic".to_string(),
        title: "Basic Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        created_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        ..Default::default()
    };

    let json = serde_json::to_string(&epic).unwrap();
    let parsed: Value = serde_json::from_str(&json).unwrap();

    // Verify epic-specific fields
    assert_eq!(parsed["id"], "epic-json-basic");
    assert_eq!(parsed["title"], "Basic Epic");
    assert_eq!(parsed["issue_type"], "epic");
    assert_eq!(parsed["status"], "open");
    assert_eq!(parsed["priority"], 2);

    // Verify JSON structure is valid
    assert!(parsed.is_object());
    assert!(parsed.get("id").is_some());
    assert!(parsed.get("issue_type").is_some());
}

#[test]
fn test_epic_json_format_with_description() {
    // Test epic JSON format with description field
    let epic = Issue {
        id: "epic-json-desc".to_string(),
        title: "Epic with Description".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        description: Some("This is an epic with a detailed description".to_string()),
        created_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        ..Default::default()
    };

    let json = serde_json::to_string(&epic).unwrap();
    let parsed: Value = serde_json::from_str(&json).unwrap();

    // Verify description is included in JSON
    assert_eq!(
        parsed["description"],
        "This is an epic with a detailed description"
    );
    assert_eq!(parsed["issue_type"], "epic");
}

#[test]
fn test_epic_json_format_with_labels() {
    // Test epic JSON format with labels
    let epic = Issue {
        id: "epic-json-labels".to_string(),
        title: "Epic with Labels".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        labels: vec![
            "phase-1".to_string(),
            "critical".to_string(),
            "backend".to_string(),
        ],
        created_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        ..Default::default()
    };

    let json = serde_json::to_string(&epic).unwrap();
    let parsed: Value = serde_json::from_str(&json).unwrap();

    // Verify labels array is included and contains all labels
    assert!(parsed["labels"].is_array());
    let labels = parsed["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&Value::String("phase-1".to_string())));
    assert!(labels.contains(&Value::String("critical".to_string())));
    assert!(labels.contains(&Value::String("backend".to_string())));
}

#[test]
fn test_epic_json_format_pretty_print() {
    // Test epic JSON format with pretty printing
    let epic = Issue {
        id: "epic-json-pretty".to_string(),
        title: "Pretty Print Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::InProgress,
        priority: Priority::LOW,
        description: Some("Testing pretty printed JSON format".to_string()),
        created_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        ..Default::default()
    };

    let json_pretty = serde_json::to_string_pretty(&epic).unwrap();

    // Verify pretty format contains newlines and indentation
    assert!(json_pretty.contains('\n'));
    assert!(json_pretty.contains("  ")); // indentation

    // Parse to ensure it's still valid JSON
    let parsed: Value = serde_json::from_str(&json_pretty).unwrap();
    assert_eq!(parsed["id"], "epic-json-pretty");
    assert_eq!(parsed["issue_type"], "epic");
}

#[test]
fn test_epic_json_deserialization_from_string() {
    // Test deserializing epic from JSON string
    let json_str = r#"{
        "id": "epic-deserialize",
        "title": "Deserialized Epic",
        "issue_type": "epic",
        "status": "open",
        "priority": 1,
        "created_at": "2026-07-06T12:00:00Z",
        "updated_at": "2026-07-06T12:00:00Z",
        "description": "Testing deserialization"
    }"#;

    let epic: Issue = serde_json::from_str(json_str).unwrap();

    // Verify all fields are deserialized correctly
    assert_eq!(epic.id, "epic-deserialize");
    assert_eq!(epic.title, "Deserialized Epic");
    assert_eq!(epic.issue_type, IssueType::Epic);
    assert_eq!(epic.status, Status::Open);
    assert_eq!(epic.priority, Priority::HIGH);
    assert_eq!(
        epic.description,
        Some("Testing deserialization".to_string())
    );
}

#[test]
fn test_epic_json_roundtrip_comprehensive() {
    // Test full roundtrip: serialize -> deserialize -> serialize
    let original_epic = Issue {
        id: "epic-roundtrip".to_string(),
        title: "Roundtrip Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        description: Some("Full roundtrip test".to_string()),
        labels: vec!["test".to_string(), "roundtrip".to_string()],
        created_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        ..Default::default()
    };

    // First serialization
    let json1 = serde_json::to_string(&original_epic).unwrap();

    // Deserialization
    let deserialized: Issue = serde_json::from_str(&json1).unwrap();

    // Second serialization
    let json2 = serde_json::to_string(&deserialized).unwrap();

    // Both serializations should be identical
    assert_eq!(json1, json2);

    // All fields should match original
    assert_eq!(deserialized.id, original_epic.id);
    assert_eq!(deserialized.title, original_epic.title);
    assert_eq!(deserialized.issue_type, original_epic.issue_type);
    assert_eq!(deserialized.status, original_epic.status);
    assert_eq!(deserialized.priority, original_epic.priority);
    assert_eq!(deserialized.description, original_epic.description);
    assert_eq!(deserialized.labels, original_epic.labels);
}

#[test]
fn test_epic_json_formatter_output() {
    // Test epic JSON output through JsonFormatter
    let formatter = JsonFormatter;

    let epic = Issue {
        id: "epic-formatter".to_string(),
        title: "Formatter Output Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        description: Some("Testing formatter output".to_string()),
        created_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        ..Default::default()
    };

    let output = formatter.format_issue(&epic);

    // Verify formatter output is valid JSON
    let parsed: Value = serde_json::from_str(&output).unwrap();

    assert_eq!(parsed["id"], "epic-formatter");
    assert_eq!(parsed["issue_type"], "epic");
    assert_eq!(parsed["status"], "open");
    assert_eq!(parsed["priority"], 2);
    assert_eq!(parsed["description"], "Testing formatter output");
}

#[test]
fn test_epic_json_multiple_issues_format() {
    // Test JSONL format with multiple epics
    let formatter = JsonFormatter;

    let epics = vec![
        Issue {
            id: "epic-1".to_string(),
            title: "First Epic".to_string(),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority: Priority::CRITICAL,
            created_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            ..Default::default()
        },
        Issue {
            id: "epic-2".to_string(),
            title: "Second Epic".to_string(),
            issue_type: IssueType::Epic,
            status: Status::InProgress,
            priority: Priority::HIGH,
            created_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            ..Default::default()
        },
    ];

    let output = formatter.format_issues(&epics);

    // Verify JSONL format (newline-separated JSON objects)
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2);

    // Parse each line
    let first: Value = serde_json::from_str(lines[0]).unwrap();
    let second: Value = serde_json::from_str(lines[1]).unwrap();

    assert_eq!(first["id"], "epic-1");
    assert_eq!(first["issue_type"], "epic");
    assert_eq!(first["priority"], 0);

    assert_eq!(second["id"], "epic-2");
    assert_eq!(second["issue_type"], "epic");
    assert_eq!(second["priority"], 1);
}

#[test]
fn test_epic_json_all_priority_levels() {
    // Test epic JSON format at all priority levels
    let priorities = vec![
        (Priority::CRITICAL, 0, "P0"),
        (Priority::HIGH, 1, "P1"),
        (Priority::MEDIUM, 2, "P2"),
        (Priority::LOW, 3, "P3"),
        (Priority::BACKLOG, 4, "P4"),
    ];

    for (priority, expected_value, display) in priorities {
        let epic = Issue {
            id: format!("epic-prio-{}", display),
            title: format!("Epic at {} priority", display),
            issue_type: IssueType::Epic,
            status: Status::Open,
            priority,
            created_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            ..Default::default()
        };

        let json = serde_json::to_string(&epic).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["priority"], expected_value);
        assert_eq!(parsed["issue_type"], "epic");
    }
}

#[test]
fn test_epic_json_empty_fields_handling() {
    // Test how epic JSON handles optional/empty fields
    let epic = Issue {
        id: "epic-empty-fields".to_string(),
        title: "Epic with Empty Fields".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        created_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        description: None,
        labels: vec![],
        ..Default::default()
    };

    let json = serde_json::to_string(&epic).unwrap();
    let parsed: Value = serde_json::from_str(&json).unwrap();

    // Verify empty labels array is present (skip_serializing_if might handle this)
    if parsed.get("labels").is_some() {
        assert!(parsed["labels"].is_array());
        assert_eq!(parsed["labels"].as_array().unwrap().len(), 0);
    }

    // Verify description field handling
    assert!(parsed.get("description").is_none() || parsed["description"].is_null());

    // Verify epic type is still present
    assert_eq!(parsed["issue_type"], "epic");
}

#[test]
fn test_epic_json_format_with_assignee() {
    // Test epic JSON format with assignee
    let epic = Issue {
        id: "epic-assignee".to_string(),
        title: "Epic with Assignee".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::HIGH,
        assignee: Some("worker-1".to_string()),
        created_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        ..Default::default()
    };

    let json = serde_json::to_string(&epic).unwrap();
    let parsed: Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["assignee"], "worker-1");
    assert_eq!(parsed["issue_type"], "epic");
}

#[test]
fn test_epic_json_output_format_integration() {
    // Test epic with get_formatter() helper
    let json_formatter = get_formatter(OutputFormat::Json);

    let epic = Issue {
        id: "epic-integration".to_string(),
        title: "Integration Test Epic".to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::CRITICAL,
        created_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339("2026-07-06T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        ..Default::default()
    };

    let output = json_formatter.format_issue(&epic);
    let parsed: Value = serde_json::from_str(&output).unwrap();

    assert_eq!(parsed["id"], "epic-integration");
    assert_eq!(parsed["issue_type"], "epic");
    assert_eq!(parsed["priority"], 0);
}
