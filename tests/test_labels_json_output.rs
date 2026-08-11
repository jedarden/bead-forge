/// Tests for LabelsOutput JSON formatting functionality.
///
/// Validates that LabelsOutput produces valid JSON in the expected format:
/// - Basic format: {"id": "...", "labels": ["label1", "label2"]}
/// - Empty labels: {"id": "...", "labels": []}
/// - Special characters handling
/// - Multiple labels
/// - Serialization/deserialization roundtrips

use bead_forge::format::LabelsOutput;
use bead_forge::model::Issue;
use serde_json::{json, Value};

fn parse_json(json_str: &str) -> Value {
    serde_json::from_str(json_str).expect("Failed to parse JSON")
}

// Helper function to compare parsed labels against expected string labels
fn assert_labels_match(parsed_labels: &Vec<Value>, expected_labels: &Vec<String>) {
    assert_eq!(parsed_labels.len(), expected_labels.len());
    for (i, label) in expected_labels.iter().enumerate() {
        assert_eq!(parsed_labels[i].as_str(), Some(label.as_str()));
    }
}

#[test]
fn test_labels_output_basic() {
    let output = LabelsOutput::new("bf-test".to_string(), vec!["bug".to_string(), "urgent".to_string()]);
    let json_str = output.to_json();

    let parsed = parse_json(&json_str);

    assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("bf-test"));

    let labels = parsed.get("labels").and_then(|v| v.as_array()).unwrap();
    assert_eq!(labels.len(), 2);
    assert_eq!(labels[0].as_str(), Some("bug"));
    assert_eq!(labels[1].as_str(), Some("urgent"));
}

#[test]
fn test_labels_output_empty_labels() {
    let output = LabelsOutput::new("bf-empty".to_string(), vec![]);
    let json_str = output.to_json();

    let parsed = parse_json(&json_str);

    assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("bf-empty"));

    // Empty labels should be an empty array, not omitted
    let labels = parsed.get("labels").and_then(|v| v.as_array());
    assert!(labels.is_some());
    assert!(labels.unwrap().is_empty());
}

#[test]
fn test_labels_output_single_label() {
    let output = LabelsOutput::new("bf-single".to_string(), vec!["feature".to_string()]);
    let json_str = output.to_json();

    let parsed = parse_json(&json_str);

    assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("bf-single"));

    let parsed_labels = parsed.get("labels").and_then(|v| v.as_array()).unwrap();
    let expected_labels = vec!["feature".to_string()];
    assert_labels_match(parsed_labels, &expected_labels);
}

#[test]
fn test_labels_output_many_labels() {
    let labels = vec![
        "bug".to_string(),
        "urgent".to_string(),
        "frontend".to_string(),
        "p0".to_string(),
        "backlog".to_string(),
    ];
    let output = LabelsOutput::new("bf-many".to_string(), labels.clone());
    let json_str = output.to_json();

    let parsed = parse_json(&json_str);

    assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("bf-many"));

    let parsed_labels = parsed.get("labels").and_then(|v| v.as_array()).unwrap();
    assert_labels_match(parsed_labels, &labels);
}

#[test]
fn test_labels_output_special_characters() {
    let labels = vec![
        "tag-with-dash".to_string(),
        "tag_with_underscore".to_string(),
        "tag.with.dot".to_string(),
        "tag@special".to_string(),
    ];
    let output = LabelsOutput::new("bf-special".to_string(), labels.clone());
    let json_str = output.to_json();

    let parsed = parse_json(&json_str);

    assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("bf-special"));

    let parsed_labels = parsed.get("labels").and_then(|v| v.as_array()).unwrap();
    assert_labels_match(parsed_labels, &labels);
}

#[test]
fn test_labels_output_unicode_characters() {
    let labels = vec![
        "unicode-tag-中文".to_string(),
        "emoji-🚀".to_string(),
        "café".to_string(),
    ];
    let output = LabelsOutput::new("bf-unicode".to_string(), labels.clone());
    let json_str = output.to_json();

    let parsed = parse_json(&json_str);

    assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("bf-unicode"));

    let parsed_labels = parsed.get("labels").and_then(|v| v.as_array()).unwrap();
    assert_labels_match(parsed_labels, &labels);
}

#[test]
fn test_labels_output_pretty_format() {
    let output = LabelsOutput::new("bf-pretty".to_string(), vec!["label1".to_string(), "label2".to_string()]);
    let json_str = output.to_json_pretty();

    // Pretty format should contain newlines and indentation
    assert!(json_str.contains('\n'));
    assert!(json_str.contains("  "));

    let parsed = parse_json(&json_str);

    assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("bf-pretty"));

    let parsed_labels = parsed.get("labels").and_then(|v| v.as_array()).unwrap();
    let expected_labels = vec!["label1".to_string(), "label2".to_string()];
    assert_labels_match(parsed_labels, &expected_labels);
}

#[test]
fn test_labels_output_from_issue() {
    let mut issue = Issue::new("bf-from-issue".to_string(), "Test Issue".to_string(), ".".to_string());
    issue.labels = vec!["bug".to_string(), "p0".to_string()];

    let output = LabelsOutput::from_issue(&issue);
    let json_str = output.to_json();

    let parsed = parse_json(&json_str);

    assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("bf-from-issue"));

    let parsed_labels = parsed.get("labels").and_then(|v| v.as_array()).unwrap();
    let expected_labels = vec!["bug".to_string(), "p0".to_string()];
    assert_labels_match(parsed_labels, &expected_labels);
}

#[test]
fn test_labels_output_from_issue_empty_labels() {
    let issue = Issue::new("bf-empty-issue".to_string(), "Test Issue".to_string(), ".".to_string());

    let output = LabelsOutput::from_issue(&issue);
    let json_str = output.to_json();

    let parsed = parse_json(&json_str);

    assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some("bf-empty-issue"));
    assert!(parsed.get("labels").and_then(|v| v.as_array()).map(|arr| arr.is_empty()).unwrap_or(false));
}

#[test]
fn test_labels_output_json_validity() {
    let output = LabelsOutput::new("bf-valid".to_string(), vec!["test".to_string()]);
    let json_str = output.to_json();

    // Should parse as valid JSON
    let parsed: Result<Value, _> = serde_json::from_str(&json_str);
    assert!(parsed.is_ok(), "Output should be valid JSON");

    // Should be an object, not an array
    let parsed = parsed.unwrap();
    assert!(parsed.is_object(), "Output should be a JSON object");

    // Should have exactly two keys: "id" and "labels"
    let obj = parsed.as_object().unwrap();
    assert_eq!(obj.len(), 2, "Output should have exactly 2 keys");
    assert!(obj.contains_key("id"));
    assert!(obj.contains_key("labels"));
}

#[test]
fn test_labels_output_roundtrip() {
    let original = LabelsOutput::new("bf-roundtrip".to_string(), vec!["a".to_string(), "b".to_string()]);
    let json_str = original.to_json();

    // Deserialize back to LabelsOutput
    let deserialized: LabelsOutput = serde_json::from_str(&json_str).expect("Roundtrip should work");

    assert_eq!(deserialized.id, "bf-roundtrip");
    assert_eq!(deserialized.labels, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn test_labels_output_order_preservation() {
    let labels = vec![
        "zebra".to_string(),
        "apple".to_string(),
        "middle".to_string(),
    ];
    let output = LabelsOutput::new("bf-order".to_string(), labels.clone());
    let json_str = output.to_json();

    let parsed = parse_json(&json_str);
    let parsed_labels = parsed.get("labels").and_then(|v| v.as_array()).unwrap();

    // Order should be preserved
    let parsed_labels_str: Vec<String> = parsed_labels.iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.to_string())
        .collect();

    assert_eq!(parsed_labels_str, labels);
}

#[test]
fn test_labels_output_duplicate_labels() {
    // The output should preserve whatever is given, even duplicates
    let labels = vec![
        "duplicate".to_string(),
        "duplicate".to_string(),
        "unique".to_string(),
    ];
    let output = LabelsOutput::new("bf-duplicates".to_string(), labels.clone());
    let json_str = output.to_json();

    let parsed = parse_json(&json_str);

    let parsed_labels = parsed.get("labels").and_then(|v| v.as_array()).unwrap();
    assert_labels_match(parsed_labels, &labels);
}

#[test]
fn test_labels_output_empty_id() {
    let output = LabelsOutput::new("".to_string(), vec!["label".to_string()]);
    let json_str = output.to_json();

    let parsed = parse_json(&json_str);
    assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some(""));
}

#[test]
fn test_labels_output_long_id() {
    let long_id = "bf-".to_string() + &"a".repeat(1000);
    let output = LabelsOutput::new(long_id.clone(), vec!["label".to_string()]);
    let json_str = output.to_json();

    let parsed = parse_json(&json_str);
    assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some(long_id.as_str()));
}

#[test]
fn test_labels_output_many_labels_performance() {
    let many_labels: Vec<String> = (0..1000)
        .map(|i| format!("label-{}", i))
        .collect();

    let output = LabelsOutput::new("bf-perf".to_string(), many_labels.clone());
    let json_str = output.to_json();

    let parsed = parse_json(&json_str);

    let parsed_labels = parsed.get("labels").and_then(|v| v.as_array()).unwrap();
    assert_labels_match(parsed_labels, &many_labels);
}

#[test]
fn test_labels_output_newlines_in_labels() {
    let labels = vec![
        "label-with\nnewline".to_string(),
        "label-with\rcarriage".to_string(),
    ];
    let output = LabelsOutput::new("bf-newlines".to_string(), labels.clone());
    let json_str = output.to_json();

    let parsed = parse_json(&json_str);

    let parsed_labels = parsed.get("labels").and_then(|v| v.as_array()).unwrap();
    assert_labels_match(parsed_labels, &labels);
}

#[test]
fn test_labels_output_quotes_in_labels() {
    let labels = vec![
        "label-with\"quote".to_string(),
        "label-with'apostrophe".to_string(),
    ];
    let output = LabelsOutput::new("bf-quotes".to_string(), labels.clone());
    let json_str = output.to_json();

    let parsed = parse_json(&json_str);

    // JSON should properly escape quotes
    let parsed_labels = parsed.get("labels").and_then(|v| v.as_array()).unwrap();
    assert_eq!(parsed_labels[0].as_str().unwrap(), "label-with\"quote");
    assert_eq!(parsed_labels[1].as_str().unwrap(), "label-with'apostrophe");
}

#[test]
fn test_labels_output_backslashes_in_labels() {
    let labels = vec![
        "path\\to\\file".to_string(),
        "escaped\\nstring".to_string(),
    ];
    let output = LabelsOutput::new("bf-backslash".to_string(), labels.clone());
    let json_str = output.to_json();

    let parsed = parse_json(&json_str);

    let parsed_labels = parsed.get("labels").and_then(|v| v.as_array()).unwrap();
    assert_labels_match(parsed_labels, &labels);
}