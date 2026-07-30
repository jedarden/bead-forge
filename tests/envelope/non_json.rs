//! Integration tests for non-JSON format envelope handling.
//!
//! This module groups tests for text and toon formats to verify that
//! they properly ignore envelope wrapping and output plain text.
//!
//! Test commands:
//!   - cargo test envelope::non_json::text
//!   - cargo test envelope::non_json::toon
//!   - cargo test envelope::non_json

use std::process::Command;
use tempfile::TempDir;

/// Resolve the freshly-built bf binary.
fn bf_path() -> String {
    std::env::var("CARGO_BIN_EXE_bf")
        .unwrap_or_else(|_| "./target/debug/bf".to_string())
}

/// Create an isolated workspace via `bf init`.
fn init_workspace() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let out = Command::new(bf_path())
        .args(["init", "--prefix", "test"])
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

/// Create a task bead via the CLI, returning its printed id.
fn create_bead(workspace: &std::path::Path, title: &str) -> String {
    let out = Command::new(bf_path())
        .args(["create", "--title", title, "--type", "task", "--priority", "2"])
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

/// Run a command with --envelope flag and return stdout as string.
fn run_with_envelope(workspace: &std::path::Path, args: &[&str]) -> String {
    let mut full_args = vec!["--envelope"];
    full_args.extend_from_slice(args);

    let out = Command::new(bf_path())
        .args(&full_args)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf command");

    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();

    if !out.status.success() {
        panic!("Command failed: {:?}\nstdout: {}\nstderr: {}", full_args, stdout, stderr);
    }

    stdout
}

// ============================================================================
// Text Format Tests
// ============================================================================

#[test]
fn text_stats_outputs_plain_text_not_json() {
    let ws = init_workspace();
    create_bead(ws.path(), "stats test");

    let output = run_with_envelope(ws.path(), &["stats", "--format", "text"]);

    // Verify output is plain text, not JSON
    assert!(!output.starts_with('{'), "Text output should not start with JSON object");
    assert!(!output.contains("\"version\":"), "Text output should not contain version field");
    assert!(!output.contains("\"kind\":"), "Text output should not contain kind field");
    assert!(!output.contains("\"data\":"), "Text output should not contain data field");
    assert!(output.contains("Total beads:"), "Text output should contain 'Total beads:'");
}

#[test]
fn text_claim_outputs_plain_text_not_json() {
    let ws = init_workspace();
    create_bead(ws.path(), "claim test");

    let output = run_with_envelope(ws.path(), &["claim", "--assignee", "test-worker", "--format", "text"]);

    // Verify output is plain text, not JSON
    assert!(!output.starts_with('{'), "Claim output should not be JSON envelope");
    assert!(!output.contains("\"version\":"), "Claim output should not contain version field");
    assert!(!output.contains("\"kind\":"), "Claim output should not contain kind field");
    assert!(!output.trim().is_empty(), "Claim output should not be empty");
}

#[test]
fn text_list_outputs_plain_text_not_json() {
    let ws = init_workspace();
    create_bead(ws.path(), "list test");

    let output = run_with_envelope(ws.path(), &["list", "--format", "text"]);

    // Verify output is plain text, not JSON
    assert!(!output.starts_with('{'), "List output should not be JSON envelope");
    assert!(!output.contains("\"version\":"), "List output should not contain version field");
    assert!(!output.contains("\"kind\":"), "List output should not contain kind field");
}

#[test]
fn text_show_outputs_plain_text_not_json() {
    let ws = init_workspace();
    let bead_id = create_bead(ws.path(), "show test");

    let output = run_with_envelope(ws.path(), &["show", &bead_id, "--format", "text"]);

    // Verify output is plain text, not JSON
    assert!(!output.starts_with('{'), "Show output should not be JSON envelope");
    assert!(output.contains("ID:"), "Show output should contain 'ID:' field");
}

#[test]
fn text_ready_outputs_plain_text_not_json() {
    let ws = init_workspace();
    create_bead(ws.path(), "ready test");

    let output = run_with_envelope(ws.path(), &["ready", "--format", "text"]);

    // Verify output is plain text, not JSON
    assert!(!output.starts_with('{'), "Ready output should not be JSON envelope");
}

// ============================================================================
// Toon Format Tests
// ============================================================================

#[test]
fn toon_stats_outputs_plain_text_not_json() {
    let ws = init_workspace();
    create_bead(ws.path(), "stats test");

    let output = run_with_envelope(ws.path(), &["stats", "--format", "toon"]);

    // Verify output is plain text, not JSON
    assert!(!output.starts_with('{'), "Toon output should not start with JSON object");
    assert!(!output.contains("\"version\":"), "Toon output should not contain version field");
    assert!(!output.contains("\"kind\":"), "Toon output should not contain kind field");
    assert!(!output.contains("\"data\":"), "Toon output should not contain data field");
    assert!(output.contains("Total beads:"), "Toon output should contain 'Total beads:'");
}

#[test]
fn toon_claim_outputs_plain_text_not_json() {
    let ws = init_workspace();
    create_bead(ws.path(), "claim test");

    let output = run_with_envelope(ws.path(), &["claim", "--assignee", "test-worker", "--format", "toon"]);

    // Verify output is plain text, not JSON
    assert!(!output.starts_with('{'), "Claim output should not be JSON envelope");
    assert!(!output.contains("\"version\":"), "Claim output should not contain version field");
    assert!(!output.contains("\"kind\":"), "Claim output should not contain kind field");
    assert!(!output.trim().is_empty(), "Claim output should not be empty");
}

#[test]
fn toon_list_outputs_plain_text_not_json() {
    let ws = init_workspace();
    create_bead(ws.path(), "list test");

    let output = run_with_envelope(ws.path(), &["list", "--format", "toon"]);

    // Verify output is plain text, not JSON
    assert!(!output.starts_with('{'), "List output should not be JSON envelope");
    assert!(!output.contains("\"version\":"), "List output should not contain version field");
    assert!(!output.contains("\"kind\":"), "List output should not contain kind field");
}

#[test]
fn toon_show_outputs_plain_text_not_json() {
    let ws = init_workspace();
    let bead_id = create_bead(ws.path(), "show test");

    let output = run_with_envelope(ws.path(), &["show", "--format", "toon", &bead_id]);

    // Verify output is plain text, not JSON
    assert!(!output.starts_with('{'), "Show output should not be JSON envelope");
    assert!(output.contains("ID:"), "Show output should contain 'ID:' field");
}

#[test]
fn toon_ready_outputs_plain_text_not_json() {
    let ws = init_workspace();
    create_bead(ws.path(), "ready test");

    let output = run_with_envelope(ws.path(), &["ready", "--format", "toon"]);

    // Verify output is plain text, not JSON
    assert!(!output.starts_with('{'), "Ready output should not be JSON envelope");
}

// ============================================================================
// Cross-Format Consistency Tests
// ============================================================================

#[test]
fn non_json_formats_ignore_envelope_flag() {
    let ws = init_workspace();
    create_bead(ws.path(), "consistency test");

    // Test that both formats ignore envelope
    let text_output = run_with_envelope(ws.path(), &["stats", "--format", "text"]);
    let toon_output = run_with_envelope(ws.path(), &["stats", "--format", "toon"]);

    // Both should be plain text
    assert!(!text_output.starts_with('{'), "Text should ignore envelope");
    assert!(!toon_output.starts_with('{'), "Toon should ignore envelope");

    // Both should contain human-readable output
    assert!(text_output.contains("Total beads:"), "Text should have total line");
    assert!(toon_output.contains("Total beads:"), "Toon should have total line");
}

#[test]
fn non_json_formats_respect_format_config() {
    let ws = init_workspace();
    create_bead(ws.path(), "format config test");

    // Verify that format selection still works with --envelope flag
    let text_output = run_with_envelope(ws.path(), &["stats", "--format", "text"]);
    let toon_output = run_with_envelope(ws.path(), &["stats", "--format", "toon"]);

    // Both should have format-appropriate content
    assert!(text_output.contains("Total beads:"), "Text format should work");
    assert!(toon_output.contains("Total beads:"), "Toon format should work");
}
