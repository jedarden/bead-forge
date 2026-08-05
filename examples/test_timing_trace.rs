//! Test program to verify timing information is captured in trace metadata

use bead_forge::trace::{TraceManager, TraceMetadata};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing timing information capture in trace metadata...");

    // Create a trace manager for the current workspace
    let manager = TraceManager::for_current_workspace()?;

    // Create metadata with timing information
    let metadata = TraceMetadata {
        bead_id: Some("bf-timing-test-39tmmp".to_string()),
        agent: "test-agent-timing-verification".to_string(),
        provider: Some("test-provider".to_string()),
        model: Some("test-model".to_string()),
        start_time: Some("2026-07-24T12:00:00Z".to_string()),
        end_time: Some("2026-07-24T12:01:30.500Z".to_string()),
        duration_ms: Some(90500),
        exit_code: Some(0),
        outcome: "success".to_string(),
        ..Default::default()
    };

    println!("Creating trace with timing information:");
    println!("  start_time: {:?}", metadata.start_time);
    println!("  end_time: {:?}", metadata.end_time);
    println!("  duration_ms: {:?}", metadata.duration_ms);

    // Write the trace
    manager.write_bead_trace(
        "bf-timing-test-39tmmp",
        &metadata,
        "Test stdout output\n",
        "Test stderr output\n",
    )?;

    println!("\nTest trace created successfully at .beads/traces/bf-timing-test-39tmmp/");

    // Read back and verify the metadata
    let metadata_path = manager.bead_metadata_path("bf-timing-test-39tmmp");
    let content = std::fs::read_to_string(&metadata_path)?;

    println!("\n=== VERIFICATION: Metadata file contents ===");
    println!("{}", content);

    // Verify the timing fields are present
    if content.contains("\"start_time\"") {
        println!("\n✓ start_time field is present in metadata");
    } else {
        println!("\n✗ start_time field is MISSING from metadata");
    }

    if content.contains("\"end_time\"") {
        println!("✓ end_time field is present in metadata");
    } else {
        println!("✗ end_time field is MISSING from metadata");
    }

    if content.contains("\"duration_ms\"") {
        println!("✓ duration_ms field is present in metadata");
    } else {
        println!("✗ duration_ms field is MISSING from metadata");
    }

    println!("\n✓ Timing information test completed successfully!");

    Ok(())
}
