//! Subprocess execution infrastructure with stdout/stderr capture.
//!
//! This module provides general-purpose command execution with separate
//! stdout and stderr capture, exit code preservation, and comprehensive
//! error handling for graceful failure recovery.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

/// Configuration for subprocess execution
#[derive(Debug, Clone)]
pub struct SubprocessConfig {
    /// Working directory for the command (default: current directory)
    pub working_dir: Option<PathBuf>,
    /// Environment variables to set (default: inherit parent process)
    pub env: Vec<(String, String)>,
    /// Timeout in seconds (None = no timeout)
    pub timeout_seconds: Option<u64>,
    /// Whether to capture stdout (default: true)
    pub capture_stdout: bool,
    /// Whether to capture stderr (default: true)
    pub capture_stderr: bool,
}

impl Default for SubprocessConfig {
    fn default() -> Self {
        Self {
            working_dir: None,
            env: Vec::new(),
            timeout_seconds: None,
            capture_stdout: true,
            capture_stderr: true,
        }
    }
}

impl SubprocessConfig {
    /// Create a new subprocess configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the working directory for the command
    pub fn working_dir<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.working_dir = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Add an environment variable
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Set timeout in seconds
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = Some(seconds);
        self
    }

    /// Configure stdout capture
    pub fn capture_stdout(mut self, capture: bool) -> Self {
        self.capture_stdout = capture;
        self
    }

    /// Configure stderr capture
    pub fn capture_stderr(mut self, capture: bool) -> Self {
        self.capture_stderr = capture;
        self
    }
}

/// Result from executing a subprocess command
#[derive(Debug)]
pub struct SubprocessResult {
    /// Exit code from the command (0 = success, non-zero = failure)
    pub exit_code: i32,
    /// Whether the command terminated successfully
    pub success: bool,
    /// Captured stdout content (if capture_stdout was enabled)
    pub stdout: String,
    /// Captured stderr content (if capture_stderr was enabled)
    pub stderr: String,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Whether the command timed out
    pub timed_out: bool,
    /// The command that was executed
    pub command: String,
    /// The arguments that were passed
    pub args: Vec<String>,
}

impl SubprocessResult {
    /// Check if the command exited with code 0
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// Get a human-readable summary of the result
    pub fn summary(&self) -> String {
        let status = if self.success { "SUCCESS" } else { "FAILURE" };
        format!(
            "Command '{}' {} (exit code: {}, duration: {}ms)",
            self.command, status, self.exit_code, self.duration_ms
        )
    }

    /// Get the stdout lines as a vector
    pub fn stdout_lines(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }

    /// Get the stderr lines as a vector
    pub fn stderr_lines(&self) -> Vec<&str> {
        self.stderr.lines().collect()
    }

    /// Check if stderr contains a specific pattern
    pub fn stderr_contains(&self, pattern: &str) -> bool {
        self.stderr.contains(pattern)
    }

    /// Check if stdout contains a specific pattern
    pub fn stdout_contains(&self, pattern: &str) -> bool {
        self.stdout.contains(pattern)
    }
}

/// Execute a command with stdout/stderr capture
///
/// This function runs the specified command and captures its output according
/// to the provided configuration. It provides comprehensive error handling
/// and timing information.
///
/// # Arguments
/// * `command` - The command to execute (e.g., "cargo", "git", "echo")
/// * `args` - Command arguments
/// * `config` - Execution configuration (use SubprocessConfig::default() for defaults)
///
/// # Returns
/// * `Result<SubprocessResult>` containing exit code, captured output, and timing
///
/// # Examples
/// ```ignore
/// use bead_forge::subprocess::{execute_command, SubprocessConfig};
///
/// // Simple command execution
/// let result = execute_command("echo", &["hello", "world"], SubprocessConfig::default())?;
/// assert_eq!(result.stdout.trim(), "hello world");
/// assert!(result.is_success());
///
/// // Command with working directory
/// let config = SubprocessConfig::new()
///     .working_dir("/tmp")
///     .capture_stderr(true);
/// let result = execute_command("pwd", &[], config)?;
///
/// // Command that fails
/// let result = execute_command("false", &[], SubprocessConfig::default())?;
/// assert!(!result.is_success());
/// assert_eq!(result.exit_code, 1);
/// ```
pub fn execute_command(
    command: &str,
    args: &[&str],
    config: SubprocessConfig,
) -> Result<SubprocessResult> {
    // Record start time
    let start = Instant::now();

    // Build the command
    let mut cmd = Command::new(command);
    cmd.args(args);

    // Configure stdout/stderr capture
    if config.capture_stdout {
        cmd.stdout(Stdio::piped());
    } else {
        cmd.stdout(Stdio::inherit());
    }

    if config.capture_stderr {
        cmd.stderr(Stdio::piped());
    } else {
        cmd.stderr(Stdio::inherit());
    }

    // Set working directory if specified
    if let Some(ref dir) = config.working_dir {
        cmd.current_dir(dir);
    }

    // Set environment variables if specified
    for (key, value) in &config.env {
        cmd.env(key, value);
    }

    // Execute the command
    let output = cmd
        .output()
        .with_context(|| format!("Failed to execute command: {} {}", command, args.join(" ")))?;

    // Calculate duration
    let duration_ms = start.elapsed().as_millis() as u64;
    let exit_code = output.status.code().unwrap_or(-1);
    let success = output.status.success();

    // Convert output to strings
    let stdout = if config.capture_stdout {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        String::new()
    };

    let stderr = if config.capture_stderr {
        String::from_utf8_lossy(&output.stderr).to_string()
    } else {
        String::new()
    };

    Ok(SubprocessResult {
        exit_code,
        success,
        stdout,
        stderr,
        duration_ms,
        timed_out: false, // TODO: Implement timeout support
        command: command.to_string(),
        args: args.iter().map(|s| s.to_string()).collect(),
    })
}

/// Execute a command and write output to trace files
///
/// This function executes a command and writes the captured output to
/// separate stdout.txt and stderr.txt files in the specified directory.
/// It also creates a metadata.json file with execution information.
///
/// # Arguments
/// * `command` - The command to execute
/// * `args` - Command arguments
/// * `config` - Execution configuration
/// * `trace_dir` - Directory to write trace files (will be created if needed)
///
/// # Returns
/// * `Result<SubprocessResult>` containing execution results
///
/// # Examples
/// ```ignore
/// use bead_forge::subprocess::{execute_command_to_trace, SubprocessConfig};
/// use std::path::Path;
///
/// let result = execute_command_to_trace(
///     "cargo",
///     &["test"],
///     SubprocessConfig::default(),
///     Path::new("/tmp/my-trace")
/// )?;
///
/// // Output files created:
/// // /tmp/my-trace/stdout.txt
/// // /tmp/my-trace/stderr.txt
/// // /tmp/my-trace/metadata.json
/// ```
pub fn execute_command_to_trace(
    command: &str,
    args: &[&str],
    config: SubprocessConfig,
    trace_dir: &Path,
) -> Result<SubprocessResult> {
    use chrono::Utc;
    use serde_json;
    use std::fs;

    // Ensure the trace directory exists
    if !trace_dir.exists() {
        fs::create_dir_all(trace_dir).with_context(|| {
            format!("Failed to create trace directory: {}", trace_dir.display())
        })?;
    }

    // Record start time
    let start_time = Utc::now().to_rfc3339();

    // Execute the command
    let result = execute_command(command, args, config)?;

    let end_time = Utc::now().to_rfc3339();

    // Write stdout to file
    let stdout_path = trace_dir.join("stdout.txt");
    fs::write(&stdout_path, &result.stdout)
        .with_context(|| format!("Failed to write stdout file: {}", stdout_path.display()))?;

    // Write stderr to file
    let stderr_path = trace_dir.join("stderr.txt");
    fs::write(&stderr_path, &result.stderr)
        .with_context(|| format!("Failed to write stderr file: {}", stderr_path.display()))?;

    // Create and write metadata
    let metadata = serde_json::json!({
        "command": result.command,
        "args": result.args,
        "exit_code": result.exit_code,
        "success": result.success,
        "duration_ms": result.duration_ms,
        "start_time": start_time,
        "end_time": end_time,
        "timed_out": result.timed_out,
        "captured_at": Utc::now().to_rfc3339(),
    });

    let metadata_path = trace_dir.join("metadata.json");
    fs::write(&metadata_path, serde_json::to_string_pretty(&metadata)?)
        .with_context(|| format!("Failed to write metadata file: {}", metadata_path.display()))?;

    Ok(result)
}

/// Execute a command with streaming output capture
///
/// This function provides a callback-based interface for processing command
/// output line-by-line as it is generated, rather than waiting for completion.
/// This is useful for long-running commands or when real-time output is needed.
///
/// # Arguments
/// * `command` - The command to execute
/// * `args` - Command arguments
/// * `config` - Execution configuration
/// * `on_stdout_line` - Callback for each stdout line (can be None)
/// * `on_stderr_line` - Callback for each stderr line (can be None)
///
/// # Returns
/// * `Result<SubprocessResult>` containing execution results
///
/// # Examples
/// ```ignore
/// use bead_forge::subprocess::{execute_command_streaming, SubprocessConfig};
///
/// let result = execute_command_streaming(
///     "find",
///     &["/tmp", "-name", "*.log"],
///     SubprocessConfig::default(),
///     Some(&|line| println!("FOUND: {}", line)),
///     Some(&|line| eprintln!("ERROR: {}", line)),
/// )?;
/// ```
pub fn execute_command_streaming(
    command: &str,
    args: &[&str],
    config: SubprocessConfig,
    on_stdout_line: Option<&dyn Fn(&str)>,
    on_stderr_line: Option<&dyn Fn(&str)>,
) -> Result<SubprocessResult> {
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Stdio};

    // Record start time
    let start = Instant::now();

    // Build the command
    let mut cmd = Command::new(command);
    cmd.args(args);

    // Configure stdout/stderr
    let stdout_pipe = if config.capture_stdout {
        Stdio::piped()
    } else {
        Stdio::inherit()
    };

    let stderr_pipe = if config.capture_stderr {
        Stdio::piped()
    } else {
        Stdio::inherit()
    };

    cmd.stdout(stdout_pipe);
    cmd.stderr(stderr_pipe);

    // Set working directory if specified
    if let Some(ref dir) = config.working_dir {
        cmd.current_dir(dir);
    }

    // Set environment variables if specified
    for (key, value) in &config.env {
        cmd.env(key, value);
    }

    // Spawn the command
    let mut child: Child = cmd
        .spawn()
        .with_context(|| format!("Failed to spawn command: {} {}", command, args.join(" ")))?;

    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();

    // Read stdout line by line
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    if let Some(callback) = on_stdout_line {
                        callback(&l);
                    }
                    stdout_lines.push(l);
                }
                Err(e) => {
                    eprintln!("Error reading stdout: {}", e);
                    break;
                }
            }
        }
    }

    // Read stderr line by line
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    if let Some(callback) = on_stderr_line {
                        callback(&l);
                    }
                    stderr_lines.push(l);
                }
                Err(e) => {
                    eprintln!("Error reading stderr: {}", e);
                    break;
                }
            }
        }
    }

    // Wait for command to complete
    let status = child
        .wait()
        .with_context(|| format!("Failed to wait for command: {} {}", command, args.join(" ")))?;

    // Calculate duration
    let duration_ms = start.elapsed().as_millis() as u64;
    let exit_code = status.code().unwrap_or(-1);
    let success = status.success();

    let stdout = stdout_lines.join("\n");
    let stderr = stderr_lines.join("\n");

    Ok(SubprocessResult {
        exit_code,
        success,
        stdout,
        stderr,
        duration_ms,
        timed_out: false,
        command: command.to_string(),
        args: args.iter().map(|s| s.to_string()).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_subprocess_config_default() {
        let config = SubprocessConfig::default();
        assert!(config.working_dir.is_none());
        assert!(config.env.is_empty());
        assert!(config.timeout_seconds.is_none());
        assert!(config.capture_stdout);
        assert!(config.capture_stderr);
    }

    #[test]
    fn test_subprocess_config_builder() {
        let config = SubprocessConfig::new()
            .working_dir("/tmp")
            .env("TEST_VAR", "test_value")
            .timeout(30)
            .capture_stdout(true)
            .capture_stderr(true);

        assert_eq!(config.working_dir, Some(PathBuf::from("/tmp")));
        assert_eq!(
            config.env,
            vec![("TEST_VAR".to_string(), "test_value".to_string())]
        );
        assert_eq!(config.timeout_seconds, Some(30));
        assert!(config.capture_stdout);
        assert!(config.capture_stderr);
    }

    #[test]
    fn test_execute_simple_command() {
        let result =
            execute_command("echo", &["hello", "world"], SubprocessConfig::default()).unwrap();

        assert!(result.is_success());
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout.trim(), "hello world");
        assert!(result.stderr.is_empty());
        assert_eq!(result.command, "echo");
        assert_eq!(result.args, vec!["hello", "world"]);
    }

    #[test]
    fn test_execute_failing_command() {
        let result = execute_command("false", &[], SubprocessConfig::default()).unwrap();

        assert!(!result.is_success());
        assert_eq!(result.exit_code, 1);
        assert!(result.stdout.is_empty());
        assert!(result.stderr.is_empty());
    }

    #[test]
    fn test_execute_command_with_stderr() {
        let result = execute_command(
            "sh",
            &["-c", "echo stdout; echo stderr >&2"],
            SubprocessConfig::default(),
        )
        .unwrap();

        assert!(result.is_success());
        assert!(result.stdout.contains("stdout"));
        assert!(result.stderr.contains("stderr"));
    }

    #[test]
    fn test_execute_command_with_working_dir() {
        let temp_dir = TempDir::new().unwrap();
        let config = SubprocessConfig::new().working_dir(temp_dir.path());

        let result = execute_command("pwd", &[], config).unwrap();

        assert!(result.is_success());
        assert!(result.stdout.contains(temp_dir.path().to_str().unwrap()));
    }

    #[test]
    fn test_execute_command_with_env() {
        let config = SubprocessConfig::new().env("TEST_VAR", "test_value");

        let result = execute_command("sh", &["-c", "echo $TEST_VAR"], config).unwrap();

        assert!(result.is_success());
        assert!(result.stdout.contains("test_value"));
    }

    #[test]
    fn test_subprocess_result_is_success() {
        let success_result = execute_command("true", &[], SubprocessConfig::default()).unwrap();
        assert!(success_result.is_success());

        let failure_result = execute_command("false", &[], SubprocessConfig::default()).unwrap();
        assert!(!failure_result.is_success());
    }

    #[test]
    fn test_subprocess_result_summary() {
        let result = execute_command("echo", &["test"], SubprocessConfig::default()).unwrap();
        let summary = result.summary();

        assert!(summary.contains("echo"));
        assert!(summary.contains("SUCCESS"));
        assert!(summary.contains("exit code: 0"));
    }

    #[test]
    fn test_subprocess_result_stdout_lines() {
        let result = execute_command(
            "echo",
            &["line1\nline2\nline3"],
            SubprocessConfig::default(),
        )
        .unwrap();
        let lines = result.stdout_lines();

        assert!(lines.len() >= 1);
        assert!(lines[0].contains("line1"));
    }

    #[test]
    fn test_subprocess_result_stderr_contains() {
        let result = execute_command(
            "sh",
            &["-c", "echo error message >&2"],
            SubprocessConfig::default(),
        )
        .unwrap();

        assert!(result.stderr_contains("error message"));
        assert!(!result.stdout_contains("error message"));
    }

    #[test]
    fn test_execute_command_to_trace() {
        let temp_dir = TempDir::new().unwrap();
        let trace_dir = temp_dir.path().join("trace");

        let result = execute_command_to_trace(
            "echo",
            &["hello from trace"],
            SubprocessConfig::default(),
            &trace_dir,
        )
        .unwrap();

        // Verify result is correct
        assert!(result.is_success());
        assert!(result.stdout.contains("hello from trace"));

        // Verify trace files exist
        assert!(trace_dir.exists());
        assert!(trace_dir.join("stdout.txt").exists());
        assert!(trace_dir.join("stderr.txt").exists());
        assert!(trace_dir.join("metadata.json").exists());

        // Verify file contents
        use std::fs;
        let stdout_content = fs::read_to_string(trace_dir.join("stdout.txt")).unwrap();
        assert!(stdout_content.contains("hello from trace"));

        let metadata_content = fs::read_to_string(trace_dir.join("metadata.json")).unwrap();
        assert!(metadata_content.contains("echo"));
        assert!(metadata_content.contains("exit_code"));
    }

    #[test]
    fn test_execute_command_streaming() {
        use std::sync::{Arc, Mutex};

        let stdout_received = Arc::new(Mutex::new(Vec::new()));
        let stderr_received = Arc::new(Mutex::new(Vec::new()));

        let stdout_clone = stdout_received.clone();
        let stderr_clone = stderr_received.clone();

        let result = execute_command_streaming(
            "sh",
            &["-c", "echo line1; echo line2; echo error >&2"],
            SubprocessConfig::default(),
            Some(&|line| {
                stdout_clone.lock().unwrap().push(line.to_string());
            }),
            Some(&|line| {
                stderr_clone.lock().unwrap().push(line.to_string());
            }),
        )
        .unwrap();

        assert!(result.is_success());
        assert!(stdout_received.lock().unwrap().len() >= 2);
        assert!(stderr_received.lock().unwrap().len() >= 1);
    }

    #[test]
    fn test_execute_command_no_capture() {
        let config = SubprocessConfig::new()
            .capture_stdout(false)
            .capture_stderr(false);

        let result = execute_command("echo", &["test"], config).unwrap();

        assert!(result.is_success());
        assert!(result.stdout.is_empty()); // No capture
        assert!(result.stderr.is_empty()); // No capture
    }

    #[test]
    fn test_execute_command_with_args_containing_spaces() {
        let result = execute_command(
            "echo",
            &["hello world", "foo bar"],
            SubprocessConfig::default(),
        )
        .unwrap();

        assert!(result.is_success());
        assert!(result.stdout.contains("hello world"));
        assert!(result.stdout.contains("foo bar"));
    }

    #[test]
    fn test_execute_command_nonexistent() {
        let result = execute_command("nonexistent-command-xyz", &[], SubprocessConfig::default());

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Failed to execute command")
                || err.to_string().contains("No such file or directory")
                || err.to_string().contains("not found")
        );
    }
}
