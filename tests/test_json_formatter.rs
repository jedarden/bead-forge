// test_json_formatter.rs
// Test JSON output formatting for all bf commands
// Bead: bf-3yfgg

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
        events: vec![],
        annotations: Default::default(),
    };

    let output = formatter.format_issue(&issue);

    // Parse raw JSON and verify it's valid
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("Invalid JSON output");

    // Verify the issue data directly (no envelope at this level)
    assert_eq!(parsed["id"], "bf-test1");
    assert_eq!(parsed["title"], "Test Bead");
    assert_eq!(parsed["status"], "open");
    assert_eq!(parsed["priority"], 2);
    assert_eq!(parsed["issue_type"], "task");

    // Verify assignee and labels are always present (display normalization)
    assert!(parsed.get("assignee").is_some());
    assert_eq!(parsed["assignee"], serde_json::Value::Null);
    assert!(parsed.get("labels").is_some());
    assert!(parsed["labels"].is_array());
    assert_eq!(parsed["labels"].as_array().unwrap().len(), 1);

    // Dependencies and comments should be stripped (empty arrays)
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

    // Parse JSONL (one JSON object per line)
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2, "Should output 2 lines for 2 issues");

    // Verify first issue
    let first: serde_json::Value =
        serde_json::from_str(lines[0]).expect("First line must be valid JSON");
    assert_eq!(first["id"], "bf-test1");
    assert_eq!(first["status"], "open");
    assert_eq!(first["priority"], 0);
    assert!(first.get("assignee").is_some());
    assert!(first.get("labels").is_some());

    // Verify second issue
    let second: serde_json::Value =
        serde_json::from_str(lines[1]).expect("Second line must be valid JSON");
    assert_eq!(second["id"], "bf-test2");
    assert_eq!(second["status"], "in_progress");
    assert_eq!(second["priority"], 1);
    assert_eq!(second["assignee"], "worker-1");
    assert_eq!(second["labels"].as_array().unwrap().len(), 1);
}

#[test]
fn test_json_formatter_empty_issues() {
    let formatter = JsonFormatter;
    let issues: Vec<Issue> = vec![];

    let output = formatter.format_issues(&issues);

    // Empty result should be an empty string (JSONL: 0 lines)
    assert!(
        output.is_empty() || output.trim().is_empty(),
        "Empty input should produce empty output"
    );
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
            title: None,
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

    // Verify simple error object
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
    // Verify raw issue data (no envelope at this level)
    assert_eq!(parsed_json["id"], "bf-test");

    let text_output = text_formatter.format_issue(&issue);
    assert!(text_output.contains("bf-test"));
    assert!(text_output.contains("Test"));

    let toon_output = toon_formatter.format_issue(&issue);
    assert!(toon_output.contains("bf-test"));
}

#[test]
fn test_format_with_envelope_single_issue() {
    let formatter = JsonFormatter;

    let issue = Issue {
        id: "bf-env-test".to_string(),
        content_hash: None,
        title: "Envelope Test".to_string(),
        description: Some("Testing envelope wrapping".to_string()),
        design: None,
        acceptance_criteria: None,
        notes: None,
        status: Status::Open,
        priority: Priority(2),
        issue_type: IssueType::Task,
        assignee: Some("test-worker".to_string()),
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
    };

    // First get the raw JSON
    let raw_json = formatter.format_issue(&issue);

    // Then wrap it in an envelope
    let envelope_output = formatter.format_with_envelope("show", &raw_json);

    let parsed: serde_json::Value =
        serde_json::from_str(&envelope_output).expect("Invalid envelope JSON");

    // Verify envelope structure
    assert_eq!(parsed["version"], 1);
    assert_eq!(parsed["kind"], "show");

    // Data field contains the issue as a parsed JSON object (when data is valid JSON)
    assert!(parsed["data"].is_object());

    // The issue data is directly accessible as an object
    let issue_data = &parsed["data"];

    assert_eq!(issue_data["id"], "bf-env-test");
    assert_eq!(issue_data["title"], "Envelope Test");
    assert_eq!(issue_data["assignee"], "test-worker");
}

#[test]
fn test_format_with_envelope_multiple_issues() {
    let formatter = JsonFormatter;

    let issues = vec![
        Issue {
            id: "bf-list-1".to_string(),
            content_hash: None,
            title: "List item 1".to_string(),
            description: None,
            design: None,
            acceptance_criteria: None,
            notes: None,
            status: Status::Open,
            priority: Priority(1),
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
        },
        Issue {
            id: "bf-list-2".to_string(),
            content_hash: None,
            title: "List item 2".to_string(),
            description: None,
            design: None,
            acceptance_criteria: None,
            notes: None,
            status: Status::InProgress,
            priority: Priority(2),
            issue_type: IssueType::Bug,
            assignee: Some("worker".to_string()),
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
            labels: vec!["urgent".to_string()],
            dependencies: vec![],
            comments: vec![],
            annotations: Default::default(),
        },
    ];

    // Get the JSONL output
    let jsonl = formatter.format_issues(&issues);

    // Wrap in envelope - JSONL will fail to parse as JSON and fall back to string
    let envelope_output = formatter.format_with_envelope("list", &jsonl);

    let parsed: serde_json::Value =
        serde_json::from_str(&envelope_output).expect("Invalid envelope JSON");

    // Verify envelope structure
    assert_eq!(parsed["version"], 1);
    assert_eq!(parsed["kind"], "list");

    // When JSONL can't be parsed as JSON, it's stored as a string
    assert!(parsed["data"].is_string());
    let data_str = parsed["data"].as_str().unwrap();

    // Verify it contains 2 lines (2 issues)
    let lines: Vec<&str> = data_str.lines().collect();
    assert_eq!(lines.len(), 2);

    // Verify each line is valid JSON
    for line in lines {
        let issue: serde_json::Value =
            serde_json::from_str(line).expect("Each line should be valid JSON");
        assert!(issue.get("id").is_some());
        assert!(issue.get("title").is_some());
    }
}

#[test]
fn test_format_with_envelope_and_warning() {
    let formatter = JsonFormatter;

    let issue = Issue {
        id: "bf-warn-test".to_string(),
        content_hash: None,
        title: "Warning Test".to_string(),
        description: None,
        design: None,
        acceptance_criteria: None,
        notes: None,
        status: Status::Open,
        priority: Priority(1),
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

    let raw_json = formatter.format_issue(&issue);
    let envelope_output =
        formatter.format_with_envelope_and_warning("show", &raw_json, Some("auto-flush failed"));

    let parsed: serde_json::Value =
        serde_json::from_str(&envelope_output).expect("Invalid envelope JSON");

    // Verify envelope structure
    assert_eq!(parsed["version"], 1);
    assert_eq!(parsed["kind"], "show");

    // Verify warning field is present
    assert!(parsed.get("warning").is_some());
    assert_eq!(parsed["warning"], "auto-flush failed");

    // Data field should still contain the issue (as a parsed object since raw_json is valid JSON)
    assert!(parsed["data"].is_object());
    assert_eq!(parsed["data"]["id"], "bf-warn-test");
    assert_eq!(parsed["data"]["title"], "Warning Test");
}

#[test]
fn test_json_formatter_assignee_and_labels_normalization() {
    let formatter = JsonFormatter;

    // Test with no assignee and no labels
    let mut issue1 = Issue {
        id: "bf-norm-1".to_string(),
        content_hash: None,
        title: "Normalization Test 1".to_string(),
        description: None,
        design: None,
        acceptance_criteria: None,
        notes: None,
        status: Status::Open,
        priority: Priority(1),
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

    let output1 = formatter.format_issue(&issue1);
    let parsed1: serde_json::Value = serde_json::from_str(&output1).expect("Invalid JSON");

    // assignee should be null when not set
    assert!(parsed1.get("assignee").is_some());
    assert_eq!(parsed1["assignee"], serde_json::Value::Null);

    // labels should be an empty array
    assert!(parsed1.get("labels").is_some());
    assert!(parsed1["labels"].is_array());
    assert_eq!(parsed1["labels"].as_array().unwrap().len(), 0);

    // Test with assignee and labels
    issue1.id = "bf-norm-2".to_string();
    issue1.assignee = Some("worker-1".to_string());
    issue1.labels = vec!["phase-1".to_string(), "urgent".to_string()];

    let output2 = formatter.format_issue(&issue1);
    let parsed2: serde_json::Value = serde_json::from_str(&output2).expect("Invalid JSON");

    assert_eq!(parsed2["assignee"], "worker-1");
    assert_eq!(parsed2["labels"].as_array().unwrap().len(), 2);
    assert_eq!(parsed2["labels"][0], "phase-1");
    assert_eq!(parsed2["labels"][1], "urgent");
}
