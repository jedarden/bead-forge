//! Example: Run cargo test in NEEDLE directory and capture output to trace files
//!
//! This demonstrates the core functionality for running cargo test with output capture.
//! The execution writes to bead-specific trace directories with metadata.json, stdout.txt, and stderr.txt.
//!
//! Run with: cargo run --example cargo_test_execution

use anyhow::Result;
use bead_forge::trace::{TraceManager, TraceMetadata};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== Cargo Test Execution with Output Capture ===\n");

    // Set up paths
    let needle_dir = PathBuf::from("/home/coding/NEEDLE");
    let bead_forge_dir = PathBuf::from("/home/coding/bead-forge");

    println!("Workspace: {}", needle_dir.display());
    println!("Bead ID: bf-3ezlq4");
    println!("Trace directory: .beads/traces/bf-3ezlq4/\n");

    // Verify NEEDLE directory exists
    if !needle_dir.exists() {
        anyhow::bail!("NEEDLE directory does not exist: {}", needle_dir.display());
    }

    // Verify NEEDLE has Cargo.toml
    let cargo_toml = needle_dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        anyhow::bail!("NEEDLE directory does not contain Cargo.toml: {}", needle_dir.display());
    }

    // Create trace manager for bead-forge workspace
    let trace_manager = TraceManager::new(&bead_forge_dir);

    // Create metadata for the execution
    let metadata = TraceMetadata {
        bead_id: Some("bf-3ezlq4".to_string()),
        agent: "claude-code-glm-4.7".to_string(),
        provider: Some("zai".to_string()),
        model: Some("glm-4.7".to_string()),
        outcome: "pending".to_string(),
        ..Default::default()
    };

    println!("Running cargo test in {}...", needle_dir.display());
    println!("This may take several minutes...\n");

    // Run cargo test and capture output to bead trace directory
    let result = trace_manager.run_cargo_test_to_bead_trace(
        &needle_dir,
        "bf-3ezlq4",
        &metadata
    )?;

    println!("=== Test Execution Complete ===\n");
    println!("Exit code: {}", result.exit_code);
    println!("Duration: {}ms ({:.2}s)", result.duration_ms, result.duration_ms as f64 / 1000.0);
    println!("Trace directory: {}", result.bead_trace_dir.display());
    println!("Start time: {}", result.start_time.unwrap_or_default());
    println!("End time: {}", result.end_time.unwrap_or_default());
    println!();

    // Show output summary
    let stdout_lines: Vec<&str> = result.stdout.lines().collect();
    let stderr_lines: Vec<&str> = result.stderr.lines().collect();

    println!("Output summary:");
    println!("  Stdout lines: {}", stdout_lines.len());
    println!("  Stderr lines: {}", stderr_lines.len());
    println!();

    // Show first few lines of stdout
    println!("First 10 lines of stdout:");
    for (i, line) in stdout_lines.iter().take(10).enumerate() {
        println!("  {}: {}", i + 1, line);
    }

    if stdout_lines.len() > 10 {
        println!("  ... ({} more lines)", stdout_lines.len() - 10);
    }

    // Show first few lines of stderr if any
    if !stderr_lines.is_empty() {
        println!("\nFirst 10 lines of stderr:");
        for (i, line) in stderr_lines.iter().take(10).enumerate() {
            println!("  {}: {}", i + 1, line);
        }

        if stderr_lines.len() > 10 {
            println!("  ... ({} more lines)", stderr_lines.len() - 10);
        }
    }

    println!("\n=== Files Written ===");
    println!("metadata.json: {}", result.bead_trace_dir.join("metadata.json").display());
    println!("stdout.txt: {}", result.bead_trace_dir.join("stdout.txt").display());
    println!("stderr.txt: {}", result.bead_trace_dir.join("stderr.txt").display());

    if result.exit_code == 0 {
        println!("\n✓ All tests passed!");
    } else {
        println!("\n✗ Some tests failed (non-zero exit code)");
        println!("  Check the trace files for details");
    }

    // Exit with same code as cargo test
    std::process::exit(result.exit_code);
}
