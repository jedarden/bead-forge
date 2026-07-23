//! Integration tests for toon format envelope handling.
//!
//! Tests verify that toon format ignores envelope wrapping and outputs
//! compact plain text regardless of --envelope flag.
//!
//! Test commands:
//!   - cargo test envelope::toon_format::stats
//!   - cargo test envelope::toon_format::claim
//!   - cargo test envelope::toon_format

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
fn run_envelope_command_toon(workspace: &std::path::Path, args: &[&str]) -> String {
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

/// Run a command without --envelope flag and return stdout as string.
fn run_command_toon(workspace: &std::path::Path, args: &[&str]) -> String {
    let out = Command::new(bf_path())
        .args(args)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf command");

    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();

    if !out.status.success() {
        panic!("Command failed: {:?}\nstdout: {}\nstderr: {}", args, stdout, stderr);
    }

    stdout
}

// ============================================================================
// Stats Command Toon Format Envelope Tests
// ============================================================================

/// Test: stats --format toon --envelope outputs plain text (not JSON envelope)
#[test]
fn stats_envelope_outputs_plain_text() {
    let ws = init_workspace();
    create_bead(ws.path(), "stats test");

    let output = run_envelope_command_toon(ws.path(), &["stats", "--format", "toon"]);

    // Verify output is plain text, not JSON
    assert!(!output.starts_with('{'), "Toon output should not start with JSON object");
    assert!(!output.contains("\"version\":"), "Toon output should not contain version field");
    assert!(!output.contains("\"kind\":"), "Toon output should not contain kind field");
    assert!(!output.contains("\"data\":"), "Toon output should not contain data field");

    // Verify output contains expected toon format content
    assert!(output.contains("Total beads:"), "Toon output should contain 'Total beads:'");
}

/// Test: stats --format toon --envelope output matches output without envelope
#[test]
fn stats_envelope_output_matches_no_envelope() {
    let ws = init_workspace();
    create_bead(ws.path(), "consistency test");
    create_bead(ws.path(), "second bead");

    let with_envelope = run_envelope_command_toon(ws.path(), &["stats", "--format", "toon"]);
    let without_envelope = run_command_toon(ws.path(), &["stats", "--format", "toon"]);

    // Output should be identical regardless of --envelope flag
    assert_eq!(with_envelope, without_envelope, "Toon output should be same with and without --envelope");
}

/// Test: stats --format toon --envelope has correct toon format structure
#[test]
fn stats_envelope_toon_structure() {
    let ws = init_workspace();
    create_bead(ws.path(), "structure test");

    let output = run_envelope_command_toon(ws.path(), &["stats", "--format", "toon"]);

    // Verify toon format structure (same as text format for stats)
    assert!(output.contains("Total beads:"), "Should have total line");
    assert!(output.contains("Open:"), "Should have open line");
    assert!(output.contains("In Progress:"), "Should have in_progress line");
    assert!(output.contains("Closed:"), "Should have closed line");
}

/// Test: stats --format toon --envelope with empty workspace outputs text
#[test]
fn stats_envelope_empty_workspace() {
    let ws = init_workspace();

    let output = run_envelope_command_toon(ws.path(), &["stats", "--format", "toon"]);

    // Verify output is text, not JSON envelope
    assert!(!output.starts_with('{'), "Empty workspace should output text, not JSON");

    // Verify total is 0 in toon format (same as text)
    assert!(output.contains("Total beads: 0"), "Empty workspace should show 0 total");
    assert!(output.contains("Open: 0"), "Empty workspace should show 0 open");
}

/// Test: stats --format toon --envelope with multiple beads shows correct counts
#[test]
fn stats_envelope_multiple_beads() {
    let ws = init_workspace();

    create_bead(ws.path(), "first");
    create_bead(ws.path(), "second");
    create_bead(ws.path(), "third");

    let output = run_envelope_command_toon(ws.path(), &["stats", "--format", "toon"]);

    // Verify toon format shows correct counts (same as text)
    assert!(output.contains("Total beads: 3"), "Should show total of 3 beads");
    assert!(output.contains("Open: 3"), "Should show 3 open beads");
}

// ============================================================================
// Claim Command Toon Format Envelope Tests
// ============================================================================

/// Test: claim --format toon --envelope outputs plain text (not JSON envelope)
#[test]
fn claim_envelope_outputs_plain_text() {
    let ws = init_workspace();
    create_bead(ws.path(), "claim test");

    let output = run_envelope_command_toon(ws.path(), &["claim", "--assignee", "test-worker", "--format", "toon"]);

    // Verify output is plain text, not JSON
    assert!(!output.starts_with('{'), "Claim output should not be JSON envelope");
    assert!(!output.contains("\"version\":"), "Claim output should not contain version field");
    assert!(!output.contains("\"kind\":"), "Claim output should not contain kind field");

    // Verify output contains bead ID
    assert!(!output.trim().is_empty(), "Claim output should not be empty");
}

/// Test: claim --format toon --envelope output matches output without envelope
#[test]
fn claim_envelope_output_matches_no_envelope() {
    let ws = init_workspace();
    create_bead(ws.path(), "claim consistency test");

    let with_envelope = run_envelope_command_toon(ws.path(), &["claim", "--assignee", "worker", "--format", "toon"]);
    let without_envelope = run_command_toon(ws.path(), &["claim", "--assignee", "worker", "--format", "toon"]);

    // Output should be identical regardless of --envelope flag
    assert_eq!(with_envelope, without_envelope, "Claim output should be same with and without --envelope");
}

/// Test: claim --format toon --envelope with empty workspace shows message
#[test]
fn claim_envelope_empty_workspace() {
    let ws = init_workspace();

    let output = run_envelope_command_toon(ws.path(), &["claim", "--assignee", "empty-worker", "--format", "toon"]);

    // Verify output is text message, not JSON
    assert!(!output.starts_with('{'), "Empty claim should output text, not JSON");
    assert!(output.contains("No beads available") || output.contains("no beads") || output.trim().is_empty(),
        "Empty claim should show availability message or be empty");
}

/// Test: claim --format toon --envelope outputs bead ID in toon format
#[test]
fn claim_envelope_outputs_bead_id() {
    let ws = init_workspace();
    let bead_id = create_bead(ws.path(), "claimable bead");

    let output = run_envelope_command_toon(ws.path(), &["claim", "--assignee", "test-assignee", "--format", "toon"]);

    // Verify output contains the bead ID (toon format outputs just the bead ID)
    assert!(output.trim().contains(&bead_id), "Claim output should contain the bead ID");
}

/// Test: claim --format toon --envelope has consistent structure across calls
#[test]
fn claim_envelope_structure_consistency() {
    let ws = init_workspace();

    // First claim
    create_bead(ws.path(), "first bead");
    let output1 = run_envelope_command_toon(ws.path(), &["claim", "--assignee", "worker", "--format", "toon"]);

    // Second claim (should have same structure)
    create_bead(ws.path(), "second bead");
    let output2 = run_envelope_command_toon(ws.path(), &["claim", "--assignee", "worker", "--format", "toon"]);

    // Both outputs should be plain text (not JSON)
    assert!(!output1.starts_with('{'), "First claim should be text");
    assert!(!output2.starts_with('{'), "Second claim should be text");
}

// ============================================================================
// List Command Toon Format Envelope Tests
// ============================================================================

/// Test: list --format toon --envelope outputs plain text (not JSON envelope)
#[test]
fn list_envelope_outputs_plain_text() {
    let ws = init_workspace();
    create_bead(ws.path(), "list test");

    let output = run_envelope_command_toon(ws.path(), &["list", "--format", "toon"]);

    // Verify output is plain text, not JSON
    assert!(!output.starts_with('{'), "List output should not be JSON envelope");
    assert!(!output.contains("\"version\":"), "List output should not contain version field");
    assert!(!output.contains("\"kind\":"), "List output should not contain kind field");
}

/// Test: list --format toon --envelope output matches output without envelope
#[test]
fn list_envelope_output_matches_no_envelope() {
    let ws = init_workspace();
    create_bead(ws.path(), "list consistency test");

    let with_envelope = run_envelope_command_toon(ws.path(), &["list", "--format", "toon"]);
    let without_envelope = run_command_toon(ws.path(), &["list", "--format", "toon"]);

    // Output should be identical regardless of --envelope flag
    assert_eq!(with_envelope, without_envelope, "List output should be same with and without --envelope");
}

/// Test: list --format toon --envelope with empty workspace
#[test]
fn list_envelope_empty_workspace() {
    let ws = init_workspace();

    let output = run_envelope_command_toon(ws.path(), &["list", "--format", "toon"]);

    // Verify output is empty or header-only (not JSON envelope)
    assert!(!output.starts_with('{'), "Empty list should be text or empty, not JSON");
}

/// Test: list --format toon --envelope shows bead information in toon format
#[test]
fn list_envelope_shows_bead_info() {
    let ws = init_workspace();
    let bead_id = create_bead(ws.path(), "list info test");

    let output = run_envelope_command_toon(ws.path(), &["list", "--format", "toon"]);

    // Verify output contains bead information in toon format
    // Toon format for list uses compact line: [id] title - status (priority)
    assert!(output.contains(&bead_id), "List should show bead ID");
    assert!(output.contains("list info test"), "List should show bead title");
}

// ============================================================================
// Ready Command Toon Format Envelope Tests
// ============================================================================

/// Test: ready --format toon --envelope outputs plain text (not JSON envelope)
#[test]
fn ready_envelope_outputs_plain_text() {
    let ws = init_workspace();
    create_bead(ws.path(), "ready test");

    let output = run_envelope_command_toon(ws.path(), &["ready", "--format", "toon"]);

    // Verify output is plain text, not JSON
    assert!(!output.starts_with('{'), "Ready output should not be JSON envelope");
}

/// Test: ready --format toon --envelope output matches output without envelope
#[test]
fn ready_envelope_output_matches_no_envelope() {
    let ws = init_workspace();
    create_bead(ws.path(), "ready consistency test");

    let with_envelope = run_envelope_command_toon(ws.path(), &["ready", "--format", "toon"]);
    let without_envelope = run_command_toon(ws.path(), &["ready", "--format", "toon"]);

    // Output should be identical regardless of --envelope flag
    assert_eq!(with_envelope, without_envelope, "Ready output should be same with and without --envelope");
}

// ============================================================================
// Show Command Toon Format Envelope Tests
// ============================================================================

/// Test: show --format toon --envelope outputs plain text (not JSON envelope)
#[test]
fn show_envelope_outputs_plain_text() {
    let ws = init_workspace();
    let bead_id = create_bead(ws.path(), "show test");

    let output = run_envelope_command_toon(ws.path(), &["show", "--id", &bead_id, "--format", "toon"]);

    // Verify output is plain text, not JSON
    assert!(!output.starts_with('{'), "Show output should not be JSON envelope");
    assert!(output.contains("ID:"), "Show output should contain 'ID:' field");
}

/// Test: show --format toon --envelope output matches output without envelope
#[test]
fn show_envelope_output_matches_no_envelope() {
    let ws = init_workspace();
    let bead_id = create_bead(ws.path(), "show consistency test");

    let with_envelope = run_envelope_command_toon(ws.path(), &["show", "--id", &bead_id, "--format", "toon"]);
    let without_envelope = run_command_toon(ws.path(), &["show", "--id", &bead_id, "--format", "toon"]);

    // Output should be identical regardless of --envelope flag
    assert_eq!(with_envelope, without_envelope, "Show output should be same with and without --envelope");
}

/// Test: show --format toon --envelope shows detailed bead information
#[test]
fn show_envelope_shows_detailed_info() {
    let ws = init_workspace();
    let bead_id = create_bead(ws.path(), "detailed test");

    let output = run_envelope_command_toon(ws.path(), &["show", "--id", &bead_id, "--format", "toon"]);

    // Verify output contains detailed bead fields
    assert!(output.contains("ID:"), "Show should contain ID field");
    assert!(output.contains("Title:"), "Show should contain Title field");
    assert!(output.contains("Status:"), "Show should contain Status field");
    assert!(output.contains("Priority:"), "Show should contain Priority field");
    assert!(output.contains("Type:"), "Show should contain Type field");
}
