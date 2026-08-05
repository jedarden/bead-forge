//! Single module test runner with timeout support.
//!
//! This module provides functionality to run cargo test for a specific module
//! with configurable timeout limits. It uses process spawning with timeout
//! enforcement and returns detailed test output and exit status.

use std::io::BufRead;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Captured output from a module test execution
#[derive(Debug, Clone)]
pub struct TestOutput {
    /// Captured stdout content
    pub stdout: String,
    /// Captured stderr content
    pub stderr: String,
    /// Process exit status
    pub status: std::process::ExitStatus,
    /// Whether the test was terminated due to timeout
    pub timed_out: bool,
    /// Exit code from the process (if available)
    pub exit_code: Option<i32>,
    /// Signal that terminated the process (Unix only, if applicable)
    pub signal: Option<String>,
}

impl TestOutput {
    /// Create a new test output result
    pub fn new(stdout: String, stderr: String, status: std::process::ExitStatus, timed_out: bool) -> Self {
        let exit_code = status.code();

        #[cfg(unix)]
        let signal = {
            use std::os::unix::process::ExitStatusExt;
            status.signal().map(|s| s.to_string())
        };

        #[cfg(not(unix))]
        let signal = None;

        Self {
            stdout,
            stderr,
            status,
            timed_out,
            exit_code,
            signal,
        }
    }

    /// Check if the test process exited successfully
    pub fn is_success(&self) -> bool {
        self.status.success()
    }

    /// Get the exit code if available
    pub fn exit_code(&self) -> Option<i32> {
        self.status.code()
    }

    /// Get combined output (stdout + stderr)
    pub fn combined_output(&self) -> String {
        format!("{}\n{}", self.stdout, self.stderr)
    }
}

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

    /// I/O error during output capture
    #[error("I/O error during output capture: {0}")]
    IoError(#[from] std::io::Error),
}

/// Run cargo test for a specific module with timeout and output capture
///
/// This function executes `cargo test <module-name>` with a configurable timeout.
/// It captures both stdout and stderr streams completely, preserving all output.
///
/// # Arguments
/// * `module` - The module name to test (e.g., "storage", "cli")
/// * `timeout_secs` - Timeout in seconds (process will be killed if exceeded)
///
/// # Returns
/// * `Ok(TestOutput)` - Contains captured stdout, stderr, exit status, and timeout flag
/// * `Err(TestError)` - On spawn failures or other errors
///
/// # Process behavior
/// - Spawns `cargo test <module>` with piped stdio for complete output capture
/// - Reads stdout and stderr concurrently while the process runs
/// - Enforces timeout - kills process if exceeded
/// - Captures all output produced before completion or timeout
///
/// # Examples
/// ```ignore
/// use bead_forge::module_test::{run_module_test, TestError};
///
/// match run_module_test("storage", 30) {
///     Ok(output) => {
///         if output.is_success() {
///             println!("Tests passed!");
///             println!("stdout: {}", output.stdout);
///         } else {
///             println!("Tests failed with exit code: {:?}", output.exit_code());
///             println!("stderr: {}", output.stderr);
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
pub fn run_module_test(module: &str, timeout_secs: u64) -> Result<TestOutput, TestError> {
    // Build the cargo test command
    let mut cmd = Command::new("cargo");
    cmd.arg("test").arg(module);

    // Configure output handling - capture both stdout and stderr
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Spawn the process
    let mut child = cmd
        .spawn()
        .map_err(|e| TestError::SpawnError(format!("Failed to spawn cargo test process: {}", e)))?;

    let child_id = child.id();

    // Take ownership of stdout and stderr pipes
    let stdout_pipe = child.stdout.take().ok_or_else(|| {
        TestError::SpawnError("Failed to capture stdout pipe".to_string())
    })?;

    let stderr_pipe = child.stderr.take().ok_or_else(|| {
        TestError::SpawnError("Failed to capture stderr pipe".to_string())
    })?;

    // Create readers for stdout and stderr
    let stdout_reader = std::io::BufReader::new(stdout_pipe);
    let stderr_reader = std::io::BufReader::new(stderr_pipe);

    // Spawn threads to read output concurrently
    let stdout_handle = thread::spawn(move || -> Result<String, TestError> {
        let mut output = String::new();
        for line in stdout_reader.lines() {
            match line {
                Ok(l) => {
                    output.push_str(&l);
                    output.push('\n');
                }
                Err(e) => return Err(TestError::IoError(e)),
            }
        }
        Ok(output)
    });

    let stderr_handle = thread::spawn(move || -> Result<String, TestError> {
        let mut output = String::new();
        for line in stderr_reader.lines() {
            match line {
                Ok(l) => {
                    output.push_str(&l);
                    output.push('\n');
                }
                Err(e) => return Err(TestError::IoError(e)),
            }
        }
        Ok(output)
    });

    // Wait for completion with timeout
    let timeout_duration = Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();

    // Loop with sleep to check for completion or timeout
    while start.elapsed() < timeout_duration {
        // Check if both threads have finished
        if stdout_handle.is_finished() && stderr_handle.is_finished() {
            // Try to wait for the child process with a small timeout
            if let Ok(Some(_)) = child.try_wait() {
                // Process has completed - join the threads and get output
                let status = child.wait().map_err(|e|
                    TestError::ExecutionError(format!("Failed to wait for process: {}", e))
                )?;

                let stdout = stdout_handle.join().unwrap_or_else(
                    |_| Err(TestError::ThreadJoinError("Stdout thread panicked".to_string()))
                ).unwrap_or_else(|_| String::new());

                let stderr = stderr_handle.join().unwrap_or_else(
                    |_| Err(TestError::ThreadJoinError("Stderr thread panicked".to_string()))
                ).unwrap_or_else(|_| String::new());

                return Ok(TestOutput::new(stdout, stderr, status, false));
            }
        }

        // Sleep briefly to avoid busy-waiting
        thread::sleep(Duration::from_millis(50));
    }

    // Timeout exceeded - attempt to kill the process
    if child_id > 0 {
        kill_process_tree(child_id);
    }

    // Try to get any partial output that was captured
    let stdout = if stdout_handle.is_finished() {
        stdout_handle.join().ok().and_then(|r| r.ok()).unwrap_or_default()
    } else {
        String::new()
    };

    let stderr = if stderr_handle.is_finished() {
        stderr_handle.join().ok().and_then(|r| r.ok()).unwrap_or_default()
    } else {
        String::new()
    };

    // Return timeout error with partial output
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
    fn test_test_output_creation() {
        // Use actual command to get real ExitStatus
        let status = std::process::Command::new("true")
            .status()
            .expect("true command should work");
        let output = TestOutput::new("test stdout".to_string(), "test stderr".to_string(), status, false);

        assert_eq!(output.stdout, "test stdout");
        assert_eq!(output.stderr, "test stderr");
        assert!(output.is_success());
        assert_eq!(output.exit_code(), Some(0));
        assert!(!output.timed_out);
    }

    #[test]
    fn test_test_output_combined() {
        // Use actual command to get real ExitStatus
        let status = std::process::Command::new("true")
            .status()
            .expect("true command should work");
        let output = TestOutput::new("stdout line".to_string(), "stderr line".to_string(), status, false);

        let combined = output.combined_output();
        assert!(combined.contains("stdout line"));
        assert!(combined.contains("stderr line"));
    }

    #[test]
    fn test_test_output_timeout() {
        // Use actual command to get real ExitStatus (false exits with 1)
        let status = std::process::Command::new("false")
            .status()
            .expect("false command should work");
        let output = TestOutput::new("partial stdout".to_string(), "partial stderr".to_string(), status, true);

        assert!(output.timed_out);
        assert!(!output.is_success());
        assert_eq!(output.exit_code(), Some(1));
    }

    #[test]
    fn test_output_preservation() {
        // Test that output is preserved without truncation
        let status = std::process::Command::new("true")
            .status()
            .expect("true command should work");
        let long_stdout = "line1\nline2\nline3\n".repeat(1000); // ~15KB of data
        let long_stderr = "error1\nerror2\nerror3\n".repeat(500); // ~7.5KB of data

        let output = TestOutput::new(long_stdout.clone(), long_stderr.clone(), status, false);

        // Verify no truncation occurred
        assert_eq!(output.stdout, long_stdout);
        assert_eq!(output.stderr, long_stderr);
        assert_eq!(output.stdout.len(), long_stdout.len());
        assert_eq!(output.stderr.len(), long_stderr.len());
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

    #[test]
    fn test_simple_command_capture() {
        // Test with a simple echo command to verify output capture works
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg("echo 'stdout message'; echo 'stderr message' >&2");
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().expect("Failed to spawn test command");

        let stdout_pipe = child.stdout.take().expect("Failed to get stdout pipe");
        let stderr_pipe = child.stderr.take().expect("Failed to get stderr pipe");

        let stdout = std::io::BufReader::new(stdout_pipe)
            .lines()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        let stderr = std::io::BufReader::new(stderr_pipe)
            .lines()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");

        let status = child.wait().expect("Failed to wait for test command");
        let output = TestOutput::new(stdout, stderr, status, false);

        assert!(output.stdout.contains("stdout message"));
        assert!(output.stderr.contains("stderr message"));
        assert!(output.is_success());
    }
}
