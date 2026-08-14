//! Event processing and timeout detection for NEEDLE worker events.
//!
//! This module provides functionality for processing events from NEEDLE workers,
//! with specific focus on detecting timeout-related exit codes (GNU timeout exit code 124)
//! and recording them with explicit timeout flags in events.jsonl.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, Write};
use std::path::Path;

/// GNU timeout exit code
pub const GNU_TIMEOUT_EXIT_CODE: i32 = 124;

/// Event types recorded by NEEDLE workers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkerEventType {
    Claim,
    Dispatch,
    Complete,
    Timeout,
    Custom(String),
}

impl WorkerEventType {
    pub fn as_str(&self) -> &str {
        match self {
            WorkerEventType::Claim => "claim",
            WorkerEventType::Dispatch => "dispatch",
            WorkerEventType::Complete => "complete",
            WorkerEventType::Timeout => "timeout",
            WorkerEventType::Custom(value) => value,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "claim" => WorkerEventType::Claim,
            "dispatch" => WorkerEventType::Dispatch,
            "complete" => WorkerEventType::Complete,
            "timeout" => WorkerEventType::Timeout,
            other => WorkerEventType::Custom(other.to_string()),
        }
    }
}

/// A worker event recorded in events.jsonl
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerEvent {
    /// Bead ID
    pub bead: String,
    /// Event type
    #[serde(rename = "event")]
    pub event_type: WorkerEventType,
    /// Timestamp (ISO 8601)
    pub ts: DateTime<Utc>,
    /// Worker name
    pub worker: String,
    /// Strand identifier
    #[serde(default)]
    pub strand: String,
    /// Optional adapter name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
    /// Optional model name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Optional exit code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Timeout flag (true if exit code 124)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub timeout: bool,
    /// Timeout duration in seconds (when available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_duration_secs: Option<u64>,
    /// Additional metadata
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl WorkerEvent {
    /// Create a new worker event
    pub fn new(
        bead: String,
        event_type: WorkerEventType,
        worker: String,
        strand: String,
    ) -> Self {
        Self {
            bead,
            event_type,
            ts: Utc::now(),
            worker,
            strand,
            adapter: None,
            model: None,
            exit_code: None,
            timeout: false,
            timeout_duration_secs: None,
            metadata: HashMap::new(),
        }
    }

    /// Add adapter information
    pub fn with_adapter(mut self, adapter: String) -> Self {
        self.adapter = Some(adapter);
        self
    }

    /// Add model information
    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }

    /// Add exit code and detect timeout
    pub fn with_exit_code(mut self, exit_code: i32) -> Self {
        self.exit_code = Some(exit_code);
        // Detect GNU timeout exit code
        if exit_code == GNU_TIMEOUT_EXIT_CODE {
            self.timeout = true;
            self.event_type = WorkerEventType::Timeout;
        }
        self
    }

    /// Add timeout duration
    pub fn with_timeout_duration(mut self, duration_secs: u64) -> Self {
        self.timeout_duration_secs = Some(duration_secs);
        self.timeout = true;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Check if this event represents a timeout
    pub fn is_timeout(&self) -> bool {
        self.timeout || self.event_type == WorkerEventType::Timeout
    }

    /// Serialize to JSON line format
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        Ok(serde_json::to_string(self)?)
    }

    /// Deserialize from JSON line format
    pub fn from_json_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }
}

/// Event processor for detecting and handling timeout events
pub struct EventProcessor {
    events_file: String,
}

impl EventProcessor {
    /// Create a new event processor
    pub fn new(events_file: String) -> Self {
        Self { events_file }
    }

    /// Process an exit code and determine if it represents a timeout
    pub fn process_exit_code(&self, exit_code: Option<i32>) -> ExitCodeInfo {
        match exit_code {
            Some(code) if code == GNU_TIMEOUT_EXIT_CODE => ExitCodeInfo {
                is_timeout: true,
                exit_code: code,
                timeout_type: TimeoutType::GnuTimeout,
                description: "GNU timeout (exit code 124)".to_string(),
            },
            Some(code) => ExitCodeInfo {
                is_timeout: false,
                exit_code: code,
                timeout_type: TimeoutType::NotTimeout,
                description: format!("Exit code {}", code),
            },
            None => ExitCodeInfo {
                is_timeout: false,
                exit_code: -1,
                timeout_type: TimeoutType::Unknown,
                description: "No exit code available".to_string(),
            },
        }
    }

    /// Record a worker event to the events.jsonl file
    pub fn record_event(&self, event: &WorkerEvent) -> std::io::Result<()> {
        let path = Path::new(&self.events_file);

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open file in append mode
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        // Write event as JSON line
        let json_line = event.to_json_line()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        writeln!(file, "{}", json_line)?;
        Ok(())
    }

    /// Record a timeout event with explicit timeout detection
    pub fn record_timeout_event(
        &self,
        bead: String,
        worker: String,
        strand: String,
        exit_code: i32,
        timeout_duration_secs: Option<u64>,
    ) -> std::io::Result<()> {
        let mut event = WorkerEvent::new(bead, WorkerEventType::Timeout, worker, strand);

        // Process the exit code
        let exit_info = self.process_exit_code(Some(exit_code));

        // Set timeout-specific fields
        event = event.with_exit_code(exit_code);
        event.timeout = exit_info.is_timeout;

        if let Some(duration) = timeout_duration_secs {
            event = event.with_timeout_duration(duration);
        }

        // Add timeout metadata
        event = event.with_metadata(
            "timeout_type".to_string(),
            exit_info.timeout_type.as_str().to_string()
        );

        self.record_event(&event)
    }

    /// Record a completion event with exit code
    pub fn record_completion_event(
        &self,
        bead: String,
        worker: String,
        strand: String,
        exit_code: i32,
    ) -> std::io::Result<()> {
        let mut event = WorkerEvent::new(bead, WorkerEventType::Complete, worker, strand);
        event = event.with_exit_code(exit_code);
        self.record_event(&event)
    }

    /// Read and parse events from the events.jsonl file
    pub fn read_events(&self) -> std::io::Result<Vec<WorkerEvent>> {
        let path = Path::new(&self.events_file);

        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(path)?;
        let mut events = Vec::new();

        for line in std::io::BufReader::new(file).lines() {
            let line = line?;
            if let Ok(event) = WorkerEvent::from_json_line(&line) {
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Filter events to get only timeout events
    pub fn get_timeout_events(&self) -> std::io::Result<Vec<WorkerEvent>> {
        let events = self.read_events()?;
        Ok(events.into_iter()
            .filter(|e| e.is_timeout())
            .collect())
    }

    /// Get timeout statistics
    pub fn get_timeout_stats(&self) -> std::io::Result<TimeoutStats> {
        let events = self.read_events()?;
        let timeout_events = events.iter().filter(|e| e.is_timeout()).collect::<Vec<_>>();

        Ok(TimeoutStats {
            total_events: events.len(),
            timeout_count: timeout_events.len(),
            workers_with_timeouts: timeout_events.iter()
                .map(|e| e.worker.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect(),
            last_timeout: timeout_events.last()
                .and_then(|e| Some(e.ts)),
        })
    }

    /// Get timeout statistics for a specific bead
    pub fn get_bead_timeout_stats(&self, bead_id: &str) -> std::io::Result<Option<BeadTimeoutStats>> {
        let events = self.read_events()?;
        let bead_timeout_events: Vec<&WorkerEvent> = events.iter()
            .filter(|e| e.bead == bead_id && e.is_timeout())
            .collect();

        if bead_timeout_events.is_empty() {
            return Ok(None);
        }

        let timeout_count = bead_timeout_events.len();
        let last_timeout = bead_timeout_events.last();
        let affected_workers: Vec<String> = bead_timeout_events.iter()
            .map(|e| e.worker.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        Ok(Some(BeadTimeoutStats {
            bead_id: bead_id.to_string(),
            timeout_count,
            last_timeout_duration_secs: last_timeout.and_then(|e| e.timeout_duration_secs),
            last_timeout_at: last_timeout.map(|e| e.ts),
            has_recurring_pattern: timeout_count >= 3,
            affected_workers,
        }))
    }
}

/// Information about an exit code
#[derive(Debug, Clone)]
pub struct ExitCodeInfo {
    /// Whether this exit code represents a timeout
    pub is_timeout: bool,
    /// The exit code value
    pub exit_code: i32,
    /// Type of timeout (if applicable)
    pub timeout_type: TimeoutType,
    /// Human-readable description
    pub description: String,
}

/// Types of timeouts
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeoutType {
    /// GNU timeout (exit code 124)
    GnuTimeout,
    /// Not a timeout
    NotTimeout,
    /// Unknown exit code
    Unknown,
}

impl TimeoutType {
    pub fn as_str(&self) -> &str {
        match self {
            TimeoutType::GnuTimeout => "gnu_timeout",
            TimeoutType::NotTimeout => "not_timeout",
            TimeoutType::Unknown => "unknown",
        }
    }
}

/// Timeout statistics for a specific bead
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadTimeoutStats {
    /// Bead ID
    pub bead_id: String,
    /// Number of timeout events for this bead
    pub timeout_count: usize,
    /// Last timeout duration (if available)
    pub last_timeout_duration_secs: Option<u64>,
    /// Last timeout timestamp
    pub last_timeout_at: Option<DateTime<Utc>>,
    /// Whether this bead has recurring timeout patterns (3+ timeouts)
    pub has_recurring_pattern: bool,
    /// Workers that experienced timeouts for this bead
    pub affected_workers: Vec<String>,
}

/// Timeout statistics (global)
#[derive(Debug, Clone)]
pub struct TimeoutStats {
    /// Total number of events
    pub total_events: usize,
    /// Number of timeout events
    pub timeout_count: usize,
    /// Workers that have experienced timeouts
    pub workers_with_timeouts: Vec<String>,
    /// Last timeout timestamp
    pub last_timeout: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_worker_event_creation() {
        let event = WorkerEvent::new(
            "bf-test".to_string(),
            WorkerEventType::Claim,
            "alpha".to_string(),
            "auto".to_string(),
        );

        assert_eq!(event.bead, "bf-test");
        assert_eq!(event.worker, "alpha");
        assert!(!event.timeout);
    }

    #[test]
    fn test_worker_event_with_exit_code_timeout_detection() {
        let event = WorkerEvent::new(
            "bf-test".to_string(),
            WorkerEventType::Complete,
            "alpha".to_string(),
            "auto".to_string(),
        ).with_exit_code(124); // GNU timeout exit code

        assert_eq!(event.exit_code, Some(124));
        assert!(event.timeout);
        assert_eq!(event.event_type, WorkerEventType::Timeout);
    }

    #[test]
    fn test_worker_event_with_normal_exit_code() {
        let event = WorkerEvent::new(
            "bf-test".to_string(),
            WorkerEventType::Complete,
            "alpha".to_string(),
            "auto".to_string(),
        ).with_exit_code(0); // Normal success

        assert_eq!(event.exit_code, Some(0));
        assert!(!event.timeout);
        assert_eq!(event.event_type, WorkerEventType::Complete);
    }

    #[test]
    fn test_worker_event_with_timeout_duration() {
        let event = WorkerEvent::new(
            "bf-test".to_string(),
            WorkerEventType::Complete,
            "alpha".to_string(),
            "auto".to_string(),
        ).with_timeout_duration(300);

        assert_eq!(event.timeout_duration_secs, Some(300));
        assert!(event.timeout);
    }

    #[test]
    fn test_worker_event_serialization() {
        let event = WorkerEvent::new(
            "bf-test".to_string(),
            WorkerEventType::Timeout,
            "alpha".to_string(),
            "auto".to_string(),
        ).with_exit_code(124);

        let json = event.to_json_line().unwrap();
        assert!(json.contains("\"timeout\":true"));
        assert!(json.contains("\"event\":\"timeout\""));
        assert!(json.contains("\"exit_code\":124"));
    }

    #[test]
    fn test_worker_event_deserialization() {
        let json = r#"{"bead":"bf-test","event":"timeout","ts":"2026-08-09T12:00:00Z","worker":"alpha","strand":"auto","exit_code":124,"timeout":true}"#;

        let event = WorkerEvent::from_json_line(json).unwrap();
        assert_eq!(event.bead, "bf-test");
        assert_eq!(event.worker, "alpha");
        assert!(event.timeout);
        assert_eq!(event.exit_code, Some(124));
    }

    #[test]
    fn test_event_processor_exit_code_processing() {
        let temp_dir = TempDir::new().unwrap();
        let events_file = temp_dir.path().join("events.jsonl").to_string_lossy().to_string();
        let processor = EventProcessor::new(events_file);

        // Test GNU timeout detection
        let timeout_info = processor.process_exit_code(Some(124));
        assert!(timeout_info.is_timeout);
        assert_eq!(timeout_info.exit_code, 124);
        assert_eq!(timeout_info.timeout_type, TimeoutType::GnuTimeout);

        // Test normal exit code
        let normal_info = processor.process_exit_code(Some(0));
        assert!(!normal_info.is_timeout);
        assert_eq!(normal_info.exit_code, 0);

        // Test unknown exit code
        let unknown_info = processor.process_exit_code(None);
        assert!(!unknown_info.is_timeout);
    }

    #[test]
    fn test_event_processor_record_and_read_events() {
        let temp_dir = TempDir::new().unwrap();
        let events_file = temp_dir.path().join("events.jsonl").to_string_lossy().to_string();
        let processor = EventProcessor::new(events_file.clone());

        // Record a timeout event
        processor.record_timeout_event(
            "bf-timeout".to_string(),
            "alpha".to_string(),
            "auto".to_string(),
            124,
            Some(300),
        ).unwrap();

        // Record a completion event
        processor.record_completion_event(
            "bf-complete".to_string(),
            "bravo".to_string(),
            "auto".to_string(),
            0,
        ).unwrap();

        // Read events back
        let events = processor.read_events().unwrap();
        assert_eq!(events.len(), 2);

        // Check timeout event
        let timeout_event = events.iter().find(|e| e.bead == "bf-timeout").unwrap();
        assert!(timeout_event.is_timeout());
        assert_eq!(timeout_event.exit_code, Some(124));
        assert_eq!(timeout_event.timeout_duration_secs, Some(300));

        // Check completion event
        let complete_event = events.iter().find(|e| e.bead == "bf-complete").unwrap();
        assert!(!complete_event.is_timeout());
        assert_eq!(complete_event.exit_code, Some(0));
    }

    #[test]
    fn test_event_processor_timeout_stats() {
        let temp_dir = TempDir::new().unwrap();
        let events_file = temp_dir.path().join("events.jsonl").to_string_lossy().to_string();
        let processor = EventProcessor::new(events_file);

        // Record some events
        processor.record_timeout_event("bf-timeout1".to_string(), "alpha".to_string(), "auto".to_string(), 124, Some(300)).unwrap();
        processor.record_timeout_event("bf-timeout2".to_string(), "bravo".to_string(), "auto".to_string(), 124, Some(300)).unwrap();
        processor.record_completion_event("bf-complete".to_string(), "alpha".to_string(), "auto".to_string(), 0).unwrap();

        let stats = processor.get_timeout_stats().unwrap();
        assert_eq!(stats.total_events, 3);
        assert_eq!(stats.timeout_count, 2);
        assert_eq!(stats.workers_with_timeouts.len(), 2);
        assert!(stats.last_timeout.is_some());
    }

    #[test]
    fn test_gnu_timeout_constant() {
        assert_eq!(GNU_TIMEOUT_EXIT_CODE, 124);
    }

    #[test]
    fn test_timeout_type_display() {
        assert_eq!(TimeoutType::GnuTimeout.as_str(), "gnu_timeout");
        assert_eq!(TimeoutType::NotTimeout.as_str(), "not_timeout");
        assert_eq!(TimeoutType::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_worker_event_from_str() {
        assert_eq!(WorkerEventType::from_str("claim"), WorkerEventType::Claim);
        assert_eq!(WorkerEventType::from_str("timeout"), WorkerEventType::Timeout);
        assert_eq!(WorkerEventType::from_str("custom_event"), WorkerEventType::Custom("custom_event".to_string()));
    }

    #[test]
    fn test_worker_event_as_str() {
        assert_eq!(WorkerEventType::Claim.as_str(), "claim");
        assert_eq!(WorkerEventType::Timeout.as_str(), "timeout");
        assert_eq!(WorkerEventType::Custom("test".to_string()).as_str(), "test");
    }

    #[test]
    fn test_multiple_exit_codes() {
        let temp_dir = TempDir::new().unwrap();
        let events_file = temp_dir.path().join("events.jsonl").to_string_lossy().to_string();
        let processor = EventProcessor::new(events_file);

        // Test various exit codes
        processor.record_completion_event("bf-0".to_string(), "alpha".to_string(), "auto".to_string(), 0).unwrap();
        processor.record_completion_event("bf-1".to_string(), "alpha".to_string(), "auto".to_string(), 1).unwrap();
        processor.record_completion_event("bf-124".to_string(), "alpha".to_string(), "auto".to_string(), 124).unwrap(); // timeout
        processor.record_completion_event("bf-127".to_string(), "alpha".to_string(), "auto".to_string(), 127).unwrap(); // command not found

        let _events = processor.read_events().unwrap();
        let timeout_events = processor.get_timeout_events().unwrap();

        assert_eq!(timeout_events.len(), 1);
        assert_eq!(timeout_events[0].bead, "bf-124");

        let stats = processor.get_timeout_stats().unwrap();
        assert_eq!(stats.total_events, 4);
        assert_eq!(stats.timeout_count, 1);
    }

    #[test]
    fn test_event_with_metadata() {
        let event = WorkerEvent::new(
            "bf-test".to_string(),
            WorkerEventType::Complete,
            "alpha".to_string(),
            "auto".to_string(),
        )
        .with_metadata("key1".to_string(), "value1".to_string())
        .with_metadata("key2".to_string(), "value2".to_string());

        assert_eq!(event.metadata.len(), 2);
        assert_eq!(event.metadata.get("key1"), Some(&"value1".to_string()));
        assert_eq!(event.metadata.get("key2"), Some(&"value2".to_string()));
    }
}