//! P0 Label Add Test Infrastructure
//!
//! Comprehensive test suite for adding labels to P0 priority beads.
//! Tests cover:
//! - CLI parsing for label add commands
//! - Integration tests with actual storage
//! - Edge cases and error handling
//! - Deduplication behavior
//! - Persistence verification

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

// ============================================================================
// Test Fixtures and Helpers
// ============================================================================

/// Test workspace with isolated environment
pub struct P0TestWorkspace {
    pub temp_dir: TempDir,
    pub workspace_dir: PathBuf,
    pub beads_dir: PathBuf,
}

impl P0TestWorkspace {
    /// Create a new isolated test workspace
    pub fn new() -> anyhow::Result<Self> {
        let temp_dir = TempDir::new()?;
        let workspace_dir = temp_dir.path().join("test-workspace");
        fs::create_dir_all(&workspace_dir)?;

        let beads_dir = workspace_dir.join(".beads");
        fs::create_dir_all(&beads_dir)?;

        // Initialize bf config
        let config_path = beads_dir.join("config.yaml");
        fs::write(
            &config_path,
            r#"issue_prefixes: [bf]
default_priority: 2
default_type: task
claim_ttl_minutes: 30
"#,
        )?;

        // Initialize metadata
        let metadata_path = beads_dir.join("metadata.json");
        fs::write(
            &metadata_path,
            r#"{"database": "beads.db", "jsonl_export": "issues.jsonl"}"#,
        )?;

        // Initialize database
        let db_path = beads_dir.join("beads.db");
        let _storage = bead_forge::storage::Storage::open(&db_path)?;

        Ok(Self {
            temp_dir,
            workspace_dir,
            beads_dir,
        })
    }

    /// Get path to the bf binary
    pub fn bf_binary(&self) -> String {
        std::env::var("CARGO_BIN_EXE_bf")
            .unwrap_or_else(|_| "./target/debug/bf".to_string())
    }

    /// Run a bf command and return output
    pub fn run_bf(&self, args: &[&str]) -> BfCommandResult {
        let out = Command::new(self.bf_binary())
            .args(args)
            .current_dir(&self.workspace_dir)
            .output()
            .expect("Failed to run bf command");

        BfCommandResult {
            stdout: String::from_utf8(out.stdout).unwrap(),
            stderr: String::from_utf8(out.stderr).unwrap(),
            success: out.status.success(),
        }
    }

    /// Create a P0 bead with optional labels
    pub fn create_p0_bead(&self, title: &str, labels: &[&str]) -> String {
        let mut args = vec!["create", "--title", title, "--priority", "0"];
        for label in labels {
            args.push("--label");
            args.push(label);
        }

        let result = self.run_bf(&args);
        assert!(result.success, "Failed to create P0 bead: {}", result.stderr);

        self.extract_bead_id(&result.stdout)
    }

    /// Extract bead ID from command output
    pub fn extract_bead_id(&self, output: &str) -> String {
        output
            .lines()
            .find(|line| line.contains("bf-"))
            .and_then(|line| line.split("bf-").nth(1))
            .map(|id| format!("bf-{}", id.trim().split_whitespace().next().unwrap_or(id)))
            .expect("Could not extract bead ID from output")
    }

    /// Get bead data in JSON format
    pub fn get_bead_json(&self, bead_id: &str) -> serde_json::Value {
        let result = self.run_bf(&["show", bead_id, "--format", "json"]);
        assert!(result.success, "Failed to get bead JSON: {}", result.stderr);

        serde_json::from_str(&result.stdout).expect("Failed to parse JSON")
    }

    /// Get labels from a bead
    pub fn get_bead_labels(&self, bead_id: &str) -> Vec<String> {
        let json = self.get_bead_json(bead_id);
        json[0]["labels"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Result of running a bf command
pub struct BfCommandResult {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

// ============================================================================
// CLI Parsing Tests
// ============================================================================

#[test]
fn test_p0_label_add_cli_parsing() {
    use bead_forge::cli::{Cli, Commands, LabelCommands};
    use clap::Parser;

    // Test basic P0 label add parsing
    let args = vec!["bf", "label", "add", "bf-12345", "--label", "P0"];
    let cli = Cli::parse_from(args);

    let command = cli.command.expect("Command should be present");
    match command {
        Commands::Label(LabelCommands::Add { id, label }) => {
            assert_eq!(label.len(), 1);
            assert_eq!(label[0], "P0");
            assert_eq!(id, "bf-12345");
        }
        _ => panic!("Expected Label::Add command"),
    }
}

#[test]
fn test_p0_label_add_multiple_labels_cli_parsing() {
    use bead_forge::cli::{Cli, Commands, LabelCommands};
    use clap::Parser;

    let args = vec![
        "bf", "label", "add", "bf-67890",
        "--label", "P0",
        "--label", "urgent",
        "--label", "critical",
    ];
    let cli = Cli::parse_from(args);

    let command = cli.command.expect("Command should be present");
    match command {
        Commands::Label(LabelCommands::Add { id, label }) => {
            assert_eq!(label.len(), 3);
            assert!(label.contains(&"P0".to_string()));
            assert!(label.contains(&"urgent".to_string()));
            assert!(label.contains(&"critical".to_string()));
            assert_eq!(id, "bf-67890");
        }
        _ => panic!("Expected Label::Add command"),
    }
}

#[test]
fn test_p0_label_add_short_flag_cli_parsing() {
    use bead_forge::cli::{Cli, Commands, LabelCommands};
    use clap::Parser;

    let args = vec!["bf", "label", "add", "bf-abcd", "-l", "P0"];
    let cli = Cli::parse_from(args);

    let command = cli.command.expect("Command should be present");
    match command {
        Commands::Label(LabelCommands::Add { id, label }) => {
            assert_eq!(label.len(), 1);
            assert_eq!(label[0], "P0");
            assert_eq!(id, "bf-abcd");
        }
        _ => panic!("Expected Label::Add command"),
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_p0_label_add_single_label() {
    let ws = P0TestWorkspace::new().unwrap();

    // Create P0 bead without labels
    let bead_id = ws.create_p0_bead("P0 Test Bead", &[]);

    // Add single label
    let result = ws.run_bf(&["label", "add", &bead_id, "-l", "test-label"]);
    assert!(result.success, "Label add failed: {}", result.stderr);

    // Verify label was added
    let labels = ws.get_bead_labels(&bead_id);
    assert_eq!(labels.len(), 1);
    assert!(labels.contains(&"test-label".to_string()));

    // Verify priority is still P0
    let json = ws.get_bead_json(&bead_id);
    assert_eq!(json[0]["priority"], 0);
}

#[test]
fn test_p0_label_add_multiple_labels() {
    let ws = P0TestWorkspace::new().unwrap();

    // Create P0 bead
    let bead_id = ws.create_p0_bead("P0 Multiple Labels", &[]);

    // Add multiple labels at once
    let result = ws.run_bf(&[
        "label", "add", &bead_id,
        "-l", "critical",
        "-l", "backend",
        "-l", "hotfix",
    ]);
    assert!(result.success, "Label add failed: {}", result.stderr);

    // Verify all labels were added
    let labels = ws.get_bead_labels(&bead_id);
    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"critical".to_string()));
    assert!(labels.contains(&"backend".to_string()));
    assert!(labels.contains(&"hotfix".to_string()));
}

#[test]
fn test_p0_label_add_deduplication() {
    let ws = P0TestWorkspace::new().unwrap();

    // Create P0 bead with initial label
    let bead_id = ws.create_p0_bead("P0 Dedup Test", &["existing"]);

    // Add duplicate label
    let result = ws.run_bf(&["label", "add", &bead_id, "-l", "existing"]);
    assert!(result.success, "Label add with duplicate failed: {}", result.stderr);

    // Verify no duplicate was added
    let labels = ws.get_bead_labels(&bead_id);
    assert_eq!(labels.len(), 1);
    assert!(labels.contains(&"existing".to_string()));
}

#[test]
fn test_p0_label_add_mixed_duplicates() {
    let ws = P0TestWorkspace::new().unwrap();

    // Create P0 bead
    let bead_id = ws.create_p0_bead("P0 Mixed Duplicates", &["keep-1"]);

    // Add mix of new and duplicate labels
    let result = ws.run_bf(&[
        "label", "add", &bead_id,
        "-l", "keep-1",      // duplicate
        "-l", "new-1",        // new
        "-l", "keep-1",      // duplicate again
        "-l", "new-2",        // new
    ]);
    assert!(result.success, "Label add with mixed duplicates failed: {}", result.stderr);

    // Verify only unique labels were added
    let labels = ws.get_bead_labels(&bead_id);
    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"keep-1".to_string()));
    assert!(labels.contains(&"new-1".to_string()));
    assert!(labels.contains(&"new-2".to_string()));
}

#[test]
fn test_p0_label_add_to_bead_with_existing_labels() {
    let ws = P0TestWorkspace::new().unwrap();

    // Create P0 bead with existing labels
    let bead_id = ws.create_p0_bead("P0 Existing Labels", &["label-1", "label-2"]);

    // Add more labels
    let result = ws.run_bf(&["label", "add", &bead_id, "-l", "label-3", "-l", "label-4"]);
    assert!(result.success, "Label add failed: {}", result.stderr);

    // Verify all labels are present
    let labels = ws.get_bead_labels(&bead_id);
    assert_eq!(labels.len(), 4);
    assert!(labels.contains(&"label-1".to_string()));
    assert!(labels.contains(&"label-2".to_string()));
    assert!(labels.contains(&"label-3".to_string()));
    assert!(labels.contains(&"label-4".to_string()));
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_p0_label_add_empty_label_list() {
    let ws = P0TestWorkspace::new().unwrap();

    // Create P0 bead
    let bead_id = ws.create_p0_bead("P0 Empty Label Test", &[]);

    // Attempt to add label without specifying any labels
    // This should fail or be handled gracefully
    let result = ws.run_bf(&["label", "add", &bead_id]);

    // Command should fail when no labels are provided
    assert!(!result.success, "Label add should fail without labels");
}

#[test]
fn test_p0_label_add_special_characters() {
    let ws = P0TestWorkspace::new().unwrap();

    // Create P0 bead
    let bead_id = ws.create_p0_bead("P0 Special Chars", &[]);

    // Add labels with special characters
    let result = ws.run_bf(&[
        "label", "add", &bead_id,
        "-l", "phase-1",
        "-l", "bug/critical",
        "-l", "team::backend",
    ]);
    assert!(result.success, "Label add with special characters failed: {}", result.stderr);

    // Verify special character labels are preserved
    let labels = ws.get_bead_labels(&bead_id);
    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"phase-1".to_string()));
    assert!(labels.contains(&"bug/critical".to_string()));
    assert!(labels.contains(&"team::backend".to_string()));
}

#[test]
fn test_p0_label_add_nonexistent_bead() {
    let ws = P0TestWorkspace::new().unwrap();

    // Try to add label to non-existent bead
    let result = ws.run_bf(&["label", "add", "bf-nonexistent", "-l", "test"]);

    // Should fail gracefully
    assert!(!result.success, "Label add should fail for non-existent bead");
    assert!(result.stderr.contains("not found") || result.stderr.contains("unknown"));
}

#[test]
fn test_p0_label_add_very_long_label() {
    let ws = P0TestWorkspace::new().unwrap();

    // Create P0 bead
    let bead_id = ws.create_p0_bead("P0 Long Label", &[]);

    // Add a very long label
    let long_label = "a".repeat(500);
    let result = ws.run_bf(&["label", "add", &bead_id, "-l", &long_label]);

    // Should either succeed or fail gracefully
    if result.success {
        let labels = ws.get_bead_labels(&bead_id);
        assert!(labels.contains(&long_label));
    }
}

#[test]
fn test_p0_label_add_unicode_labels() {
    let ws = P0TestWorkspace::new().unwrap();

    // Create P0 bead
    let bead_id = ws.create_p0_bead("P0 Unicode", &[]);

    // Add labels with unicode characters
    let result = ws.run_bf(&[
        "label", "add", &bead_id,
        "-l", "🔥-critical",
        "-l", "tëst-label",
        "-l", "日本語",
    ]);
    assert!(result.success, "Label add with unicode failed: {}", result.stderr);

    // Verify unicode labels are preserved
    let labels = ws.get_bead_labels(&bead_id);
    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"🔥-critical".to_string()));
    assert!(labels.contains(&"tëst-label".to_string()));
    assert!(labels.contains(&"日本語".to_string()));
}

// ============================================================================
// Persistence Tests
// ============================================================================

#[test]
fn test_p0_label_add_persistence_after_flush() {
    let ws = P0TestWorkspace::new().unwrap();

    // Create P0 bead
    let bead_id = ws.create_p0_bead("P0 Persistence Test", &[]);

    // Add labels
    let _ = ws.run_bf(&["label", "add", &bead_id, "-l", "persistent-1", "-l", "persistent-2"]);

    // Flush to JSONL
    let result = ws.run_bf(&["sync", "--flush-only"]);
    assert!(result.success, "Flush failed: {}", result.stderr);

    // Verify labels persist after flush
    let labels = ws.get_bead_labels(&bead_id);
    assert_eq!(labels.len(), 2);
    assert!(labels.contains(&"persistent-1".to_string()));
    assert!(labels.contains(&"persistent-2".to_string()));
}

#[test]
fn test_p0_label_add_priority_preservation() {
    let ws = P0TestWorkspace::new().unwrap();

    // Create P0 bead
    let bead_id = ws.create_p0_bead("P0 Priority Test", &[]);

    // Perform multiple label operations
    let _ = ws.run_bf(&["label", "add", &bead_id, "-l", "label-1"]);
    let _ = ws.run_bf(&["label", "add", &bead_id, "-l", "label-2"]);
    let _ = ws.run_bf(&["label", "add", &bead_id, "-l", "label-3"]);

    // Verify priority is still P0
    let json = ws.get_bead_json(&bead_id);
    assert_eq!(json[0]["priority"], 0);
}

// ============================================================================
// Test Counter
// ============================================================================

#[test]
fn test_p0_label_add_test_count() {
    // This test verifies the test infrastructure is working
    // by checking that we can create and manipulate P0 beads
    let ws = P0TestWorkspace::new().unwrap();

    let bead_id = ws.create_p0_bead("Count Verification", &[]);
    let _ = ws.run_bf(&["label", "add", &bead_id, "-l", "counted"]);

    let labels = ws.get_bead_labels(&bead_id);
    assert_eq!(labels.len(), 1);
    assert!(labels.contains(&"counted".to_string()));
}
