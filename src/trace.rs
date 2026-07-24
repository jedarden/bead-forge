//! Trace file infrastructure for capturing and storing test/execution output.
//!
//! Provides structured trace file management with proper error handling for
//! directory creation and file writing operations.

use anyhow::{Context, Result};
use chrono::Utc;
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;

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
}
