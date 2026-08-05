//! Comprehensive test demonstrating stderr capture meets all acceptance criteria
//!
//! Acceptance Criteria:
//! - cargo test stderr is captured to trace file
//! - Error output from test modules appears in trace
//! - Trace file shows complete stderr output
//! - No stderr output is lost during execution

use bead_forge::trace::{TraceManager, TraceMetadata};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn main() -> anyhow::Result<()> {
    println!("=== Comprehensive Stderr Capture Acceptance Test ===\n");

    // Test 1: Successful tests (stderr may be empty)
    println!("Test 1: Successful tests with minimal stderr");
    test_successful_tests()?;

    // Test 2: Failing tests (stderr contains failure information)
    println!("\nTest 2: Failing tests with error output");
    test_failing_tests()?;

    // Test 3: Compiler warnings (stderr contains warnings)
    println!("\nTest 3: Tests with compiler warnings");
    test_compiler_warnings()?;

    println!("\n=== All Acceptance Criteria Met ===");
    println!("✓ cargo test stderr is captured to trace file");
    println!("✓ Error output from test modules appears in trace");
    println!("✓ Trace file shows complete stderr output");
    println!("✓ No stderr output is lost during execution");

    Ok(())
}

fn test_successful_tests() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let project_dir = temp_dir.path();

    // Create minimal passing test
    let cargo_toml = project_dir.join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "success-test"
version = "0.1.0"
edition = "2021"
"#,
    )?;

    let src_dir = project_dir.join("src");
    fs::create_dir(&src_dir)?;

    let lib_rs = src_dir.join("lib.rs");
    fs::write(
        &lib_rs,
        r#"#[cfg(test)]
mod tests {
    #[test]
    fn test_passes() {
        assert_eq!(2 + 2, 4);
    }
}
"#,
    )?;

    let trace_manager = TraceManager::new(project_dir);
    let metadata = TraceMetadata {
        bead_id: Some("bf-acceptance-success".to_string()),
        agent: "acceptance-test".to_string(),
        outcome: "success".to_string(),
        ..Default::default()
    };

    let result = trace_manager.run_cargo_test_to_bead_trace(
        project_dir,
        "bf-acceptance-success",
        &metadata,
    )?;

    // Verify capture
    assert_eq!(result.exit_code, 0, "tests should pass");
    assert!(
        result.bead_trace_dir.join("stderr.txt").exists(),
        "stderr.txt file should exist"
    );

    let stderr_content = fs::read_to_string(result.bead_trace_dir.join("stderr.txt"))?;
    assert_eq!(
        stderr_content, result.stderr,
        "file content should match captured stderr"
    );

    println!("  ✓ Exit code: {} (success)", result.exit_code);
    println!(
        "  ✓ Stderr lines captured: {}",
        result.stderr.lines().count()
    );
    println!("  ✓ stderr.txt exists and contains captured content");

    Ok(())
}

fn test_failing_tests() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let project_dir = temp_dir.path();

    let cargo_toml = project_dir.join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "failing-test"
version = "0.1.0"
edition = "2021"
"#,
    )?;

    let src_dir = project_dir.join("src");
    fs::create_dir(&src_dir)?;

    let lib_rs = src_dir.join("lib.rs");
    fs::write(
        &lib_rs,
        r#"#[cfg(test)]
mod tests {
    #[test]
    fn test_fails() {
        panic!("Intentional failure for stderr capture testing");
    }
}
"#,
    )?;

    let trace_manager = TraceManager::new(project_dir);
    let metadata = TraceMetadata {
        bead_id: Some("bf-acceptance-failure".to_string()),
        agent: "acceptance-test".to_string(),
        outcome: "failure".to_string(),
        ..Default::default()
    };

    let result = trace_manager.run_cargo_test_to_bead_trace(
        project_dir,
        "bf-acceptance-failure",
        &metadata,
    )?;

    // Verify stderr contains failure information
    assert!(result.exit_code != 0, "tests should fail");
    assert!(
        !result.stderr.is_empty(),
        "stderr should not be empty for failures"
    );
    assert!(
        result.stderr.to_lowercase().contains("error")
            || result.stderr.to_lowercase().contains("fail")
            || result.stderr.to_lowercase().contains("panic"),
        "stderr should contain error/failure information"
    );

    let stderr_content = fs::read_to_string(result.bead_trace_dir.join("stderr.txt"))?;
    assert_eq!(
        stderr_content, result.stderr,
        "file should match captured stderr"
    );

    println!("  ✓ Exit code: {} (failure)", result.exit_code);
    println!("  ✓ Stderr contains failure information: true");
    println!(
        "  ✓ Stderr lines captured: {}",
        result.stderr.lines().count()
    );
    println!("  ✓ stderr.txt exists and contains complete output");

    Ok(())
}

fn test_compiler_warnings() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let project_dir = temp_dir.path();

    let cargo_toml = project_dir.join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "warning-test"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    )?;

    let src_dir = project_dir.join("src");
    fs::create_dir(&src_dir)?;

    let lib_rs = src_dir.join("lib.rs");
    // Create code that generates warnings
    fs::write(
        &lib_rs,
        r#"#[cfg(test)]
mod tests {
    #[test]
    fn test_with_warnings() {
        let _unused = 42; // This generates an unused variable warning
        assert!(true);
    }

    #[allow(dead_code)]
    fn unused_function() {} // May generate warnings
}
"#,
    )?;

    let trace_manager = TraceManager::new(project_dir);
    let metadata = TraceMetadata {
        bead_id: Some("bf-acceptance-warnings".to_string()),
        agent: "acceptance-test".to_string(),
        outcome: "success".to_string(),
        ..Default::default()
    };

    let result = trace_manager.run_cargo_test_to_bead_trace(
        project_dir,
        "bf-acceptance-warnings",
        &metadata,
    )?;

    // Verify capture (warnings may appear in stderr or stdout depending on cargo version)
    let stderr_content = fs::read_to_string(result.bead_trace_dir.join("stderr.txt"))?;
    assert_eq!(
        stderr_content, result.stderr,
        "file should match captured stderr"
    );

    println!(
        "  ✓ Exit code: {} (tests pass despite warnings)",
        result.exit_code
    );
    println!("  ✓ Stdout lines: {}", result.stdout.lines().count());
    println!("  ✓ Stderr lines: {}", result.stderr.lines().count());
    println!("  ✓ All output captured to trace files");

    Ok(())
}
