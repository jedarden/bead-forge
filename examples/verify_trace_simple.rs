// Simple verification of trace capture for cargo test execution
//
// This example demonstrates that trace capture works for cargo test:
// 1. Stdout/stderr capture
// 2. Execution time recording
// 3. Complete trace file format

use anyhow::Result;
use bead_forge::trace::{TraceManager, TraceMetadata};
use tempfile::TempDir;

fn main() -> Result<()> {
    println!("=== Verifying Trace Capture for Cargo Test ===\n");

    // Create a temporary workspace for testing
    let temp_dir = TempDir::new()?;
    let workspace_dir = temp_dir.path();

    // Create a minimal Rust project
    let cargo_toml = workspace_dir.join("Cargo.toml");
    std::fs::write(
        &cargo_toml,
        r#"[package]
name = "verify-trace-test"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
    )?;

    let src_dir = workspace_dir.join("src");
    std::fs::create_dir(&src_dir)?;

    let lib_rs = src_dir.join("lib.rs");
    std::fs::write(
        &lib_rs,
        r#"#[cfg(test)]
mod tests {
    #[test]
    fn test_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_with_output() {
        println!("TEST_STDOUT_OUTPUT");
        assert!(true);
    }
}
"#
    )?;

    println!("✓ Created test workspace");

    // Create trace manager
    let trace_manager = TraceManager::new(workspace_dir);
    trace_manager.ensure_traces_dir()?;
    println!("✓ Trace manager initialized");

    // Create metadata
    let metadata = TraceMetadata {
        bead_id: Some("bf-verify-trace".to_string()),
        agent: "verification-script".to_string(),
        provider: Some("test".to_string()),
        model: Some("verification".to_string()),
        outcome: "success".to_string(),
        ..Default::default()
    };
    println!("✓ Test metadata created");

    // Run cargo test with trace capture
    println!("\n=== Running cargo test with trace capture ===");
    let result = trace_manager.run_cargo_test_to_bead_trace(
        workspace_dir,
        "bf-verify-trace",
        &metadata
    )?;

    println!("✓ Cargo test completed");
    println!("  Exit code: {}", result.exit_code);
    println!("  Duration: {}ms ({:.2}s)", result.duration_ms, result.duration_ms as f64 / 1000.0);
    println!("  Trace directory: {}", result.bead_trace_dir.display());

    // Verify stdout capture
    println!("\n=== Stdout Capture ===");
    if !result.stdout.is_empty() {
        println!("✓ Stdout captured ({} bytes)", result.stdout.len());
        if result.stdout.contains("TEST_STDOUT_OUTPUT") {
            println!("✓ Test stdout output found in captured stdout");
        } else {
            println!("⚠ Test stdout output not found (may be filtered by cargo)");
        }
    } else {
        println!("✗ Stdout is empty");
    }

    // Verify stderr capture
    println!("\n=== Stderr Capture ===");
    if !result.stderr.is_empty() {
        println!("✓ Stderr captured ({} bytes)", result.stderr.len());
    } else {
        println!("✓ Stderr is empty (no errors/warnings)");
    }

    // Verify execution time recording
    println!("\n=== Execution Time Recording ===");
    if result.duration_ms > 0 {
        println!("✓ Execution time recorded: {}ms", result.duration_ms);
    } else {
        println!("✗ Execution time not recorded");
    }

    if let Some(start) = &result.start_time {
        println!("✓ Start time: {}", start);
    }
    if let Some(end) = &result.end_time {
        println!("✓ End time: {}", end);
    }

    // Verify trace file format completeness
    println!("\n=== Trace File Format Verification ===");
    let metadata_path = result.bead_trace_dir.join("metadata.json");
    let stdout_path = result.bead_trace_dir.join("stdout.txt");
    let stderr_path = result.bead_trace_dir.join("stderr.txt");

    let mut all_files_exist = true;

    if metadata_path.exists() {
        println!("✓ metadata.json exists");
        let content = std::fs::read_to_string(&metadata_path)?;
        let parsed: serde_json::Value = serde_json::from_str(&content)?;

        // Verify required fields
        let required_fields = ["bead_id", "agent", "exit_code", "outcome", "start_time", "end_time", "duration_ms"];
        for field in &required_fields {
            if parsed.get(*field).is_some() {
                println!("  ✓ Field '{}': present", field);
            } else {
                println!("  ✗ Field '{}': MISSING", field);
                all_files_exist = false;
            }
        }
    } else {
        println!("✗ metadata.json MISSING");
        all_files_exist = false;
    }

    if stdout_path.exists() {
        println!("✓ stdout.txt exists");
        let size = std::fs::metadata(&stdout_path)?.len();
        println!("  Size: {} bytes", size);
    } else {
        println!("✗ stdout.txt MISSING");
        all_files_exist = false;
    }

    if stderr_path.exists() {
        println!("✓ stderr.txt exists");
        let size = std::fs::metadata(&stderr_path)?.len();
        println!("  Size: {} bytes", size);
    } else {
        println!("✗ stderr.txt MISSING");
        all_files_exist = false;
    }

    // Final verification summary
    println!("\n=== Verification Summary ===");
    if all_files_exist && result.exit_code == 0 && result.duration_ms > 0 {
        println!("✓ ALL ACCEPTANCE CRITERIA MET:");
        println!("  ✓ Trace capture mechanism works for cargo test");
        println!("  ✓ Both stdout and stderr are captured");
        println!("  ✓ Execution time is recorded");
        println!("  ✓ Trace file format is complete and valid");
        println!("\nTrace files available at: {}", result.bead_trace_dir.display());
        return Ok(());
    } else {
        println!("✗ Some acceptance criteria not met");
        if !all_files_exist {
            println!("  ✗ Trace file format incomplete");
        }
        if result.exit_code != 0 {
            println!("  ✗ Tests failed (exit code {})", result.exit_code);
        }
        if result.duration_ms == 0 {
            println!("  ✗ Execution time not recorded");
        }
        anyhow::bail!("Verification failed");
    }
}
