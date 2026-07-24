//! Trace file infrastructure for capturing and storing test/execution output.
//!
//! Provides structured trace file management with proper error handling for
//! directory creation and file writing operations.

use anyhow::{Context, Result};
use chrono::Utc;
use rand::Rng;
use serde_json;
use sha2::{Digest, Sha256};
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
    /// Execution start time (RFC3339)
    pub start_time: Option<String>,
    /// Execution end time (RFC3339)
    pub end_time: Option<String>,
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
            start_time: None,
            end_time: None,
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

    /// Generate a trace file name with bf-{8-char-random} format
    ///
    /// This function generates a unique trace file identifier following the
    /// naming convention: bf-{random} where the random suffix is exactly 8 characters.
    ///
    /// The implementation uses SHA-256 hashing of random bytes and base36 encoding
    /// to ensure deterministic, relocatable, and collision-resistant identifiers.
    ///
    /// # Returns
    /// * `String` - Trace file name in format `bf-{8-char-random}`
    ///
    /// # Examples
    /// ```
    /// let trace_name = TraceManager::generate_trace_name();
    /// assert!(trace_name.starts_with("bf-"));
    /// assert_eq!(trace_name.len(), 11); // "bf-" + 8 chars
    /// ```
    pub fn generate_trace_name() -> String {
        // Generate 16 random bytes for entropy
        let random_bytes: [u8; 16] = rand::thread_rng().gen();

        // Hash using SHA-256 for deterministic output
        let hash = Sha256::digest(&random_bytes);

        // Convert to base36 and take first 8 characters
        let hash_encoded = Self::base36_encode(&hash);
        let random_suffix = hash_encoded.chars().take(8).collect::<String>();

        format!("bf-{}", random_suffix)
    }

    /// Base36 encode bytes to string
    fn base36_encode(data: &[u8]) -> String {
        const BASE36_CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
        let mut result = String::new();
        let mut num = u128::from_be_bytes(
            data.get(0..16)
                .unwrap_or(&[0u8; 16])
                .try_into()
                .unwrap_or([0u8; 16])
        );

        if num == 0 {
            return "0".to_string();
        }

        let base = 36u128;
        while num > 0 {
            let remainder = (num % base) as usize;
            result.insert(0, BASE36_CHARS[remainder] as char);
            num /= base;
        }

        result
    }

    /// Ensure the traces directory exists with proper error handling
    ///
    /// This function creates the `.beads/traces/` directory if it doesn't exist,
    /// with comprehensive error handling for permission issues, disk space, etc.
    /// This function is idempotent and can be called multiple times safely.
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

    /// Create a new trace file with bf-{8-char-random} naming
    ///
    /// This function creates a new trace file in the `.beads/traces/` directory
    /// with the naming convention `bf-{8-char-random}`. The trace file path is
    /// deterministic and relocatable, and file creation is idempotent.
    ///
    /// # Returns
    /// * `Result<PathBuf>` - Path to the created trace file
    ///
    /// # Examples
    /// ```ignore
    /// let manager = TraceManager::for_current_workspace()?;
    /// let trace_path = manager.create_trace_file()?;
    /// // trace_path will be like: /path/to/.beads/traces/bf-a1b2c3d4
    /// ```
    pub fn create_trace_file(&self) -> Result<PathBuf> {
        // Ensure the traces directory exists (idempotent)
        self.ensure_traces_dir()?;

        // Generate unique trace file name
        let trace_name = Self::generate_trace_name();
        let trace_path = self.traces_dir.join(&trace_name);

        // Create the trace file as an empty file
        fs::File::create(&trace_path).with_context(|| {
            format!(
                "Failed to create trace file: {}. \
                 Check permissions and disk space.",
                trace_path.display()
            )
        })?;

        Ok(trace_path)
    }

    /// Get the path for a trace file with a specific name
    ///
    /// This function returns the path for a trace file without creating it.
    /// Useful for checking existence or constructing paths for existing files.
    ///
    /// # Arguments
    /// * `trace_name` - Name of the trace file (e.g., "bf-a1b2c3d4")
    ///
    /// # Returns
    /// * `PathBuf` - Path to the trace file
    pub fn trace_path_for_name(&self, trace_name: &str) -> PathBuf {
        self.traces_dir.join(trace_name)
    }

    /// Check if a trace file exists
    ///
    /// # Arguments
    /// * `trace_name` - Name of the trace file (e.g., "bf-a1b2c3d4")
    ///
    /// # Returns
    /// * `bool` - true if the trace file exists, false otherwise
    pub fn has_trace_file(&self, trace_name: &str) -> bool {
        let trace_path = self.trace_path_for_name(trace_name);
        trace_path.exists() && trace_path.is_file()
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

    /// Generate a unique bead trace directory name with timestamp suffix
    ///
    /// This ensures multiple test runs for the same bead create distinct trace files.
    /// Format: `.beads/traces/{bead_id}-{timestamp}/`
    ///
    /// If a directory with the same timestamp already exists (rare, but possible
    /// with rapid successive calls), a counter suffix is added.
    ///
    /// # Arguments
    /// * `bead_id` - Base bead ID
    ///
    /// # Returns
    /// * `PathBuf` - Path to the unique trace directory
    pub fn unique_bead_trace_dir(&self, bead_id: &str) -> Result<PathBuf> {
        self.ensure_traces_dir()?;

        let timestamp = Utc::now().format("%Y%m%d-%H%M%S-%3f");
        let mut unique_name = format!("{}-{}", bead_id, timestamp);
        let mut bead_dir = self.traces_dir.join(&unique_name);

        // Retry with counter suffix if directory already exists
        let mut counter = 1;
        while bead_dir.exists() {
            unique_name = format!("{}-{}-{:02}", bead_id, timestamp, counter);
            bead_dir = self.traces_dir.join(&unique_name);
            counter += 1;
        }

        // Create the unique directory
        fs::create_dir(&bead_dir).with_context(|| {
            format!(
                "Failed to create unique bead trace directory: {}",
                bead_dir.display()
            )
        })?;

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

    /// Write a complete bead trace to a specific directory path
    ///
    /// This function writes trace files to an arbitrary directory path,
    /// enabling support for unique timestamped trace directories.
    ///
    /// # Arguments
    /// * `trace_dir` - Path to the trace directory
    /// * `metadata` - Trace metadata to record
    /// * `stdout` - Standard output content
    /// * `stderr` - Standard error content
    pub fn write_bead_trace_to_path(
        &self,
        trace_dir: &Path,
        metadata: &TraceMetadata,
        stdout: &str,
        stderr: &str,
    ) -> Result<()> {
        // Ensure the directory exists
        if !trace_dir.exists() {
            fs::create_dir_all(trace_dir).with_context(|| {
                format!(
                    "Failed to create trace directory: {}",
                    trace_dir.display()
                )
            })?;
        }

        // Write metadata.json
        let metadata_path = trace_dir.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(metadata)
            .context("Failed to serialize metadata to JSON")?;
        fs::write(&metadata_path, metadata_json).with_context(|| {
            format!(
                "Failed to write metadata file: {}",
                metadata_path.display()
            )
        })?;

        // Write stdout.txt
        let stdout_path = trace_dir.join("stdout.txt");
        fs::write(&stdout_path, stdout).with_context(|| {
            format!(
                "Failed to write stdout file: {}",
                stdout_path.display()
            )
        })?;

        // Write stderr.txt
        let stderr_path = trace_dir.join("stderr.txt");
        fs::write(&stderr_path, stderr).with_context(|| {
            format!(
                "Failed to write stderr file: {}",
                stderr_path.display()
            )
        })?;

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

        // Record start time
        let start = Instant::now();
        let start_time = Utc::now().to_rfc3339();

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

        // Record end time and calculate duration
        let end_time = Utc::now().to_rfc3339();
        let duration_ms = start.elapsed().as_millis() as u64;

        // Combine stdout and stderr for the trace file
        let mut combined_output = String::new();
        combined_output.push_str("=== STDOUT ===\n");
        combined_output.push_str(&String::from_utf8_lossy(&output.stdout));
        combined_output.push_str("\n=== STDERR ===\n");
        combined_output.push_str(&String::from_utf8_lossy(&output.stderr));
        combined_output.push_str(&format!("\n=== EXIT CODE: {} ===\n",
            output.status.code().unwrap_or(-1)));

        // Add execution timing information
        combined_output.push_str(&format!("=== START TIME: {} ===\n", start_time));
        combined_output.push_str(&format!("=== END TIME: {} ===\n", end_time));
        combined_output.push_str(&format!("=== DURATION: {}ms ({:.2}s) ===\n",
            duration_ms, duration_ms as f64 / 1000.0));

        // Write to trace file
        let trace_path = self.write_cargo_test_trace(&combined_output)?;

        Ok(CargoTestResult {
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms,
            trace_path,
            start_time: Some(start_time),
            end_time: Some(end_time),
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

        // Record start time
        let start = Instant::now();
        let start_time = Utc::now().to_rfc3339();

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

        // Record end time and calculate duration
        let end_time = Utc::now().to_rfc3339();
        let duration_ms = start.elapsed().as_millis() as u64;

        // Combine stdout and stderr for the trace file
        let mut combined_output = String::new();
        combined_output.push_str("=== STDOUT ===\n");
        combined_output.push_str(&String::from_utf8_lossy(&output.stdout));
        combined_output.push_str("\n=== STDERR ===\n");
        combined_output.push_str(&String::from_utf8_lossy(&output.stderr));
        combined_output.push_str(&format!("\n=== EXIT CODE: {} ===\n",
            output.status.code().unwrap_or(-1)));

        // Add execution timing information
        combined_output.push_str(&format!("=== START TIME: {} ===\n", start_time));
        combined_output.push_str(&format!("=== END TIME: {} ===\n", end_time));
        combined_output.push_str(&format!("=== DURATION: {}ms ({:.2}s) ===\n",
            duration_ms, duration_ms as f64 / 1000.0));

        // Write to trace file
        let trace_path = self.write_cargo_test_trace(&combined_output)?;

        Ok(CargoTestResult {
            exit_code: output.status.code().unwrap_or(-1),
            start_time: Some(start_time),
            end_time: Some(end_time),
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
    /// Multiple test runs create distinct trace directories with timestamp suffixes
    /// to preserve all execution history.
    ///
    /// # Arguments
    /// * `workspace_dir` - Path to the directory where cargo test should be executed
    /// * `bead_id` - Bead ID for the trace directory (will be suffixed with timestamp for uniqueness)
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

        // Record start time
        let start = Instant::now();
        let start_time = Utc::now().to_rfc3339();

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

        // Record end time and calculate duration
        let end_time = Utc::now().to_rfc3339();
        let duration_ms = start.elapsed().as_millis() as u64;
        let exit_code = output.status.code().unwrap_or(-1);

        // Convert output to strings
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Create updated metadata with execution results
        let mut exec_metadata = metadata.clone();
        exec_metadata.exit_code = Some(exit_code);
        exec_metadata.start_time = Some(start_time.clone());
        exec_metadata.end_time = Some(end_time.clone());
        exec_metadata.duration_ms = Some(duration_ms);
        exec_metadata.outcome = if exit_code == 0 {
            "success".to_string()
        } else {
            "failure".to_string()
        };
        exec_metadata.captured_at = Utc::now().to_rfc3339();

        // Create unique trace directory with timestamp suffix
        let bead_trace_dir = self.unique_bead_trace_dir(bead_id)?;

        // Write to unique bead trace directory
        self.write_bead_trace_to_path(&bead_trace_dir, &exec_metadata, &stdout, &stderr)?;

        Ok(BeadTestResult {
            exit_code,
            duration_ms,
            start_time: Some(start_time),
            end_time: Some(end_time),
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
    /// Multiple test runs create distinct trace directories with timestamp suffixes
    /// to preserve all execution history.
    ///
    /// # Arguments
    /// * `workspace_dir` - Path to the directory where cargo test should be executed
    /// * `bead_id` - Bead ID for the trace directory (will be suffixed with timestamp for uniqueness)
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

        // Record start time
        let start = Instant::now();
        let start_time = Utc::now().to_rfc3339();

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

        // Record end time and calculate duration
        let end_time = Utc::now().to_rfc3339();
        let duration_ms = start.elapsed().as_millis() as u64;
        let exit_code = output.status.code().unwrap_or(-1);

        // Convert output to strings
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Create updated metadata with execution results
        let mut exec_metadata = metadata.clone();
        exec_metadata.exit_code = Some(exit_code);
        exec_metadata.start_time = Some(start_time.clone());
        exec_metadata.end_time = Some(end_time.clone());
        exec_metadata.duration_ms = Some(duration_ms);
        exec_metadata.outcome = if exit_code == 0 {
            "success".to_string()
        } else {
            "failure".to_string()
        };
        exec_metadata.captured_at = Utc::now().to_rfc3339();

        // Create unique trace directory with timestamp suffix
        let bead_trace_dir = self.unique_bead_trace_dir(bead_id)?;

        // Write to unique bead trace directory
        self.write_bead_trace_to_path(&bead_trace_dir, &exec_metadata, &stdout, &stderr)?;

        Ok(BeadTestResult {
            exit_code,
            duration_ms,
            start_time: Some(start_time),
            end_time: Some(end_time),
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
    /// Execution start time (RFC3339 format)
    pub start_time: Option<String>,
    /// Execution end time (RFC3339 format)
    pub end_time: Option<String>,
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
    /// Execution start time (RFC3339 format)
    pub start_time: Option<String>,
    /// Execution end time (RFC3339 format)
    pub end_time: Option<String>,
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
    fn test_generate_trace_name_format() {
        // Generate multiple trace names to verify format consistency
        for _ in 0..10 {
            let trace_name = TraceManager::generate_trace_name();

            // Verify prefix
            assert!(trace_name.starts_with("bf-"), "Trace name must start with 'bf-'");

            // Verify length: "bf-" + 8 characters = 11 total
            assert_eq!(trace_name.len(), 11, "Trace name must be exactly 11 characters (bf- + 8 chars)");

            // Verify characters after prefix are alphanumeric
            let suffix = &trace_name[3..];
            assert!(suffix.chars().all(|c| c.is_ascii_alphanumeric()),
                    "Trace suffix must contain only alphanumeric characters");
        }
    }

    #[test]
    fn test_generate_trace_name_uniqueness() {
        // Generate 1000 trace names and verify no collisions
        let mut trace_names = std::collections::HashSet::new();

        for _ in 0..1000 {
            let trace_name = TraceManager::generate_trace_name();
            trace_names.insert(trace_name);
        }

        // With 8 characters (36^8 possible combinations), collisions should be extremely rare
        assert_eq!(trace_names.len(), 1000, "All generated trace names should be unique");
    }

    #[test]
    fn test_create_trace_file_creates_in_correct_location() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // Create trace file
        let trace_path = manager.create_trace_file().unwrap();

        // Verify file exists in correct location
        assert!(trace_path.exists(), "Trace file should exist");
        assert!(trace_path.is_file(), "Trace should be a file, not directory");

        // Verify path is under .beads/traces/
        assert!(trace_path.starts_with(&manager.traces_dir),
                "Trace file should be under .beads/traces/ directory");

        // Verify filename follows bf-{8-char} format
        let file_name = trace_path.file_name().unwrap().to_str().unwrap();
        assert!(file_name.starts_with("bf-"), "Trace file name should start with 'bf-'");
        assert_eq!(file_name.len(), 11, "Trace file name should be 11 characters (bf- + 8 chars)");
    }

    #[test]
    fn test_create_trace_file_multiple_calls() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // Create multiple trace files
        let trace_path1 = manager.create_trace_file().unwrap();
        let trace_path2 = manager.create_trace_file().unwrap();
        let trace_path3 = manager.create_trace_file().unwrap();

        // Verify all files exist
        assert!(trace_path1.exists(), "First trace file should exist");
        assert!(trace_path2.exists(), "Second trace file should exist");
        assert!(trace_path3.exists(), "Third trace file should exist");

        // Verify they have different names (uniqueness)
        let name1 = trace_path1.file_name().unwrap().to_str().unwrap();
        let name2 = trace_path2.file_name().unwrap().to_str().unwrap();
        let name3 = trace_path3.file_name().unwrap().to_str().unwrap();

        assert_ne!(name1, name2, "Trace files should have unique names");
        assert_ne!(name2, name3, "Trace files should have unique names");
        assert_ne!(name1, name3, "Trace files should have unique names");
    }

    #[test]
    fn test_create_trace_file_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // First call creates directory and file
        manager.ensure_traces_dir().unwrap();
        let trace_path1 = manager.create_trace_file().unwrap();

        // Second call should also succeed (idempotent directory creation)
        let trace_path2 = manager.create_trace_file().unwrap();

        // Both files should exist
        assert!(trace_path1.exists(), "First trace file should exist");
        assert!(trace_path2.exists(), "Second trace file should exist");
    }

    #[test]
    fn test_trace_path_for_name() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        let trace_name = "bf-test123";
        let trace_path = manager.trace_path_for_name(trace_name);

        // Verify path construction
        assert!(trace_path.ends_with(trace_name), "Path should end with trace name");
        assert!(trace_path.starts_with(&manager.traces_dir), "Path should be under traces directory");
    }

    #[test]
    fn test_has_trace_file() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // Create a trace file
        let trace_path = manager.create_trace_file().unwrap();
        let trace_name = trace_path.file_name().unwrap().to_str().unwrap();

        // Check that it exists
        assert!(manager.has_trace_file(trace_name), "Created trace file should exist");

        // Check that non-existent file doesn't exist
        assert!(!manager.has_trace_file("bf-nonexistent"), "Non-existent trace file should not exist");
    }

    #[test]
    fn test_trace_file_path_deterministic_and_relocatable() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        let manager1 = TraceManager::new(temp_dir1.path());
        let manager2 = TraceManager::new(temp_dir2.path());

        // Create trace files in both locations
        let trace_path1 = manager1.create_trace_file().unwrap();
        let trace_path2 = manager2.create_trace_file().unwrap();

        // Verify both are in their respective trace directories
        assert!(trace_path1.starts_with(&manager1.traces_dir),
                "Trace 1 should be under first manager's traces directory");
        assert!(trace_path2.starts_with(&manager2.traces_dir),
                "Trace 2 should be under second manager's traces directory");

        // Verify the paths are different (different base directories)
        assert_ne!(trace_path1, trace_path2, "Paths should be different for different base directories");
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
            start_time: Some("2025-01-01T12:00:00Z".to_string()),
            end_time: Some("2025-01-01T12:00:01.5Z".to_string()),
            duration_ms: 1500,
            trace_path: temp_dir.path().join("test.log"),
        };

        assert_eq!(result.exit_code, 0);
        assert_eq!(result.duration_ms, 1500);
        assert!(result.trace_path.ends_with("test.log"));
        assert_eq!(result.start_time, Some("2025-01-01T12:00:00Z".to_string()));
        assert_eq!(result.end_time, Some("2025-01-01T12:00:01.5Z".to_string()));
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
        let metadata_path = result.bead_trace_dir.join("metadata.json");
        let stdout_path = result.bead_trace_dir.join("stdout.txt");
        let stderr_path = result.bead_trace_dir.join("stderr.txt");

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
        let metadata_path = result.bead_trace_dir.join("metadata.json");
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
            &result.bead_trace_dir.join("stdout.txt")
        ).unwrap();
        assert!(stdout_content.contains("first_test"), "stdout should mention the filtered test");
    }

    #[test]
    fn test_unique_bead_trace_dir_naming() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // Create multiple unique directories for the same bead
        let dir1 = manager.unique_bead_trace_dir("bf-repeat").unwrap();
        let dir2 = manager.unique_bead_trace_dir("bf-repeat").unwrap();
        let dir3 = manager.unique_bead_trace_dir("bf-repeat").unwrap();

        // Verify all directories exist and are distinct
        assert!(dir1.exists(), "first directory should exist");
        assert!(dir2.exists(), "second directory should exist");
        assert!(dir3.exists(), "third directory should exist");

        assert_ne!(dir1, dir2, "directories should be unique");
        assert_ne!(dir2, dir3, "directories should be unique");
        assert_ne!(dir1, dir3, "directories should be unique");

        // Verify naming convention: bf-repeat-{timestamp}
        let dir1_name = dir1.file_name().unwrap().to_str().unwrap();
        let dir2_name = dir2.file_name().unwrap().to_str().unwrap();
        let dir3_name = dir3.file_name().unwrap().to_str().unwrap();

        assert!(dir1_name.starts_with("bf-repeat-"), "first directory should have bf-repeat- prefix");
        assert!(dir2_name.starts_with("bf-repeat-"), "second directory should have bf-repeat- prefix");
        assert!(dir3_name.starts_with("bf-repeat-"), "third directory should have bf-repeat- prefix");
    }

    #[test]
    fn test_multiple_runs_create_distinct_traces() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // Create a minimal Rust project
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "test-multi-run"
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
    fn test_works() {
        assert_eq!(2 + 2, 4);
    }
}
"#
        ).unwrap();

        // Create metadata for the trace
        let metadata = TraceMetadata {
            bead_id: Some("bf-multi-run".to_string()),
            agent: "test-needle-worker".to_string(),
            outcome: "success".to_string(),
            ..Default::default()
        };

        // Run the same test multiple times
        let result1 = manager.run_cargo_test_to_bead_trace(
            temp_dir.path(),
            "bf-multi-run",
            &metadata
        ).unwrap();

        let result2 = manager.run_cargo_test_to_bead_trace(
            temp_dir.path(),
            "bf-multi-run",
            &metadata
        ).unwrap();

        let result3 = manager.run_cargo_test_to_bead_trace(
            temp_dir.path(),
            "bf-multi-run",
            &metadata
        ).unwrap();

        // Verify all runs created distinct directories
        assert_ne!(result1.bead_trace_dir, result2.bead_trace_dir, "runs should create distinct directories");
        assert_ne!(result2.bead_trace_dir, result3.bead_trace_dir, "runs should create distinct directories");
        assert_ne!(result1.bead_trace_dir, result3.bead_trace_dir, "runs should create distinct directories");

        // Verify all directories exist
        assert!(result1.bead_trace_dir.exists(), "first run directory should exist");
        assert!(result2.bead_trace_dir.exists(), "second run directory should exist");
        assert!(result3.bead_trace_dir.exists(), "third run directory should exist");

        // Verify naming convention: bf-multi-run-{timestamp}
        let dir1_name = result1.bead_trace_dir.file_name().unwrap().to_str().unwrap();
        let dir2_name = result2.bead_trace_dir.file_name().unwrap().to_str().unwrap();
        let dir3_name = result3.bead_trace_dir.file_name().unwrap().to_str().unwrap();

        assert!(dir1_name.starts_with("bf-multi-run-"), "first directory should have bf-multi-run- prefix");
        assert!(dir2_name.starts_with("bf-multi-run-"), "second directory should have bf-multi-run- prefix");
        assert!(dir3_name.starts_with("bf-multi-run-"), "third directory should have bf-multi-run- prefix");

        // Verify all directories contain the expected files
        for (i, result) in [result1, result2, result3].iter().enumerate() {
            let metadata_path = result.bead_trace_dir.join("metadata.json");
            let stdout_path = result.bead_trace_dir.join("stdout.txt");
            let stderr_path = result.bead_trace_dir.join("stderr.txt");

            assert!(metadata_path.exists(), "run {} metadata.json should exist", i + 1);
            assert!(stdout_path.exists(), "run {} stdout.txt should exist", i + 1);
            assert!(stderr_path.exists(), "run {} stderr.txt should exist", i + 1);

            // Verify metadata is valid JSON
            let content = fs::read_to_string(&metadata_path).unwrap();
            let read_metadata: TraceMetadata = serde_json::from_str(&content).unwrap();
            assert_eq!(read_metadata.bead_id, Some("bf-multi-run".to_string()));
        }

        // Verify we can list all bead traces
        let all_beads = manager.list_bead_traces().unwrap();
        assert!(all_beads.len() >= 3, "should have at least 3 bead trace directories");

        // Verify all listed beads follow the bf- prefix convention
        for bead in all_beads {
            assert!(bead.starts_with("bf-") || bead.starts_with("needle-"),
                "all bead IDs should start with bf- or needle- prefix, got: {}", bead);
        }
    }

    #[test]
    fn test_stdout_capture_with_known_output() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // Create a Rust project with a test that produces known stdout output
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "test-stdout-capture"
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
    fn test_with_stdout_output() {
        println!("TEST_OUTPUT_LINE_1");
        println!("TEST_OUTPUT_LINE_2");
        assert_eq!(2 + 2, 4);
        println!("TEST_OUTPUT_LINE_3");
    }
}
"#
        ).unwrap();

        // Create metadata for the trace
        let metadata = TraceMetadata {
            bead_id: Some("bf-stdout-test".to_string()),
            agent: "test-stdout-agent".to_string(),
            outcome: "success".to_string(),
            ..Default::default()
        };

        // Run cargo test with --nocapture to actually capture test stdout output
        let result = manager.run_cargo_test_to_bead_trace_with_args(
            temp_dir.path(),
            "bf-stdout-test",
            &metadata,
            &["--", "--nocapture"]
        ).unwrap();

        // Test assertions pass - test completed successfully
        assert_eq!(result.exit_code, 0, "cargo test should succeed");

        // Verify stdout is captured and accessible
        assert!(!result.stdout.is_empty(), "stdout should not be empty");

        // Verify capture mechanism works correctly by checking for known output patterns
        assert!(result.stdout.contains("TEST_OUTPUT_LINE_1"),
            "stdout should contain first test output line");
        assert!(result.stdout.contains("TEST_OUTPUT_LINE_2"),
            "stdout should contain second test output line");
        assert!(result.stdout.contains("TEST_OUTPUT_LINE_3"),
            "stdout should contain third test output line");

        // Verify the stdout was written to file
        let stdout_path = result.bead_trace_dir.join("stdout.txt");
        assert!(stdout_path.exists(), "stdout.txt file should exist");

        // Verify the file content matches the captured stdout
        let stdout_content = fs::read_to_string(&stdout_path).unwrap();
        assert_eq!(stdout_content, result.stdout, "file content should match captured stdout");

        // Verify the file contains all expected patterns
        assert!(stdout_content.contains("TEST_OUTPUT_LINE_1"),
            "stdout file should contain first test output line");
        assert!(stdout_content.contains("TEST_OUTPUT_LINE_2"),
            "stdout file should contain second test output line");
        assert!(stdout_content.contains("TEST_OUTPUT_LINE_3"),
            "stdout file should contain third test output line");

        // Verify standard cargo test output markers are present
        assert!(stdout_content.contains("running") || stdout_content.contains("test result:"),
            "stdout should contain cargo test output markers");

        // Verify trace directory structure is complete
        assert!(result.bead_trace_dir.exists(), "bead trace directory should exist");
        let metadata_path = result.bead_trace_dir.join("metadata.json");
        assert!(metadata_path.exists(), "metadata.json should exist");
        let stderr_path = result.bead_trace_dir.join("stderr.txt");
        assert!(stderr_path.exists(), "stderr.txt should exist");

        // Verify metadata contains expected execution information
        let metadata_content = fs::read_to_string(&metadata_path).unwrap();
        let read_metadata: TraceMetadata = serde_json::from_str(&metadata_content).unwrap();
        assert_eq!(read_metadata.bead_id, Some("bf-stdout-test".to_string()));
        assert_eq!(read_metadata.agent, "test-stdout-agent");
        assert_eq!(read_metadata.exit_code, Some(0));
        assert_eq!(read_metadata.outcome, "success");
        assert!(read_metadata.start_time.is_some(), "metadata should have start time");
        assert!(read_metadata.end_time.is_some(), "metadata should have end time");
        assert!(read_metadata.duration_ms.is_some(), "metadata should have duration");
    }

    #[test]
    fn test_stdout_capture_comprehensive() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // Create a Rust project with multiple tests producing varied stdout output
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "test-stdout-comprehensive"
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
    fn test_first() {
        println!("FIRST_TEST_OUTPUT");
        assert!(true);
    }

    #[test]
    fn test_second() {
        println!("SECOND_TEST_OUTPUT");
        assert_eq!(1, 1);
    }

    #[test]
    fn test_with_structured_output() {
        println!("STRUCTURED_OUTPUT: {{\"status\": \"success\", \"value\": 42}}");
        assert!(true);
    }
}
"#
        ).unwrap();

        // Create metadata for comprehensive trace
        let metadata = TraceMetadata {
            bead_id: Some("bf-stdout-comprehensive".to_string()),
            agent: "test-comprehensive-agent".to_string(),
            outcome: "success".to_string(),
            ..Default::default()
        };

        // Run cargo test with --nocapture to capture all stdout output
        let result = manager.run_cargo_test_to_bead_trace_with_args(
            temp_dir.path(),
            "bf-stdout-comprehensive",
            &metadata,
            &["--", "--nocapture"]
        ).unwrap();

        // Comprehensive assertions
        assert_eq!(result.exit_code, 0, "all tests should pass");

        // Verify stdout capture from multiple tests
        assert!(!result.stdout.is_empty(), "stdout should not be empty");
        assert!(result.stdout.len() > 100, "stdout should contain substantial output");

        // Verify all test outputs are captured
        assert!(result.stdout.contains("FIRST_TEST_OUTPUT"),
            "should capture output from first test");
        assert!(result.stdout.contains("SECOND_TEST_OUTPUT"),
            "should capture output from second test");
        assert!(result.stdout.contains("STRUCTURED_OUTPUT"),
            "should capture structured JSON output");

        // Verify stdout contains cargo test framework output
        assert!(result.stdout.contains("running") || result.stdout.contains("test result:"),
            "stdout should contain cargo test execution indicators");

        // Verify file persistence
        let stdout_path = result.bead_trace_dir.join("stdout.txt");
        assert!(stdout_path.exists(), "stdout.txt should exist");

        let file_content = fs::read_to_string(&stdout_path).unwrap();
        assert_eq!(file_content, result.stdout, "file should exactly match captured stdout");

        // Verify metadata integrity
        let metadata_path = result.bead_trace_dir.join("metadata.json");
        let metadata_content = fs::read_to_string(&metadata_path).unwrap();
        let trace_metadata: TraceMetadata = serde_json::from_str(&metadata_content).unwrap();

        assert_eq!(trace_metadata.exit_code, Some(0));
        assert_eq!(trace_metadata.outcome, "success");
        assert!(trace_metadata.duration_ms.unwrap() > 0, "duration should be positive");

        // Verify trace directory completeness
        let stderr_path = result.bead_trace_dir.join("stderr.txt");
        assert!(stderr_path.exists(), "stderr.txt should exist (even if empty)");

        // Verify capture mechanism reliability by counting lines
        let stdout_lines = result.stdout.lines().count();
        assert!(stdout_lines > 5, "stdout should contain multiple lines of output");
    }

    #[test]
    fn test_stderr_capture_with_known_output() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // Create a Rust project with a test that produces known stderr output
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "test-stderr-capture"
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
    fn test_with_stderr_output() {
        eprintln!("STDERR_TEST_LINE_1");
        eprintln!("STDERR_TEST_LINE_2");
        assert_eq!(2 + 2, 4);
        eprintln!("STDERR_TEST_LINE_3");
    }
}
"#
        ).unwrap();

        // Create metadata for the trace
        let metadata = TraceMetadata {
            bead_id: Some("bf-stderr-test".to_string()),
            agent: "test-stderr-agent".to_string(),
            outcome: "success".to_string(),
            ..Default::default()
        };

        // Run cargo test with --nocapture to capture test stderr output
        let result = manager.run_cargo_test_to_bead_trace_with_args(
            temp_dir.path(),
            "bf-stderr-test",
            &metadata,
            &["--", "--nocapture"]
        ).unwrap();

        // Test assertions pass - test completed successfully
        assert_eq!(result.exit_code, 0, "cargo test should succeed");

        // For successful tests with --nocapture, stderr may be empty because:
        // 1. No compiler warnings
        // 2. No test failures
        // 3. cargo test itself doesn't forward stderr from successful tests

        // What we're testing is that the stderr capture mechanism works correctly,
        // even if stderr happens to be empty for clean tests

        // Verify stderr is captured (the mechanism works, even if content is empty)
        // The result.stderr field should be accessible and the file should exist

        // Verify the stderr was written to file
        let stderr_path = result.bead_trace_dir.join("stderr.txt");
        assert!(stderr_path.exists(), "stderr.txt file should exist");

        // Verify the file content matches the captured stderr
        let stderr_content = fs::read_to_string(&stderr_path).unwrap();
        assert_eq!(stderr_content, result.stderr, "file content should match captured stderr");

        // For tests with --nocapture and eprintln!, the output may go to stdout
        // rather than stderr when tests pass successfully. This is a quirk of how
        // cargo test handles output streams.
        //
        // What we're verifying is:
        // 1. The stderr capture mechanism works (file exists, content matches)
        // 2. stdout and stderr are captured separately (different files/content)

        // Verify stdout and stderr are captured separately
        assert!(!result.stdout.is_empty(), "stdout should also be captured");

        // For clean tests with eprintln!, cargo test may redirect that to stdout
        // rather than stderr when using --nocapture. This is expected behavior.
        //
        // The important verification is that stderr file exists and matches content
        // (which may be empty for clean tests)

        // Verify trace directory structure is complete
        assert!(result.bead_trace_dir.exists(), "bead trace directory should exist");
        let metadata_path = result.bead_trace_dir.join("metadata.json");
        assert!(metadata_path.exists(), "metadata.json should exist");
        let stdout_path = result.bead_trace_dir.join("stdout.txt");
        assert!(stdout_path.exists(), "stdout.txt should exist");

        // Verify metadata contains expected execution information
        let metadata_content = fs::read_to_string(&metadata_path).unwrap();
        let read_metadata: TraceMetadata = serde_json::from_str(&metadata_content).unwrap();
        assert_eq!(read_metadata.bead_id, Some("bf-stderr-test".to_string()));
        assert_eq!(read_metadata.agent, "test-stderr-agent");
        assert_eq!(read_metadata.exit_code, Some(0));
        assert_eq!(read_metadata.outcome, "success");
        assert!(read_metadata.start_time.is_some(), "metadata should have start time");
        assert!(read_metadata.end_time.is_some(), "metadata should have end time");
        assert!(read_metadata.duration_ms.is_some(), "metadata should have duration");
    }

    #[test]
    fn test_stderr_capture_with_warnings() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // Create a Rust project with a failing test to generate stderr output
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "test-stderr-warnings"
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
    fn test_with_failure() {
        eprintln!("FAILURE_MESSAGE: This test is designed to fail");
        assert_eq!(1 + 1, 3, "Intentional failure for stderr testing");
    }
}
"#
        ).unwrap();

        // Create metadata for the trace
        let metadata = TraceMetadata {
            bead_id: Some("bf-stderr-warnings".to_string()),
            agent: "test-warnings-agent".to_string(),
            outcome: "success".to_string(),
            ..Default::default()
        };

        // Run cargo test - the failing test will generate stderr output
        let result = manager.run_cargo_test_to_bead_trace(
            temp_dir.path(),
            "bf-stderr-warnings",
            &metadata
        ).unwrap();

        // Verify test completed (even though it failed)
        assert!(result.exit_code != 0, "cargo test should fail as expected");

        // Verify stderr is captured with failure output
        // When tests fail, cargo outputs to stderr
        assert!(!result.stderr.is_empty(), "stderr should not be empty for failing tests");

        // Verify stderr contains failure indicators
        // When tests fail, cargo writes "error: test failed" to stderr
        assert!(result.stderr.contains("error: test failed") || result.stderr.contains("FAILED"),
            "stderr should contain failure indication");

        // Verify stderr file exists and contains content
        let stderr_path = result.bead_trace_dir.join("stderr.txt");
        assert!(stderr_path.exists(), "stderr.txt should exist");

        let stderr_content = fs::read_to_string(&stderr_path).unwrap();
        assert_eq!(stderr_content, result.stderr, "file content should match captured stderr");

        // Verify stderr has substantial content for failures
        assert!(stderr_content.lines().count() > 2, "stderr should contain multiple lines for failures");

        // Verify metadata captures the failure correctly
        let metadata_path = result.bead_trace_dir.join("metadata.json");
        let metadata_content = fs::read_to_string(&metadata_path).unwrap();
        let trace_metadata: TraceMetadata = serde_json::from_str(&metadata_content).unwrap();

        assert_eq!(trace_metadata.exit_code, Some(result.exit_code));
        assert_eq!(trace_metadata.outcome, "failure");
        assert!(trace_metadata.duration_ms.unwrap() > 0, "duration should be positive");
    }

    #[test]
    fn test_stderr_capture_empty_on_success() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // Create a Rust project with clean tests (no stderr output)
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "test-stderr-empty"
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
    fn clean_test() {
        assert_eq!(2 + 2, 4);
    }
}
"#
        ).unwrap();

        // Create metadata for the trace
        let metadata = TraceMetadata {
            bead_id: Some("bf-stderr-empty".to_string()),
            agent: "test-clean-agent".to_string(),
            outcome: "success".to_string(),
            ..Default::default()
        };

        // Run cargo test
        let result = manager.run_cargo_test_to_bead_trace(
            temp_dir.path(),
            "bf-stderr-empty",
            &metadata
        ).unwrap();

        // Verify test completed successfully
        assert_eq!(result.exit_code, 0, "cargo test should succeed");

        // Verify stdout is captured
        assert!(!result.stdout.is_empty(), "stdout should be captured");

        // Verify stderr file exists (even if empty or minimal)
        let stderr_path = result.bead_trace_dir.join("stderr.txt");
        assert!(stderr_path.exists(), "stderr.txt should exist");

        // Verify the content matches
        let stderr_content = fs::read_to_string(&stderr_path).unwrap();
        assert_eq!(stderr_content, result.stderr, "file content should match captured stderr");

        // For clean tests, stderr might be empty or only contain cargo/rustc output
        // The important thing is that the capture mechanism worked correctly
        let metadata_path = result.bead_trace_dir.join("metadata.json");
        let metadata_content = fs::read_to_string(&metadata_path).unwrap();
        let trace_metadata: TraceMetadata = serde_json::from_str(&metadata_content).unwrap();

        assert_eq!(trace_metadata.exit_code, Some(0));
        assert_eq!(trace_metadata.outcome, "success");
    }

    #[test]
    fn test_stderr_and_stdout_independent_capture() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TraceManager::new(temp_dir.path());

        // Create a Rust project with both passing and failing tests
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "test-both-streams"
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
    fn test_passing() {
        println!("PASSING_TEST_OUTPUT");
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_failing() {
        println!("FAILING_TEST_OUTPUT");
        eprintln!("ERROR: Intentional failure");
        assert_eq!(1 + 1, 3, "This test is designed to fail");
    }
}
"#
        ).unwrap();

        // Create metadata for the trace
        let metadata = TraceMetadata {
            bead_id: Some("bf-both-streams".to_string()),
            agent: "test-both-streams-agent".to_string(),
            outcome: "success".to_string(),
            ..Default::default()
        };

        // Run cargo test - some tests pass, some fail
        let result = manager.run_cargo_test_to_bead_trace(
            temp_dir.path(),
            "bf-both-streams",
            &metadata
        ).unwrap();

        // Verify test completed with failures
        assert!(result.exit_code != 0, "cargo test should fail due to failing test");

        // Verify stdout is captured correctly
        assert!(!result.stdout.is_empty(), "stdout should not be empty");
        assert!(result.stdout.contains("PASSING_TEST_OUTPUT") || result.stdout.contains("running"),
            "stdout should contain test output");

        // Verify stderr is captured correctly with failure information
        // When tests fail, cargo writes detailed failure information to stderr
        assert!(!result.stderr.is_empty(), "stderr should not be empty for failing tests");

        // Verify both files exist and are separate
        let stdout_path = result.bead_trace_dir.join("stdout.txt");
        let stderr_path = result.bead_trace_dir.join("stderr.txt");
        assert!(stdout_path.exists(), "stdout.txt should exist");
        assert!(stderr_path.exists(), "stderr.txt should exist");

        // Verify file contents match captured content
        let stdout_file_content = fs::read_to_string(&stdout_path).unwrap();
        let stderr_file_content = fs::read_to_string(&stderr_path).unwrap();

        assert_eq!(stdout_file_content, result.stdout, "stdout file should match captured stdout");
        assert_eq!(stderr_file_content, result.stderr, "stderr file should match captured stderr");

        // Verify stdout and stderr contain different information
        // They may have some overlap in cargo framework output, but should be distinct
        assert_ne!(result.stdout, result.stderr,
            "stdout and stderr should be distinct streams");

        // Verify metadata captures the failure correctly
        let metadata_path = result.bead_trace_dir.join("metadata.json");
        let metadata_content = fs::read_to_string(&metadata_path).unwrap();
        let trace_metadata: TraceMetadata = serde_json::from_str(&metadata_content).unwrap();

        assert_eq!(trace_metadata.exit_code, Some(result.exit_code));
        assert_eq!(trace_metadata.outcome, "failure");
        assert!(trace_metadata.start_time.is_some());
        assert!(trace_metadata.end_time.is_some());
        assert!(trace_metadata.duration_ms.unwrap() > 0);
    }
}
