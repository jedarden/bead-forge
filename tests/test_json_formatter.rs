// test_json_formatter.rs
// Test JSON output formatting for all bf commands
// Bead: bf-634y

use bead_forge::format::{get_formatter, Formatter, JsonFormatter, OutputFormat};
use bead_forge::model::{Comment, Dependency, DependencyType, Issue, IssueType, Priority, Status};
use chrono::{DateTime, Utc};

#[test]
fn test_json_formatter_single_issue() {
    let formatter = JsonFormatter;

    let issue = Issue {
        id: "bf-test1".to_string(),
        content_hash: None,
        title: "Test Bead".to_string(),
        description: Some("Test description".to_string()),
        design: None,
        acceptance_criteria: None,
        notes: None,
        status: Status::Open,
        priority: Priority(2),
        issue_type: IssueType::Task,
        assignee: None,
        owner: None,
        estimated_minutes: None,
        created_at: DateTime::parse_from_rfc3339("2026-07-03T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        created_by: None,
        updated_at: DateTime::parse_from_rfc3339("2026-07-03T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        closed_at: None,
        close_reason: None,
        closed_by_session: None,
        due_at: None,
        defer_until: None,
        external_ref: None,
        source_system: None,
        source_repo: None,
        deleted_at: None,
        deleted_by: None,
        delete_reason: None,
        original_type: None,
        compaction_level: None,
        compacted_at: None,
        compacted_at_commit: None,
        original_size: None,
        sender: None,
        ephemeral: false,
        pinned: false,
        is_template: false,
        labels: vec!["test".to_string()],
        dependencies: vec![],
        comments: vec![],
        annotations: Default::default(),
    };

    let output = formatter.format_issue(&issue);

    // Parse and verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("Invalid JSON output");

    eprintln!("JSON output: {}", output);

    assert_eq!(parsed["id"], "bf-test1");
    assert_eq!(parsed["title"], "Test Bead");
    assert_eq!(parsed["status"], "open");
    assert_eq!(parsed["priority"], 2);
    assert_eq!(parsed["issue_type"], "task");

    // Empty vectors are skipped in serialization (skip_serializing_if), so they may not be present
    // If present, they should be empty arrays
    if parsed.get("dependencies").is_some() {
        assert!(parsed["dependencies"].is_array());
        assert_eq!(parsed["dependencies"].as_array().unwrap().len(), 0);
    }
    if parsed.get("comments").is_some() {
        assert!(parsed["comments"].is_array());
        assert_eq!(parsed["comments"].as_array().unwrap().len(), 0);
    }
}

#[test]
fn test_json_formatter_multiple_issues() {
    let formatter = JsonFormatter;

    let issues = vec![
        Issue {
            id: "bf-test1".to_string(),
            content_hash: None,
            title: "First Bead".to_string(),
            description: None,
            design: None,
            acceptance_criteria: None,
            notes: None,
            status: Status::Open,
            priority: Priority(0),
            issue_type: IssueType::Bug,
            assignee: None,
            owner: None,
            estimated_minutes: None,
            created_at: DateTime::parse_from_rfc3339("2026-07-03T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            created_by: None,
            updated_at: DateTime::parse_from_rfc3339("2026-07-03T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            closed_at: None,
            close_reason: None,
            closed_by_session: None,
            due_at: None,
            defer_until: None,
            external_ref: None,
            source_system: None,
            source_repo: None,
            deleted_at: None,
            deleted_by: None,
            delete_reason: None,
            original_type: None,
            compaction_level: None,
            compacted_at: None,
            compacted_at_commit: None,
            original_size: None,
            sender: None,
            ephemeral: false,
            pinned: false,
            is_template: false,
            labels: vec![],
            dependencies: vec![],
            comments: vec![],
            annotations: Default::default(),
        },
        Issue {
            id: "bf-test2".to_string(),
            content_hash: None,
            title: "Second Bead".to_string(),
            description: None,
            design: None,
            acceptance_criteria: None,
            notes: None,
            status: Status::InProgress,
            priority: Priority(1),
            issue_type: IssueType::Feature,
            assignee: Some("worker-1".to_string()),
            owner: None,
            estimated_minutes: None,
            created_at: DateTime::parse_from_rfc3339("2026-07-03T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            created_by: None,
            updated_at: DateTime::parse_from_rfc3339("2026-07-03T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            closed_at: None,
            close_reason: None,
            closed_by_session: None,
            due_at: None,
            defer_until: None,
            external_ref: None,
            source_system: None,
            source_repo: None,
            deleted_at: None,
            deleted_by: None,
            delete_reason: None,
            original_type: None,
            compaction_level: None,
            compacted_at: None,
            compacted_at_commit: None,
            original_size: None,
            sender: None,
            ephemeral: false,
            pinned: false,
            is_template: false,
            labels: vec!["phase-1".to_string()],
            dependencies: vec![],
            comments: vec![],
            annotations: Default::default(),
        },
    ];

    let output = formatter.format_issues(&issues);

    // Verify JSONL format (newline-separated JSON objects)
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2);

    // Parse each line as JSON
    let first: serde_json::Value =
        serde_json::from_str(lines[0]).expect("Invalid JSON on first line");
    let second: serde_json::Value =
        serde_json::from_str(lines[1]).expect("Invalid JSON on second line");

    assert_eq!(first["id"], "bf-test1");
    assert_eq!(first["status"], "open");
    assert_eq!(first["priority"], 0);

    assert_eq!(second["id"], "bf-test2");
    assert_eq!(second["status"], "in_progress");
    assert_eq!(second["priority"], 1);
    assert_eq!(second["assignee"], "worker-1");
}

#[test]
fn test_json_formatter_empty_issues() {
    let formatter = JsonFormatter;
    let issues: Vec<Issue> = vec![];

    let output = formatter.format_issues(&issues);

    // Empty result should be empty string (consistent with current behavior)
    assert!(output.is_empty());
}

#[test]
fn test_json_formatter_strips_dependencies_and_comments() {
    let formatter = JsonFormatter;

    let issue = Issue {
        id: "bf-test1".to_string(),
        content_hash: None,
        title: "Test Bead".to_string(),
        description: None,
        design: None,
        acceptance_criteria: None,
        notes: None,
        status: Status::Open,
        priority: Priority(2),
        issue_type: IssueType::Task,
        assignee: None,
        owner: None,
        estimated_minutes: None,
        created_at: DateTime::parse_from_rfc3339("2026-07-03T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        created_by: None,
        updated_at: DateTime::parse_from_rfc3339("2026-07-03T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        closed_at: None,
        close_reason: None,
        closed_by_session: None,
        due_at: None,
        defer_until: None,
        external_ref: None,
        source_system: None,
        source_repo: None,
        deleted_at: None,
        deleted_by: None,
        delete_reason: None,
        original_type: None,
        compaction_level: None,
        compacted_at: None,
        compacted_at_commit: None,
        original_size: None,
        sender: None,
        ephemeral: false,
        pinned: false,
        is_template: false,
        labels: vec![],
        // These should be stripped in JSON output
        dependencies: vec![Dependency {
            issue_id: "bf-test1".to_string(),
            depends_on_id: "bf-blocker".to_string(),
            dep_type: DependencyType::Blocks,
            created_at: DateTime::parse_from_rfc3339("2026-07-03T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            created_by: None,
            metadata: None,
            thread_id: None,
        }],
        comments: vec![Comment {
            id: 1,
            issue_id: "bf-test1".to_string(),
            author: "test-user".to_string(),
            body: "Test comment".to_string(),
            created_at: DateTime::parse_from_rfc3339("2026-07-03T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        }],
        annotations: Default::default(),
    };

    let output = formatter.format_issue(&issue);
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("Invalid JSON output");

    eprintln!("JSON output with deps/comments: {}", output);

    // Verify dependencies and comments are stripped (br compatibility)
    // Empty vectors are skipped in serialization (skip_serializing_if)
    if parsed.get("dependencies").is_some() {
        assert!(parsed["dependencies"].is_array());
        assert_eq!(parsed["dependencies"].as_array().unwrap().len(), 0);
    } else {
        // If not present, that's also OK (skip_serializing_if removed them)
    }
    if parsed.get("comments").is_some() {
        assert!(parsed["comments"].is_array());
        assert_eq!(parsed["comments"].as_array().unwrap().len(), 0);
    } else {
        // If not present, that's also OK (skip_serializing_if removed them)
    }
}

#[test]
fn test_json_formatter_error_formatting() {
    let formatter = JsonFormatter;

    let error_msg = "Test error message";
    let output = formatter.format_error(error_msg);

    let parsed: serde_json::Value =
        serde_json::from_str(&output).expect("Invalid JSON error output");

    assert_eq!(parsed["error"], "Test error message");
}

#[test]
fn test_output_format_from_str() {
    assert_eq!(OutputFormat::from_str("text"), Some(OutputFormat::Text));
    assert_eq!(OutputFormat::from_str("TEXT"), Some(OutputFormat::Text));
    assert_eq!(OutputFormat::from_str("json"), Some(OutputFormat::Json));
    assert_eq!(OutputFormat::from_str("JSON"), Some(OutputFormat::Json));
    assert_eq!(OutputFormat::from_str("toon"), Some(OutputFormat::Toon));
    assert_eq!(OutputFormat::from_str("TOON"), Some(OutputFormat::Toon));
    assert_eq!(OutputFormat::from_str("invalid"), None);
}

#[test]
fn test_output_format_as_str() {
    assert_eq!(OutputFormat::Text.as_str(), "text");
    assert_eq!(OutputFormat::Json.as_str(), "json");
    assert_eq!(OutputFormat::Toon.as_str(), "toon");
}

#[test]
fn test_get_formatter() {
    let json_formatter = get_formatter(OutputFormat::Json);
    let text_formatter = get_formatter(OutputFormat::Text);
    let toon_formatter = get_formatter(OutputFormat::Toon);

    // Test that the returned formatters produce the expected output type
    let issue = Issue {
        id: "bf-test".to_string(),
        content_hash: None,
        title: "Test".to_string(),
        description: None,
        design: None,
        acceptance_criteria: None,
        notes: None,
        status: Status::Open,
        priority: Priority(2),
        issue_type: IssueType::Task,
        assignee: None,
        owner: None,
        estimated_minutes: None,
        created_at: DateTime::parse_from_rfc3339("2026-07-03T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        created_by: None,
        updated_at: DateTime::parse_from_rfc3339("2026-07-03T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        closed_at: None,
        close_reason: None,
        closed_by_session: None,
        due_at: None,
        defer_until: None,
        external_ref: None,
        source_system: None,
        source_repo: None,
        deleted_at: None,
        deleted_by: None,
        delete_reason: None,
        original_type: None,
        compaction_level: None,
        compacted_at: None,
        compacted_at_commit: None,
        original_size: None,
        sender: None,
        ephemeral: false,
        pinned: false,
        is_template: false,
        labels: vec![],
        dependencies: vec![],
        comments: vec![],
        annotations: Default::default(),
    };

    let json_output = json_formatter.format_issue(&issue);
    let parsed_json: serde_json::Value =
        serde_json::from_str(&json_output).expect("JSON formatter should produce valid JSON");
    assert_eq!(parsed_json["id"], "bf-test");

    let text_output = text_formatter.format_issue(&issue);
    assert!(text_output.contains("bf-test"));
    assert!(text_output.contains("Test"));

    let toon_output = toon_formatter.format_issue(&issue);
    assert!(toon_output.contains("bf-test"));
}
