//! Quick demonstration: Run specific cargo test module with output capture
//!
//! This demonstrates cargo test execution with output capture using a single test module
//! to keep execution time manageable while still validating the functionality.

use anyhow::Result;
use bead_forge::trace::{TraceManager, TraceMetadata};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== Quick Cargo Test Execution Demo ===\n");

    // Set up paths
    let needle_dir = PathBuf::from("/home/coding/NEEDLE");
    let bead_forge_dir = PathBuf::from("/home/coding/bead-forge");

    println!("Workspace: {}", needle_dir.display());
    println!("Bead ID: bf-3ezlq4");
    println!("Test filter: bead_store (single module for quick demo)\n");

    // Verify NEEDLE directory exists
    if !needle_dir.exists() {
        anyhow::bail!("NEEDLE directory does not exist: {}", needle_dir.display());
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

    println!("Running cargo test -- bead_store...");
    println!("This will run only the bead_store test module for quick demonstration.\n");

    // Run cargo test with specific test filter and capture output to bead trace directory
    let result = trace_manager.run_cargo_test_to_bead_trace_with_args(
        &needle_dir,
        "bf-3ezlq4",
        &metadata,
        &["--", "bead_store"], // Run only bead_store tests
    )?;

    println!("=== Test Execution Complete ===\n");
    println!("Exit code: {}", result.exit_code);
    println!(
        "Duration: {}ms ({:.2}s)",
        result.duration_ms,
        result.duration_ms as f64 / 1000.0
    );
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
    println!("First 15 lines of stdout:");
    for (i, line) in stdout_lines.iter().take(15).enumerate() {
        println!("  {}: {}", i + 1, line);
    }

    if stdout_lines.len() > 15 {
        println!("  ... ({} more lines)", stdout_lines.len() - 15);
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
    let metadata_path = result.bead_trace_dir.join("metadata.json");
    let stdout_path = result.bead_trace_dir.join("stdout.txt");
    let stderr_path = result.bead_trace_dir.join("stderr.txt");

    println!("metadata.json: {}", metadata_path.display());
    println!("stdout.txt: {}", stdout_path.display());
    println!("stderr.txt: {}", stderr_path.display());

    // Verify files exist
    println!("\n=== Verification ===");
    println!("metadata.json exists: {}", metadata_path.exists());
    println!("stdout.txt exists: {}", stdout_path.exists());
    println!("stderr.txt exists: {}", stderr_path.exists());

    if result.exit_code == 0 {
        println!("\n✓ All bead_store tests passed!");
    } else {
        println!("\n✗ Some bead_store tests failed (non-zero exit code)");
        println!("  Check the trace files for details");
    }

    // Exit with same code as cargo test
    std::process::exit(result.exit_code);
}
