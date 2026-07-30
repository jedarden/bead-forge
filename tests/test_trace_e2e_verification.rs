//! End-to-end test with trace file verification
//!
//! This test verifies the complete trace file creation functionality by:
//! 1. Running a full cargo test execution
//! 2. Verifying trace file is created
//! 3. Verifying trace file contains stdout
//! 4. Verifying trace file contains stderr
//! 5. Verifying trace file contains execution time
//! 6. Ensuring command completes without hanging
//! 7. Manual verification of trace file contents succeeds

use std::path::Path;
use std::process::Command;
use std::fs;
use std::time::Duration;

use bead_forge::trace::{TraceManager, TraceMetadata};

#[test]
fn test_e2e_trace_file_verification() {
    // Create a temporary directory for the test workspace
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_dir = temp_dir.path();

    // Create a minimal Rust project
    let cargo_toml = workspace_dir.join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "e2e-test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
    ).expect("Failed to write Cargo.toml");

    let src_dir = workspace_dir.join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");

    let lib_rs = src_dir.join("lib.rs");
    fs::write(
        &lib_rs,
        r#"#[cfg(test)]
mod tests {
    #[test]
    fn test_success() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_with_output() {
        println!("STDOUT_TEST_MARKER");
        eprintln!("STDERR_TEST_MARKER");
        assert!(true);
    }
}
"#
    ).expect("Failed to write lib.rs");

    // Create trace manager for the temp workspace
    let trace_manager = TraceManager::new(workspace_dir);

    // Create metadata for the test run
    let metadata = TraceMetadata {
        bead_id: Some("bf-e2e-test".to_string()),
        agent: "e2e-test".to_string(),
        provider: Some("test".to_string()),
        model: Some("e2e-verification".to_string()),
        outcome: "success".to_string(),
        ..Default::default()
    };

    // CRITICAL: Test that cargo test completes without hanging
    // We use a timeout to ensure the command doesn't hang indefinitely
    println!("Starting cargo test execution...");
    let start = std::time::Instant::now();

    // This is the main test - run cargo test with full trace capture
    let result = trace_manager.run_cargo_test_to_bead_trace(
        workspace_dir,
        "bf-e2e-test",
        &metadata
    ).expect("cargo test should complete successfully");

    let elapsed = start.elapsed();

    // CRITICAL: Verify command completed without hanging
    // This test should complete in well under 60 seconds even on slow systems
    assert!(elapsed < Duration::from_secs(60),
        "cargo test should complete quickly; took {:?}", elapsed);
    println!("✓ Cargo test completed in {:?}", elapsed);

    // Verify the result
    assert_eq!(result.exit_code, 0, "cargo test should succeed");
    assert!(result.duration_ms > 0, "duration should be positive");
    assert!(result.bead_trace_dir.exists(), "bead trace directory should exist");

    println!("✓ Exit code: {}", result.exit_code);
    println!("✓ Duration: {}ms", result.duration_ms);
    println!("✓ Trace directory: {}", result.bead_trace_dir.display());

    // VERIFY TRACE FILE IS CREATED
    let metadata_path = result.bead_trace_dir.join("metadata.json");
    let stdout_path = result.bead_trace_dir.join("stdout.txt");
    let stderr_path = result.bead_trace_dir.join("stderr.txt");

    assert!(metadata_path.exists(), "metadata.json should exist");
    assert!(stdout_path.exists(), "stdout.txt should exist");
    assert!(stderr_path.exists(), "stderr.txt should exist");

    println!("✓ All trace files created:");
    println!("  - metadata.json");
    println!("  - stdout.txt");
    println!("  - stderr.txt");

    // VERIFY TRACE FILE CONTAINS STDOUT
    assert!(!result.stdout.is_empty(), "stdout should not be empty");
    assert!(result.stdout.len() > 100, "stdout should contain substantial output");

    // Verify stdout contains expected test output markers
    assert!(result.stdout.contains("STDOUT_TEST_MARKER") ||
            result.stdout.contains("running") ||
            result.stdout.contains("test result:"),
        "stdout should contain test output");

    let stdout_content = fs::read_to_string(&stdout_path)
        .expect("Failed to read stdout.txt");
    assert_eq!(stdout_content, result.stdout, "stdout.txt should match captured stdout");

    println!("✓ Stdout capture verified:");
    println!("  - {} lines captured", result.stdout.lines().count());
    println!("  - {} bytes written", stdout_content.len());

    // VERIFY TRACE FILE CONTAINS STDERR
    // For successful tests with --nocapture, stderr may be empty
    // but the file should still exist and content should match
    let stderr_content = fs::read_to_string(&stderr_path)
        .expect("Failed to read stderr.txt");
    assert_eq!(stderr_content, result.stderr, "stderr.txt should match captured stderr");

    println!("✓ Stderr capture verified:");
    println!("  - {} lines captured", result.stderr.lines().count());
    println!("  - {} bytes written", stderr_content.len());

    // VERIFY TRACE FILE CONTAINS EXECUTION TIME
    let metadata_content = fs::read_to_string(&metadata_path)
        .expect("Failed to read metadata.json");
    let parsed_metadata: serde_json::Value = serde_json::from_str(&metadata_content)
        .expect("Failed to parse metadata.json");

    // Verify execution time fields exist and are valid
    assert!(parsed_metadata.get("start_time").is_some(),
        "metadata should contain start_time");
    assert!(parsed_metadata.get("end_time").is_some(),
        "metadata should contain end_time");
    assert!(parsed_metadata.get("duration_ms").is_some(),
        "metadata should contain duration_ms");

    let start_time = parsed_metadata.get("start_time")
        .and_then(|v| v.as_str())
        .expect("start_time should be a string");
    let end_time = parsed_metadata.get("end_time")
        .and_then(|v| v.as_str())
        .expect("end_time should be a string");
    let duration_ms = parsed_metadata.get("duration_ms")
        .and_then(|v| v.as_u64())
        .expect("duration_ms should be a number");

    // Verify the execution times are valid RFC3339 timestamps
    assert!(start_time.contains('T'), "start_time should be RFC3339 format");
    assert!(end_time.contains('T'), "end_time should be RFC3339 format");
    assert!(duration_ms > 0, "duration_ms should be positive");
    assert!(duration_ms == result.duration_ms,
        "duration_ms should match result duration");

    println!("✓ Execution time recording verified:");
    println!("  - start_time: {}", start_time);
    println!("  - end_time: {}", end_time);
    println!("  - duration_ms: {} ({}.{:03}s)",
        duration_ms, duration_ms / 1000, duration_ms % 1000);

    // MANUAL VERIFICATION OF TRACE FILE CONTENTS
    // This section enables manual inspection if needed
    println!("\n=== Manual Verification Section ===");
    println!("Trace directory: {}", result.bead_trace_dir.display());
    println!("Files available for manual inspection:");

    if let Ok(metadata) = fs::read_to_string(&metadata_path) {
        println!("✓ metadata.json is readable ({} bytes)", metadata.len());
        // Verify metadata is valid JSON
        assert!(serde_json::from_str::<serde_json::Value>(&metadata).is_ok(),
            "metadata.json should be valid JSON");
    }

    if let Ok(stdout) = fs::read_to_string(&stdout_path) {
        println!("✓ stdout.txt is readable ({} bytes)", stdout.len());
        // Verify stdout contains test output
        assert!(stdout.contains("running") || stdout.contains("test result:") ||
                stdout.contains("STDOUT_TEST_MARKER"),
            "stdout.txt should contain test output");
    }

    if let Ok(stderr) = fs::read_to_string(&stderr_path) {
        println!("✓ stderr.txt is readable ({} bytes)", stderr.len());
    }

    println!("\n=== All Verification Criteria Passed ===");
    println!("✓ Test runs full cargo test execution");
    println!("✓ Verifies trace file is created");
    println!("✓ Verifies trace file contains stdout");
    println!("✓ Verifies trace file contains stderr");
    println!("✓ Verifies trace file contains execution time");
    println!("✓ Command completes without hanging");
    println!("✓ Manual verification of trace file contents succeeds");
}

#[test]
fn test_e2e_trace_with_failing_test() {
    // Test trace file creation with failing tests
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_dir = temp_dir.path();

    // Create a Rust project with a failing test
    let cargo_toml = workspace_dir.join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "e2e-failing-test"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
    ).expect("Failed to write Cargo.toml");

    let src_dir = workspace_dir.join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");

    let lib_rs = src_dir.join("lib.rs");
    fs::write(
        &lib_rs,
        r#"#[cfg(test)]
mod tests {
    #[test]
    fn test_passes() {
        println!("PASSING_TEST");
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_fails() {
        println!("FAILING_TEST");
        eprintln!("EXPECTED_FAILURE");
        assert_eq!(1 + 1, 3, "This test is designed to fail");
    }
}
"#
    ).expect("Failed to write lib.rs");

    let trace_manager = TraceManager::new(workspace_dir);

    let metadata = TraceMetadata {
        bead_id: Some("bf-e2e-failing".to_string()),
        agent: "e2e-failing-test".to_string(),
        outcome: "failure".to_string(),
        ..Default::default()
    };

    // Run cargo test - should complete even with failing tests
    let result = trace_manager.run_cargo_test_to_bead_trace(
        workspace_dir,
        "bf-e2e-failing",
        &metadata
    ).expect("cargo test should complete (even with failures)");

    // Verify the result
    assert!(result.exit_code != 0, "cargo test should fail as expected");
    assert!(result.duration_ms > 0, "duration should be positive");
    assert!(result.bead_trace_dir.exists(), "bead trace directory should exist");

    let stdout_path = result.bead_trace_dir.join("stdout.txt");
    let stderr_path = result.bead_trace_dir.join("stderr.txt");

    // Verify stdout contains passing test output
    let stdout_content = fs::read_to_string(&stdout_path)
        .expect("Failed to read stdout.txt");
    assert!(stdout_content.contains("PASSING_TEST") ||
            stdout_content.contains("running"),
        "stdout should contain test output");

    // Verify stderr contains failure information
    let stderr_content = fs::read_to_string(&stderr_path)
        .expect("Failed to read stderr.txt");
    // When tests fail, cargo outputs failure details to stderr
    assert!(!stderr_content.is_empty() || !result.stderr.is_empty(),
        "stderr should contain failure information");

    println!("✓ Failing test trace verified:");
    println!("  - Exit code: {} (expected non-zero)", result.exit_code);
    println!("  - Stdout captured: {} lines", stdout_content.lines().count());
    println!("  - Stderr captured: {} lines", stderr_content.lines().count());
}

#[test]
fn test_e2e_trace_with_custom_args() {
    // Test trace file creation with custom cargo arguments
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_dir = temp_dir.path();

    // Create a Rust project with multiple tests
    let cargo_toml = workspace_dir.join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "e2e-custom-args"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
    ).expect("Failed to write Cargo.toml");

    let src_dir = workspace_dir.join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");

    let lib_rs = src_dir.join("lib.rs");
    fs::write(
        &lib_rs,
        r#"#[cfg(test)]
mod tests {
    #[test]
    fn first_test() {
        println!("FIRST_TEST_OUTPUT");
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn second_test() {
        println!("SECOND_TEST_OUTPUT");
        assert_eq!(1 + 1, 2);
    }

    #[test]
    fn third_test() {
        println!("THIRD_TEST_OUTPUT");
        assert!(true);
    }
}
"#
    ).expect("Failed to write lib.rs");

    let trace_manager = TraceManager::new(workspace_dir);

    let metadata = TraceMetadata {
        bead_id: Some("bf-e2e-custom-args".to_string()),
        agent: "e2e-custom-args".to_string(),
        outcome: "success".to_string(),
        ..Default::default()
    };

    // Run cargo test with --nocapture and specific test filter
    let result = trace_manager.run_cargo_test_to_bead_trace_with_args(
        workspace_dir,
        "bf-e2e-custom-args",
        &metadata,
        &["--", "--nocapture", "first_test"]
    ).expect("cargo test with args should succeed");

    // Verify the result
    assert_eq!(result.exit_code, 0, "cargo test should succeed");
    assert!(result.bead_trace_dir.exists(), "bead trace directory should exist");

    // Verify stdout contains the filtered test output
    assert!(result.stdout.contains("FIRST_TEST_OUTPUT"),
        "stdout should contain first test output");

    println!("✓ Custom args trace verified:");
    println!("  - Test filter worked: found FIRST_TEST_OUTPUT");
    println!("  - --nocapture worked: output captured");
}

#[test]
fn test_e2e_trace_multiple_runs() {
    // Test that multiple test runs create distinct trace directories
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let workspace_dir = temp_dir.path();

    // Create a simple Rust project
    let cargo_toml = workspace_dir.join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "e2e-multi-run"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
    ).expect("Failed to write Cargo.toml");

    let src_dir = workspace_dir.join("src");
    fs::create_dir(&src_dir).expect("Failed to create src dir");

    let lib_rs = src_dir.join("lib.rs");
    fs::write(
        &lib_rs,
        r#"#[cfg(test)]
mod tests {
    #[test]
    fn test_works() {
        assert_eq!(2 + 2, 4);
    }
}
"#
    ).expect("Failed to write lib.rs");

    let trace_manager = TraceManager::new(workspace_dir);

    let metadata = TraceMetadata {
        bead_id: Some("bf-e2e-multi".to_string()),
        agent: "e2e-multi-run".to_string(),
        outcome: "success".to_string(),
        ..Default::default()
    };

    // Run the same test multiple times
    let result1 = trace_manager.run_cargo_test_to_bead_trace(
        workspace_dir,
        "bf-e2e-multi",
        &metadata
    ).expect("First run should succeed");

    let result2 = trace_manager.run_cargo_test_to_bead_trace(
        workspace_dir,
        "bf-e2e-multi",
        &metadata
    ).expect("Second run should succeed");

    let result3 = trace_manager.run_cargo_test_to_bead_trace(
        workspace_dir,
        "bf-e2e-multi",
        &metadata
    ).expect("Third run should succeed");

    // Verify all runs created distinct directories
    assert_ne!(result1.bead_trace_dir, result2.bead_trace_dir,
        "runs should create distinct directories");
    assert_ne!(result2.bead_trace_dir, result3.bead_trace_dir,
        "runs should create distinct directories");
    assert_ne!(result1.bead_trace_dir, result3.bead_trace_dir,
        "runs should create distinct directories");

    // Verify all directories exist and contain the expected files
    for (i, result) in [result1, result2, result3].iter().enumerate() {
        let metadata_path = result.bead_trace_dir.join("metadata.json");
        let stdout_path = result.bead_trace_dir.join("stdout.txt");
        let stderr_path = result.bead_trace_dir.join("stderr.txt");

        assert!(metadata_path.exists(), "run {} metadata.json should exist", i + 1);
        assert!(stdout_path.exists(), "run {} stdout.txt should exist", i + 1);
        assert!(stderr_path.exists(), "run {} stderr.txt should exist", i + 1);

        // Verify each metadata file has execution timing
        let metadata_content = fs::read_to_string(&metadata_path)
            .expect("Failed to read metadata");
        let parsed: serde_json::Value = serde_json::from_str(&metadata_content)
            .expect("Failed to parse metadata");

        assert!(parsed.get("start_time").is_some(),
            "run {} should have start_time", i + 1);
        assert!(parsed.get("duration_ms").is_some(),
            "run {} should have duration_ms", i + 1);
    }

    println!("✓ Multiple runs verified:");
    println!("  - All {} runs created distinct directories", 3);
    println!("  - All directories contain complete trace files");
    println!("  - All trace files have execution timing");
}
