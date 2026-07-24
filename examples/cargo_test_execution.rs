//! Example: Running cargo test and capturing output
//!
//! This example demonstrates how to use the TraceManager to execute
//! cargo test in a workspace and capture all output to trace files.
//!
//! Run with: cargo run --example cargo_test_execution

use anyhow::Result;
use bead_forge::trace::TraceManager;
use std::path::Path;

fn main() -> Result<()> {
    println!("=== Cargo Test Execution Example ===\n");

    // Get the current workspace (bead-forge)
    let workspace_dir = std::env::current_dir()?;
    println!("Workspace: {}", workspace_dir.display());

    // Create a TraceManager for the current workspace
    let trace_manager = TraceManager::new(&workspace_dir);

    println!("\nRunning cargo test...");
    println!("This will execute 'cargo test' and capture all output.\n");

    // Run cargo test and capture output
    match trace_manager.run_cargo_test(&workspace_dir) {
        Ok(result) => {
            println!("✓ Cargo test execution completed");
            println!("  Exit code: {}", result.exit_code);
            println!("  Duration: {}ms", result.duration_ms);
            println!("  Trace file: {}", result.trace_path.display());

            if result.exit_code == 0 {
                println!("\n✓ All tests passed!");
            } else {
                println!("\n✗ Some tests failed (non-zero exit code)");
                println!("  Check the trace file for details");
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to run cargo test: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
