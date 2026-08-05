//! Test assignee field serialization across all export paths.
//!
//! This test verifies that the assignee field is correctly handled:
//! - Present when Some(value)
//! - Absent when None (for JSONL storage format)
//! - Present as null when None (for CLI display format)

use std::path::PathBuf;

#[path = "../src/model.rs"]
mod model;

#[path = "../src/jsonl.rs"]
mod jsonl;

#[path = "../src/format/mod.rs"]
mod format;

#[path = "../src/format/json.rs"]
mod json_formatter;

use model::Issue;
use serde_json::Value;

fn create_test_bead_with_assignee() -> Issue {
    let mut issue = Issue::new("test-assignee".to_string(), "Test assignee field".to_string(), ".".to_string());
    issue.assignee = Some("claude-code".to_string());
    issue
}

fn create_test_bead_without_assignee() -> Issue {
    Issue::new("test-no-assignee".to_string(), "Test no assignee".to_string(), ".".to_string())
}

#[test]
fn test_model_serialize_with_assignee() {
    let issue = create_test_bead_with_assignee();
    let json = serde_json::to_string(&issue).unwrap();
    let v: Value = serde_json::from_str(&json).unwrap();
    
    assert_eq!(v.get("assignee").and_then(|a| a.as_str()), Some("claude-code"));
}

#[test]
fn test_model_serialize_without_assignee() {
    let issue = create_test_bead_without_assignee();
    let json = serde_json::to_string(&issue).unwrap();
    let v: Value = serde_json::from_str(&json).unwrap();
    
    // For standard serialization, assignee field should be absent when None
    assert!(v.get("assignee").is_none(), "assignee should be omitted when None");
}

#[test]
fn test_cli_formatter_includes_assignee_when_unset() {
    use json_formatter::JsonFormatter;
    use format::Formatter;
    
    let issue = create_test_bead_without_assignee();
    let output = JsonFormatter.format_issue(&issue);
    let v: Value = serde_json::from_str(&output).unwrap();
    
    // CLI formatter should always include assignee (as null when unset)
    assert_eq!(v.get("assignee"), Some(&Value::Null));
}

#[test]
fn test_cli_formatter_includes_assignee_when_set() {
    use json_formatter::JsonFormatter;
    use format::Formatter;
    
    let issue = create_test_bead_with_assignee();
    let output = JsonFormatter.format_issue(&issue);
    let v: Value = serde_json::from_str(&output).unwrap();
    
    assert_eq!(v.get("assignee").and_then(|a| a.as_str()), Some("claude-code"));
}

#[test]
fn test_jsonl_export_preserves_assignee_when_set() {
    use std::fs;
    use tempfile::TempDir;
    
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("test.jsonl");
    
    let issue = create_test_bead_with_assignee();
    jsonl::export_jsonl(&path, || Ok(vec![issue.clone()])).unwrap();
    
    let contents = fs::read_to_string(&path).unwrap();
    let v: Value = serde_json::from_str(contents.trim()).unwrap();
    
    assert_eq!(v.get("assignee").and_then(|a| a.as_str()), Some("claude-code"));
}

#[test]
fn test_jsonl_export_omits_assignee_when_unset() {
    use std::fs;
    use tempfile::TempDir;
    
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("test.jsonl");
    
    let issue = create_test_bead_without_assignee();
    jsonl::export_jsonl(&path, || Ok(vec![issue.clone()])).unwrap();
    
    let contents = fs::read_to_string(&path).unwrap();
    let v: Value = serde_json::from_str(contents.trim()).unwrap();
    
    // JSONL export should omit assignee when None (compact storage)
    assert!(v.get("assignee").is_none(), "JSONL should omit assignee when None");
}
