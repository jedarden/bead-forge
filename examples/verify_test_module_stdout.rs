//! Comprehensive verification that stdout from test modules is captured correctly
//!
//! This verifies that:
//! 1. cargo test stdout is captured to trace file
//! 2. Standard output from test modules appears in trace
//! 3. Trace file shows complete stdout output
//! 4. No stdout output is lost during execution

use anyhow::Result;
use bead_forge::trace::{TraceManager, TraceMetadata};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== Comprehensive Test Module Stdout Verification ===\n");

    // Create a temporary test project with stdout output
    let test_project_dir = PathBuf::from("/tmp/stdout_test_project");

    if !test_project_dir.exists() {
        anyhow::bail!("Test project directory not found at: {}", test_project_dir.display());
    }

    println!("✓ Test project directory exists: {}", test_project_dir.display());
    println!();

    // Create trace manager for bead-forge workspace
    let trace_manager = TraceManager::for_current_workspace()?;
    println!("✓ Trace manager created for bead-forge workspace");

    // Create metadata for the test run
    let metadata = TraceMetadata {
        bead_id: Some("bf-5uq6lp".to_string()), // This bead ID
        agent: "stdout-verification".to_string(),
        provider: Some("test".to_string()),
        model: Some("verification".to_string()),
        outcome: "pending".to_string(),
        ..Default::default()
    };
    println!("✓ Metadata created for bead bf-5uq6lp");
    println!();

    println!("=== Running cargo test with --nocapture ===");
    println!("Testing stdout capture from test modules...\n");

    // Run cargo test with --nocapture to capture test stdout
    let result = trace_manager.run_cargo_test_to_bead_trace_with_args(
        &test_project_dir,
        "bf-5uq6lp",
        &metadata,
        &["--", "--nocapture"] // CRITICAL: --nocapture shows test stdout
    )?;

    println!("=== Test Execution Complete ===\n");
    println!("Exit code: {}", result.exit_code);
    println!("Duration: {}ms ({:.2}s)", result.duration_ms, result.duration_ms as f64 / 1000.0);
    println!("Trace directory: {}", result.bead_trace_dir.display());
    println!();

    // Verify stdout capture
    println!("=== Acceptance Criteria 1: cargo test stdout captured to trace file ===");
    println!("Stdout length: {} bytes", result.stdout.len());
    println!("Stdout lines: {}", result.stdout.lines().count());
    println!();

    if result.stdout.is_empty() {
        anyhow::bail!("FAIL: Stdout was not captured - empty result.stdout");
    }
    println!("✓ Stdout captured to memory");

    // Verify stdout contains test module output
    println!("\n=== Acceptance Criteria 2: Standard output from test modules appears in trace ===");

    let expected_lines = vec![
        "MODULE_STDOUT_LINE_1",
        "MODULE_STDOUT_LINE_2",
        "MODULE_STDOUT_LINE_3"
    ];

    let mut all_found = true;
    for expected in &expected_lines {
        if result.stdout.contains(expected) {
            println!("✓ Found expected output: {}", expected);
        } else {
            println!("✗ Missing expected output: {}", expected);
            all_found = false;
        }
    }

    if !all_found {
        println!("\nFAIL: Not all expected stdout lines were found");
        println!("Complete stdout content:\n{}", result.stdout);
        anyhow::bail!("Stdout capture incomplete");
    }

    println!("\n✓ Standard output from test modules captured correctly");

    // Verify stdout file exists and contains content
    println!("\n=== Acceptance Criteria 3: Trace file shows complete stdout output ===");
    let stdout_path = result.bead_trace_dir.join("stdout.txt");

    if !stdout_path.exists() {
        anyhow::bail!("FAIL: stdout.txt file does not exist at: {}", stdout_path.display());
    }
    println!("✓ stdout.txt file exists at: {}", stdout_path.display());

    let stdout_content = std::fs::read_to_string(&stdout_path)
        .map_err(|e| anyhow::anyhow!("Failed to read stdout.txt: {}: {}", stdout_path.display(), e))?;

    println!("✓ stdout.txt file is readable: {} bytes", stdout_content.len());

    // Verify file content matches captured stdout
    if stdout_content != result.stdout {
        println!("✗ FAIL: File content does not match captured stdout");
        println!("File length: {}, Captured length: {}", stdout_content.len(), result.stdout.len());
        anyhow::bail!("stdout.txt content mismatch");
    }
    println!("✓ stdout.txt content matches captured stdout");

    // Verify no stdout output is lost
    println!("\n=== Acceptance Criteria 4: No stdout output is lost during execution ===");

    // Check that we have all expected content
    let stdout_lines: Vec<&str> = stdout_content.lines().collect();
    println!("Total stdout lines: {}", stdout_lines.len());

    // Count expected markers
    let test_output_count = stdout_content.matches("MODULE_STDOUT_LINE").count();
    println!("Test stdout lines found: {}", test_output_count);

    if test_output_count < 3 {
        println!("✗ FAIL: Expected 3 test stdout lines, found {}", test_output_count);
        anyhow::bail!("Stdout output lost during execution");
    }
    println!("✓ All expected stdout output preserved");

    // Verify complete trace structure
    println!("\n=== Verifying complete trace structure ===");
    let metadata_path = result.bead_trace_dir.join("metadata.json");
    let stderr_path = result.bead_trace_dir.join("stderr.txt");

    if !metadata_path.exists() {
        anyhow::bail!("FAIL: metadata.json missing");
    }
    println!("✓ metadata.json exists");

    if !stderr_path.exists() {
        anyhow::bail!("FAIL: stderr.txt missing");
    }
    println!("✓ stderr.txt exists");

    println!("\n=== Sample stdout content (first 10 lines) ===");
    for (i, line) in stdout_content.lines().take(10).enumerate() {
        println!("  {}: {}", i + 1, line);
    }

    // Verify metadata contains execution info
    println!("\n=== Verifying metadata execution info ===");
    let metadata_content = std::fs::read_to_string(&metadata_path)?;
    let parsed_metadata: serde_json::Value = serde_json::from_str(&metadata_content)?;

    if let Some(exit_code) = parsed_metadata.get("exit_code") {
        println!("✓ exit_code in metadata: {}", exit_code);
    }
    if let Some(start_time) = parsed_metadata.get("start_time") {
        println!("✓ start_time in metadata: {}", start_time);
    }
    if let Some(end_time) = parsed_metadata.get("end_time") {
        println!("✓ end_time in metadata: {}", end_time);
    }
    if let Some(duration_ms) = parsed_metadata.get("duration_ms") {
        println!("✓ duration_ms in metadata: {}", duration_ms);
    }

    println!("\n=== ✓ ALL ACCEPTANCE CRITERIA MET ===");
    println!("✓ AC1: cargo test stdout captured to trace file");
    println!("✓ AC2: Standard output from test modules appears in trace");
    println!("✓ AC3: Trace file shows complete stdout output");
    println!("✓ AC4: No stdout output is lost during execution");
    println!();
    println!("Trace files available at: {}", result.bead_trace_dir.display());
    println!("Stdout capture verification complete!");

    Ok(())
}
