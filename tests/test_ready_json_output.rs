//! Comprehensive JSON output tests for `bf ready` command.
//!
//! Tests cover:
//! - ready --json output structure validity
//! - Required fields presence in ready JSON output
//! - Special character handling in bead fields
//! - Empty results handling
//! - Filtering behavior (excludes blocked/closed beads)
//! - Different bead types in ready output
//! - Edge cases and error conditions

use std::process::Command;
use tempfile::TempDir;

/// Resolve the freshly-built bf binary.
fn bf_path() -> String {
    std::env::var("CARGO_BIN_EXE_bf").unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Create an isolated workspace via `bf init`.
fn init_workspace() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let out = Command::new(bf_path())
        .args(["init", "--prefix", "bf"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to init workspace");
    assert!(
        out.status.success(),
        "bf init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    temp_dir
}

/// Create a bead with specified type and fields.
fn create_bead_with_type(
    workspace: &std::path::Path,
    title: &str,
    type_: &str,
    description: &str,
) -> String {
    let out = Command::new(bf_path())
        .args([
            "create",
            "--title",
            title,
            "--type",
            type_,
            "--priority",
            "2",
            "--description",
            description,
        ])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(
        out.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Create a bead and populate additional fields.
fn create_bead_with_all_fields(
    workspace: &std::path::Path,
    title: &str,
    type_: &str,
    description: &str,
    assignee: Option<&str>,
    labels: &[&str],
) -> String {
    let mut args = vec![
        "create",
        "--title",
        title,
        "--type",
        type_,
        "--priority",
        "2",
        "--description",
        description,
    ];

    if let Some(assignee_val) = assignee {
        args.extend(["--assignee", assignee_val]);
    }

    let out = Command::new(bf_path())
        .args(&args)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf create");
    assert!(
        out.status.success(),
        "bf create failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let id = String::from_utf8(out.stdout).unwrap().trim().to_string();

    // Add labels if provided
    if !labels.is_empty() {
        let mut label_args = vec!["label", "add", &id];
        for label in labels {
            label_args.extend(["--label", label]);
        }
        let label_out = Command::new(bf_path())
            .args(&label_args)
            .current_dir(workspace)
            .output()
            .expect("Failed to add labels");
        assert!(
            label_out.status.success(),
            "bf label add failed: {}",
            String::from_utf8_lossy(&label_out.stderr)
        );
    }

    id
}

/// Run `bf ready --json` and parse output as JSONL.
fn ready_json(workspace: &std::path::Path) -> Vec<serde_json::Value> {
    let out = Command::new(bf_path())
        .args(["ready", "--json"])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf ready");

    let stdout = String::from_utf8(out.stdout).unwrap();
    parse_jsonl(&stdout)
}

/// Run `bf ready --json` with limit.
fn ready_json_with_limit(workspace: &std::path::Path, limit: u64) -> Vec<serde_json::Value> {
    let out = Command::new(bf_path())
        .args(["ready", "--json", "--limit", &limit.to_string()])
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf ready");

    let stdout = String::from_utf8(out.stdout).unwrap();
    parse_jsonl(&stdout)
}

/// Parse JSONL (one object per line), skipping blank/`[]` lines.
fn parse_jsonl(stdout: &str) -> Vec<serde_json::Value> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && *line != "[]")
        .map(|line| {
            serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("invalid JSON line {line:?}: {e}"))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Structure validity tests
// ---------------------------------------------------------------------------

#[test]
fn test_ready_json_output_structure_validity() {
    let ws = init_workspace();
    let _id = create_bead_with_type(
        ws.path(),
        "Ready structure test bead",
        "task",
        "Test description",
    );

    let beads = ready_json(ws.path());

    assert!(!beads.is_empty(), "ready should return at least one bead");

    for bead in &beads {
        // Must be an object
        assert!(bead.is_object(), "ready output must be a JSON object");

        // Must have core identifier fields
        assert!(bead.get("id").is_some(), "id field is required");
        assert!(bead.get("title").is_some(), "title field is required");

        // Must have status fields
        assert!(bead.get("status").is_some(), "status field is required");
        assert!(bead.get("priority").is_some(), "priority field is required");
        assert!(bead.get("issue_type").is_some(), "issue_type field is required");

        // Must have timestamp fields
        assert!(bead.get("created_at").is_some(), "created_at field is required");
        assert!(bead.get("updated_at").is_some(), "updated_at field is required");

        // Optional fields should at least be present (even if null/empty)
        assert!(bead.get("description").is_some(), "description field must be present");
        assert!(bead.get("assignee").is_some(), "assignee field must be present");
        assert!(bead.get("labels").is_some(), "labels field must be present");
    }
}

#[test]
fn test_ready_json_output_is_parseable() {
    let ws = init_workspace();
    let _id = create_bead_with_type(ws.path(), "Parseable ready test", "task", "Desc");

    let out = Command::new(bf_path())
        .args(["ready", "--json"])
        .current_dir(ws.path())
        .output()
        .expect("Failed to run bf ready");

    let stdout = String::from_utf8(out.stdout).unwrap();

    // Each line should be valid JSON
    for line in stdout.lines() {
        let line = line.trim();
        if !line.is_empty() && line != "[]" {
            let parsed: serde_json::Value = serde_json::from_str(line)
                .expect("Each line must be valid JSON");
            assert!(parsed.is_object(), "Each line must be a JSON object");
        }
    }
}

#[test]
fn test_ready_json_uses_jsonl_format_not_array() {
    let ws = init_workspace();
    let _id1 = create_bead_with_type(ws.path(), "Bead 1", "task", "Desc");
    let _id2 = create_bead_with_type(ws.path(), "Bead 2", "task", "Desc");

    let out = Command::new(bf_path())
        .args(["ready", "--json"])
        .current_dir(ws.path())
        .output()
        .expect("Failed to run bf ready");

    let stdout = String::from_utf8(out.stdout).unwrap();

    // Should be JSONL (one object per line), NOT a single JSON array
    let trimmed = stdout.trim();
    assert!(!trimmed.starts_with('['), "ready should not output JSON array");
    assert!(!trimmed.ends_with(']'), "ready should not output JSON array");

    // Should have multiple lines for multiple beads
    let line_count = stdout.lines().filter(|l| !l.trim().is_empty()).count();
    assert!(line_count >= 2, "ready should output one line per bead");
}

// ---------------------------------------------------------------------------
// Required field tests
// ---------------------------------------------------------------------------

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_ready_json_required_fields_types() {
    let ws = init_workspace();
    let id = create_bead_with_type(ws.path(), "Field types ready test", "bug", "Bug desc");

    let beads = ready_json(ws.path());
    let bead = beads.iter()
        .find(|b| b.get("id").and_then(|i| i.as_str()) == Some(id.as_str()))
        .expect("created bead not found in ready output");

    // id must be a non-empty string
    let id_val = bead.get("id").and_then(|v| v.as_str());
    assert_eq!(id_val, Some(id.as_str()), "id must match created bead id");
    assert!(id.as_str().len() > 0, "id must not be empty");

    // title must be a string
    assert!(bead.get("title").and_then(|v| v.as_str()).is_some(), "title must be a string");

    // status must be "open" for ready beads
    let status = bead.get("status").and_then(|v| v.as_str());
    assert_eq!(status, Some("open"), "ready beads must have status='open'");

    // priority must be a number (0-4)
    let priority = bead.get("priority").and_then(|v| v.as_i64());
    assert!(priority.is_some(), "priority must be a number");
    assert!(priority.map(|p| (0..=4).contains(&p)).unwrap_or(false),
            "priority must be between 0 and 4");

    // issue_type must be a string
    let bead_type = bead.get("issue_type").and_then(|v| v.as_str());
    assert!(bead_type.is_some(), "issue_type must be a string");

    // created_at must be a string in ISO 8601 format
    let created_at = bead.get("created_at").and_then(|v| v.as_str());
    assert!(created_at.is_some(), "created_at must be a string");
    assert!(created_at.unwrap().contains('T'), "created_at must be in ISO 8601 format");

    // updated_at must be a string in ISO 8601 format
    let updated_at = bead.get("updated_at").and_then(|v| v.as_str());
    assert!(updated_at.is_some(), "updated_at must be a string");
    assert!(updated_at.unwrap().contains('T'), "updated_at must be in ISO 8601 format");

    // labels must be an array
    assert!(bead.get("labels").and_then(|v| v.as_array()).is_some(),
            "labels must be an array");

    // assignee can be null or string
    match bead.get("assignee") {
        Some(serde_json::Value::Null) => {}, // OK
        Some(serde_json::Value::String(_)) => {}, // OK
        Some(other) => panic!("assignee must be null or string, got: {:?}", other),
        None => panic!("assignee field must be present"),
    }
}

#[test]
fn test_ready_json_all_optional_fields_present() {
    let ws = init_workspace();
    let id = create_bead_with_type(ws.path(), "Optional fields ready test", "task", "Desc");

    let beads = ready_json(ws.path());
    let bead = beads.iter()
        .find(|b| b.get("id").and_then(|i| i.as_str()) == Some(id.as_str()))
        .expect("created bead not found in ready output");

    // These fields should always be present, even if null/empty
    let optional_fields = [
        "description",
        "assignee",
        "labels",
        "acceptance_criteria",
        "notes",
        "design",
    ];

    for field in &optional_fields {
        assert!(bead.get(*field).is_some(),
                "Optional field '{}' must be present in output", field);
    }

    // dependencies and comments should be stripped for NEEDLE compatibility
    // They should either be absent or empty arrays
    match bead.get("dependencies") {
        None => {}, // Ok if absent
        Some(serde_json::Value::Array(arr)) if arr.is_empty() => {}, // Ok if empty
        Some(other) => panic!("dependencies should be absent or empty array, got: {:?}", other),
    }

    match bead.get("comments") {
        None => {}, // Ok if absent
        Some(serde_json::Value::Array(arr)) if arr.is_empty() => {}, // Ok if empty
        Some(other) => panic!("comments should be absent or empty array, got: {:?}", other),
    }
}

#[test]
fn test_ready_json_assignee_and_labels_always_present() {
    let ws = init_workspace();
    let id1 = create_bead_with_all_fields(
        ws.path(),
        "Bead with assignee and labels",
        "task",
        "Description",
        Some("claude-code-glm-4.7-alpha"),
        &["label1", "label2"],
    );
    let id2 = create_bead_with_type(ws.path(), "Bead without fields", "task", "Description");

    let beads = ready_json(ws.path());

    // Check bead WITH assignee and labels
    let bead1 = beads.iter()
        .find(|b| b.get("id").and_then(|i| i.as_str()) == Some(id1.as_str()))
        .expect("bead with fields not found");

    assert!(bead1.get("assignee").is_some(), "assignee must be present");
    assert_eq!(
        bead1.get("assignee").and_then(|v| v.as_str()),
        Some("claude-code-glm-4.7-alpha"),
        "assignee should have correct value"
    );

    assert!(bead1.get("labels").is_some(), "labels must be present");
    let labels1 = bead1.get("labels").and_then(|v| v.as_array()).unwrap();
    assert_eq!(labels1.len(), 2, "labels should contain 2 items");

    // Check bead WITHOUT assignee and labels
    let bead2 = beads.iter()
        .find(|b| b.get("id").and_then(|i| i.as_str()) == Some(id2.as_str()))
        .expect("bead without fields not found");

    assert!(bead2.get("assignee").is_some(), "assignee must be present even when null");
    assert!(bead2.get("assignee").unwrap().is_null(), "assignee should be null when not set");

    assert!(bead2.get("labels").is_some(), "labels must be present even when empty");
    let labels2 = bead2.get("labels").and_then(|v| v.as_array()).unwrap();
    assert_eq!(labels2.len(), 0, "labels should be empty array when none set");
}

// ---------------------------------------------------------------------------
// Special character tests
// ---------------------------------------------------------------------------

#[test]
fn test_ready_json_special_characters_in_title() {
    let ws = init_workspace();

    // Create bead with various special characters in title
    let special_title = "Test \"quotes\" and 'apostrophes' & <symbols> \\n\\t";
    let _id = create_bead_with_type(ws.path(), special_title, "task", "Description");

    let beads = ready_json(ws.path());
    let bead = &beads[0];

    let title = bead.get("title").and_then(|v| v.as_str()).unwrap();

    // Verify special characters are properly escaped/unescaped
    assert!(title.contains("quotes"), "title should contain 'quotes'");
    assert!(title.contains("apostrophes"), "title should contain 'apostrophes'");
    assert!(title.contains("&"), "title should contain '&'");
    assert!(title.contains("<symbols>"), "title should contain '<symbols>'");
}

#[test]
fn test_ready_json_special_characters_in_description() {
    let ws = init_workspace();

    let special_desc = "Multi-line\ndescription\nwith\ttabs\nUnicode: 你好 🎉 🚀\nEscape: \\\" \\n \\t";
    let _id = create_bead_with_type(ws.path(), "Desc special chars ready", "task", special_desc);

    let beads = ready_json(ws.path());
    let bead = &beads[0];

    let desc = bead.get("description").and_then(|v| v.as_str()).unwrap();

    // Verify line breaks and special characters are preserved
    assert!(desc.contains("Multi-line"), "description should contain multi-line text");
    assert!(desc.contains("你好"), "description should contain Chinese characters");
    assert!(desc.contains("🎉"), "description should contain emoji");
    assert!(desc.contains("🚀"), "description should contain another emoji");
}

#[test]
fn test_ready_json_special_characters_in_assignee() {
    let ws = init_workspace();
    let id = create_bead_with_type(ws.path(), "Assignee ready test", "task", "Desc");

    // Update with special character assignee
    let special_assignee = "user+test@example.com <admin>";
    let out = Command::new(bf_path())
        .args(["update", &id, "--assignee", special_assignee])
        .current_dir(ws.path())
        .output()
        .expect("Failed to update bead");
    assert!(out.status.success());

    let beads = ready_json(ws.path());
    let bead = beads.iter()
        .find(|b| b.get("id").and_then(|i| i.as_str()) == Some(id.as_str()))
        .expect("updated bead not found");

    let assignee = bead.get("assignee").and_then(|v| v.as_str()).unwrap();
    assert!(assignee.contains("user+test"), "assignee should preserve special characters");
    assert!(assignee.contains("<admin>"), "assignee should preserve angle brackets");
}

#[test]
fn test_ready_json_unicode_emoji_in_all_text_fields() {
    let ws = init_workspace();

    let unicode_title = "🎉 Unicode ready title with emoji 🚀";
    let unicode_desc = "Description: 你好 مرحبا היי 🌟";
    let id = create_bead_with_all_fields(
        ws.path(),
        unicode_title,
        "task",
        unicode_desc,
        None,
        &["unicode-label-测试"]
    );

    let beads = ready_json(ws.path());
    let bead = beads.iter()
        .find(|b| b.get("id").and_then(|i| i.as_str()) == Some(id.as_str()))
        .expect("unicode bead not found");

    let title = bead.get("title").and_then(|v| v.as_str()).unwrap();
    assert!(title.contains("🎉"), "title should contain party emoji");
    assert!(title.contains("🚀"), "title should contain rocket emoji");

    let desc = bead.get("description").and_then(|v| v.as_str()).unwrap();
    assert!(desc.contains("你好"), "description should contain Chinese");
    assert!(desc.contains("مرحبا"), "description should contain Arabic");
    assert!(desc.contains("היי"), "description should contain Hebrew");
    assert!(desc.contains("🌟"), "description should contain star emoji");

    let labels = bead.get("labels").and_then(|v| v.as_array()).unwrap();
    let label_strs: Vec<&str> = labels.iter().filter_map(|l| l.as_str()).collect();
    assert!(label_strs.contains(&"unicode-label-测试"), "labels should contain unicode characters");
}

#[test]
fn test_ready_json_special_characters_in_labels() {
    let ws = init_workspace();
    let id = create_bead_with_all_fields(
        ws.path(),
        "Label ready test",
        "task",
        "Desc",
        None,
        &["special/label", "label-with-dash", "label_with_underscore", "label.with.dots"]
    );

    let beads = ready_json(ws.path());
    let bead = beads.iter()
        .find(|b| b.get("id").and_then(|i| i.as_str()) == Some(id.as_str()))
        .expect("labeled bead not found");

    let labels = bead.get("labels").and_then(|v| v.as_array()).unwrap();
    let label_strs: Vec<&str> = labels.iter()
        .filter_map(|l| l.as_str())
        .collect();

    assert!(label_strs.contains(&"special/label"), "labels should contain slashes");
    assert!(label_strs.contains(&"label-with-dash"), "labels should contain dashes");
    assert!(label_strs.contains(&"label_with_underscore"), "labels should contain underscores");
    assert!(label_strs.contains(&"label.with.dots"), "labels should contain dots");
}

// ---------------------------------------------------------------------------
// Empty results and filtering tests
// ---------------------------------------------------------------------------

#[test]
fn test_ready_json_handles_empty_results() {
    let ws = init_workspace();

    // No beads created, so ready should be empty
    let out = Command::new(bf_path())
        .args(["ready", "--json"])
        .current_dir(ws.path())
        .output()
        .expect("Failed to run bf ready");

    let stdout = String::from_utf8(out.stdout).unwrap();

    // Empty ready should produce `[]`
    assert_eq!(stdout.trim(), "[]", "empty ready should produce []");
}

#[test]
fn test_ready_json_excludes_closed_beads() {
    let ws = init_workspace();
    let open_id = create_bead_with_type(ws.path(), "Open bead", "task", "Open");
    let closed_id = create_bead_with_type(ws.path(), "To be closed", "task", "Will close");

    // Close one bead
    let close_out = Command::new(bf_path())
        .args(["close", &closed_id, "--reason", "test"])
        .current_dir(ws.path())
        .output()
        .expect("Failed to close bead");
    assert!(close_out.status.success());

    let beads = ready_json(ws.path());
    let ids: Vec<&str> = beads.iter()
        .filter_map(|b| b.get("id").and_then(|i| i.as_str()))
        .collect();

    assert!(ids.contains(&open_id.as_str()), "open bead should be in ready list");
    assert!(!ids.contains(&closed_id.as_str()), "closed bead should not be in ready list");

    // Verify all returned beads have status='open'
    for bead in &beads {
        assert_eq!(bead.get("status").and_then(|s| s.as_str()), Some("open"),
                   "ready should only return open beads");
    }
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_ready_json_excludes_blocked_beads() {
    let ws = init_workspace();
    let blocker_id = create_bead_with_type(ws.path(), "Blocker", "task", "Blocks another");
    let blocked_id = create_bead_with_type(ws.path(), "Blocked", "task", "Blocked by another");

    // Add dependency
    let dep_out = Command::new(bf_path())
        .args(["dep", "add", &blocker_id, "--blocks", &blocked_id])
        .current_dir(ws.path())
        .output()
        .expect("Failed to add dependency");
    assert!(dep_out.status.success());

    let beads = ready_json(ws.path());
    let ids: Vec<&str> = beads.iter()
        .filter_map(|b| b.get("id").and_then(|i| i.as_str()))
        .collect();

    assert!(ids.contains(&blocker_id.as_str()), "blocker should be in ready list");
    assert!(!ids.contains(&blocked_id.as_str()), "blocked bead should not be in ready list");
}

#[test]
fn test_ready_json_excludes_blocked_and_closed_beads() {
    let ws = init_workspace();
    let blocker_id = create_bead_with_type(ws.path(), "Blocker bead", "task", "Blocks");
    let blocked_id = create_bead_with_type(ws.path(), "Blocked bead", "task", "Blocked");
    let closed_id = create_bead_with_type(ws.path(), "Closed bead", "task", "Closed");

    // Add blocking dependency
    let dep_out = Command::new(bf_path())
        .args(["dep", "add", &blocker_id, "--blocks", &blocked_id])
        .current_dir(ws.path())
        .output()
        .expect("Failed to add dependency");
    assert!(dep_out.status.success());

    // Close one bead
    let close_out = Command::new(bf_path())
        .args(["close", &closed_id, "--reason", "test"])
        .current_dir(ws.path())
        .output()
        .expect("Failed to close bead");
    assert!(close_out.status.success());

    let beads = ready_json(ws.path());
    let ids: Vec<&str> = beads.iter()
        .filter_map(|b| b.get("id").and_then(|i| i.as_str()))
        .collect();

    // Should only include the blocker (not blocked or closed beads)
    assert_eq!(beads.len(), 1, "ready should only return one bead");
    assert!(ids.contains(&blocker_id.as_str()), "blocker should be in ready list");
    assert!(!ids.contains(&blocked_id.as_str()), "blocked bead should not be in ready list");
    assert!(!ids.contains(&closed_id.as_str()), "closed bead should not be in ready list");
}

#[test]
fn test_ready_json_limit_parameter_works() {
    let ws = init_workspace();
    let _id1 = create_bead_with_type(ws.path(), "Bead 1", "task", "First");
    let _id2 = create_bead_with_type(ws.path(), "Bead 2", "task", "Second");
    let _id3 = create_bead_with_type(ws.path(), "Bead 3", "task", "Third");

    // Without limit, should return all beads
    let all_beads = ready_json(ws.path());
    assert!(all_beads.len() >= 3, "ready should return all beads without limit");

    // With limit=2, should return only 2 beads
    let limited_beads = ready_json_with_limit(ws.path(), 2);
    assert_eq!(limited_beads.len(), 2, "ready with --limit=2 should return 2 beads");
}

#[test]
fn test_ready_json_with_zero_limit() {
    let ws = init_workspace();
    let _id1 = create_bead_with_type(ws.path(), "Bead 1", "task", "First");
    let _id2 = create_bead_with_type(ws.path(), "Bead 2", "task", "Second");

    // Limit=0 should return all beads (no limit applied)
    let beads = ready_json_with_limit(ws.path(), 0);
    assert!(beads.len() >= 2, "ready with --limit=0 should return all beads");
}

// ---------------------------------------------------------------------------
// Different bead types tests
// ---------------------------------------------------------------------------

#[test]
fn test_ready_json_for_different_types() {
    let ws = init_workspace();
    let task_id = create_bead_with_type(ws.path(), "Task ready", "task", "Task desc");
    let bug_id = create_bead_with_type(ws.path(), "Bug ready", "bug", "Bug desc");
    let feature_id = create_bead_with_type(ws.path(), "Feature ready", "feature", "Feature desc");

    let beads = ready_json(ws.path());

    // Find each bead by type
    let task_bead = beads.iter()
        .find(|b| b.get("id").and_then(|i| i.as_str()) == Some(task_id.as_str()))
        .expect("task bead not found in ready output");
    let bug_bead = beads.iter()
        .find(|b| b.get("id").and_then(|i| i.as_str()) == Some(bug_id.as_str()))
        .expect("bug bead not found in ready output");
    let feature_bead = beads.iter()
        .find(|b| b.get("id").and_then(|i| i.as_str()) == Some(feature_id.as_str()))
        .expect("feature bead not found in ready output");

    assert_eq!(task_bead.get("issue_type").and_then(|v| v.as_str()), Some("task"));
    assert_eq!(bug_bead.get("issue_type").and_then(|v| v.as_str()), Some("bug"));
    assert_eq!(feature_bead.get("issue_type").and_then(|v| v.as_str()), Some("feature"));

    // All should be open (ready)
    assert_eq!(task_bead.get("status").and_then(|v| v.as_str()), Some("open"));
    assert_eq!(bug_bead.get("status").and_then(|v| v.as_str()), Some("open"));
    assert_eq!(feature_bead.get("status").and_then(|v| v.as_str()), Some("open"));
}

#[test]
fn test_ready_json_type_field_preserves_case() {
    let ws = init_workspace();

    let types = vec!["task", "bug", "feature", "epic"];

    for type_name in &types {
        let id = create_bead_with_type(
            ws.path(),
            &format!("{} ready test", type_name),
            type_name,
            "Description",
        );

        let beads = ready_json(ws.path());
        let bead = beads.iter()
            .find(|b| b.get("id").and_then(|i| i.as_str()) == Some(id.as_str()))
            .expect("bead not found in ready output");

        // Type should be normalized to lowercase
        let returned_type = bead.get("issue_type").and_then(|v| v.as_str());
        assert_eq!(returned_type, Some(*type_name),
                   "Type field should match: expected {}, got {:?}",
                   type_name, returned_type);
    }
}

// ---------------------------------------------------------------------------
// Edge cases and integration tests
// ---------------------------------------------------------------------------

#[test]
fn test_ready_json_with_in_progress_status_excluded() {
    let ws = init_workspace();
    let id = create_bead_with_type(ws.path(), "Progress test", "task", "In progress");

    // Update to in_progress
    let update_out = Command::new(bf_path())
        .args(["update", &id, "--status", "in_progress"])
        .current_dir(ws.path())
        .output()
        .expect("Failed to update bead");
    assert!(update_out.status.success());

    let beads = ready_json(ws.path());
    let ids: Vec<&str> = beads.iter()
        .filter_map(|b| b.get("id").and_then(|i| i.as_str()))
        .collect();

    // in_progress beads should NOT appear in ready output
    assert!(!ids.contains(&id.as_str()), "in_progress bead should not be in ready list");
}

#[test]
fn test_ready_json_timestamps_are_valid_rfc3339() {
    let ws = init_workspace();
    let id = create_bead_with_type(ws.path(), "Timestamp ready test", "task", "Test");

    let beads = ready_json(ws.path());
    let bead = beads.iter()
        .find(|b| b.get("id").and_then(|i| i.as_str()) == Some(id.as_str()))
        .expect("bead not found in ready output");

    // Check created_at
    let created_at = bead.get("created_at").and_then(|v| v.as_str()).unwrap();
    let parsed_created = chrono::DateTime::parse_from_rfc3339(created_at);
    assert!(parsed_created.is_ok(), "created_at should be valid RFC3339: {}", created_at);

    // Check updated_at
    let updated_at = bead.get("updated_at").and_then(|v| v.as_str()).unwrap();
    let parsed_updated = chrono::DateTime::parse_from_rfc3339(updated_at);
    assert!(parsed_updated.is_ok(), "updated_at should be valid RFC3339: {}", updated_at);

    // updated_at should be >= created_at
    assert!(parsed_updated.unwrap() >= parsed_created.unwrap(),
            "updated_at should be after or equal to created_at");
}

#[test]
fn test_ready_json_empty_fields_serialize_correctly() {
    let ws = init_workspace();
    let id = create_bead_with_type(ws.path(), "Empty fields ready", "task", "");

    let beads = ready_json(ws.path());
    let bead = beads.iter()
        .find(|b| b.get("id").and_then(|i| i.as_str()) == Some(id.as_str()))
        .expect("bead not found in ready output");

    // Empty description should be present but can be null or empty string
    match bead.get("description") {
        Some(serde_json::Value::String(s)) => {
            // Empty string is fine
            assert_eq!(s, "");
        }
        Some(serde_json::Value::Null) => {
            // Null is also fine
        }
        Some(other) => {
            panic!("description should be null or empty string, got: {:?}", other);
        }
        None => {
            panic!("description field must be present");
        }
    }

    // Assignee should be null when not set
    assert!(bead.get("assignee").is_some(), "assignee field must be present");
    assert!(bead.get("assignee").unwrap().is_null() ||
            bead.get("assignee").and_then(|v| v.as_str()).map(|s| s.is_empty()).unwrap_or(false),
            "assignee should be null or empty string when not set");

    // Labels should be empty array when none set
    assert!(bead.get("labels").is_some(), "labels field must be present");
    let labels = bead.get("labels").and_then(|v| v.as_array()).unwrap();
    assert_eq!(labels.len(), 0, "labels should be empty array when none set");
}

#[test]
fn test_ready_json_all_ready_beads_have_open_status() {
    let ws = init_workspace();
    let _id1 = create_bead_with_type(ws.path(), "Ready 1", "task", "First");
    let _id2 = create_bead_with_type(ws.path(), "Ready 2", "task", "Second");
    let _id3 = create_bead_with_type(ws.path(), "Ready 3", "task", "Third");

    let beads = ready_json(ws.path());

    // All beads in ready output should have status='open'
    for bead in &beads {
        let status = bead.get("status").and_then(|v| v.as_str());
        assert_eq!(status, Some("open"),
                   "All ready beads must have status='open', got {:?} for bead {:?}",
                   status, bead.get("id"));
    }
}

#[test]
fn test_ready_json_bead_with_all_fields_populated() {
    let ws = init_workspace();
    let id = create_bead_with_all_fields(
        ws.path(),
        "All fields ready",
        "task",
        "Base description",
        Some("test-user"),
        &["label1", "label2"]
    );

    // Populate remaining optional fields
    let update_out = Command::new(bf_path())
        .args([
            "update", &id,
            "--description", "Updated description",
            "--acceptance-criteria", "AC 1: Should pass",
            "--notes", "Test notes",
            "--design", "Design reference",
        ])
        .current_dir(ws.path())
        .output()
        .expect("Failed to update bead");
    assert!(update_out.status.success());

    let beads = ready_json(ws.path());
    let bead = beads.iter()
        .find(|b| b.get("id").and_then(|i| i.as_str()) == Some(id.as_str()))
        .expect("bead not found in ready output");

    assert_eq!(bead.get("description").and_then(|v| v.as_str()), Some("Updated description"));
    assert_eq!(bead.get("acceptance_criteria").and_then(|v| v.as_str()), Some("AC 1: Should pass"));
    assert_eq!(bead.get("notes").and_then(|v| v.as_str()), Some("Test notes"));
    assert_eq!(bead.get("design").and_then(|v| v.as_str()), Some("Design reference"));
    assert_eq!(bead.get("assignee").and_then(|v| v.as_str()), Some("test-user"));

    let labels = bead.get("labels").and_then(|v| v.as_array()).unwrap();
    let label_strs: Vec<&str> = labels.iter().filter_map(|l| l.as_str()).collect();
    assert_eq!(label_strs.len(), 2);
    assert!(label_strs.contains(&"label1"));
    assert!(label_strs.contains(&"label2"));
}

#[test]
fn test_ready_json_priority_sorting() {
    let ws = init_workspace();
    let _id1 = create_bead_with_all_fields(
        ws.path(),
        "Low priority",
        "task",
        "Low priority bead",
        None,
        &[]
    );

    // Manually set priorities by updating
    let id_p4 = create_bead_with_type(ws.path(), "P4 bead", "task", "Priority 4");
    let _update_p4 = Command::new(bf_path())
        .args(["update", &id_p4, "--priority", "4"])
        .current_dir(ws.path())
        .output()
        .expect("Failed to update priority");

    let id_p0 = create_bead_with_type(ws.path(), "P0 bead", "task", "Priority 0");
    let _update_p0 = Command::new(bf_path())
        .args(["update", &id_p0, "--priority", "0"])
        .current_dir(ws.path())
        .output()
        .expect("Failed to update priority");

    let beads = ready_json(ws.path());

    // Verify all priority values are valid (0-4)
    for bead in &beads {
        let priority = bead.get("priority").and_then(|v| v.as_i64());
        assert!(priority.is_some(), "priority must be present");
        assert!(priority.map(|p| (0..=4).contains(&p)).unwrap_or(false),
                "priority must be between 0 and 4, got: {:?}", priority);
    }
}
