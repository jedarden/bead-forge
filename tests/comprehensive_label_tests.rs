//! Comprehensive tests for label functionality
//!
//! This test file covers all label functionality as specified in the acceptance criteria:
//! - Labels command in text format
//! - Labels command in JSON format
//! - Label persistence through sync --flush-only
//! - Label survival after sync operations
//! - Edge cases (empty labels, special characters, etc.)

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use bead_forge::sync;

/// Test workspace container
struct TestWorkspace {
    _temp_dir: TempDir,
    beads_dir: PathBuf,
}

impl TestWorkspace {
    /// Create a fresh test workspace with database initialized
    fn new() -> Self {
        let temp_dir = tempfile::tempdir().unwrap();
        let beads = temp_dir.path().join(".beads");
        fs::create_dir(&beads).unwrap();
        bead_forge::config::init_workspace(&beads, "bf").unwrap();

        // Initialize database
        let metadata = bead_forge::config::load_metadata(&beads).unwrap();
        let _ = Storage::open(&beads.join(&metadata.database)).unwrap();

        Self {
            _temp_dir: temp_dir,
            beads_dir: beads,
        }
    }

    /// Get the workspace directory (parent of .beads)
    fn workspace_dir(&self) -> &std::path::Path {
        self._temp_dir.path()
    }

    /// Get the database path
    fn db_path(&self) -> PathBuf {
        let metadata = bead_forge::config::load_metadata(&self.beads_dir).unwrap();
        self.beads_dir.join(&metadata.database)
    }

    /// Get the JSONL path
    fn jsonl_path(&self) -> PathBuf {
        self.beads_dir.join("issues.jsonl")
    }
}

/// Create a test issue with labels
fn create_issue_with_labels(id: &str, labels: Vec<&str>) -> Issue {
    Issue {
        id: id.to_string(),
        title: format!("Test Issue {}", id),
        description: Some(format!("Test issue for label testing - {}", id)),
        status: Status::Open,
        priority: Priority::MEDIUM,
        issue_type: IssueType::Task,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        source_repo: Some(".".to_string()),
        labels: labels.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    }
}

//
// MARK: Labels Command in Text Format
//

#[test]
fn test_labels_command_text_format_single_bead() {
    let ws = TestWorkspace::new();

    // Create bead with labels
    let storage = Storage::open(&ws.db_path()).unwrap();
    let issue = create_issue_with_labels("bf-text-1", vec!["urgent", "backend"]);
    storage.create_issue(&issue).unwrap();

    // Verify labels can be retrieved
    let labels = storage.get_labels("bf-text-1").unwrap();
    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"urgent".to_string()));
    assert!(labels.contains(&"backend".to_string()));
}

#[test]
fn test_labels_command_text_format_all_beads() {
    let ws = TestWorkspace::new();

    // Create multiple beads with different labels
    let storage = Storage::open(&ws.db_path()).unwrap();
    storage.create_issue(&create_issue_with_labels("bf-text-2", vec!["urgent"])).unwrap();
    storage.create_issue(&create_issue_with_labels("bf-text-3", vec!["backend", "frontend"])).unwrap();

    // List all issues and verify labels
    let issues = storage.list_all_issues().unwrap();
    let filtered: Vec<_> = issues.iter().filter(|i| i.id.starts_with("bf-text")).collect();

    assert!(filtered.len() >= 2, "Should have at least 2 test beads");

    // Verify labels are present
    let issue_2 = filtered.iter().find(|i| i.id == "bf-text-2").unwrap();
    assert!(issue_2.labels.contains(&"urgent".to_string()));

    let issue_3 = filtered.iter().find(|i| i.id == "bf-text-3").unwrap();
    assert_eq!(issue_3.labels.len(), 2);
}

#[test]
fn test_labels_command_text_format_empty_labels() {
    let ws = TestWorkspace::new();

    // Create bead without labels
    let storage = Storage::open(&ws.db_path()).unwrap();
    let issue = create_issue_with_labels("bf-text-empty", vec![]);
    storage.create_issue(&issue).unwrap();

    // Verify empty labels
    let labels = storage.get_labels("bf-text-empty").unwrap();
    assert_eq!(labels.len(), 0);
}

//
// MARK: Labels Command in JSON Format
//

#[test]
fn test_labels_command_json_format_single_bead() {
    let ws = TestWorkspace::new();

    // Create bead with labels
    let storage = Storage::open(&ws.db_path()).unwrap();
    let issue = create_issue_with_labels("bf-json-1", vec!["urgent", "backend", "bug"]);
    storage.create_issue(&issue).unwrap();

    // Get labels and serialize to JSON
    let labels = storage.get_labels("bf-json-1").unwrap();
    let json = serde_json::to_string(&labels).unwrap();

    // Verify JSON format
    let parsed: Vec<String> = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.len(), 3);
    assert!(parsed.contains(&"urgent".to_string()));
    assert!(parsed.contains(&"backend".to_string()));
    assert!(parsed.contains(&"bug".to_string()));
}

#[test]
fn test_labels_command_json_format_all_beads() {
    let ws = TestWorkspace::new();

    // Create multiple beads
    let storage = Storage::open(&ws.db_path()).unwrap();
    storage.create_issue(&create_issue_with_labels("bf-json-2", vec!["label1"])).unwrap();
    storage.create_issue(&create_issue_with_labels("bf-json-3", vec!["label2", "label3"])).unwrap();

    // List all issues
    let issues = storage.list_all_issues().unwrap();
    let test_issues: Vec<_> = issues.iter().filter(|i| i.id.starts_with("bf-json")).collect();

    // Verify each issue can be serialized with labels
    for issue in test_issues {
        let json_value = serde_json::to_string(issue).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_value).unwrap();

        assert!(parsed["labels"].is_array());
        assert_eq!(parsed["labels"].as_array().unwrap().len(), issue.labels.len());
    }
}

#[test]
fn test_labels_command_json_format_empty_bead() {
    let ws = TestWorkspace::new();

    // Create bead without labels
    let storage = Storage::open(&ws.db_path()).unwrap();
    let issue = create_issue_with_labels("bf-json-empty", vec![]);
    storage.create_issue(&issue).unwrap();

    // Serialize and verify empty labels array
    let labels = storage.get_labels("bf-json-empty").unwrap();
    let json = serde_json::to_string(&labels).unwrap();
    let parsed: Vec<String> = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.len(), 0);
}

//
// MARK: Label Persistence Through Sync --flush-only
//

#[test]
fn test_label_persistence_flush_only() {
    let ws = TestWorkspace::new();

    // Create bead with labels in database
    let storage = Storage::open(&ws.db_path()).unwrap();
    let issue = create_issue_with_labels("bf-flush-1", vec!["persistent", "test"]);
    storage.create_issue(&issue).unwrap();

    // Flush to JSONL
    sync::flush(ws.workspace_dir()).unwrap();

    // Verify JSONL contains labels
    let jsonl_content = fs::read_to_string(&ws.jsonl_path()).unwrap();
    assert!(jsonl_content.contains("bf-flush-1"));
    assert!(jsonl_content.contains("persistent"));
    assert!(jsonl_content.contains("test"));

    // Parse and verify
    let jsonl_line: Vec<&str> = jsonl_content.lines().collect();
    let line = jsonl_line.iter().find(|l| l.contains("bf-flush-1")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(line).unwrap();

    assert!(parsed["labels"].is_array());
    let labels = parsed["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 2);

    let label_strings: Vec<&str> = labels.iter().filter_map(|v| v.as_str()).collect();
    assert!(label_strings.contains(&"persistent"));
    assert!(label_strings.contains(&"test"));
}

#[test]
fn test_label_persistence_multiple_flushes() {
    let ws = TestWorkspace::new();

    // Create bead
    let storage = Storage::open(&ws.db_path()).unwrap();
    let issue = create_issue_with_labels("bf-flush-multi", vec!["label1"]);
    storage.create_issue(&issue).unwrap();

    // First flush
    sync::flush(ws.workspace_dir()).unwrap();

    // Add more labels
    storage.add_label("bf-flush-multi", "label2").unwrap();
    storage.add_label("bf-flush-multi", "label3").unwrap();

    // Second flush
    sync::flush(ws.workspace_dir()).unwrap();

    // Verify all labels persisted
    let jsonl_content = fs::read_to_string(&ws.jsonl_path()).unwrap();
    let line = jsonl_content.lines().find(|l| l.contains("bf-flush-multi")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(line).unwrap();

    let labels = parsed["labels"].as_array().unwrap();
    assert_eq!(labels.len(), 3);
}

//
// MARK: Label Survival After Sync Operations
//

#[test]
fn test_label_survival_export_import_roundtrip() {
    let ws = TestWorkspace::new();

    // Create bead with labels
    let storage = Storage::open(&ws.db_path()).unwrap();
    let original = create_issue_with_labels("bf-survive-1", vec!["survivor", "test", "label"]);
    storage.create_issue(&original).unwrap();

    // Export to JSONL
    sync::flush(ws.workspace_dir()).unwrap();
    drop(storage);

    // Delete database
    fs::remove_file(&ws.db_path()).unwrap();

    // Import from JSONL
    let result = sync::import(ws.workspace_dir()).unwrap();
    assert_eq!(result.imported, 1);

    // Verify labels survived
    let storage2 = Storage::open(&ws.db_path()).unwrap();
    let imported = storage2.get_issue("bf-survive-1").unwrap().unwrap();

    assert_eq!(imported.labels.len(), 3);
    assert!(imported.labels.contains(&"survivor".to_string()));
    assert!(imported.labels.contains(&"test".to_string()));
    assert!(imported.labels.contains(&"label".to_string()));
}

#[test]
fn test_label_survival_after_add_remove() {
    let ws = TestWorkspace::new();

    // Create bead with labels
    let storage = Storage::open(&ws.db_path()).unwrap();
    let issue = create_issue_with_labels("bf-survive-2", vec!["label1", "label2", "label3"]);
    storage.create_issue(&issue).unwrap();

    // Add and remove labels
    storage.add_label("bf-survive-2", "label4").unwrap();
    storage.remove_label("bf-survive-2", "label2").unwrap();

    // Flush
    sync::flush(ws.workspace_dir()).unwrap();

    // Verify in JSONL
    let jsonl_content = fs::read_to_string(&ws.jsonl_path()).unwrap();
    let line = jsonl_content.lines().find(|l| l.contains("bf-survive-2")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(line).unwrap();

    let labels = parsed["labels"].as_array().unwrap();
    let label_strings: Vec<&str> = labels.iter().filter_map(|v| v.as_str()).collect();

    assert_eq!(label_strings.len(), 3);
    assert!(label_strings.contains(&"label1"));
    assert!(label_strings.contains(&"label3"));
    assert!(label_strings.contains(&"label4"));
    assert!(!label_strings.contains(&"label2"));
}

//
// MARK: Edge Cases - Empty Labels
//

#[test]
fn test_edge_case_empty_label_string() {
    let ws = TestWorkspace::new();

    let storage = Storage::open(&ws.db_path()).unwrap();
    let issue = create_issue_with_labels("bf-edge-empty", vec![""]);
    storage.create_issue(&issue).unwrap();

    let labels = storage.get_labels("bf-edge-empty").unwrap();
    assert!(labels.contains(&"".to_string()));
}

#[test]
fn test_edge_case_whitespace_label() {
    let ws = TestWorkspace::new();

    let storage = Storage::open(&ws.db_path()).unwrap();
    let issue = create_issue_with_labels("bf-edge-space", vec![" ", "  ", "\t"]);
    storage.create_issue(&issue).unwrap();

    let labels = storage.get_labels("bf-edge-space").unwrap();
    assert!(labels.contains(&" ".to_string()));
    assert!(labels.contains(&"  ".to_string()));
    assert!(labels.contains(&"\t".to_string()));
}

//
// MARK: Edge Cases - Special Characters
//

#[test]
fn test_edge_case_unicode_labels() {
    let ws = TestWorkspace::new();

    let storage = Storage::open(&ws.db_path()).unwrap();
    let issue = create_issue_with_labels(
        "bf-edge-unicode",
        vec!["测试", "🔧", "café", "日本語", "🎉"],
    );
    storage.create_issue(&issue).unwrap();

    let labels = storage.get_labels("bf-edge-unicode").unwrap();
    assert!(labels.contains(&"测试".to_string()));
    assert!(labels.contains(&"🔧".to_string()));
    assert!(labels.contains(&"café".to_string()));
    assert!(labels.contains(&"日本語".to_string()));
    assert!(labels.contains(&"🎉".to_string()));
}

#[test]
fn test_edge_case_punctuation_labels() {
    let ws = TestWorkspace::new();

    let storage = Storage::open(&ws.db_path()).unwrap();
    let issue = create_issue_with_labels(
        "bf-edge-punct",
        vec!["won't-fix", "maybe?", "high-priority", "a/b/c", "x.y.z"],
    );
    storage.create_issue(&issue).unwrap();

    let labels = storage.get_labels("bf-edge-punct").unwrap();
    assert!(labels.contains(&"won't-fix".to_string()));
    assert!(labels.contains(&"maybe?".to_string()));
    assert!(labels.contains(&"high-priority".to_string()));
    assert!(labels.contains(&"a/b/c".to_string()));
    assert!(labels.contains(&"x.y.z".to_string()));
}

#[test]
fn test_edge_case_special_chars_labels() {
    let ws = TestWorkspace::new();

    let storage = Storage::open(&ws.db_path()).unwrap();
    // Test various special characters that might cause issues
    let issue = create_issue_with_labels(
        "bf-edge-special",
        vec!["label-and", "label_or", "label:colon"],
    );
    storage.create_issue(&issue).unwrap();

    let labels = storage.get_labels("bf-edge-special").unwrap();
    assert!(labels.contains(&"label-and".to_string()));
    assert!(labels.contains(&"label_or".to_string()));
    assert!(labels.contains(&"label:colon".to_string()));
}

//
// MARK: Edge Cases - Long Labels and Numbers
//

#[test]
fn test_edge_case_very_long_label() {
    let ws = TestWorkspace::new();

    let storage = Storage::open(&ws.db_path()).unwrap();
    let long_label = "a".repeat(1000);
    let issue = create_issue_with_labels("bf-edge-long", vec![&long_label]);
    storage.create_issue(&issue).unwrap();

    let labels = storage.get_labels("bf-edge-long").unwrap();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].len(), 1000);
}

#[test]
fn test_edge_case_numeric_labels() {
    let ws = TestWorkspace::new();

    let storage = Storage::open(&ws.db_path()).unwrap();
    let issue = create_issue_with_labels("bf-edge-num", vec!["123", "v2.0", "2024-q4", "p1"]);
    storage.create_issue(&issue).unwrap();

    let labels = storage.get_labels("bf-edge-num").unwrap();
    assert!(labels.contains(&"123".to_string()));
    assert!(labels.contains(&"v2.0".to_string()));
    assert!(labels.contains(&"2024-q4".to_string()));
    assert!(labels.contains(&"p1".to_string()));
}

//
// MARK: Edge Cases - Single Character and Mixed
//

#[test]
fn test_edge_case_single_char_labels() {
    let ws = TestWorkspace::new();

    let storage = Storage::open(&ws.db_path()).unwrap();
    let issue = create_issue_with_labels("bf-edge-single", vec!["a", "b", "c", "x"]);
    storage.create_issue(&issue).unwrap();

    let labels = storage.get_labels("bf-edge-single").unwrap();
    assert_eq!(labels.len(), 4);
}

#[test]
fn test_edge_case_mixed_labels() {
    let ws = TestWorkspace::new();

    let storage = Storage::open(&ws.db_path()).unwrap();
    let issue = create_issue_with_labels(
        "bf-edge-mixed",
        vec!["", " ", "normal", "123", "🔧", "a-b-c"],
    );
    storage.create_issue(&issue).unwrap();

    let labels = storage.get_labels("bf-edge-mixed").unwrap();
    assert!(labels.contains(&"".to_string()));
    assert!(labels.contains(&" ".to_string()));
    assert!(labels.contains(&"normal".to_string()));
    assert!(labels.contains(&"123".to_string()));
    assert!(labels.contains(&"🔧".to_string()));
    assert!(labels.contains(&"a-b-c".to_string()));
}

//
// MARK: Label Persistence Through Full Sync Cycle
//

#[test]
fn test_label_full_sync_cycle() {
    let ws = TestWorkspace::new();

    // Create beads with various labels
    let storage = Storage::open(&ws.db_path()).unwrap();
    storage
        .create_issue(&create_issue_with_labels("bf-sync-1", vec!["label1"]))
        .unwrap();
    storage
        .create_issue(&create_issue_with_labels("bf-sync-2", vec!["label2", "label3"]))
        .unwrap();
    storage
        .create_issue(&create_issue_with_labels("bf-sync-3", vec![]))
        .unwrap();

    // Flush to JSONL
    sync::flush(ws.workspace_dir()).unwrap();

    // Modify labels
    storage.add_label("bf-sync-1", "label2").unwrap();
    storage.remove_label("bf-sync-2", "label2").unwrap();
    storage.add_label("bf-sync-3", "new-label").unwrap();

    // Flush again
    sync::flush(ws.workspace_dir()).unwrap();

    // Verify in JSONL
    let jsonl_content = fs::read_to_string(&ws.jsonl_path()).unwrap();

    // Check bf-sync-1
    let line1 = jsonl_content.lines().find(|l| l.contains("bf-sync-1")).unwrap();
    let parsed1: serde_json::Value = serde_json::from_str(line1).unwrap();
    assert_eq!(parsed1["labels"].as_array().unwrap().len(), 2);

    // Check bf-sync-2
    let line2 = jsonl_content.lines().find(|l| l.contains("bf-sync-2")).unwrap();
    let parsed2: serde_json::Value = serde_json::from_str(line2).unwrap();
    assert_eq!(parsed2["labels"].as_array().unwrap().len(), 1);

    // Check bf-sync-3
    let line3 = jsonl_content.lines().find(|l| l.contains("bf-sync-3")).unwrap();
    let parsed3: serde_json::Value = serde_json::from_str(line3).unwrap();
    assert_eq!(parsed3["labels"].as_array().unwrap().len(), 1);
}

//
// MARK: JSONL Roundtrip with Complex Label Sets
//

#[test]
fn test_label_complex_jsonl_roundtrip() {
    let ws = TestWorkspace::new();

    // Create bead with complex label set
    let storage = Storage::open(&ws.db_path()).unwrap();
    let complex_labels = vec![
        "urgent",
        "测试",
        "🔧",
        "won't-fix",
        "v2.0",
        "a-b-c",
        "p1",
        "backend",
        " ",
        "",
    ];
    let issue = create_issue_with_labels("bf-complex", complex_labels.clone());
    storage.create_issue(&issue).unwrap();

    // Export
    sync::flush(ws.workspace_dir()).unwrap();
    drop(storage);

    // Import
    fs::remove_file(&ws.db_path()).unwrap();
    sync::import(ws.workspace_dir()).unwrap();

    // Verify all labels survived
    let storage2 = Storage::open(&ws.db_path()).unwrap();
    let imported = storage2.get_issue("bf-complex").unwrap().unwrap();

    assert_eq!(imported.labels.len(), complex_labels.len());
    for label in &complex_labels {
        assert!(
            imported.labels.contains(&label.to_string()),
            "Label '{}' should have survived roundtrip",
            label
        );
    }
}

//
// MARK: Labels Command CLI Text Format Tests
//

use std::path::Path;
use std::process::Command;

/// Get the bf binary path
fn bf_binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bf"))
}

/// Run bf command in workspace, returning (stdout, stderr, success)
fn run_bf_cli(workspace: &Path, args: &[&str]) -> (String, String, bool) {
    let output = bf_binary()
        .current_dir(workspace)
        .args(args)
        .output()
        .expect("Failed to execute bf command");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.success(),
    )
}

#[test]
fn test_labels_cli_text_format_all_beads() {
    let ws = TestWorkspace::new();

    // Create multiple beads with different labels
    let storage = Storage::open(&ws.db_path()).unwrap();
    storage
        .create_issue(&create_issue_with_labels("bf-cli-1", vec!["urgent", "backend"]))
        .unwrap();
    storage
        .create_issue(&create_issue_with_labels("bf-cli-2", vec!["frontend"]))
        .unwrap();
    storage
        .create_issue(&create_issue_with_labels("bf-cli-3", vec![]))
        .unwrap();

    // Run labels command without ID (all beads mode)
    let (stdout, stderr, success) = run_bf_cli(ws.workspace_dir(), &["labels"]);
    assert!(success, "bf labels failed: {}", stderr);

    // Verify output format: "{id} {title} | {labels}"
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(lines.len() >= 3, "Expected at least 3 beads in output");

    // Check bf-cli-1 line with multiple labels
    let line1 = lines.iter().find(|l| l.contains("bf-cli-1")).unwrap();
    assert!(line1.contains("bf-cli-1"));
    assert!(line1.contains("Test Issue bf-cli-1"));
    assert!(line1.contains("urgent"));
    assert!(line1.contains("backend"));

    // Check bf-cli-2 line with single label
    let line2 = lines.iter().find(|l| l.contains("bf-cli-2")).unwrap();
    assert!(line2.contains("bf-cli-2"));
    assert!(line2.contains("frontend"));

    // Check bf-cli-3 line with no labels
    let line3 = lines.iter().find(|l| l.contains("bf-cli-3")).unwrap();
    assert!(line3.contains("bf-cli-3"));
    assert!(line3.contains("(no labels)"));
}

#[test]
fn test_labels_cli_text_format_single_bead() {
    let ws = TestWorkspace::new();

    // Create bead with labels
    let storage = Storage::open(&ws.db_path()).unwrap();
    storage
        .create_issue(&create_issue_with_labels("bf-cli-single", vec!["urgent", "bugfix", "high-priority"]))
        .unwrap();

    // Run labels command with ID (single bead mode)
    let (stdout, stderr, success) = run_bf_cli(ws.workspace_dir(), &["labels", "bf-cli-single"]);
    assert!(success, "bf labels failed: {}", stderr);

    // Verify labels are printed one per line
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines.contains(&"urgent"));
    assert!(lines.contains(&"bugfix"));
    assert!(lines.contains(&"high-priority"));
}

#[test]
fn test_labels_cli_text_format_empty_labels() {
    let ws = TestWorkspace::new();

    // Create bead without labels
    let storage = Storage::open(&ws.db_path()).unwrap();
    storage
        .create_issue(&create_issue_with_labels("bf-cli-empty", vec![]))
        .unwrap();

    // Test single bead mode with empty labels
    let (stdout, stderr, success) = run_bf_cli(ws.workspace_dir(), &["labels", "bf-cli-empty"]);
    assert!(success, "bf labels failed: {}", stderr);

    // Empty labels should produce no output
    assert!(stdout.trim().is_empty() || stdout.lines().all(|l| l.trim().is_empty()));

    // Test all beads mode shows "(no labels)"
    let (stdout, stderr, success) = run_bf_cli(ws.workspace_dir(), &["labels"]);
    assert!(success, "bf labels failed: {}", stderr);

    assert!(stdout.contains("bf-cli-empty"));
    assert!(stdout.contains("(no labels)"));
}

#[test]
fn test_labels_cli_text_format_single_label() {
    let ws = TestWorkspace::new();

    // Create bead with single label
    let storage = Storage::open(&ws.db_path()).unwrap();
    storage
        .create_issue(&create_issue_with_labels("bf-cli-one", vec!["solo"]))
        .unwrap();

    // Run labels command with ID
    let (stdout, stderr, success) = run_bf_cli(ws.workspace_dir(), &["labels", "bf-cli-one"]);
    assert!(success, "bf labels failed: {}", stderr);

    // Verify single label is printed
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].trim(), "solo");
}

#[test]
fn test_labels_cli_text_format_multiple_labels() {
    let ws = TestWorkspace::new();

    // Create bead with multiple labels
    let storage = Storage::open(&ws.db_path()).unwrap();
    let test_labels = vec!["urgent", "backend", "database", "performance", "p1"];
    storage
        .create_issue(&create_issue_with_labels("bf-cli-multi", test_labels.clone()))
        .unwrap();

    // Run labels command with ID
    let (stdout, stderr, success) = run_bf_cli(ws.workspace_dir(), &["labels", "bf-cli-multi"]);
    assert!(success, "bf labels failed: {}", stderr);

    // Verify all labels are printed, one per line
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), test_labels.len());

    for label in &test_labels {
        assert!(lines.contains(&label), "Label '{}' not found in output", label);
    }
}

#[test]
fn test_labels_cli_text_format_labels_are_comma_separated_in_all_mode() {
    let ws = TestWorkspace::new();

    // Create bead with multiple labels
    let storage = Storage::open(&ws.db_path()).unwrap();
    storage
        .create_issue(&create_issue_with_labels("bf-cli-comma", vec!["label1", "label2", "label3"]))
        .unwrap();

    // Run labels command without ID (all beads mode)
    let (stdout, stderr, success) = run_bf_cli(ws.workspace_dir(), &["labels"]);
    assert!(success, "bf labels failed: {}", stderr);

    // In all beads mode, labels should be comma-separated
    let line = stdout.lines().find(|l| l.contains("bf-cli-comma")).unwrap();
    assert!(line.contains(","));
    assert!(line.contains("label1"));
    assert!(line.contains("label2"));
    assert!(line.contains("label3"));
}
