//! Exit code handling for bead-forge commands.
//!
//! Provides structured exit status representation and logging utilities.

use std::fmt;

/// Exit status for CLI commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    /// Command succeeded.
    Success = 0,
    /// General failure.
    Failure = 1,
    /// Usage error (invalid arguments, missing required args, etc.).
    Usage = 2,
    /// Database error or corruption.
    Database = 3,
    /// File I/O error.
    Io = 4,
    /// Validation error.
    Validation = 5,
    /// Conflict or concurrency error.
    Conflict = 6,
}

impl fmt::Display for ExitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExitStatus::Success => write!(f, "success"),
            ExitStatus::Failure => write!(f, "failure"),
            ExitStatus::Usage => write!(f, "usage error"),
            ExitStatus::Database => write!(f, "database error"),
            ExitStatus::Io => write!(f, "I/O error"),
            ExitStatus::Validation => write!(f, "validation error"),
            ExitStatus::Conflict => write!(f, "conflict"),
        }
    }
}

impl From<std::io::Error> for ExitStatus {
    fn from(_err: std::io::Error) -> Self {
        ExitStatus::Io
    }
}

impl ExitStatus {
    /// Get the numeric exit code.
    pub fn code(self) -> i32 {
        self as i32
    }

    /// Check if the exit status represents success.
    pub fn is_success(self) -> bool {
        self == ExitStatus::Success
    }

    /// Create an exit status from a numeric code.
    ///
    /// Unknown codes are mapped to `ExitStatus::Failure`.
    pub fn from_code(code: i32) -> Self {
        match code {
            0 => ExitStatus::Success,
            2 => ExitStatus::Usage,
            3 => ExitStatus::Database,
            4 => ExitStatus::Io,
            5 => ExitStatus::Validation,
            6 => ExitStatus::Conflict,
            _ => ExitStatus::Failure,
        }
    }
}

/// Format an exit code for logging purposes.
///
/// Converts a numeric exit code into a human-readable string
/// suitable for log output.
///
/// # Arguments
///
/// * `code` - The numeric exit code
///
/// # Returns
///
/// A string describing the exit code
pub fn format_exit_code_to_log(code: i32) -> String {
    let status = ExitStatus::from_code(code);
    format!("Exit code {}: {}", code, status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_status_codes() {
        assert_eq!(ExitStatus::Success.code(), 0);
        assert_eq!(ExitStatus::Failure.code(), 1);
        assert_eq!(ExitStatus::Usage.code(), 2);
        assert_eq!(ExitStatus::Database.code(), 3);
        assert_eq!(ExitStatus::Io.code(), 4);
        assert_eq!(ExitStatus::Validation.code(), 5);
        assert_eq!(ExitStatus::Conflict.code(), 6);
    }

    #[test]
    fn test_exit_status_is_success() {
        assert!(ExitStatus::Success.is_success());
        assert!(!ExitStatus::Failure.is_success());
    }

    #[test]
    fn test_exit_status_from_code() {
        assert_eq!(ExitStatus::from_code(0), ExitStatus::Success);
        assert_eq!(ExitStatus::from_code(2), ExitStatus::Usage);
        assert_eq!(ExitStatus::from_code(3), ExitStatus::Database);
        assert_eq!(ExitStatus::from_code(4), ExitStatus::Io);
        assert_eq!(ExitStatus::from_code(5), ExitStatus::Validation);
        assert_eq!(ExitStatus::from_code(6), ExitStatus::Conflict);
        assert_eq!(ExitStatus::from_code(1), ExitStatus::Failure);
        assert_eq!(ExitStatus::from_code(99), ExitStatus::Failure);
    }

    #[test]
    fn test_format_exit_code_to_log() {
        assert_eq!(
            format_exit_code_to_log(0),
            "Exit code 0: success"
        );
        assert_eq!(
            format_exit_code_to_log(1),
            "Exit code 1: failure"
        );
        assert_eq!(
            format_exit_code_to_log(2),
            "Exit code 2: usage error"
        );
        assert_eq!(
            format_exit_code_to_log(99),
            "Exit code 99: failure"
        );
    }

    #[test]
    fn test_exit_status_display() {
        assert_eq!(format!("{}", ExitStatus::Success), "success");
        assert_eq!(format!("{}", ExitStatus::Failure), "failure");
        assert_eq!(format!("{}", ExitStatus::Usage), "usage error");
        assert_eq!(format!("{}", ExitStatus::Database), "database error");
        assert_eq!(format!("{}", ExitStatus::Io), "I/O error");
        assert_eq!(format!("{}", ExitStatus::Validation), "validation error");
        assert_eq!(format!("{}", ExitStatus::Conflict), "conflict");
    }
}
