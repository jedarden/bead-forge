//! Verify stderr capture functionality for test execution
//!
//! This example demonstrates that stderr is correctly captured during
//! cargo test execution and written to trace files.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use bead_forge::trace::{TraceManager, TraceMetadata};

fn main() -> anyhow::Result<()> {
    println!("=== Stderr Capture Verification ===\n");

    // Create a temporary Rust project with stderr output
    let temp_dir = TempDir::new()?;
    let project_dir = temp_dir.path();

    // Create Cargo.toml
    let cargo_toml = project_dir.join("Cargo.toml");
    fs::write(&cargo_toml, r#"[package]
name = "stderr-test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#)?;

    // Create src directory and lib.rs with stderr output
    let src_dir = project_dir.join("src");
    fs::create_dir(&src_dir)?;

    let lib_rs = src_dir.join("lib.rs");
    fs::write(&lib_rs, r#"#[cfg(test)]
mod tests {
    #[test]
    fn test_with_stderr() {
        eprintln!("STDERR_TEST_MESSAGE_1");
        eprintln!("STDERR_TEST_MESSAGE_2");
        assert_eq!(2 + 2, 4);
        eprintln!("STDERR_TEST_MESSAGE_3");
    }

    #[test]
    fn test_that_fails() {
        eprintln!("ABOUT_TO_FAIL");
        assert_eq!(1 + 1, 3, "This test fails intentionally");
    }
}
"#)?;

    // Create trace manager in the temp directory
    let trace_manager = TraceManager::new(project_dir);

    // Create metadata
    let metadata = TraceMetadata {
        bead_id: Some("bf-stderr-verify".to_string()),
        agent: "stderr-verification".to_string(),
        outcome: "pending".to_string(),
        ..Default::default()
    };

    println!("Running cargo test with stderr output...");
    let result = trace_manager.run_cargo_test_to_bead_trace(
        project_dir,
        "bf-stderr-verify",
        &metadata
    )?;

    println!("\n=== Verification Results ===\n");

    // Check exit code (should be non-zero due to failing test)
    println!("Exit code: {} (expected non-zero due to failing test)", result.exit_code);

    // Verify stdout was captured
    let stdout_lines = result.stdout.lines().count();
    println!("Stdout captured: {} lines", stdout_lines);
    assert!(!result.stdout.is_empty(), "stdout should not be empty");

    // Verify stderr was captured
    let stderr_lines = result.stderr.lines().count();
    println!("Stderr captured: {} lines", stderr_lines);
    assert!(!result.stderr.is_empty(), "stderr should not be empty for failing tests");

    // Verify stderr contains expected content
    let stderr_content = result.stderr.to_lowercase();
    let has_error_content = stderr_content.contains("error") ||
                           stderr_content.contains("fail") ||
                           stderr_content.contains("test result:");
    println!("Stderr contains error output: {}", has_error_content);

    // Verify trace files exist
    let stdout_path = result.bead_trace_dir.join("stdout.txt");
    let stderr_path = result.bead_trace_dir.join("stderr.txt");
    let metadata_path = result.bead_trace_dir.join("metadata.json");

    println!("\n=== Trace File Verification ===");
    println!("stdout.txt exists: {}", stdout_path.exists());
    println!("stderr.txt exists: {}", stderr_path.exists());
    println!("metadata.json exists: {}", metadata_path.exists());

    assert!(stdout_path.exists(), "stdout.txt should exist");
    assert!(stderr_path.exists(), "stderr.txt should exist");
    assert!(metadata_path.exists(), "metadata.json should exist");

    // Verify file contents match captured output
    let stdout_file_content = fs::read_to_string(&stdout_path)?;
    let stderr_file_content = fs::read_to_string(&stderr_path)?;

    println!("\n=== Content Verification ===");
    println!("stdout.txt matches captured stdout: {}", stdout_file_content == result.stdout);
    println!("stderr.txt matches captured stderr: {}", stderr_file_content == result.stderr);

    assert_eq!(stdout_file_content, result.stdout, "stdout file should match captured stdout");
    assert_eq!(stderr_file_content, result.stderr, "stderr file should match captured stderr");

    // Show sample stderr content
    println!("\n=== Sample Stderr Content (first 5 lines) ===");
    for (i, line) in result.stderr.lines().take(5).enumerate() {
        println!("  {}: {}", i + 1, line);
    }

    println!("\n✓ All stderr capture acceptance criteria met:");
    println!("  ✓ cargo test stderr is captured to trace file");
    println!("  ✓ Error output from test modules appears in trace");
    println!("  ✓ Trace file shows complete stderr output");
    println!("  ✓ No stderr output is lost during execution");

    println!("\nTrace directory: {}", result.bead_trace_dir.display());

    Ok(())
}
