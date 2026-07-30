//! Integration tests for cargo test execution functionality
//!
//! This test module verifies that the TraceManager can successfully
//! execute cargo test commands and capture their output.

use bead_forge::trace::{TraceManager, CargoTestResult};
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_integration_cargo_test_execution() {
    // Create a temporary directory with a minimal Rust project
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path();

    // Create Cargo.toml
    let cargo_toml = workspace_dir.join("Cargo.toml");
    std::fs::write(
        &cargo_toml,
        r#"[package]
name = "integration-test"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
    ).unwrap();

    // Create src directory and lib.rs
    let src_dir = workspace_dir.join("src");
    std::fs::create_dir(&src_dir).unwrap();

    let lib_rs = src_dir.join("lib.rs");
    std::fs::write(
        &lib_rs,
        r#"#[cfg(test)]
mod tests {
    #[test]
    fn test_addition() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_multiplication() {
        assert_eq!(3 * 3, 9);
    }
}
"#
    ).unwrap();

    // Create a TraceManager and run cargo test
    let trace_manager = TraceManager::new(workspace_dir);
    let result: CargoTestResult = trace_manager.run_cargo_test(workspace_dir).unwrap();

    // Verify results
    assert_eq!(result.exit_code, 0, "cargo test should succeed");
    assert!(result.duration_ms > 0, "should have positive duration");
    assert!(result.trace_path.exists(), "trace file should exist");

    // Verify trace file content
    let content = std::fs::read_to_string(&result.trace_path).unwrap();
    assert!(content.contains("=== STDOUT ==="));
    assert!(content.contains("=== STDERR ==="));
    assert!(content.contains("=== EXIT CODE: 0 ==="));
    assert!(content.contains("test_addition") || content.contains("test_multiplication"));
}

#[test]
fn test_integration_cargo_test_with_failing_test() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path();

    // Create a project with a failing test
    let cargo_toml = workspace_dir.join("Cargo.toml");
    std::fs::write(
        &cargo_toml,
        r#"[package]
name = "failing-test"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
    ).unwrap();

    let src_dir = workspace_dir.join("src");
    std::fs::create_dir(&src_dir).unwrap();

    let lib_rs = src_dir.join("lib.rs");
    std::fs::write(
        &lib_rs,
        r#"#[cfg(test)]
mod tests {
    #[test]
    fn test_failure() {
        panic!("This test is designed to fail");
    }
}
"#
    ).unwrap();

    // Run cargo test - it should complete even with failing tests
    let trace_manager = TraceManager::new(workspace_dir);
    let result = trace_manager.run_cargo_test(workspace_dir).unwrap();

    // Should complete with non-zero exit code
    assert!(result.exit_code != 0, "cargo test should fail");
    assert!(result.duration_ms > 0);
    assert!(result.trace_path.exists());

    // Verify error output is captured
    let content = std::fs::read_to_string(&result.trace_path).unwrap();
    assert!(content.contains("=== STDERR ==="));
    assert!(content.contains("=== EXIT CODE"));
}

#[test]
fn test_integration_cargo_test_with_specific_test() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path();

    // Create a project with multiple tests
    let cargo_toml = workspace_dir.join("Cargo.toml");
    std::fs::write(
        &cargo_toml,
        r#"[package]
name = "multi-test"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
    ).unwrap();

    let src_dir = workspace_dir.join("src");
    std::fs::create_dir(&src_dir).unwrap();

    let lib_rs = src_dir.join("lib.rs");
    std::fs::write(
        &lib_rs,
        r#"#[cfg(test)]
mod tests {
    #[test]
    fn test_one() {
        assert_eq!(1, 1);
    }

    #[test]
    fn test_two() {
        assert_eq!(2, 2);
    }

    #[test]
    fn test_three() {
        assert_eq!(3, 3);
    }
}
"#
    ).unwrap();

    // Run only test_two
    let trace_manager = TraceManager::new(workspace_dir);
    let result = trace_manager
        .run_cargo_test_with_args(workspace_dir, &["--", "test_two"])
        .unwrap();

    // Should succeed
    assert_eq!(result.exit_code, 0);
    assert!(result.trace_path.exists());

    // Verify output mentions test_two
    let content = std::fs::read_to_string(&result.trace_path).unwrap();
    assert!(content.contains("test_two"));
}
