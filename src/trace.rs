//! Trace file infrastructure for capturing and storing test/execution output.
//!
//! Provides structured trace file management with proper error handling for
//! directory creation and file writing operations.

use anyhow::{Context, Result};
use chrono::Utc;
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// Trace file metadata structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TraceMetadata {
    /// Bead ID (if applicable)
    pub bead_id: Option<String>,
    /// Agent that created the trace
    pub agent: String,
    /// Provider/model information
    pub provider: Option<String>,
    pub model: Option<String>,
    /// Exit code (if applicable)
    pub exit_code: Option<i32>,
    /// Outcome status
    pub outcome: String,
    /// Duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Token usage (if applicable)
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cost_usd: Option<f64>,
    /// When the trace was captured
    pub captured_at: String,
    /// Trace format
    pub trace_format: String,
    /// Whether trace was pruned
    pub pruned: bool,
    /// Template version (if applicable)
    pub template_version: Option<String>,
}

impl Default for TraceMetadata {
    fn default() -> Self {
        Self {
            bead_id: None,
            agent: "unknown".to_string(),
            provider: None,
            model: None,
            exit_code: None,
            outcome: "unknown".to_string(),
            duration_ms: None,
            input_tokens: None,
            output_tokens: None,
            cost_usd: None,
            captured_at: Utc::now().to_rfc3339(),
            trace_format: "claude_json".to_string(),
            pruned: false,
            template_version: None,
        }
    }
}

/// Trace file manager
pub struct TraceManager {
    /// Base directory for trace files
    traces_dir: PathBuf,
}

impl TraceManager {
    /// Create a new TraceManager with the specified base directory
    pub fn new(base_dir: &Path) -> Self {
        Self {
            traces_dir: base_dir.join(".beads").join("traces"),
        }
    }

    /// Create a new TraceManager for the current workspace
    pub fn for_current_workspace() -> Result<Self> {
        let current_dir = std::env::current_dir()
            .context("Failed to get current directory")?;
        Ok(Self::new(&current_dir))
    }

    /// Ensure the traces directory exists with proper error handling
    ///
    /// This function creates the `.beads/traces/` directory if it doesn't exist,
    /// with comprehensive error handling for permission issues, disk space, etc.
    pub fn ensure_traces_dir(&self) -> Result<()> {
        // Check if directory already exists
        if self.traces_dir.exists() {
            if !self.traces_dir.is_dir() {
                anyhow::bail!(
                    "Trace path exists but is not a directory: {}",
                    self.traces_dir.display()
                );
            }
            return Ok(());
        }

        // Create parent directory if needed
        if let Some(parent) = self.traces_dir.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .with_context(|| {
                        format!(
                            "Failed to create parent directory: {}",
                            parent.display()
                        )
                    })?;
            }
        }

        // Create the traces directory
        fs::create_dir(&self.traces_dir).with_context(|| {
            format!(
                "Failed to create traces directory: {}. \
                 Check permissions and disk space.",
                self.traces_dir.display()
            )
        })
    }

    /// Generate a timestamped filename for cargo test logs
    ///
    /// Format: `cargo-test-YYYYMMDD-HHMMSS.log`
    pub fn cargo_test_filename() -> String {
        let now = Utc::now();
        format!("cargo-test-{}.log", now.format("%Y%m%d-%H%M%S"))
    }

    /// Get the path for a cargo test trace file
    pub fn cargo_test_path(&self) -> Result<PathBuf> {
        self.ensure_traces_dir()?;
        Ok(self.traces_dir.join(Self::cargo_test_filename()))
    }

    /// Get the path for the "latest" cargo test symlink
    pub fn cargo_test_latest_path(&self) -> PathBuf {
        self.traces_dir.join("cargo-test-latest.log")
    }

    /// Create a bead-specific trace directory
    ///
    /// Format: `.beads/traces/{bead_id}/`
    pub fn bead_trace_dir(&self, bead_id: &str) -> Result<PathBuf> {
        self.ensure_traces_dir()?;

        let bead_dir = self.traces_dir.join(bead_id);

        // Create bead directory if it doesn't exist
        if !bead_dir.exists() {
            fs::create_dir(&bead_dir).with_context(|| {
                format!(
                    "Failed to create bead trace directory: {}",
                    bead_dir.display()
                )
            })?;
        }

        Ok(bead_dir)
    }

    /// Write metadata.json for a bead trace
    pub fn write_metadata(&self, bead_id: &str, metadata: &TraceMetadata) -> Result<()> {
        let bead_dir = self.bead_trace_dir(bead_id)?;
        let metadata_path = bead_dir.join("metadata.json");

        let metadata_json = serde_json::to_string_pretty(metadata)
            .context("Failed to serialize metadata to JSON")?;

        fs::write(&metadata_path, metadata_json).with_context(|| {
            format!(
                "Failed to write metadata file: {}",
                metadata_path.display()
            )
        })
    }

    /// Write stdout.txt for a bead trace
    pub fn write_stdout(&self, bead_id: &str, stdout: &str) -> Result<()> {
        let bead_dir = self.bead_trace_dir(bead_id)?;
        let stdout_path = bead_dir.join("stdout.txt");

        fs::write(&stdout_path, stdout).with_context(|| {
            format!(
                "Failed to write stdout file: {}",
                stdout_path.display()
            )
        })
    }

    /// Write stderr.txt for a bead trace
    pub fn write_stderr(&self, bead_id: &str, stderr: &str) -> Result<()> {
        let bead_dir = self.bead_trace_dir(bead_id)?;
        let stderr_path = bead_dir.join("stderr.txt");

        fs::write(&stderr_path, stderr).with_context(|| {
            format!(
                "Failed to write stderr file: {}",
                stderr_path.display()
            )
        })
    }

    /// Write a complete bead trace (metadata, stdout, stderr)
    pub fn write_bead_trace(
        &self,
        bead_id: &str,
        metadata: &TraceMetadata,
        stdout: &str,
        stderr: &str,
    ) -> Result<()> {
        self.write_metadata(bead_id, metadata)?;
        self.write_stdout(bead_id, stdout)?;
        self.write_stderr(bead_id, stderr)?;
        Ok(())
    }

    /// Write cargo test output to a timestamped trace file
    pub fn write_cargo_test_trace(&self, output: &str) -> Result<PathBuf> {
        let trace_path = self.cargo_test_path()?;

        fs::write(&trace_path, output).with_context(|| {
            format!(
                "Failed to write cargo test trace: {}",
                trace_path.display()
            )
        })?;

        // Update the "latest" symlink
        self.update_cargo_test_latest(&trace_path)?;

        Ok(trace_path)
    }

    /// Update the cargo-test-latest.log symlink
    #[cfg(unix)]
    fn update_cargo_test_latest(&self, target: &Path) -> Result<()> {
        use std::os::unix::fs::symlink;

        let latest_path = self.cargo_test_latest_path();

        // Remove existing symlink if it exists
        if latest_path.exists() || latest_path.is_symlink() {
            fs::remove_file(&latest_path).with_context(|| {
                format!(
                    "Failed to remove old symlink: {}",
                    latest_path.display()
                )
            })?;
        }

        // Create new symlink
        let target_name = target.file_name()
            .context("Failed to get target filename")?;

        symlink(target_name, &latest_path).with_context(|| {
            format!(
                "Failed to create symlink: {} -> {}",
                latest_path.display(),
                target_name.to_string_lossy()
            )
        })
    }

    /// Update the cargo-test-latest.log symlink (Windows stub)
    #[cfg(windows)]
    fn update_cargo_test_latest(&self, _target: &Path) -> Result<()> {
        // Symlinks on Windows require admin privileges; skip for now
        Ok(())
    }

    /// List all bead trace directories
    pub fn list_bead_traces(&self) -> Result<Vec<String>> {
        self.ensure_traces_dir()?;

        let mut bead_ids = Vec::new();

        for entry in fs::read_dir(&self.traces_dir)
            .with_context(|| {
                format!(
                    "Failed to read traces directory: {}",
                    self.traces_dir.display()
                )
            })?
        {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            // Only include directories that look like bead IDs (start with "bf-")
            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        if name_str.starts_with("bf-") || name_str.starts_with("needle-") {
                            bead_ids.push(name_str.to_string());
                        }
                    }
                }
            }
        }

        bead_ids.sort();
        Ok(bead_ids)
    }

    /// Check if a bead trace exists
    pub fn has_bead_trace(&self, bead_id: &str) -> bool {
        let bead_dir = self.traces_dir.join(bead_id);
        bead_dir.exists() && bead_dir.is_dir()
    }

    /// Get the path to a bead's metadata file
    pub fn bead_metadata_path(&self, bead_id: &str) -> PathBuf {
        self.traces_dir.join(bead_id).join("metadata.json")
    }

    /// Get the path to a bead's stdout file
    pub fn bead_stdout_path(&self, bead_id: &str) -> PathBuf {
        self.traces_dir.join(bead_id).join("stdout.txt")
    }

    /// Get the path to a bead's stderr file
    pub fn bead_stderr_path(&self, bead_id: &str) -> PathBuf {
        self.traces_dir.join(bead_id).join("stderr.txt")
    }

    /// Execute cargo test in the specified directory and capture all output
    ///
    /// This function runs `cargo test` in the given directory, captures both
    /// stdout and stderr, and writes the combined output to a trace file.
    ///
    /// # Arguments
    /// * `workspace_dir` - Path to the directory where cargo test should be executed
    ///
    /// # Returns
    /// * `Result<CargoTestResult>` containing exit code, duration, and trace path
    ///
    /// # Examples
    /// ```ignore
    /// let manager = TraceManager::for_current_workspace()?;
    /// let result = manager.run_cargo_test(Path::new("/home/coding/NEEDLE"))?;
    /// println!("Exit code: {}", result.exit_code);
    /// println!("Duration: {}ms", result.duration_ms);
    /// println!("Output written to: {}", result.trace_path.display());
    /// ```
    pub fn run_cargo_test(&self, workspace_dir: &Path) -> Result<CargoTestResult> {
        // Ensure the traces directory exists
        self.ensure_traces_dir()?;

        // Start timing
        let start = Instant::now();

        // Execute cargo test, capturing both stdout and stderr
        let output = Command::new("cargo")
            .arg("test")
            .current_dir(workspace_dir)
            .output()
            .with_context(|| {
                format!(
                    "Failed to execute cargo test in workspace: {}",
                    workspace_dir.display()
                )
            })?;

        // Calculate duration
        let duration_ms = start.elapsed().as_millis() as u64;

        // Combine stdout and stderr for the trace file
        let mut combined_output = String::new();
        combined_output.push_str("=== STDOUT ===\n");
        combined_output.push_str(&String::from_utf8_lossy(&output.stdout));
        combined_output.push_str("\n=== STDERR ===\n");
        combined_output.push_str(&String::from_utf8_lossy(&output.stderr));
        combined_output.push_str(&format!("\n=== EXIT CODE: {} ===\n",
            output.status.code().unwrap_or(-1)));

        // Write to trace file
        let trace_path = self.write_cargo_test_trace(&combined_output)?;

        Ok(CargoTestResult {
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms,
            trace_path,
        })
    }

    /// Execute cargo test with custom arguments
    ///
    /// This function runs `cargo test` with additional arguments, which is
    /// useful for running specific tests or with different options.
    ///
    /// # Arguments
    /// * `workspace_dir` - Path to the directory where cargo test should be executed
    /// * `args` - Additional arguments to pass to cargo test
    ///
    /// # Returns
    /// * `Result<CargoTestResult>` containing exit code, duration, and trace path
    pub fn run_cargo_test_with_args(&self, workspace_dir: &Path, args: &[&str]) -> Result<CargoTestResult> {
        // Ensure the traces directory exists
        self.ensure_traces_dir()?;

        // Start timing
        let start = Instant::now();

        // Build the command with custom arguments
        let mut cmd = Command::new("cargo");
        cmd.arg("test").args(args).current_dir(workspace_dir);

        // Execute cargo test, capturing both stdout and stderr
        let output = cmd.output().with_context(|| {
            format!(
                "Failed to execute cargo test in workspace: {}",
                workspace_dir.display()
            )
        })?;

        // Calculate duration
        let duration_ms = start.elapsed().as_millis() as u64;

        // Combine stdout and stderr for the trace file
        let mut combined_output = String::new();
        combined_output.push_str("=== STDOUT ===\n");
        combined_output.push_str(&String::from_utf8_lossy(&output.stdout));
        combined_output.push_str("\n=== STDERR ===\n");
        combined_output.push_str(&String::from_utf8_lossy(&output.stderr));
        combined_output.push_str(&format!("\n=== EXIT CODE: {} ===\n",
            output.status.code().unwrap_or(-1)));

        // Write to trace file
        let trace_path = self.write_cargo_test_trace(&combined_output)?;

        Ok(CargoTestResult {
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms,
            trace_path,
        })
    }

    /// Execute cargo test and write to a bead-specific trace directory
    ///
    /// This function runs `cargo test` in the given directory, captures both
    /// stdout and stderr, and writes the output to a bead-specific trace directory
    /// using the naming scheme from bf-177v7f (metadata.json, stdout.txt, stderr.txt).
    ///
    /// # Arguments
    /// * `workspace_dir` - Path to the directory where cargo test should be executed
    /// * `bead_id` - Bead ID for the trace directory
    /// * `metadata` - Trace metadata to record
    ///
    /// # Returns
    /// * `Result<BeadTestResult>` containing exit code, duration, and bead trace directory
    ///
    /// # Examples
    /// ```ignore
    /// let manager = TraceManager::for_current_workspace()?;
    /// let metadata = TraceMetadata {
    ///     bead_id: Some("bf-8ei6pa".to_string()),
    ///     agent: "needle-worker".to_string(),
    ///     ..Default::default()
    /// };
    /// let result = manager.run_cargo_test_to_bead_trace(
    ///     Path::new("/home/coding/NEEDLE"),
    ///     "bf-8ei6pa",
    ///     &metadata
    /// )?;
    /// println!("Exit code: {}", result.exit_code);
    /// println!("Duration: {}ms", result.duration_ms);
    /// println!("Output written to: {}", result.bead_trace_dir.display());
    /// ```
    pub fn run_cargo_test_to_bead_trace(
        &self,
        workspace_dir: &Path,
        bead_id: &str,
        metadata: &TraceMetadata,
    ) -> Result<BeadTestResult> {
        // Ensure the traces directory exists
        self.ensure_traces_dir()?;

        // Start timing
        let start = Instant::now();

        // Execute cargo test, capturing both stdout and stderr
        let output = Command::new("cargo")
            .arg("test")
            .current_dir(workspace_dir)
            .output()
            .with_context(|| {
                format!(
                    "Failed to execute cargo test in workspace: {}",
                    workspace_dir.display()
                )
            })?;

        // Calculate duration
        let duration_ms = start.elapsed().as_millis() as u64;
        let exit_code = output.status.code().unwrap_or(-1);

        // Convert output to strings
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Create updated metadata with execution results
        let mut exec_metadata = metadata.clone();
        exec_metadata.exit_code = Some(exit_code);
        exec_metadata.duration_ms = Some(duration_ms);
        exec_metadata.outcome = if exit_code == 0 {
            "success".to_string()
        } else {
            "failure".to_string()
        };
        exec_metadata.captured_at = Utc::now().to_rfc3339();

        // Write to bead trace directory
        self.write_bead_trace(bead_id, &exec_metadata, &stdout, &stderr)?;

        let bead_trace_dir = self.bead_trace_dir(bead_id)?;

        Ok(BeadTestResult {
            exit_code,
            duration_ms,
            bead_trace_dir,
            stdout,
            stderr,
        })
    }

    /// Execute cargo test with custom arguments to a bead-specific trace directory
    ///
    /// This function runs `cargo test` with additional arguments and writes
    /// the output to a bead-specific trace directory.
    ///
    /// # Arguments
    /// * `workspace_dir` - Path to the directory where cargo test should be executed
    /// * `bead_id` - Bead ID for the trace directory
    /// * `metadata` - Trace metadata to record
    /// * `args` - Additional arguments to pass to cargo test
    ///
    /// # Returns
    /// * `Result<BeadTestResult>` containing exit code, duration, and bead trace directory
    pub fn run_cargo_test_to_bead_trace_with_args(
        &self,
        workspace_dir: &Path,
        bead_id: &str,
        metadata: &TraceMetadata,
        args: &[&str],
    ) -> Result<BeadTestResult> {
        // Ensure the traces directory exists
        self.ensure_traces_dir()?;

        // Start timing
        let start = Instant::now();

        // Build the command with custom arguments
        let mut cmd = Command::new("cargo");
        cmd.arg("test").args(args).current_dir(workspace_dir);

        // Execute cargo test, capturing both stdout and stderr
        let output = cmd.output().with_context(|| {
            format!(
                "Failed to execute cargo test in workspace: {}",
                workspace_dir.display()
            )
        })?;

        // Calculate duration
        let duration_ms = start.elapsed().as_millis() as u64;
        let exit_code = output.status.code().unwrap_or(-1);

        // Convert output to strings
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Create updated metadata with execution results
        let mut exec_metadata = metadata.clone();
        exec_metadata.exit_code = Some(exit_code);
        exec_metadata.duration_ms = Some(duration_ms);
        exec_metadata.outcome = if exit_code == 0 {
            "success".to_string()
        } else {
            "failure".to_string()
        };
        exec_metadata.captured_at = Utc::now().to_rfc3339();

        // Write to bead trace directory
        self.write_bead_trace(bead_id, &exec_metadata, &stdout, &stderr)?;

        let bead_trace_dir = self.bead_trace_dir(bead_id)?;

        Ok(BeadTestResult {
            exit_code,
            duration_ms,
            bead_trace_dir,
            stdout,
            stderr,
        })
    }
}

/// Result from running cargo test
#[derive(Debug)]
pub struct CargoTestResult {
    /// Exit code from cargo test (0 = success, non-zero = tests failed or error)
    pub exit_code: i32,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Path to the trace file containing captured output
    pub trace_path: PathBuf,
}

/// Result from running cargo test to a bead-specific trace directory
#[derive(Debug)]
pub struct BeadTestResult {
    /// Exit code from cargo test (0 = success, non-zero = tests failed or error)
    pub exit_code: i32,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Path to the bead trace directory containing captured output
    pub bead_trace_dir: PathBuf,
    /// Captured stdout content
    pub stdout: String,
    /// Captured stderr content
    pub stderr: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_trace_metadata_default() {
        let metadata = TraceMetadata::default();
        assert_eq!(metadata.bead_id, None);
        assert_eq!(metadata.agent, "unknown");
        assert_eq!(metadata.outcome, "unknown");
        assert!(!metadata.pruned);
    }

    #[test]
    fn test_cargo_test_filename_format() {
        let filename = TraceManager::cargo_test_filename();
        assert!(filename.starts_with("cargo-test-"));
        assert!(filename.ends_with(".log"));
        // Format: cargo-test-YYYYMMDD-HHMMSS.log
        assert!(filename.len() == "cargo-test-20260724-123456.log".len());
    }

    #[test]
    fn test_ensure_traces_dir_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // Directory should not exist initially
        assert!(!manager.traces_dir.exists());

        // After ensuring, it should exist
        manager.ensure_traces_dir().unwrap();
        assert!(manager.traces_dir.is_dir());
    }

    #[test]
    fn test_ensure_traces_dir_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // First call creates directory
        manager.ensure_traces_dir().unwrap();

        // Second call should also succeed (idempotent)
        manager.ensure_traces_dir().unwrap();
        assert!(manager.traces_dir.is_dir());
    }

    #[test]
    fn test_bead_trace_dir_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        let bead_dir = manager.bead_trace_dir("bf-test123").unwrap();
        assert!(bead_dir.exists());
        assert!(bead_dir.ends_with("bf-test123"));
    }

    #[test]
    fn test_write_and_read_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        let mut metadata = TraceMetadata::default();
        metadata.bead_id = Some("bf-test123".to_string());
        metadata.agent = "test-agent".to_string();
        metadata.outcome = "success".to_string();

        manager.write_metadata("bf-test123", &metadata).unwrap();

        // Verify file exists and can be read
        let metadata_path = manager.bead_metadata_path("bf-test123");
        assert!(metadata_path.exists());

        let content = fs::read_to_string(&metadata_path).unwrap();
        let read_metadata: TraceMetadata = serde_json::from_str(&content).unwrap();
        assert_eq!(read_metadata.bead_id, Some("bf-test123".to_string()));
        assert_eq!(read_metadata.agent, "test-agent");
    }

    #[test]
    fn test_write_bead_trace() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        let metadata = TraceMetadata {
            bead_id: Some("bf-test456".to_string()),
            agent: "test-agent".to_string(),
            outcome: "success".to_string(),
            ..Default::default()
        };

        let stdout = "Test stdout output";
        let stderr = "Test stderr output";

        manager.write_bead_trace("bf-test456", &metadata, stdout, stderr).unwrap();

        // Verify all files exist
        assert!(manager.bead_metadata_path("bf-test456").exists());
        assert!(manager.bead_stdout_path("bf-test456").exists());
        assert!(manager.bead_stderr_path("bf-test456").exists());
    }

    #[test]
    fn test_has_bead_trace() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        assert!(!manager.has_bead_trace("bf-nonexistent"));

        manager.bead_trace_dir("bf-existing").unwrap();
        assert!(manager.has_bead_trace("bf-existing"));
    }

    #[test]
    fn test_write_cargo_test_trace() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        let test_output = "running 1 test.. test ok!";
        let trace_path = manager.write_cargo_test_trace(test_output).unwrap();

        assert!(trace_path.exists());
        let content = fs::read_to_string(&trace_path).unwrap();
        assert_eq!(content, test_output);
    }

    #[test]
    fn test_cargo_test_result_structure() {
        let temp_dir = TempDir::new().unwrap();
        let result = CargoTestResult {
            exit_code: 0,
            duration_ms: 1500,
            trace_path: temp_dir.path().join("test.log"),
        };

        assert_eq!(result.exit_code, 0);
        assert_eq!(result.duration_ms, 1500);
        assert!(result.trace_path.ends_with("test.log"));
    }

    #[test]
    fn test_run_cargo_test_in_temp_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // Create a minimal Rust project in the temp directory
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
        ).unwrap();

        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        let lib_rs = src_dir.join("lib.rs");
        fs::write(
            &lib_rs,
            r#"#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
"#
        ).unwrap();

        // Run cargo test
        let result = manager.run_cargo_test(temp_dir.path()).unwrap();

        // Verify the result
        assert_eq!(result.exit_code, 0, "cargo test should succeed");
        assert!(result.duration_ms > 0, "duration should be positive");
        assert!(result.trace_path.exists(), "trace file should exist");

        // Verify trace file contains expected output
        let content = fs::read_to_string(&result.trace_path).unwrap();
        assert!(content.contains("=== STDOUT ==="), "should have stdout section");
        assert!(content.contains("=== STDERR ==="), "should have stderr section");
        assert!(content.contains("=== EXIT CODE: 0 ==="), "should have exit code");
    }

    #[test]
    fn test_run_cargo_test_with_failing_tests() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // Create a minimal Rust project with a failing test
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "test-project-fail"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
        ).unwrap();

        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        let lib_rs = src_dir.join("lib.rs");
        fs::write(
            &lib_rs,
            r#"#[cfg(test)]
mod tests {
    #[test]
    fn it_fails() {
        assert_eq!(2 + 2, 5, "This test is designed to fail");
    }
}
"#
        ).unwrap();

        // Run cargo test - it should complete even though tests fail
        let result = manager.run_cargo_test(temp_dir.path()).unwrap();

        // Verify the result - should have non-zero exit code but still complete
        assert!(result.exit_code != 0, "cargo test should fail");
        assert!(result.duration_ms > 0, "duration should be positive");
        assert!(result.trace_path.exists(), "trace file should exist");

        // Verify trace file contains error output
        let content = fs::read_to_string(&result.trace_path).unwrap();
        assert!(content.contains("=== STDERR ==="), "should have stderr section");
        assert!(content.contains("=== EXIT CODE"), "should have exit code");
    }

    #[test]
    fn test_run_cargo_test_with_custom_args() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // Create a minimal Rust project
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "test-project-args"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
        ).unwrap();

        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        let lib_rs = src_dir.join("lib.rs");
        fs::write(
            &lib_rs,
            r#"#[cfg(test)]
mod tests {
    #[test]
    fn first_test() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn second_test() {
        assert_eq!(1 + 1, 2);
    }
}
"#
        ).unwrap();

        // Run cargo test with specific test filter
        let result = manager.run_cargo_test_with_args(
            temp_dir.path(),
            &["--", "first_test"]
        ).unwrap();

        // Verify the result
        assert_eq!(result.exit_code, 0, "cargo test should succeed");
        assert!(result.trace_path.exists(), "trace file should exist");
    }

    #[test]
    fn test_run_cargo_test_to_bead_trace() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // Create a minimal Rust project
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "test-bead-trace"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
        ).unwrap();

        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        let lib_rs = src_dir.join("lib.rs");
        fs::write(
            &lib_rs,
            r#"#[cfg(test)]
mod tests {
    #[test]
    fn bead_trace_test() {
        assert_eq!(2 + 2, 4);
    }
}
"#
        ).unwrap();

        // Create metadata for the trace
        let metadata = TraceMetadata {
            bead_id: Some("bf-test-8ei6pa".to_string()),
            agent: "test-needle-worker".to_string(),
            provider: Some("test-provider".to_string()),
            model: Some("test-model".to_string()),
            outcome: "success".to_string(),
            ..Default::default()
        };

        // Run cargo test and write to bead trace directory
        let result = manager.run_cargo_test_to_bead_trace(
            temp_dir.path(),
            "bf-test-8ei6pa",
            &metadata
        ).unwrap();

        // Verify the result
        assert_eq!(result.exit_code, 0, "cargo test should succeed");
        assert!(result.duration_ms > 0, "duration should be positive");
        assert!(result.bead_trace_dir.exists(), "bead trace directory should exist");
        assert!(!result.stdout.is_empty(), "stdout should not be empty");
        assert!(!result.stderr.is_empty() || result.exit_code == 0, "stderr may be empty on success");

        // Verify all expected files exist in the bead trace directory
        let metadata_path = manager.bead_metadata_path("bf-test-8ei6pa");
        let stdout_path = manager.bead_stdout_path("bf-test-8ei6pa");
        let stderr_path = manager.bead_stderr_path("bf-test-8ei6pa");

        assert!(metadata_path.exists(), "metadata.json should exist");
        assert!(stdout_path.exists(), "stdout.txt should exist");
        assert!(stderr_path.exists(), "stderr.txt should exist");

        // Verify metadata content
        let metadata_content = fs::read_to_string(&metadata_path).unwrap();
        let read_metadata: TraceMetadata = serde_json::from_str(&metadata_content).unwrap();
        assert_eq!(read_metadata.bead_id, Some("bf-test-8ei6pa".to_string()));
        assert_eq!(read_metadata.agent, "test-needle-worker");
        assert_eq!(read_metadata.exit_code, Some(0));
        assert_eq!(read_metadata.outcome, "success");

        // Verify stdout content
        let stdout_content = fs::read_to_string(&stdout_path).unwrap();
        assert!(stdout_content.contains("running 1 test") || stdout_content.contains("test result: ok"),
            "stdout should contain test output");
    }

    #[test]
    fn test_run_cargo_test_to_bead_trace_with_failure() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // Create a minimal Rust project with a failing test
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "test-bead-trace-fail"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
        ).unwrap();

        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        let lib_rs = src_dir.join("lib.rs");
        fs::write(
            &lib_rs,
            r#"#[cfg(test)]
mod tests {
    #[test]
    fn failing_test() {
        assert_eq!(2 + 2, 5, "This test is designed to fail");
    }
}
"#
        ).unwrap();

        // Create metadata for the trace
        let metadata = TraceMetadata {
            bead_id: Some("bf-test-fail".to_string()),
            agent: "test-needle-worker".to_string(),
            outcome: "failure".to_string(),
            ..Default::default()
        };

        // Run cargo test - it should complete even though tests fail
        let result = manager.run_cargo_test_to_bead_trace(
            temp_dir.path(),
            "bf-test-fail",
            &metadata
        ).unwrap();

        // Verify the result - should have non-zero exit code but still complete
        assert!(result.exit_code != 0, "cargo test should fail");
        assert!(result.duration_ms > 0, "duration should be positive");
        assert!(result.bead_trace_dir.exists(), "bead trace directory should exist");

        // Verify metadata shows failure
        let metadata_path = manager.bead_metadata_path("bf-test-fail");
        let metadata_content = fs::read_to_string(&metadata_path).unwrap();
        let read_metadata: TraceMetadata = serde_json::from_str(&metadata_content).unwrap();
        assert_eq!(read_metadata.exit_code, Some(result.exit_code));
        assert_eq!(read_metadata.outcome, "failure");
    }

    #[test]
    fn test_run_cargo_test_to_bead_trace_with_args() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // Create a minimal Rust project with multiple tests
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "test-bead-trace-args"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
        ).unwrap();

        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        let lib_rs = src_dir.join("lib.rs");
        fs::write(
            &lib_rs,
            r#"#[cfg(test)]
mod tests {
    #[test]
    fn first_test() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn second_test() {
        assert_eq!(1 + 1, 2);
    }
}
"#
        ).unwrap();

        // Create metadata for the trace
        let metadata = TraceMetadata {
            bead_id: Some("bf-test-args".to_string()),
            agent: "test-needle-worker".to_string(),
            outcome: "success".to_string(),
            ..Default::default()
        };

        // Run cargo test with specific test filter
        let result = manager.run_cargo_test_to_bead_trace_with_args(
            temp_dir.path(),
            "bf-test-args",
            &metadata,
            &["--", "first_test"]
        ).unwrap();

        // Verify the result
        assert_eq!(result.exit_code, 0, "cargo test should succeed");
        assert!(result.bead_trace_dir.exists(), "bead trace directory should exist");

        // Verify stdout contains the filtered test output
        let stdout_content = fs::read_to_string(
            &manager.bead_stdout_path("bf-test-args")
        ).unwrap();
        assert!(stdout_content.contains("first_test"), "stdout should mention the filtered test");
    }
}
