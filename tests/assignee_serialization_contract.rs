//! Comprehensive tests for assignee field serialization contract
//!
//! This test suite validates the assignee serialization contract documented in
//! docs/assignee-serialization-contract.md. All serialization paths must obey
//! the same contract: None → absent field, Some(value) → "assignee": "value",
//! Some("") → "assignee": "" (rare, transitional).
//!
//! Contract Rules:
//! 1. Never serializes as null (field is either present with value or absent)
//! 2. None → field absent from JSON
//! 3. Some(value) → field present with value
//! 4. Some("") → field present with empty string (transitional state)

use bead_forge::model::{Issue, IssueType, Priority, Status};
use chrono::Utc;

/// Create a test issue with controlled assignee value
fn create_test_issue(assignee: Option<String>) -> Issue {
    let now = Utc::now();
    Issue {
        id: "bf-test-assignee".to_string(),
        title: "Test assignee serialization".to_string(),
        description: Some("Testing assignee field contract".to_string()),
        assignee,
        status: Status::Open,
        priority: Priority::MEDIUM,
        issue_type: IssueType::Task,
        created_at: now,
        updated_at: now,
        source_repo: Some(".".to_string()),
        ..Default::default()
    }
}

/// Verify JSON contains assignee field with expected value
fn assert_assignee_in_json(json: &str, expected_value: Option<&str>) {
    match expected_value {
        None => {
            assert!(
                !json.contains("assignee"),
                "assignee field should be absent when None, got: {}",
                json
            );
        }
        Some(value) => {
            assert!(
                json.contains("assignee"),
                "assignee field should be present when Some(value), got: {}",
                json
            );
            assert!(
                json.contains(&format!("\"assignee\":\"{}\"", value)),
                "assignee should have value '{}', got: {}",
                value,
                json
            );
        }
    }
}

#[test]
fn test_contract_none_field_absent() {
    // Contract Rule: None → field absent from JSON
    let issue = create_test_issue(None);
    let json = serde_json::to_string(&issue).expect("Serialization failed");

    // Verify field is absent
    assert!(!json.contains("assignee"));

    // Verify we can deserialize back
    let deserialized: Issue = serde_json::from_str(&json).expect("Deserialization failed");
    assert!(deserialized.assignee.is_none());
}

#[test]
fn test_contract_some_value_field_present() {
    // Contract Rule: Some(value) → field present with value
    let test_cases = vec![
        "alice",
        "bob@example.com",
        "Charlie Smith",
        "worker-1",
        "claude-code-glm-4.7-delta",
    ];

    for assignee_value in test_cases {
        let issue = create_test_issue(Some(assignee_value.to_string()));
        let json = serde_json::to_string(&issue).expect("Serialization failed");

        // Verify field is present with correct value
        assert_assignee_in_json(&json, Some(assignee_value));

        // Verify roundtrip
        let deserialized: Issue = serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(deserialized.assignee.as_deref(), Some(assignee_value));
    }
}

#[test]
fn test_contract_empty_string_field_present() {
    // Contract Rule: Some("") → field present with empty string (transitional)
    let issue = create_test_issue(Some(String::new()));
    let json = serde_json::to_string(&issue).expect("Serialization failed");

    // Verify field is present with empty string
    assert!(json.contains("\"assignee\":\"\""));

    // Verify roundtrip
    let deserialized: Issue = serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(deserialized.assignee.as_deref(), Some(""));
}

#[test]
fn test_contract_never_serializes_as_null() {
    // Contract Rule: Never serializes as null
    let issue = create_test_issue(None);
    let json = serde_json::to_string(&issue).expect("Serialization failed");

    // Verify "assignee":null never appears
    assert!(!json.contains("\"assignee\":null"));
}

#[test]
fn test_contract_roundtrip_none() {
    // Verify None → JSON → None roundtrip
    let original = create_test_issue(None);
    let json = serde_json::to_string(&original).expect("Serialization failed");
    let roundtrip: Issue = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(roundtrip.assignee, None);
    assert_eq!(original.assignee, roundtrip.assignee);
}

#[test]
fn test_contract_roundtrip_some_value() {
    // Verify Some(value) → JSON → Some(value) roundtrip
    let original = create_test_issue(Some("alice".to_string()));
    let json = serde_json::to_string(&original).expect("Serialization failed");
    let roundtrip: Issue = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(roundtrip.assignee.as_deref(), Some("alice"));
    assert_eq!(original.assignee, roundtrip.assignee);
}

#[test]
fn test_contract_roundtrip_empty_string() {
    // Verify Some("") → JSON → Some("") roundtrip
    let original = create_test_issue(Some(String::new()));
    let json = serde_json::to_string(&original).expect("Serialization failed");
    let roundtrip: Issue = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(roundtrip.assignee.as_deref(), Some(""));
    assert_eq!(original.assignee, roundtrip.assignee);
}

#[test]
fn test_contract_pretty_json_none() {
    // Verify contract holds with pretty-printed JSON
    let issue = create_test_issue(None);
    let json = serde_json::to_string_pretty(&issue).expect("Serialization failed");

    // Field should still be absent in pretty format
    assert!(!json.contains("assignee"));

    // Verify roundtrip from pretty JSON
    let deserialized: Issue = serde_json::from_str(&json).expect("Deserialization failed");
    assert!(deserialized.assignee.is_none());
}

#[test]
fn test_contract_pretty_json_some_value() {
    // Verify contract holds with pretty-printed JSON
    let issue = create_test_issue(Some("bob".to_string()));
    let json = serde_json::to_string_pretty(&issue).expect("Serialization failed");

    // Field should be present with value in pretty format
    assert!(json.contains("assignee"));
    assert!(json.contains("bob"));

    // Verify roundtrip from pretty JSON
    let deserialized: Issue = serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(deserialized.assignee.as_deref(), Some("bob"));
}

#[test]
fn test_contract_json_value_none() {
    // Verify serde_json::to_value obeys contract
    let issue = create_test_issue(None);
    let value = serde_json::to_value(&issue).expect("to_value failed");

    // Field should not exist in the Value object
    let obj = value.as_object().expect("Should be an object");
    assert!(!obj.contains_key("assignee"));
}

#[test]
fn test_contract_json_value_some() {
    // Verify serde_json::to_value obeys contract
    let issue = create_test_issue(Some("charlie".to_string()));
    let value = serde_json::to_value(&issue).expect("to_value failed");

    // Field should exist with correct value
    let obj = value.as_object().expect("Should be an object");
    assert!(obj.contains_key("assignee"));
    assert_eq!(
        obj.get("assignee").and_then(|v| v.as_str()),
        Some("charlie")
    );
}

#[test]
fn test_contract_issue_with_all_fields_none() {
    // Verify contract when multiple optional fields are None
    let issue = Issue {
        id: "bf-multiple-none".to_string(),
        title: "Multiple None fields".to_string(),
        assignee: None,
        owner: None,
        description: None,
        design: None,
        notes: None,
        acceptance_criteria: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    let json = serde_json::to_string(&issue).expect("Serialization failed");

    // All these fields should be absent
    assert!(!json.contains("assignee"));
    assert!(!json.contains("owner"));
    assert!(!json.contains("description"));
    assert!(!json.contains("design"));
    assert!(!json.contains("notes"));
    assert!(!json.contains("acceptance_criteria"));
}

#[test]
fn test_contract_issue_with_mixed_fields() {
    // Verify contract with some fields present, some absent
    let issue = Issue {
        id: "bf-mixed".to_string(),
        title: "Mixed fields".to_string(),
        assignee: Some("alice".to_string()),
        owner: None,
        description: Some("Has description".to_string()),
        design: None,
        notes: None,
        acceptance_criteria: Some("Has criteria".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    let json = serde_json::to_string(&issue).expect("Serialization failed");

    // Present fields
    assert!(json.contains("assignee"));
    assert!(json.contains("alice"));
    assert!(json.contains("description"));
    assert!(json.contains("acceptance_criteria"));

    // Absent fields
    assert!(!json.contains("owner"));
    assert!(!json.contains("design"));
    assert!(!json.contains("notes"));
}

#[test]
fn test_contract_whitespace_normalization_not_applied_in_serialization() {
    // Verify that whitespace in assignee is preserved during serialization
    // (normalization happens during input processing, not serialization)
    let test_cases = vec![
        " alice ",  // Leading/trailing spaces
        "\tworker\t",  // Tabs
        "multi\nline",  // Newline (edge case)
    ];

    for whitespace_value in test_cases {
        let issue = create_test_issue(Some(whitespace_value.to_string()));
        let json = serde_json::to_string(&issue).expect("Serialization failed");

        // Whitespace should be preserved in serialization
        assert!(json.contains("assignee"));

        let deserialized: Issue = serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(deserialized.assignee.as_deref(), Some(whitespace_value));
    }
}

#[test]
fn test_contract_special_characters_preserved() {
    // Verify special characters in assignee are preserved
    let test_cases = vec![
        "alice@example.com",
        "user+tag@domain.co.uk",
        "O'Brien",
        " Müller ",
        "日本語_user",
        "🎯worker",
        "user@company.com (remote)",
    ];

    for special_value in test_cases {
        let issue = create_test_issue(Some(special_value.to_string()));
        let json = serde_json::to_string(&issue).expect("Serialization failed");

        // Verify special characters are preserved
        assert!(json.contains("assignee"));

        let deserialized: Issue = serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(deserialized.assignee.as_deref(), Some(special_value));
    }
}

#[test]
fn test_contract_unicode_emoji() {
    // Verify emoji and unicode are preserved
    let test_cases = vec![
        "👷 worker",
        "🚀 dev-team",
        "💻 developer",
        "🎨 designer",
    ];

    for emoji_value in test_cases {
        let issue = create_test_issue(Some(emoji_value.to_string()));
        let json = serde_json::to_string(&issue).expect("Serialization failed");

        // Verify emoji are preserved
        assert!(json.contains("assignee"));

        let deserialized: Issue = serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(deserialized.assignee.as_deref(), Some(emoji_value));
    }
}

#[test]
fn test_contract_long_assignee() {
    // Verify long assignee strings are preserved
    let long_assignee = "a".repeat(1000);
    let issue = create_test_issue(Some(long_assignee.clone()));
    let json = serde_json::to_string(&issue).expect("Serialization failed");

    // Verify long string is preserved
    assert!(json.contains("assignee"));

    let deserialized: Issue = serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(deserialized.assignee.as_deref(), Some(long_assignee.as_str()));
}

#[test]
fn test_contract_very_long_assignee_within_reasonable_limits() {
    // Verify very long but reasonable assignee strings work
    // (This tests practical limits, not theoretical maximums)
    let very_long = "claude-code-worker-".repeat(100); // ~2100 characters
    let issue = create_test_issue(Some(very_long.clone()));
    let json = serde_json::to_string(&issue).expect("Serialization failed");

    // Verify it still works
    assert!(json.contains("assignee"));

    let deserialized: Issue = serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(deserialized.assignee.as_deref(), Some(very_long.as_str()));
}

#[test]
fn test_contract_multiple_issues_array() {
    // Verify contract holds when serializing array of issues
    let issues = vec![
        create_test_issue(None),
        create_test_issue(Some("alice".to_string())),
        create_test_issue(Some("bob".to_string())),
        create_test_issue(None),
        create_test_issue(Some(String::new())),
    ];

    let json = serde_json::to_string(&issues).expect("Serialization failed");

    // Count assignee field occurrences
    let assignee_count = json.matches("assignee").count();
    assert_eq!(assignee_count, 3, "Should have 3 assignee fields (2 Some + 1 empty string)");

    // Verify no null values
    assert!(!json.contains("\"assignee\":null"));
}

#[test]
fn test_contract_field_order_independence() {
    // Verify contract is independent of field order in JSON
    let issue = create_test_issue(Some("alice".to_string()));

    // Serialize to canonical JSON
    let json1 = serde_json::to_string(&issue).expect("Serialization failed");

    // Serialize to pretty JSON (different field order potentially)
    let json2 = serde_json::to_string_pretty(&issue).expect("Serialization failed");

    // Both should contain assignee field
    assert!(json1.contains("assignee"));
    assert!(json2.contains("assignee"));

    // Both should roundtrip identically
    let from_json1: Issue = serde_json::from_str(&json1).expect("Deserialization failed");
    let from_json2: Issue = serde_json::from_str(&json2).expect("Deserialization failed");

    assert_eq!(from_json1.assignee, from_json2.assignee);
    assert_eq!(from_json1.assignee.as_deref(), Some("alice"));
}

#[test]
fn test_contract_backward_compatibility_with_absent_field() {
    // Verify that JSON without assignee field deserializes to None
    let json_without_assignee = r#"{
        "id": "bf-old",
        "title": "Old format",
        "status": "open",
        "priority": 2,
        "issue_type": "task",
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z"
    }"#;

    let issue: Issue = serde_json::from_str(json_without_assignee).expect("Deserialization failed");
    assert_eq!(issue.assignee, None, "Absent field should deserialize to None");
}

#[test]
fn test_contract_backward_compatibility_with_value() {
    // Verify that JSON with assignee field deserializes correctly
    let json_with_assignee = r#"{
        "id": "bf-old-2",
        "title": "Old format with assignee",
        "assignee": "alice",
        "status": "open",
        "priority": 2,
        "issue_type": "task",
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z"
    }"#;

    let issue: Issue = serde_json::from_str(json_with_assignee).expect("Deserialization failed");
    assert_eq!(issue.assignee.as_deref(), Some("alice"));
}

#[test]
fn test_contract_backward_compatibility_with_empty_string() {
    // Verify that JSON with empty string assignee deserializes correctly
    let json_with_empty = r#"{
        "id": "bf-empty",
        "title": "Empty assignee",
        "assignee": "",
        "status": "open",
        "priority": 2,
        "issue_type": "task",
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z"
    }"#;

    let issue: Issue = serde_json::from_str(json_with_empty).expect("Deserialization failed");
    assert_eq!(issue.assignee.as_deref(), Some(""));
}

#[test]
fn test_contract_null_value_rejected_or_normalized() {
    // Verify behavior with explicit null value
    // Current behavior: serde_json with Option<String> will deserialize null to None
    let json_with_null = r#"{
        "id": "bf-null",
        "title": "Null assignee",
        "assignee": null,
        "status": "open",
        "priority": 2,
        "issue_type": "task",
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z"
    }"#;

    let issue: Issue = serde_json::from_str(json_with_null).expect("Deserialization failed");
    assert_eq!(issue.assignee, None, "null should deserialize to None");

    // But serialization should never produce null
    let json = serde_json::to_string(&issue).expect("Serialization failed");
    assert!(!json.contains("\"assignee\":null"), "Should never serialize as null");
    assert!(!json.contains("assignee"), "Should be absent when None");
}

#[test]
fn test_contract_minimal_json_size_for_none() {
    // Verify that absent field (when None) minimizes JSON size
    let issue_with_assignee = create_test_issue(Some("alice".to_string()));
    let issue_without_assignee = create_test_issue(None);

    let json_with = serde_json::to_string(&issue_with_assignee).expect("Serialization failed");
    let json_without = serde_json::to_string(&issue_without_assignee).expect("Serialization failed");

    // JSON without assignee should be smaller
    assert!(json_without.len() < json_with.len());

    // The difference should be roughly the length of the assignee field
    let size_diff = json_with.len() - json_without.len();
    assert!(size_diff >= 17); // Length of `"assignee":"alice",` roughly
}

#[test]
fn test_contract_consistency_across_multiple_serializations() {
    // Verify that serializing the same issue multiple times produces identical JSON
    let issue = create_test_issue(Some("consistent".to_string()));

    let json1 = serde_json::to_string(&issue).expect("Serialization failed");
    let json2 = serde_json::to_string(&issue).expect("Serialization failed");
    let json3 = serde_json::to_string(&issue).expect("Serialization failed");

    // All serializations should be identical
    assert_eq!(json1, json2);
    assert_eq!(json2, json3);
}

#[test]
fn test_contract_consistency_across_deserialization_serialization_cycle() {
    // Verify that multiple deserialize-serialize cycles are stable
    let original = create_test_issue(Some("stable".to_string()));

    // First cycle
    let json1 = serde_json::to_string(&original).expect("Serialization failed");
    let issue1: Issue = serde_json::from_str(&json1).expect("Deserialization failed");

    // Second cycle
    let json2 = serde_json::to_string(&issue1).expect("Serialization failed");
    let issue2: Issue = serde_json::from_str(&json2).expect("Deserialization failed");

    // Third cycle
    let json3 = serde_json::to_string(&issue2).expect("Serialization failed");

    // All JSON outputs should be identical
    assert_eq!(json1, json2);
    assert_eq!(json2, json3);

    // All issue objects should be identical
    assert_eq!(original, issue1);
    assert_eq!(issue1, issue2);
}
