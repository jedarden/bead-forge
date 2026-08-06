//! Centralized error types for bead-forge.
//!
//! This module defines the core error type used throughout the bead-forge
//! codebase, providing consistent error handling with helpful context and
//! actionable messages.

use std::path::PathBuf;

/// Centralized error type for bead-forge operations.
///
/// This enum covers all error categories that can occur during bead-forge
/// operations, from database interactions to file I/O and validation.
#[derive(Debug, thiserror::Error)]
pub enum BeadForgeError {
    /// Database-related errors from SQLite operations.
    ///
    /// This wraps rusqlite errors with additional context about the operation
    /// that failed and which database/file was being accessed.
    #[error("Database error: {message}")]
    Database {
        message: String,
        #[source]
        source: rusqlite::Error,
        database_path: Option<PathBuf>,
    },

    /// I/O errors from file system operations.
    ///
    /// Covers file reading, writing, and general filesystem operations.
    #[error("I/O error: {message}")]
    Io {
        message: String,
        #[source]
        source: std::io::Error,
        path: Option<PathBuf>,
    },

    /// Parsing errors from structured data formats.
    ///
    /// Covers JSON, YAML, and other serialization/deserialization failures.
    #[error("Parsing error: {message}")]
    Parsing {
        message: String,
        format: ParsingFormat,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Validation errors for user input and data integrity.
    ///
    /// Covers malformed IDs, invalid state transitions, and constraint violations.
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        constraint: Option<String>,
    },

    /// Configuration errors from malformed or missing config.
    ///
    /// Covers config file parsing errors and missing required settings.
    #[error("Configuration error: {message}")]
    Config {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Not found errors for missing resources.
    ///
    /// Covers missing beads, files, directories, and other resources.
    #[error("Not found: {resource} '{identifier}'")]
    NotFound {
        resource: String,
        identifier: String,
        search_path: Option<PathBuf>,
    },

    /// Concurrent access errors from claim conflicts.
    ///
    /// Covers bead claiming failures due to concurrent access.
    #[error("Concurrent access error: {message}")]
    ConcurrentAccess {
        message: String,
        bead_id: Option<String>,
    },

    /// Migration errors during workspace or data migration.
    ///
    /// Covers migration failures and data inconsistencies.
    #[error("Migration error: {message}")]
    Migration {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Secret detection errors.
    ///
    /// Covers detected secrets in bead descriptions or comments.
    #[error("Secret detected: {0}")]
    Secret(String),

    /// Git operation errors.
    ///
    /// Covers git command execution failures.
    #[error("Git error: {message}")]
    Git {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Subprocess execution errors.
    ///
    /// Covers external command execution failures.
    #[error("Subprocess error: {message}")]
    Subprocess {
        message: String,
        command: String,
        exit_code: Option<i32>,
    },
}

/// The format being parsed when a parsing error occurs.
#[derive(Debug, Clone, Copy)]
pub enum ParsingFormat {
    Json,
    Yaml,
    Toml,
    Jsonl,
    Custom,
}

impl std::fmt::Display for ParsingFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParsingFormat::Json => write!(f, "JSON"),
            ParsingFormat::Yaml => write!(f, "YAML"),
            ParsingFormat::Toml => write!(f, "TOML"),
            ParsingFormat::Jsonl => write!(f, "JSONL"),
            ParsingFormat::Custom => write!(f, "custom format"),
        }
    }
}

/// Result type alias for bead-forge operations.
///
/// This is the recommended return type for all functions that can fail
/// in the bead-forge codebase.
pub type Result<T> = std::result::Result<T, BeadForgeError>;

// ============================================================================
// Conversion traits from common error types
// ============================================================================

impl From<rusqlite::Error> for BeadForgeError {
    fn from(err: rusqlite::Error) -> Self {
        BeadForgeError::Database {
            message: format!("SQLite operation failed: {}", err),
            source: err,
            database_path: None,
        }
    }
}

impl From<std::io::Error> for BeadForgeError {
    fn from(err: std::io::Error) -> Self {
        BeadForgeError::Io {
            message: format!("I/O operation failed: {}", err),
            source: err,
            path: None,
        }
    }
}

impl From<serde_json::Error> for BeadForgeError {
    fn from(err: serde_json::Error) -> Self {
        BeadForgeError::Parsing {
            message: format!("JSON parsing failed: {}", err),
            format: ParsingFormat::Json,
            source: Some(Box::new(err)),
        }
    }
}

impl From<serde_yaml::Error> for BeadForgeError {
    fn from(err: serde_yaml::Error) -> Self {
        BeadForgeError::Parsing {
            message: format!("YAML parsing failed: {}", err),
            format: ParsingFormat::Yaml,
            source: Some(Box::new(err)),
        }
    }
}

impl From<anyhow::Error> for BeadForgeError {
    fn from(err: anyhow::Error) -> Self {
        // anyhow::Error is a wrapper type - extract the underlying error if possible
        // Since we can't clone/own the underlying errors, we preserve context in messages

        if let Some(sqlite_err) = err.downcast_ref::<rusqlite::Error>() {
            return BeadForgeError::Database {
                message: format!("SQLite operation failed: {}", sqlite_err),
                source: rusqlite::Error::InvalidQuery, // Fallback source, message has full context
                database_path: None,
            };
        }

        if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
            return BeadForgeError::Io {
                message: format!("I/O operation failed: {}", io_err),
                source: std::io::Error::new(io_err.kind(), "anyhow error"), // Fallback source
                path: None,
            };
        }

        if let Some(json_err) = err.downcast_ref::<serde_json::Error>() {
            return BeadForgeError::Parsing {
                message: format!("JSON parsing failed: {}", json_err),
                format: ParsingFormat::Json,
                source: None, // Message preserves full error context
            };
        }

        if let Some(yaml_err) = err.downcast_ref::<serde_yaml::Error>() {
            return BeadForgeError::Parsing {
                message: format!("YAML parsing failed: {}", yaml_err),
                format: ParsingFormat::Yaml,
                source: None, // Message preserves full error context
            };
        }

        // Fallback: preserve error chain in message, no source needed
        BeadForgeError::Parsing {
            message: format!("Operation failed: {}", err),
            format: ParsingFormat::Custom,
            source: None,
        }
    }
}

// Forward SecretError to Secret variant (SecretError is local to sqlite.rs)
impl From<crate::storage::sqlite::SecretError> for BeadForgeError {
    fn from(err: crate::storage::sqlite::SecretError) -> Self {
        BeadForgeError::Secret(err.0)
    }
}

// chrono::ParseError conversion
impl From<chrono::ParseError> for BeadForgeError {
    fn from(err: chrono::ParseError) -> Self {
        BeadForgeError::Parsing {
            message: format!("Date/time parsing failed: {}", err),
            format: ParsingFormat::Custom,
            source: None,
        }
    }
}

// ============================================================================
// Constructor methods for each error variant
// ============================================================================

impl BeadForgeError {
    /// Create a database error with context.
    pub fn database<S: Into<String>>(
        message: S,
        source: rusqlite::Error,
        database_path: Option<PathBuf>,
    ) -> Self {
        BeadForgeError::Database {
            message: message.into(),
            source,
            database_path,
        }
    }

    /// Create an I/O error with context.
    pub fn io<S: Into<String>>(
        message: S,
        source: std::io::Error,
        path: Option<PathBuf>,
    ) -> Self {
        BeadForgeError::Io {
            message: message.into(),
            source,
            path,
        }
    }

    /// Create a parsing error with context.
    pub fn parsing<S: Into<String>>(
        message: S,
        format: ParsingFormat,
    ) -> Self {
        BeadForgeError::Parsing {
            message: message.into(),
            format,
            source: None,
        }
    }

    /// Create a parsing error from a source error.
    pub fn parsing_with_source<S: Into<String>>(
        message: S,
        format: ParsingFormat,
        source: Box<dyn std::error::Error + Send + Sync>,
    ) -> Self {
        BeadForgeError::Parsing {
            message: message.into(),
            format,
            source: Some(source),
        }
    }

    /// Create a validation error.
    pub fn validation<S: Into<String>>(message: S) -> Self {
        BeadForgeError::Validation {
            message: message.into(),
            constraint: None,
        }
    }

    /// Create a validation error with a specific constraint.
    pub fn validation_with_constraint<S: Into<String>, C: Into<String>>(
        message: S,
        constraint: C,
    ) -> Self {
        BeadForgeError::Validation {
            message: message.into(),
            constraint: Some(constraint.into()),
        }
    }

    /// Create a configuration error.
    pub fn config<S: Into<String>>(message: S) -> Self {
        BeadForgeError::Config {
            message: message.into(),
            source: None,
        }
    }

    /// Create a configuration error from a source error.
    pub fn config_with_source<S: Into<String>>(
        message: S,
        source: Box<dyn std::error::Error + Send + Sync>,
    ) -> Self {
        BeadForgeError::Config {
            message: message.into(),
            source: Some(source),
        }
    }

    /// Create a "not found" error.
    pub fn not_found<R: Into<String>, I: Into<String>>(
        resource: R,
        identifier: I,
        search_path: Option<PathBuf>,
    ) -> Self {
        BeadForgeError::NotFound {
            resource: resource.into(),
            identifier: identifier.into(),
            search_path,
        }
    }

    /// Create a concurrent access error.
    pub fn concurrent_access<S: Into<String>>(message: S, bead_id: Option<String>) -> Self {
        BeadForgeError::ConcurrentAccess {
            message: message.into(),
            bead_id,
        }
    }

    /// Create a migration error.
    pub fn migration<S: Into<String>>(message: S) -> Self {
        BeadForgeError::Migration {
            message: message.into(),
            source: None,
        }
    }

    /// Create a migration error from a source error.
    pub fn migration_with_source<S: Into<String>>(
        message: S,
        source: Box<dyn std::error::Error + Send + Sync>,
    ) -> Self {
        BeadForgeError::Migration {
            message: message.into(),
            source: Some(source),
        }
    }

    /// Create a git error.
    pub fn git<S: Into<String>>(message: S) -> Self {
        BeadForgeError::Git {
            message: message.into(),
            source: None,
        }
    }

    /// Create a git error from a source error.
    pub fn git_with_source<S: Into<String>>(
        message: S,
        source: Box<dyn std::error::Error + Send + Sync>,
    ) -> Self {
        BeadForgeError::Git {
            message: message.into(),
            source: Some(source),
        }
    }

    /// Create a subprocess error.
    pub fn subprocess<S: Into<String>, C: Into<String>>(
        message: S,
        command: C,
        exit_code: Option<i32>,
    ) -> Self {
        BeadForgeError::Subprocess {
            message: message.into(),
            command: command.into(),
            exit_code,
        }
    }

    /// Get the error category as a string for logging/filtering.
    pub fn category(&self) -> &'static str {
        match self {
            BeadForgeError::Database { .. } => "database",
            BeadForgeError::Io { .. } => "io",
            BeadForgeError::Parsing { .. } => "parsing",
            BeadForgeError::Validation { .. } => "validation",
            BeadForgeError::Config { .. } => "config",
            BeadForgeError::NotFound { .. } => "not_found",
            BeadForgeError::ConcurrentAccess { .. } => "concurrent_access",
            BeadForgeError::Migration { .. } => "migration",
            BeadForgeError::Secret(_) => "secret",
            BeadForgeError::Git { .. } => "git",
            BeadForgeError::Subprocess { .. } => "subprocess",
        }
    }

    /// Check if this error is retryable (e.g., transient failures).
    pub fn is_retryable(&self) -> bool {
        match self {
            // Database lock errors are typically retryable
            BeadForgeError::Database { source, .. } => {
                matches!(source, rusqlite::Error::SqliteFailure(_, _))
            }
            // Concurrent access can be retried
            BeadForgeError::ConcurrentAccess { .. } => true,
            // I/O errors that might be transient (some file locking cases)
            BeadForgeError::Io { source, .. } => {
                source.kind() == std::io::ErrorKind::WouldBlock
                    || source.kind() == std::io::ErrorKind::Interrupted
            }
            // Most other errors are not retryable
            _ => false,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categories() {
        // Create a valid SqliteFailure error with proper Error struct
        let sqlite_err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error { code: rusqlite::ffi::ErrorCode::CannotOpen, extended_code: 0 },
            Some("test error".to_string()),
        );
        let db_err = BeadForgeError::database("test", sqlite_err, None);
        assert_eq!(db_err.category(), "database");

        let io_err = BeadForgeError::io("test", std::io::Error::new(std::io::ErrorKind::NotFound, "test"), None);
        assert_eq!(io_err.category(), "io");

        let not_found_err = BeadForgeError::not_found("bead", "bf-123", None);
        assert_eq!(not_found_err.category(), "not_found");
    }

    #[test]
    fn test_retryable_database_errors() {
        // SqliteFailure errors are retryable (includes busy/locked states)
        let sqlite_err = BeadForgeError::database(
            "test",
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error { code: rusqlite::ffi::ErrorCode::DatabaseBusy, extended_code: 0 },
                Some("test busy".to_string()),
            ),
            None,
        );
        assert!(sqlite_err.is_retryable());

        let other_err = BeadForgeError::not_found("bead", "bf-123", None);
        assert!(!other_err.is_retryable());
    }

    #[test]
    fn test_conversion_from_rusqlite() {
        let sqlite_err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error { code: rusqlite::ffi::ErrorCode::CannotOpen, extended_code: 0 },
            Some("test error".to_string()),
        );
        let bf_err = BeadForgeError::from(sqlite_err);
        assert!(matches!(bf_err, BeadForgeError::Database { .. }));
    }

    #[test]
    fn test_conversion_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let bf_err = BeadForgeError::from(io_err);
        assert!(matches!(bf_err, BeadForgeError::Io { .. }));
    }

    #[test]
    fn test_conversion_from_serde_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let bf_err = BeadForgeError::from(json_err);
        assert!(matches!(bf_err, BeadForgeError::Parsing { format: ParsingFormat::Json, .. }));
    }

    #[test]
    fn test_helpful_error_messages() {
        let not_found = BeadForgeError::not_found("bead", "bf-1234", Some(PathBuf::from("/workspace")));
        let msg = format!("{}", not_found);
        assert!(msg.contains("bf-1234"));
        assert!(msg.contains("bead"));
    }

    #[test]
    fn test_parsing_format_display() {
        assert_eq!(format!("{}", ParsingFormat::Json), "JSON");
        assert_eq!(format!("{}", ParsingFormat::Yaml), "YAML");
        assert_eq!(format!("{}", ParsingFormat::Jsonl), "JSONL");
    }
}
