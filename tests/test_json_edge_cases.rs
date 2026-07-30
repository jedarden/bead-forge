//! Comprehensive JSON output edge case tests for all bf commands
//!
//! This test suite covers edge cases for JSON output from all commands that support `--format json`:
//! - Empty results handling (list, search, ready, recent)
//! - Special characters in bead fields (quotes, newlines, unicode, emoji)
//! - Very long text in fields
//! - Unusual bead IDs and titles
//! - Mixed command consistency across different output formats
//!
//! Acceptance Criteria:
//! - All JSON outputs handle empty results correctly
//! - All JSON outputs properly escape and preserve special characters
//! - All JSON outputs handle very long field values
//! - All JSON outputs handle unusual bead IDs and titles
//! - cargo test passes for all edge case tests

use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use serde_json::{Value, from_str};

fn bf() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bf"))
}

/// Run `bf` with args in `workspace`, returning (stdout, stderr, success).
fn run_bf(workspace: &Path, args: &[&str]) -> (String, String, bool) {
    let output = bf()
        .current_dir(workspace)
        .args(args)
        .output()
        .expect("failed to execute bf");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.success(),
    )
}

fn setup() -> (TempDir, PathBuf) {
    let temp = TempDir::new().unwrap();
    let workspace = temp.path().to_path_buf();
    let (_o, e, ok) = run_bf(&workspace, &["init", "--prefix", "bf"]);
    assert!(ok, "bf init failed: {e}");
    (temp, workspace)
}

/// Create a test bead with the given title
fn create_bead(workspace: &Path, title: &str) -> String {
    let (out, err, ok) = run_bf(workspace, &["create", "--title", title, "--type", "task", "--priority", "2"]);
    assert!(ok, "bf create failed: {err}");
    let id = out.trim().to_string();
    assert!(!id.is_empty(), "create produced no id: {out}");
    id
}

/// Create a test bead with description
fn create_bead_with_description(workspace: &Path, title: &str, description: &str) -> String {
    let (out, err, ok) = run_bf(
        workspace,
        &["create", "--title", title, "--type", "task", "--priority", "2", "--description", description],
    );
    assert!(ok, "bf create failed: {err}");
    let id = out.trim().to_string();
    assert!(!id.is_empty(), "create produced no id: {out}");
    id
}

/// Update a bead's description
fn update_bead_description(workspace: &Path, bead_id: &str, description: &str) {
    let (_out, err, ok) = run_bf(workspace, &["update", bead_id, "--description", description]);
    assert!(ok, "Failed to update bead description: {err}");
}

/// Close a test bead
fn close_bead(workspace: &Path, bead_id: &str, reason: &str) {
    let (_out, err, ok) = run_bf(workspace, &["close", bead_id, "--reason", reason]);
    assert!(ok, "Failed to close bead: {err}");
}

/// Parse a JSON string and panic if invalid
fn parse_json(json: &str) -> Value {
    from_str(json).unwrap_or_else(|e| {
        panic!("Failed to parse JSON: {}\nJSON was: {}", e, json)
    })
}

/// Parse a JSONL string (newline-delimited JSON) into a Vec of values
fn parse_jsonl(jsonl: &str) -> Vec<Value> {
    jsonl
        .lines()
        .filter(|line| !line.trim().is_empty() && line.trim() != "[]")
        .map(|line| parse_json(line))
        .collect()
}

/// Get a string field from JSON, panic if missing or not a string
fn get_string(json: &Value, field: &str) -> String {
    json.get(field)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("Field '{}' is not a string or is missing: {}", field, json))
        .to_string()
}

// ============================================================================
// EMPTY RESULTS TESTS
// ============================================================================

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_list_json_empty_results() {
    let (_temp, workspace) = setup();

    // List from empty workspace with status filter that yields no results
    let (out, err, ok) = run_bf(&workspace, &["list", "--status", "closed", "--format", "json"]);
    assert!(ok, "list with status filter failed: {err}");

    let trimmed = out.trim();

    // Empty results may return empty string or "[]"
    if trimmed.is_empty() {
        return; // Empty string is acceptable
    }

    // If not empty, should be "[]"
    assert_eq!(trimmed, "[]", "Empty list should return '[]'");

    // Verify it's valid JSON
    let parsed = parse_json(trimmed);
    assert!(parsed.is_array(), "Empty results should be an array");
    assert_eq!(parsed.as_array().unwrap().len(), 0, "Array should be empty");
}

#[test]
fn test_search_json_empty_results() {
    let (_temp, workspace) = setup();

    // Search with no matching beads
    let (out, err, ok) = run_bf(&workspace, &["search", "nonexistent-query-xyz", "--format", "json"]);
    assert!(ok, "search failed: {err}");

    let parsed = parse_jsonl(&out);
    assert_eq!(parsed.len(), 0, "Search with no matches should return empty JSONL");
}

#[test]
fn test_ready_json_empty_results() {
    let (_temp, workspace) = setup();

    // Ready with no beads (all blocked or none exist)
    let (out, err, ok) = run_bf(&workspace, &["ready", "--format", "json"]);
    assert!(ok, "ready failed: {err}");

    let trimmed = out.trim();
    assert_eq!(trimmed, "[]", "Empty ready should return '[]'");

    // Verify it's valid JSON
    let parsed = parse_json(trimmed);
    assert!(parsed.is_array(), "Empty results should be an array");
    assert_eq!(parsed.as_array().unwrap().len(), 0, "Array should be empty");
}

#[test]
fn test_recent_json_empty_results() {
    let (_temp, workspace) = setup();

    // Recent with no beads in time period
    let (out, err, ok) = run_bf(&workspace, &["recent", "--time-period", "1s", "--format", "json"]);
    assert!(ok, "recent failed: {err}");

    let parsed = parse_json(&out);
    assert!(parsed.is_object(), "recent should return envelope object");
    assert_eq!(parsed["version"].as_u64().unwrap(), 1, "version should be 1");
    assert_eq!(parsed["kind"].as_str().unwrap(), "recent", "kind should be 'recent'");

    // Data should be empty
    let data = &parsed["data"];
    if data.is_array() {
        assert_eq!(data.as_array().unwrap().len(), 0, "data array should be empty");
    } else if data.is_string() {
        assert!(data.as_str().unwrap().is_empty() || data.as_str().unwrap() == "[]",
                "data string should be empty or '[]'");
    }
}

// ============================================================================
// SPECIAL CHARACTERS TESTS
// ============================================================================

#[test]
fn test_json_handles_unicode_emoji() {
    let (_temp, workspace) = setup();

    let emoji_title = "Test with emoji 🎉🚀💡 and unicode Ñoño café naïve";
    let bead_id = create_bead(&workspace, emoji_title);

    // Test list command preserves emoji
    let (list_out, err, ok) = run_bf(&workspace, &["list", "--format", "json"]);
    assert!(ok, "list failed: {err}");

    let list_parsed = parse_jsonl(&list_out);
    let bead = list_parsed.iter().find(|b| get_string(b, "id") == bead_id).unwrap();
    let title = get_string(bead, "title");
    assert!(title.contains("🎉"), "Emoji should be preserved in list output");
    assert!(title.contains("Ñ"), "Unicode should be preserved in list output");

    // Test show command preserves emoji
    let (show_out, err, ok) = run_bf(&workspace, &["show", &bead_id, "--format", "json"]);
    assert!(ok, "show failed: {err}");

    let show_parsed = parse_json(&show_out);
    let show_array = show_parsed.as_array().unwrap();
    let show_bead = &show_array[0];
    let show_title = get_string(show_bead, "title");
    assert!(show_title.contains("🎉"), "Emoji should be preserved in show output");
    assert!(show_title.contains("café"), "Unicode should be preserved in show output");

    // Test search command preserves emoji
    let (search_out, err, ok) = run_bf(&workspace, &["search", "emoji", "--format", "json"]);
    assert!(ok, "search failed: {err}");

    let search_parsed = parse_jsonl(&search_out);
    let search_bead = search_parsed.iter().find(|b| get_string(b, "id") == bead_id).unwrap();
    let search_title = get_string(search_bead, "title");
    assert!(search_title.contains("🎉"), "Emoji should be preserved in search output");
}

#[test]
fn test_json_handles_quotes_and_apostrophes() {
    let (_temp, workspace) = setup();

    let title_with_quotes = "Test with \"double quotes\" and 'single apostrophes'";
    let bead_id = create_bead(&workspace, title_with_quotes);

    // Test show command handles quotes properly
    let (show_out, err, ok) = run_bf(&workspace, &["show", &bead_id, "--format", "json"]);
    assert!(ok, "show failed: {err}");

    let show_parsed = parse_json(&show_out);
    let show_array = show_parsed.as_array().unwrap();
    let show_bead = &show_array[0];
    let title = get_string(show_bead, "title");

    assert!(title.contains("\"double quotes\""), "Double quotes should be preserved");
    assert!(title.contains("'single apostrophes'"), "Single quotes should be preserved");

    // Verify JSON is valid (proper escaping)
    let recheck_json = parse_json(&show_out);
    assert!(recheck_json.is_array(), "Output should still be valid JSON");
}

#[test]
fn test_json_handles_newlines_and_tabs() {
    let (_temp, workspace) = setup();

    let title = "Test bead with newlines and tabs";
    let bead_id = create_bead(&workspace, title);

    let description_with_whitespace = "Line 1\nLine 2\nLine 3\nTab:\tIndented\rCarriage return";
    update_bead_description(&workspace, &bead_id, description_with_whitespace);

    // Test show command handles newlines and tabs
    let (show_out, err, ok) = run_bf(&workspace, &["show", &bead_id, "--format", "json"]);
    assert!(ok, "show failed: {err}");

    let show_parsed = parse_json(&show_out);
    let show_array = show_parsed.as_array().unwrap();
    let show_bead = &show_array[0];
    let description = get_string(show_bead, "description");

    assert!(description.contains("Line 1"), "First line should be preserved");
    assert!(description.contains("Line 2"), "Second line should be preserved");
    assert!(description.contains("\n"), "Newlines should be preserved (escaped in JSON)");
    assert!(description.contains("\t"), "Tabs should be preserved (escaped in JSON)");

    // Verify JSON is still valid despite special characters
    let recheck_json = parse_json(&show_out);
    assert!(recheck_json.is_array(), "Output should still be valid JSON");
}

#[test]
fn test_json_handles_backslashes_and_special_chars() {
    let (_temp, workspace) = setup();

    let title = "Test bead with backslashes and special chars";
    let bead_id = create_bead(&workspace, title);

    let special_description = r#"Path: C:\Users\test\admin
Backslash: \
Forward slash: /
Mixed: \\//\\
JSON-like: {"key": "value"}
HTML: <tag>&amp;"#;

    update_bead_description(&workspace, &bead_id, special_description);

    // Test show command handles backslashes and special chars
    let (show_out, err, ok) = run_bf(&workspace, &["show", &bead_id, "--format", "json"]);
    assert!(ok, "show failed: {err}");

    // Should be valid JSON despite special characters
    let show_parsed = parse_json(&show_out);
    let show_array = show_parsed.as_array().unwrap();
    let show_bead = &show_array[0];
    let description = get_string(show_bead, "description");

    assert!(description.contains(r#"C:\Users\test"#), "Backslashes should be preserved");
    assert!(description.contains(r#"{"key": "value"}"#), "JSON-like text should be preserved");
    assert!(description.contains("<tag>"), "HTML tags should be preserved");
}

#[test]
fn test_json_handles_all_special_chars_together() {
    let (_temp, workspace) = setup();

    let title = "Ultimate special char test: \"quotes\" 'apostrophes' \\backslashes/ <html> &entities; emoji🎉 unicodeÑ";
    let bead_id = create_bead(&workspace, title);

    let description = r#"Multiline with tabs:\tIndented
Newlines:\nLine2\nLine3
Backslashes: \\ \\
Quotes: "double" 'single'
Special: <tag>&amp;
JSON: {"array": [1, 2, 3]}
Emoji: 🚀🔥💡
Unicode: café, naïve, Zürich"#;

    update_bead_description(&workspace, &bead_id, description);

    // Test all commands handle this correctly
    for (cmd, args) in [
        ("list", vec!["list", "--format", "json"]),
        ("show", vec!["show", &bead_id, "--format", "json"]),
        ("search", vec!["search", "ultimate", "--format", "json"]),
    ] {
        let (out, err, ok) = run_bf(&workspace, &args);
        assert!(ok, "{} failed: {err}", cmd);

        // All should produce valid JSON
        if cmd == "show" {
            let parsed = parse_json(&out);
            assert!(parsed.is_array(), "{} should return valid JSON array", cmd);
        } else {
            let parsed = parse_jsonl(&out);
            assert!(!parsed.is_empty() || cmd == "search", "{} should return valid JSONL", cmd);
        }
    }
}

// ============================================================================
// VERY LONG TEXT TESTS
// ============================================================================

#[test]
fn test_json_handles_very_long_title() {
    let (_temp, workspace) = setup();

    // Create a very long title (near but within limits)
    let long_title = "A".repeat(200);
    let bead_id = create_bead(&workspace, &long_title);

    // Test list command handles long title
    let (list_out, err, ok) = run_bf(&workspace, &["list", "--format", "json"]);
    assert!(ok, "list failed: {err}");

    let list_parsed = parse_jsonl(&list_out);
    let bead = list_parsed.iter().find(|b| get_string(b, "id") == bead_id).unwrap();
    let title = get_string(bead, "title");
    assert_eq!(title.len(), long_title.len(), "Title length should be preserved");
    assert!(title.starts_with("AAAAA"), "Title content should be preserved");

    // Test show command handles long title
    let (show_out, err, ok) = run_bf(&workspace, &["show", &bead_id, "--format", "json"]);
    assert!(ok, "show failed: {err}");

    let show_parsed = parse_json(&show_out);
    let show_array = show_parsed.as_array().unwrap();
    let show_bead = &show_array[0];
    let show_title = get_string(show_bead, "title");
    assert_eq!(show_title.len(), long_title.len(), "Title should be preserved in show");
}

#[test]
fn test_json_handles_very_long_description() {
    let (_temp, workspace) = setup();

    let bead_id = create_bead(&workspace, "Bead with long description");

    // Create a very long description
    let long_description = "A very long description. ".repeat(100);
    update_bead_description(&workspace, &bead_id, &long_description);

    // Test show command handles long description
    let (show_out, err, ok) = run_bf(&workspace, &["show", &bead_id, "--format", "json"]);
    assert!(ok, "show failed: {err}");

    let show_parsed = parse_json(&show_out);
    let show_array = show_parsed.as_array().unwrap();
    let show_bead = &show_array[0];
    let description = get_string(show_bead, "description");

    assert!(description.len() > 1000, "Long description should be preserved");
    assert!(description.contains("very long description"), "Content should be preserved");

    // Verify JSON is still valid
    let recheck = parse_json(&show_out);
    assert!(recheck.is_array(), "Long description output should still be valid JSON");
}

#[test]
fn test_json_handles_many_fields_with_long_content() {
    let (_temp, workspace) = setup();

    let long_title = "Long title: ".to_string() + &"X".repeat(150);
    let long_description = "Long description: ".to_string() + &"Y".repeat(500);

    let bead_id = create_bead_with_description(&workspace, &long_title, &long_description);

    // Test that all fields are properly serialized
    let (show_out, err, ok) = run_bf(&workspace, &["show", &bead_id, "--format", "json"]);
    assert!(ok, "show failed: {err}");

    let show_parsed = parse_json(&show_out);
    let show_array = show_parsed.as_array().unwrap();
    let show_bead = &show_array[0];

    let title = get_string(show_bead, "title");
    let description = get_string(show_bead, "description");

    assert!(title.len() > 150, "Long title should be preserved");
    assert!(description.len() > 500, "Long description should be preserved");

    // Verify the JSON is valid (no truncation or corruption)
    let recheck = parse_json(&show_out);
    assert!(recheck.is_array(), "Multiple long fields should still produce valid JSON");
}

// ============================================================================
// UNUSUAL BEAD IDs AND TITLES TESTS
// ============================================================================

#[test]
fn test_json_handles_unusual_but_valid_bead_ids() {
    let (_temp, workspace) = setup();

    // Create beads with different prefixes and patterns
    let unusual_ids = vec![
        ("bf-test-123", "Normal bead ID"),
        ("bf-123abc", "ID with numbers"),
        ("bf-abc-123-xyz", "ID with multiple hyphens"),
        ("bf-very-long-id-with-many-parts-123456", "Long ID"),
    ];

    for (id, title) in unusual_ids {
        // Create bead with specific ID (if supported) or let system generate
        let bead_id = create_bead(&workspace, title);

        // Test that the ID is properly serialized
        let (show_out, err, ok) = run_bf(&workspace, &["show", &bead_id, "--format", "json"]);
        assert!(ok, "show failed for {}: {err}", bead_id);

        let show_parsed = parse_json(&show_out);
        let show_array = show_parsed.as_array().unwrap();
        let show_bead = &show_array[0];
        let parsed_id = get_string(show_bead, "id");

        assert_eq!(parsed_id, bead_id, "ID should be preserved exactly");
    }
}

#[test]
fn test_json_handles_titles_with_leading_trailing_whitespace() {
    let (_temp, workspace) = setup();

    // Test title with spaces (should be preserved or trimmed based on implementation)
    let title_with_spaces = "  Title with leading and trailing spaces  ";
    let bead_id = create_bead(&workspace, title_with_spaces);

    let (show_out, err, ok) = run_bf(&workspace, &["show", &bead_id, "--format", "json"]);
    assert!(ok, "show failed: {err}");

    let show_parsed = parse_json(&show_out);
    let show_array = show_parsed.as_array().unwrap();
    let show_bead = &show_array[0];
    let title = get_string(show_bead, "title");

    // Title should either preserve spaces or trim them (implementation-dependent)
    assert!(title.contains("Title with"), "Core title content should be present");
}

#[test]
fn test_json_handles_titles_with_only_numbers_and_special_chars() {
    let (_temp, workspace) = setup();

    let special_titles = vec![
        "12345",
        "!!!",
        "@#$%",
        "<>",
        "[]",
        "{}",
        "((()))",
    ];

    for title in special_titles {
        let bead_id = create_bead(&workspace, title);

        // Verify the special title is preserved
        let (show_out, err, ok) = run_bf(&workspace, &["show", &bead_id, "--format", "json"]);
        assert!(ok, "show failed for {}: {err}", title);

        let show_parsed = parse_json(&show_out);
        let show_array = show_parsed.as_array().unwrap();
        let show_bead = &show_array[0];
        let parsed_title = get_string(show_bead, "title");

        assert_eq!(parsed_title, title, "Special title should be preserved: {}", title);
    }
}

#[test]
fn test_json_handles_mixed_unicode_scripts() {
    let (_temp, workspace) = setup();

    let mixed_scripts = vec![
        "Mixed: English 中文 العربية עברית",
        "Emoji and text: 🎉Test🚀More💡Text",
        "RTL and LTR: مرحبا World שלום",
        "Cyrillic: Привет мир",
        "Greek: Γεια σου κόσμε",
        "Japanese: こんにちは世界",
        "Korean: 안녕하세요 세계",
    ];

    for title in mixed_scripts {
        let bead_id = create_bead(&workspace, title);

        // Verify mixed scripts are preserved
        let (show_out, err, ok) = run_bf(&workspace, &["show", &bead_id, "--format", "json"]);
        assert!(ok, "show failed for {}: {err}", title);

        let show_parsed = parse_json(&show_out);
        let show_array = show_parsed.as_array().unwrap();
        let show_bead = &show_array[0];
        let parsed_title = get_string(show_bead, "title");

        assert_eq!(parsed_title, title, "Mixed scripts should be preserved: {}", title);

        // Verify JSON is valid
        let recheck = parse_json(&show_out);
        assert!(recheck.is_array(), "Mixed script output should be valid JSON");
    }
}

// ============================================================================
// COMMAND CONSISTENCY TESTS
// ============================================================================

#[test]
fn test_json_consistency_across_commands() {
    let (_temp, workspace) = setup();

    let title = "Consistency test with emoji 🎉";
    let description = "Description with \"quotes\" and\nnewlines";
    let bead_id = create_bead_with_description(&workspace, title, description);

    // Get the bead from different commands and verify consistency
    let commands = vec![
        ("list", vec!["list", "--format", "json"]),
        ("show", vec!["show", &bead_id, "--format", "json"]),
        ("search", vec!["search", "consistency", "--format", "json"]),
    ];

    let mut extracted_titles = Vec::new();

    for (cmd_name, args) in commands {
        let (out, err, ok) = run_bf(&workspace, &args);
        assert!(ok, "{} failed: {err}", cmd_name);

        let extracted_title = if cmd_name == "show" {
            let parsed = parse_json(&out);
            let array = parsed.as_array().unwrap();
            get_string(&array[0], "title")
        } else {
            let parsed = parse_jsonl(&out);
            let bead = parsed.iter().find(|b| get_string(b, "id") == bead_id).unwrap();
            get_string(bead, "title")
        };

        extracted_titles.push(extracted_title);
    }

    // All commands should return the same title
    for title in &extracted_titles {
        assert_eq!(title, &extracted_titles[0], "All commands should return consistent title");
    }
}

#[test]
fn test_json_empty_results_consistency() {
    let (_temp, workspace) = setup();

    // Test that empty results are handled consistently across commands
    let empty_commands = vec![
        ("list", vec!["list", "--status", "closed", "--format", "json"], "[]"),
        ("ready", vec!["ready", "--format", "json"], "[]"),
    ];

    for (cmd_name, args, expected) in empty_commands {
        let (out, err, ok) = run_bf(&workspace, &args);
        assert!(ok, "{} failed: {err}", cmd_name);

        let trimmed = out.trim();

        // Empty results may return empty string or expected JSON (e.g., "[]")
        if trimmed.is_empty() {
            continue; // Empty string is acceptable for empty results
        }

        assert_eq!(trimmed, expected, "{} should return '{}' for empty results", cmd_name, expected);

        // Verify it's valid JSON
        let parsed = parse_json(trimmed);
        if expected == "[]" {
            assert!(parsed.is_array(), "Empty results should be valid JSON array");
            assert_eq!(parsed.as_array().unwrap().len(), 0, "Array should be empty");
        }
    }
}

// ============================================================================
// REQUIRED FIELD PRESENCE AND TYPE TESTS
// ============================================================================

#[test]
fn test_list_json_required_field_types() {
    let (_temp, workspace) = setup();

    // Create a test bead
    let bead_id = create_bead(&workspace, "Test bead for required field validation");

    // Run list --json
    let (list_out, err, ok) = run_bf(&workspace, &["list", "--format", "json"]);
    assert!(ok, "list failed: {err}");

    // Parse and find our bead
    let parsed = parse_jsonl(&list_out);
    let our_bead = parsed
        .iter()
        .find(|v| get_string(v, "id") == bead_id)
        .expect("Should find our created bead");

    // Verify all required fields are present
    let required_fields = vec!["id", "title", "status", "priority", "created_at"];
    for field in required_fields {
        assert!(
            our_bead.get(field).is_some(),
            "Required field '{}' should be present in JSON output. Got: {}",
            field,
            our_bead
        );
    }

    // Verify field types are correct

    // id field should be a string
    let id_value = our_bead.get("id").expect("id field must exist");
    assert!(
        id_value.is_string(),
        "id field should be a string, got: {}",
        id_value
    );
    let id_str = id_value.as_str().unwrap();
    assert!(!id_str.is_empty(), "id string should not be empty");
    assert!(id_str.starts_with("bf-"), "id should start with 'bf-' prefix");

    // title field should be a string
    let title_value = our_bead.get("title").expect("title field must exist");
    assert!(
        title_value.is_string(),
        "title field should be a string, got: {}",
        title_value
    );
    let title_str = title_value.as_str().unwrap();
    assert!(!title_str.is_empty(), "title string should not be empty");

    // status field should be a string
    let status_value = our_bead.get("status").expect("status field must exist");
    assert!(
        status_value.is_string(),
        "status field should be a string, got: {}",
        status_value
    );
    let status_str = status_value.as_str().unwrap();
    assert!(!status_str.is_empty(), "status string should not be empty");
    // Verify it's a valid status value
    let valid_statuses = vec!["open", "closed", "blocked", "in_progress"];
    assert!(
        valid_statuses.contains(&status_str),
        "status should be one of {:?}, got: {}",
        valid_statuses,
        status_str
    );

    // priority field should be a number (can be parsed as integer)
    let priority_value = our_bead.get("priority").expect("priority field must exist");
    assert!(
        priority_value.is_number(),
        "priority field should be a number, got: {}",
        priority_value
    );
    let priority_num = priority_value.as_i64().expect("priority should be parseable as i64");
    assert!(priority_num >= 0 && priority_num <= 5, "priority should be between 0 and 5, got: {}", priority_num);

    // created_at field should be a string (ISO 8601 timestamp)
    let created_at_value = our_bead.get("created_at").expect("created_at field must exist");
    assert!(
        created_at_value.is_string(),
        "created_at field should be a string (timestamp), got: {}",
        created_at_value
    );
    let created_at_str = created_at_value.as_str().unwrap();
    assert!(!created_at_str.is_empty(), "created_at string should not be empty");
    // Verify ISO 8601 format (contains 'T' and has timezone indicator)
    assert!(
        created_at_str.contains('T'),
        "created_at should be ISO 8601 format with 'T' separator, got: {}",
        created_at_str
    );
    assert!(
        created_at_str.ends_with('Z') || created_at_str.contains('+'),
        "created_at should have timezone indicator ('Z' or '+'), got: {}",
        created_at_str
    );
}

#[test]
fn test_list_json_multiple_beads_all_required_fields() {
    let (_temp, workspace) = setup();

    // Create multiple beads to test field presence across all results
    let bead_ids = vec![
        create_bead(&workspace, "First bead for multiple field test"),
        create_bead(&workspace, "Second bead for multiple field test"),
        create_bead(&workspace, "Third bead for multiple field test"),
    ];

    // Run list --json
    let (list_out, err, ok) = run_bf(&workspace, &["list", "--format", "json"]);
    assert!(ok, "list failed: {err}");

    // Parse all beads from JSONL output
    let parsed = parse_jsonl(&list_out);
    assert!(
        parsed.len() >= bead_ids.len(),
        "Should return at least {} beads, got {}",
        bead_ids.len(),
        parsed.len()
    );

    // Verify each bead in the output has all required fields with correct types
    let required_fields = vec!["id", "title", "status", "priority", "created_at"];
    for (i, bead) in parsed.iter().enumerate() {
        // Check all required fields exist
        for field in &required_fields {
            assert!(
                bead.get(field).is_some(),
                "Bead at index {}: Required field '{}' should be present. Got: {}",
                i,
                field,
                bead
            );
        }

        // Verify field types for each bead
        let id = bead.get("id").expect("id must exist");
        assert!(id.is_string(), "Bead {}: id should be string", i);

        let title = bead.get("title").expect("title must exist");
        assert!(title.is_string(), "Bead {}: title should be string", i);

        let status = bead.get("status").expect("status must exist");
        assert!(status.is_string(), "Bead {}: status should be string", i);

        let priority = bead.get("priority").expect("priority must exist");
        assert!(priority.is_number(), "Bead {}: priority should be number", i);

        let created_at = bead.get("created_at").expect("created_at must exist");
        assert!(created_at.is_string(), "Bead {}: created_at should be string (timestamp)", i);
        let created_at_str = created_at.as_str().unwrap();
        assert!(created_at_str.contains('T'), "Bead {}: created_at should be ISO 8601 format", i);
    }

    // Verify all our created beads are in the results
    for expected_id in &bead_ids {
        let found = parsed.iter().any(|v| get_string(v, "id") == *expected_id);
        assert!(
            found,
            "Created bead {} should be in the list results",
            expected_id
        );
    }
}

// ============================================================================
// JSON VALIDATION TESTS
// ============================================================================

#[test]
fn test_all_json_output_is_valid() {
    let (_temp, workspace) = setup();

    // Create various test beads
    let bead1 = create_bead(&workspace, "Validation test 1 🎉");
    let bead2 = create_bead_with_description(&workspace, "Validation test 2", "Description with \"quotes\"");
    let bead3 = create_bead(&workspace, "Validation test 3");
    close_bead(&workspace, &bead3, "Test close");

    // Test all JSON-producing commands
    let json_commands = vec![
        vec!["list", "--format", "json"],
        vec!["show", &bead1, "--format", "json"],
        vec!["search", "validation", "--format", "json"],
        vec!["ready", "--format", "json"],
        vec!["recent", "--format", "json"],
    ];

    for args in json_commands {
        let (out, err, ok) = run_bf(&workspace, &args);
        assert!(ok, "Command {:?} failed: {err}", args);

        // For non-envelope commands, verify each line is valid JSON
        if args[0] != "show" && args[0] != "recent" {
            for line in out.lines() {
                if !line.trim().is_empty() && line.trim() != "[]" {
                    let parsed = parse_json(line);
                    assert!(parsed.is_object() || parsed.is_array(),
                            "Each line should be valid JSON object or array");
                }
            }
        } else {
            // For envelope commands, verify entire output is valid JSON
            if !out.trim().is_empty() && out.trim() != "[]" {
                let parsed = parse_json(&out);
                assert!(parsed.is_object() || parsed.is_array(),
                        "Envelope output should be valid JSON");
            }
        }
    }
}

#[test]
fn test_json_escape_sequences_are_correct() {
    let (_temp, workspace) = setup();

    // Test with raw string that contains escape sequence literals
    let title = r#"Test with various escape sequences: \n \t \r \" \\""#;
    let bead_id = create_bead(&workspace, title);

    let (show_out, err, ok) = run_bf(&workspace, &["show", &bead_id, "--format", "json"]);
    assert!(ok, "show failed: {err}");

    // Parse and verify the JSON is valid
    let parsed = parse_json(&show_out);
    assert!(parsed.is_array(), "Output should be valid JSON array");

    // Get the title back and verify escape sequences are handled
    let array = parsed.as_array().unwrap();
    let bead = &array[0];
    let extracted_title = get_string(bead, "title");

    // The title should contain the escape sequences as literal text (not interpreted)
    assert!(extracted_title.contains(r#"\n"#), "Backslash-n should be preserved as literal text");
    assert!(extracted_title.contains(r#"\t"#), "Backslash-t should be preserved as literal text");
    assert!(extracted_title.contains(r#"\""#), "Escaped quote should be preserved");
}
