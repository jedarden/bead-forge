//! Execution time recording with cross-process persistence.
//!
//! This module provides timing instrumentation that survives process boundaries
//! by persisting timing state to disk. This enables accurate duration tracking
//! even when the monitoring process crashes and is restarted.
//!
//! # Features
//!
//! - Record start time to disk before command execution
//! - Calculate and record elapsed time after completion
//! - Resume timing from previous process invocation
//! - Accumulate timing across multiple runs
//! - Integration with trace metadata
//!
//! # Examples
//!
//! ```ignore
//! use bead_forge::timing::{ExecutionTimer, TimerState};
//! use std::path::Path;
//!
//! // Start a timer and persist to disk
//! let timer = ExecutionTimer::start(Path::new(".beads/timing/bf-123.json"))?;
//!
//! // ... do some work ...
//!
//! // In a new process (after crash/restart), resume the timer
//! let resumed = ExecutionTimer::resume(Path::new(".beads/timing/bf-123.json"))?;
//! let elapsed = resumed.elapsed_ms()?;
//!
//! // Complete the timer and write to trace
//! let metadata = timer.complete_with_metadata(&trace_manager, "bf-123")?;
//! ```

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Persistent timer state that can be written to disk and read back.
///
/// This structure captures all timing information needed to resume
/// a timer across process boundaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerState {
    /// Unique identifier for this timer
    pub id: String,
    /// When the timer was started (RFC3339)
    pub start_time: String,
    /// Unix timestamp when started (for precise duration calculation)
    pub start_timestamp_ms: i64,
    /// Optional description of what is being timed
    pub description: Option<String>,
    /// Optional bead ID associated with this timer
    pub bead_id: Option<String>,
    /// Whether the timer is currently running
    pub running: bool,
    /// Accumulated duration from previous runs (in milliseconds)
    pub accumulated_ms: u64,
    /// When this state was last updated
    pub updated_at: String,
}

impl TimerState {
    /// Create a new timer state.
    pub fn new(id: String, description: Option<String>, bead_id: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id,
            start_time: now.to_rfc3339(),
            start_timestamp_ms: now.timestamp_millis(),
            description,
            bead_id,
            running: true,
            accumulated_ms: 0,
            updated_at: now.to_rfc3339(),
        }
    }

    /// Mark the timer as stopped with a final duration.
    pub fn stop(&mut self, duration_ms: u64) {
        self.running = false;
        self.accumulated_ms = duration_ms;
        self.updated_at = Utc::now().to_rfc3339();
    }

    /// Add more accumulated time (for multi-phase timing).
    pub fn add_accumulated(&mut self, additional_ms: u64) {
        self.accumulated_ms += additional_ms;
        self.updated_at = Utc::now().to_rfc3339();
    }
}

/// Execution timer with cross-process persistence.
///
/// This timer can be started in one process, written to disk, and
/// resumed in another process. This enables accurate duration tracking
/// even across process crashes and restarts.
pub struct ExecutionTimer {
    /// In-memory start time (for calculating duration within this process)
    local_start: Instant,
    /// Persistent timer state
    state: TimerState,
    /// Path where the state file is stored
    state_path: PathBuf,
}

impl ExecutionTimer {
    /// Start a new execution timer and persist state to disk.
    ///
    /// This creates a new timer, records its start time, and immediately
    /// writes the state to disk. This ensures that even if the process
    /// crashes immediately after starting, the start time is preserved.
    ///
    /// # Arguments
    /// * `state_path` - Path where timer state will be persisted
    ///
    /// # Returns
    /// * `Result<ExecutionTimer>` - The new timer
    ///
    /// # Examples
    /// ```ignore
    /// let timer = ExecutionTimer::start(
    ///     Path::new(".beads/timing/bf-123.json")
    /// )?;
    /// ```
    pub fn start(state_path: &Path) -> Result<Self> {
        Self::start_with_metadata(state_path, None, None)
    }

    /// Start a new execution timer with metadata.
    ///
    /// # Arguments
    /// * `state_path` - Path where timer state will be persisted
    /// * `description` - Optional description of what is being timed
    /// * `bead_id` - Optional bead ID associated with this timer
    ///
    /// # Returns
    /// * `Result<ExecutionTimer>` - The new timer
    pub fn start_with_metadata(
        state_path: &Path,
        description: Option<String>,
        bead_id: Option<String>,
    ) -> Result<Self> {
        // Generate a unique timer ID
        let id = generate_timer_id();
        let state = TimerState::new(id.clone(), description, bead_id);

        // Create parent directory if needed
        if let Some(parent) = state_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create timing directory: {}", parent.display())
                })?;
            }
        }

        // Persist state immediately
        let state_json =
            serde_json::to_string_pretty(&state).context("Failed to serialize timer state")?;
        fs::write(state_path, state_json)
            .with_context(|| format!("Failed to write timer state to: {}", state_path.display()))?;

        Ok(Self {
            local_start: Instant::now(),
            state,
            state_path: state_path.to_path_buf(),
        })
    }

    /// Resume an existing timer from disk.
    ///
    /// This reads a previously-saved timer state and creates a new
    /// ExecutionTimer that can calculate the correct elapsed time,
    /// accounting for the time that passed while the process was not running.
    ///
    /// # Arguments
    /// * `state_path` - Path where timer state is stored
    ///
    /// # Returns
    /// * `Result<ExecutionTimer>` - The resumed timer
    ///
    /// # Examples
    /// ```ignore
    /// // In a new process after crash/restart
    /// let timer = ExecutionTimer::resume(
    ///     Path::new(".beads/timing/bf-123.json")
    /// )?;
    /// ```
    pub fn resume(state_path: &Path) -> Result<Self> {
        if !state_path.exists() {
            anyhow::bail!("Timer state file does not exist: {}", state_path.display());
        }

        // Read and deserialize state
        let state_json = fs::read_to_string(state_path).with_context(|| {
            format!("Failed to read timer state from: {}", state_path.display())
        })?;

        let mut state: TimerState =
            serde_json::from_str(&state_json).context("Failed to deserialize timer state")?;

        // Update the state to reflect that we're resuming
        state.updated_at = Utc::now().to_rfc3339();

        // Write updated state back to disk
        let updated_json = serde_json::to_string_pretty(&state)
            .context("Failed to serialize updated timer state")?;
        fs::write(state_path, updated_json).with_context(|| {
            format!(
                "Failed to write updated timer state to: {}",
                state_path.display()
            )
        })?;

        Ok(Self {
            local_start: Instant::now(),
            state,
            state_path: state_path.to_path_buf(),
        })
    }

    /// Check if a timer state file exists.
    ///
    /// # Arguments
    /// * `state_path` - Path to check
    ///
    /// # Returns
    /// * `bool` - true if the timer state file exists
    pub fn exists(state_path: &Path) -> bool {
        state_path.exists() && state_path.is_file()
    }

    /// Load timer state without resuming.
    ///
    /// This reads the timer state but doesn't create a new ExecutionTimer.
    /// Useful for checking status or reading metadata.
    ///
    /// # Arguments
    /// * `state_path` - Path where timer state is stored
    ///
    /// # Returns
    /// * `Result<TimerState>` - The timer state
    pub fn load_state(state_path: &Path) -> Result<TimerState> {
        if !state_path.exists() {
            anyhow::bail!("Timer state file does not exist: {}", state_path.display());
        }

        let state_json = fs::read_to_string(state_path).with_context(|| {
            format!("Failed to read timer state from: {}", state_path.display())
        })?;

        serde_json::from_str(&state_json).context("Failed to deserialize timer state")
    }

    /// Get the elapsed time in milliseconds.
    ///
    /// This calculates the duration from the original start time (accounting
    /// for process restarts) plus any accumulated time from previous runs.
    ///
    /// # Returns
    /// * `Result<u64>` - Elapsed time in milliseconds
    pub fn elapsed_ms(&self) -> Result<u64> {
        let now = Utc::now();
        let current_timestamp_ms = now.timestamp_millis();

        // Calculate elapsed time from the persisted start timestamp
        let elapsed_from_start =
            (current_timestamp_ms - self.state.start_timestamp_ms).max(0) as u64;

        // Add any previously accumulated time
        Ok(elapsed_from_start + self.state.accumulated_ms)
    }

    /// Get the elapsed time as a human-readable string.
    ///
    /// # Returns
    /// * `Result<String>` - Formatted duration (e.g., "1.5s", "350ms")
    pub fn elapsed_display(&self) -> Result<String> {
        let ms = self.elapsed_ms()?;
        Ok(format_duration(ms))
    }

    /// Stop the timer and persist the final state.
    ///
    /// This marks the timer as stopped and writes the final state to disk.
    ///
    /// # Returns
    /// * `Result<u64>` - Final duration in milliseconds
    pub fn stop(mut self) -> Result<u64> {
        let duration_ms = self.elapsed_ms()?;
        self.state.stop(duration_ms);

        let state_json = serde_json::to_string_pretty(&self.state)
            .context("Failed to serialize final timer state")?;
        fs::write(&self.state_path, state_json).with_context(|| {
            format!(
                "Failed to write final timer state to: {}",
                self.state_path.display()
            )
        })?;

        Ok(duration_ms)
    }

    /// Stop the timer and create trace metadata.
    ///
    /// This stops the timer and populates a TraceMetadata structure
    /// with the timing information.
    ///
    /// # Arguments
    /// * `bead_id` - Bead ID to include in metadata
    ///
    /// # Returns
    /// * `Result<crate::trace::TraceMetadata>` - Populated trace metadata
    pub fn complete_with_metadata(mut self, bead_id: &str) -> Result<crate::trace::TraceMetadata> {
        // Store the start time before consuming self
        let start_time = self.state.start_time.clone();
        let duration_ms = self.stop()?;

        let end_time = Utc::now().to_rfc3339();

        Ok(crate::trace::TraceMetadata {
            bead_id: Some(bead_id.to_string()),
            agent: "bead-forge".to_string(),
            provider: None,
            model: None,
            exit_code: None,
            outcome: "completed".to_string(),
            start_time: Some(start_time),
            end_time: Some(end_time),
            duration_ms: Some(duration_ms),
            input_tokens: None,
            output_tokens: None,
            cost_usd: None,
            captured_at: Utc::now().to_rfc3339(),
            trace_format: "timing".to_string(),
            pruned: false,
            template_version: None,
        })
    }

    /// Get a reference to the timer state.
    pub fn state(&self) -> &TimerState {
        &self.state
    }

    /// Get the timer ID.
    pub fn id(&self) -> &str {
        &self.state.id
    }

    /// Get the start time (RFC3339 format).
    pub fn start_time(&self) -> &str {
        &self.state.start_time
    }

    /// Check if the timer is currently running.
    pub fn is_running(&self) -> bool {
        self.state.running
    }

    /// Delete the timer state file.
    ///
    /// This removes the persisted state file from disk. Use this after
    /// completing the timer if you no longer need the state.
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    pub fn cleanup(self) -> Result<()> {
        if self.state_path.exists() {
            fs::remove_file(&self.state_path).with_context(|| {
                format!(
                    "Failed to remove timer state file: {}",
                    self.state_path.display()
                )
            })?;
        }
        Ok(())
    }
}

/// Generate a unique timer ID.
///
/// Uses timestamp + random components for uniqueness.
fn generate_timer_id() -> String {
    let now = Utc::now();
    let timestamp = now.format("%Y%m%d-%H%M%S-%3f");
    let random: u32 = rand::random();
    format!("timer-{}-{:08x}", timestamp, random)
}

/// Format a duration in milliseconds as a human-readable string.
///
/// # Arguments
/// * `ms` - Duration in milliseconds
///
/// # Returns
/// * `String` - Formatted duration
///
/// # Examples
/// ```
/// use bead_forge::timing::format_duration;
///
/// assert_eq!(format_duration(100), "100ms");
/// assert_eq!(format_duration(1500), "1.50s");
/// assert_eq!(format_duration(65000), "65.0s (1m 5s)");
/// ```
pub fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.2}s", ms as f64 / 1000.0)
    } else if ms < 3_600_000 {
        let seconds = (ms / 1000) % 60;
        let minutes = ms / 60_000;
        format!("{:.1}s ({}m {}s)", ms as f64 / 1000.0, minutes, seconds)
    } else {
        let hours = ms / 3_600_000;
        let minutes = (ms / 60_000) % 60;
        format!("{}h {}m ({:.1}s)", hours, minutes, ms as f64 / 1000.0)
    }
}

/// Record a start timestamp to a file.
///
/// This is a low-level function that writes just the start time to disk.
/// Used when you need to record a start time before spawning a subprocess.
///
/// # Arguments
/// * `start_file` - Path where start time will be written
///
/// # Returns
/// * `Result<i64>` - Start timestamp in milliseconds
pub fn record_start_time(start_file: &Path) -> Result<i64> {
    if let Some(parent) = start_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create timing directory: {}", parent.display())
            })?;
        }
    }

    let start_timestamp_ms = Utc::now().timestamp_millis();
    let start_time = Utc::now().to_rfc3339();

    let start_data = serde_json::json!({
        "start_time": start_time,
        "start_timestamp_ms": start_timestamp_ms,
        "recorded_at": Utc::now().to_rfc3339(),
    });

    fs::write(start_file, serde_json::to_string_pretty(&start_data)?)
        .with_context(|| format!("Failed to write start time to: {}", start_file.display()))?;

    Ok(start_timestamp_ms)
}

/// Read a start timestamp from a file.
///
/// This reads a previously-recorded start time and returns the timestamp.
///
/// # Arguments
/// * `start_file` - Path where start time is stored
///
/// # Returns
/// * `Result<i64>` - Start timestamp in milliseconds
pub fn read_start_time(start_file: &Path) -> Result<i64> {
    if !start_file.exists() {
        anyhow::bail!("Start time file does not exist: {}", start_file.display());
    }

    let content = fs::read_to_string(start_file)
        .with_context(|| format!("Failed to read start time from: {}", start_file.display()))?;

    let data: serde_json::Value =
        serde_json::from_str(&content).context("Failed to parse start time file")?;

    data.get("start_timestamp_ms")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| anyhow::anyhow!("Invalid start timestamp in file"))
}

/// Calculate elapsed time from a start file.
///
/// This reads the start time from a file and calculates the elapsed
/// duration until now.
///
/// # Arguments
/// * `start_file` - Path where start time is stored
///
/// # Returns
/// * `Result<u64>` - Elapsed time in milliseconds
pub fn calculate_elapsed_from_file(start_file: &Path) -> Result<u64> {
    let start_timestamp_ms = read_start_time(start_file)?;
    let current_timestamp_ms = Utc::now().timestamp_millis();

    Ok((current_timestamp_ms - start_timestamp_ms).max(0) as u64)
}

/// Record completion time and duration to a file.
///
/// This writes the completion metadata including duration to a file.
///
/// # Arguments
/// * `completion_file` - Path where completion data will be written
/// * `start_file` - Path to the start time file (for calculating duration)
/// * `exit_code` - Optional exit code to record
///
/// # Returns
/// * `Result<CompletionRecord>` - The completion record that was written
pub fn record_completion(
    completion_file: &Path,
    start_file: &Path,
    exit_code: Option<i32>,
) -> Result<CompletionRecord> {
    let start_timestamp_ms = read_start_time(start_file)?;
    let current_timestamp_ms = Utc::now().timestamp_millis();
    let duration_ms = (current_timestamp_ms - start_timestamp_ms).max(0) as u64;

    let start_time = if start_file.exists() {
        let content = fs::read_to_string(start_file)?;
        let data: serde_json::Value = serde_json::from_str(&content)?;
        data.get("start_time")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    } else {
        None
    };

    let end_time = Utc::now().to_rfc3339();

    let record = CompletionRecord {
        start_time,
        end_time: end_time.clone(),
        start_timestamp_ms,
        end_timestamp_ms: current_timestamp_ms,
        duration_ms,
        exit_code,
        recorded_at: Utc::now().to_rfc3339(),
    };

    if let Some(parent) = completion_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    fs::write(completion_file, serde_json::to_string_pretty(&record)?).with_context(|| {
        format!(
            "Failed to write completion record to: {}",
            completion_file.display()
        )
    })?;

    Ok(record)
}

/// Completion record written to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRecord {
    /// Start time (RFC3339)
    pub start_time: Option<String>,
    /// End time (RFC3339)
    pub end_time: String,
    /// Start timestamp (milliseconds since epoch)
    pub start_timestamp_ms: i64,
    /// End timestamp (milliseconds since epoch)
    pub end_timestamp_ms: i64,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Exit code (if applicable)
    pub exit_code: Option<i32>,
    /// When this record was written
    pub recorded_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(50), "50ms");
        assert_eq!(format_duration(999), "999ms");
        assert_eq!(format_duration(1000), "1.00s");
        assert_eq!(format_duration(1500), "1.50s");
        assert_eq!(format_duration(65_000), "65.0s (1m 5s)");
        assert_eq!(format_duration(3_700_000), "1h 1m (3700.0s)");
    }

    #[test]
    fn test_timer_state_creation() {
        let state = TimerState::new(
            "test-timer".to_string(),
            Some("Test description".to_string()),
            Some("bf-123".to_string()),
        );

        assert_eq!(state.id, "test-timer");
        assert_eq!(state.description, Some("Test description".to_string()));
        assert_eq!(state.bead_id, Some("bf-123".to_string()));
        assert!(state.running);
        assert_eq!(state.accumulated_ms, 0);
    }

    #[test]
    fn test_timer_state_stop() {
        let mut state = TimerState::new("test-timer".to_string(), None, None);

        state.stop(5000);
        assert!(!state.running);
        assert_eq!(state.accumulated_ms, 5000);
    }

    #[test]
    fn test_timer_state_add_accumulated() {
        let mut state = TimerState::new("test-timer".to_string(), None, None);

        state.add_accumulated(1000);
        assert_eq!(state.accumulated_ms, 1000);

        state.add_accumulated(500);
        assert_eq!(state.accumulated_ms, 1500);
    }

    #[test]
    fn test_execution_timer_start() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("timer.json");

        let timer = ExecutionTimer::start(&state_path).unwrap();

        assert!(timer.is_running());
        assert!(state_path.exists());
        assert!(timer.elapsed_ms().unwrap() < 100); // Should be very recent
    }

    #[test]
    fn test_execution_timer_start_with_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("timer.json");

        let timer = ExecutionTimer::start_with_metadata(
            &state_path,
            Some("Test operation".to_string()),
            Some("bf-456".to_string()),
        )
        .unwrap();

        assert_eq!(timer.state().bead_id, Some("bf-456".to_string()));
        assert_eq!(
            timer.state().description,
            Some("Test operation".to_string())
        );
    }

    #[test]
    fn test_execution_timer_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("timer.json");

        // Start a timer
        let timer1 = ExecutionTimer::start_with_metadata(
            &state_path,
            Some("Persist test".to_string()),
            None,
        )
        .unwrap();
        let id1 = timer1.id().to_string();
        let start_time1 = timer1.start_time().to_string();

        // Wait a bit
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Resume the timer in a "new process"
        let timer2 = ExecutionTimer::resume(&state_path).unwrap();

        assert_eq!(timer2.id(), id1);
        assert_eq!(timer2.start_time(), start_time1);
        assert_eq!(timer2.state().description, Some("Persist test".to_string()));
        assert!(timer2.is_running());
    }

    #[test]
    fn test_execution_timer_elapsed() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("timer.json");

        let timer = ExecutionTimer::start(&state_path).unwrap();

        // Wait a bit
        std::thread::sleep(std::time::Duration::from_millis(100));

        let elapsed = timer.elapsed_ms().unwrap();
        assert!(elapsed >= 100);
        assert!(elapsed < 200); // Should not be too long
    }

    #[test]
    fn test_execution_timer_stop() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("timer.json");

        let timer = ExecutionTimer::start(&state_path).unwrap();

        // Wait a bit
        std::thread::sleep(std::time::Duration::from_millis(50));

        let duration_ms = timer.stop().unwrap();
        assert!(duration_ms >= 50);

        // Verify state file was updated
        let loaded_state = ExecutionTimer::load_state(&state_path).unwrap();
        assert!(!loaded_state.running);
        assert_eq!(loaded_state.accumulated_ms, duration_ms);
    }

    #[test]
    fn test_execution_timer_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("timer.json");

        let timer = ExecutionTimer::start(&state_path).unwrap();
        assert!(state_path.exists());

        timer.cleanup().unwrap();
        assert!(!state_path.exists());
    }

    #[test]
    fn test_record_start_time() {
        let temp_dir = TempDir::new().unwrap();
        let start_file = temp_dir.path().join("start.json");

        let timestamp_ms = record_start_time(&start_file).unwrap();

        assert!(start_file.exists());
        assert!(timestamp_ms > 0);

        // Verify we can read it back
        let read_timestamp = read_start_time(&start_file).unwrap();
        assert_eq!(timestamp_ms, read_timestamp);
    }

    #[test]
    fn test_calculate_elapsed_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let start_file = temp_dir.path().join("start.json");

        record_start_time(&start_file).unwrap();

        // Wait a bit
        std::thread::sleep(std::time::Duration::from_millis(50));

        let elapsed = calculate_elapsed_from_file(&start_file).unwrap();
        assert!(elapsed >= 50);
        assert!(elapsed < 150);
    }

    #[test]
    fn test_record_completion() {
        let temp_dir = TempDir::new().unwrap();
        let start_file = temp_dir.path().join("start.json");
        let completion_file = temp_dir.path().join("completion.json");

        record_start_time(&start_file).unwrap();

        // Simulate some work
        std::thread::sleep(std::time::Duration::from_millis(50));

        let record = record_completion(&completion_file, &start_file, Some(0)).unwrap();

        assert!(completion_file.exists());
        assert_eq!(record.exit_code, Some(0));
        assert!(record.duration_ms >= 50);
        assert!(record.start_time.is_some());
        assert!(!record.end_time.is_empty());
    }

    #[test]
    fn test_timer_exists() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("timer.json");

        assert!(!ExecutionTimer::exists(&state_path));

        ExecutionTimer::start(&state_path).unwrap();
        assert!(ExecutionTimer::exists(&state_path));
    }

    #[test]
    fn test_elapsed_display() {
        let temp_dir = TempDir::new().unwrap();
        let state_path = temp_dir.path().join("timer.json");

        let timer = ExecutionTimer::start(&state_path).unwrap();

        // Very short duration should be in ms
        let display = timer.elapsed_display().unwrap();
        assert!(display.ends_with("ms") || display.ends_with("s"));
    }
}
