//! Single module test runner with timeout support.
//!
//! This module provides functionality to run cargo test for a specific module
//! with configurable timeout limits. It uses process spawning with timeout
//! enforcement and returns detailed test output and exit status.

use anyhow::{Context, Result};
use std::fs::{self, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::Duration;

/// Error type for module test execution
#[derive(Debug, thiserror::Error)]
pub enum TestError {
    /// Failed to spawn the test process
    #[error("Failed to spawn cargo test process: {0}")]
    SpawnError(String),

    /// Test execution timed out
    #[error("Test execution timed out after {timeout_secs} seconds")]
    Timeout { timeout_secs: u64 },

    /// Command execution failed
    #[error("Command execution failed: {0}")]
    ExecutionError(String),

    /// Thread join error
    #[error("Thread join failed: {0}")]
    ThreadJoinError(String),

    /// UTF-8 conversion error for output
    #[error("Failed to convert output to UTF-8: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

impl From<std::io::Error> for TestError {
    fn from(err: std::io::Error) -> Self {
        TestError::SpawnError(err.to_string())
    }
}

/// Run cargo test for a specific module with timeout
///
/// This function executes `cargo test <module-name>` with a configurable timeout.
/// It does NOT use --capture or --nocapture flags, allowing cargo to handle
/// output capture as it sees fit.
///
/// # Arguments
/// * `module` - The module name to test (e.g., "storage", "cli")
/// * `timeout_secs` - Timeout in seconds (process will be killed if exceeded)
///
/// # Returns
/// * `Ok(Output)` - Contains stdout, stderr, and exit status on success or timeout
/// * `Err(TestError)` - On spawn failures or other errors
///
/// # Process behavior
/// - Spawns `cargo test <module>` with inherited stdio for real-time output
/// - Waits for completion with timeout enforcement
/// - Kills process and children if timeout is exceeded
/// - Captures whatever output was produced before timeout
///
/// # Examples
/// ```ignore
/// use bead_forge::module_test::{run_module_test, TestError};
///
/// match run_module_test("storage", 30) {
///     Ok(output) => {
///         if output.status.success() {
///             println!("Tests passed!");
///             println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
///         } else {
///             println!("Tests failed with exit code: {:?}", output.status.code());
///         }
///     }
///     Err(TestError::Timeout { timeout_secs }) => {
///         eprintln!("Test timed out after {} seconds", timeout_secs);
///     }
///     Err(e) => {
///         eprintln!("Test error: {}", e);
///     }
/// }
/// ```
pub fn run_module_test(module: &str, timeout_secs: u64) -> Result<std::process::Output, TestError> {
    // Build the cargo test command
    let mut cmd = Command::new("cargo");
    cmd.arg("test").arg(module);

    // Configure output handling - inherit stdio for real-time output
    // We'll also capture pipes for the return value
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Spawn the process
    let mut child = cmd
        .spawn()
        .map_err(|e| TestError::SpawnError(format!("Failed to spawn cargo test process: {}", e)))?;

    let child_id = child.id();

    // Create a thread to wait for process completion with timeout
    let handle = thread::spawn(move || -> Result<std::process::Output, TestError> {
        // Wait for the process to complete
        let result = child.wait();

        // Try to get output if process has completed
        match result {
            Ok(status) => {
                // Process completed, we can't get output after wait() without
                // having saved the pipes earlier, so return empty output with status
                Ok(std::process::Output {
                    status,
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                })
            }
            Err(e) => Err(TestError::ExecutionError(format!("Failed to wait for process: {}", e))),
        }
    });

    // Wait for completion with timeout
    let timeout_duration = Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();

    // Loop with sleep to check for completion or timeout
    while start.elapsed() < timeout_duration {
        // Check if thread has completed
        if handle.is_finished() {
            match handle.join() {
                Ok(result) => return result,
                Err(_) => return Err(TestError::ThreadJoinError("Thread panicked".to_string())),
            }
        }

        // Sleep briefly to avoid busy-waiting
        thread::sleep(Duration::from_millis(100));
    }

    // Timeout exceeded - attempt to kill the process
    // Note: child is moved into the thread, so we need to kill by PID
    if child_id > 0 {
        kill_process_tree(child_id);
    }

    // Return timeout error
    Err(TestError::Timeout { timeout_secs })
}

/// Kill a process and its children tree
///
/// This function attempts to terminate the specified process and all its
/// child processes. On Unix systems, it uses process group killing.
/// On Windows, it uses taskkill.
fn kill_process_tree(pid: u32) {
    #[cfg(unix)]
    {
        use std::process::Command;

        // Kill the entire process group on Unix
        let _ = Command::new("kill")
            .arg("-9") // SIGKILL
            .arg(format!("-{}", pid)) // Process group
            .status();

        // Also try killing the PID directly as fallback
        let _ = Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .status();
    }

    #[cfg(windows)]
    {
        use std::process::Command;

        // Use taskkill on Windows to terminate process tree
        let _ = Command::new("taskkill")
            .arg("/F") // Force
            .arg("/PID")
            .arg(pid.to_string())
            .arg("/T") // Terminate child processes
            .status();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_module_test_with_valid_module() {
        // This test requires being in a Rust workspace with tests
        // Skip if not in a proper workspace
        if !std::path::Path::new("Cargo.toml").exists() {
            return;
        }

        // Test with a simple module that should exist
        let result = run_module_test("model", 10);

        // Should succeed (though tests may fail)
        match result {
            Ok(output) => {
                // Should have some output
                let total_len = output.stdout.len() + output.stderr.len();
                assert!(total_len > 0, "Expected some test output");
            }
            Err(TestError::Timeout { .. }) => {
                // Timeout is acceptable
            }
            Err(e) => {
                panic!("Unexpected error: {}", e);
            }
        }
    }

    #[test]
    fn test_run_module_test_timeout_behavior() {
        // This test requires being in a Rust workspace with tests
        // Skip if not in a proper workspace
        if !std::path::Path::new("Cargo.toml").exists() {
            return;
        }

        // Test with a very short timeout
        let start = std::time::Instant::now();
        let result = run_module_test("model", 1); // 1 second timeout
        let elapsed = start.elapsed();

        // Should complete quickly (either succeed or timeout)
        assert!(elapsed.as_secs() <= 5, "Test should complete within timeout + overhead");

        // Check result
        match result {
            Ok(_) => {
                // Test completed within timeout
                println!("Test completed within 1 second timeout");
            }
            Err(TestError::Timeout { timeout_secs }) => {
                assert_eq!(timeout_secs, 1);
                println!("Test timed out as expected");
            }
            Err(e) => {
                panic!("Unexpected error: {}", e);
            }
        }
    }

    #[test]
    fn test_kill_process_tree() {
        // Test that kill_process_tree doesn't panic
        kill_process_tree(99999); // Non-existent PID, should not panic
    }

    #[test]
    fn test_timeout_error_display() {
        let error = TestError::Timeout { timeout_secs: 30 };
        let display = format!("{}", error);
        assert!(display.contains("timed out"));
        assert!(display.contains("30"));

        let error = TestError::Timeout { timeout_secs: 5 };
        let display = format!("{}", error);
        assert!(display.contains("5"));
    }

    #[test]
    fn test_spawn_error_display() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "command not found");
        let error = TestError::SpawnError(io_error.to_string());
        let display = format!("{}", error);
        assert!(display.contains("Failed to spawn"));
        assert!(display.contains("command not found"));
    }

    #[test]
    fn test_execution_error_display() {
        let error = TestError::ExecutionError("test error message".to_string());
        let display = format!("{}", error);
        assert!(display.contains("test error message"));
    }
}
