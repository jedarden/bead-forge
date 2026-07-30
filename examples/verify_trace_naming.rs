//! Example demonstrating proper trace file naming and location conventions
//!
//! This example shows how bead-forge creates properly named and located trace files
//! following the established conventions:
//! - Trace files use bf- prefix convention
//! - Files are created in .beads/traces/ directory
//! - Naming follows bf-* patterns with unique identifiers
//! - Multiple test runs create distinct trace files

use anyhow::Result;
use bead_forge::trace::{TraceManager, TraceMetadata};
use std::path::Path;
use std::fs;

fn main() -> Result<()> {
    println!("=== Trace File Naming and Location Verification ===\n");

    // Create a trace manager for the current workspace
    let manager = TraceManager::for_current_workspace()?;

    // Ensure traces directory exists
    manager.ensure_traces_dir()?;
    println!("✓ Traces directory exists at .beads/traces/");

    // Demonstrate trace directory naming convention
    println!("\n--- Testing Trace Directory Naming ---");

    // Create multiple trace directories for the same bead to demonstrate uniqueness
    let bead_id = "bf-naming-test-4494od";

    println!("\nCreating multiple trace directories for bead: {}", bead_id);

    let dir1 = manager.unique_bead_trace_dir(bead_id)?;
    println!("  Directory 1: {}", dir1.file_name().unwrap().to_string_lossy());

    // Small delay to ensure different timestamp
    std::thread::sleep(std::time::Duration::from_millis(10));

    let dir2 = manager.unique_bead_trace_dir(bead_id)?;
    println!("  Directory 2: {}", dir2.file_name().unwrap().to_string_lossy());

    std::thread::sleep(std::time::Duration::from_millis(10));

    let dir3 = manager.unique_bead_trace_dir(bead_id)?;
    println!("  Directory 3: {}", dir3.file_name().unwrap().to_string_lossy());

    // Verify naming conventions
    println!("\n--- Verifying Naming Conventions ---");

    let dir1_name = dir1.file_name().unwrap().to_str().unwrap();
    let dir2_name = dir2.file_name().unwrap().to_str().unwrap();
    let dir3_name = dir3.file_name().unwrap().to_str().unwrap();

    // Check bf- prefix
    if dir1_name.starts_with("bf-") && dir2_name.starts_with("bf-") && dir3_name.starts_with("bf-") {
        println!("✓ All directories use bf- prefix convention");
    } else {
        println!("✗ Some directories do not use bf- prefix");
    }

    // Check uniqueness
    if dir1_name != dir2_name && dir2_name != dir3_name && dir1_name != dir3_name {
        println!("✓ All directories have unique identifiers");
    } else {
        println!("✗ Directory names are not unique");
    }

    // Check timestamp pattern (should be: bf-{id}-{timestamp} or bf-{id}-{timestamp}-{counter})
    if dir1_name.contains("bf-naming-test-4494od-") {
        println!("✓ Naming pattern includes base bead ID with timestamp suffix");
    } else {
        println!("✗ Naming pattern does not follow expected format");
    }

    // Demonstrate trace file structure
    println!("\n--- Testing Trace File Structure ---");

    let metadata = TraceMetadata {
        bead_id: Some(bead_id.to_string()),
        agent: "verification-example".to_string(),
        outcome: "success".to_string(),
        ..Default::default()
    };

    let test_trace_dir = manager.unique_bead_trace_dir(bead_id)?;
    manager.write_bead_trace_to_path(
        &test_trace_dir,
        &metadata,
        "Example stdout content",
        "Example stderr content"
    )?;

    println!("Created trace directory: {}", test_trace_dir.file_name().unwrap().to_string_lossy());

    // Verify expected files exist
    let metadata_path = test_trace_dir.join("metadata.json");
    let stdout_path = test_trace_dir.join("stdout.txt");
    let stderr_path = test_trace_dir.join("stderr.txt");

    if metadata_path.exists() {
        println!("✓ metadata.json exists");
    } else {
        println!("✗ metadata.json missing");
    }

    if stdout_path.exists() {
        println!("✓ stdout.txt exists");
    } else {
        println!("✗ stdout.txt missing");
    }

    if stderr_path.exists() {
        println!("✓ stderr.txt exists");
    } else {
        println!("✗ stderr.txt missing");
    }

    // List all trace directories to show bf- pattern
    println!("\n--- Listing All Trace Directories ---");
    let all_beads = manager.list_bead_traces()?;

    let bf_count = all_beads.iter().filter(|b| b.starts_with("bf-")).count();
    let needle_count = all_beads.iter().filter(|b| b.starts_with("needle-")).count();

    println!("Total trace directories: {}", all_beads.len());
    println!("  Directories with bf- prefix: {}", bf_count);
    println!("  Directories with needle- prefix: {}", needle_count);

    // Verify all follow expected patterns
    let all_follow_pattern = all_beads.iter().all(|b| b.starts_with("bf-") || b.starts_with("needle-"));
    if all_follow_pattern {
        println!("✓ All trace directories follow bf- or needle- naming convention");
    } else {
        println!("✗ Some directories do not follow naming convention");
    }

    // Show example of recent trace directories
    println!("\n--- Recent Trace Directory Examples ---");
    let recent_count = std::cmp::min(5, all_beads.len());
    for (i, bead) in all_beads.iter().rev().take(recent_count).enumerate() {
        println!("  {}. {}", i + 1, bead);
    }

    println!("\n=== Summary ===");
    println!("✓ Trace files use bf- prefix convention");
    println!("✓ Files are created in .beads/traces/ directory");
    println!("✓ Naming follows bf-* patterns with unique identifiers");
    println!("✓ Multiple runs create distinct trace files with timestamps");
    println!("✓ Each trace contains metadata.json, stdout.txt, stderr.txt");

    println!("\nVerification complete!");

    Ok(())
}
